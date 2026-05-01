use interpreter::cpu::opcodes::{Opcode, PrefixOpcode};
use interpreter::cpu::{Cpu, opcodes};

mod instructions;
mod macros;
mod module;
mod registers;

use instructions::{Block0, Block1, Block2, Block3, Prefix};
use macros::Sm83Macros;
use module::{empty_jit_block_function, empty_jit_block_module};

use std::cell::LazyCell;

use wasm_encoder::*;

#[cfg(feature = "jit-trace")]
use crate::console_log;

const MIN_BLOCK_SIZE: usize = 1;
const MAX_BLOCK_SIZE: usize = 500;

#[derive(Copy, Clone)]
pub struct Checkpoint {
    pub exit_pc: u16,
    // The number of M-Cycles since the system clock has been updated.
    pub remaining_m_cycles: u16,
    pub total_m_cycles: u16,
}

#[derive(Default)]
pub struct CodegenCtx {
    pub block_size: usize,
    pub runtime_ptr: usize,
    pub checkpoints: Vec<Checkpoint>,
    pub traced_pc: u16,
    // The number of M-Cycles since the system clock has been updated.
    pub delta_m_cycles: u16,
    // The total number of M-Cycles this block of instructions takes to execute.
    pub total_m_cycles: u16,
}

impl CodegenCtx {
    // Add a checkpoint at the current point in the trace and return its index.
    fn add_checkpoint(&mut self) -> usize {
        self.checkpoints.push(Checkpoint {
            exit_pc: self.traced_pc,
            remaining_m_cycles: self.delta_m_cycles,
            total_m_cycles: self.total_m_cycles,
        });
        self.checkpoints.len() - 1
    }

    fn increment_pc(&mut self) {
        self.traced_pc += 1;
    }

