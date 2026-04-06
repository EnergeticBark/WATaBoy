use sm83_interp::cpu::opcodes::parameters::R8;

use crate::codegen::{macros::Sm83Macros, registers::r8_to_reg_param};

use wasm_encoder::*;

// Emit Wasm bytecode for Block 1.
// See: https://gbdev.io/pandocs/CPU_Instruction_Set.html#block-1-8-bit-register-to-register-loads
pub trait Block1 {
    fn ld_hl_r(&mut self, r8_src: R8, delta_m_cycles: u16) -> &mut Self;
    fn ld_r_r(&mut self, r8_dst: R8, r8_src: R8) -> &mut Self;
}

impl Block1 for InstructionSink<'_> {
    fn ld_hl_r(&mut self, r8_src: R8, delta_m_cycles: u16) -> &mut Self {
        self.local_get(r8_to_reg_param(R8::H))
            .i32_const(8)
            .i32_shl()
            .local_get(r8_to_reg_param(R8::L))
            .i32_or()
            .local_get(r8_to_reg_param(r8_src))
            .i32_const(delta_m_cycles as i32)
            .call_write_byte()
    }

    fn ld_r_r(&mut self, r8_dst: R8, r8_src: R8) -> &mut Self {
        self.local_get(r8_to_reg_param(r8_src))
            .local_set(r8_to_reg_param(r8_dst))
    }
}
