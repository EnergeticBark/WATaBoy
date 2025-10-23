use std::fs;
use egui::DroppedFile;
use sm83_interp::cpu::Cpu;

#[cfg(not(target_arch = "wasm32"))]
pub fn handle_dropped_rom(file: DroppedFile, dmg_state: &mut Cpu) {
    let path = file.path.unwrap();
    let rom = fs::read(path).expect("failed to read file");

    dmg_state.memory[0..rom.len()].copy_from_slice(&rom);
    dmg_state.registers.pc = 0x0100;
}


#[cfg(target_arch = "wasm32")]
pub fn handle_dropped_rom(file: DroppedFile, dmg_state: &mut Cpu) {
    let bytes = file.bytes.unwrap();

    dmg_state.memory[0..bytes.len()].copy_from_slice(&bytes);
    dmg_state.registers.pc = 0x0100;
}