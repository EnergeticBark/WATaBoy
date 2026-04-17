// TODO: If block_cache becomes more complicated than just a HashMap move it to this file.
// Possibly alongside lowestSafeFuncIndex calculations.

use crate::codegen::Checkpoint;

pub struct CompiledBlock {
    // Func index for the runtime's Wasm table.
    pub func_idx: i32,
    pub checkpoints: Vec<Checkpoint>,
}
