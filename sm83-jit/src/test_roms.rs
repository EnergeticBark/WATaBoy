use interpreter::addressable::Addressable;

use crate::runtime::JitRuntime;

// Helper functions for automating test ROM execution within a JitRuntime.

fn read_ascii_from_tile_map(runtime: &JitRuntime) -> Vec<String> {
    let lines_buffer: Vec<u8> = (0x9800..0x9C00)
        .map(|i| runtime.dmg_state.memory.ppu.read_byte(i, 0))
        .collect();
    lines_buffer
        .chunks_exact(32)
        .map(str::from_utf8)
        .map(|result| result.unwrap().to_owned())
        .collect()
}

/// Execute loaded Blargg test ROM, callable from JavaScript.
///
/// ## Arguments
/// `passed_line`: which line in the tile map we expect the word "Passed" to be written (usually 3).
///
/// ## Returns
/// If the test passed, `true`, otherwise `false`.
#[unsafe(no_mangle)]
pub extern "C" fn run_blargg_test(runtime: &mut JitRuntime, passed_line: usize) -> bool {
    // This is a totally arbitrary number of execute calls, all that matters is it's enough for the test to finish.
    for _ in 0..10000000 {
        runtime.execute();
    }

    let lines = read_ascii_from_tile_map(runtime);
    lines[passed_line].starts_with("Passed")
}

/// Execute loaded Mooneye test ROM, callable from JavaScript.
///
/// ## Returns
/// If the test passed, `true`, otherwise `false`.
#[unsafe(no_mangle)]
pub extern "C" fn run_mooneye_test(runtime: &mut JitRuntime) -> bool {
    for _ in 0..10000000 {
        runtime.execute();
    }

    let regs = &runtime.dmg_state.registers;
    let bcdehl = [
        regs.bc.b(),
        regs.bc.c(),
        regs.de.d(),
        regs.de.e(),
        regs.hl.h(),
        regs.hl.l(),
    ];

    const FIBONACCI: [u8; 6] = [3, 5, 8, 13, 21, 34];
    bcdehl == FIBONACCI
}
