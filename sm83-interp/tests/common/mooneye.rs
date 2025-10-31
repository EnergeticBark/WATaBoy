use sm83_interp::cpu::Cpu;
use sm83_interp::opcodes;
use sm83_interp::opcodes::Opcode;
use sm83_interp::parameters::R8;

pub fn read_bcdehl(cpu: &Cpu) -> [u8; 6] {
    let regs = &cpu.registers;
    [
        regs.bc.b(),
        regs.bc.c(),
        regs.de.d(),
        regs.de.e(),
        regs.hl.h(),
        regs.hl.l(),
    ]
}

pub fn execute_until_ld_b_b(cpu: &mut Cpu) {
    loop {
        let next_byte = cpu.memory.buffer[cpu.registers.pc as usize];
        if let Ok(Opcode::LdRR { x: R8::B, y: R8::B }) = opcodes::decode(next_byte) {
            break;
        }

        cpu.execute().unwrap();
        cpu.handle_interrupts();
    }
}
