pub mod common;

use common::mooneye::{FIBONACCI, run_mooneye_test};

#[test]
fn test_bits_bank1() {
    let bcdehl = run_mooneye_test(include_bytes!("roms/mooneye/mbc1/bits_bank1.gb"));
    assert_eq!(bcdehl, FIBONACCI);
}

#[test]
fn test_bits_bank2() {
    let bcdehl = run_mooneye_test(include_bytes!("roms/mooneye/mbc1/bits_bank2.gb"));
    assert_eq!(bcdehl, FIBONACCI);
}

#[test]
fn test_bits_mode() {
    let bcdehl = run_mooneye_test(include_bytes!("roms/mooneye/mbc1/bits_mode.gb"));
    assert_eq!(bcdehl, FIBONACCI);
}

#[test]
fn test_bits_ramg() {
    let bcdehl = run_mooneye_test(include_bytes!("roms/mooneye/mbc1/bits_ramg.gb"));
    assert_eq!(bcdehl, FIBONACCI);
}

// TODO: Fix this test, it requires that all reads from SRAM start as FF.
/*#[test]
fn test_ram_64kb() {
    let bcdehl = run_mooneye_test(include_bytes!("roms/mooneye/mbc1/ram_64kb.gb"));
    assert_eq!(bcdehl, FIBONACCI);
}*/
