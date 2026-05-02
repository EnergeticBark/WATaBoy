use crate::codegen::CodegenCtx;
use crate::codegen::macros::{FlagBit, Sm83Macros};
use crate::codegen::module::PROLOGE_LENGTH;
use crate::codegen::registers::{A, C, SP};

use interpreter::cpu::opcodes::parameters::{R16, R16Stack};
use wasm_encoder::InstructionSink;

// Emit Wasm bytecode for Block 1.
// See: https://gbdev.io/pandocs/CPU_Instruction_Set.html#block-1-8-bit-register-to-register-loads
pub trait Block3 {
    fn add_n(&mut self, imm: u8) -> &mut Self;
    fn adc_n(&mut self, imm: u8) -> &mut Self;
    fn sub_n(&mut self, imm: u8) -> &mut Self;
    fn sbc_n(&mut self, imm: u8) -> &mut Self;
    fn and_n(&mut self, imm: u8) -> &mut Self;
    fn xor_n(&mut self, imm: u8) -> &mut Self;
    fn or_n(&mut self, imm: u8) -> &mut Self;
    fn cp_n(&mut self, imm: u8) -> &mut Self;
    fn call_nn(&mut self, ctx: &mut CodegenCtx) -> &mut Self;
    fn pop_rr(&mut self, ctx: &mut CodegenCtx, r16_stack: R16Stack) -> &mut Self;
    fn push_rr(&mut self, ctx: &mut CodegenCtx, r16_stack: R16Stack) -> &mut Self;
    fn ldh_c_a(&mut self, ctx: &mut CodegenCtx) -> &mut Self;
    fn ldh_n_a(&mut self, ctx: &mut CodegenCtx, imm: u8) -> &mut Self;
    fn ld_nn_a(&mut self, ctx: &mut CodegenCtx, imm: u16) -> &mut Self;
    fn ldh_a_n(&mut self, ctx: &mut CodegenCtx, imm: u8) -> &mut Self;
    fn ld_a_nn(&mut self, ctx: &mut CodegenCtx, imm: u16) -> &mut Self;
    fn ld_hl_sp_plus_e(&mut self, e: i8) -> &mut Self;
    fn ld_sp_hl(&mut self) -> &mut Self;
}

