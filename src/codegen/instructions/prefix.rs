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
    fn sla_r(&mut self, ctx: &mut CodegenCtx, r8: R8) -> &mut Self;
    fn swap_r(&mut self, ctx: &mut CodegenCtx, r8: R8) -> &mut Self;
}

impl Prefix for InstructionSink<'_> {
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
            /* Perform the swap:
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
}
