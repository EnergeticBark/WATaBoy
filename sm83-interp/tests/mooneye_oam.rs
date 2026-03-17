pub mod common;

use crate::common::mooneye::{FIBONACCI, run_mooneye_test};

#[test]
fn test_basic() {
    let bcdehl = run_mooneye_test(include_bytes!("roms/mooneye/oam/basic.gb"));
    assert_eq!(bcdehl, FIBONACCI);
}

#[test]
fn test_reg_read() {
    let bcdehl = run_mooneye_test(include_bytes!("roms/mooneye/oam/reg_read.gb"));
    assert_eq!(bcdehl, FIBONACCI);
}
