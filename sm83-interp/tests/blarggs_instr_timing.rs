pub mod common;

use common::blarggs::{BlarggTest, run_blargg_test};

const INSTR_TIMING: BlarggTest = BlarggTest {
    rom: include_bytes!("roms/blarggs/instr_timing/instr_timing.gb"),
    final_pc: 0xC8B0,
};

#[test]
fn test_instr_timing() {
    let lines = run_blargg_test(&INSTR_TIMING);
    assert!(lines[3].starts_with("Passed"));
}
