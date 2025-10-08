use sm83_interp::cpu::Cpu;
use sm83_interp::opcodes;

fn main() {
    let mut cpu = Cpu::default();
    cpu.load_boot_rom();
    loop {
        let bytecode = cpu.memory[cpu.registers.pc as usize];
        let opcode = opcodes::decode(bytecode).unwrap();
        println!("PC: {}: {:?}", cpu.registers.pc, opcode);
        cpu.execute();
    }
}
