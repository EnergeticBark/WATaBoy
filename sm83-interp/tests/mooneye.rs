pub mod common;

use common::mooneye::{FIBONACCI, run_mooneye_test};

#[test]
fn test_boot_div_dmg_abc_mgb() {
    let bcdehl = run_mooneye_test(include_bytes!("roms/mooneye/boot_div-dmgABCmgb.gb"));
    assert_eq!(bcdehl, FIBONACCI);
}

// TODO: Fix this test, but only if I can do it without implementing the APU.
/*#[test]
fn test_boot_hwio_dmg_abc_mgb() {
    let bcdehl = run_mooneye_test(include_bytes!("roms/mooneye/boot_hwio-dmgABCmgb.gb"));
    assert_eq!(bcdehl, FIBONACCI);
}*/

#[test]
fn test_boot_regs_mgb() {
    let bcdehl = run_mooneye_test(include_bytes!("roms/mooneye/boot_regs-mgb.gb"));
    assert_eq!(bcdehl, FIBONACCI);
}

#[test]
fn test_div_timing() {
    let bcdehl = run_mooneye_test(include_bytes!("roms/mooneye/div_timing.gb"));
    assert_eq!(bcdehl, FIBONACCI);
}
