pub mod common;

use common::mooneye::{FIBONACCI, run_mooneye_test};

#[macro_export]
macro_rules! mooneye_test {
    ($i:ident, $r:expr) => {
        mooneye_test_pathless!($i, concat!("roms/mooneye/", $r));
    };
}

mooneye_test!(add_sp_e_timing, "add_sp_e_timing.gb");

// TODO: Implement boot ROM skipping so I can pass this without distributing the original mgb boot ROM.
//mooneye_test!(boot_div_dmg_abc_mgb, "boot_div-dmgABCmgb.gb");

// TODO: Fix this test, but only if I can do it without implementing the APU.
//mooneye_test!(boot_hwio_dmg_abc_mgb, "boot_hwio-dmgABCmgb.gb");

mooneye_test!(boot_regs_mgb, "boot_regs-mgb.gb");
mooneye_test!(call_cc_timing, "call_cc_timing.gb");
mooneye_test!(call_cc_timing2, "call_cc_timing2.gb");
mooneye_test!(call_timing, "call_timing.gb");
mooneye_test!(call_timing2, "call_timing2.gb");
mooneye_test!(di_timing_gs, "di_timing-GS.gb");
mooneye_test!(div_timing, "div_timing.gb");
mooneye_test!(intr_timing, "intr_timing.gb");
mooneye_test!(oam_dma_restart, "oam_dma_restart.gb");
mooneye_test!(oam_dma_start, "oam_dma_start.gb");
mooneye_test!(oam_dma_timing, "oam_dma_timing.gb");
