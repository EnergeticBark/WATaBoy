use arbitrary_int::{u2, u3};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum R8 {
    B = 0,
    C = 1,
    D = 2,
    E = 3,
    H = 4,
    L = 5,
    IndirectHL = 6,
    A = 7,
}
impl R8 {
    pub(crate) const fn from_bits(value: u3) -> Self {
        use R8::*;
        match value.value() {
            0 => B,
            1 => C,
            2 => D,
            3 => E,
            4 => H,
            5 => L,
            6 => IndirectHL,
            7 => A,
            _ => unreachable!(),
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum R16 {
    Bc = 0,
    De = 1,
    Hl = 2,
    Sp = 3,
}
impl R16 {
    pub(crate) const fn from_bits(value: u2) -> Self {
        use R16::*;
        match value.value() {
            0 => Bc,
            1 => De,
            2 => Hl,
            3 => Sp,
            _ => unreachable!(),
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum R16Stack {
    Bc = 0,
    De = 1,
    Hl = 2,
    Af = 3,
}
impl R16Stack {
    pub(crate) const fn from_bits(value: u2) -> Self {
        use R16Stack::*;
        match value.value() {
            0 => Bc,
            1 => De,
            2 => Hl,
            3 => Af,
            _ => unreachable!(),
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum R16Mem {
    Bc = 0,
    De = 1,
    HlInc = 2,
    HlDec = 3,
}
impl R16Mem {
    pub(crate) const fn from_bits(value: u2) -> Self {
        use R16Mem::*;
        match value.value() {
            0 => Bc,
            1 => De,
            2 => HlInc,
            3 => HlDec,
            _ => unreachable!(),
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Condition {
    Nz = 0,
    Z = 1,
    Nc = 2,
    C = 3,
}
impl Condition {
    pub(crate) const fn from_bits(value: u2) -> Self {
        use Condition::*;
        match value.value() {
            0 => Nz,
            1 => Z,
            2 => Nc,
            3 => C,
            _ => unreachable!(),
        }
    }
}
