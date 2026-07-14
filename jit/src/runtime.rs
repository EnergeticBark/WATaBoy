use std::panic;

use crate::cache::{BlockCache, BlockSlot, CacheAddress, CompiledBlock};
use crate::codegen::WasmBlock;
use crate::{call_indirect, console_error, console_log};

use hw_constants::io_regs::IF;
use interpreter::cpu::Cpu;
use interpreter::cpu::opcodes::Opcode;
use interpreter::cpu::registers::Flags;
use interpreter::joypad::ButtonsHeld;

#[cfg(feature = "log-uncompiled")]
use {interpreter::cpu::opcodes::PrefixOpcode, std::collections::HashMap};

// TODO: Should probably implement PostBoot too/instead...
#[derive(Default)]
pub struct JitRuntime {
    ptr: usize,
    rom_ptr: usize,
    block_start_clock: u64,
    checkpoint_index: usize,
    pub(crate) dmg_state: Cpu,
    block_cache: BlockCache,
    currently_executing: CacheAddress,
    rom_buffer: Vec<u8>,
    sram_buffer: Vec<u8>,
    next_vblank: u64,
    #[cfg(feature = "log-uncompiled")]
    uncompiled: HashMap<u16, u32>,
    #[cfg(feature = "log-uncompiled")]
    too_long: usize,
    #[cfg(feature = "log-uncompiled")]
    not_too_long: usize,
}

