use bitfield_struct::bitenum;

#[bitenum]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum R8 {
    #[fallback]
    B = 0,
    C = 1,
    D = 2,
    E = 3,
    H = 4,
    L = 5,
    IndirectHL = 6,
    A = 7,
}

#[bitenum]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum R16 {
    #[fallback]
    Bc = 0,
    De = 1,
    Hl = 2,
    Sp = 3,
}

#[bitenum]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum R16Stack {
    #[fallback]
    Bc = 0,
    De = 1,
    Hl = 2,
    Af = 3,
}

#[bitenum]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum R16Mem {
    #[fallback]
    Bc = 0,
    De = 1,
    HlInc = 2,
    HlDec = 3,
}

#[bitenum]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Condition {
    #[fallback]
    Nz = 0,
    Z = 1,
    Nc = 2,
    C = 3,
}
