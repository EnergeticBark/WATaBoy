pub mod common;

use hw_constants::PostBoot;
use sm83_interp::cpu::Cpu;
use crate::common::blarggs::{run_blargg_test, BlarggTest};

const SPECIAL_01: BlarggTest = BlarggTest {
    rom: include_bytes!("roms/blarggs/cpu_instrs/01-special.gb"),
    final_pc: 0xC7D2,
};
const INTERRUPTS_02: BlarggTest = BlarggTest {
    rom: include_bytes!("roms/blarggs/cpu_instrs/02-interrupts.gb"),
    final_pc: 0xC7F4,
};
const OP_SP_HL_03: BlarggTest = BlarggTest {
    rom: include_bytes!("roms/blarggs/cpu_instrs/03-op sp,hl.gb"),
    final_pc: 0xCB44,
};
const OP_R_IMM_04: BlarggTest = BlarggTest {
    rom: include_bytes!("roms/blarggs/cpu_instrs/04-op r,imm.gb"),
    final_pc: 0xCB35,
};
const OP_RP_05: BlarggTest = BlarggTest {
    rom: include_bytes!("roms/blarggs/cpu_instrs/05-op rp.gb"),
    final_pc: 0xCB31,
};
const LD_R_R_06: BlarggTest = BlarggTest {
    rom: include_bytes!("roms/blarggs/cpu_instrs/06-ld r,r.gb"),
    final_pc: 0xCC5F,
};
const JR_JP_CALL_RET_RST_07: BlarggTest = BlarggTest {
    rom: include_bytes!("roms/blarggs/cpu_instrs/07-jr,jp,call,ret,rst.gb"),
    final_pc: 0xCBB0,
};
const MISC_INSTRS_08: BlarggTest = BlarggTest {
    rom: include_bytes!("roms/blarggs/cpu_instrs/08-misc instrs.gb"),
    final_pc: 0xCB91,
};
const OP_R_R_09: BlarggTest = BlarggTest {
    rom: include_bytes!("roms/blarggs/cpu_instrs/09-op r,r.gb"),
    final_pc: 0xCE67,
};
const BIT_OPS_10: BlarggTest = BlarggTest {
    rom: include_bytes!("roms/blarggs/cpu_instrs/10-bit ops.gb"),
    final_pc: 0xCF58,
};
const OP_A_HL_11: BlarggTest = BlarggTest {
    rom: include_bytes!("roms/blarggs/cpu_instrs/11-op a,(hl).gb"),
    final_pc: 0xCC62,
};

#[test]
fn test_01_special() {
    let mut cpu = Cpu::post_boot_dmg();
    let lines = run_blargg_test(&mut cpu, &SPECIAL_01);
    assert!(lines[3].starts_with("Passed"));
}

#[test]
fn test_02_interrupts() {
    let mut cpu = Cpu::post_boot_dmg();
    let lines = run_blargg_test(&mut cpu, &INTERRUPTS_02);
    assert!(lines[3].starts_with("Passed"));
}

#[test]
fn test_03_op_sp_hl() {
    let mut cpu = Cpu::post_boot_dmg();
    let lines = run_blargg_test(&mut cpu, &OP_SP_HL_03);
    assert!(lines[3].starts_with("Passed"));
}

#[test]
fn test_04_op_r_imm() {
    let mut cpu = Cpu::post_boot_dmg();
    let lines = run_blargg_test(&mut cpu, &OP_R_IMM_04);
    assert!(lines[3].starts_with("Passed"));
}

#[test]
fn test_05_op_rp() {
    let mut cpu = Cpu::post_boot_dmg();
    let lines = run_blargg_test(&mut cpu, &OP_RP_05);
    assert!(lines[3].starts_with("Passed"));
}

#[test]
fn test_06_ld_r_r() {
    let mut cpu = Cpu::post_boot_dmg();
    let lines = run_blargg_test(&mut cpu, &LD_R_R_06);
    assert!(lines[3].starts_with("Passed"));
}

#[test]
fn test_07_jr_jp_call_ret_rst() {
    let mut cpu = Cpu::post_boot_dmg();
    let lines = run_blargg_test(&mut cpu, &JR_JP_CALL_RET_RST_07);
    // This test's name is so long that "Passed" ends up on line 4 :)
    assert!(lines[4].starts_with("Passed"));
}

#[test]
fn test_08_misc_instrs() {
    let mut cpu = Cpu::post_boot_dmg();
    let lines = run_blargg_test(&mut cpu, &MISC_INSTRS_08);
    assert!(lines[3].starts_with("Passed"));
}

#[test]
fn test_09_op_r_r() {
    let mut cpu = Cpu::post_boot_dmg();
    let lines = run_blargg_test(&mut cpu, &OP_R_R_09);
    assert!(lines[3].starts_with("Passed"));
}

#[test]
fn test_10_bit_ops() {
    let mut cpu = Cpu::post_boot_dmg();
    let lines = run_blargg_test(&mut cpu, &BIT_OPS_10);
    assert!(lines[3].starts_with("Passed"));
}

#[test]
fn test_11_op_a_hl() {
    let mut cpu = Cpu::post_boot_dmg();
    let lines = run_blargg_test(&mut cpu, &OP_A_HL_11);
    assert!(lines[3].starts_with("Passed"));
}
