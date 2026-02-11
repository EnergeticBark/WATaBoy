pub mod io_regs;
mod post_boot;

pub use post_boot::PostBoot;
pub use post_boot::post_boot_hwio;

pub const OAM: u16 = 0xFE00;

pub const IE: u16 = 0xFFFF;

pub const MEM_MAP_SIZE: usize = 0x10000;

// Screen
pub const SCREEN_WIDTH: u8 = 160;
pub const SCREEN_HEIGHT: u8 = 144;
pub const SCREEN_SIZE: usize = SCREEN_WIDTH as usize * SCREEN_HEIGHT as usize;

pub const TILE_MAP_SIZE: u16 = 256;

pub const TILE_SIZE: u8 = 8;
