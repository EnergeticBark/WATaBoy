use bitfield_struct::bitenum;
use bitfield_struct::bitfield;

#[bitenum]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum StatMode {
    #[fallback]
    HBlank = 0, // Or LCD off.
    VBlank = 1,
    OamScan = 2,
    Drawing = 3,
}

#[bitfield(u8, order = Msb)]
pub struct LcdStatus {
    #[bits(1)]
    __: bool, // Padding
    pub lyc_int_select: bool,
    pub mode2_int_select: bool,
    pub mode1_int_select: bool,
    pub mode0_int_select: bool,
    pub coincidence: bool,
    #[bits(2)]
    pub mode: StatMode,
}
