pub mod common;

use crate::common::mooneye::{FIBONACCI, run_mooneye_test};

use hw_constants::PostBoot;
use sm83_interp::cpu::Cpu;

#[test]
fn test_boot_hwio_dmg_abc_mgb() {
    let mut cpu = Cpu::post_boot_mgb();
    let bcdehl = run_mooneye_test(
        &mut cpu,
        include_bytes!("roms/mooneye/boot_hwio-dmgABCmgb.gb"),
    );

    assert_eq!(bcdehl, FIBONACCI);
}

#[test]
fn test_boot_regs_mgb() {
    let mut cpu = Cpu::post_boot_mgb();
    let bcdehl = run_mooneye_test(&mut cpu, include_bytes!("roms/mooneye/boot_regs-mgb.gb"));

    assert_eq!(bcdehl, FIBONACCI);
}
