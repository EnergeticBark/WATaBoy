use interpreter::cpu::opcodes::parameters::R8;

#[derive(Clone, Copy)]
pub enum LocalReg {
    A,
    F,
    B,
    C,
    D,
    E,
    H,
    L,
    SP,
}

impl LocalReg {
    pub fn to_index(self) -> u32 {
        match self {
            LocalReg::A => 0,
            LocalReg::F => 1,
            LocalReg::B => 2,
            LocalReg::C => 3,
            LocalReg::D => 4,
            LocalReg::E => 5,
            LocalReg::H => 6,
            LocalReg::L => 7,
            LocalReg::SP => 8,
        }
    }
}

impl TryFrom<R8> for LocalReg {
    type Error = &'static str;

    fn try_from(value: R8) -> Result<Self, Self::Error> {
        let local = match value {
            R8::B => LocalReg::B,
            R8::C => LocalReg::C,
            R8::D => LocalReg::D,
            R8::E => LocalReg::E,
            R8::H => LocalReg::H,
            R8::L => LocalReg::L,
            R8::IndirectHL => Err("IndirectHL is not a valid LocalReg")?,
            R8::A => LocalReg::A,
        };
        Ok(local)
    }
}
