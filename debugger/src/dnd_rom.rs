use egui::DroppedFile;
use interpreter::cpu::Cpu;
use std::fs;

use crate::DebuggerApp;

pub fn handle_dropped_rom(app_state: &mut DebuggerApp, file: DroppedFile) {
    // Save SRAM of previous ROM before loading the next ROM.
    app_state.sav_to_disk();

    app_state.dmg_state = Cpu::default();

    let mut path = file.path.unwrap();
    let rom = fs::read(&path).expect("Failed to read the ROM file");

    app_state.dmg_state.memory.load_rom(&rom);

    path.set_extension("sav");
    app_state.sav_path = Some(path.clone());
    if let Ok(sram) = fs::read(&path) {
        app_state.dmg_state.memory.load_sram(&sram);
    } else {
        println!("Couldn't read a save file at {}", path.display());
    }
}
