use super::opcodes::parameters::{R16, R16Stack};

use bitfield_struct::bitfield;
use hw_constants::PostBoot;
use rkyv::{Archive, Deserialize, Serialize};

#[bitfield(u8, order = Msb)]
#[derive(Archive, Deserialize, Serialize)]
pub struct Flags {
    pub z: bool, // Zero
    pub n: bool, // Subtraction
    pub h: bool, // Half carry
    pub c: bool, // Carry
    #[bits(4)]
    __: u8, // Padding
}

#[bitfield(u16, order = Msb)]
#[derive(Archive, Deserialize, Serialize)]
pub struct AccumAndFlags {
    pub a: u8,
    #[bits(8)]
    pub f: Flags,
}

#[bitfield(u16, order = Msb)]
#[derive(Archive, Deserialize, Serialize)]
pub struct Bc {
    pub b: u8,
    pub c: u8,
}
#[bitfield(u16, order = Msb)]
#[derive(Archive, Deserialize, Serialize)]
pub struct De {
    pub d: u8,
    pub e: u8,
}
#[bitfield(u16, order = Msb)]
#[derive(Archive, Deserialize, Serialize)]
pub struct Hl {
    pub h: u8,
    pub l: u8,
}

#[derive(Default, Archive, Deserialize, Serialize)]
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

    pub(crate) fn r16_stack(&mut self, r16_stack: R16Stack) -> u16 {
        match r16_stack {
            R16Stack::Bc => self.bc.0,
            R16Stack::De => self.de.0,
            R16Stack::Hl => self.hl.0,
            R16Stack::Af => self.af.0,
        }
    }

    pub(crate) fn set_r16_stack(&mut self, r16_stack: R16Stack, value: u16) {
        match r16_stack {
            R16Stack::Bc => self.bc.0 = value,
            R16Stack::De => self.de.0 = value,
            R16Stack::Hl => self.hl.0 = value,
            R16Stack::Af => {
                self.af.0 = value;
                self.af.0 &= 0xFFF0;
            }
        }
    }
}

impl PostBoot for Registers {
    /// Initialize registers to the value they'd be just after executing the MGB boot rom.
    /// See: <https://gbdev.io/pandocs/Power_Up_Sequence.html#cpu-registers>
    fn post_boot_mgb() -> Self {
        Self {
            af: AccumAndFlags::new().with_a(0xFF).with_f(
                Flags::new()
                    .with_z(true)
                    .with_n(false)
                    .with_h(true)
                    .with_c(true),
            ),
            bc: Bc::new().with_b(0x00).with_c(0x13),
            de: De::new().with_d(0x00).with_e(0xD8),
            hl: Hl::new().with_h(0x01).with_l(0x4D),
            pc: 0x0100,
            sp: 0xFFFE,
        }
    }
}
