mod lcd_control;
mod lcd_status;

pub use lcd_control::LcdControl;
pub use lcd_status::{LcdStatus, StatMode};

#[derive(Default)]
pub(super) struct IoRegisters {
    // Read-only from the CPU's perspective.
    pub(super) ly: u8,
}
