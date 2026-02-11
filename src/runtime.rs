use crate::{call_indirect, codegen};

use sm83_interp::cpu::Cpu;
use sm83_interp::parameters::R8;

unsafe extern "C" {
    // Compiles and instantiates a Wasm module using the bytecode in `buffer`, then adds its function to table 1 of *this* module.
    // Wasm-bindgen uses table 0 for doing Rust <-> JavaScript communication, so we use table 1 to avoid stepping on its toes.
    // Returns the index of the newly linked function in table 1.
    fn instantiate_and_link_module(buffer: *const u8, len: u32) -> i32;
}

struct JitRuntime {
    dmg_state: Cpu,
}

impl JitRuntime {
    // TODO: Checks whether the PC points to the start of a cached, JIT-compiled block.
    // If so, it executes it. Otherwise, it calls recompile(&Cpu) to JIT and cache a block.
    // If neither of these are possible for some reason, it will just call the interpreter’s execute function.
    /*#[unsafe(no_mangle)]
    pub extern "C" fn execute(&mut self) {
        if let Some(jit_block) = codegen::recompile(&mut self.dmg_state) {
            let func_idx = unsafe { instantiate_and_link_module(&jit_block.buffer) };

            call_indirect(func_idx);

            // Update dmg_state's register values to the register values returned by that call.
        } else {
            // Fallback to interpreter.
            self.dmg_state.execute().unwrap();
        }
    }*/
}

#[unsafe(no_mangle)]
pub extern "C" fn test_make_add() -> i32 {
    let bytecode = codegen::generate_add_r(R8::B);

    let ptr = bytecode.as_ptr();
    let len = bytecode.len() as u32;
    let func_idx = unsafe { instantiate_and_link_module(ptr, len) };

    call_indirect(func_idx)
}
