pub mod common;

use crate::common::mooneye::{FIBONACCI, run_mooneye_test};

use hw_constants::PostBoot;
use sm83_interp::cpu::Cpu;

#[test]
fn test_basic() {
    let mut cpu = Cpu::post_boot_mgb();
    let bcdehl = run_mooneye_test(&mut cpu, include_bytes!("roms/mooneye/oam/basic.gb"));

    assert_eq!(bcdehl, FIBONACCI);
}

#[test]
fn test_reg_read() {
    let mut cpu = Cpu::post_boot_mgb();
    let bcdehl = run_mooneye_test(&mut cpu, include_bytes!("roms/mooneye/oam/reg_read.gb"));

    assert_eq!(bcdehl, FIBONACCI);
}
