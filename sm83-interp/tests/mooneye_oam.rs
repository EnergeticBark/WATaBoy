pub mod common;

use common::mooneye;
use sm83_interp::common::post_boot::PostBoot;
use sm83_interp::cpu::Cpu;

fn run_mooneye_test(cpu: &mut Cpu, rom: &[u8]) -> [u8; 6] {
    cpu.memory.load_rom(rom);
    mooneye::execute_until_ld_b_b(cpu);

    mooneye::read_bcdehl(cpu)
}

const FIBONACCI: [u8; 6] = [3, 5, 8, 13, 21, 34];

#[test]
fn test_basic() {
    let mut cpu = Cpu::post_boot_dmg();
    let bcdehl = run_mooneye_test(&mut cpu, include_bytes!("roms/oam/basic.gb"));

    assert_eq!(bcdehl, FIBONACCI);
}

#[test]
fn test_reg_read() {
    let mut cpu = Cpu::post_boot_dmg();
    let bcdehl = run_mooneye_test(&mut cpu, include_bytes!("roms/oam/reg_read.gb"));

    assert_eq!(bcdehl, FIBONACCI);
}