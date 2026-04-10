use sm83_interp::cpu::opcodes::parameters::{R8, R16};

use crate::codegen::CodegenCtx;
use crate::codegen::macros::{FlagBit, Sm83Macros};
use crate::codegen::module::PROLOGE_LENGTH;

use wasm_encoder::*;

// Emit Wasm bytecode for Block 0.
// See: https://gbdev.io/pandocs/CPU_Instruction_Set.html#block-0
pub trait Block0 {
    fn nop(&mut self) -> &mut Self;
    fn inc_rr(&mut self, r16: R16) -> &mut Self;
    fn dec_rr(&mut self, r16: R16) -> &mut Self;
    fn inc_r(&mut self, ctx: &mut CodegenCtx, r8: R8) -> &mut Self;
    fn dec_r(&mut self, ctx: &mut CodegenCtx, r8: R8) -> &mut Self;
    fn ld_r_n(&mut self, ctx: &mut CodegenCtx, r8: R8, imm: i32) -> &mut Self;
}

impl Block0 for InstructionSink<'_> {
    fn nop(&mut self) -> &mut Self {
        self.nop()
    }

    fn inc_rr(&mut self, r16: R16) -> &mut Self {
        // Name our scratch register.
        const TEMP: u32 = PROLOGE_LENGTH as u32;
        self.get_r16(r16).i32_const(1).i32_add().set_r16(r16, TEMP)
    }

    fn dec_rr(&mut self, r16: R16) -> &mut Self {
        // Name our scratch register.
        const TEMP: u32 = PROLOGE_LENGTH as u32;
        self.get_r16(r16).i32_const(1).i32_sub().set_r16(r16, TEMP)
    }

    fn inc_r(&mut self, ctx: &mut CodegenCtx, r8: R8) -> &mut Self {
        // Name our scratch register.
        const RESULT: u32 = PROLOGE_LENGTH as u32;
        self.check_flag(FlagBit::Carry) // *** Preserve the original value of Carry on the stack. ***
            .clear_flags()
            .set_flag(FlagBit::Carry) // Restore Carry flag.
            /* Perform the increment and truncate:
             * R8 = (R8 + 1) & 0xff
             */
            .get_r8(ctx, r8)
            .i32_const(1)
            .i32_add()
            .i32_const(0xff)
            .i32_and()
            .local_tee(RESULT)
            .set_r8(ctx, r8)
            .local_get(RESULT)
            /* Calculate Half-Carry Flag:
             * RESULT.trailing_zeros() >= 4
             */
            .i32_ctz()
            .i32_const(3)
            .i32_gt_u()
            .set_flag(FlagBit::HalfCarry)
            // *** Calculate Zero Flag. ***
            .local_get(RESULT)
            .i32_eqz() // If the R8 is zero, then 1, otherwise 0.
            .set_flag(FlagBit::Zero)
    }

    fn dec_r(&mut self, ctx: &mut CodegenCtx, r8: R8) -> &mut Self {
        // Name our scratch register.
        const RESULT: u32 = PROLOGE_LENGTH as u32;
        const R8_VAL: u32 = PROLOGE_LENGTH as u32 + 1;
        self.check_flag(FlagBit::Carry) // *** Preserve the original value of Carry on the stack. ***
            .assign_flags(false, true, false, false) // Always set subtraction to 1.
            .set_flag(FlagBit::Carry) // Restore Carry flag.
            /* Perform the decrement and truncate:
             * R8 = (R8 - 1) & 0xff
             */
            .get_r8(ctx, r8)
            .local_tee(R8_VAL)
            .i32_const(1)
            .i32_sub()
            .i32_const(0xff)
            .i32_and()
            .local_tee(RESULT)
            .set_r8(ctx, r8)
            .local_get(R8_VAL)
            /* Calculate Half-Carry Flag:
             * R8.trailing_zeros() >= 4
             */
            .i32_ctz()
            .i32_const(3)
            .i32_gt_u()
            .set_flag(FlagBit::HalfCarry)
            // *** Calculate Zero Flag. ***
            .local_get(RESULT)
            .i32_eqz() // If the R8 is zero, then 1, otherwise 0.
            .set_flag(FlagBit::Zero)
    }

    fn ld_r_n(&mut self, ctx: &mut CodegenCtx, r8: R8, imm: i32) -> &mut Self {
        ctx.delta_m_cycles += 2;
        ctx.total_m_cycles += 2;
        self.i32_const(imm).set_r8(ctx, r8)
    }
}
