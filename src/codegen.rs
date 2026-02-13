use sm83_interp::cpu::Cpu;
use sm83_interp::opcodes;
use sm83_interp::parameters::R8;

mod macros;
mod module;
mod registers;

use macros::{FlagBit, Sm83Macros};
use module::empty_jit_block_module;
use registers::A;

use wasm_encoder::*;

pub struct JitBlock {
    // Wasm bytecode.
    pub buffer: Vec<u8>,
}

// TODO: Takes a PC address and CPU state as input and produces a JitBlock.
// TODO: JitBlock includes the raw bytes of Wasm as well as metadata, e.g. how many total cycles it takes to execute.
// TODO: Read one opcode at a time until a branching statement is reached. -> Codegen Wasm for each instruction.
pub fn recompile(dmg_state: &mut Cpu) -> Option<JitBlock> {
    let pc = dmg_state.registers.pc;
    let bytecode = dmg_state.memory[pc];
    let opcode = opcodes::decode(bytecode).unwrap();

    match opcode {
        // Ignore adds that access memory for now...
        opcodes::Opcode::AddR { x: R8::IndirectHL } => None,
        opcodes::Opcode::AddR { x } => Some(JitBlock {
            buffer: generate_add_r(x),
        }),
        _ => None,
    }
}

fn r8_to_reg_param(r8: R8) -> u32 {
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

pub fn generate_add_r(r8: R8) -> Vec<u8> {
    let mut module = empty_jit_block_module();

    // Encode the code section.
    let mut codes = CodeSection::new();
    let locals = vec![(2, ValType::I32)];
    let prev_a = 8;
    let prev_r8 = 9;
    let mut add_r = Function::new(locals);
    add_r
        .instructions()
        .clear_flags() // Maybe add a macro for *assigning* flags too so we don't have to do this separately from setting the first flag.
        // *** Store original values of A and R8 so they can be used to calculate the half-carry. ***
        .local_get(A)
        .local_tee(prev_a)
        .local_get(r8_to_reg_param(r8))
        .local_tee(prev_r8)
        /* Perform the addition (result not yet truncated):
         * A = A + R8
         */
        .i32_add()
        .local_tee(A)
        /* Calculate Overflow Flag:
         * A > 0xff
         */
        .i32_const(0xff)
        .i32_gt_u() // If result > 255 (overflow), then 1, otherwise 0.
        .set_flag(FlagBit::Carry)
        /* Truncate A to 8-bits:
         * A &= 0xff
         */
        .local_get(A)
        .i32_const(0xff)
        .i32_and()
        .local_tee(A)
        // *** Calculate Zero Flag. ***
        .i32_eqz() // If the A is zero, then 1, otherwise 0.
        .set_flag(FlagBit::Zero)
        /* Calculate Half-Carry Flag:
         * ((A & 0x0f) + (R8 & 0x0f)) > 0x0f
         */
        .local_get(prev_a)
        .i32_const(0x0f)
        .i32_and() // (A & 0x0f)
        .local_get(prev_r8)
        .i32_const(0x0f)
        .i32_and() // (R8 & 0x0f)
        .i32_add()
        .i32_const(0x0f)
        .i32_gt_u()
        .set_flag(FlagBit::HalfCarry)
        .return_regs()
        .end();
    codes.function(&add_r);

    module.section(&codes);

    module.finish()
}
