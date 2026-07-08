pub mod common;

use common::mooneye::{FIBONACCI, run_mooneye_test};

#[test]
fn test_add_sp_e_timing() {
    let bcdehl = run_mooneye_test(include_bytes!("roms/mooneye/add_sp_e_timing.gb"));
    assert_eq!(bcdehl, FIBONACCI);
}

// TODO: Implement boot ROM skipping so I can pass this without distributing the original mgb boot ROM.
/*#[test]
fn test_boot_div_dmg_abc_mgb() {
    let bcdehl = run_mooneye_test(include_bytes!("roms/mooneye/boot_div-dmgABCmgb.gb"));
    assert_eq!(bcdehl, FIBONACCI);
}*/

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

#[test]
fn test_oam_dma_restart() {
    let bcdehl = run_mooneye_test(include_bytes!("roms/mooneye/oam_dma_restart.gb"));
    assert_eq!(bcdehl, FIBONACCI);
}

#[test]
fn test_oam_dma_start() {
    let bcdehl = run_mooneye_test(include_bytes!("roms/mooneye/oam_dma_start.gb"));
    assert_eq!(bcdehl, FIBONACCI);
}

#[test]
fn test_oam_dma_timing() {
    let bcdehl = run_mooneye_test(include_bytes!("roms/mooneye/oam_dma_timing.gb"));
    assert_eq!(bcdehl, FIBONACCI);
}
