mod common;

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
fn test_bits_bank1() {
    let mut cpu = Cpu::post_boot_dmg();
    let bcdehl = run_mooneye_test(&mut cpu, include_bytes!("roms/mbc1/bits_bank1.gb"));

    assert_eq!(bcdehl, FIBONACCI);
}

#[test]
fn test_bits_bank2() {
    let mut cpu = Cpu::post_boot_dmg();
    let bcdehl = run_mooneye_test(&mut cpu, include_bytes!("roms/mbc1/bits_bank2.gb"));

    assert_eq!(bcdehl, FIBONACCI);
}

#[test]
fn test_bits_mode() {
    let mut cpu = Cpu::post_boot_dmg();
    let bcdehl = run_mooneye_test(&mut cpu, include_bytes!("roms/mbc1/bits_mode.gb"));

    assert_eq!(bcdehl, FIBONACCI);
}

#[test]
fn test_bits_ramg() {
    let mut cpu = Cpu::post_boot_dmg();
    let bcdehl = run_mooneye_test(&mut cpu, include_bytes!("roms/mbc1/bits_ramg.gb"));

    assert_eq!(bcdehl, FIBONACCI);
}

#[test]
fn test_ram_64kb() {
    let mut cpu = Cpu::post_boot_dmg();
    let bcdehl = run_mooneye_test(&mut cpu, include_bytes!("roms/mbc1/ram_64kb.gb"));

    assert_eq!(bcdehl, FIBONACCI);
}