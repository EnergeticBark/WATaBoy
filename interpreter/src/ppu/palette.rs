use std::hint::unreachable_unchecked;

use bitfield_struct::bitenum;
use bitfield_struct::bitfield;

use super::bg_fetcher::ColorIndex;

// See: https://gbdev.io/pandocs/Palettes.html#lcd-monochrome-palettes

#[bitenum]
#[derive(Copy, Clone, Debug)]
#[repr(u8)]
pub enum Color {
    #[fallback]
    White = 0,
    LightGrey = 1,
    DarkGrey = 2,
    Black = 3,
}

#[bitfield(u8, order = Msb)]
pub(super) struct Palette {
    #[bits(2)]
    pub id_3: Color,
    #[bits(2)]
    pub id_2: Color,
    #[bits(2)]
    pub id_1: Color,
    #[bits(2)]
    pub id_0: Color,
}

impl Palette {
    #[inline]
    pub(super) fn map_to_color(self, color_index: ColorIndex) -> Color {
        match color_index.into_bits() {
            0 => self.id_0(),
            1 => self.id_1(),
            2 => self.id_2(),
            3 => self.id_3(),
            _ => unsafe { unreachable_unchecked() },
        }
    }
}

#[bitenum]
#[derive(Copy, Clone, Debug)]
#[repr(u8)]
pub enum PaletteSelect {
    #[fallback]
    Bgp = 0,
    Obp0 = 1,
    Obp1 = 2,
}
