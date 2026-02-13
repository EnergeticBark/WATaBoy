use wasm_encoder::InstructionSink;

use crate::codegen::registers::{A, B, C, D, E, F, H, L};

pub enum FlagBit {
    Zero = 7,
    Subtraction = 6,
    HalfCarry = 5,
    Carry = 4,
}

pub trait Sm83Macros {
    fn clear_flags(&mut self) -> &mut Self;
    fn set_flag(&mut self, flag_bit: FlagBit) -> &mut Self;
    fn return_regs(&mut self) -> &mut Self;
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
}
