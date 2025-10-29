use sm83_interp::cpu::Cpu;
use sm83_interp::registers::Registers;
use crate::BlarggTest;

pub fn read_ascii_from_tile_map(cpu: &Cpu) -> Vec<String> {
    let (lines, _) = cpu.memory.buffer[0x9800..0x9C00].as_chunks::<48>();
    lines.iter()
        .map(|line| str::from_utf8(line))
        .map(|result| result.unwrap().to_owned())
        .collect()
}

pub fn load_test_rom(cpu: &mut Cpu, test_rom: &[u8]) {
    cpu.memory[0..0x8000].copy_from_slice(&test_rom[0..0x8000]);
    cpu.registers = Registers::after_boot_rom_dmg();
}

pub fn execute_until(cpu: &mut Cpu, final_pc: u16) {
    while cpu.registers.pc != final_pc {
        cpu.execute().unwrap();
        cpu.handle_interrupts();
    }
}

pub fn run_test_rom(cpu: &mut Cpu, test: &BlarggTest) -> Vec<String> {
    load_test_rom(cpu, test.rom);
    execute_until(cpu, test.final_pc);

    read_ascii_from_tile_map(cpu)
}