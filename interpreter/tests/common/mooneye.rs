use interpreter::cpu::Cpu;
use interpreter::cpu::opcodes::Opcode;
use interpreter::cpu::opcodes::parameters::R8;

pub const FIBONACCI: [u8; 6] = [3, 5, 8, 13, 21, 34];
fn read_bcdehl(cpu: &Cpu) -> [u8; 6] {
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

fn execute_until_ld_b_b(cpu: &mut Cpu) {
    loop {
        let next_byte = cpu.memory.read_byte(cpu.registers.pc);
        if let Ok(Opcode::LdRR { x: R8::B, y: R8::B }) = Opcode::decode(next_byte) {
            break;
        }

        cpu.execute().unwrap();
    }
}

#[must_use]
pub fn run_mooneye_test(rom: &[u8]) -> [u8; 6] {
    let mut cpu = Cpu::default();
    cpu.memory.load_rom(rom);
    execute_until_ld_b_b(&mut cpu);

    read_bcdehl(&cpu)
}

#[macro_export]
macro_rules! mooneye_test_pathless {
    ($i:ident, $p:expr) => {
        #[test]
        fn $i() {
            let bcdehl = run_mooneye_test(include_bytes!($p));
            assert_eq!(bcdehl, FIBONACCI);
        }
    };
}
