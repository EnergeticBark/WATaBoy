use interpreter::cpu::opcodes::parameters::R8;

use crate::codegen::CodegenCtx;
use crate::codegen::macros::{FlagBit, Sm83Macros};
use crate::codegen::module::PROLOGE_LENGTH;
use crate::codegen::registers::A;

use wasm_encoder::InstructionSink;

// Emit Wasm bytecode for Block 2.
// See: https://gbdev.io/pandocs/CPU_Instruction_Set.html#block-2-8-bit-arithmetic
pub trait Block2 {
    fn add_r(&mut self, ctx: &mut CodegenCtx, r8: R8) -> &mut Self;
    fn adc_r(&mut self, ctx: &mut CodegenCtx, r8: R8) -> &mut Self;
    fn sub_r(&mut self, ctx: &mut CodegenCtx, r8: R8) -> &mut Self;
    fn sbc_r(&mut self, ctx: &mut CodegenCtx, r8: R8) -> &mut Self;
    fn and_r(&mut self, ctx: &mut CodegenCtx, r8: R8) -> &mut Self;
    fn xor_r(&mut self, ctx: &mut CodegenCtx, r8: R8) -> &mut Self;
    fn or_r(&mut self, ctx: &mut CodegenCtx, r8: R8) -> &mut Self;
    fn cp_r(&mut self, ctx: &mut CodegenCtx, r8: R8) -> &mut Self;
}

