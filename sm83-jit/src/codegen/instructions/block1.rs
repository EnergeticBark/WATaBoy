use interpreter::cpu::opcodes::parameters::R8;

use crate::codegen::{CodegenCtx, macros::Sm83Macros};

use wasm_encoder::*;

// Emit Wasm bytecode for Block 1.
// See: https://gbdev.io/pandocs/CPU_Instruction_Set.html#block-1-8-bit-register-to-register-loads
pub trait Block1 {
    fn ld_r_r(&mut self, ctx: &mut CodegenCtx, r8_dst: R8, r8_src: R8) -> &mut Self;
}

impl Block1 for InstructionSink<'_> {
    fn ld_r_r(&mut self, ctx: &mut CodegenCtx, r8_dst: R8, r8_src: R8) -> &mut Self {
        self.get_r8(ctx, r8_src).set_r8(ctx, r8_dst)
    }
}
