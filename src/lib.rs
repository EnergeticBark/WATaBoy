#![feature(asm_experimental_arch)]

use std::arch::asm;

mod codegen;
pub mod runtime;

unsafe extern "C" {
    fn console_log_glue(buffer: *const u8, len: u32);
}

/// This function is expensive to call, it should have no usages in the codebase unless it's actively being used for debugging.
pub fn console_log(message: &str) {
    unsafe {
        console_log_glue(message.as_ptr(), message.len() as u32);
    }
}

// I might clean this up later, I still haven't determined whether its faster passing the registers in raw or not.
#[allow(clippy::too_many_arguments)]
fn call_indirect(
    index: i32,
    mut a: i32,
    mut f: i32,
    mut b: i32,
    mut c: i32,
    mut d: i32,
    mut e: i32,
    mut h: i32,
    mut l: i32,
) -> (i32, i32, i32, i32, i32, i32, i32, i32) {
    unsafe {
        asm!("local.get {8}",
            "local.get {7}",
            "local.get {6}",
            "local.get {5}",
            "local.get {4}",
            "local.get {3}",
            "local.get {2}",
            "local.get {1}",
            "local.get {0}",
            "call_indirect (i32, i32, i32, i32, i32, i32, i32, i32) -> (i32, i32, i32, i32, i32, i32, i32, i32)",
            "local.set {1}",
            "local.set {2}",
            "local.set {3}",
            "local.set {4}",
            "local.set {5}",
            "local.set {6}",
            "local.set {7}",
            "local.set {8}",
            in(local) index,
            inout(local) l,
            inout(local) h,
            inout(local) e,
            inout(local) d,
            inout(local) c,
            inout(local) b,
            inout(local) f,
            inout(local) a,
        );
    }
    (a, f, b, c, d, e, h, l)
}
