use bitfield_struct::bitenum;
use bitfield_struct::bitfield;

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
pub struct Palette {
    #[bits(2)]
    pub id_3: Color,
    #[bits(2)]
    pub id_2: Color,
    #[bits(2)]
    pub id_1: Color,
    #[bits(2)]
    pub id_0: Color,
}

#[derive(Copy, Clone)]
pub enum PaletteSelect {
    Bgp,
    Obp0,
    Obp1,
}

pub fn map_to_palette(palette: Palette, value: u8) -> Color {
    match value {
        0 => palette.id_0(),
        1 => palette.id_1(),
        2 => palette.id_2(),
        3 => palette.id_3(),
        _ => unreachable!(),
    }
}
