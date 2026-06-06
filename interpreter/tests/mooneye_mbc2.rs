pub mod common;

use common::mooneye::{FIBONACCI, run_mooneye_test};

#[test]
fn test_bits_ramg() {
    let bcdehl = run_mooneye_test(include_bytes!("roms/mooneye/mbc2/bits_ramg.gb"));
    assert_eq!(bcdehl, FIBONACCI);
}

#[test]
fn test_bits_romb() {
    let bcdehl = run_mooneye_test(include_bytes!("roms/mooneye/mbc2/bits_romb.gb"));
    assert_eq!(bcdehl, FIBONACCI);
}

#[test]
fn test_bits_unused() {
    let bcdehl = run_mooneye_test(include_bytes!("roms/mooneye/mbc2/bits_unused.gb"));
    assert_eq!(bcdehl, FIBONACCI);
}
