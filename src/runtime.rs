use intmap::{IntKey, IntMap};
use std::panic;

use crate::cache::CompiledBlock;
use crate::{call_indirect, codegen, console_log};

use sm83_interp::cpu::Cpu;

#[cfg(feature = "log-uncompiled")]
use sm83_interp::cpu::opcodes::Opcode;
#[cfg(feature = "log-uncompiled")]
use sm83_interp::cpu::opcodes::PrefixOpcode;

use sm83_interp::cpu::registers::Flags;
use sm83_interp::joypad::ButtonsHeld;

unsafe extern "C" {
    // Compiles and instantiates a Wasm module using the bytecode in `buffer`, then adds its function to table 1 of *this* module.
    // Wasm-bindgen uses table 0 for doing Rust <-> JavaScript communication, so we use table 1 to avoid stepping on its toes.
    // Returns the index of the newly linked function in table 1.
    fn instantiate_and_link_module(buffer: *const u8, len: u32) -> i32;
}

#[derive(Clone, Copy, Default)]
struct CacheAddress {
    bank_number: u8,
    address: u16,
}

impl IntKey for CacheAddress {
    type Int = u32;

    // You could also choose another prime number
    const PRIME: Self::Int = u32::PRIME;

    fn into_int(self) -> Self::Int {
        ((self.bank_number as u32) << 16) | self.address as u32
    }
}

// TODO: Should probably implement PostBoot too/instead...
#[derive(Default)]
pub struct JitRuntime {
    ptr: usize,
    block_start_clock: u64,
    checkpoint_index: usize,
    pub(crate) dmg_state: Cpu,
    block_cache: IntMap<CacheAddress, CompiledBlock>,
    currently_executing: CacheAddress,
    rom_buffer: Vec<u8>,
    next_vblank: u64,
    #[cfg(feature = "log-uncompiled")]
    uncompiled: IntMap<u16, u32>,
}

impl JitRuntime {
    fn execute_compiled_block(&mut self, cache_address: CacheAddress) {
        let compiled_block = self.block_cache.get(cache_address).unwrap();
        self.currently_executing = cache_address;
        self.block_start_clock = self.dmg_state.memory.clock;
        self.checkpoint_index = compiled_block.checkpoints.len() - 1;

        // Provide registers for the JIT's prologue.
        let a = self.dmg_state.registers.af.a().into();
        let f = self.dmg_state.registers.af.f().into_bits().into();
        let b = self.dmg_state.registers.bc.b().into();
        let c = self.dmg_state.registers.bc.c().into();
        let d = self.dmg_state.registers.de.d().into();
        let e = self.dmg_state.registers.de.e().into();
        let h = self.dmg_state.registers.hl.h().into();
        let l = self.dmg_state.registers.hl.l().into();
        let sp = self.dmg_state.registers.sp.into();
        let (a, f, b, c, d, e, h, l, sp) =
            call_indirect(compiled_block.func_idx, a, f, b, c, d, e, h, l, sp);
        // Update dmg_state's registers based on the values returned in the JIT's epilogue.
        self.dmg_state.registers.af.set_a(a as u8);
        self.dmg_state.registers.af.set_f(Flags::from_bits(f as u8));
        self.dmg_state.registers.bc.set_b(b as u8);
        self.dmg_state.registers.bc.set_c(c as u8);
        self.dmg_state.registers.de.set_d(d as u8);
        self.dmg_state.registers.de.set_e(e as u8);
        self.dmg_state.registers.hl.set_h(h as u8);
        self.dmg_state.registers.hl.set_l(l as u8);
        self.dmg_state.registers.sp = sp as u16;

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
            ..0x4000 => 0,
            0x4000..0x8000 => self.dmg_state.memory.mbc.current_rom_bank,
            _ => 0,
        };

        CacheAddress {
            bank_number,
            address: pc,
        }
    }

    // Get the next CompiledBlock at PC, either from the cache or by compiling a new block.
    fn get_compiled_block(&mut self) -> Option<CacheAddress> {
        let cache_address = self.current_cache_address();
        if self.block_cache.contains_key(cache_address) {
            Some(cache_address)
        } else {
            let jit_block = codegen::recompile(&mut self.dmg_state, self.ptr)?;
            #[cfg(feature = "jit-trace")]
            console_log(&wasmprinter::print_bytes(&jit_block.buffer).unwrap());

            let ptr = jit_block.buffer.as_ptr();
            let len = jit_block.buffer.len() as u32;
            let func_idx = unsafe { instantiate_and_link_module(ptr, len) };
            let compiled_block = CompiledBlock {
                func_idx,
                checkpoints: jit_block.ctx.checkpoints,
            };

            // Add the block we just compiled to the cache.
            self.block_cache.insert(cache_address, compiled_block);
            Some(cache_address)
        }
    }

    // Check if we can execute compiled_block up to the first checkpoint without being interrupted.
    fn wont_be_interrupted(&self, cache_address: CacheAddress) -> bool {
        let compiled_block = self.block_cache.get(cache_address).unwrap();
        self.dmg_state.memory.clock + compiled_block.checkpoints[0].total_m_cycles as u64 * 4
            <= self.dmg_state.memory.next_interrupt
    }

    // If possible, execute the next JIT-compiled block.
    pub(crate) fn execute(&mut self) {
        let pc = self.dmg_state.registers.pc;
        // Only cache from ROM banks for now.
        if pc < 0x8000
            && let Some(cache_address) = self.get_compiled_block()
            && self.wont_be_interrupted(cache_address)
        {
            self.execute_compiled_block(cache_address);
        } else {
            #[cfg(feature = "log-uncompiled")]
            {
                if pc < 0x8000 {
                    let mut opcode = self.dmg_state.memory.read_byte(pc) as u16;

                    if opcode == 0xCB {
                        let prefix_opcode = self.dmg_state.memory.read_byte(pc + 1);
                        opcode = u16::from_be_bytes([0xCB, prefix_opcode]);
                    }

                    if let Some(count) = self.uncompiled.get_mut(opcode) {
                        *count += 1;
                    } else {
                        self.uncompiled.insert(opcode, 1);
                    }
                }
            }

            // Fallback to interpreter.
            self.dmg_state.execute().unwrap();
        }
    }

    #[cfg(feature = "log-uncompiled")]
    fn log_uncompiled(&self) {
        let mut not_compiled_vec: Vec<(u16, &u32)> = self.uncompiled.iter().collect();
        not_compiled_vec.sort_by(|a, b| b.1.cmp(a.1));
        for (opcode, count) in not_compiled_vec {
            if opcode <= 0xff {
                let opcode = Opcode::decode(opcode as u8);
                console_log(&format!("{opcode:?}: {count}"));
            } else {
                let prefix_opcode = PrefixOpcode::decode((opcode & 0xff) as u8);
                console_log(&format!("{prefix_opcode:?}: {count}"));
            }
        }
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
        let current_block = runtime
            .block_cache
            .get(runtime.currently_executing)
            .unwrap();
        let next_checkpoint = current_block.checkpoints[checkpoint_index as usize + 1];
        let next_checkpoint_clock =
            runtime.block_start_clock + next_checkpoint.total_m_cycles as u64 * 4;

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
        #[cfg(feature = "log-uncompiled")]
        self.log_uncompiled();

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
