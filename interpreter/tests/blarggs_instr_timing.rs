pub mod common;

use common::blarggs::{BlarggTest, run_blargg_test};

#[test]
fn test_instr_timing() {
    let lines = run_blargg_test(&BlarggTest {
        rom: include_bytes!("roms/blarggs/instr_timing/instr_timing.gb"),
        final_pc: 0xC8B0,
    });
    assert!(lines[3].starts_with("Passed"));
}
