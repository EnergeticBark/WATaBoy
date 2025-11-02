use std::fs;
use egui::DroppedFile;
use sm83_interp::cpu::Cpu;

#[cfg(not(target_arch = "wasm32"))]
pub fn handle_dropped_rom(file: DroppedFile, dmg_state: &mut Cpu) {
    let path = file.path.unwrap();
    let rom = fs::read(path).expect("failed to read file");

    dmg_state.memory.load_rom(&rom);
}


#[cfg(target_arch = "wasm32")]
pub fn handle_dropped_rom(file: DroppedFile, dmg_state: &mut Cpu) {
    let bytes = file.bytes.unwrap();

    dmg_state.memory.load_rom(&bytes);
}