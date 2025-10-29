mod common;

use sm83_interp::cpu::Cpu;
use sm83_interp::registers::Registers;

const SPECIAL_TEST_ROM: &[u8] = include_bytes!("./roms/01-special.gb");
/* Blargg's test roms will put themselves in an infinite loop after passing or failing.
   We stop and run our assertions once the program counter reaches this looping instruction.
   [We probably want to implement a timeout at some point.]
 */
const FINAL_PC: u16 = 0xC7D2;

#[test]
fn test_01_special() {
    let mut cpu = Cpu::default();
    cpu.memory[0..0x8000].copy_from_slice(&SPECIAL_TEST_ROM[0..0x8000]);
    cpu.registers = Registers::after_boot_rom_dmg();

    while cpu.registers.pc != FINAL_PC {
        cpu.execute().unwrap();
        cpu.handle_interrupts();
    }

    let lines = common::read_ascii_from_tile_map(&cpu);
    assert!(lines[2].starts_with("Passed"));
}