#![feature(asm_experimental_arch)]

use std::arch::asm;

mod cache;
mod codegen;
pub mod runtime;
pub mod test_roms;

#[global_allocator]
static TALC: talc::wasm::WasmDynamicTalc = talc::wasm::new_wasm_dynamic_allocator();

unsafe extern "C" {
    fn link_new_module_glue(buffer: *const u8, len: usize) -> i32;

    fn console_log_glue(buffer: *const u8, len: usize);
}

/// Compile and instantiate a new Wasm module using the bytecode in `buffer`, then add its exported function to *this* module's function table.
/// ## Returns
/// The index of the newly linked function.
fn link_new_module(buffer: &[u8]) -> i32 {
    unsafe { link_new_module_glue(buffer.as_ptr(), buffer.len()) }
}

/// Log the UTF-8 contents of `buffer` to the embedder's console.
/// This function is expensive to call, it should have no usages in the codebase unless it's actively being used for debugging.
fn console_log(message: &str) {
    unsafe {
        console_log_glue(message.as_ptr(), message.len());
    }
}

/// Indirectly call the function at `index` in this module's function table.
fn call_indirect(index: i32) {
    unsafe {
        asm!(
            "local.get {0}",
            "call_indirect () -> ()",
            in(local) index,
        );
    }
}
