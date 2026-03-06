use std::hint::unreachable_unchecked;

use bitfield_struct::bitenum;

#[derive(Copy, Clone)]
pub enum Palette {
    Bgp,
    Obp0,
    Obp1,
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

/// # Safety
/// The argument passed to `value` *MUST* be a 2-bit unsigned integer.
pub fn map_to_palette(palette: u8, value: u8) -> Color {
    match value {
        0 => Color::from_bits(palette & 0b0000_0011),
        1 => Color::from_bits((palette >> 2) & 0b0000_0011),
        2 => Color::from_bits((palette >> 4) & 0b0000_0011),
        3 => Color::from_bits((palette >> 6) & 0b0000_0011),
        _ => unsafe { unreachable_unchecked() },
    }
}
