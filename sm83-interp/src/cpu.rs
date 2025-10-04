use arbitrary_int::{u2, u3};
use bitfield_struct::bitfield;

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
    accumulator: u8,
    #[bits(8)]
    flags: Flags,
}

struct Cpu {
    af: AccumAndFlags,
    bc: u16,
    de: u16,
    hl: u16,
    sp: u16,
    pc: u16,
}

impl Cpu {
    fn get_8bit_register(&self, idx: u3) {
        todo!();
    }

    fn get_16bit_register(&self, idx: u2) {
        todo!();
    }
}
