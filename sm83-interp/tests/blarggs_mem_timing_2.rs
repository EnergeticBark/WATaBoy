pub mod common;

use common::blarggs::{BlarggTest, run_blargg_test};

#[test]
fn test_01_read_timing() {
    let lines = run_blargg_test(&BlarggTest {
        rom: include_bytes!("roms/blarggs/mem_timing_2/rom_singles/01-read_timing.gb"),
        final_pc: 0xC8BE,
    });
    assert!(lines[3].starts_with("Passed"));
}

#[test]
fn test_02_read_timing() {
    let lines = run_blargg_test(&BlarggTest {
        rom: include_bytes!("roms/blarggs/mem_timing_2/rom_singles/02-write_timing.gb"),
        final_pc: 0xC87C,
    });
    assert!(lines[3].starts_with("Passed"));
}

#[test]
fn test_03_modify_timing() {
    let lines = run_blargg_test(&BlarggTest {
        rom: include_bytes!("roms/blarggs/mem_timing_2/rom_singles/03-modify_timing.gb"),
        final_pc: 0xC8E4,
    });
    assert!(lines[3].starts_with("Passed"));
}
