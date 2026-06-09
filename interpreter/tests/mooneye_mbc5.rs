pub mod common;

use common::mooneye::{FIBONACCI, run_mooneye_test};

#[test]
fn test_rom_512kb() {
    let bcdehl = run_mooneye_test(include_bytes!("roms/mooneye/mbc5/rom_512kb.gb"));
    assert_eq!(bcdehl, FIBONACCI);
}

#[test]
fn test_rom_1mb() {
    let bcdehl = run_mooneye_test(include_bytes!("roms/mooneye/mbc5/rom_1Mb.gb"));
    assert_eq!(bcdehl, FIBONACCI);
}

#[test]
fn test_rom_2mb() {
    let bcdehl = run_mooneye_test(include_bytes!("roms/mooneye/mbc5/rom_2Mb.gb"));
    assert_eq!(bcdehl, FIBONACCI);
}

#[test]
fn test_rom_4mb() {
    let bcdehl = run_mooneye_test(include_bytes!("roms/mooneye/mbc5/rom_4Mb.gb"));
    assert_eq!(bcdehl, FIBONACCI);
}

#[test]
fn test_rom_8mb() {
    let bcdehl = run_mooneye_test(include_bytes!("roms/mooneye/mbc5/rom_8Mb.gb"));
    assert_eq!(bcdehl, FIBONACCI);
}

#[test]
fn test_rom_16mb() {
    let bcdehl = run_mooneye_test(include_bytes!("roms/mooneye/mbc5/rom_16Mb.gb"));
    assert_eq!(bcdehl, FIBONACCI);
}

#[test]
fn test_rom_32mb() {
    let bcdehl = run_mooneye_test(include_bytes!("roms/mooneye/mbc5/rom_32Mb.gb"));
    assert_eq!(bcdehl, FIBONACCI);
}

#[test]
fn test_rom_64mb() {
    let bcdehl = run_mooneye_test(include_bytes!("roms/mooneye/mbc5/rom_64Mb.gb"));
    assert_eq!(bcdehl, FIBONACCI);
}
