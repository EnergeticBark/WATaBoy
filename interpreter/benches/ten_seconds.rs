use std::fs;
use std::time::{Duration, Instant};

use interpreter::cpu::Cpu;

const DURATION_SECS: u32 = 10;
const CYCLES_PER_FRAME: u32 = 70224;
const ONE_GHZ: f64 = 1e+9;
const GB_REFRESH_RATE: f64 = 59.73;

#[derive(Default)]
pub struct AppState {
    mgb_state: Cpu,
    next_vblank: u64,
}

impl AppState {
    /// Update the Game Boy's state, emulating a frame worth of cycles.
    fn step_vblank(&mut self) {
        self.next_vblank += 70224;
        while self.mgb_state.memory.clock < self.next_vblank {
            self.mgb_state.execute().unwrap();
        }
    }
}

fn run_10_seconds_wall_clock(rom: &[u8]) {
    let mut app_state = AppState::default();
    app_state.mgb_state.memory.load_rom(rom);

    let mut frames_emulated = 0;

    let start_time = Instant::now();
    let end_time = start_time + Duration::from_secs(u64::from(DURATION_SECS));

    while Instant::now() < end_time {
        app_state.step_vblank();
        frames_emulated += 1;
    }

    println!("Frames emulated: {frames_emulated}\n");

    println!(
        "Frames per second: {}\n",
        f64::from(frames_emulated) / f64::from(DURATION_SECS)
    );

    println!(
        "Emulation speed relative to Game Boy: {}x\n",
        f64::from(frames_emulated) / f64::from(DURATION_SECS) / GB_REFRESH_RATE
    );

    println!(
        "Emulated master clock cycles per second: {}GHz\n",
        f64::from(frames_emulated * CYCLES_PER_FRAME) / f64::from(DURATION_SECS) / ONE_GHZ
    );
}

fn main() {
    let path_str = std::env::args().nth(1).expect("no path given");
    let path = std::path::PathBuf::from(path_str);

    let rom = fs::read(path).expect("failed to read file");

    for i in 1..=5 {
        println!("*** RUN {i} ***");
        run_10_seconds_wall_clock(&rom);
        println!();
    }
}
