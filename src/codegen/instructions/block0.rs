use wasm_encoder::*;

// Emit Wasm bytecode for Block 0.
// See: https://gbdev.io/pandocs/CPU_Instruction_Set.html#block-0
pub trait Block0 {
    fn nop(&mut self) -> &mut Self;
}

impl Block0 for InstructionSink<'_> {
    fn nop(&mut self) -> &mut Self {
        self.nop()
    }
}
