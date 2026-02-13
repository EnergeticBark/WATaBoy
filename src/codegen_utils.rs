use wasm_encoder::InstructionSink;

pub enum FlagBit {
    Zero = 7,
    Subtraction = 6,
    HalfCarry = 5,
    Carry = 4,
}

pub trait Sm83Macros {
    fn set_flag(&mut self, flag_bit: FlagBit) -> &mut Self;
}

const F: u32 = 1;

impl Sm83Macros for InstructionSink<'_> {
    /// Signature: (bool: i32) -> ()
    /// Set the selected bit in the flag register. This will only change a 0 to a 1, not vice-versa.
    /// `F |= (top_of_stack << flag_bit)`
    fn set_flag(&mut self, flag_bit: FlagBit) -> &mut Self {
        self.i32_const(flag_bit as i32)
            .i32_shl()
            .local_get(F)
            .i32_or()
            .local_set(F)
    }
}
