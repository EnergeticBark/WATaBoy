use sm83_interp::cpu::opcodes::parameters::{R8, R16};
use wasm_encoder::InstructionSink;

use crate::codegen::registers::{A, B, C, D, E, F, H, L, r8_to_reg_param};

pub(crate) enum FlagBit {
    Zero = 7,
    Subtraction = 6,
    HalfCarry = 5,
    Carry = 4,
}

pub(crate) trait Sm83Macros {
    fn get_r16(&mut self, r16: R16) -> &mut Self;
    fn clear_flags(&mut self) -> &mut Self;
    fn assign_flags(
        &mut self,
        zero: bool,
        subtraction: bool,
        half_carry: bool,
        carry: bool,
    ) -> &mut Self;
    fn set_flag(&mut self, flag_bit: FlagBit) -> &mut Self;
    fn check_flag(&mut self, flag_bit: FlagBit) -> &mut Self;
    fn return_regs(&mut self) -> &mut Self;
    fn call_write_byte(&mut self) -> &mut Self;
}

impl Sm83Macros for InstructionSink<'_> {
    /// Get the value of the specified 16-bit register.
    /// # Signature
    /// ```
    /// () -> (r16: i32)
    /// ```
    fn get_r16(&mut self, r16: R16) -> &mut Self {
        let (high_reg, low_reg) = match r16 {
            R16::Bc => (R8::B, R8::C),
            R16::De => (R8::D, R8::E),
            R16::Hl => (R8::H, R8::L),
            R16::Sp => unimplemented!("SP isn't in the JIT prelude/epilogue yet."),
        };

        self.local_get(r8_to_reg_param(high_reg))
            .i32_const(8)
            .i32_shl()
            .local_get(r8_to_reg_param(low_reg))
            .i32_or()
    }

    /// Clear all bits in the flag register.
    /// # Signature
    /// ```
    /// () -> ()
    /// ```
    /// # Pseudocode
    /// ```
    /// F = 0x00
    /// ```
    fn clear_flags(&mut self) -> &mut Self {
        self.i32_const(0x00).local_set(F)
    }

    /// Assign the bits in the flag register, overwriting any previous value.
    /// # Signature
    /// ```
    /// () -> ()
    /// ```
    /// # Pseudocode
    /// ```
    /// F = flag_bits
    /// ```
    fn assign_flags(
        &mut self,
        zero: bool,
        subtraction: bool,
        half_carry: bool,
        carry: bool,
    ) -> &mut Self {
        let mut flags: u8 = 0b0000_0000;
        if zero {
            flags |= 1 << FlagBit::Zero as usize;
        }
        if subtraction {
            flags |= 1 << FlagBit::Subtraction as usize;
        }
        if half_carry {
            flags |= 1 << FlagBit::HalfCarry as usize;
        }
        if carry {
            flags |= 1 << FlagBit::Carry as usize;
        }

        self.i32_const(flags as i32).local_set(F)
    }

    /// Set the selected bit in the flag register. This will only change a 0 to a 1, not vice-versa.
    /// # Signature
    /// ```
    /// (bool: i32) -> ()
    /// ```
    /// # Pseudocode
    /// ```
    /// F |= (top_of_stack << flag_bit)
    /// ```
    fn set_flag(&mut self, flag_bit: FlagBit) -> &mut Self {
        self.i32_const(flag_bit as i32)
            .i32_shl()
            .local_get(F)
            .i32_or()
            .local_set(F)
    }

    /// Check if the selected flag's bit is set in the flag register.
    /// # Signature
    /// ```
    /// () -> (bool: i32)
    /// ```
    /// # Pseudocode
    /// ```
    /// return (F >> flag_bit) & 1
    /// ```
    fn check_flag(&mut self, flag_bit: FlagBit) -> &mut Self {
        self.local_get(F)
            .i32_const(flag_bit as i32)
            .i32_shr_u()
            .i32_const(0x01)
            .i32_and()
    }

    /// Return all of the registers to satisfy the calling convention.
    /// Usually this is the final macro in a block.
    /// # Signature
    /// ```
    /// () -> (i32, i32, i32, i32, i32, i32, i32, i32)
    /// ```
    fn return_regs(&mut self) -> &mut Self {
        self.local_get(A)
            .local_get(F)
            .local_get(B)
            .local_get(C)
            .local_get(D)
            .local_get(E)
            .local_get(H)
            .local_get(L)
    }

    /// Write a byte to the specified address in the Game Boy's memory.
    /// # Signature
    /// ```
    /// (addr: i32, value: i32, delta_m_cycles: i32) -> ()
    /// ```
    fn call_write_byte(&mut self) -> &mut Self {
        self.call(0)
    }
}
