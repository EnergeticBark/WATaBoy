use bitfield_struct::bitenum;
use bitfield_struct::bitfield;
use rkyv::{Archive, Deserialize, Serialize};

#[bitenum]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Archive, Deserialize, Serialize)]
#[repr(u8)]
pub(super) enum ClockSelect {
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
}

#[bitfield(u8, order = Msb)]
#[derive(Archive, Deserialize, Serialize)]
pub(super) struct TimerControl {
    #[bits(5)]
    __: u8, // Padding
    #[bits(1)]
    pub(super) tima_enabled: bool,
    #[bits(2)]
    pub(super) clock_select: ClockSelect,
}
