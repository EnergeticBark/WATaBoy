pub mod common;

use common::blarggs::{BlarggTest, run_blargg_test};

#[test]
fn test_01_special() {
    let lines = run_blargg_test(&BlarggTest {
        rom: include_bytes!("roms/blarggs/cpu_instrs/01-special.gb"),
        final_pc: 0xC7D2,
    });
    assert!(lines[3].starts_with("Passed"));
}

#[test]
fn test_02_interrupts() {
    let lines = run_blargg_test(&BlarggTest {
        rom: include_bytes!("roms/blarggs/cpu_instrs/02-interrupts.gb"),
        final_pc: 0xC7F4,
    });
    assert!(lines[3].starts_with("Passed"));
}

#[test]
fn test_03_op_sp_hl() {
    let lines = run_blargg_test(&BlarggTest {
        rom: include_bytes!("roms/blarggs/cpu_instrs/03-op sp,hl.gb"),
        final_pc: 0xCB44,
    });
    assert!(lines[3].starts_with("Passed"));
}

#[test]
fn test_04_op_r_imm() {
    let lines = run_blargg_test(&BlarggTest {
        rom: include_bytes!("roms/blarggs/cpu_instrs/04-op r,imm.gb"),
        final_pc: 0xCB35,
    });
    assert!(lines[3].starts_with("Passed"));
}

#[test]
fn test_05_op_rp() {
    let lines = run_blargg_test(&BlarggTest {
        rom: include_bytes!("roms/blarggs/cpu_instrs/05-op rp.gb"),
        final_pc: 0xCB31,
    });
    assert!(lines[3].starts_with("Passed"));
}

#[test]
fn test_06_ld_r_r() {
    let lines = run_blargg_test(&BlarggTest {
        rom: include_bytes!("roms/blarggs/cpu_instrs/06-ld r,r.gb"),
        final_pc: 0xCC5F,
    });
    assert!(lines[3].starts_with("Passed"));
}

#[test]
fn test_07_jr_jp_call_ret_rst() {
    let lines = run_blargg_test(&BlarggTest {
        rom: include_bytes!("roms/blarggs/cpu_instrs/07-jr,jp,call,ret,rst.gb"),
        final_pc: 0xCBB0,
    });
    // This test's name is so long that "Passed" ends up on line 4 :)
    assert!(lines[4].starts_with("Passed"));
}

#[test]
fn test_08_misc_instrs() {
    let lines = run_blargg_test(&BlarggTest {
        rom: include_bytes!("roms/blarggs/cpu_instrs/08-misc instrs.gb"),
        final_pc: 0xCB91,
    });
    assert!(lines[3].starts_with("Passed"));
}

#[test]
fn test_09_op_r_r() {
    let lines = run_blargg_test(&BlarggTest {
        rom: include_bytes!("roms/blarggs/cpu_instrs/09-op r,r.gb"),
        final_pc: 0xCE67,
    });
    assert!(lines[3].starts_with("Passed"));
}

#[test]
fn test_10_bit_ops() {
    let lines = run_blargg_test(&BlarggTest {
        rom: include_bytes!("roms/blarggs/cpu_instrs/10-bit ops.gb"),
        final_pc: 0xCF58,
    });
    assert!(lines[3].starts_with("Passed"));
}

#[test]
fn test_11_op_a_hl() {
    let lines = run_blargg_test(&BlarggTest {
        rom: include_bytes!("roms/blarggs/cpu_instrs/11-op a,(hl).gb"),
        final_pc: 0xCC62,
    });
    assert!(lines[3].starts_with("Passed"));
}
