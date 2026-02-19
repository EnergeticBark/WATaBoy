use sm83_interp::cpu::Cpu;
use sm83_interp::opcodes;
use sm83_interp::parameters::R8;

mod macros;
mod module;
mod registers;

use macros::{FlagBit, Sm83Macros};
use module::{empty_jit_block_function, empty_jit_block_module};
use registers::A;

use wasm_encoder::*;

#[cfg(feature = "jit-trace")]
use crate::console_log;

// Stores the raw Wasm bytecode dynamically recompiled from a
// block of SM83 instructions and the metadata needed to execute
// it, e.g. how many M-cycles it takes to execute.
pub struct WasmBlock {
    // Wasm bytecode.
    pub buffer: Vec<u8>,
    pub pc_delta: u16,
    // TODO: Store M-cycles.
}

// Try to produce a WasmBlock starting at dmg_state's current program counter.
// TODO: Read one opcode at a time until a branching statement is reached. -> Codegen Wasm for each instruction.
pub fn recompile(dmg_state: &mut Cpu) -> Option<WasmBlock> {
    let pc = dmg_state.registers.pc;
    // Only cache from ROM bank 00 for now.
    if pc >= 0x4000 {
        return None;
    }

    let bytecode = dmg_state.memory[pc];
    let opcode = opcodes::decode(bytecode).unwrap();

    // Early return if the first opcode is incompatible so we don't have to create the function.
    // I really don't like that this means we call decode() twice... theres probably a better way to do this.
    // Maybe look into how LazyLock would perform?
    match opcode {
        opcodes::Opcode::AddR { x: R8::IndirectHL } => return None,
        opcodes::Opcode::AddR { .. } => (),
        _ => return None,
    }

    #[cfg(feature = "jit-trace")]
    let mut sm83_disassembly = String::new();

    let mut function = empty_jit_block_function();
    let mut instruction_sink = function.instructions();

    let mut pc_delta = 0;
    loop {
        let bytecode = dmg_state.memory[pc + pc_delta];
        let opcode = opcodes::decode(bytecode).unwrap();

        match opcode {
            // Ignore ADD (HL) for now...
            opcodes::Opcode::AddR { x: R8::IndirectHL } => break,
            opcodes::Opcode::AddR { x } => {
                instruction_sink.add_r(x);
                pc_delta += 1;
            }
            _ => break,
        }

        #[cfg(feature = "jit-trace")]
        sm83_disassembly.push_str(&format!("{:?}\n", opcode))
    }

    #[cfg(feature = "jit-trace")]
    console_log(&sm83_disassembly);

    let mut module = empty_jit_block_module();

    // Encode the code section.
    let mut codes = CodeSection::new();

    instruction_sink.return_regs().end();
    codes.function(&function);
    module.section(&codes);

    Some(WasmBlock {
        buffer: module.finish(),
        pc_delta,
    })
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

trait Sm83Instructions {
    fn add_r(&mut self, r8: R8) -> &mut Self;
}

impl Sm83Instructions for InstructionSink<'_> {
    fn add_r(&mut self, r8: R8) -> &mut Self {
        // Name our scratch registers.
        const PREV_A: u32 = 8;
        const PREV_R8: u32 = 9;
        self.clear_flags() // Maybe add a macro for *assigning* flags too so we don't have to do this separately from setting the first flag.
            // *** Store original values of A and R8 so they can be used to calculate the half-carry. ***
            .local_get(A)
            .local_tee(PREV_A)
            .local_get(r8_to_reg_param(r8))
            .local_tee(PREV_R8)
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
            .local_get(PREV_A)
            .i32_const(0x0f)
            .i32_and() // (A & 0x0f)
            .local_get(PREV_R8)
            .i32_const(0x0f)
            .i32_and() // (R8 & 0x0f)
            .i32_add()
            .i32_const(0x0f)
            .i32_gt_u()
            .set_flag(FlagBit::HalfCarry)
    }
}
