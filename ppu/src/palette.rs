use bitfield_struct::bitenum;
use hw_constants::io_regs;

#[derive(Copy, Clone)]
pub enum Palette {
    BGP,
    OBP0,
    OBP1,
}

#[bitenum]
#[derive(Copy, Clone)]
#[repr(u8)]
pub enum Color {
    #[fallback]
    White = 0,
    LightGrey = 1,
    DarkGrey = 2,
    Black = 3,
}

pub fn map_to_palette(palette: u8, value: u8) -> Color {
    match value {
        0 => Color::from_bits(palette & 0b0000_0011),
        1 => Color::from_bits((palette >> 2) & 0b0000_0011),
        2 => Color::from_bits((palette >> 4) & 0b0000_0011),
        3 => Color::from_bits((palette >> 6) & 0b0000_0011),
        _ => unreachable!()
    }
}

pub fn map_to_bgp(memory: &[u8], value: u8) -> Color {
    let bgp = memory[io_regs::BGP as usize];
    map_to_palette(bgp, value)
}

pub fn map_to_obp0(memory: &[u8], value: u8) -> Color {
    let obp0 = memory[io_regs::OBP0 as usize];
    map_to_palette(obp0, value)
}

pub fn map_to_obp1(memory: &[u8], value: u8) -> Color {
    let obp1 = memory[io_regs::OBP1 as usize];
    map_to_palette(obp1, value)
}