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
    pub traced_pc: u16,
    // The number of M-Cycles since the system clock has been updated.
    pub delta_m_cycles: u16,
    // The total number of M-Cycles this block of instructions takes to execute.
    pub total_m_cycles: u16,
}

impl CodegenCtx {
    fn increment_pc(&mut self) {
        self.traced_pc += 1;
    } 
}

// Stores the raw Wasm bytecode dynamically recompiled from a
// block of SM83 instructions and the metadata needed to execute
// it, e.g. how many M-cycles it takes to execute.
pub struct WasmBlock {
    // Wasm bytecode.
    pub buffer: Vec<u8>,
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
    let mut instruction_sink = LazyCell::new(|| {
        let mut instruction_sink = function.instructions();
        // Wrap our JIT'd instructions in a block so we can break early if needed.
        instruction_sink.block(BlockType::Empty);
        instruction_sink
    });

    let mut ctx = CodegenCtx {
        traced_pc: pc,
        ..Default::default()
    };
    loop {
        let bytecode = dmg_state.memory.read_byte(ctx.traced_pc);
        let opcode = Opcode::decode(bytecode).unwrap();

        match opcode {
            // Block 0
            Opcode::Nop => {
                // Need to use fully-qualified syntax to call *our* nop function.
                ctx.increment_pc();
                <InstructionSink as Block0>::nop(&mut instruction_sink);
            }
            Opcode::DecRr { x } => {
                ctx.increment_pc();
                instruction_sink.dec_rr(x);
            }
            Opcode::IncRr { x } => {
                ctx.increment_pc();
                instruction_sink.inc_rr(x);
            }
            Opcode::IncR { x } => {
                ctx.increment_pc();
                instruction_sink.inc_r(&mut ctx, x);
            }
            Opcode::DecR { x } => {
                ctx.increment_pc();
                instruction_sink.dec_r(&mut ctx, x);
            }
            Opcode::LdRN { x } => {
                ctx.increment_pc();
                let imm = dmg_state.memory.read_byte(ctx.traced_pc);
                ctx.increment_pc();
                instruction_sink.ld_r_n(&mut ctx, x, imm as i32);
            }

            // Block 1
            Opcode::LdRR { x, y } => {
                ctx.increment_pc();
                instruction_sink.ld_r_r(&mut ctx, x, y);
            }

            // Block 2
            Opcode::AddR { x } => {
                ctx.increment_pc();
                instruction_sink.add_r(&mut ctx, x);
            }
            Opcode::AdcR { x } => {
                ctx.increment_pc();
                instruction_sink.adc_r(&mut ctx, x);
            }
            Opcode::SubR { x } => {
                ctx.increment_pc();
                instruction_sink.sub_r(&mut ctx, x);
            }
            Opcode::SbcR { x } => {
                ctx.increment_pc();
                instruction_sink.sbc_r(&mut ctx, x);
            }
            Opcode::AndR { x } => {
                ctx.increment_pc();
                instruction_sink.and_r(&mut ctx, x);
            }
            Opcode::XorR { x } => {
                ctx.increment_pc();
                instruction_sink.xor_r(&mut ctx, x);
            }
            Opcode::OrR { x } => {
                ctx.increment_pc();
                instruction_sink.or_r(&mut ctx, x);
            }
            Opcode::CpR { x } => {
                ctx.increment_pc();
                instruction_sink.cp_r(&mut ctx, x);
            }

            // Block 3
            Opcode::AddN => {
                ctx.increment_pc();
                let imm = dmg_state.memory.read_byte(ctx.traced_pc);
                ctx.increment_pc();
                instruction_sink.add_n(imm as i32);
            }
            Opcode::AndN => {
                ctx.increment_pc();
                let imm = dmg_state.memory.read_byte(ctx.traced_pc);
                ctx.increment_pc();
                instruction_sink.and_n(imm as i32);
            }
            Opcode::CpN => {
                ctx.increment_pc();
                let imm = dmg_state.memory.read_byte(ctx.traced_pc);
                ctx.increment_pc();
                instruction_sink.cp_n(imm as i32);
            }
            Opcode::LdhNA => {
                ctx.increment_pc();
                let imm = dmg_state.memory.read_byte(ctx.traced_pc);
                ctx.increment_pc();
                instruction_sink.ldh_n_a(&mut ctx, imm);
            }
            Opcode::LdNnA => {
                ctx.increment_pc();
                let first_byte = dmg_state.memory.read_byte(ctx.traced_pc);
                ctx.increment_pc();
                let second_byte = dmg_state.memory.read_byte(ctx.traced_pc);
                
                let address = u16::from_le_bytes([first_byte, second_byte]);
                
                ctx.increment_pc();
                instruction_sink.ld_nn_a(&mut ctx, address);
            }
            Opcode::LdhAN => {
                ctx.increment_pc();
                let imm = dmg_state.memory.read_byte(ctx.traced_pc);
                ctx.increment_pc();
                instruction_sink.ldh_a_n(&mut ctx, imm);
            }
            Opcode::LdANn => {
                ctx.increment_pc();
                let first_byte = dmg_state.memory.read_byte(ctx.traced_pc);
                ctx.increment_pc();
                let second_byte = dmg_state.memory.read_byte(ctx.traced_pc);
                
                let address = u16::from_le_bytes([first_byte, second_byte]);
                ctx.increment_pc();
                instruction_sink.ld_a_nn(&mut ctx, address);
            }
            Opcode::PopRr { x } => {
                ctx.increment_pc();
                instruction_sink.pop_rr(&mut ctx, x);
            }
            Opcode::PushRr { x } => {
                ctx.increment_pc();
                instruction_sink.push_rr(&mut ctx, x);
            }
            _ => break,
        }

        // Add the number of cycles this instruction took to delta_m_cycles and total_m_cycles.
        // TODO: Remember to handle any context dependent instructions separately!!
        // TODO: PopRr and PushRr tick manually here, but not in the interpreter.
        // Remove this if statement once the interpreter ticks them manually.
        if !matches!(opcode, Opcode::PopRr { .. } | Opcode::PushRr { .. }) {
            ctx.delta_m_cycles += opcodes::cycles::m_cycles(opcode);
            ctx.total_m_cycles += opcodes::cycles::m_cycles(opcode);
        }

        #[cfg(feature = "jit-trace")]
        sm83_disassembly.push_str(&format!("{:?}\n", opcode))
    }

    if ctx.traced_pc == pc {
        return None;
    }

    #[cfg(feature = "jit-trace")]
    console_log(&sm83_disassembly);

    let mut module = empty_jit_block_module();

    // Encode the code section.
    let mut codes = CodeSection::new();

    instruction_sink
    // End the inner block.
    .end()
    // Add the calling convention's epilogue.
    .return_regs()
    .end();
    codes.function(&function);
    module.section(&codes);

    Some(WasmBlock {
        buffer: module.finish(),
        ctx,
    })
}