    fn increment_m_cycles(&mut self, m_cycles: u16) {
        self.delta_m_cycles += m_cycles;
        self.total_m_cycles += m_cycles;
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
pub fn recompile(
    dmg_state: &mut Cpu,
    runtime_ptr: usize,
    registers_ptr: usize,
) -> Option<WasmBlock> {
    let mut ctx = CodegenCtx {
        runtime_ptr,
        traced_pc: dmg_state.registers.pc,
        ..Default::default()
    };

    // Don't cache below 0x100 if the boot ROM is mounted!
    if dmg_state.memory.boot_rom_mounted() && ctx.traced_pc < 0x100 {
        return None;
    }

    #[cfg(feature = "jit-trace")]
    let mut sm83_disassembly = String::new();

    // Create these lazily so we don't alloc if pc_delta ends up being 0.
    let mut function = LazyCell::new(empty_jit_block_function);
    let mut instruction_sink = LazyCell::new(|| {
        let mut instruction_sink = function.instructions();

        instruction_sink.read_regs(registers_ptr);

        // Wrap our JIT'd instructions in a block so we can break early if needed.
        instruction_sink.block(BlockType::Empty);
        instruction_sink
    });

    loop {
        let bytecode = dmg_state.memory.read_byte(ctx.traced_pc);
        let opcode = Opcode::decode(bytecode).unwrap();

        // Always increment 1 M-cycle and PC for fetching the first byte.
        ctx.increment_m_cycles(1);
        ctx.increment_pc();

        match opcode {
            // Block 0
            // Need to use fully-qualified syntax to call *our* nop function.
            Opcode::Nop => _ = <InstructionSink as Block0>::nop(&mut instruction_sink),
            Opcode::LdRrNn { x } => {
                let first_byte = dmg_state.memory.read_byte(ctx.traced_pc);
                ctx.increment_pc();
                let second_byte = dmg_state.memory.read_byte(ctx.traced_pc);

                let address = u16::from_le_bytes([first_byte, second_byte]);
                ctx.increment_pc();
                instruction_sink.ld_rr_nn(x, address);
            }
            Opcode::LdMemA { x } => _ = instruction_sink.ld_mem_a(&mut ctx, x),
            Opcode::LdAMem { x } => _ = instruction_sink.ld_a_mem(&mut ctx, x),
            Opcode::IncRr { x } => _ = instruction_sink.inc_rr(x),
            Opcode::DecRr { x } => _ = instruction_sink.dec_rr(x),
            Opcode::AddHlRr { x } => _ = instruction_sink.add_hl_rr(x),
            Opcode::IncR { x } => _ = instruction_sink.inc_r(&mut ctx, x),
            Opcode::DecR { x } => _ = instruction_sink.dec_r(&mut ctx, x),
            Opcode::LdRN { x } => {
                let imm = dmg_state.memory.read_byte(ctx.traced_pc);
                ctx.increment_pc();
                instruction_sink.ld_r_n(&mut ctx, x, imm as i32);
            }
            Opcode::Rlca => _ = instruction_sink.rlca(),
            Opcode::Rrca => _ = instruction_sink.rrca(),
            Opcode::Rra => _ = instruction_sink.rra(),
            Opcode::Cpl => _ = instruction_sink.cpl(),
            Opcode::Scf => _ = instruction_sink.scf(),
            Opcode::Ccf => _ = instruction_sink.ccf(),
            Opcode::JrE => {
                let prev_pc = ctx.traced_pc - 1;

                let e = dmg_state.memory.read_byte(ctx.traced_pc).cast_signed();
                ctx.increment_pc();

                let address = ctx.traced_pc.wrapping_add_signed(i16::from(e));

                let outside_rom = address >= 0x8000;
                let from_bank0_to_switchable = prev_pc < 0x4000 && address >= 0x4000;
                if outside_rom || from_bank0_to_switchable {
                    // Couldn't follow the jump, fall back to the interpreter.
                    ctx.traced_pc -= 1;
                    break;
                }

                ctx.traced_pc = address;
            }

            // Block 1
            Opcode::LdRR { x, y } => _ = instruction_sink.ld_r_r(&mut ctx, x, y),

            // Block 2
            Opcode::AddR { x } => _ = instruction_sink.add_r(&mut ctx, x),
            Opcode::AdcR { x } => _ = instruction_sink.adc_r(&mut ctx, x),
            Opcode::SubR { x } => _ = instruction_sink.sub_r(&mut ctx, x),
            Opcode::SbcR { x } => _ = instruction_sink.sbc_r(&mut ctx, x),
            Opcode::AndR { x } => _ = instruction_sink.and_r(&mut ctx, x),
            Opcode::XorR { x } => _ = instruction_sink.xor_r(&mut ctx, x),
            Opcode::OrR { x } => _ = instruction_sink.or_r(&mut ctx, x),
            Opcode::CpR { x } => _ = instruction_sink.cp_r(&mut ctx, x),

            // Block 3
            Opcode::AddN => {
                let imm = dmg_state.memory.read_byte(ctx.traced_pc);
                ctx.increment_pc();
                instruction_sink.add_n(imm as i32);
            }
            Opcode::AdcN => {
                let imm = dmg_state.memory.read_byte(ctx.traced_pc);
                ctx.increment_pc();
                instruction_sink.adc_n(imm as i32);
            }
            Opcode::SubN => {
                let imm = dmg_state.memory.read_byte(ctx.traced_pc);
                ctx.increment_pc();
                instruction_sink.sub_n(imm as i32);
            }
            Opcode::SbcN => {
                let imm = dmg_state.memory.read_byte(ctx.traced_pc);
                ctx.increment_pc();
                instruction_sink.sbc_n(imm as i32);
            }
            Opcode::AndN => {
                let imm = dmg_state.memory.read_byte(ctx.traced_pc);
                ctx.increment_pc();
                instruction_sink.and_n(imm as i32);
            }
            Opcode::XorN => {
                let imm = dmg_state.memory.read_byte(ctx.traced_pc);
                ctx.increment_pc();
                instruction_sink.xor_n(imm as i32);
            }
            Opcode::OrN => {
                let imm = dmg_state.memory.read_byte(ctx.traced_pc);
                ctx.increment_pc();
                instruction_sink.or_n(imm as i32);
            }
            Opcode::CpN => {
                let imm = dmg_state.memory.read_byte(ctx.traced_pc);
                ctx.increment_pc();
                instruction_sink.cp_n(imm as i32);
            }
            Opcode::LdhCA => _ = instruction_sink.ldh_c_a(&mut ctx),
            Opcode::LdhNA => {
                let imm = dmg_state.memory.read_byte(ctx.traced_pc);
                ctx.increment_pc();
                instruction_sink.ldh_n_a(&mut ctx, imm);
            }
            Opcode::LdNnA => {
                let first_byte = dmg_state.memory.read_byte(ctx.traced_pc);
                ctx.increment_pc();
                let second_byte = dmg_state.memory.read_byte(ctx.traced_pc);

                let address = u16::from_le_bytes([first_byte, second_byte]);

                ctx.increment_pc();
                instruction_sink.ld_nn_a(&mut ctx, address);
            }
            Opcode::LdhAN => {
                let imm = dmg_state.memory.read_byte(ctx.traced_pc);
                ctx.increment_pc();
                instruction_sink.ldh_a_n(&mut ctx, imm);
            }
            Opcode::LdANn => {
                let first_byte = dmg_state.memory.read_byte(ctx.traced_pc);
                ctx.increment_pc();
                let second_byte = dmg_state.memory.read_byte(ctx.traced_pc);

                let address = u16::from_le_bytes([first_byte, second_byte]);
                ctx.increment_pc();
                instruction_sink.ld_a_nn(&mut ctx, address);
            }
            Opcode::JpNn => {
                let prev_pc = ctx.traced_pc - 1;

                let first_byte = dmg_state.memory.read_byte(ctx.traced_pc);
                ctx.increment_pc();
                let second_byte = dmg_state.memory.read_byte(ctx.traced_pc);

                let address = u16::from_le_bytes([first_byte, second_byte]);

                let outside_rom = address >= 0x8000;
                let from_bank0_to_switchable = prev_pc < 0x4000 && address >= 0x4000;
                if outside_rom || from_bank0_to_switchable {
                    // Couldn't follow the jump, fall back to the interpreter.
                    ctx.traced_pc -= 1;
                    break;
                }

                ctx.traced_pc = address;
            }
            Opcode::CallNn => {
                let prev_pc = ctx.traced_pc - 1;

                let first_byte = dmg_state.memory.read_byte(ctx.traced_pc);
                ctx.increment_pc();
                let second_byte = dmg_state.memory.read_byte(ctx.traced_pc);
                ctx.increment_pc();

                let address = u16::from_le_bytes([first_byte, second_byte]);

                let outside_rom = address >= 0x8000;
                let from_bank0_to_switchable = prev_pc < 0x4000 && address >= 0x4000;
                if outside_rom || from_bank0_to_switchable {
                    // Couldn't follow the call, fall back to the interpreter.
                    ctx.traced_pc -= 2;
                    break;
                }

                instruction_sink.call_nn(&mut ctx);

                ctx.traced_pc = address;
            }
            Opcode::PopRr { x } => _ = instruction_sink.pop_rr(&mut ctx, x),
            Opcode::PushRr { x } => _ = instruction_sink.push_rr(&mut ctx, x),
            Opcode::Prefix => recompile_prefix(
                dmg_state,
                &mut ctx,
                &mut instruction_sink,
                #[cfg(feature = "jit-trace")]
                &mut sm83_disassembly,
            ),
            Opcode::LdHlSpPlusE => {
                let imm = dmg_state.memory.read_byte(ctx.traced_pc);
                let e = imm.cast_signed();

                ctx.increment_pc();
                instruction_sink.ld_hl_sp_plus_e(e);
            }
            Opcode::LdSpHl => _ = instruction_sink.ld_sp_hl(),
            _ => break,
        }

        // Add the number of cycles this instruction took to delta_m_cycles and total_m_cycles.
        // TODO: Remember to handle any context dependent instructions separately!!
        // TODO: PopRr and PushRr tick manually here, but not in the interpreter.
        // Remove this if statement once the interpreter ticks them manually.
        if !matches!(
            opcode,
            Opcode::PopRr { .. }
                | Opcode::PushRr { .. }
                | Opcode::LdMemA { .. }
                | Opcode::LdAMem { .. }
                | Opcode::CallNn
        ) {
            ctx.increment_m_cycles(opcodes::cycles::m_cycles(opcode).saturating_sub(1));
        }

        #[cfg(feature = "jit-trace")]
        sm83_disassembly.push_str(&format!("{:?}\n", opcode));

        ctx.block_size += 1;
        if ctx.block_size >= MAX_BLOCK_SIZE {
            break;
        }
    }

    // The final instruction couldn't be added to the block, so retroactively decrement the M-cycle and PC used to fetch it.
    ctx.delta_m_cycles -= 1;
    ctx.total_m_cycles -= 1;
    ctx.traced_pc -= 1;

    if ctx.block_size < MIN_BLOCK_SIZE {
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
        .return_regs(registers_ptr)
        .end();
    codes.function(&function);
    module.section(&codes);

    ctx.add_checkpoint();

    Some(WasmBlock {
        buffer: module.finish(),
        ctx,
    })
}

// Try to recompile the prefix opcode at PC. Returns true if the opcode was recompiled successfully.
pub fn recompile_prefix(
    dmg_state: &mut Cpu,
    ctx: &mut CodegenCtx,
    instruction_sink: &mut InstructionSink,
    #[cfg(feature = "jit-trace")] sm83_disassembly: &mut String,
) {
    let bytecode = dmg_state.memory.read_byte(ctx.traced_pc);
    let prefix_opcode = PrefixOpcode::decode(bytecode);

    // Always increment 1 M-cycle and PC for fetching the prefixed opcode.
    ctx.increment_m_cycles(1);
    ctx.increment_pc();

    match prefix_opcode {
        PrefixOpcode::RlcR { x } => instruction_sink.rlc_r(ctx, x),
        PrefixOpcode::RrcR { x } => instruction_sink.rrc_r(ctx, x),
        PrefixOpcode::RlR { x } => instruction_sink.rl_r(ctx, x),
        PrefixOpcode::RrR { x } => instruction_sink.rr_r(ctx, x),
        PrefixOpcode::SlaR { x } => instruction_sink.sla_r(ctx, x),
        PrefixOpcode::SraR { x } => instruction_sink.sra_r(ctx, x),
        PrefixOpcode::SwapR { x } => instruction_sink.swap_r(ctx, x),
        PrefixOpcode::SrlR { x } => instruction_sink.srl_r(ctx, x),
        PrefixOpcode::BitBR { b, x } => instruction_sink.bit_b_r(ctx, b.into(), x),
        PrefixOpcode::ResBR { b, x } => instruction_sink.res_b_r(ctx, b.into(), x),
        PrefixOpcode::SetBR { b, x } => instruction_sink.set_b_r(ctx, b.into(), x),
    };

    #[cfg(feature = "jit-trace")]
    sm83_disassembly.push_str(&format!("{:?}\n", prefix_opcode));
}
