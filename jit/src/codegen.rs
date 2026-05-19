use hw_constants::ROM_BANK_0_END;
use interpreter::cpu::opcodes::{Opcode, PrefixOpcode};
use interpreter::cpu::{Cpu, opcodes};

mod instructions;
mod macros;
mod module;
mod registers;

use instructions::{Block0, Block1, Block2, Block3, Prefix};
use macros::Sm83Macros;
use module::{empty_jit_block_function, empty_jit_block_module};
use registers::LocalReg;

use std::collections::HashMap;
use wasm_encoder::{BlockType, CodeSection, InstructionSink};

#[cfg(feature = "log-traces")]
use crate::console_log;

const MIN_BLOCK_SIZE: usize = 1;
const MAX_BLOCK_SIZE: usize = 500;

#[derive(Copy, Clone)]
pub struct Checkpoint {
    pub exit_pc: u16,
    pub total_m_cycles: u16,
}

#[derive(Default)]
pub struct CodegenCtx {
    pub needs_outer_block: bool,
    pub regs_used: HashMap<LocalReg, u32>,
    pub block_size: usize,
    pub runtime_ptr: usize,
    pub work_ram_ptr: usize,
    pub rom_ptr: usize,
    pub checkpoints: Vec<Checkpoint>,
    pub traced_pc: u16,
    /// The total number of M-Cycles this block of instructions has taken to execute so far.
    pub total_m_cycles: u16,
}

impl CodegenCtx {
    /// Add a checkpoint at the current point in the trace and return its index.
    fn add_checkpoint(&mut self) -> usize {
        self.checkpoints.push(Checkpoint {
            exit_pc: self.traced_pc,
            total_m_cycles: self.total_m_cycles,
        });
        self.checkpoints.len() - 1
    }

    fn increment_pc(&mut self) {
        self.traced_pc += 1;
    }

