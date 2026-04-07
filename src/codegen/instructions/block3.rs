use crate::codegen::macros::{FlagBit, Sm83Macros};
use crate::codegen::registers::A;

use wasm_encoder::*;

// Emit Wasm bytecode for Block 1.
// See: https://gbdev.io/pandocs/CPU_Instruction_Set.html#block-1-8-bit-register-to-register-loads
pub trait Block3 {
    fn add_n(&mut self, imm: i32) -> &mut Self;
    fn cp_n(&mut self, imm: i32) -> &mut Self;
}

impl Block3 for InstructionSink<'_> {
    fn add_n(&mut self, imm: i32) -> &mut Self {
        // TODO: Ensure immediate values in separate ROM banks aren't cached.
        // E.g. 0x3FFF: AddN, 0x4000: 64. A bank switch could invalidate this immediate value.
        // Name our scratch register.
        const PREV_A: u32 = 8;
        self.clear_flags() // Maybe add a macro for *assigning* flags too so we don't have to do this separately from setting the first flag.
            // *** Store original value of A so it can be used to calculate the half-carry. ***
            .local_get(A)
            .local_tee(PREV_A)
            .i32_const(imm)
            /* Perform the addition (result not yet truncated):
             * A = A + IMM
             */
            .i32_add()
            .local_tee(A)
            /* Calculate Overflow Flag:
             * A > 0xff
             */
            .i32_const(0xff)
            .i32_gt_u() // If result > 255 (overflow), then 1, otherwise 0.
            .set_flag(FlagBit::Carry)
            /* Truncate A to 8-bits:
             * A &= 0xff
             */
            .local_get(A)
            .i32_const(0xff)
            .i32_and()
            .local_tee(A)
            // *** Calculate Zero Flag. ***
            .i32_eqz() // If the A is zero, then 1, otherwise 0.
            .set_flag(FlagBit::Zero)
            /* Calculate Half-Carry Flag:
             * ((A & 0x0f) + (IMM & 0x0f)) > 0x0f
             */
            .local_get(PREV_A)
            .i32_const(0x0f)
            .i32_and() // (A & 0x0f)
            .i32_const(imm & 0x0f) // (IMM & 0x0f)
            .i32_add()
            .i32_const(0x0f)
            .i32_gt_u()
            .set_flag(FlagBit::HalfCarry)
    }

    // Block 3
    fn cp_n(&mut self, imm: i32) -> &mut Self {
        // TODO: Ensure immediate values in separate ROM banks aren't cached.
        // E.g. 0x3FFF: AddN, 0x4000: 64. A bank switch could invalidate this immediate value.
        self.assign_flags(false, true, false, false) // Always set subtraction to 1.
            /* Calculate Half-Carry Flag:
             * (A & 0x0f) < (R8 & 0x0f)
             */
            .local_get(A)
            .i32_const(0x0f)
            .i32_and() // (A & 0x0f)
            .i32_const(imm & 0x0f)
            .i32_lt_u()
            .set_flag(FlagBit::HalfCarry)
            /* Calculate Carry Flag:
             * A < R8
             */
            .local_get(A)
            .i32_const(imm)
            .i32_lt_u() // If A < R8 (underflow), then 1, otherwise 0.
            .set_flag(FlagBit::Carry)
            /* Perform the SUB:
             * (A - R8) & 0xff
             */
            .local_get(A)
            .i32_const(imm)
            .i32_sub()
            .i32_const(0xff)
            .i32_and()
            // *** Calculate Zero Flag. ***
            .i32_eqz() // If the result is zero, then 1, otherwise 0.
            .set_flag(FlagBit::Zero)
    }
}
