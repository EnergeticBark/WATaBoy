pub mod common;

use common::mooneye::{FIBONACCI, run_mooneye_test};

#[macro_export]
macro_rules! mooneye_mbc2_test {
    ($i:ident, $r:expr) => {
        mooneye_test_pathless!($i, concat!("roms/mooneye/mbc2/", $r));
    };
}

mooneye_mbc2_test!(bits_ramg, "bits_ramg.gb");
mooneye_mbc2_test!(bits_romb, "bits_romb.gb");
mooneye_mbc2_test!(bits_unused, "bits_unused.gb");
mooneye_mbc2_test!(ram, "ram.gb");
