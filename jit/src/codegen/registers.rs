use interpreter::cpu::opcodes::parameters::R8;

use crate::codegen::CodegenCtx;
use crate::codegen::module::NUM_SCRATCH_REGS;

#[derive(Clone, Copy, Hash, PartialEq, Eq)]
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

#[allow(clippy::cast_possible_truncation)]
impl LocalReg {
    pub fn to_index(self, ctx: &mut CodegenCtx) -> u32 {
        if let Some(index) = ctx.regs_used.get(&self) {
            *index
        } else {
            let index = NUM_SCRATCH_REGS + ctx.regs_used.len() as u32;
            ctx.regs_used.insert(self, index);
            index
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
