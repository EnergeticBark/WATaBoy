use sm83_interp::cpu::opcodes::Opcode;
use sm83_interp::cpu::opcodes::parameters::R8;
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

// Stores the raw Wasm bytecode dynamically recompiled from a
// block of SM83 instructions and the metadata needed to execute
// it, e.g. how many M-cycles it takes to execute.
pub struct WasmBlock {
    // Wasm bytecode.
    pub buffer: Vec<u8>,
    pub pc_delta: u16,
    // The number of M-Cycles since the system clock has been updated.
    pub delta_m_cycles: u16,
    // The total number of M-Cycles this block of instructions takes to execute.
    pub total_m_cycles: u16,
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
    let mut delta_m_cycles = 0;
    let mut total_m_cycles = 0;
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

            // Ignore INC (HL) for now...
            Opcode::IncR { x: R8::IndirectHL } => break,
            Opcode::IncR { x } => {
                instruction_sink.inc_r(x);
                pc_delta += 1;
            }

            // Block 1
            // Ignore LD (HL), y and LD x, (HL) for now...
            Opcode::LdRR {
                y: R8::IndirectHL, ..
            } => break,
            
            Opcode::LdRR {
                x: R8::IndirectHL, y
            } => {
                // Account for the m_cycle already spent fetching this instruction.
                delta_m_cycles += 1;
                instruction_sink.ld_hl_r(y, delta_m_cycles);
                // Reset delta_m_cycles, because the system clock just caught up.
                delta_m_cycles = 0;
                pc_delta += 1;
            }
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
            // Ignore SUB (HL) for now...
            Opcode::SubR { x: R8::IndirectHL } => break,
            Opcode::SubR { x } => {
                instruction_sink.sub_r(x);
                pc_delta += 1;
            }
            // Ignore SBC (HL) for now...
            Opcode::SbcR { x: R8::IndirectHL } => break,
            Opcode::SbcR { x } => {
                instruction_sink.sbc_r(x);
                pc_delta += 1;
            }
            // Ignore AND (HL) for now...
            Opcode::AndR { x: R8::IndirectHL } => break,
            Opcode::AndR { x } => {
                instruction_sink.and_r(x);
                pc_delta += 1;
            }
            // Ignore XOR (HL) for now...
            Opcode::XorR { x: R8::IndirectHL } => break,
            Opcode::XorR { x } => {
                instruction_sink.xor_r(x);
                pc_delta += 1;
            }
            // Ignore OR (HL) for now...
            Opcode::OrR { x: R8::IndirectHL } => break,
            Opcode::OrR { x } => {
                instruction_sink.or_r(x);
                pc_delta += 1;
            }
            Opcode::CpR { x: R8::IndirectHL } => break,
            Opcode::CpR { x } => {
                instruction_sink.cp_r(x);
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
            _ => break,
        }

        // Add the number of cycles this instruction took to delta_m_cycles and total_m_cycles.
        // TODO: Remember to handle any context dependent instructions separately!!
        delta_m_cycles += opcodes::cycles::m_cycles(opcode);
        total_m_cycles += opcodes::cycles::m_cycles(opcode);

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
        delta_m_cycles,
        total_m_cycles,
    })
}
