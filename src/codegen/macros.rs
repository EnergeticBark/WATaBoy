use wasm_encoder::InstructionSink;

use crate::codegen::registers::{A, B, C, D, E, F, H, L};

pub(crate) enum FlagBit {
    Zero = 7,
    Subtraction = 6,
    HalfCarry = 5,
    Carry = 4,
}

pub(crate) trait Sm83Macros {
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
