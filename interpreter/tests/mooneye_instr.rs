pub mod common;

use common::mooneye::{FIBONACCI, run_mooneye_test};

#[macro_export]
macro_rules! mooneye_instr_test {
    ($i:ident, $r:expr) => {
        mooneye_test_pathless!($i, concat!("roms/mooneye/instr/", $r));
    };
}

mooneye_instr_test!(daa, "daa.gb");
