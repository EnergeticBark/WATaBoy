use crate::parameters::{Condition, R8, R16, R16Mem};
use crate::registers::Registers;
use crate::{opcodes, registers};
use crate::opcodes::Opcode::{LdNnA, SubN};

const DMG_BOOT_ROM: &[u8] = include_bytes!("../dmg.bin");
const MEM_MAP_SIZE: usize = 0x10000;

pub struct Cpu {
    pub registers: Registers,
    pub memory: [u8; MEM_MAP_SIZE],
    pub ime: bool, // Interrupt master enable flag
}

impl Cpu {
    pub(crate) fn r8(&self, r8: R8) -> u8 {
        match r8 {
            R8::B => self.registers.bc.b(),
            R8::C => self.registers.bc.c(),
            R8::D => self.registers.de.d(),
            R8::E => self.registers.de.e(),
            R8::H => self.registers.hl.h(),
            R8::L => self.registers.hl.l(),
            R8::IndirectHL => self.memory[self.registers.hl.into_bits() as usize],
            R8::A => self.registers.af.a(),
        }
    }

    pub(crate) fn set_r8(&mut self, r8: R8, value: u8) {
        match r8 {
            R8::B => self.registers.bc.set_b(value),
            R8::C => self.registers.bc.set_c(value),
            R8::D => self.registers.de.set_d(value),
            R8::E => self.registers.de.set_e(value),
            R8::H => self.registers.hl.set_h(value),
            R8::L => self.registers.hl.set_l(value),
            R8::IndirectHL => self.memory[self.registers.hl.into_bits() as usize] = value,
            R8::A => self.registers.af.set_a(value),
        }
    }

    pub(crate) fn r16_mem(&mut self, r16_mem: R16Mem) -> u8 {
        match r16_mem {
            R16Mem::Bc => self.memory[self.registers.bc.into_bits() as usize],
            R16Mem::De => self.memory[self.registers.de.into_bits() as usize],
            R16Mem::HlInc => {
                let value = self.memory[self.registers.hl.into_bits() as usize];
                self.registers.hl = registers::Hl::from_bits(self.registers.hl.into_bits().wrapping_add(1));
                value
            }
            R16Mem::HlDec => {
                let value = self.memory[self.registers.hl.into_bits() as usize];
                self.registers.hl = registers::Hl::from_bits(self.registers.hl.into_bits().wrapping_sub(1));
                value
            }
        }
    }

    pub(crate) fn set_r16_mem(&mut self, r16_mem: R16Mem, value: u8) {
        match r16_mem {
            R16Mem::Bc => self.memory[self.registers.bc.into_bits() as usize] = value,
            R16Mem::De => self.memory[self.registers.de.into_bits() as usize] = value,
            R16Mem::HlInc => {
                self.memory[self.registers.hl.into_bits() as usize] = value;
                self.registers.hl = registers::Hl::from_bits(self.registers.hl.into_bits().wrapping_add(1));
            }
            R16Mem::HlDec => {
                self.memory[self.registers.hl.into_bits() as usize] = value;
                self.registers.hl = registers::Hl::from_bits(self.registers.hl.into_bits().wrapping_sub(1));
            }
        }
    }

    fn check_condition(&self, condition: Condition) -> bool {
        use Condition::*;
        let flags = self.registers.af.f();
        match condition {
            Nz => !flags.z(),
            Z => flags.z(),
            Nc => !flags.c(),
            C => flags.c(),
        }
    }

    pub fn load_boot_rom(&mut self) {
        self.memory[0..DMG_BOOT_ROM.len()].copy_from_slice(DMG_BOOT_ROM);
    }