impl Block2 for InstructionSink<'_> {
    fn add_r(&mut self, ctx: &mut CodegenCtx, r8: R8) -> &mut Self {
        // Name our scratch register.
        const R8_VAL: u32 = PROLOGE_LENGTH;
        self.clear_flags() // Maybe add a macro for *assigning* a flag too so we don't have to do this separately from setting the first flag.
            .get_r8(ctx, r8)
            .local_set(R8_VAL)
            /* Calculate Half-Carry Flag:
             * ((A & 0x0f) + (R8 & 0x0f)) > 0x0f
             */
            .local_get(A)
            .i32_const(0x0f)
            .i32_and() // (A & 0x0f)
            .local_get(R8_VAL)
            .i32_const(0x0f)
            .i32_and() // (R8 & 0x0f)
            .i32_add()
            .i32_const(0x0f)
            .i32_gt_u()
            .set_flag(FlagBit::HalfCarry)
            /* Perform the ADD (result not yet truncated):
             * A = A + R8
             */
            .local_get(A)
            .local_get(R8_VAL)
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
    }

    fn adc_r(&mut self, ctx: &mut CodegenCtx, r8: R8) -> &mut Self {
        // Name our scratch registers.
        const PREV_CARRY: u32 = PROLOGE_LENGTH;
        const R8_VAL: u32 = PROLOGE_LENGTH + 1;
        self.check_flag(FlagBit::Carry) // *** Store original value of Carry. ***
            .local_set(PREV_CARRY)
            .clear_flags()
            .get_r8(ctx, r8)
            .local_set(R8_VAL)
            /* Calculate Half-Carry Flag:
             * ((A & 0x0f) + (R8 & 0x0f)) + PREV_CARRY  > 0x0f
             */
            .local_get(A)
            .i32_const(0x0f)
            .i32_and() // (A & 0x0f)
            .local_get(R8_VAL)
            .i32_const(0x0f)
            .i32_and() // (R8 & 0x0f)
            .i32_add()
            .local_get(PREV_CARRY)
            .i32_add()
            .i32_const(0x0f)
            .i32_gt_u()
            .set_flag(FlagBit::HalfCarry)
            /* Perform the ADD (result not yet truncated):
             * A = A + R8 + PREV_CARRY
             */
            .local_get(A)
            .local_get(R8_VAL)
            .i32_add()
            .local_get(PREV_CARRY)
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
    }

    fn sub_r(&mut self, ctx: &mut CodegenCtx, r8: R8) -> &mut Self {
        // Name our scratch register.
        const R8_VAL: u32 = PROLOGE_LENGTH;
        self.assign_flags(false, true, false, false) // Always set subtraction to 1.
            .get_r8(ctx, r8)
            .local_set(R8_VAL)
            /* Calculate Half-Carry Flag:
             * (A & 0x0f) < (R8 & 0x0f)
             */
            .local_get(A)
            .i32_const(0x0f)
            .i32_and() // (A & 0x0f)
            .local_get(R8_VAL)
            .i32_const(0x0f)
            .i32_and() // (R8 & 0x0f)
            .i32_lt_u()
            .set_flag(FlagBit::HalfCarry)
            /* Calculate Carry Flag:
             * A < R8
             */
            .local_get(A)
            .local_get(R8_VAL)
            .i32_lt_u() // If A < R8 (underflow), then 1, otherwise 0.
            .set_flag(FlagBit::Carry)
            /* Perform the SUB:
             * A = (A - R8) & 0xff
             */
            .local_get(A)
            .local_get(R8_VAL)
            .i32_sub()
            .i32_const(0xff)
            .i32_and()
            .local_tee(A)
            // *** Calculate Zero Flag. ***
            .i32_eqz() // If the A is zero, then 1, otherwise 0.
            .set_flag(FlagBit::Zero)
    }

    fn sbc_r(&mut self, ctx: &mut CodegenCtx, r8: R8) -> &mut Self {
        // Name our scratch registers.
        const PREV_CARRY: u32 = PROLOGE_LENGTH;
        const R8_VAL: u32 = PROLOGE_LENGTH + 1;
        self.check_flag(FlagBit::Carry) // *** Store original value of Carry. ***
            .local_set(PREV_CARRY)
            .assign_flags(false, true, false, false) // Always set subtraction to 1.
            .get_r8(ctx, r8)
            .local_set(R8_VAL)
            /* Calculate Half-Carry Flag:
             * (A & 0x0f) < ((R8 & 0x0f) + PREV_CARRY)
             */
            .local_get(A)
            .i32_const(0x0f)
            .i32_and() // (A & 0x0f)
            .local_get(R8_VAL)
            .i32_const(0x0f)
            .i32_and()
            .local_get(PREV_CARRY)
            .i32_add() // ((R8 & 0x0f) + PREV_CARRY)
            .i32_lt_u()
            .set_flag(FlagBit::HalfCarry)
            /* Calculate Carry Flag:
             * A < (R8 + PREV_CARRY)
             */
            .local_get(A)
            .local_get(R8_VAL)
            .local_get(PREV_CARRY)
            .i32_add()
            .i32_lt_u() // If A < (R8 + PREV_CARRY) (underflow), then 1, otherwise 0.
            .set_flag(FlagBit::Carry)
            /* Perform the SUB:
             * A = (A - (R8 + PREV_CARRY)) & 0xff
             */
            .local_get(A)
            .local_get(R8_VAL)
            .local_get(PREV_CARRY)
            .i32_add() // (R8 + PREV_CARRY)
            .i32_sub()
            .i32_const(0xff)
            .i32_and()
            .local_tee(A)
            // *** Calculate Zero Flag. ***
            .i32_eqz() // If the A is zero, then 1, otherwise 0.
            .set_flag(FlagBit::Zero)
    }

    fn and_r(&mut self, ctx: &mut CodegenCtx, r8: R8) -> &mut Self {
        self.assign_flags(false, false, true, false) // Always set half-carry to 1.
            .local_get(A)
            .get_r8(ctx, r8)
            /* Perform the AND:
             * A = A & R8
             */
            .i32_and()
            .local_tee(A)
            // *** Calculate Zero Flag. ***
            .i32_eqz() // If the A is zero, then 1, otherwise 0.
            .set_flag(FlagBit::Zero)
    }

    fn xor_r(&mut self, ctx: &mut CodegenCtx, r8: R8) -> &mut Self {
        self.clear_flags()
            .local_get(A)
            .get_r8(ctx, r8)
            /* Perform the XOR:
             * A = A ^ R8
             */
            .i32_xor()
            .local_tee(A)
            // *** Calculate Zero Flag. ***
            .i32_eqz() // If the A is zero, then 1, otherwise 0.
            .set_flag(FlagBit::Zero)
    }

    fn or_r(&mut self, ctx: &mut CodegenCtx, r8: R8) -> &mut Self {
        self.clear_flags()
            .local_get(A)
            .get_r8(ctx, r8)
            /* Perform the OR:
             * A = A | R8
             */
            .i32_or()
            .local_tee(A)
            // *** Calculate Zero Flag. ***
            .i32_eqz() // If the A is zero, then 1, otherwise 0.
            .set_flag(FlagBit::Zero)
    }

    // Identical to SUB r but doesn't update A.
    fn cp_r(&mut self, ctx: &mut CodegenCtx, r8: R8) -> &mut Self {
        // Name our scratch register.
        const R8_VAL: u32 = PROLOGE_LENGTH;
        self.assign_flags(false, true, false, false) // Always set subtraction to 1.
            .get_r8(ctx, r8)
            .local_set(R8_VAL)
            /* Calculate Half-Carry Flag:
             * (A & 0x0f) < (R8 & 0x0f)
             */
            .local_get(A)
            .i32_const(0x0f)
            .i32_and() // (A & 0x0f)
            .local_get(R8_VAL)
            .i32_const(0x0f)
            .i32_and() // (R8 & 0x0f)
            .i32_lt_u()
            .set_flag(FlagBit::HalfCarry)
            /* Calculate Carry Flag:
             * A < R8
             */
            .local_get(A)
            .local_get(R8_VAL)
            .i32_lt_u() // If A < R8 (underflow), then 1, otherwise 0.
            .set_flag(FlagBit::Carry)
            /* Perform the SUB:
             * (A - R8) & 0xff
             */
            .local_get(A)
            .local_get(R8_VAL)
            .i32_sub()
            .i32_const(0xff)
            .i32_and()
            // *** Calculate Zero Flag. ***
            .i32_eqz() // If the result is zero, then 1, otherwise 0.
            .set_flag(FlagBit::Zero)
    }
}
