use std::fs;
use egui::DroppedFile;
use sm83_interp::cpu::Cpu;
use sm83_interp::registers::Registers;

#[cfg(not(target_arch = "wasm32"))]
pub fn handle_dropped_rom(file: DroppedFile, dmg_state: &mut Cpu) {
    let path = file.path.unwrap();
    let rom = fs::read(path).expect("failed to read file");

    dmg_state.memory[0..0x8000].copy_from_slice(&rom[0..0x8000]);
    dmg_state.registers = Registers::after_boot_rom_dmg();
}


#[cfg(target_arch = "wasm32")]
pub fn handle_dropped_rom(file: DroppedFile, dmg_state: &mut Cpu) {
    let bytes = file.bytes.unwrap();

    dmg_state.memory[0..0x8000].copy_from_slice(&rom[0..0x8000]);
    dmg_state.registers = Registers::after_boot_rom_dmg();
}