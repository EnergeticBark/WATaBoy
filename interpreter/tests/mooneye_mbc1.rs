pub mod common;

use common::mooneye::{FIBONACCI, run_mooneye_test};

#[macro_export]
macro_rules! mooneye_mbc1_test {
    ($i:ident, $r:expr) => {
        mooneye_test_pathless!($i, concat!("roms/mooneye/mbc1/", $r));
    };
}

mooneye_mbc1_test!(bits_bank1, "bits_bank1.gb");
mooneye_mbc1_test!(bits_bank2, "bits_bank2.gb");
mooneye_mbc1_test!(bits_mode, "bits_mode.gb");
mooneye_mbc1_test!(bits_ramg, "bits_ramg.gb");

// TODO: Fix this test, it requires that all reads from SRAM start as FF.
//mooneye_mbc1_test!(ram_64kb, "ram_64kb.gb");
