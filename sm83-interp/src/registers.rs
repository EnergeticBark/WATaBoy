use bitfield_struct::bitfield;
use crate::parameters::R16;

#[bitfield(u8)]
pub(crate) struct Flags {
    pub(crate) z: bool, // Zero
    pub(crate) n: bool, // Subtraction
    pub(crate) h: bool, // Half carry
    pub(crate) c: bool, // Carry
    #[bits(4)]
    __: u8, // Padding
}

#[bitfield(u16)]
pub(crate) struct AccumAndFlags {
    pub(crate) a: u8,
    #[bits(8)]
    pub(crate) f: Flags,
}

#[bitfield(u16)]
pub(crate) struct Bc {
    pub(crate) b: u8,
    pub(crate) c: u8,
}
#[bitfield(u16)]
pub(crate) struct De {
    pub(crate) d: u8,
    pub(crate) e: u8,
}
#[bitfield(u16)]
pub(crate) struct Hl {
    pub(crate) h: u8,
    pub(crate) l: u8,
}

#[derive(Default)]
pub(crate) struct Registers {
    pub(crate) af: AccumAndFlags,
    pub(crate) bc: Bc,
    pub(crate) de: De,
    pub(crate) hl: Hl,
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