// Functions exported publicly to the embedder.
impl JitRuntime {
    /// Create a new instance of `JitRuntime` on the heap.
    /// Usually the first function called from the embedder after the main Wasm module has been instantiated.
    /// ## Returns
    /// A pointer to the newly created `JitRuntime`
    #[must_use]
    #[unsafe(no_mangle)]
    pub extern "C" fn make_runtime() -> *mut Self {
        // Log panic messages to the JavaScript console.
        panic::set_hook(Box::new(|panic_info| {
            console_error(&format!("{panic_info}"));
        }));

        let mut runtime = Box::new(JitRuntime::default());

        // The runtime needs to know its own location in memory for the codegen to
        // emit self referencing function calls to `read_byte_mem`, `processes_checkpoint`, etc.
        runtime.ptr = &raw const *runtime as usize;

        // Leak the JitRuntime and return its pointer to the embedder.
        Box::into_raw(runtime)
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn read_byte_mem(
        address: u16,
        total_m_cycles: u16,
        runtime: &mut JitRuntime,
    ) -> u8 {
        let delta_m_cycles = runtime.remaining_m_cycles(total_m_cycles);
        runtime.dmg_state.memory.increment_timers(delta_m_cycles);
        runtime.dmg_state.memory.read_byte(address)
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn write_byte_mem(
        value: u8,
        address: u16,
        total_m_cycles: u16,
        runtime: &mut JitRuntime,
    ) {
        let delta_m_cycles = runtime.remaining_m_cycles(total_m_cycles);
        runtime.dmg_state.memory.increment_timers(delta_m_cycles);
        runtime.dmg_state.memory.write_byte(address, value);
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn process_checkpoint(checkpoint_index: u32, runtime: &mut JitRuntime) -> bool {
        let current_block =
            runtime.block_cache[runtime.currently_executing].unwrap_compiled_block();

        let next_checkpoint = current_block.checkpoints[checkpoint_index as usize + 1];
        let next_checkpoint_clock =
            runtime.block_start_clock + u64::from(next_checkpoint.total_m_cycles) * 4;

        if runtime.dmg_state.memory.next_interrupt < next_checkpoint_clock {
            runtime.checkpoint_index = checkpoint_index as usize;
            true
        } else {
            false
        }
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn realloc_rom_buffer(&mut self, rom_length: usize) -> *mut u8 {
        self.rom_buffer = vec![0; rom_length];
        self.rom_buffer.as_mut_ptr()
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn load_rom_from_buffer(&mut self) {
        self.dmg_state.memory.load_rom(&self.rom_buffer);
        self.rom_ptr = self.dmg_state.memory.mbc.rom_base_ptr() as usize;
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn realloc_sram_buffer(&mut self, sram_length: usize) -> *mut u8 {
        self.sram_buffer = vec![0; sram_length];
        self.sram_buffer.as_mut_ptr()
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn load_sram_from_buffer(&mut self) {
        self.dmg_state.memory.load_sram(&self.sram_buffer);
    }

    #[must_use]
    #[unsafe(no_mangle)]
    pub extern "C" fn get_sram_ptr(&self) -> *const u8 {
        self.dmg_state.memory.dump_sram().as_ptr()
    }

    #[must_use]
    #[unsafe(no_mangle)]
    pub extern "C" fn get_sram_len(&self) -> usize {
        self.dmg_state.memory.dump_sram().len()
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn step_vblank(&mut self) {
        self.next_vblank += 70224;
        while self.dmg_state.memory.ppu.line_number >= 144
            && self.dmg_state.memory.clock < self.next_vblank
        {
            self.execute();
        }

        while self.dmg_state.memory.ppu.line_number < 144
            && self.dmg_state.memory.clock < self.next_vblank
        {
            self.execute();
        }

        // Force catch-up the PPU.
        let mmu = &mut self.dmg_state.memory;
        mmu.ppu.catch_up(mmu.clock, &mut mmu.buffer[IF as usize]);
    }

    // TODO: Figure out a nice way to pass C structs across runtime boundaries without resorting to wasm-bindgen.
    #[unsafe(no_mangle)]
    pub extern "C" fn update_joypad(
        &mut self,
        start: bool,
        select: bool,
        b: bool,
        a: bool,
        down: bool,
        up: bool,
        left: bool,
        right: bool,
    ) {
        self.dmg_state.memory.buttons_held = ButtonsHeld {
            start,
            select,
            b,
            a,
            down,
            up,
            left,
            right,
        };
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn get_lcd_buffer(runtime: &mut JitRuntime) -> *const u8 {
        runtime.dmg_state.memory.ppu.lcd_buffer.as_ptr()
    }

    #[cfg(feature = "log-uncompiled")]
    #[unsafe(no_mangle)]
    pub extern "C" fn log_uncompiled(&self) {
        let mut not_compiled_vec: Vec<(&u16, &u32)> = self.uncompiled.iter().collect();
        not_compiled_vec.sort_by(|a, b| b.1.cmp(a.1));
        for (opcode, count) in not_compiled_vec {
            if *opcode <= 0xff {
                let opcode = Opcode::decode(*opcode as u8);
                console_log(&format!("{opcode:?}: {count}"));
            } else {
                let prefix_opcode = PrefixOpcode::decode((opcode & 0xff) as u8);
                console_log(&format!("{prefix_opcode:?}: {count}"));
            }
        }

        console_log(&format!("Too Long: {}", self.too_long));
        console_log(&format!("Not Too Long: {}", self.not_too_long));
    }
}

impl JitRuntime {
    #[allow(clippy::cast_possible_truncation)]
    fn remaining_m_cycles(&self, total_m_cycles: u16) -> u16 {
        let m_cycles_so_far = (self.dmg_state.memory.clock - self.block_start_clock) / 4;
        (u64::from(total_m_cycles) - m_cycles_so_far) as u16
    }

    fn execute_compiled_block(&mut self, cache_address: CacheAddress) {
        let compiled_block = self.block_cache[cache_address].unwrap_compiled_block();

        self.currently_executing = cache_address;
        self.block_start_clock = self.dmg_state.memory.clock;
        self.checkpoint_index = compiled_block.checkpoints.len() - 1;

        call_indirect(compiled_block.func_idx);

        // Ensure that the unused bits in Flags go unset.
        // TODO: Do this in the block itself.
        self.dmg_state.registers.af.set_f(Flags::from_bits(
            self.dmg_state.registers.af.f().into_bits(),
        ));

        let checkpoint = compiled_block.checkpoints[self.checkpoint_index];
        // Update the program counter and clock.
        self.dmg_state.registers.pc = checkpoint.exit_pc;

        let remaining_m_cycles = self.remaining_m_cycles(checkpoint.total_m_cycles);
        self.dmg_state.memory.increment_timers(remaining_m_cycles);
    }

    // TODO: Handle bank switches while executing in a switchable bank (do any games or test ROMs do this?).
    fn current_cache_address(&self) -> CacheAddress {
        let pc = self.dmg_state.registers.pc;
        let bank_number = match pc {
            0x4000..0x8000 => self.dmg_state.memory.mbc.current_rom_bank(),
            _ => 0,
        };

        CacheAddress::new()
            .with_bank_number(bank_number)
            .with_address(pc)
    }

    // Get the next CompiledBlock at PC, either from the cache or by compiling a new block.
    fn has_compiled_block(&mut self) -> Option<CacheAddress> {
        let cache_address = self.current_cache_address();
        match self.block_cache[cache_address] {
            BlockSlot::Uncompilable => None,
            BlockSlot::Compiled(_) => Some(cache_address),
            BlockSlot::Uncompiled => {
                // Don't cache below 0x100 if the boot ROM is mounted!
                if cache_address.address() < 0x100 && self.dmg_state.memory.boot_rom_mounted() {
                    None
                } else if let Some(wasm_block) =
                    WasmBlock::recompile(&mut self.dmg_state, self.ptr, self.rom_ptr)
                {
                    let compiled_block = CompiledBlock::new(wasm_block);

                    // Add the block we just compiled to the cache.
                    self.block_cache[cache_address] = BlockSlot::Compiled(compiled_block);
                    Some(cache_address)
                } else {
                    // Cache "Uncompilable", indicating that the instruction at this address must be interpreted.
                    self.block_cache[cache_address] = BlockSlot::Uncompilable;

                    #[cfg(feature = "log-uncompiled")]
                    {
                        let pc = self.dmg_state.registers.pc;
                        if !self.dmg_state.memory.boot_rom_mounted() && pc < 0x8000 {
                            let mut opcode = self.dmg_state.memory.read_byte(pc) as u16;

                            if opcode == 0xCB {
                                let prefix_opcode = self.dmg_state.memory.read_byte(pc + 1);
                                opcode = u16::from_be_bytes([0xCB, prefix_opcode]);
                            }

                            if let Some(count) = self.uncompiled.get_mut(&opcode) {
                                *count += 1;
                            } else {
                                self.uncompiled.insert(opcode, 1);
                            }
                        }
                    }

                    None
                }
            }
        }
    }

    // Check if we can execute compiled_block up to the first checkpoint without being interrupted.
    // TODO: Should probably ensure that an OAM DMA isn't active too.
    fn wont_be_interrupted(&self, cache_address: CacheAddress) -> bool {
        let compiled_block = self.block_cache[cache_address].unwrap_compiled_block();

        self.dmg_state.memory.clock + u64::from(compiled_block.checkpoints[0].total_m_cycles) * 4
            <= self.dmg_state.memory.next_interrupt
    }

    // If possible, execute the next JIT-compiled block.
    pub(crate) fn execute(&mut self) {
        self.dmg_state.handle_interrupts();
        if self.dmg_state.halted {
            return;
        }

        let pc = self.dmg_state.registers.pc;

        cfg_select! {
            feature = "disable-jit" => {
                // Always use the interpreter if JIT is disabled.
                let bytecode = self.dmg_state.memory.read_byte(pc);
                let opcode = Opcode::decode(bytecode).unwrap();
                self.dmg_state.execute_op(opcode).unwrap();
            }
            _ => {
                // Only cache from ROM banks for now.
                if pc < 0x8000
                    && let Some(cache_address) = self.has_compiled_block()
                    && self.wont_be_interrupted(cache_address)
                {
                    self.execute_compiled_block(cache_address);

                    #[cfg(feature = "log-uncompiled")]
                    {
                        self.not_too_long += 1;
                    }
                } else {
                    #[cfg(feature = "log-uncompiled")]
                    if pc < 0x8000
                        && let Some(cache_address) = self.has_compiled_block()
                        && !self.wont_be_interrupted(cache_address)
                    {
                        self.too_long += 1;
                    }

                    // Fallback to interpreter.
                    let bytecode = self.dmg_state.memory.read_byte(pc);
                    let opcode = Opcode::decode(bytecode).unwrap();
                    self.dmg_state.execute_op(opcode).unwrap();
                }
            }
        }
    }
}
