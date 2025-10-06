use arbitrary_int::{u2, u3};

#[derive(Debug, PartialEq, Eq)]
#[repr(u8)]
pub(crate) enum R8 {
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
            _ => unreachable!()
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
#[repr(u8)]
pub(crate) enum R16Mem {
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
            _ => unreachable!()
        }
    }
}