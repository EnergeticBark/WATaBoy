use bitfield_struct::bitfield;

// For the JOYP register, 1 = not held and 0 = held.
#[bitfield(u8, order = Msb, default = false)]
pub struct Joyp {
    #[bits(2)]
    __: u8,
    pub select_buttons: bool,
    pub select_dpad: bool,
    pub start_down: bool,
    pub select_up: bool,
    pub b_left: bool,
    pub a_right: bool,
}

impl Default for Joyp {
    fn default() -> Self {
        // Set none of the buttons to be held by default.
        Joyp(0b11111111)
    }
}

#[derive(Copy, Clone, Default)]
pub struct ButtonsHeld {
    pub start: bool,
    pub select: bool,
    pub b: bool,
    pub a: bool,
    pub down: bool,
    pub up: bool,
    pub left: bool,
    pub right: bool,
}

