use std::collections::HashMap;

use crate::cache::CompiledBlock;
use crate::{call_indirect, codegen, console_log};

use sm83_interp::cpu::Cpu;
use sm83_interp::cpu::registers::Flags;
use sm83_interp::joypad::ButtonsHeld;

unsafe extern "C" {
    // Compiles and instantiates a Wasm module using the bytecode in `buffer`, then adds its function to table 1 of *this* module.
    // Wasm-bindgen uses table 0 for doing Rust <-> JavaScript communication, so we use table 1 to avoid stepping on its toes.
    // Returns the index of the newly linked function in table 1.
    fn instantiate_and_link_module(buffer: *const u8, len: u32) -> i32;
}

// TODO: Should probably implement PostBoot too/instead...
#[derive(Default)]
pub struct JitRuntime {
    pub(crate) dmg_state: Cpu,
    block_cache: HashMap<u16, CompiledBlock>,
    rom_buffer: Vec<u8>,
    next_vblank: u64,
}

impl JitRuntime {
    fn execute_compiled_block(&mut self, compiled_block: CompiledBlock) {
        // Provide registers for the JIT's prologue.
        let a = self.dmg_state.registers.af.a().into();
        let f = self.dmg_state.registers.af.f().into_bits().into();
        let b = self.dmg_state.registers.bc.b().into();
        let c = self.dmg_state.registers.bc.c().into();
        let d = self.dmg_state.registers.de.d().into();
        let e = self.dmg_state.registers.de.e().into();
        let h = self.dmg_state.registers.hl.h().into();
        let l = self.dmg_state.registers.hl.l().into();
        let (a, f, b, c, d, e, h, l) =
            call_indirect(compiled_block.func_idx, a, f, b, c, d, e, h, l);
        // Update dmg_state's registers based on the values returned in the JIT's epilogue.
        self.dmg_state.registers.af.set_a(a as u8);
        self.dmg_state.registers.af.set_f(Flags::from_bits(f as u8));
        self.dmg_state.registers.bc.set_b(b as u8);
        self.dmg_state.registers.bc.set_c(c as u8);
        self.dmg_state.registers.de.set_d(d as u8);
        self.dmg_state.registers.de.set_e(e as u8);
        self.dmg_state.registers.hl.set_h(h as u8);
        self.dmg_state.registers.hl.set_l(l as u8);

        // Update the program counter.
        self.dmg_state.registers.pc += compiled_block.pc_delta;
    }

    // TODO: Checks whether the PC points to the start of a cached, JIT-compiled block.
    // If so, it executes it. Otherwise, it calls recompile(&Cpu) to JIT and cache a block.
    // If neither of these are possible for some reason, it will just call the interpreter’s execute function.
    pub(crate) fn execute(&mut self) {
        let pc = self.dmg_state.registers.pc;

        if let Some(&compiled_block) = self.block_cache.get(&pc) {
            self.execute_compiled_block(compiled_block);
        }

        if false && let Some(jit_block) = codegen::recompile(&mut self.dmg_state) {
            #[cfg(feature = "jit-trace")]
            console_log(&wasmprinter::print_bytes(&jit_block.buffer).unwrap());

            let ptr = jit_block.buffer.as_ptr();
            let len = jit_block.buffer.len() as u32;
            let func_idx = unsafe { instantiate_and_link_module(ptr, len) };
            let compiled_block = CompiledBlock {
                func_idx,
                pc_delta: jit_block.pc_delta,
            };

            // Add the block we just compiled to the cache.
            #[cfg(feature = "caching")]
            self.block_cache.insert(pc, compiled_block);

            self.execute_compiled_block(compiled_block);
        } else {
            // Fallback to interpreter.
            self.dmg_state.execute().unwrap();
        }
    }
}

impl JitRuntime {
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
}

#[unsafe(no_mangle)]
pub extern "C" fn make_runtime() -> *const JitRuntime {
    let runtime = Box::new(JitRuntime::default());
    // Leak the JitRuntime and return its pointer to the embedder.
    Box::into_raw(runtime)
}