impl Block3 for InstructionSink<'_> {
    // TODO: Ensure immediate values in separate ROM banks aren't cached.
    // E.g. 0x3FFF: AddN, 0x4000: 64. A bank switch could invalidate this immediate value.
    fn add_n(&mut self, imm: u8) -> &mut Self {
        // Name our scratch register.
        const PREV_A: u32 = PROLOGE_LENGTH;
        self.clear_flags() // Maybe add a macro for *assigning* flags too so we don't have to do this separately from setting the first flag.
            // *** Store original value of A so it can be used to calculate the half-carry. ***
            .local_get(A)
            .local_tee(PREV_A)
            .i32_const(i32::from(imm))
            /* Perform the addition (result not yet truncated):
             * A = A + IMM
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
             * ((A & 0x0f) + (IMM & 0x0f)) > 0x0f
             */
            .local_get(PREV_A)
            .i32_const(0x0f)
            .i32_and() // (A & 0x0f)
            .i32_const(i32::from(imm) & 0x0f) // (IMM & 0x0f)
            .i32_add()
            .i32_const(0x0f)
            .i32_gt_u()
            .set_flag(FlagBit::HalfCarry)
    }
    fn adc_n(&mut self, imm: u8) -> &mut Self {
        // Name our scratch registers.
        const PREV_CARRY: u32 = PROLOGE_LENGTH;
        self.check_flag(FlagBit::Carry) // *** Store original value of Carry. ***
            .local_set(PREV_CARRY)
            .clear_flags()
            /* Calculate Half-Carry Flag:
             * ((A & 0x0f) + (IMM & 0x0f)) + PREV_CARRY  > 0x0f
             */
            .local_get(A)
            .i32_const(0x0f)
            .i32_and() // (A & 0x0f)
            .i32_const(i32::from(imm) & 0x0f)
            .i32_add()
            .local_get(PREV_CARRY)
            .i32_add()
            .i32_const(0x0f)
            .i32_gt_u()
            .set_flag(FlagBit::HalfCarry)
            /* Perform the ADD (result not yet truncated):
             * A = A + IMM + PREV_CARRY
             */
            .local_get(A)
            .i32_const(i32::from(imm))
            .i32_add()
            .local_get(PREV_CARRY)
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
    }
    fn sub_n(&mut self, imm: u8) -> &mut Self {
        // Name our scratch register.
        const PREV_A: u32 = PROLOGE_LENGTH;
        self.assign_flags(false, true, false, false) // Always set subtraction to 1.
            .local_get(A)
            .local_tee(PREV_A)
            /* Calculate Half-Carry Flag:
             * (A & 0x0F) < (IMM & 0x0F)
             */
            .i32_const(0x0f)
            .i32_and() // (A & 0x0f)
            .i32_const(i32::from(imm) & 0x0f)
            .i32_lt_u()
            .set_flag(FlagBit::HalfCarry)
            /* Calculate Carry Flag:
             * A < IMM
             */
            .local_get(A)
            .i32_const(i32::from(imm))
            .i32_lt_u() // If A < IMM (underflow), then 1, otherwise 0.
            .set_flag(FlagBit::Carry)
            /* Perform the SUB:
             * A = (A - IMM) & 0xff
             */
            .local_get(A)
            .i32_const(i32::from(imm))
            .i32_sub()
            .i32_const(0xff)
            .i32_and()
            .local_tee(A)
            // *** Calculate Zero Flag. ***
            .i32_eqz() // If the A is zero, then 1, otherwise 0.
            .set_flag(FlagBit::Zero)
    }
    fn sbc_n(&mut self, imm: u8) -> &mut Self {
        // Name our scratch registers.
        const PREV_CARRY: u32 = PROLOGE_LENGTH;
        self.check_flag(FlagBit::Carry) // *** Store original value of Carry. ***
            .local_set(PREV_CARRY)
            .assign_flags(false, true, false, false) // Always set subtraction to 1.
            /* Calculate Half-Carry Flag:
             * (A & 0x0f) < ((IMM & 0x0f) + PREV_CARRY)
             */
            .local_get(A)
            .i32_const(0x0f)
            .i32_and() // (A & 0x0f)
            .i32_const(i32::from(imm) & 0x0f)
            .local_get(PREV_CARRY)
            .i32_add() // ((IMM & 0x0f) + PREV_CARRY)
            .i32_lt_u()
            .set_flag(FlagBit::HalfCarry)
            /* Calculate Carry Flag:
             * A < (IMM + PREV_CARRY)
             */
            .local_get(A)
            .i32_const(i32::from(imm))
            .local_get(PREV_CARRY)
            .i32_add()
            .i32_lt_u() // If A < (IMM + PREV_CARRY) (underflow), then 1, otherwise 0.
            .set_flag(FlagBit::Carry)
            /* Perform the SUB:
             * A = (A - (IMM + PREV_CARRY)) & 0xff
             */
            .local_get(A)
            .i32_const(i32::from(imm))
            .local_get(PREV_CARRY)
            .i32_add() // (IMM + PREV_CARRY)
            .i32_sub()
            .i32_const(0xff)
            .i32_and()
            .local_tee(A)
            // *** Calculate Zero Flag. ***
            .i32_eqz() // If the A is zero, then 1, otherwise 0.
            .set_flag(FlagBit::Zero)
    }
    fn and_n(&mut self, imm: u8) -> &mut Self {
        self.assign_flags(false, false, true, false) // Always set half-carry to 1.
            .local_get(A)
            .i32_const(i32::from(imm))
            /* Perform the AND:
             * A = A & IMM
             */
            .i32_and()
            .local_tee(A)
            // *** Calculate Zero Flag. ***
            .i32_eqz() // If the A is zero, then 1, otherwise 0.
            .set_flag(FlagBit::Zero)
    }
    fn xor_n(&mut self, imm: u8) -> &mut Self {
        self.clear_flags()
            .local_get(A)
            .i32_const(i32::from(imm))
            /* Perform the XOR:
             * A = A ^ IMM
             */
            .i32_xor()
            .local_tee(A)
            // *** Calculate Zero Flag. ***
            .i32_eqz() // If the A is zero, then 1, otherwise 0.
            .set_flag(FlagBit::Zero)
    }
    fn or_n(&mut self, imm: u8) -> &mut Self {
        self.clear_flags()
            .local_get(A)
            .i32_const(i32::from(imm))
            /* Perform the OR:
             * A = A | IMM
             */
            .i32_or()
            .local_tee(A)
            // *** Calculate Zero Flag. ***
            .i32_eqz() // If the A is zero, then 1, otherwise 0.
            .set_flag(FlagBit::Zero)
    }
    fn cp_n(&mut self, imm: u8) -> &mut Self {
        self.assign_flags(false, true, false, false) // Always set subtraction to 1.
            /* Calculate Half-Carry Flag:
             * (A & 0x0f) < (IMM & 0x0f)
             */
            .local_get(A)
            .i32_const(0x0f)
            .i32_and() // (A & 0x0f)
            .i32_const(i32::from(imm) & 0x0f)
            .i32_lt_u()
            .set_flag(FlagBit::HalfCarry)
            /* Calculate Carry Flag:
             * A < IMM
             */
            .local_get(A)
            .i32_const(i32::from(imm))
            .i32_lt_u() // If A < IMM (underflow), then 1, otherwise 0.
            .set_flag(FlagBit::Carry)
            /* Perform the SUB:
             * (A - IMM) & 0xff
             */
            .local_get(A)
            .i32_const(i32::from(imm))
            .i32_sub()
            .i32_const(0xff)
            .i32_and()
            // *** Calculate Zero Flag. ***
            .i32_eqz() // If the result is zero, then 1, otherwise 0.
            .set_flag(FlagBit::Zero)
    }
    fn call_nn(&mut self, ctx: &mut CodegenCtx) -> &mut Self {
        ctx.increment_m_cycles(3);
        let [low, high] = ctx.traced_pc.to_le_bytes();
        // Pop the low byte.
        self.i32_const(i32::from(high))
            // Push the high byte.
            .push_byte(ctx)
            .i32_const(i32::from(low))
            // Push the low byte.
            .push_byte(ctx)
    }
    fn pop_rr(&mut self, ctx: &mut CodegenCtx, r16_stack: R16Stack) -> &mut Self {
        // Pop the low byte.
        self.pop_byte(ctx)
            // Pop the high byte.
            .pop_byte(ctx)
            .set_r16_stack(r16_stack)
    }
    fn push_rr(&mut self, ctx: &mut CodegenCtx, r16_stack: R16Stack) -> &mut Self {
        ctx.increment_m_cycles(1);
        self.get_r16_stack(r16_stack)
            // Push the high byte.
            .push_byte(ctx)
            // Push the low byte.
            .push_byte(ctx)
    }
    fn ldh_c_a(&mut self, ctx: &mut CodegenCtx) -> &mut Self {
        self.local_get(A)
            .local_get(C)
            .i32_const(0xFF00)
            .i32_or()
            .call_write_byte(ctx)
            .insert_checkpoint(ctx)
    }
    fn ldh_n_a(&mut self, ctx: &mut CodegenCtx, imm: u8) -> &mut Self {
        ctx.increment_m_cycles(1);
        let address = u16::from_le_bytes([imm, 0xFF]);
        self.local_get(A)
            .i32_const(i32::from(address))
            .call_write_byte(ctx)
            .insert_checkpoint(ctx)
    }
    fn ld_nn_a(&mut self, ctx: &mut CodegenCtx, imm: u16) -> &mut Self {
        ctx.increment_m_cycles(2);
        self.local_get(A)
            .i32_const(i32::from(imm))
            .call_write_byte(ctx)
    }
    fn ldh_a_n(&mut self, ctx: &mut CodegenCtx, imm: u8) -> &mut Self {
        ctx.increment_m_cycles(1);
        let address = u16::from_le_bytes([imm, 0xFF]);
        self.read_byte_static(ctx, address).local_set(A)
    }
    fn ld_a_nn(&mut self, ctx: &mut CodegenCtx, imm: u16) -> &mut Self {
        ctx.increment_m_cycles(2);
        self.read_byte_static(ctx, imm).local_set(A)
    }
    fn ld_hl_sp_plus_e(&mut self, e: i8) -> &mut Self {
        // Name our scratch register.
        const TEMP: u32 = PROLOGE_LENGTH;
        self.clear_flags()
            /* Calculate Half-Carry Flag:
             * ((SP & 0x0f) + (E & 0x0f)) > 0x0f
             */
            .local_get(SP)
            .i32_const(0x0f)
            .i32_and() // (SP & 0x0f)
            .i32_const(i32::from(e) & 0x0f) // (E & 0x0f)
            .i32_add()
            .i32_const(0x0f)
            .i32_gt_u()
            .set_flag(FlagBit::HalfCarry)
            /* Calculate Carry Flag:
             * (SP & 0xFF) + E > 0xFF
             */
            .local_get(SP)
            .i32_const(0xff)
            .i32_and()
            .i32_const(i32::from(e))
            .i32_add()
            .i32_const(0xff)
            .i32_gt_u() // If result > 0xFF (overflow), then 1, otherwise 0.
            .set_flag(FlagBit::Carry)
            /* Perform the addition:
             * HL = (SP + E) & 0xFFFF
             */
            .local_get(SP)
            .i32_const(i32::from(e))
            .i32_add()
            .i32_const(0xFFFF)
            .i32_and()
            .set_r16(R16::Hl, TEMP)
    }
    fn ld_sp_hl(&mut self) -> &mut Self {
        self.get_r16(R16::Hl).local_set(SP)
    }
}
