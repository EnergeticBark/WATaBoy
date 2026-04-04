// TODO: If block_cache becomes more complicated than just a HashMap move it to this file.
// Possibly alongside lowestSafeFuncIndex calculations.

#[derive(Copy, Clone)]
pub struct CompiledBlock {
    // Func index for the runtime's Wasm table.
    pub func_idx: i32,
    // In theory, I could update the PC value and clock in the generated Wasm...
    // Maybe do that if it's cleaner.
    pub pc_delta: u16,
    pub delta_m_cycles: u16,
}
