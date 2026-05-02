#![feature(asm_experimental_arch)]

use std::arch::asm;

mod cache;
mod codegen;
pub mod runtime;
pub mod test_roms;

unsafe extern "C" {
    fn console_log_glue(buffer: *const u8, len: usize);
}

/// This function is expensive to call, it should have no usages in the codebase unless it's actively being used for debugging.
pub fn console_log(message: &str) {
    unsafe {
        console_log_glue(message.as_ptr(), message.len());
    }
}

fn call_indirect(index: i32) {
    unsafe {
        asm!(
            "local.get {0}",
            "call_indirect () -> ()",
            in(local) index,
        );
    }
}
