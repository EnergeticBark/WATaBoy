use sm83_interp::cpu::opcodes::parameters::R8;

use crate::codegen::{
    CodegenCtx,
    macros::{FlagBit, Sm83Macros},
    module::PROLOGE_LENGTH,
};

use wasm_encoder::*;

// Emit Wasm bytecode for Block 1.
// See: https://gbdev.io/pandocs/CPU_Instruction_Set.html#block-1-8-bit-register-to-register-loads
pub trait Prefix {
    fn rrc_r(&mut self, ctx: &mut CodegenCtx, r8: R8) -> &mut Self;
    fn rl_r(&mut self, ctx: &mut CodegenCtx, r8: R8) -> &mut Self;
    fn sla_r(&mut self, ctx: &mut CodegenCtx, r8: R8) -> &mut Self;
    fn swap_r(&mut self, ctx: &mut CodegenCtx, r8: R8) -> &mut Self;
    fn srl_r(&mut self, ctx: &mut CodegenCtx, r8: R8) -> &mut Self;
    fn bit_b_r(&mut self, ctx: &mut CodegenCtx, bit_index: u8, r8: R8) -> &mut Self;
}

impl Prefix for InstructionSink<'_> {
    fn rrc_r(&mut self, ctx: &mut CodegenCtx, r8: R8) -> &mut Self {
        // Name our scratch registers.
        const R8_VAL: u32 = PROLOGE_LENGTH as u32;
        const BIT_0: u32 = PROLOGE_LENGTH as u32 + 1;
        self.clear_flags()
            .get_r8(ctx, r8)
            .local_tee(R8_VAL)
            // *** Calculate Zero Flag. ***
            .i32_eqz()
            .set_flag(FlagBit::Zero)
            .local_get(R8_VAL)
            /* Calculate the Carry flag:
             * (R8_VAL & 0b0000_0001) == 0b0000_0001
             */
            .i32_const(0b0000_0001)
            .i32_and()
            .local_tee(BIT_0)
            .set_flag(FlagBit::Carry)
            /* Perform the shift right and set the highest bit to BIT_0:
             * R8_VAL = (R8_VAL >> 1) | (BIT_0 << 7)
             */
            .local_get(R8_VAL)
            .i32_const(1)
            .i32_shr_u()
            .local_get(BIT_0)
            .i32_const(7)
            .i32_shl()
            .i32_or()
            .set_r8(ctx, r8)
    }

    fn rl_r(&mut self, ctx: &mut CodegenCtx, r8: R8) -> &mut Self {
        // Name our scratch registers.
        const R8_VAL: u32 = PROLOGE_LENGTH as u32;
        const CARRY: u32 = PROLOGE_LENGTH as u32 + 1;
        self.check_flag(FlagBit::Carry)
            .local_set(CARRY)
            .clear_flags()
            .get_r8(ctx, r8)
            .local_tee(R8_VAL)
            /* Calculate the Carry flag:
             * (R8_VAL & 0b1000_0000) != 0
             */
            .i32_const(0b1000_0000)
            .i32_and()
            .i32_const(0)
            .i32_ne()
            .set_flag(FlagBit::Carry)
            /* Perform the shift left and set the lowest bit to CARRY:
             * R8_VAL = ((R8_VAL << 1) | CARRY) & 0xFF
             */
            .local_get(R8_VAL)
            .i32_const(1)
            .i32_shl()
            .local_get(CARRY)
            .i32_or()
            .i32_const(0xff)
            .i32_and()
            .local_tee(R8_VAL)
            // *** Calculate Zero Flag. ***
            .i32_eqz()
            .set_flag(FlagBit::Zero)
            // *** Assign the new value to R8. ***
            .local_get(R8_VAL)
            .set_r8(ctx, r8)
    }

    fn sla_r(&mut self, ctx: &mut CodegenCtx, r8: R8) -> &mut Self {
        // Name our scratch register.
        const R8_VAL: u32 = PROLOGE_LENGTH as u32;
        self.clear_flags()
            .get_r8(ctx, r8)
            .local_tee(R8_VAL)
            // *** Calculate Carry Flag. ***
            .i32_const(7)
            .i32_shr_u()
            .set_flag(FlagBit::Carry)
            /* Perform the shift left:
             * R8_VAL = (R8_VAL << 1) & 0xFF
             */
            .local_get(R8_VAL)
            .i32_const(1)
            .i32_shl()
            .i32_const(0xff)
            .i32_and()
            .local_tee(R8_VAL)
            // *** Calculate Zero Flag. ***
            .i32_eqz()
            .set_flag(FlagBit::Zero)
            // *** Assign the new value to R8. ***
            .local_get(R8_VAL)
            .set_r8(ctx, r8)
    }

    fn swap_r(&mut self, ctx: &mut CodegenCtx, r8: R8) -> &mut Self {
        // Name our scratch register.
        const R8_VAL: u32 = PROLOGE_LENGTH as u32;
        self.clear_flags()
            .get_r8(ctx, r8)
            .local_tee(R8_VAL)
            // *** Calculate Zero Flag. ***
            .i32_eqz()
            .set_flag(FlagBit::Zero)
            /* Perform the swap:
             * R8 = ((R8_VAL << 4) | (R8_VAL >> 4)) & 0xFF
             */
            .local_get(R8_VAL)
            .i32_const(4)
            .i32_shl()
            .local_get(R8_VAL)
            .i32_const(4)
            .i32_shr_u()
            .i32_or()
            .i32_const(0xff)
            .i32_and()
            .set_r8(ctx, r8)
    }

    fn srl_r(&mut self, ctx: &mut CodegenCtx, r8: R8) -> &mut Self {
        // Name our scratch registers.
        const R8_VAL: u32 = PROLOGE_LENGTH as u32;
        self.clear_flags()
            .get_r8(ctx, r8)
            .local_tee(R8_VAL)
            /* Calculate the Carry flag:
             * (R8_VAL & 0b0000_0001) == 0b0000_0001
             */
            .i32_const(0b0000_0001)
            .i32_and()
            .set_flag(FlagBit::Carry)
            /* Perform the shift right:
             * R8_VAL = (R8_VAL >> 1)
             */
            .local_get(R8_VAL)
            .i32_const(1)
            .i32_shr_u()
            .local_tee(R8_VAL)
            // *** Calculate Zero Flag. ***
            .i32_eqz()
            .set_flag(FlagBit::Zero)
            // *** Assign the new value to R8. ***
            .local_get(R8_VAL)
            .set_r8(ctx, r8)
    }

    fn bit_b_r(&mut self, ctx: &mut CodegenCtx, bit_index: u8, r8: R8) -> &mut Self {
        self.check_flag(FlagBit::Carry) // *** Preserve the original value of Carry on the stack. ***
            .assign_flags(false, false, true, false)
            .set_flag(FlagBit::Carry) // Restore Carry flag.
            .get_r8(ctx, r8)
            /* Calculate the Zero flag:
             * (R8_VAL >> bit_index) & 0b0000_0001 == 0
             */
            .i32_const(bit_index as i32)
            .i32_shr_u()
            .i32_const(0b0000_0001)
            .i32_and()
            .i32_eqz()
            .set_flag(FlagBit::Zero)
    }
}
