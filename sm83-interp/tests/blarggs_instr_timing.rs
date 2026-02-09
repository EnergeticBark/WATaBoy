pub mod common;

use crate::common::blarggs::{BlarggTest, run_blargg_test};

use hw_constants::PostBoot;
use sm83_interp::cpu::Cpu;

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
