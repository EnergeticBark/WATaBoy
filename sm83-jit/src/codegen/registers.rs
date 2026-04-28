use sm83_interp::cpu::opcodes::parameters::R8;

pub const A: u32 = 0;
pub const F: u32 = 1;
pub const B: u32 = 2;
pub const C: u32 = 3;
pub const D: u32 = 4;
pub const E: u32 = 5;
pub const H: u32 = 6;
pub const L: u32 = 7;
pub const SP: u32 = 8;

pub(crate) fn r8_to_reg_param(r8: R8) -> u32 {
    match r8 {
        R8::B => 2,
        R8::C => 3,
        R8::D => 4,
        R8::E => 5,
        R8::H => 6,
        R8::L => 7,
        R8::IndirectHL => unreachable!(),
        R8::A => 0,
    }
}
