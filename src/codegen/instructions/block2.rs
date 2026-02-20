use sm83_interp::parameters::R8;

use crate::codegen::macros::{FlagBit, Sm83Macros};
use crate::codegen::registers::{A, r8_to_reg_param};

use wasm_encoder::*;

// Emit Wasm bytecode for Block 2.
// See: https://gbdev.io/pandocs/CPU_Instruction_Set.html#block-2-8-bit-arithmetic
pub trait Block2 {
    fn add_r(&mut self, r8: R8) -> &mut Self;
    fn adc_r(&mut self, r8: R8) -> &mut Self;
    fn sub_r(&mut self, r8: R8) -> &mut Self;
    fn and_r(&mut self, r8: R8) -> &mut Self;
}

impl Block2 for InstructionSink<'_> {
    fn add_r(&mut self, r8: R8) -> &mut Self {
        self.clear_flags() // Maybe add a macro for *assigning* a flag too so we don't have to do this separately from setting the first flag.
            /* Calculate Half-Carry Flag:
             * ((A & 0x0f) + (R8 & 0x0f)) > 0x0f
             */
            .local_get(A)
            .i32_const(0x0f)
            .i32_and() // (A & 0x0f)
            .local_get(r8_to_reg_param(r8))
            .i32_const(0x0f)
            .i32_and() // (R8 & 0x0f)
            .i32_add()
            .i32_const(0x0f)
            .i32_gt_u()
            .set_flag(FlagBit::HalfCarry)
            /* Perform the addition (result not yet truncated):
             * A = A + R8
             */
            .local_get(A)
            .local_get(r8_to_reg_param(r8))
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

    fn adc_r(&mut self, r8: R8) -> &mut Self {
        // Name our scratch register.
        const PREV_CARRY: u32 = 8;
        self.check_flag(FlagBit::Carry) // *** Store original value of Carry. ***
            .local_set(PREV_CARRY)
            .clear_flags()
            /* Calculate Half-Carry Flag:
             * ((A & 0x0f) + (R8 & 0x0f)) + PREV_CARRY  > 0x0f
             */
            .local_get(A)
            .i32_const(0x0f)
            .i32_and() // (A & 0x0f)
            .local_get(r8_to_reg_param(r8))
            .i32_const(0x0f)
            .i32_and() // (R8 & 0x0f)
            .i32_add()
            .local_get(PREV_CARRY)
            .i32_add()
            .i32_const(0x0f)
            .i32_gt_u()
            .set_flag(FlagBit::HalfCarry)
            /* Perform the addition (result not yet truncated):
             * A = A + R8 + PREV_CARRY
             */
            .local_get(A)
            .local_get(r8_to_reg_param(r8))
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

    fn sub_r(&mut self, r8: R8) -> &mut Self {
        self.assign_flags(false, true, false, false) // Always set subtraction to 1.
            /* Calculate Half-Carry Flag:
             * (A & 0x0f) < (R8 & 0x0f)
             */
            .local_get(A)
            .i32_const(0x0f)
            .i32_and() // (A & 0x0f)
            .local_get(r8_to_reg_param(r8))
            .i32_const(0x0f)
            .i32_and() // (R8 & 0x0f)
            .i32_lt_u()
            .set_flag(FlagBit::HalfCarry)
            /* Calculate Carry Flag:
             * A < R8
             */
            .local_get(A)
            .local_get(r8_to_reg_param(r8))
            .i32_lt_u() // If A < R8 (underflow), then 1, otherwise 0.
            .set_flag(FlagBit::Carry)
            /* Perform the subtraction:
             * A = (A - R8) & 0xff
             */
            .local_get(A)
            .local_get(r8_to_reg_param(r8))
            .i32_sub()
            .i32_const(0xff)
            .i32_and()
            .local_tee(A)
            // *** Calculate Zero Flag. ***
            .i32_eqz() // If the A is zero, then 1, otherwise 0.
            .set_flag(FlagBit::Zero)
    }

    fn and_r(&mut self, r8: R8) -> &mut Self {
        self.assign_flags(false, false, true, false) // Always set half-carry to 1.
            .local_get(A)
            .local_get(r8_to_reg_param(r8))
            /* Perform the and:
             * A = A & R8
             */
            .i32_and()
            .local_tee(A)
            // *** Calculate Zero Flag. ***
            .i32_eqz() // If the A is zero, then 1, otherwise 0.
            .set_flag(FlagBit::Zero)
    }
}
