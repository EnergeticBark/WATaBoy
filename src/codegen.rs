use sm83_interp::cpu::Cpu;
use sm83_interp::opcodes;
use sm83_interp::opcodes::Opcode;
use sm83_interp::parameters::R8;

mod instructions;
mod macros;
mod module;
mod registers;

use instructions::{Block0, Block1, Block2, Block3};
use macros::Sm83Macros;
use module::{empty_jit_block_function, empty_jit_block_module};

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

    #[cfg(feature = "jit-trace")]
    let mut sm83_disassembly = String::new();

    // TODO: At some point, either make this a LazyLock or early return before we create `function`.
    let mut function = empty_jit_block_function();
    let mut instruction_sink = function.instructions();

    let mut pc_delta = 0;
    loop {
        let bytecode = dmg_state.memory[pc + pc_delta];
        let opcode = opcodes::decode(bytecode).unwrap();

        match opcode {
            // Block 0
            Opcode::Nop => {
                // Need to use fully-qualified syntax to call *our* nop function.
                <InstructionSink as Block0>::nop(&mut instruction_sink);
                pc_delta += 1;
            }

            // Block 1
            // Ignore LD (HL), y and LD x, (HL) for now...
            Opcode::LdRR {
                x: R8::IndirectHL, ..
            }
            | Opcode::LdRR {
                y: R8::IndirectHL, ..
            } => break,
            Opcode::LdRR { x, y } => {
                instruction_sink.ld_r_r(x, y);
                pc_delta += 1;
            }

            // Block 2
            // Ignore ADD (HL) for now...
            Opcode::AddR { x: R8::IndirectHL } => break,
            Opcode::AddR { x } => {
                instruction_sink.add_r(x);
                pc_delta += 1;
            }
            // Ignore ADC (HL) for now...
            Opcode::AdcR { x: R8::IndirectHL } => break,
            Opcode::AdcR { x } => {
                instruction_sink.adc_r(x);
                pc_delta += 1;
            }
            // Ignore AND (HL) for now...
            Opcode::AndR { x: R8::IndirectHL } => break,
            Opcode::AndR { x } => {
                instruction_sink.and_r(x);
                pc_delta += 1;
            }

            // Block 3
            Opcode::AddN => {
                pc_delta += 1;
                let current_pc = dmg_state.registers.pc + pc_delta;
                let imm = dmg_state.memory[current_pc];
                instruction_sink.add_n(imm as i32);
                pc_delta += 1;
            }
            _ => break,
        }

        #[cfg(feature = "jit-trace")]
        sm83_disassembly.push_str(&format!("{:?}\n", opcode))
    }

    if pc_delta == 0 {
        return None;
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
