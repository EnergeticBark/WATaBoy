mod common;

use sm83_interp::cpu::Cpu;
use crate::common::run_test_rom;

struct BlarggTest {
    rom: &'static [u8],
    /* Blargg's test roms will put themselves in an infinite loop after passing or failing.
      We stop and run our assertions once the program counter reaches this looping instruction.
      [We probably want to implement a timeout at some point.]
    */
    final_pc: u16,
}

const SPECIAL_TEST: BlarggTest = BlarggTest {
    rom: include_bytes!("./roms/01-special.gb"),
    final_pc: 0xC7D2,
};
const INTERRUPTS_TEST: BlarggTest = BlarggTest {
    rom: include_bytes!("./roms/02-interrupts.gb"),
    final_pc: 0xC7F4,
};
const OP_SP_HL_TEST: BlarggTest = BlarggTest {
    rom: include_bytes!("./roms/03-op sp,hl.gb"),
    final_pc: 0xCB44,
};

#[test]
fn test_01_special() {
    let mut cpu = Cpu::default();
    let lines = run_test_rom(&mut cpu, &SPECIAL_TEST);
    assert!(lines[2].starts_with("Passed"));
}

#[test]
fn test_02_interrupts() {
    let mut cpu = Cpu::default();
    let lines = run_test_rom(&mut cpu, &INTERRUPTS_TEST);
    assert!(lines[2].starts_with("Passed"));
}

#[test]
fn test_03_op_sp_hl() {
    let mut cpu = Cpu::default();
    let lines = run_test_rom(&mut cpu, &OP_SP_HL_TEST);
    assert!(lines[2].starts_with("Passed"));
}
