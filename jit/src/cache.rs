// TODO: If block_cache becomes more complicated than just a HashMap move it to this file.
// Possibly alongside lowestSafeFuncIndex calculations.

use std::ops::{Index, IndexMut};

use crate::codegen::Checkpoint;

pub struct CompiledBlock {
    // Func index for the runtime's Wasm table.
    pub func_idx: i32,
    pub checkpoints: Vec<Checkpoint>,
}

pub enum BlockSlot {
    Uncompiled,
    Compiled(CompiledBlock),
    Uncompilable,
}

impl BlockSlot {
    pub fn unwrap_compiled_block(&self) -> &CompiledBlock {
        if let Self::Compiled(compiled_block) = self {
            compiled_block
        } else {
            panic!("BlockSlot wasn't Compiled!")
        }
    }
}

pub struct BlockCache(Vec<BlockSlot>);

impl Default for BlockCache {
    fn default() -> Self {
        let mut vec = Vec::new();
        // Highest ROM bank value (0x7F) * address space (0xFFFF), not optimal size-wise, but it works for now.
        // TODO: Pick the smallest needed dynamically based on the number of ROM banks on ROM load.
        vec.resize_with(0x7EFF81, || BlockSlot::Uncompiled);

        Self(vec)
    }
}

impl Index<usize> for BlockCache {
    type Output = BlockSlot;

    fn index(&self, index: usize) -> &Self::Output {
        &self.0[index]
    }
}

impl IndexMut<usize> for BlockCache {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.0[index]
    }
}
