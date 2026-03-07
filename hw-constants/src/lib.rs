pub mod io_regs;
mod post_boot;

pub use post_boot::PostBoot;
pub use post_boot::post_boot_hwio;

pub const MEM_MAP_SIZE: usize = 0x10000;

pub const VRAM_START: u16 = 0x8000;
pub const VRAM_END: u16 = 0xA000;
pub const VRAM_SIZE: u16 = VRAM_END - VRAM_START;

pub const OAM_START: u16 = 0xFE00;
pub const OAM_END: u16 = 0xFEA0;
pub const OAM_SIZE: u16 = OAM_END - OAM_START;

pub const IE: u16 = 0xFFFF;

// Screen
pub const SCREEN_WIDTH: u8 = 160;
pub const SCREEN_HEIGHT: u8 = 144;
pub const SCREEN_SIZE: usize = SCREEN_WIDTH as usize * SCREEN_HEIGHT as usize;

pub const TILE_MAP_SIZE: u16 = 256;

pub const TILE_SIZE: u8 = 8;
