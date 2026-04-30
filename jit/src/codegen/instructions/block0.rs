use interpreter::cpu::opcodes::parameters::{R8, R16, R16Mem};

use crate::codegen::CodegenCtx;
use crate::codegen::macros::{FlagBit, Sm83Macros};
use crate::codegen::module::PROLOGE_LENGTH;
use crate::codegen::registers::A;

use wasm_encoder::*;

// Emit Wasm bytecode for Block 0.
// See: https://gbdev.io/pandocs/CPU_Instruction_Set.html#block-0
pub trait Block0 {
    fn nop(&mut self) -> &mut Self;
    fn ld_rr_nn(&mut self, r16: R16, imm: u16) -> &mut Self;
    fn ld_mem_a(&mut self, ctx: &mut CodegenCtx, r16_mem: R16Mem) -> &mut Self;
    fn ld_a_mem(&mut self, ctx: &mut CodegenCtx, r16_mem: R16Mem) -> &mut Self;
    fn inc_rr(&mut self, r16: R16) -> &mut Self;
    fn dec_rr(&mut self, r16: R16) -> &mut Self;
    fn add_hl_rr(&mut self, r16: R16) -> &mut Self;
    fn inc_r(&mut self, ctx: &mut CodegenCtx, r8: R8) -> &mut Self;
    fn dec_r(&mut self, ctx: &mut CodegenCtx, r8: R8) -> &mut Self;
    fn ld_r_n(&mut self, ctx: &mut CodegenCtx, r8: R8, imm: i32) -> &mut Self;
    fn rlca(&mut self) -> &mut Self;
    fn rrca(&mut self) -> &mut Self;
    fn rra(&mut self) -> &mut Self;
    fn cpl(&mut self) -> &mut Self;
    fn scf(&mut self) -> &mut Self;
    fn ccf(&mut self) -> &mut Self;
}

