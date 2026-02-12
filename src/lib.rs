#![feature(asm_experimental_arch)]

use std::arch::asm;

mod codegen;
pub mod runtime;

#[unsafe(no_mangle)]
pub fn call_indirect(index: i32) -> i32 {
    let mut a: i32 = 100;
    let mut f: i32 = 50;
    let mut b: i32 = 75;
    let mut c: i32 = 0;
    let mut d: i32 = 0;
    let mut e: i32 = 0;
    let mut h: i32 = 0;
    let mut l: i32 = 0;
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
    a
}