    fn increment_m_cycles(&mut self, m_cycles: u16) {
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

impl WasmBlock {
    /// Starting at `dmg_state`'s current program counter, read one opcode at a time until a branching instruction is reached.
    /// ## Returns
    /// An Option containing either a `Some(WasmBlock)` or `None` if no instructions could be recompiled.
    #[allow(clippy::too_many_lines)]
    pub(crate) fn recompile(
        dmg_state: &mut Cpu,
        runtime_ptr: usize,
        rom_ptr: usize,
    ) -> Option<WasmBlock> {
        let work_ram_ptr = dmg_state.memory.buffer.as_ptr() as usize;
        let registers_ptr = &raw const dmg_state.registers as usize;

        let mut ctx = CodegenCtx {
            runtime_ptr,
            work_ram_ptr,
            rom_ptr,
            traced_pc: dmg_state.registers.pc,
            ..Default::default()
        };

        #[cfg(feature = "log-traces")]
        let mut sm83_disassembly = String::new();

        let mut instruction_vec = Vec::new();
        let mut instruction_sink = InstructionSink::new(&mut instruction_vec);

        loop {
            let bytecode = dmg_state.memory.read_byte(ctx.traced_pc);
            let opcode = Opcode::decode(bytecode).unwrap();

            // Always increment 1 M-cycle and PC for fetching the first byte.
            ctx.increment_m_cycles(1);
            ctx.increment_pc();

            if ctx.block_size >= MAX_BLOCK_SIZE {
                break;
            }

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
                    instruction_sink.ld_rr_nn(&mut ctx, x, address);
                }
                Opcode::LdMemA { x } => _ = instruction_sink.ld_mem_a(&mut ctx, x),
                Opcode::LdAMem { x } => _ = instruction_sink.ld_a_mem(&mut ctx, x),
                Opcode::IncRr { x } => _ = instruction_sink.inc_rr(&mut ctx, x),
                Opcode::DecRr { x } => _ = instruction_sink.dec_rr(&mut ctx, x),
                Opcode::AddHlRr { x } => _ = instruction_sink.add_hl_rr(&mut ctx, x),
                Opcode::IncR { x } => _ = instruction_sink.inc_r(&mut ctx, x),
                Opcode::DecR { x } => _ = instruction_sink.dec_r(&mut ctx, x),
                Opcode::LdRN { x } => {
                    let imm = dmg_state.memory.read_byte(ctx.traced_pc);
                    ctx.increment_pc();
                    instruction_sink.ld_r_n(&mut ctx, x, imm);
                }
                Opcode::Rlca => _ = instruction_sink.rlca(&mut ctx),
                Opcode::Rrca => _ = instruction_sink.rrca(&mut ctx),
                Opcode::Rra => _ = instruction_sink.rra(&mut ctx),
                Opcode::Cpl => _ = instruction_sink.cpl(&mut ctx),
                Opcode::Scf => _ = instruction_sink.scf(&mut ctx),
                Opcode::Ccf => _ = instruction_sink.ccf(&mut ctx),
                Opcode::JrE => {
                    let prev_pc = ctx.traced_pc - 1;

                    let e = dmg_state.memory.read_byte(ctx.traced_pc).cast_signed();
                    ctx.increment_pc();

                    let address = ctx.traced_pc.wrapping_add_signed(i16::from(e));

                    let outside_rom = address >= 0x8000;
                    if outside_rom || leaving_bank_0(prev_pc, address) {
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
                    instruction_sink.add_n(&mut ctx, imm);
                }
                Opcode::AdcN => {
                    let imm = dmg_state.memory.read_byte(ctx.traced_pc);
                    ctx.increment_pc();
                    instruction_sink.adc_n(&mut ctx, imm);
                }
                Opcode::SubN => {
                    let imm = dmg_state.memory.read_byte(ctx.traced_pc);
                    ctx.increment_pc();
                    instruction_sink.sub_n(&mut ctx, imm);
                }
                Opcode::SbcN => {
                    let imm = dmg_state.memory.read_byte(ctx.traced_pc);
                    ctx.increment_pc();
                    instruction_sink.sbc_n(&mut ctx, imm);
                }
                Opcode::AndN => {
                    let imm = dmg_state.memory.read_byte(ctx.traced_pc);
                    ctx.increment_pc();
                    instruction_sink.and_n(&mut ctx, imm);
                }
                Opcode::XorN => {
                    let imm = dmg_state.memory.read_byte(ctx.traced_pc);
                    ctx.increment_pc();
                    instruction_sink.xor_n(&mut ctx, imm);
                }
                Opcode::OrN => {
                    let imm = dmg_state.memory.read_byte(ctx.traced_pc);
                    ctx.increment_pc();
                    instruction_sink.or_n(&mut ctx, imm);
                }
                Opcode::CpN => {
                    let imm = dmg_state.memory.read_byte(ctx.traced_pc);
                    ctx.increment_pc();
                    instruction_sink.cp_n(&mut ctx, imm);
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
                    if outside_rom || leaving_bank_0(prev_pc, address) {
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
                    if outside_rom || leaving_bank_0(prev_pc, address) {
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
                    #[cfg(feature = "log-traces")]
                    &mut sm83_disassembly,
                ),
                Opcode::LdHlSpPlusE => {
                    let imm = dmg_state.memory.read_byte(ctx.traced_pc);
                    let e = imm.cast_signed();

                    ctx.increment_pc();
                    instruction_sink.ld_hl_sp_plus_e(&mut ctx, e);
                }
                Opcode::LdSpHl => _ = instruction_sink.ld_sp_hl(&mut ctx),
                _ => break,
            }

            // Add the number of cycles this instruction took to total_m_cycles.
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

            #[cfg(feature = "log-traces")]
            sm83_disassembly.push_str(&format!("{:?}\n", opcode));

            ctx.block_size += 1;
            if ctx.block_size >= MAX_BLOCK_SIZE {
                break;
            }
        }

        // The final instruction couldn't be added to the block, so retroactively decrement the M-cycle and PC used to fetch it.
        ctx.total_m_cycles -= 1;
        ctx.traced_pc -= 1;

        if ctx.block_size < MIN_BLOCK_SIZE {
            return None;
        }

        #[cfg(feature = "log-traces")]
        console_log(&sm83_disassembly);

        let mut module = empty_jit_block_module();

        // Encode the code section.
        let mut codes = CodeSection::new();

        let mut function = empty_jit_block_function(ctx.regs_used.len() as u32);
        function
            .instructions()
            .prologue(registers_ptr, &ctx.regs_used);

        if ctx.needs_outer_block {
            // Wrap our JIT'd instructions in a block so we can break early if needed.
            function.instructions().block(BlockType::Empty);
        }

        function.raw(instruction_vec);

        if ctx.needs_outer_block {
            // End the inner block.
            function.instructions().end();
        }

        // Add the calling convention's epilogue.
        function
            .instructions()
            .epilogue(registers_ptr, &ctx.regs_used)
            .end();
        codes.function(&function);
        module.section(&codes);

        ctx.add_checkpoint();

        Some(WasmBlock {
            buffer: module.finish(),
            ctx,
        })
    }
}

/// Recompile the prefix opcode at PC.
fn recompile_prefix(
    dmg_state: &mut Cpu,
    ctx: &mut CodegenCtx,
    instruction_sink: &mut InstructionSink,
    #[cfg(feature = "log-traces")] sm83_disassembly: &mut String,
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

    #[cfg(feature = "log-traces")]
    sm83_disassembly.push_str(&format!("{:?}\n", prefix_opcode));
}

fn leaving_bank_0(prev_pc: u16, pc: u16) -> bool {
    prev_pc < ROM_BANK_0_END && pc >= ROM_BANK_0_END
}
