use bitfield_struct::bitfield;
use crate::parameters::R16;

#[bitfield(u8)]
struct Flags {
    z: bool, // Zero
    n: bool, // Subtraction
    h: bool, // Half carry
    c: bool, // Carry
    #[bits(4)]
    __: u8, // Padding
}

#[bitfield(u16)]
struct AccumAndFlags {
    a: u8,
    #[bits(8)]
    f: Flags,
}

#[bitfield(u16)]
struct Bc { b: u8, c: u8 }
#[bitfield(u16)]
struct De { d: u8, e: u8 }
#[bitfield(u16)]
struct Hl { h: u8, l: u8 }

#[derive(Default)]
pub(crate) struct Registers {
    af: AccumAndFlags,
    bc: Bc,
    de: De,
    hl: Hl,
    pub(crate) sp: u16,
    pub(crate) pc: u16,
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
}
