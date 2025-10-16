use sm83_interp::cpu::Cpu;
use sm83_interp::opcodes;

const NINTENDO_LOGO: &[u8; 48] = include_bytes!("../nintendo_logo.bin");

fn main() {
    let mut cpu = Cpu::default();
    cpu.load_boot_rom();

    cpu.memory[0x0104..0x0134].copy_from_slice(NINTENDO_LOGO);
    /* Our rom header is all zeros, so just hardcode the checksum of those zeros to make the bootrom
       happy. See: https://gbdev.io/pandocs/The_Cartridge_Header.html#014d--header-checksum
    */
    cpu.memory[0x014D] = 0xE7;

    loop {
        /* Cycle the LCD Y coordinate so the bootrom doesn't get stuck waiting for a v-blank.
           Once I actually implement the PPU alongside the CPU, I'll want to do this with proper
           timing. See: https://gbdev.io/pandocs/Rendering.html
        */
        cpu.memory[0xFF44] = (cpu.memory[0xFF44] + 1) % 154;
        let bytecode = cpu.memory[cpu.registers.pc as usize];
        let opcode = opcodes::decode(bytecode).unwrap();
        println!("PC: {}: {:?}", cpu.registers.pc, opcode);
        cpu.execute();
    }
}
