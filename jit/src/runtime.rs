use std::panic;

use crate::cache::{BlockCache, BlockSlot, CompiledBlock};
use crate::{call_indirect, codegen, console_log};

use interpreter::cpu::Cpu;
use interpreter::cpu::opcodes::Opcode;
use interpreter::cpu::registers::Flags;
use interpreter::joypad::ButtonsHeld;

#[cfg(feature = "log-uncompiled")]
use {
    interpreter::cpu::opcodes::Opcode, interpreter::cpu::opcodes::PrefixOpcode,
    std::collections::HashMap,
};

unsafe extern "C" {
    // Compiles and instantiates a Wasm module using the bytecode in `buffer`, then adds its function to table 1 of *this* module.
    // Wasm-bindgen uses table 0 for doing Rust <-> JavaScript communication, so we use table 1 to avoid stepping on its toes.
    // Returns the index of the newly linked function in table 1.
    fn instantiate_and_link_module(buffer: *const u8, len: usize) -> i32;
}

#[derive(Clone, Copy, Default)]
struct CacheAddress {
    bank_number: u8,
    address: u16,
}

impl CacheAddress {
    fn to_usize(self) -> usize {
        ((self.bank_number as usize) << 16) | self.address as usize
    }
}

// TODO: Should probably implement PostBoot too/instead...
#[derive(Default)]
pub struct JitRuntime {
    ptr: usize,
    registers_ptr: usize,
    work_ram_ptr: usize,
    block_start_clock: u64,
    checkpoint_index: usize,
    pub(crate) dmg_state: Cpu,
    block_cache: BlockCache,
    currently_executing: CacheAddress,
    rom_buffer: Vec<u8>,
    next_vblank: u64,
    #[cfg(feature = "log-uncompiled")]
    uncompiled: HashMap<u16, u32>,
    #[cfg(feature = "log-uncompiled")]
    too_long: usize,
    #[cfg(feature = "log-uncompiled")]
    not_too_long: usize,
}

impl JitRuntime {
    fn execute_compiled_block(&mut self, cache_address: CacheAddress) {
        let compiled_block = self.block_cache[cache_address.to_usize()].unwrap_compiled_block();

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
        self.dmg_state
            .memory
            .increment_timers(checkpoint.remaining_m_cycles);
    }

    // TODO: Handle bank switches while executing in a switchable bank (do any games or test ROMs do this?).
    fn current_cache_address(&self) -> CacheAddress {
        let pc = self.dmg_state.registers.pc;
        let bank_number = match pc {
            0x4000..0x8000 => self.dmg_state.memory.mbc.current_rom_bank,
            _ => 0,
        };

        CacheAddress {
            bank_number,
            address: pc,
        }
    }

    // Get the next CompiledBlock at PC, either from the cache or by compiling a new block.
    fn has_compiled_block(&mut self) -> Option<CacheAddress> {
        let cache_address = self.current_cache_address();
        match self.block_cache[cache_address.to_usize()] {
            BlockSlot::Uncompilable => None,
            BlockSlot::Compiled(_) => Some(cache_address),
            BlockSlot::Uncompiled => {
                if let Some(jit_block) = codegen::recompile(
                    &mut self.dmg_state,
                    self.ptr,
                    self.registers_ptr,
                    self.work_ram_ptr,
                ) {
                    #[cfg(feature = "jit-trace")]
                    console_log(&wasmprinter::print_bytes(&jit_block.buffer).unwrap());

                    let ptr = jit_block.buffer.as_ptr();
                    let len = jit_block.buffer.len();
                    let func_idx = unsafe { instantiate_and_link_module(ptr, len) };
                    let compiled_block = CompiledBlock {
                        func_idx,
                        checkpoints: jit_block.ctx.checkpoints,
                    };

                    // Add the block we just compiled to the cache.
                    self.block_cache[cache_address.to_usize()] =
                        BlockSlot::Compiled(compiled_block);
                    Some(cache_address)
                } else {
                    // Cache "None", indicating that the instruction at this address must be interpreted.
                    self.block_cache[cache_address.to_usize()] = BlockSlot::Uncompilable;

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
    fn wont_be_interrupted(&self, cache_address: CacheAddress) -> bool {
        let compiled_block = self.block_cache[cache_address.to_usize()].unwrap_compiled_block();

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
    #[unsafe(no_mangle)]
    pub extern "C" fn read_byte_mem(
        address: u16,
        delta_m_cycles: u16,
        runtime: &mut JitRuntime,
    ) -> u8 {
        runtime.dmg_state.memory.increment_timers(delta_m_cycles);
        runtime.dmg_state.memory.read_byte(address)
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn write_byte_mem(
        value: u8,
        address: u16,
        delta_m_cycles: u16,
        runtime: &mut JitRuntime,
    ) {
        runtime.dmg_state.memory.increment_timers(delta_m_cycles);
        runtime.dmg_state.memory.write_byte(address, value);
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn process_checkpoint(checkpoint_index: u32, runtime: &mut JitRuntime) -> bool {
        let current_block =
            runtime.block_cache[runtime.currently_executing.to_usize()].unwrap_compiled_block();

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
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn step_vblank(&mut self) {
        self.next_vblank += 70224;
        while self.dmg_state.memory.clock < self.next_vblank {
            self.execute();
        }
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

    #[unsafe(no_mangle)]
    pub extern "C" fn set_ptr(runtime: &mut JitRuntime, ptr: usize) {
        runtime.ptr = ptr;
        runtime.registers_ptr = core::ptr::addr_of!(runtime.dmg_state.registers) as usize;
        runtime.work_ram_ptr = runtime.dmg_state.memory.buffer.as_ptr() as usize;
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn make_runtime() -> *const JitRuntime {
    // Log panic messages to the JavaScript console.
    panic::set_hook(Box::new(|panic_info| {
        console_log(&format!("panic occurred: {panic_info}"));
    }));

    let runtime = Box::new(JitRuntime::default());
    // Leak the JitRuntime and return its pointer to the embedder.
    Box::into_raw(runtime)
}
