mod lcd_control;
mod lcd_status;

pub use lcd_control::LcdControl;
pub use lcd_status::{LcdStatus, StatMode};

use hw_constants::PostBoot;

#[derive(Default)]
pub(super) struct IoRegisters {
    pub(super) lcdc: LcdControl,
    // Read-only from the CPU's perspective.
    pub(super) ly: u8,
}

impl PostBoot for IoRegisters {
    fn post_boot_dmg() -> Self {
        IoRegisters {
            lcdc: 0x91.into(),
            ly: 0x00,
        }
    }
}
