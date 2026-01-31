pub mod io_regs;
mod post_boot;

pub use post_boot::PostBoot;
pub use post_boot::post_boot_hwio;

pub const OAM: u16 = 0xFE00;

pub const IE: u16 = 0xFFFF;

pub const MEM_MAP_SIZE: usize = 0x10000;

pub const SCREEN_WIDTH: usize = 160;
pub const SCREEN_HEIGHT: usize = 144;

pub const TILE_MAP_SIZE: usize = 256;

pub const TILE_SIZE: usize = 8;
