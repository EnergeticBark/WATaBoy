pub mod common;

use crate::common::mooneye::{FIBONACCI, run_mooneye_test};

#[test]
fn test_div_write() {
    let bcdehl = run_mooneye_test(include_bytes!("roms/mooneye/timer/div_write.gb"));
    assert_eq!(bcdehl, FIBONACCI);
}

#[test]
fn test_rapid_toggle() {
    let bcdehl = run_mooneye_test(include_bytes!("roms/mooneye/timer/rapid_toggle.gb"));
    assert_eq!(bcdehl, FIBONACCI);
}

#[test]
fn test_tim00_div_trigger() {
    let bcdehl = run_mooneye_test(include_bytes!("roms/mooneye/timer/tim00_div_trigger.gb"));
    assert_eq!(bcdehl, FIBONACCI);
}

#[test]
fn test_tim00() {
    let bcdehl = run_mooneye_test(include_bytes!("roms/mooneye/timer/tim00.gb"));
    assert_eq!(bcdehl, FIBONACCI);
}

#[test]
fn test_tim01_div_trigger() {
    let bcdehl = run_mooneye_test(include_bytes!("roms/mooneye/timer/tim01_div_trigger.gb"));
    assert_eq!(bcdehl, FIBONACCI);
}

#[test]
fn test_tim01() {
    let bcdehl = run_mooneye_test(include_bytes!("roms/mooneye/timer/tim01.gb"));
    assert_eq!(bcdehl, FIBONACCI);
}

#[test]
fn test_tim10_div_trigger() {
    let bcdehl = run_mooneye_test(include_bytes!("roms/mooneye/timer/tim10_div_trigger.gb"));
    assert_eq!(bcdehl, FIBONACCI);
}

#[test]
fn test_tim10() {
    let bcdehl = run_mooneye_test(include_bytes!("roms/mooneye/timer/tim10.gb"));
    assert_eq!(bcdehl, FIBONACCI);
}

#[test]
fn test_tim11_div_trigger() {
    let bcdehl = run_mooneye_test(include_bytes!("roms/mooneye/timer/tim11_div_trigger.gb"));
    assert_eq!(bcdehl, FIBONACCI);
}

#[test]
fn test_tim11() {
    let bcdehl = run_mooneye_test(include_bytes!("roms/mooneye/timer/tim11.gb"));
    assert_eq!(bcdehl, FIBONACCI);
}

#[test]
fn test_tima_reload() {
    let bcdehl = run_mooneye_test(include_bytes!("roms/mooneye/timer/tima_reload.gb"));
    assert_eq!(bcdehl, FIBONACCI);
}

#[test]
fn test_tima_write_reloading() {
    let bcdehl = run_mooneye_test(include_bytes!("roms/mooneye/timer/tima_write_reloading.gb"));
    assert_eq!(bcdehl, FIBONACCI);
}
