use crate::parameters::{R16, R16Stack};
use bitfield_struct::bitfield;

#[bitfield(u8, order = Lsb)]
pub struct Flags {
    pub z: bool, // Zero
    pub n: bool, // Subtraction
    pub h: bool, // Half carry
    pub c: bool, // Carry
    #[bits(4)]
    __: u8, // Padding
}

#[bitfield(u16, order = Msb)]
pub struct AccumAndFlags {
    pub a: u8,
    #[bits(8)]
    pub f: Flags,
}

#[bitfield(u16, order = Msb)]
pub struct Bc {
    pub b: u8,
    pub c: u8,
}
#[bitfield(u16, order = Msb)]
pub struct De {
    pub d: u8,
    pub e: u8,
}
#[bitfield(u16, order = Msb)]
pub struct Hl {
    pub h: u8,
    pub l: u8,
}

#[derive(Default)]
pub struct Registers {
    pub af: AccumAndFlags,
    pub bc: Bc,
    pub de: De,
    pub hl: Hl,
    pub sp: u16,
    pub pc: u16,
}

impl Registers {
    pub(crate) fn r16_mut(&mut self, r16: R16) -> &mut u16 {
        match r16 {
            R16::Bc => &mut self.bc.0,
            R16::De => &mut self.de.0,
            R16::Hl => &mut self.hl.0,
            R16::Sp => &mut self.sp,
        }
    }

    pub(crate) fn r16_stack_mut(&mut self, r16_stack: R16Stack) -> &mut u16 {
        match r16_stack {
            R16Stack::Bc => &mut self.bc.0,
            R16Stack::De => &mut self.de.0,
            R16Stack::Hl => &mut self.hl.0,
            R16Stack::Af => &mut self.af.0,
        }
    }
}
