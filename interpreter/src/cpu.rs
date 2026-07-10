mod interrupt_bits;
pub mod opcodes;
pub mod registers;

pub use interrupt_bits::InterruptBits;

use std::error::Error;

use hw_constants::{PostBoot, io_regs};
use rkyv::{Archive, Deserialize, Serialize};

#[cfg(feature = "cpu-logging")]
use log::info;

use crate::bus::AddressBus;
use opcodes::cycles::m_cycles;
use opcodes::parameters::{Condition, R8, R16, R16Mem};
use opcodes::{Opcode, PrefixOpcode};
use registers::Registers;

#[derive(Default, Archive, Deserialize, Serialize)]
pub struct Cpu {
    pub registers: Registers,
    pub memory: AddressBus,
    // Interrupt master enable flag
    pub ime: bool,
    pub halted: bool,
}

impl Cpu {
    fn r8(&mut self, r8: R8) -> u8 {
        match r8 {
            R8::B => self.registers.bc.b(),
            R8::C => self.registers.bc.c(),
            R8::D => self.registers.de.d(),
            R8::E => self.registers.de.e(),
            R8::H => self.registers.hl.h(),
            R8::L => self.registers.hl.l(),
            R8::IndirectHL => {
                let value = self.memory.read_byte(self.registers.hl.into_bits());
                self.memory.increment_timers(1);
                value
            }
            R8::A => self.registers.af.a(),
        }
    }

    fn set_r8(&mut self, r8: R8, value: u8) {
        match r8 {
            R8::B => self.registers.bc.set_b(value),
            R8::C => self.registers.bc.set_c(value),
            R8::D => self.registers.de.set_d(value),
            R8::E => self.registers.de.set_e(value),
            R8::H => self.registers.hl.set_h(value),
            R8::L => self.registers.hl.set_l(value),
            R8::IndirectHL => {
                self.memory.write_byte(self.registers.hl.into_bits(), value);
                self.memory.increment_timers(1);
            }
            R8::A => self.registers.af.set_a(value),
        }
    }

    pub(crate) fn r16_mem(&mut self, r16_mem: R16Mem) -> u8 {
        match r16_mem {
            R16Mem::Bc => self.memory.read_byte(self.registers.bc.into_bits()),
            R16Mem::De => self.memory.read_byte(self.registers.de.into_bits()),
            R16Mem::HlInc => {
                let value = self.memory.read_byte(self.registers.hl.into_bits());
                self.registers.hl =
                    registers::Hl::from_bits(self.registers.hl.into_bits().wrapping_add(1));
                value
            }
            R16Mem::HlDec => {
                let value = self.memory.read_byte(self.registers.hl.into_bits());
                self.registers.hl =
                    registers::Hl::from_bits(self.registers.hl.into_bits().wrapping_sub(1));
                value
            }
        }
    }

    pub(crate) fn set_r16_mem(&mut self, r16_mem: R16Mem, value: u8) {
        match r16_mem {
            R16Mem::Bc => self.memory.write_byte(self.registers.bc.into_bits(), value),
            R16Mem::De => self.memory.write_byte(self.registers.de.into_bits(), value),
            R16Mem::HlInc => {
                self.memory.write_byte(self.registers.hl.into_bits(), value);
                self.registers.hl =
                    registers::Hl::from_bits(self.registers.hl.into_bits().wrapping_add(1));
            }
            R16Mem::HlDec => {
                self.memory.write_byte(self.registers.hl.into_bits(), value);
                self.registers.hl =
                    registers::Hl::from_bits(self.registers.hl.into_bits().wrapping_sub(1));
            }
        }
    }

    fn check_condition(&self, condition: Condition) -> bool {
        let flags = self.registers.af.f();
        match condition {
            Condition::Nz => !flags.z(),
            Condition::Z => flags.z(),
            Condition::Nc => !flags.c(),
            Condition::C => flags.c(),
        }
    }

