use sm83_interp::cpu::opcodes::Opcode;
use sm83_interp::cpu::{Cpu, opcodes};

mod instructions;
mod macros;
mod module;
mod registers;

use instructions::{Block0, Block1, Block2, Block3};
use macros::Sm83Macros;
use module::{empty_jit_block_function, empty_jit_block_module};

use std::cell::LazyCell;

use wasm_encoder::*;

#[cfg(feature = "jit-trace")]
use crate::console_log;

#[derive(Default)]
pub struct CodegenCtx {
    // The number of M-Cycles since the system clock has been updated.
    pub delta_m_cycles: u16,
    // The total number of M-Cycles this block of instructions takes to execute.
    pub total_m_cycles: u16,
}

// Stores the raw Wasm bytecode dynamically recompiled from a
// block of SM83 instructions and the metadata needed to execute
// it, e.g. how many M-cycles it takes to execute.
pub struct WasmBlock {
    // Wasm bytecode.
    pub buffer: Vec<u8>,
    pub pc_delta: u16,
    pub ctx: CodegenCtx,
}

// Try to produce a WasmBlock starting at dmg_state's current program counter.
// TODO: Read one opcode at a time until a branching statement is reached. -> Codegen Wasm for each instruction.
pub fn recompile(dmg_state: &mut Cpu) -> Option<WasmBlock> {
    let pc = dmg_state.registers.pc;

    #[cfg(feature = "caching")]
    // Don't cache below 0x100 if the boot ROM is mounted!
    if dmg_state.memory.boot_rom_mounted() && pc < 0x100 
    // Only cache from ROM bank 00 for now.
    || pc >= 0x4000
    {
        return None;
    }

    #[cfg(feature = "jit-trace")]
    let mut sm83_disassembly = String::new();

    // Create these lazily so we don't alloc if pc_delta ends up being 0.
    let mut function = LazyCell::new(empty_jit_block_function);
    let mut instruction_sink = LazyCell::new(|| function.instructions());

    let mut pc_delta = 0;
    let mut ctx = CodegenCtx::default();
    loop {
        let bytecode = dmg_state.memory.read_byte(pc + pc_delta);
        let opcode = Opcode::decode(bytecode).unwrap();

        match opcode {
            // Block 0
            Opcode::Nop => {
                // Need to use fully-qualified syntax to call *our* nop function.
                <InstructionSink as Block0>::nop(&mut instruction_sink);
                pc_delta += 1;
            }
            Opcode::IncR { x } => {
                instruction_sink.inc_r(&mut ctx, x);
                pc_delta += 1;
            }

            // Block 1
            Opcode::LdRR { x, y } => {
                instruction_sink.ld_r_r(&mut ctx, x, y);
                pc_delta += 1;
            }

            // Block 2
            Opcode::AddR { x } => {
                instruction_sink.add_r(&mut ctx, x);
                pc_delta += 1;
            }
            Opcode::AdcR { x } => {
                instruction_sink.adc_r(&mut ctx, x);
                pc_delta += 1;
            }
            Opcode::SubR { x } => {
                instruction_sink.sub_r(&mut ctx, x);
                pc_delta += 1;
            }
            Opcode::SbcR { x } => {
                instruction_sink.sbc_r(&mut ctx, x);
                pc_delta += 1;
            }
            Opcode::AndR { x } => {
                instruction_sink.and_r(&mut ctx, x);
                pc_delta += 1;
            }
            Opcode::XorR { x } => {
                instruction_sink.xor_r(&mut ctx, x);
                pc_delta += 1;
            }
            Opcode::OrR { x } => {
                instruction_sink.or_r(&mut ctx, x);
                pc_delta += 1;
            }
            Opcode::CpR { x } => {
                instruction_sink.cp_r(&mut ctx, x);
                pc_delta += 1;
            }

            // Block 3
            Opcode::AddN => {
                pc_delta += 1;
                let current_pc = dmg_state.registers.pc + pc_delta;
                let imm = dmg_state.memory.read_byte(current_pc);
                instruction_sink.add_n(imm as i32);
                pc_delta += 1;
            }
            Opcode::CpN => {
                pc_delta += 1;
                let current_pc = dmg_state.registers.pc + pc_delta;
                let imm = dmg_state.memory.read_byte(current_pc);
                instruction_sink.cp_n(imm as i32);
                pc_delta += 1;
            }
            Opcode::LdhAN => {
                pc_delta += 1;
                let current_pc = dmg_state.registers.pc + pc_delta;
                let imm = dmg_state.memory.read_byte(current_pc);
                instruction_sink.ldh_a_n(&mut ctx, imm);
                pc_delta += 1;
            }
            _ => break,
        }

        // Add the number of cycles this instruction took to delta_m_cycles and total_m_cycles.
        // TODO: Remember to handle any context dependent instructions separately!!
        ctx.delta_m_cycles += opcodes::cycles::m_cycles(opcode);
        ctx.total_m_cycles += opcodes::cycles::m_cycles(opcode);

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
        ctx,
    })
}
