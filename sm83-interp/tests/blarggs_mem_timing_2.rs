pub mod common;

use sm83_interp::cpu::Cpu;
use crate::common::blarggs::{run_blargg_test, BlarggTest};
use sm83_interp::common::post_boot::PostBoot;

const READ_TIMING_01: BlarggTest = BlarggTest {
    rom: include_bytes!("roms/blarggs/mem_timing_2/rom_singles/01-read_timing.gb"),
    final_pc: 0xC8BE,
};

const WRITE_TIMING_02: BlarggTest = BlarggTest {
    rom: include_bytes!("roms/blarggs/mem_timing_2/rom_singles/02-write_timing.gb"),
    final_pc: 0xC87C,
};

#[test]
fn test_01_read_timing() {
    let mut cpu = Cpu::post_boot_dmg();
    let lines = run_blargg_test(&mut cpu, &READ_TIMING_01);
    assert!(lines[3].starts_with("Passed"));
}

#[test]
fn test_02_read_timing() {
    let mut cpu = Cpu::post_boot_dmg();
    let lines = run_blargg_test(&mut cpu, &WRITE_TIMING_02);
    assert!(lines[3].starts_with("Passed"));
}