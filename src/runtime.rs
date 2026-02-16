use crate::{call_indirect, codegen};

use hw_constants::PostBoot;
use sm83_interp::cpu::Cpu;
use sm83_interp::joypad::ButtonsHeld;
use sm83_interp::registers::Flags;

const TEST_ROM: &[u8; 32768] = include_bytes!("../09-op r,r.gb");
/*const TEST_ROM: &[u8; 1048576] =
include_bytes!("../Pokemon - Blue Version (USA, Europe) (SGB Enhanced).sgb");*/

unsafe extern "C" {
    // Compiles and instantiates a Wasm module using the bytecode in `buffer`, then adds its function to table 1 of *this* module.
    // Wasm-bindgen uses table 0 for doing Rust <-> JavaScript communication, so we use table 1 to avoid stepping on its toes.
    // Returns the index of the newly linked function in table 1.
    fn instantiate_and_link_module(buffer: *const u8, len: u32) -> i32;
}

pub struct JitRuntime {
    dmg_state: Cpu,
}

impl JitRuntime {
    fn execute_cached_block(&mut self, func_idx: i32) {
        let a = self.dmg_state.registers.af.a().into();
        let f = self.dmg_state.registers.af.f().into_bits().into();
        let b = self.dmg_state.registers.bc.b().into();
        let c = self.dmg_state.registers.bc.c().into();
        let d = self.dmg_state.registers.de.d().into();
        let e = self.dmg_state.registers.de.e().into();
        let h = self.dmg_state.registers.hl.h().into();
        let l = self.dmg_state.registers.hl.l().into();
        let (a, f, b, c, d, e, h, l) = call_indirect(func_idx, a, f, b, c, d, e, h, l);
        self.dmg_state.registers.af.set_a(a as u8);
        self.dmg_state.registers.af.set_f(Flags::from_bits(f as u8));
        self.dmg_state.registers.bc.set_b(b as u8);
        self.dmg_state.registers.bc.set_c(c as u8);
        self.dmg_state.registers.de.set_d(d as u8);
        self.dmg_state.registers.de.set_e(e as u8);
        self.dmg_state.registers.hl.set_h(h as u8);
        self.dmg_state.registers.hl.set_l(l as u8);
    }

    // TODO: Checks whether the PC points to the start of a cached, JIT-compiled block.
    // If so, it executes it. Otherwise, it calls recompile(&Cpu) to JIT and cache a block.
    // If neither of these are possible for some reason, it will just call the interpreter’s execute function.
    fn execute(&mut self) {
        if let Some(jit_block) = codegen::recompile(&mut self.dmg_state) {
            let ptr = jit_block.buffer.as_ptr();
            let len = jit_block.buffer.len() as u32;
            let func_idx = unsafe { instantiate_and_link_module(ptr, len) };

            self.execute_cached_block(func_idx);

            // Eventually move `total_pc_count` to a struct with the func_idx so it can be cached alongside it.
            self.dmg_state.registers.pc += jit_block.pc_delta;
        } else {
            // Fallback to interpreter.
            self.dmg_state.execute().unwrap();
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn make_runtime() -> *const JitRuntime {
    let runtime = Box::new(JitRuntime {
        dmg_state: {
            let mut cpu = Cpu::post_boot_dmg();
            cpu.memory.load_rom(TEST_ROM);
            cpu
        },
    });
    // Leak the JitRuntime and return its pointer to the embedder.
    Box::into_raw(runtime)
}

#[unsafe(no_mangle)]
pub extern "C" fn step_vblank(runtime: &mut JitRuntime) {
    loop {
        let ly_before_vblank = runtime.dmg_state.memory.ppu.ly() == 143;
        runtime.execute();
        if ly_before_vblank && runtime.dmg_state.memory.ppu.ly() == 144 {
            return;
        }
    }
}

// TODO: Figure out a nice way to pass C structs across runtime boundaries without resorting to wasm-bindgen.
#[unsafe(no_mangle)]
pub extern "C" fn update_joypad(
    runtime: &mut JitRuntime,
    start: bool,
    select: bool,
    b: bool,
    a: bool,
    down: bool,
    up: bool,
    left: bool,
    right: bool,
) {
    runtime.dmg_state.memory.buttons_held = ButtonsHeld {
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
