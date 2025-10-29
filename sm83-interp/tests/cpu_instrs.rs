mod common;

use sm83_interp::cpu::Cpu;

struct BlarggTest {
    rom: &'static [u8],
    /* Blargg's test roms will put themselves in an infinite loop after passing or failing.
      We stop and run our assertions once the program counter reaches this looping instruction.
      [We probably want to implement a timeout at some point.]
    */
    final_pc: u16,
}

fn run_blargg_test(cpu: &mut Cpu, test: &BlarggTest) -> Vec<String> {
    common::load_test_rom(cpu, test.rom);
    common::execute_until(cpu, test.final_pc);

    common::read_ascii_from_tile_map(cpu)
}

const SPECIAL_01: BlarggTest = BlarggTest {
    rom: include_bytes!("./roms/01-special.gb"),
    final_pc: 0xC7D2,
};
const INTERRUPTS_02: BlarggTest = BlarggTest {
    rom: include_bytes!("./roms/02-interrupts.gb"),
    final_pc: 0xC7F4,
};
const OP_SP_HL_03: BlarggTest = BlarggTest {
    rom: include_bytes!("./roms/03-op sp,hl.gb"),
    final_pc: 0xCB44,
};
const OP_R_IMM_04: BlarggTest = BlarggTest {
    rom: include_bytes!("./roms/04-op r,imm.gb"),
    final_pc: 0xCB35,
};
const OP_RP_05: BlarggTest = BlarggTest {
    rom: include_bytes!("./roms/05-op rp.gb"),
    final_pc: 0xCB31,
};
const LD_R_R_06: BlarggTest = BlarggTest {
    rom: include_bytes!("./roms/06-ld r,r.gb"),
    final_pc: 0xCC5F,
};
const JR_JP_CALL_RET_RST_07: BlarggTest = BlarggTest {
    rom: include_bytes!("./roms/07-jr,jp,call,ret,rst.gb"),
    final_pc: 0xCBB0,
};
const MISC_INSTRS_08: BlarggTest = BlarggTest {
    rom: include_bytes!("./roms/08-misc instrs.gb"),
    final_pc: 0xCB91,
};

#[test]
fn test_01_special() {
    let mut cpu = Cpu::default();
    let lines = run_blargg_test(&mut cpu, &SPECIAL_01);
    assert!(lines[3].starts_with("Passed"));
}

#[test]
fn test_02_interrupts() {
    let mut cpu = Cpu::default();
    let lines = run_blargg_test(&mut cpu, &INTERRUPTS_02);
    assert!(lines[3].starts_with("Passed"));
}

#[test]
fn test_03_op_sp_hl() {
    let mut cpu = Cpu::default();
    let lines = run_blargg_test(&mut cpu, &OP_SP_HL_03);
    assert!(lines[3].starts_with("Passed"));
}

#[test]
fn test_04_op_r_imm() {
    let mut cpu = Cpu::default();
    let lines = run_blargg_test(&mut cpu, &OP_R_IMM_04);
    assert!(lines[3].starts_with("Passed"));
}

#[test]
fn test_05_op_rp() {
    let mut cpu = Cpu::default();
    let lines = run_blargg_test(&mut cpu, &OP_RP_05);
    assert!(lines[3].starts_with("Passed"));
}

#[test]
fn test_06_ld_r_r() {
    let mut cpu = Cpu::default();
    let lines = run_blargg_test(&mut cpu, &LD_R_R_06);
    assert!(lines[3].starts_with("Passed"));
}

#[test]
fn test_07_jr_jp_call_ret_rst() {
    let mut cpu = Cpu::default();
    let lines = run_blargg_test(&mut cpu, &JR_JP_CALL_RET_RST_07);
    // This test's name is so long that "Passed" ends up on line 4 :)
    assert!(lines[4].starts_with("Passed"));
}

#[test]
fn test_08_misc_instrs() {
    let mut cpu = Cpu::default();
    let lines = run_blargg_test(&mut cpu, &MISC_INSTRS_08);
    assert!(lines[3].starts_with("Passed"));
}