    /// # Errors
    ///
    /// Will return an error if the instruction at the current program counter is unimplemented.
    #[allow(clippy::too_many_lines)]
    pub fn execute(&mut self) -> Result<(), String> {
        use opcodes::Opcode::*;

        let pc = self.registers.pc;
        let bytecode = self.memory[pc as usize];
        let opcode = opcodes::decode(bytecode)?;

        match opcode {
            // Block 0
            Nop => {
                self.registers.pc += 1;
            }
            LdRrNn { x } => {
                let next_two_bytes = u16::from_le_bytes([
                    self.memory[pc as usize + 1],
                    self.memory[pc as usize + 2],
                ]);
                *self.registers.r16_mut(x) = next_two_bytes;

                self.registers.pc += 3;
            }
            LdMemA { x } => {
                self.set_r16_mem(x, self.registers.af.a());
                self.registers.pc += 1;
            }
            LdAMem { x } => {
                let value = self.r16_mem(x);
                self.registers.af.set_a(value);
                self.registers.pc += 1;
            }
            LdNnSp => {
                let [low_sp, high_sp] = self.registers.sp.to_le_bytes();

                let destination = u16::from_le_bytes([
                    self.memory[pc as usize + 1],
                    self.memory[pc as usize + 2],
                ]);

                self.memory[destination as usize] = low_sp;
                self.memory[destination as usize + 1] = high_sp;
                self.registers.pc += 3;
            }
            IncRr { x } => {
                *self.registers.r16_mut(x) = self.registers.r16_mut(x).wrapping_add(1);
                self.registers.pc += 1;
            }
            DecRr { x } => {
                let result = self.registers.r16_mut(x).wrapping_sub(1);
                *self.registers.r16_mut(x) = result;
                self.registers.pc += 1;
            }
            AddHlRr { x } => {
                let hl = self.registers.hl.into_bits();
                let r16 = *self.registers.r16_mut(x);
                let (result, carry) = hl.overflowing_add(r16);
                let half_carry = ((hl & 0x0fff) + (r16 & 0x0fff)) & 0x1000 == 0x1000;

                *self.registers.r16_mut(R16::Hl) = result;
                self.registers.af.set_f(
                    self.registers
                        .af
                        .f()
                        .with_n(false)
                        .with_h(half_carry)
                        .with_c(carry),
                );

                self.registers.pc += 1;
            }
            IncR { x } => {
                let value = self.r8(x).wrapping_add(1);
                self.set_r8(x, value);

                self.registers.af.set_f(
                    self.registers
                        .af
                        .f()
                        .with_z(value == 0)
                        .with_n(false)
                        .with_h(value.trailing_zeros() >= 4),
                );
                self.registers.pc += 1;
            }
            DecR { x } => {
                let value = self.r8(x).wrapping_sub(1);
                self.set_r8(x, value);

                self.registers.af.set_f(
                    self.registers
                        .af
                        .f()
                        .with_z(value == 0)
                        .with_n(true)
                        .with_h(value.trailing_zeros() >= 4),
                );
                self.registers.pc += 1;
            }
            LdRN { x } => {
                let next_byte = self.memory[pc as usize + 1];
                self.set_r8(x, next_byte);
                self.registers.pc += 2;
            }
            Rlca => {
                // input:  [c]  [b7][b6][b5][b4][b3][b2][b1][b0]
                // output: [b7] [b6][b5][b4][b3][b2][b1][b0][b7]
                let value = self.registers.af.a();
                let b7 = value & 0b1000_0000 == 0b1000_0000;
                let rotated = value.rotate_left(1);

                self.registers.af.set_a(rotated);
                self.registers.af.set_f(
                    self.registers
                        .af
                        .f()
                        .with_z(false)
                        .with_n(false)
                        .with_h(false)
                        .with_c(b7),
                );
                self.registers.pc += 1;
            }
            Rla => {
                // input:  [c]  [b7][b6][b5][b4][b3][b2][b1][b0]
                // output: [b7] [b6][b5][b4][b3][b2][b1][b0][c]
                let value = self.registers.af.a();
                let b7 = value & 0b1000_0000 == 0b1000_0000;
                let mut shifted = value << 1;
                // Put the old carry bit in the least significant bit.
                if self.registers.af.f().c() {
                    shifted |= 0b0000_0001;
                }

                self.registers.af.set_a(shifted);
                self.registers.af.set_f(
                    self.registers
                        .af
                        .f()
                        .with_z(false)
                        .with_n(false)
                        .with_h(false)
                        .with_c(b7),
                );
                self.registers.pc += 1;
            }
            Rra => {
                // input:  [c]  [b7][b6][b5][b4][b3][b2][b1][b0]
                // output: [b0] [c][b7][b6][b5][b4][b3][b2][b1]
                let value = self.registers.af.a();
                let b0 = value & 1 == 1;
                let mut shifted = value >> 1;
                // Put the old carry bit in the most significant bit.
                if self.registers.af.f().c() {
                    shifted |= 0b1000_0000;
                }

                self.registers.af.set_a(shifted);
                self.registers.af.set_f(
                    self.registers
                        .af
                        .f()
                        .with_z(false)
                        .with_n(false)
                        .with_h(false)
                        .with_c(b0),
                );
                self.registers.pc += 1;
            }
            Daa => {
                // A surprisingly complicated instruction. This implementation is largely based on:
                // https://rgbds.gbdev.io/docs/v0.9.4/gbz80.7#DAA
                let a = self.registers.af.a();
                let mut adjustment = 0;
                let mut new_carry = false;

                let result = if self.registers.af.f().n() {
                    if self.registers.af.f().h() {
                        adjustment += 0x06;
                    }
                    if self.registers.af.f().c() {
                        adjustment += 0x60;
                        new_carry = true;
                    }
                    a.wrapping_sub(adjustment)
                } else {
                    if self.registers.af.f().h() || a & 0x0f > 0x09 {
                        adjustment += 0x06;
                    }
                    if self.registers.af.f().c() || a > 0x99 {
                        adjustment += 0x60;
                        new_carry = true;
                    }
                    a.wrapping_add(adjustment)
                };

                self.registers.af.set_a(result);
                self.registers.af.set_f(
                    self.registers
                        .af
                        .f()
                        .with_z(result == 0)
                        .with_h(false)
                        .with_c(new_carry),
                );
                self.registers.pc += 1;
            }
            Cpl => {
                let flipped = !self.registers.af.a();

                self.registers.af.set_a(flipped);
                self.registers
                    .af
                    .set_f(self.registers.af.f().with_n(true).with_h(true));
                self.registers.pc += 1;
            }
            Scf => {
                self.registers.af.set_f(
                    self.registers.af.f()
                        .with_n(false)
                        .with_h(false)
                        .with_c(true)
                );
                self.registers.pc += 1;
            }
            Ccf => {
                let carry = !self.registers.af.f().c();
                self.registers.af.set_f(
                    self.registers.af.f()
                        .with_n(false)
                        .with_h(false)
                        .with_c(carry)
                );
                self.registers.pc += 1;
            }
            JrE => {
                let jump_offset = self.memory[pc as usize + 1].cast_signed();
                self.registers.pc = self
                    .registers
                    .pc
                    .wrapping_add_signed(i16::from(jump_offset));
                self.registers.pc += 2;
            }
            JrCcE { c } => {
                let jump_offset = self.memory[pc as usize + 1].cast_signed();
                if self.check_condition(c) {
                    self.registers.pc = self
                        .registers
                        .pc
                        .wrapping_add_signed(i16::from(jump_offset));
                }
                self.registers.pc += 2;
            }

            // Block 1
            LdRR { x: dest, y: src } => {
                self.set_r8(dest, self.r8(src));
                self.registers.pc += 1;
            }

            // Block 2
            AddR { x } => {
                let a = self.registers.af.a();
                let r8 = self.r8(x);
                let (result, carry) = a.overflowing_add(r8);
                let half_carry = ((a & 0x0f) + (r8 & 0x0f)) & 0x10 == 0x10;

                self.registers.af.set_a(result);
                self.registers.af.set_f(
                    self.registers
                        .af
                        .f()
                        .with_z(result == 0)
                        .with_n(false)
                        .with_h(half_carry)
                        .with_c(carry),
                );
                self.registers.pc += 1;
            }
            SubR { x } => {
                let a = self.registers.af.a();
                let r8 = self.r8(x);
                let (result, carry) = a.overflowing_sub(r8);

                self.registers.af.set_a(result);
                self.registers.af.set_f(
                    self.registers
                        .af
                        .f()
                        .with_z(result == 0)
                        .with_n(true)
                        .with_h(a & 0x0F < r8 & 0x0F)
                        .with_c(carry),
                );
                self.registers.pc += 1;
            }
            AndR { x } => {
                let result = self.registers.af.a() & self.r8(x);

                self.registers.af.set_a(result);
                self.registers.af.set_f(
                    self.registers
                        .af
                        .f()
                        .with_z(result == 0)
                        .with_n(false)
                        .with_h(true)
                        .with_c(false),
                );
                self.registers.pc += 1;
            }
            XorR { x } => {
                let result = self.registers.af.a() ^ self.r8(x);

                self.registers.af.set_a(result);
                self.registers.af.set_f(
                    self.registers
                        .af
                        .f()
                        .with_z(result == 0)
                        .with_n(false)
                        .with_h(false)
                        .with_c(false),
                );
                self.registers.pc += 1;
            }
            OrR { x } => {
                let result = self.registers.af.a() | self.r8(x);

                self.registers.af.set_a(result);
                self.registers.af.set_f(
                    self.registers
                        .af
                        .f()
                        .with_z(result == 0)
                        .with_n(false)
                        .with_h(false)
                        .with_c(false),
                );
                self.registers.pc += 1;
            }
            CpR { x } => {
                let a = self.registers.af.a();
                let r8 = self.r8(x);
                let result = a.wrapping_sub(r8);

                self.registers.af.set_f(
                    self.registers
                        .af
                        .f()
                        .with_z(result == 0)
                        .with_n(true)
                        .with_h(a & 0x0F < r8 & 0x0F)
                        .with_c(a < r8),
                );
                self.registers.pc += 1;
            }

            // Block 3
            AddN => {
                let a = self.registers.af.a();
                let next_byte = self.memory[pc as usize + 1];
                let (result, carry) = a.overflowing_add(next_byte);
                let half_carry = ((a & 0x0f) + (next_byte & 0x0f)) & 0x10 == 0x10;

                self.registers.af.set_a(result);
                self.registers.af.set_f(
                    self.registers
                        .af
                        .f()
                        .with_z(result == 0)
                        .with_n(false)
                        .with_h(half_carry)
                        .with_c(carry),
                );
                self.registers.pc += 2;
            }
            AdcN => {
                let a = self.registers.af.a();
                let next_byte = self.memory[pc as usize + 1];
                let prev_carry = u8::from(self.registers.af.f().c());
                let full_result = u16::from(a) + u16::from(next_byte) + u16::from(prev_carry);
                // Carry if the 9th bit is set.
                let carry = full_result & 0x0100 == 0x0100;
                // Keep the lower 8 bits.
                #[allow(clippy::cast_possible_truncation)]
                let result = full_result as u8;

                let half_carry = ((a & 0x0f) + (next_byte & 0x0f) + prev_carry) & 0x10 == 0x10;

                self.registers.af.set_a(result);
                self.registers.af.set_f(
                    self.registers
                        .af
                        .f()
                        .with_z(result == 0)
                        .with_n(false)
                        .with_h(half_carry)
                        .with_c(carry),
                );
                self.registers.pc += 2;
            }
            SubN => {
                let a = self.registers.af.a();
                let next_byte = self.memory[pc as usize + 1];
                let (result, carry) = a.overflowing_sub(next_byte);

                self.registers.af.set_a(result);
                self.registers.af.set_f(
                    self.registers
                        .af
                        .f()
                        .with_z(result == 0)
                        .with_n(true)
                        .with_h(a & 0x0F < next_byte & 0x0F)
                        .with_c(carry),
                );
                self.registers.pc += 2;
            }
            SbcN => {
                let a = self.registers.af.a();
                let next_byte = self.memory[pc as usize + 1];
                let prev_carry = u8::from(self.registers.af.f().c());

                let (first_diff, first_carry) = a.overflowing_sub(next_byte);
                let (result, second_carry) = first_diff.overflowing_sub(prev_carry);

                // Carry if the 9th bit is set.
                let carry = first_carry | second_carry;

                let half_carry = ((a & 0x0f) < (next_byte & 0x0f)) | (first_diff & 0x0f < prev_carry);

                self.registers.af.set_a(result);
                self.registers.af.set_f(
                    self.registers
                        .af
                        .f()
                        .with_z(result == 0)
                        .with_n(true)
                        .with_h(half_carry)
                        .with_c(carry),
                );
                self.registers.pc += 2;
            }
            AndN => {
                let next_byte = self.memory[pc as usize + 1];
                let result = self.registers.af.a() & next_byte;

                self.registers.af.set_a(result);
                self.registers.af.set_f(
                    self.registers
                        .af
                        .f()
                        .with_z(result == 0)
                        .with_n(false)
                        .with_h(true)
                        .with_c(false),
                );
                self.registers.pc += 2;
            }
            XorN => {
                let next_byte = self.memory[pc as usize + 1];
                let result = self.registers.af.a() ^ next_byte;

                self.registers.af.set_a(result);
                self.registers.af.set_f(
                    self.registers
                        .af
                        .f()
                        .with_z(result == 0)
                        .with_n(false)
                        .with_h(false)
                        .with_c(false),
                );
                self.registers.pc += 2;
            }
            OrN => {
                let next_byte = self.memory[pc as usize + 1];
                let result = self.registers.af.a() | next_byte;

                self.registers.af.set_a(result);
                self.registers.af.set_f(
                    self.registers
                        .af
                        .f()
                        .with_z(result == 0)
                        .with_n(false)
                        .with_h(false)
                        .with_c(false),
                );
                self.registers.pc += 2;
            }
            CpN => {
                let a = self.registers.af.a();
                let next_byte = self.memory[pc as usize + 1];

                self.registers.af.set_f(
                    self.registers
                        .af
                        .f()
                        .with_z(a.wrapping_sub(next_byte) == 0)
                        .with_n(true)
                        .with_h(a & 0x0F < next_byte & 0x0F)
                        .with_c(a < next_byte),
                );
                self.registers.pc += 2;
            }
            RetCc { c } => {
                self.registers.pc += 1;
                if self.check_condition(c) {
                    let destination = u16::from_le_bytes([
                        self.memory[self.registers.sp as usize],
                        self.memory[self.registers.sp as usize + 1],
                    ]);
                    self.registers.sp += 2;

                    self.registers.pc = destination;
                }
            }
            Ret => {
                let destination = u16::from_le_bytes([
                    self.memory[self.registers.sp as usize],
                    self.memory[self.registers.sp as usize + 1],
                ]);
                self.registers.sp += 2;

                self.registers.pc = destination;
            }
            Reti => {
                let destination = u16::from_le_bytes([
                    self.memory[self.registers.sp as usize],
                    self.memory[self.registers.sp as usize + 1],
                ]);
                self.registers.sp += 2;

                self.ime = true;
                self.registers.pc = destination;
            }
            JpCcNn { c } => {
                let destination = u16::from_le_bytes([
                    self.memory[pc as usize + 1],
                    self.memory[pc as usize + 2],
                ]);
                self.registers.pc += 3;

                if self.check_condition(c) {
                    self.registers.pc = destination;
                }
            }
            JpNn => {
                let destination = u16::from_le_bytes([
                    self.memory[pc as usize + 1],
                    self.memory[pc as usize + 2],
                ]);
                self.registers.pc = destination;
            }
            JpHl => {
                self.registers.pc = self.registers.hl.into_bits();
            }
            CallCcNn {c } => {
                let destination = u16::from_le_bytes([
                    self.memory[pc as usize + 1],
                    self.memory[pc as usize + 2],
                ]);

                self.registers.pc += 3;

                if self.check_condition(c) {
                    // Push the address of the next instruction to the stack.
                    self.registers.sp -= 2;
                    let [low, high] = (pc + 3).to_le_bytes();
                    self.memory[self.registers.sp as usize] = low;
                    self.memory[self.registers.sp as usize + 1] = high;

                    self.registers.pc = destination;
                }
            }
            CallNn => {
                // Push the address of the next instruction to the stack.
                self.registers.sp -= 2;
                let [low, high] = (pc + 3).to_le_bytes();
                self.memory[self.registers.sp as usize] = low;
                self.memory[self.registers.sp as usize + 1] = high;

                let destination = u16::from_le_bytes([
                    self.memory[pc as usize + 1],
                    self.memory[pc as usize + 2],
                ]);
                self.registers.pc = destination;
            }
            RstN { x } => {
                // Push the address of the next instruction to the stack.
                self.registers.sp -= 2;
                let [low, high] = (pc + 1).to_le_bytes();
                self.memory[self.registers.sp as usize] = low;
                self.memory[self.registers.sp as usize + 1] = high;

                let destination = u16::from_le_bytes([
                    // Rst's parameter is pre-divided by 8, so we multiply it by 8 here.
                    x.value() * 8,
                    0x00,
                ]);
                self.registers.pc = destination;
            }
            PopRr { x } => {
                let low = self.memory[self.registers.sp as usize];
                let high = self.memory[self.registers.sp as usize + 1];
                self.registers.sp += 2;

                self.registers.set_r16_stack(x, u16::from_le_bytes([low, high]));

                self.registers.pc += 1;
            }
            PushRr { x } => {
                let [low, high] = self.registers.r16_stack(x).to_le_bytes();
                // Make room on the stack for a 16-bit value.
                self.registers.sp -= 2;
                // Game Boy is little-endian, so load the low byte then the high byte.
                self.memory[self.registers.sp as usize] = low;
                self.memory[self.registers.sp as usize + 1] = high;
                self.registers.pc += 1;
            }
            Prefix => self.execute_prefix(),
            LdhCA => {
                let destination = u16::from_le_bytes([self.registers.bc.c(), 0xFF]);
                self.memory[destination as usize] = self.registers.af.a();
                self.registers.pc += 1;
            }
            LdhNA => {
                let next_byte = self.memory[pc as usize + 1];
                let destination = u16::from_le_bytes([next_byte, 0xFF]);
                self.memory[destination as usize] = self.registers.af.a();
                self.registers.pc += 2;
            }
            LdNnA => {
                let next_two_bytes = u16::from_le_bytes([
                    self.memory[pc as usize + 1],
                    self.memory[pc as usize + 2],
                ]);
                self.memory[next_two_bytes as usize] = self.registers.af.a();
                self.registers.pc += 3;
            }
            LdhAC => {
                let address = u16::from_le_bytes([
                    self.registers.bc.c(),
                    0xFF,
                ]);
                let value = self.memory[address as usize];
                self.set_r8(R8::A, value);
                self.registers.pc += 1;
            }
            LdhAN => {
                let next_byte = self.memory[pc as usize + 1];
                let address = u16::from_le_bytes([next_byte, 0xFF]);
                let value = self.memory[address as usize];
                self.registers.af.set_a(value);
                self.registers.pc += 2;
            }
            LdANn => {
                let next_two_bytes = u16::from_le_bytes([
                    self.memory[pc as usize + 1],
                    self.memory[pc as usize + 2],
                ]);
                let value = self.memory[next_two_bytes as usize];

                self.registers.af.set_a(value);
                self.registers.pc += 3;
            }
            AddSpE => {
                let next_byte = self.memory[pc as usize + 1];
                let e = next_byte.cast_signed();
                let result = self.registers.sp.wrapping_add_signed(i16::from(e));

                let carry = ((self.registers.sp & 0xff) + u16::from(next_byte)) & 0x0100 == 0x0100;
                let half_carry = (((self.registers.sp & 0x0f) as u8) + (next_byte & 0x0f)) & 0x10 == 0x10;

                self.registers.sp = result;
                self.registers.af.set_f(
                    self.registers
                        .af
                        .f()
                        .with_z(false)
                        .with_n(false)
                        .with_h(half_carry)
                        .with_c(carry),
                );
                self.registers.pc += 2;
            }
            LdHlSpPlusE => {
                let next_byte = self.memory[pc as usize + 1];
                let e = next_byte.cast_signed();
                let result = self.registers.sp.wrapping_add_signed(i16::from(e));

                let carry = ((self.registers.sp & 0xff) + u16::from(next_byte)) & 0x0100 == 0x0100;
                let half_carry = (((self.registers.sp & 0x0f) as u8) + (next_byte & 0x0f)) & 0x10 == 0x10;

                *self.registers.r16_mut(R16::Hl) = result;
                self.registers.af.set_f(
                    self.registers
                        .af
                        .f()
                        .with_z(false)
                        .with_n(false)
                        .with_h(half_carry)
                        .with_c(carry),
                );
                self.registers.pc += 2;
            }
            LdSpHl => {
                self.registers.sp = self.registers.hl.into_bits();
                self.registers.pc += 1;
            }
            Di => {
                self.ime = false;
                self.registers.pc += 1;
            }
            Ei => {
                // TODO: For accuracy, wait until the next instruction to actually enable interrupts
                // See: https://rgbds.gbdev.io/docs/v0.9.4/gbz80.7#EI
                self.ime = true;
                self.registers.pc += 1;
            }
            opcode => Err(format!("unimplemented opcode: {opcode:?}"))?,
        }
        Ok(())
    }

