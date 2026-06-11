use egui::DroppedFile;
use interpreter::cpu::Cpu;

use std::fs;

#[cfg(not(target_arch = "wasm32"))]
pub fn handle_dropped_rom(file: DroppedFile, dmg_state: &mut Cpu) {
    let mut path = file.path.unwrap();
    let rom = fs::read(&path).expect("Failed to read the ROM file");

    dmg_state.memory.load_rom(&rom);

    path.set_extension("sav");
    if let Ok(sram) = fs::read(&path) {
        dmg_state.memory.load_sram(&sram);
    } else {
        println!("Couldn't read a save file at {}", path.display());
    };
}

#[cfg(target_arch = "wasm32")]
pub fn handle_dropped_rom(file: DroppedFile, dmg_state: &mut Cpu) {
    let bytes = file.bytes.unwrap();

    dmg_state.memory.load_rom(&bytes);
}
