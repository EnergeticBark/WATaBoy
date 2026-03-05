use sm83_interp::cpu::opcodes::parameters::R8;

use crate::codegen::registers::r8_to_reg_param;

use wasm_encoder::*;

// Emit Wasm bytecode for Block 1.
// See: https://gbdev.io/pandocs/CPU_Instruction_Set.html#block-1-8-bit-register-to-register-loads
pub trait Block1 {
    fn ld_r_r(&mut self, r8_dst: R8, r8_src: R8) -> &mut Self;
}

impl Block1 for InstructionSink<'_> {
    fn ld_r_r(&mut self, r8_dst: R8, r8_src: R8) -> &mut Self {
        self.local_get(r8_to_reg_param(r8_src))
            .local_set(r8_to_reg_param(r8_dst))
    }
}