    #[allow(clippy::too_many_lines)]
    fn execute_prefix(&mut self) {
        use opcodes::PrefixOpcode::*;

        let second_byte = self.memory[self.registers.pc as usize + 1];
        let prefix_opcode = opcodes::decode_prefix(second_byte);

        match prefix_opcode {
            RlR { x } => {
                // input:  [c]  [b7][b6][b5][b4][b3][b2][b1][b0]
                // output: [b7] [b6][b5][b4][b3][b2][b1][b0][c]
                let value = self.r8(x);
                let b7 = value & 0b1000_0000 == 0b1000_0000;
                let mut shifted = value << 1;
                // Put the old carry bit in the least significant bit.
                if self.registers.af.f().c() {
                    shifted |= 0b0000_0001;
                }

                self.set_r8(x, shifted);
                self.registers.af.set_f(
                    self.registers
                        .af
                        .f()
                        .with_z(shifted == 0)
                        .with_n(false)
                        .with_h(false)
                        .with_c(b7),
                );
                self.registers.pc += 2;
            }
            RrR { x } => {
                // input:  [c]  [b7][b6][b5][b4][b3][b2][b1][b0]
                // output: [b0]  [c][b7][b6][b5][b4][b3][b2][b1]
                let value = self.r8(x);
                let b0 = value & 1 == 1;
                let mut shifted = value >> 1;
                // Put the old carry bit in the most significant bit.
                if self.registers.af.f().c() {
                    shifted |= 0b1000_0000;
                }

                self.set_r8(x, shifted);
                self.registers.af.set_f(
                    self.registers
                        .af
                        .f()
                        .with_z(shifted == 0)
                        .with_n(false)
                        .with_h(false)
                        .with_c(b0),
                );
                self.registers.pc += 2;
            }
            SlaR { x } => {
                // input:  [c]  [b7][b6][b5][b4][b3][b2][b1][b0]
                // output: [b7] [b6][b5][b4][b3][b2][b1][b0][0]
                let value = self.r8(x);
                let b7 = value & 0b1000_0000 == 0b1000_0000;
                let shifted = value << 1;

                self.set_r8(x, shifted);
                self.registers.af.set_f(
                    self.registers
                        .af
                        .f()
                        .with_z(shifted == 0)
                        .with_n(false)
                        .with_h(false)
                        .with_c(b7),
                );
                self.registers.pc += 2;
            }
            SwapR { x } => {
                // input:  [b7][b6][b5][b4][b3][b2][b1][b0]
                // output: [b3][b2][b1][b0][b7][b6][b5][b4]
                let value = self.r8(x);
                let swapped = value.rotate_right(4);

                self.set_r8(x, swapped);
                self.registers.af.set_f(
                    self.registers
                        .af
                        .f()
                        .with_z(swapped == 0)
                        .with_n(false)
                        .with_h(false)
                        .with_c(false),
                );
                self.registers.pc += 2;
            }
            SrlR { x } => {
                // input:  [c]  [b7][b6][b5][b4][b3][b2][b1][b0]
                // output: [b0] [0][b7][b6][b5][b4][b3][b2][b1]
                let value = self.r8(x);
                let b0 = value & 1 == 1;
                let shifted = value >> 1;

                self.set_r8(x, shifted);
                self.registers.af.set_f(
                    self.registers
                        .af
                        .f()
                        .with_z(shifted == 0)
                        .with_n(false)
                        .with_h(false)
                        .with_c(b0),
                );
                self.registers.pc += 2;
            }
            BitBR { b: bit_index, x } => {
                let value = self.r8(x);
                let nth_bit = value >> bit_index.value() & 1;
                let nth_bit_set = nth_bit != 0;

                self.registers.af.set_f(
                    self.registers
                        .af
                        .f()
                        .with_z(!nth_bit_set)
                        .with_n(false)
                        .with_h(true),
                );
                self.registers.pc += 2;
            }
            ResBR { b: bit_index, x } => {
                let value = self.r8(x);
                let mask = !(1 << bit_index.value());
                let result = value & mask;

                self.set_r8(x, result);
                self.registers.pc += 2;
            }
            SetBR { b: bit_index, x } => {
                let value = self.r8(x);
                let bit = 1 << bit_index.value();
                let result = value | bit;

                self.set_r8(x, result);
                self.registers.pc += 2;
            }
            prefix_opcode => unimplemented!("prefix opcode: {:?}", prefix_opcode),
        }
    }
}

impl Default for Cpu {
    fn default() -> Self {
        Self {
            registers: Registers::default(),
            memory: [0; MEM_MAP_SIZE],
            ime: false,
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
        cpu.execute().unwrap();
        assert_eq!(cpu.registers.sp, 0xFFFE);
    }
}
