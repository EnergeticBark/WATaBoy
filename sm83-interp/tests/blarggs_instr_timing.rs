pub mod common;

use sm83_interp::cpu::Cpu;
use crate::common::blarggs::{run_blargg_test, BlarggTest};
use sm83_interp::common::post_boot::PostBoot;

const INSTR_TIMING: BlarggTest = BlarggTest {
    rom: include_bytes!("roms/blarggs/instr_timing/instr_timing.gb"),
    final_pc: 0xC8B0,
};

#[test]
fn test_instr_timing() {
    let mut cpu = Cpu::post_boot_dmg();
    let lines = run_blargg_test(&mut cpu, &INSTR_TIMING);
    assert!(lines[3].starts_with("Passed"));
}