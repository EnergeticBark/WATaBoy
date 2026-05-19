// TODO: If block_cache becomes more complicated than just a HashMap move it to this file.
// Possibly alongside lowestSafeFuncIndex calculations.

use bitfield_struct::bitfield;
use std::ops::{Index, IndexMut};

#[cfg(feature = "log-traces")]
use crate::console_log;

use crate::codegen::{Checkpoint, WasmBlock};
use crate::link_new_module;

pub(crate) struct CompiledBlock {
    // Func index for the runtime's Wasm table.
    pub func_idx: i32,
    pub checkpoints: Vec<Checkpoint>,
}

impl CompiledBlock {
    pub(crate) fn new(wasm_block: WasmBlock) -> Self {
        #[cfg(feature = "log-traces")]
        console_log(&wasmprinter::print_bytes(&wasm_block.buffer).unwrap());

        let func_idx = link_new_module(&wasm_block.buffer);
        CompiledBlock {
            func_idx,
            checkpoints: wasm_block.ctx.checkpoints,
        }
    }
}

pub(crate) enum BlockSlot {
    Uncompiled,
    Compiled(CompiledBlock),
    Uncompilable,
}

impl BlockSlot {
    pub(crate) fn unwrap_compiled_block(&self) -> &CompiledBlock {
        if let Self::Compiled(compiled_block) = self {
            compiled_block
        } else {
            panic!("BlockSlot wasn't Compiled!")
        }
    }
}

#[bitfield(u32, order = Msb)]
pub(crate) struct CacheAddress {
    #[bits(8)]
    __: u8, // Padding
    pub(crate) bank_number: u8,
    pub(crate) address: u16,
}

pub(crate) struct BlockCache(Vec<BlockSlot>);

impl Default for BlockCache {
    fn default() -> Self {
        let mut vec = Vec::new();
        // Highest ROM bank value (0x7F) * address space (0xFFFF), not optimal size-wise, but it works for now.
        // TODO: Pick the smallest needed dynamically based on the number of ROM banks on ROM load.
        vec.resize_with(0x7E_FF81, || BlockSlot::Uncompiled);

        Self(vec)
    }
}

impl Index<CacheAddress> for BlockCache {
    type Output = BlockSlot;

    fn index(&self, index: CacheAddress) -> &Self::Output {
        &self.0[index.into_bits() as usize]
    }
}

impl IndexMut<CacheAddress> for BlockCache {
    fn index_mut(&mut self, index: CacheAddress) -> &mut Self::Output {
        &mut self.0[index.into_bits() as usize]
    }
}
