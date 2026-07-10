pub mod common;

use common::mooneye::{FIBONACCI, run_mooneye_test};

#[macro_export]
macro_rules! mooneye_oam_test {
    ($i:ident, $r:expr) => {
        mooneye_test_pathless!($i, concat!("roms/mooneye/oam/", $r));
    };
}

mooneye_oam_test!(basic, "basic.gb");
mooneye_oam_test!(reg_read, "reg_read.gb");
mooneye_oam_test!(sources_gs, "sources-GS.gb");
