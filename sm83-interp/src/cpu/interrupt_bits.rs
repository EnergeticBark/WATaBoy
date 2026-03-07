use bitfield_struct::bitfield;

// This is valid for the IE and IF I/O registers.
#[bitfield(u8, order = Msb)]
pub struct InterruptBits {
    #[bits(3)]
    __: u8, // Padding
    pub joypad: bool,
    pub serial: bool,
    pub timer: bool,
    pub lcd: bool,
    pub vblank: bool,
}
