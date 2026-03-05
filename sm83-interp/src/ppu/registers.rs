#[derive(Default)]
pub(super) struct IoRegisters {
    // Read-only from the CPU's perspective.
    pub(super) ly: u8,
}
