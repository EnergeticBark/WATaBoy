use crate::registers::Registers;
use crate::opcodes;

const MEM_MAP_SIZE: usize = u16::MAX as usize;

struct Cpu {
    registers: Registers,
    memory: [u8; MEM_MAP_SIZE],
}

impl Cpu {
    fn load_boot_rom(&mut self) {
        let agb0_boot_rom = include_bytes!("../agb0.bin");
        self.memory[0..agb0_boot_rom.len()].copy_from_slice(agb0_boot_rom);
    }

    fn execute(&mut self) {
        let pc = self.registers.pc;
        let bytecode = self.memory[pc as usize];
        let opcode = opcodes::decode(bytecode).unwrap();

        use opcodes::Opcode::*;
        match opcode {
            LdRrNn { x} => {
                let next_two_bytes = u16::from_le_bytes([
                    self.memory[pc as usize + 1],
                    self.memory[pc as usize + 2],
                ]);
                *self.registers.r16_mut(x) = next_two_bytes;

                self.registers.pc += 3;
            },

            _ => panic!("uh oh"),
        }
    }
}

impl Default for Cpu {
    fn default() -> Self {
        Self {
            registers: Default::default(),
            memory: [0; MEM_MAP_SIZE],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_bootrom_instruction() {
        let mut cpu = Cpu::default();
        cpu.load_boot_rom();
        cpu.execute();
        assert_eq!(cpu.registers.sp, 0xFFFE);
    }
}