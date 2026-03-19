pub mod common;

use common::mooneye::{FIBONACCI, run_mooneye_test};

// TODO: Fix this test...
/*#[test]
fn test_hblank_ly_scx_timing_gs() {
    let bcdehl = run_mooneye_test(include_bytes!(
        "roms/mooneye/ppu/hblank_ly_scx_timing-GS.gb"
    ));
    assert_eq!(bcdehl, FIBONACCI);
}*/

#[test]
fn test_intr_1_2_timing_gs() {
    let bcdehl = run_mooneye_test(include_bytes!("roms/mooneye/ppu/intr_1_2_timing-GS.gb"));
    assert_eq!(bcdehl, FIBONACCI);
}

#[test]
fn test_intr_2_0_timing() {
    let bcdehl = run_mooneye_test(include_bytes!("roms/mooneye/ppu/intr_2_0_timing.gb"));
    assert_eq!(bcdehl, FIBONACCI);
}

#[test]
fn test_intr_2_mode0_timing_sprites() {
    let bcdehl = run_mooneye_test(include_bytes!(
        "roms/mooneye/ppu/intr_2_mode0_timing_sprites.gb"
    ));

    assert_eq!(bcdehl, FIBONACCI);
}

#[test]
fn test_intr_2_mode0_timing() {
    let bcdehl = run_mooneye_test(include_bytes!("roms/mooneye/ppu/intr_2_mode0_timing.gb"));
    assert_eq!(bcdehl, FIBONACCI);
}

#[test]
fn test_intr_2_mode3_timing() {
    let bcdehl = run_mooneye_test(include_bytes!("roms/mooneye/ppu/intr_2_mode3_timing.gb"));
    assert_eq!(bcdehl, FIBONACCI);
}

#[test]
fn test_intr_2_oam_ok_timing() {
    let bcdehl = run_mooneye_test(include_bytes!("roms/mooneye/ppu/intr_2_oam_ok_timing.gb"));
    assert_eq!(bcdehl, FIBONACCI);
}

#[test]
fn test_lcdon_timing_gs() {
    let bcdehl = run_mooneye_test(include_bytes!("roms/mooneye/ppu/lcdon_timing-GS.gb"));
    assert_eq!(bcdehl, FIBONACCI);
}

#[test]
fn test_lcdon_write_timing_gs() {
    let bcdehl = run_mooneye_test(include_bytes!("roms/mooneye/ppu/lcdon_write_timing-GS.gb"));
    assert_eq!(bcdehl, FIBONACCI);
}

#[test]
fn test_stat_irq_blocking() {
    let bcdehl = run_mooneye_test(include_bytes!("roms/mooneye/ppu/stat_irq_blocking.gb"));
    assert_eq!(bcdehl, FIBONACCI);
}

#[test]
fn test_stat_lyc_onoff() {
    let bcdehl = run_mooneye_test(include_bytes!("roms/mooneye/ppu/stat_lyc_onoff.gb"));
    assert_eq!(bcdehl, FIBONACCI);
}

#[test]
fn test_vblank_stat_intr_gs() {
    let bcdehl = run_mooneye_test(include_bytes!("roms/mooneye/ppu/vblank_stat_intr-GS.gb"));
    assert_eq!(bcdehl, FIBONACCI);
}
