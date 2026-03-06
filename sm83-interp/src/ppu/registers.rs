mod lcd_control;
mod lcd_status;

pub use lcd_control::LcdControl;
pub use lcd_status::{LcdStatus, StatMode};

use hw_constants::PostBoot;

#[derive(Clone, Copy, Default)]
pub(super) struct IoRegisters {
    pub(super) lcdc: LcdControl,
    pub(super) stat: LcdStatus,
    pub(super) scy: u8,
    pub(super) scx: u8,
    // Read-only from the CPU's perspective.
    pub(super) ly: u8,
    // R/w
    pub(super) wy: u8,
    pub(super) wx: u8,
}

impl PostBoot for IoRegisters {
    fn post_boot_dmg() -> Self {
        IoRegisters {
            lcdc: 0x91.into(),
            stat: 0x85.into(),
            // Zero out the other regs.
            ..Default::default()
        }
    }
}
