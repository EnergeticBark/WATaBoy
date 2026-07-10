pub mod common;

use common::mooneye::{FIBONACCI, run_mooneye_test};

#[macro_export]
macro_rules! mooneye_timer_test {
    ($i:ident, $r:expr) => {
        mooneye_test_pathless!($i, concat!("roms/mooneye/timer/", $r));
    };
}

mooneye_timer_test!(div_write, "div_write.gb");
mooneye_timer_test!(rapid_toggle, "rapid_toggle.gb");
mooneye_timer_test!(tim00_div_trigger, "tim00_div_trigger.gb");
mooneye_timer_test!(tim00, "tim00.gb");
mooneye_timer_test!(tim01_div_trigger, "tim01_div_trigger.gb");
mooneye_timer_test!(tim01, "tim01.gb");
mooneye_timer_test!(tim10_div_trigger, "tim10_div_trigger.gb");
mooneye_timer_test!(tim10, "tim10.gb");
mooneye_timer_test!(tim11_div_trigger, "tim11_div_trigger.gb");
mooneye_timer_test!(tim11, "tim11.gb");
mooneye_timer_test!(tima_reload, "tima_reload.gb");
mooneye_timer_test!(tima_write_reloading, "tima_write_reloading.gb");
mooneye_timer_test!(tma_write_reloading, "tma_write_reloading.gb");
