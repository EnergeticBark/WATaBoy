pub mod common;

use sm83_interp::common::post_boot::PostBoot;
use sm83_interp::cpu::Cpu;
use crate::common::mooneye::{run_mooneye_test, FIBONACCI};

#[test]
fn test_basic() {
    let mut cpu = Cpu::post_boot_dmg();
    let bcdehl = run_mooneye_test(&mut cpu, include_bytes!("roms/mooneye/oam/basic.gb"));

    assert_eq!(bcdehl, FIBONACCI);
}

#[test]
fn test_reg_read() {
    let mut cpu = Cpu::post_boot_dmg();
    let bcdehl = run_mooneye_test(&mut cpu, include_bytes!("roms/mooneye/oam/reg_read.gb"));

    assert_eq!(bcdehl, FIBONACCI);
}