impl Block0 for InstructionSink<'_> {
    fn nop(&mut self) -> &mut Self {
        self.nop()
    }

    fn ld_rr_nn(&mut self, r16: R16, imm: u16) -> &mut Self {
        // Name our scratch register.
        const TEMP: u32 = PROLOGE_LENGTH as u32;
        self.i32_const(imm as i32).set_r16(r16, TEMP)
    }

    fn ld_mem_a(&mut self, ctx: &mut CodegenCtx, r16_mem: R16Mem) -> &mut Self {
        // Name our scratch register.
        const TEMP: u32 = PROLOGE_LENGTH as u32;
        self.local_get(A).set_r16_mem(ctx, r16_mem, TEMP)
    }

    fn ld_a_mem(&mut self, ctx: &mut CodegenCtx, r16_mem: R16Mem) -> &mut Self {
        // Name our scratch register.
        const TEMP: u32 = PROLOGE_LENGTH as u32;
        self.get_r16_mem(ctx, r16_mem, TEMP).local_set(A)
    }

    fn inc_rr(&mut self, r16: R16) -> &mut Self {
        // Name our scratch register.
        const TEMP: u32 = PROLOGE_LENGTH as u32;
        self.get_r16(r16).i32_const(1).i32_add().set_r16(r16, TEMP)
    }

    fn dec_rr(&mut self, r16: R16) -> &mut Self {
        // Name our scratch register.
        const TEMP: u32 = PROLOGE_LENGTH as u32;
        self.get_r16(r16).i32_const(1).i32_sub().set_r16(r16, TEMP)
    }

    fn add_hl_rr(&mut self, r16: R16) -> &mut Self {
        // Name our scratch registers.
        const PREV_HL: u32 = PROLOGE_LENGTH as u32;
        const PREV_RR: u32 = PROLOGE_LENGTH as u32 + 1;
        self.check_flag(FlagBit::Zero) // *** Preserve the original value of Zero on the stack. ***
            .clear_flags()
            .set_flag(FlagBit::Zero) // Restore Zero flag.
            // *** Store original values of HL and RR so we can calculate the half-carry first. ***
            .get_r16(R16::Hl)
            .local_tee(PREV_HL)
            .i32_const(0x0FFF)
            .i32_and()
            .get_r16(r16)
            .local_tee(PREV_RR)
            .i32_const(0x0FFF)
            .i32_and()
            /* Calculate Half-Carry Flag:
             * ((HL & 0x0FFF) + (RR & 0x0FFF)) & 0x1000 == 0x1000
             */
            .i32_add()
            .i32_const(0x1000)
            .i32_and()
            .i32_const(0x1000)
            .i32_eq()
            .set_flag(FlagBit::HalfCarry)
            /* Calculate Overflow Flag:
             * HL + RR > 0xFFFF
             */
            .local_get(PREV_HL)
            .local_get(PREV_RR)
            .i32_add()
            .i32_const(0xffff)
            .i32_gt_u()
            .set_flag(FlagBit::Carry)
            /* Perform the addition (truncated by set_r16()):
             * HL = HL + RR
             */
            .local_get(PREV_HL)
            .local_get(PREV_RR)
            .i32_add();
        // Reclaim a scratch registers to use as TEMP.
        // TODO: It would be nice to actually drop PREV_HL from scope somehow...
        const TEMP: u32 = PREV_HL;
        self.set_r16(R16::Hl, TEMP)
    }

    fn inc_r(&mut self, ctx: &mut CodegenCtx, r8: R8) -> &mut Self {
        // Name our scratch register.
        const RESULT: u32 = PROLOGE_LENGTH as u32;
        self.check_flag(FlagBit::Carry) // *** Preserve the original value of Carry on the stack. ***
            .clear_flags()
            .set_flag(FlagBit::Carry) // Restore Carry flag.
            /* Perform the increment and truncate:
             * R8 = (R8 + 1) & 0xff
             */
            .get_r8(ctx, r8)
            .i32_const(1)
            .i32_add()
            .i32_const(0xff)
            .i32_and()
            .local_tee(RESULT)
            .set_r8(ctx, r8)
            .local_get(RESULT)
            /* Calculate Half-Carry Flag:
             * RESULT.trailing_zeros() >= 4
             */
            .i32_ctz()
            .i32_const(3)
            .i32_gt_u()
            .set_flag(FlagBit::HalfCarry)
            // *** Calculate Zero Flag. ***
            .local_get(RESULT)
            .i32_eqz() // If the R8 is zero, then 1, otherwise 0.
            .set_flag(FlagBit::Zero)
    }

    fn dec_r(&mut self, ctx: &mut CodegenCtx, r8: R8) -> &mut Self {
        // Name our scratch register.
        const RESULT: u32 = PROLOGE_LENGTH as u32;
        const R8_VAL: u32 = PROLOGE_LENGTH as u32 + 1;
        self.check_flag(FlagBit::Carry) // *** Preserve the original value of Carry on the stack. ***
            .assign_flags(false, true, false, false) // Always set subtraction to 1.
            .set_flag(FlagBit::Carry) // Restore Carry flag.
            /* Perform the decrement and truncate:
             * R8 = (R8 - 1) & 0xff
             */
            .get_r8(ctx, r8)
            .local_tee(R8_VAL)
            .i32_const(1)
            .i32_sub()
            .i32_const(0xff)
            .i32_and()
            .local_tee(RESULT)
            .set_r8(ctx, r8)
            .local_get(R8_VAL)
            /* Calculate Half-Carry Flag:
             * R8.trailing_zeros() >= 4
             */
            .i32_ctz()
            .i32_const(3)
            .i32_gt_u()
            .set_flag(FlagBit::HalfCarry)
            // *** Calculate Zero Flag. ***
            .local_get(RESULT)
            .i32_eqz() // If the R8 is zero, then 1, otherwise 0.
            .set_flag(FlagBit::Zero)
    }

    fn ld_r_n(&mut self, ctx: &mut CodegenCtx, r8: R8, imm: i32) -> &mut Self {
        ctx.increment_m_cycles(1);
        self.i32_const(imm).set_r8(ctx, r8)
    }

    fn rlca(&mut self) -> &mut Self {
        // Name our scratch register.
        const BIT_7: u32 = PROLOGE_LENGTH as u32;
        self.clear_flags()
            /* Calculate the Carry flag:
             * (A >> 7) == 0b0000_0001
             */
            .local_get(A)
            .i32_const(7)
            .i32_shr_u()
            .local_tee(BIT_7)
            .set_flag(FlagBit::Carry)
            /* Perform the shift left, set the lowest bit to BIT_7, and truncate:
             * A = ((A << 1) | BIT_7) & 0xff
             */
            .local_get(A)
            .i32_const(1)
            .i32_shl()
            .local_get(BIT_7)
            .i32_or()
            .i32_const(0xff)
            .i32_and()
            .local_set(A)
    }

    fn rrca(&mut self) -> &mut Self {
        // Name our scratch register.
        const BIT_0: u32 = PROLOGE_LENGTH as u32;
        self.clear_flags()
            /* Calculate the Carry flag:
             * A & 0b0000_0001 == 0b0000_0001
             */
            .local_get(A)
            .i32_const(0b0000_0001)
            .i32_and()
            .local_tee(BIT_0)
            .set_flag(FlagBit::Carry)
            /* Perform the shift right and set the highest bit to BIT_0:
             * A = (A >> 1) | (BIT_0 << 7)
             */
            .local_get(A)
            .i32_const(1)
            .i32_shr_u()
            .local_get(BIT_0)
            .i32_const(7)
            .i32_shl()
            .i32_or()
            .local_set(A)
    }

    fn rra(&mut self) -> &mut Self {
        // Name our scratch register.
        const PREV_CARRY: u32 = PROLOGE_LENGTH as u32;
        self.check_flag(FlagBit::Carry) // *** Store original value of Carry. ***
            .local_set(PREV_CARRY)
            .clear_flags()
            /* Calculate the Carry flag:
             * A & 0b0000_0001 == 0b0000_0001
             */
            .local_get(A)
            .i32_const(0b0000_0001)
            .i32_and()
            .set_flag(FlagBit::Carry)
            /* Perform the shift right and set the highest bit to PREV_CARRY:
             * A = (A >> 1) | (PREV_CARRY << 7)
             */
            .local_get(A)
            .i32_const(1)
            .i32_shr_u()
            .local_get(PREV_CARRY)
            .i32_const(7)
            .i32_shl()
            .i32_or()
            .local_set(A)
    }

    fn cpl(&mut self) -> &mut Self {
        self.set_flags(false, true, true, false) // Always set subtraction and half carry to 1.
            /* Flip the bits in A:
             * A = (!A) & 0xff
             */
            .local_get(A)
            .i32_const(0xff)
            .i32_xor()
            .local_set(A)
    }

    fn scf(&mut self) -> &mut Self {
        self // *** Preserve the original value of Zero on the stack. ***
            .check_flag(FlagBit::Zero)
            .assign_flags(false, false, false, true)
            .set_flag(FlagBit::Zero) // Restore Zero flag.
    }

    fn ccf(&mut self) -> &mut Self {
        self.check_flag(FlagBit::Carry) // *** Preserve the original value of Carry and Zero on the stack. ***
            .check_flag(FlagBit::Zero)
            .clear_flags()
            .set_flag(FlagBit::Zero) // Restore Zero flag.
            /* Negate the Carry flag:
             * Carry = Carry == 0
             */
            .i32_eqz()
            .set_flag(FlagBit::Carry)
    }
}
