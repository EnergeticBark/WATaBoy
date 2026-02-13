use sm83_interp::cpu::Cpu;
use sm83_interp::opcodes;
use sm83_interp::parameters::R8;

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
    // For now, I'm just generating the whole module in here, but later, it would append to an existing function.

    let mut module = Module::new();

    // Encode the type section.
    let mut types = TypeSection::new();
    // Parameters: (0: A), (1: F), (2: B), (3: C), (4: D), (5: E), (6: H), and (7: L) registers.
    let params = vec![ValType::I32; 8];
    // Return those same registers, but modified.
    let results = vec![ValType::I32; 8];
    const A: u32 = 0;
    const F: u32 = 1;
    types.ty().function(params, results);
    module.section(&types);

    // Encode the function section.
    let mut functions = FunctionSection::new();
    let type_index = 0;
    functions.function(type_index);
    module.section(&functions);

    // Encode the export section
    let mut exports = ExportSection::new();
    exports.export("add_r", ExportKind::Func, 0);
    module.section(&exports);

    // Encode the code section.
    let mut codes = CodeSection::new();
    let locals = vec![(2, ValType::I32)];
    let prev_a = 8;
    let prev_r8 = 9;
    let mut add_r = Function::new(locals);
    add_r
        .instructions()
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
        /* Assign Overflow Flag:
         * F = (gt_u_result << 4)
         */
        .i32_const(4)
        .i32_shl()
        .local_set(F)
        /* Truncate A to 8-bits:
         * A &= 0xff
         */
        .local_get(A)
        .i32_const(0xff)
        .i32_and()
        .local_tee(A)
        // *** Calculate Zero Flag. ***
        .i32_eqz() // If the A is zero, then 1, otherwise 0.
        /* Update Zero Flag:
         * F |= (eqz_result << 7)
         */
        .i32_const(7)
        .i32_shl()
        .local_get(F)
        .i32_or()
        .local_set(F)
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
        /* Update Half-Carry Flag:
         * F |= (gt_u_result << 5)
         */
        .i32_const(5)
        .i32_shl()
        .local_get(F)
        .i32_or()
        .local_set(F)
        // Return all the registers. :)
        .local_get(A)
        .local_get(F)
        .local_get(2)
        .local_get(3)
        .local_get(4)
        .local_get(5)
        .local_get(6)
        .local_get(7)
        .end();
    codes.function(&add_r);

    module.section(&codes);

    module.finish()
}
