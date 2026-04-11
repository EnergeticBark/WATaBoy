use crate::codegen::CodegenCtx;
use crate::codegen::macros::{FlagBit, Sm83Macros};
use crate::codegen::module::PROLOGE_LENGTH;
use crate::codegen::registers::A;

use sm83_interp::cpu::opcodes::parameters::R16Stack;
use wasm_encoder::*;

// Emit Wasm bytecode for Block 1.
// See: https://gbdev.io/pandocs/CPU_Instruction_Set.html#block-1-8-bit-register-to-register-loads
pub trait Block3 {
    fn add_n(&mut self, imm: i32) -> &mut Self;
    fn and_n(&mut self, imm: i32) -> &mut Self;
    fn cp_n(&mut self, imm: i32) -> &mut Self;
    fn pop_rr(&mut self, ctx: &mut CodegenCtx, r16_stack: R16Stack) -> &mut Self;
    fn push_rr(&mut self, ctx: &mut CodegenCtx, r16_stack: R16Stack) -> &mut Self;
    fn ldh_n_a(&mut self, ctx: &mut CodegenCtx, imm: u8) -> &mut Self;
    fn ld_nn_a(&mut self, ctx: &mut CodegenCtx, imm: u16) -> &mut Self;
    fn ldh_a_n(&mut self, ctx: &mut CodegenCtx, imm: u8) -> &mut Self;
    fn ld_a_nn(&mut self, ctx: &mut CodegenCtx, imm: u16) -> &mut Self;
}

impl Block3 for InstructionSink<'_> {
    // TODO: Ensure immediate values in separate ROM banks aren't cached.
    // E.g. 0x3FFF: AddN, 0x4000: 64. A bank switch could invalidate this immediate value.
    fn add_n(&mut self, imm: i32) -> &mut Self {
        // Name our scratch register.
        const PREV_A: u32 = PROLOGE_LENGTH as u32;
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
    fn and_n(&mut self, imm: i32) -> &mut Self {
        self.assign_flags(false, false, true, false) // Always set half-carry to 1.
            .local_get(A)
            .i32_const(imm)
            /* Perform the AND:
             * A = A & R8
             */
            .i32_and()
            .local_tee(A)
            // *** Calculate Zero Flag. ***
            .i32_eqz() // If the A is zero, then 1, otherwise 0.
            .set_flag(FlagBit::Zero)
    }
    fn cp_n(&mut self, imm: i32) -> &mut Self {
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
    fn pop_rr(&mut self, ctx: &mut CodegenCtx, r16_stack: R16Stack) -> &mut Self {
        // Pop the low byte.
        self.pop_byte(ctx)
            // Pop the high byte.
            .pop_byte(ctx)
            .set_r16_stack(r16_stack)
    }
    fn push_rr(&mut self, ctx: &mut CodegenCtx, r16_stack: R16Stack) -> &mut Self {
        ctx.increment_m_cycles(1);
        self.get_r16_stack(r16_stack)
            // Push the high byte.
            .push_byte(ctx)
            // Push the low byte.
            .push_byte(ctx)
    }
    fn ldh_n_a(&mut self, ctx: &mut CodegenCtx, imm: u8) -> &mut Self {
        ctx.increment_m_cycles(1);
        let address = u16::from_le_bytes([imm, 0xFF]);
        self.local_get(A)
            .i32_const(address as i32)
            .call_write_byte(ctx)
            .insert_checkpoint(ctx)
    }
    fn ld_nn_a(&mut self, ctx: &mut CodegenCtx, imm: u16) -> &mut Self {
        ctx.increment_m_cycles(2);
        self.local_get(A).i32_const(imm as i32).call_write_byte(ctx)
    }
    fn ldh_a_n(&mut self, ctx: &mut CodegenCtx, imm: u8) -> &mut Self {
        ctx.increment_m_cycles(1);
        let address = u16::from_le_bytes([imm, 0xFF]);
        self.i32_const(address as i32)
            .call_read_byte(ctx)
            .local_set(A)
    }
    fn ld_a_nn(&mut self, ctx: &mut CodegenCtx, imm: u16) -> &mut Self {
        ctx.increment_m_cycles(2);
        self.i32_const(imm as i32).call_read_byte(ctx).local_set(A)
    }
}
