pub mod common;

use common::mooneye::{FIBONACCI, run_mooneye_test};

#[macro_export]
macro_rules! mooneye_ppu_test {
    ($i:ident, $r:expr) => {
        mooneye_test_pathless!($i, concat!("roms/mooneye/ppu/", $r));
    };
}

mooneye_ppu_test!(hblank_ly_scx_timing_gs, "hblank_ly_scx_timing-GS.gb");
mooneye_ppu_test!(intr_1_2_timing_gs, "intr_1_2_timing-GS.gb");
mooneye_ppu_test!(intr_2_0_timing, "intr_2_0_timing.gb");
mooneye_ppu_test!(
    intr_2_mode0_timing_sprites,
    "intr_2_mode0_timing_sprites.gb"
);
mooneye_ppu_test!(intr_2_mode0_timing, "intr_2_mode0_timing.gb");
mooneye_ppu_test!(intr_2_mode3_timing, "intr_2_mode3_timing.gb");
mooneye_ppu_test!(intr_2_oam_ok_timing, "intr_2_oam_ok_timing.gb");
mooneye_ppu_test!(lcdon_timing_gs, "lcdon_timing-GS.gb");
mooneye_ppu_test!(lcdon_write_timing_gs, "lcdon_write_timing-GS.gb");
mooneye_ppu_test!(stat_irq_blocking, "stat_irq_blocking.gb");
mooneye_ppu_test!(stat_lyc_onoff, "stat_lyc_onoff.gb");
mooneye_ppu_test!(vblank_stat_intr_gs, "vblank_stat_intr-GS.gb");
