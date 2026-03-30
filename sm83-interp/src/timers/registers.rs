use bitfield_struct::bitenum;
use bitfield_struct::bitfield;
use rkyv::{Archive, Deserialize, Serialize};

#[bitenum]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Archive, Deserialize, Serialize)]
#[repr(u8)]
pub enum ClockSelect {
    #[fallback]
    Ninth = 0,
    Third = 1,
    Fifth = 2,
    Seventh = 3,
}

impl ClockSelect {
    pub(super) fn mask(self) -> u16 {
        1 << match self {
            ClockSelect::Ninth => 9,
            ClockSelect::Third => 3,
            ClockSelect::Fifth => 5,
            ClockSelect::Seventh => 7,
        }
    }

    // See: https://gbdev.io/pandocs/Timer_and_Divider_Registers.html
    pub(super) fn period(self) -> u16 {
        match self {
            ClockSelect::Ninth => 256,
            ClockSelect::Third => 4,
            ClockSelect::Fifth => 16,
            ClockSelect::Seventh => 64,
        }
    }
}

#[bitfield(u8, order = Msb)]
#[derive(Archive, Deserialize, Serialize)]
pub struct TimerControl {
    #[bits(5, default = 0xFF)]
    __: u8, // Unused bits pulled high.
    #[bits(1)]
    pub tima_enabled: bool,
    #[bits(2)]
    pub clock_select: ClockSelect,
}
