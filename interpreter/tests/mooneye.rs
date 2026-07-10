pub mod common;

use common::mooneye::{FIBONACCI, run_mooneye_test};

mooneye_test!(add_sp_e_timing, "roms/mooneye/add_sp_e_timing.gb");

// TODO: Implement boot ROM skipping so I can pass this without distributing the original mgb boot ROM.
//mooneye_test!(boot_div_dmg_abc_mgb, "roms/mooneye/boot_div-dmgABCmgb.gb");

// TODO: Fix this test, but only if I can do it without implementing the APU.
//mooneye_test!(boot_hwio_dmg_abc_mgb, "roms/mooneye/boot_hwio-dmgABCmgb.gb");

mooneye_test!(boot_regs_mgb, "roms/mooneye/boot_regs-mgb.gb");
mooneye_test!(call_cc_timing, "roms/mooneye/call_cc_timing.gb");
mooneye_test!(call_cc_timing2, "roms/mooneye/call_cc_timing2.gb");
mooneye_test!(call_timing, "roms/mooneye/call_timing.gb");
mooneye_test!(call_timing2, "roms/mooneye/call_timing2.gb");
mooneye_test!(div_timing, "roms/mooneye/div_timing.gb");
mooneye_test!(oam_dma_restart, "roms/mooneye/oam_dma_restart.gb");
mooneye_test!(oam_dma_start, "roms/mooneye/oam_dma_start.gb");
mooneye_test!(oam_dma_timing, "roms/mooneye/oam_dma_timing.gb");
