use sm83_interp::cpu::opcodes::parameters::{R8, R16, R16Stack};
use wasm_encoder::InstructionSink;

use crate::codegen::{
    CodegenCtx,
    registers::{A, B, C, D, E, F, H, L, SP, r8_to_reg_param},
};

pub(crate) enum FlagBit {
    Zero = 7,
    Subtraction = 6,
    HalfCarry = 5,
    Carry = 4,
}

pub(crate) trait Sm83Macros {
    fn get_r8(&mut self, ctx: &mut CodegenCtx, r8: R8) -> &mut Self;
    fn set_r8(&mut self, ctx: &mut CodegenCtx, r8: R8) -> &mut Self;
    fn get_r16(&mut self, r16: R16) -> &mut Self;
    fn set_r16_stack(&mut self, r16: R16Stack) -> &mut Self;
    fn pop_byte(&mut self, ctx: &mut CodegenCtx) -> &mut Self;
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
    fn call_read_byte(&mut self) -> &mut Self;
}

impl Sm83Macros for InstructionSink<'_> {
    /// Get the value of the specified 8-bit register.
    /// If R8 is [HL], delta_m_cycles will reset to 0 and total_m_cycles will increase by 1.
    /// # Signature
    /// ```
    /// () -> (value: i32)
    /// ```
    fn get_r8(&mut self, ctx: &mut CodegenCtx, r8: R8) -> &mut Self {
        match r8 {
            R8::IndirectHL => {
                // Account for the m_cycle already spent fetching this instruction.
                ctx.delta_m_cycles += 1;
                ctx.total_m_cycles += 1;
                let sink = self
                    .get_r16(R16::Hl)
                    .i32_const(ctx.delta_m_cycles as i32)
                    .call_read_byte();
                // Reset delta_m_cycles, because the system clock just caught up.
                ctx.delta_m_cycles = 0;
                sink
            }
            _ => self.local_get(r8_to_reg_param(r8)),
        }
    }

    /// Set the value of the specified 8-bit register.
    /// If R8 is [HL], delta_m_cycles will reset to 0 and total_m_cycles will increase by 1.
    /// # Signature
    /// ```
    /// (value: i32) -> ()
    /// ```
    fn set_r8(&mut self, ctx: &mut CodegenCtx, r8: R8) -> &mut Self {
        match r8 {
            R8::IndirectHL => {
                // Account for the m_cycle already spent fetching this instruction.
                ctx.delta_m_cycles += 1;
                ctx.total_m_cycles += 1;
                let sink = self
                    .get_r16(R16::Hl)
                    .i32_const(ctx.delta_m_cycles as i32)
                    .call_write_byte();
                // Reset delta_m_cycles, because the system clock just caught up.
                ctx.delta_m_cycles = 0;
                sink
            }
            _ => self.local_set(r8_to_reg_param(r8)),
        }
    }

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

    /// Set the value of the specified 16-bit stack register.
    /// # Signature
    /// ```
    /// (high_byte: i32, low_byte: i32) -> ()
    /// ```
    fn set_r16_stack(&mut self, r16: R16Stack) -> &mut Self {
        let (high_reg, low_reg) = match r16 {
            R16Stack::Bc => (B, C),
            R16Stack::De => (D, E),
            R16Stack::Hl => (H, L),
            // TODO: Don't set the lower nibble of F!!!
            R16Stack::Af => (A, F),
        };

        self.local_set(high_reg).local_set(low_reg)
    }

    /// Pop an 8-bit value from the stack.
    /// # Signature
    /// ```
    /// () -> (value: i32)
    /// ```
    /// # Side Effects
    /// Increments the system clock by 1 M-cycle after reading SP.
    /// # Pseudocode
    /// ```
    /// mem[SP++]
    /// ```
    fn pop_byte(&mut self, ctx: &mut CodegenCtx) -> &mut Self {
        self.local_get(SP)
            .i32_const(ctx.delta_m_cycles as i32)
            .call_read_byte()
            .local_get(SP)
            .i32_const(1)
            .i32_add()
            .local_set(SP);
        // Reset delta_m_cycles, because the system clock just caught up.
        ctx.delta_m_cycles = 0;
        ctx.delta_m_cycles += 1;
        ctx.total_m_cycles += 1;
        self
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
            .local_get(SP)
    }

    /// Read a byte from the specified address in the Game Boy's memory.
    /// # Signature
    /// ```
    /// (addr: i32, delta_m_cycles: i32) -> (value: i32)
    /// ```
    fn call_read_byte(&mut self) -> &mut Self {
        self.call(0)
    }

    /// Write a byte to the specified address in the Game Boy's memory.
    /// # Signature
    /// ```
    /// (value: i32, addr: i32, delta_m_cycles: i32) -> ()
    /// ```
    fn call_write_byte(&mut self) -> &mut Self {
        self.call(1)
    }
}
