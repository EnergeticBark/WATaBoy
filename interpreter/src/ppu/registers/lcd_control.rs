use bitfield_struct::bitfield;

#[bitfield(u8, order = Msb)]
pub struct LcdControl {
    pub lcd_and_ppu_enabled: bool,
    pub window_tile_map: bool,
    pub window_enabled: bool,
    pub bg_and_window_tiles: bool,
    pub bg_tile_map: bool,
    pub obj_size: bool,
    pub obj_enabled: bool,
    pub bg_and_window_enabled: bool,
}
