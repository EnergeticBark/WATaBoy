use sm83_interp::cpu::opcodes::parameters::R8;

use crate::codegen::macros::{FlagBit, Sm83Macros};
use crate::codegen::registers::r8_to_reg_param;

use wasm_encoder::*;

// Emit Wasm bytecode for Block 0.
// See: https://gbdev.io/pandocs/CPU_Instruction_Set.html#block-0
pub trait Block0 {
    fn nop(&mut self) -> &mut Self;
    fn inc_r(&mut self, r8: R8) -> &mut Self;
}

impl Block0 for InstructionSink<'_> {
    fn nop(&mut self) -> &mut Self {
        self.nop()
    }

    fn inc_r(&mut self, r8: R8) -> &mut Self {
        self.check_flag(FlagBit::Carry) // *** Preserve the original value of Carry on the stack. ***
            .clear_flags()
            .set_flag(FlagBit::Carry) // Restore Carry flag.
            /* Perform the increment and truncate:
             * R8 = (R8 + 1) & 0xff
             */
            .local_get(r8_to_reg_param(r8))
            .i32_const(1)
            .i32_add()
            .i32_const(0xff)
            .i32_and()
            .local_tee(r8_to_reg_param(r8))
            /* Calculate Half-Carry Flag:
             * R8.trailing_zeros() >= 4
             */
            .i32_ctz()
            .i32_const(3)
            .i32_gt_u()
            .set_flag(FlagBit::HalfCarry)
            // *** Calculate Zero Flag. ***
            .local_get(r8_to_reg_param(r8))
            .i32_eqz() // If the R8 is zero, then 1, otherwise 0.
            .set_flag(FlagBit::Zero)
    }
}
