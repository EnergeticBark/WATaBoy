pub mod common;

use crate::common::mooneye::{FIBONACCI, run_mooneye_test};

#[test]
fn test_div_write() {
    let bcdehl = run_mooneye_test(include_bytes!("roms/mooneye/timer/div_write.gb"));
    assert_eq!(bcdehl, FIBONACCI);
}