    pub fn handle_interrupts(&mut self) {
        self.memory.update_joypad(); // Is this the best place to put this?

        if self.halted {
            let t_cycles_until_exit = self.memory.next_interrupt.saturating_sub(self.memory.clock);
            let m_cycles_until_exit = t_cycles_until_exit / 4;

            if m_cycles_until_exit > 0 {
                self.memory
                    .increment_timers((m_cycles_until_exit).try_into().unwrap());
            }

            // DMG checks interrupt flags *between* M-Cycles in halt mode.
            self.memory.half_increment_timers();
        }

        // If an interrupt's enabled and flag bit is set, it needs to be serviced.
        let to_service = self.memory.buffer[hw_constants::IE as usize]
            & self.memory.buffer[io_regs::IF as usize]
            & 0b0001_1111;

        if self.halted {
            // Another half tick to complete the M-Cycle.
            self.memory.half_increment_timers();
        }

        if to_service == 0 {
            // Nothing to do, so return early.
            return;
        }

        if self.halted {
            #[cfg(feature = "cpu-logging")]
            info!(target: "cpu_halt", "Leaving HALT on clock: {}", self.memory.clock);

            self.halted = false;
        }

        if self.ime {
            // Turn off interrupt master enable.
            self.ime = false;

            // Interrupts are serviced with a priority in order of least-to-most significant bit.
            // 0: VBlank, 1: LCD, 2: Timer, 3: Serial, and 4: Joypad.
            let nth_interrupt = to_service.trailing_zeros();

            #[cfg(feature = "cpu-logging")]
            info!(target: "cpu_interrupt", "Serving interrupt {nth_interrupt} on clock {}", self.memory.clock);

            // Clear the flag bit of the interrupt we're servicing.
            let flag_mask = !(0b0000_0001 << nth_interrupt);
            self.memory.buffer[io_regs::IF as usize] &= flag_mask;

            // Push the PC to the stack.
            self.registers.sp = self.registers.sp.wrapping_sub(2);
            let [low, high] = self.registers.pc.to_le_bytes();
            self.memory.write_byte(self.registers.sp, low);
            self.memory.write_byte(self.registers.sp + 1, high);

            // Each interrupt handler is 8 bytes apart in memory, starting at 0x0040.
            #[allow(clippy::cast_possible_truncation)]
            let interrupt_handler_offset = 0x0008 * nth_interrupt as u16;
            self.registers.pc = 0x0040 + interrupt_handler_offset;

            // Interrupt handling takes 5 MCycles
            // See: https://gbdev.io/pandocs/Interrupts.html#interrupt-handling
            self.memory.increment_timers(5);
        }
    }

    fn calculate_m_cycles(&self, opcode: Opcode) -> u16 {
        match opcode {
            Opcode::JpCcNn { c } => {
                if self.check_condition(c) {
                    4
                } else {
                    3
                }
            }
            Opcode::JrCcE { c } => {
                if self.check_condition(c) {
                    3
                } else {
                    2
                }
            }
            Opcode::RetCc { c } => {
                if self.check_condition(c) {
                    5
                } else {
                    2
                }
            }
            _ => m_cycles(opcode),
        }
    }

    pub fn execute(&mut self) -> Result<(), Box<dyn Error>> {
        // Our halted CPU just early returns forever unless handle_interrupts gets us out of halted mode.
        // This function may tick other components if we're halted and/or servicing an interrupt.
        self.handle_interrupts();
        if self.halted {
            return Ok(());
        }

        let bytecode = self.memory.read_byte(self.registers.pc);
        let opcode = Opcode::decode(bytecode)?;
        self.execute_op(opcode)
    }

