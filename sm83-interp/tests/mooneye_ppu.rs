pub mod common;

use sm83_interp::common::post_boot::PostBoot;
use sm83_interp::cpu::Cpu;
use crate::common::mooneye::{run_mooneye_test, FIBONACCI};

#[test]
fn test_intr_1_2_timing() {
    let mut cpu = Cpu::post_boot_dmg();
    let bcdehl = run_mooneye_test(&mut cpu, include_bytes!("roms/mooneye/ppu/intr_1_2_timing-GS.gb"));

    assert_eq!(bcdehl, FIBONACCI);
}

#[test]
fn test_intr_2_0_timing() {
    let mut cpu = Cpu::post_boot_dmg();
    let bcdehl = run_mooneye_test(&mut cpu, include_bytes!("roms/mooneye/ppu/intr_2_0_timing.gb"));

    assert_eq!(bcdehl, FIBONACCI);
}

#[test]
fn test_stat_irq_blocking() {
    let mut cpu = Cpu::post_boot_dmg();
    let bcdehl = run_mooneye_test(&mut cpu, include_bytes!("roms/mooneye/ppu/stat_irq_blocking.gb"));

    assert_eq!(bcdehl, FIBONACCI);
}