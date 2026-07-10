pub mod common;

use common::mooneye::{FIBONACCI, run_mooneye_test};

#[macro_export]
macro_rules! mooneye_mbc5_test {
    ($i:ident, $r:expr) => {
        mooneye_test_pathless!($i, concat!("roms/mooneye/mbc5/", $r));
    };
}

mooneye_mbc5_test!(rom_512kb, "rom_512kb.gb");
mooneye_mbc5_test!(rom_1mb, "rom_1Mb.gb");
mooneye_mbc5_test!(rom_2mb, "rom_2Mb.gb");
mooneye_mbc5_test!(rom_4mb, "rom_4Mb.gb");
mooneye_mbc5_test!(rom_8mb, "rom_8Mb.gb");
mooneye_mbc5_test!(rom_16mb, "rom_16Mb.gb");
mooneye_mbc5_test!(rom_32mb, "rom_32Mb.gb");
mooneye_mbc5_test!(rom_64mb, "rom_64Mb.gb");