    /// # Errors
    ///
    /// Will return an error if the opcode is unimplemented.
    #[allow(clippy::too_many_lines)]
    pub fn execute_op(&mut self, opcode: Opcode) -> Result<(), Box<dyn Error>> {
        let pc = self.registers.pc;
        let mut m_cycles = self.calculate_m_cycles(opcode);

        if m_cycles > 0 {
            self.memory.increment_timers(1);
            m_cycles -= 1;
        }

        match opcode {
            // Block 0
            Opcode::Nop => {
                self.registers.pc += 1;
            }
            Opcode::LdRrNn { x } => {
                let next_two_bytes = u16::from_le_bytes([
                    self.memory.read_byte(pc + 1),
                    self.memory.read_byte(pc + 2),
                ]);
                *self.registers.r16_mut(x) = next_two_bytes;

                self.registers.pc += 3;
            }
            Opcode::LdMemA { x } => {
                self.set_r16_mem(x, self.registers.af.a());
                self.registers.pc += 1;
            }
            Opcode::LdAMem { x } => {
                let value = self.r16_mem(x);
                self.registers.af.set_a(value);
                self.registers.pc += 1;
            }
            Opcode::LdNnSp => {
                let [low_sp, high_sp] = self.registers.sp.to_le_bytes();

                let destination = u16::from_le_bytes([
                    self.memory.read_byte(pc + 1),
                    self.memory.read_byte(pc + 2),
                ]);

                self.memory.write_byte(destination, low_sp);
                self.memory.write_byte(destination + 1, high_sp);
                self.registers.pc += 3;
            }
            Opcode::IncRr { x } => {
                *self.registers.r16_mut(x) = self.registers.r16_mut(x).wrapping_add(1);
                self.registers.pc += 1;
            }
            Opcode::DecRr { x } => {
                let result = self.registers.r16_mut(x).wrapping_sub(1);
                *self.registers.r16_mut(x) = result;
                self.registers.pc += 1;
            }
            Opcode::AddHlRr { x } => {
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
            Opcode::IncR { x } => {
                let r8 = self.r8(x);
                let result = r8.wrapping_add(1);
                self.set_r8(x, result);

                self.registers.af.set_f(
                    self.registers
                        .af
                        .f()
                        .with_z(result == 0)
                        .with_n(false)
                        .with_h(result.trailing_zeros() >= 4),
                );
                self.registers.pc += 1;
            }
            Opcode::DecR { x } => {
                let r8 = self.r8(x);
                let result = r8.wrapping_sub(1);
                self.set_r8(x, result);

                self.registers.af.set_f(
                    self.registers
                        .af
                        .f()
                        .with_z(result == 0)
                        .with_n(true)
                        .with_h(r8.trailing_zeros() >= 4),
                );
                self.registers.pc += 1;
            }
            Opcode::LdRN { x } => {
                // TICKS MANUALLY
                self.memory.increment_timers(1);

                let next_byte = self.memory.read_byte(pc + 1);
                self.memory.increment_timers(1);

                self.set_r8(x, next_byte);

                self.registers.pc += 2;
            }
            Opcode::Rlca => {
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
            Opcode::Rrca => {
                // input:  [c]  [b7][b6][b5][b4][b3][b2][b1][b0]
                // output: [b0] [b0][b7][b6][b5][b4][b3][b2][b1]
                let value = self.registers.af.a();
                let b0 = value & 1 == 1;
                let rotated = value.rotate_right(1);

                self.registers.af.set_a(rotated);
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
            Opcode::Rla => {
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
            Opcode::Rra => {
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
            Opcode::Daa => {
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
            Opcode::Cpl => {
                let flipped = !self.registers.af.a();

                self.registers.af.set_a(flipped);
                self.registers
                    .af
                    .set_f(self.registers.af.f().with_n(true).with_h(true));
                self.registers.pc += 1;
            }
            Opcode::Scf => {
                self.registers.af.set_f(
                    self.registers
                        .af
                        .f()
                        .with_n(false)
                        .with_h(false)
                        .with_c(true),
                );
                self.registers.pc += 1;
            }
            Opcode::Ccf => {
                let carry = !self.registers.af.f().c();
                self.registers.af.set_f(
                    self.registers
                        .af
                        .f()
                        .with_n(false)
                        .with_h(false)
                        .with_c(carry),
                );
                self.registers.pc += 1;
            }
            Opcode::JrE => {
                let jump_offset = self.memory.read_byte(pc + 1).cast_signed();
                self.registers.pc = self
                    .registers
                    .pc
                    .wrapping_add_signed(i16::from(jump_offset));
                self.registers.pc += 2;
            }
            Opcode::JrCcE { c } => {
                let jump_offset = self.memory.read_byte(pc + 1).cast_signed();

                if self.check_condition(c) {
                    self.registers.pc = self
                        .registers
                        .pc
                        .wrapping_add_signed(i16::from(jump_offset));
                }
                self.registers.pc += 2;
            }
            Opcode::Stop => unimplemented!("STOP opcode reached."),

            // Block 1
            Opcode::LdRR { x: dest, y: src } => {
                let value = self.r8(src);
                self.set_r8(dest, value);
                self.registers.pc += 1;
            }
            Opcode::Halt => {
                #[cfg(feature = "cpu-logging")]
                info!(target: "cpu_halt", "HALTING on PPU dot: {}", self.memory.ppu.dot_counter % 456);

                self.registers.pc += 1;
                self.halted = true;
            }

            // Block 2
            Opcode::AddR { x } => {
                let a = self.registers.af.a();
                let r8 = self.r8(x);
                let (result, carry) = a.overflowing_add(r8);
                let half_carry = ((a & 0x0f) + (r8 & 0x0f)) > 0x0f;

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
            Opcode::AdcR { x } => {
                let a = self.registers.af.a();
                let r8 = self.r8(x);
                let prev_carry = self.registers.af.f().c();

                let (result, carry) = a.carrying_add(r8, self.registers.af.f().c());

                let (half_result, _) = (a & 0x0f).carrying_add(r8 & 0x0f, prev_carry);
                let half_carry = half_result & 0x10 == 0x10;

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
            Opcode::SubR { x } => {
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
            Opcode::SbcR { x } => {
                let a = self.registers.af.a();
                let r8 = self.r8(x);
                let prev_carry = self.registers.af.f().c();

                let (result, carry) = a.borrowing_sub(r8, prev_carry);
                let (_, half_carry) = (a & 0x0f).borrowing_sub(r8 & 0x0f, prev_carry);

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
                self.registers.pc += 1;
            }
            Opcode::AndR { x } => {
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
            Opcode::XorR { x } => {
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
            Opcode::OrR { x } => {
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
            Opcode::CpR { x } => {
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
            Opcode::AddN => {
                let a = self.registers.af.a();
                let next_byte = self.memory.read_byte(pc + 1);
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
            Opcode::AdcN => {
                let a = self.registers.af.a();
                let next_byte = self.memory.read_byte(pc + 1);
                let prev_carry = self.registers.af.f().c();

                let (result, carry) = a.carrying_add(next_byte, prev_carry);

                let (half_result, _) = (a & 0x0f).carrying_add(next_byte & 0x0f, prev_carry);
                let half_carry = half_result & 0x10 == 0x10;

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
            Opcode::SubN => {
                let a = self.registers.af.a();
                let next_byte = self.memory.read_byte(pc + 1);
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
            Opcode::SbcN => {
                let a = self.registers.af.a();
                let next_byte = self.memory.read_byte(pc + 1);
                let prev_carry = self.registers.af.f().c();

                let (result, carry) = a.borrowing_sub(next_byte, prev_carry);
                let (_, half_carry) = (a & 0x0f).borrowing_sub(next_byte & 0x0f, prev_carry);

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
            Opcode::AndN => {
                let next_byte = self.memory.read_byte(pc + 1);
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
            Opcode::XorN => {
                let next_byte = self.memory.read_byte(pc + 1);
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
            Opcode::OrN => {
                let next_byte = self.memory.read_byte(pc + 1);
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
            Opcode::CpN => {
                let a = self.registers.af.a();
                let next_byte = self.memory.read_byte(pc + 1);

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
            Opcode::RetCc { c } => {
                self.registers.pc += 1;

                if self.check_condition(c) {
                    let destination = u16::from_le_bytes([
                        self.memory.read_byte(self.registers.sp),
                        self.memory.read_byte(self.registers.sp + 1),
                    ]);
                    self.registers.sp += 2;

                    self.registers.pc = destination;
                }
            }
            Opcode::Ret => {
                let destination = u16::from_le_bytes([
                    self.memory.read_byte(self.registers.sp),
                    self.memory.read_byte(self.registers.sp + 1),
                ]);
                self.registers.sp += 2;

                self.registers.pc = destination;
            }
            Opcode::Reti => {
                let destination = u16::from_le_bytes([
                    self.memory.read_byte(self.registers.sp),
                    self.memory.read_byte(self.registers.sp + 1),
                ]);
                self.registers.sp += 2;

                self.ime = true;
                self.registers.pc = destination;
            }
            Opcode::JpCcNn { c } => {
                let destination = u16::from_le_bytes([
                    self.memory.read_byte(pc + 1),
                    self.memory.read_byte(pc + 2),
                ]);
                self.registers.pc += 3;

                if self.check_condition(c) {
                    self.registers.pc = destination;
                }
            }
            Opcode::JpNn => {
                let destination = u16::from_le_bytes([
                    self.memory.read_byte(pc + 1),
                    self.memory.read_byte(pc + 2),
                ]);
                self.registers.pc = destination;
            }
            Opcode::JpHl => {
                self.registers.pc = self.registers.hl.into_bits();
            }
            Opcode::CallCcNn { c } => {
                // TICKS MANUALLY
                self.memory.increment_timers(1);

                let low_dest = self.memory.read_byte(pc + 1);
                self.memory.increment_timers(1);

                let high_dest = self.memory.read_byte(pc + 2);
                let destination = u16::from_le_bytes([low_dest, high_dest]);
                self.memory.increment_timers(1);

                self.registers.pc += 3;

                if self.check_condition(c) {
                    self.memory.increment_timers(1);

                    // Push the address of the next instruction to the stack.
                    let [low, high] = (pc + 3).to_le_bytes();
                    self.registers.sp -= 1;
                    self.memory.write_byte(self.registers.sp, high);
                    self.memory.increment_timers(1);

                    self.registers.sp -= 1;
                    self.memory.write_byte(self.registers.sp, low);
                    self.memory.increment_timers(1);

                    self.registers.pc = destination;
                }
            }
            Opcode::CallNn => {
                // TICKS MANUALLY
                self.memory.increment_timers(1);

                let low_dest = self.memory.read_byte(pc + 1);
                self.memory.increment_timers(1);

                let high_dest = self.memory.read_byte(pc + 2);
                let destination = u16::from_le_bytes([low_dest, high_dest]);
                self.memory.increment_timers(2);

                // Push the address of the next instruction to the stack.
                let [low, high] = (pc + 3).to_le_bytes();
                self.registers.sp -= 1;
                self.memory.write_byte(self.registers.sp, high);
                self.memory.increment_timers(1);

                self.registers.sp -= 1;
                self.memory.write_byte(self.registers.sp, low);
                self.memory.increment_timers(1);

                self.registers.pc = destination;
                // For timing see: https://github.com/Gekkio/mooneye-test-suite/blob/443f6e1f2a8d83ad9da051cbb960311c5aaaea66/acceptance/call_timing.s
            }
            Opcode::RstN { x } => {
                // Push the address of the next instruction to the stack.
                self.registers.sp -= 2;
                let [low, high] = (pc + 1).to_le_bytes();
                self.memory.write_byte(self.registers.sp, low);
                self.memory.write_byte(self.registers.sp + 1, high);

                let destination = u16::from_le_bytes([
                    // Rst's parameter is pre-divided by 8, so we multiply it by 8 here.
                    x.value() * 8,
                    0x00,
                ]);
                self.registers.pc = destination;
            }
            Opcode::PopRr { x } => {
                let low = self.memory.read_byte(self.registers.sp);
                let high = self.memory.read_byte(self.registers.sp + 1);
                self.registers.sp += 2;

                self.registers
                    .set_r16_stack(x, u16::from_le_bytes([low, high]));

                self.registers.pc += 1;
            }
            Opcode::PushRr { x } => {
                let [low, high] = self.registers.r16_stack(x).to_le_bytes();
                // Make room on the stack for a 16-bit value.
                self.registers.sp -= 2;
                // Game Boy is little-endian, so load the low byte then the high byte.
                self.memory.write_byte(self.registers.sp, low);
                self.memory.write_byte(self.registers.sp + 1, high);
                self.registers.pc += 1;
            }
            Opcode::Prefix => self.execute_prefix(),
            Opcode::LdhCA => {
                let destination = u16::from_le_bytes([self.registers.bc.c(), 0xFF]);
                self.memory.write_byte(destination, self.registers.af.a());
                self.registers.pc += 1;
            }
            Opcode::LdhNA => {
                // TICKS MANUALLY
                self.memory.increment_timers(1);

                let next_byte = self.memory.read_byte(pc + 1);
                self.memory.increment_timers(1);

                let destination = u16::from_le_bytes([next_byte, 0xFF]);
                self.memory.write_byte(destination, self.registers.af.a());
                self.memory.increment_timers(1);

                self.registers.pc += 2;
            }
            Opcode::LdNnA => {
                // TICKS MANUALLY
                self.memory.increment_timers(1);

                let first_byte = self.memory.read_byte(pc + 1);
                self.memory.increment_timers(1);

                let second_byte = self.memory.read_byte(pc + 2);
                self.memory.increment_timers(1);

                let address = u16::from_le_bytes([first_byte, second_byte]);
                self.memory.write_byte(address, self.registers.af.a());
                self.memory.increment_timers(1);

                self.registers.pc += 3;
            }
            Opcode::LdhAC => {
                // TICKS MANUALLY
                self.memory.increment_timers(1);

                let address = u16::from_le_bytes([self.registers.bc.c(), 0xFF]);
                let value = self.memory.read_byte(address);
                self.memory.increment_timers(1);

                self.set_r8(R8::A, value);

                self.registers.pc += 1;
            }
            Opcode::LdhAN => {
                // TICKS MANUALLY
                self.memory.increment_timers(1);

                let next_byte = self.memory.read_byte(pc + 1);
                self.memory.increment_timers(1);

                let address = u16::from_le_bytes([next_byte, 0xFF]);
                let value = self.memory.read_byte(address);
                self.registers.af.set_a(value);
                self.memory.increment_timers(1);

                self.registers.pc += 2;
            }
            Opcode::LdANn => {
                // TICKS MANUALLY
                self.memory.increment_timers(1);

                let first_byte = self.memory.read_byte(pc + 1);
                self.memory.increment_timers(1);

                let second_byte = self.memory.read_byte(pc + 2);
                self.memory.increment_timers(1);

                let value = self
                    .memory
                    .read_byte(u16::from_le_bytes([first_byte, second_byte]));
                self.registers.af.set_a(value);
                self.memory.increment_timers(1);

                self.registers.pc += 3;
            }
            Opcode::AddSpE => {
                let next_byte = self.memory.read_byte(pc + 1);
                let e = next_byte.cast_signed();
                let result = self.registers.sp.wrapping_add_signed(i16::from(e));

                let carry = ((self.registers.sp & 0xff) + u16::from(next_byte)) & 0x0100 == 0x0100;
                let half_carry =
                    (((self.registers.sp & 0x0f) as u8) + (next_byte & 0x0f)) & 0x10 == 0x10;

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
            Opcode::LdHlSpPlusE => {
                let next_byte = self.memory.read_byte(pc + 1);
                let e = next_byte.cast_signed();
                let result = self.registers.sp.wrapping_add_signed(i16::from(e));

                let carry = ((self.registers.sp & 0xff) + u16::from(next_byte)) & 0x0100 == 0x0100;
                let half_carry =
                    (((self.registers.sp & 0x0f) as u8) + (next_byte & 0x0f)) & 0x10 == 0x10;

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
            Opcode::LdSpHl => {
                self.registers.sp = self.registers.hl.into_bits();
                self.registers.pc += 1;
            }
            Opcode::Di => {
                self.ime = false;
                self.registers.pc += 1;
            }
            Opcode::Ei => {
                // TODO: For accuracy, wait until the next instruction to actually enable interrupts
                // See: https://rgbds.gbdev.io/docs/v0.9.4/gbz80.7#EI
                #[cfg(feature = "cpu-logging")]
                {
                    info!(target: "cpu_ei", "Enabling interrupts at: {}", self.memory.clock);
                    info!("FLAGS {:b}", self.memory.buffer[io_regs::IF as usize]);
                    info!("ENABLED {:b}", self.memory.buffer[io_regs::IE as usize]);
                }

                self.ime = true;
                self.registers.pc += 1;
            }
        }

        self.memory.increment_timers(m_cycles);
        Ok(())
    }

    #[allow(clippy::too_many_lines)]
    fn execute_prefix(&mut self) {
        // Decoding the 0xCB instruction took 1 MCycle.
        self.memory.increment_timers(1);

        let second_byte = self.memory.read_byte(self.registers.pc + 1);
        let prefix_opcode = PrefixOpcode::decode(second_byte);

        // Decoding the next byte took another.
        self.memory.increment_timers(1);

        match prefix_opcode {
            PrefixOpcode::RlcR { x } => {
                // input:  [c]  [b7][b6][b5][b4][b3][b2][b1][b0]
                // output: [b7] [b6][b5][b4][b3][b2][b1][b0][b7]
                let value = self.r8(x);
                let b7 = value & 0b1000_0000 == 0b1000_0000;
                let shifted = value.rotate_left(1);

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
            PrefixOpcode::RrcR { x } => {
                // input:  [c]  [b7][b6][b5][b4][b3][b2][b1][b0]
                // output: [b0] [b0][b7][b6][b5][b4][b3][b2][b1]
                let value = self.r8(x);
                let b0 = value & 1 == 1;
                let shifted = value.rotate_right(1);

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
            PrefixOpcode::RlR { x } => {
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
            PrefixOpcode::RrR { x } => {
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
            PrefixOpcode::SlaR { x } => {
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
            PrefixOpcode::SraR { x } => {
                // input:  [c]  [b7][b6][b5][b4][b3][b2][b1][b0]
                // output: [b0] [b7][b7][b6][b5][b4][b3][b2][b1]
                // Rust only arithmetically shifts signed integers, so cast r8 signed.
                let value = self.r8(x).cast_signed();
                let b0 = value & 1 == 1;
                let shifted = (value >> 1).cast_unsigned();

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
            PrefixOpcode::SwapR { x } => {
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
            PrefixOpcode::SrlR { x } => {
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
            PrefixOpcode::BitBR { b: bit_index, x } => {
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
            PrefixOpcode::ResBR { b: bit_index, x } => {
                let value = self.r8(x);
                let mask = !(1 << bit_index.value());
                let result = value & mask;

                self.set_r8(x, result);
                self.registers.pc += 2;
            }
            PrefixOpcode::SetBR { b: bit_index, x } => {
                let value = self.r8(x);
                let bit = 1 << bit_index.value();
                let result = value | bit;

                self.set_r8(x, result);
                self.registers.pc += 2;
            }
        }
    }
}

impl PostBoot for Cpu {
    fn post_boot_mgb() -> Self {
        Self {
            registers: Registers::post_boot_mgb(),
            memory: AddressBus::post_boot_mgb(),
            ..Self::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_bootrom_instruction() {
        let mut cpu = Cpu::default();
        cpu.memory.load_rom(&vec![0; 0x8000]);
        cpu.execute().unwrap();
        assert_eq!(cpu.registers.sp, 0xFFFE);
    }
}
