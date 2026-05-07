use std::collections::HashMap;

use hw_constants::ROM_BANK_0_END;
use interpreter::cpu::opcodes::parameters::{R8, R16, R16Mem, R16Stack};
use wasm_encoder::{BlockType, InstructionSink, MemArg};

use crate::codegen::{
    CodegenCtx,
    module::{RW_ADDR_REG, WRITE_VAL_REG},
    registers::LocalReg,
};

pub(crate) enum FlagBit {
    Zero = 7,
    Subtraction = 6,
    HalfCarry = 5,
    Carry = 4,
}

pub(crate) trait Sm83Macros {
    fn get_reg(&mut self, ctx: &mut CodegenCtx, reg: LocalReg) -> &mut Self;
    fn set_reg(&mut self, ctx: &mut CodegenCtx, reg: LocalReg) -> &mut Self;
    fn tee_reg(&mut self, ctx: &mut CodegenCtx, reg: LocalReg) -> &mut Self;
    fn get_r8(&mut self, ctx: &mut CodegenCtx, r8: R8) -> &mut Self;
    fn set_r8(&mut self, ctx: &mut CodegenCtx, r8: R8) -> &mut Self;
    fn get_r16(&mut self, ctx: &mut CodegenCtx, r16: R16) -> &mut Self;
    fn set_r16(&mut self, ctx: &mut CodegenCtx, r16: R16, temp_reg: u32) -> &mut Self;
    fn set_r16_static(&mut self, ctx: &mut CodegenCtx, r16: R16, value: u16) -> &mut Self;
    fn get_r16_mem(&mut self, ctx: &mut CodegenCtx, r16_mem: R16Mem, temp_reg: u32) -> &mut Self;
    fn set_r16_mem(&mut self, ctx: &mut CodegenCtx, r16: R16Mem, temp_reg: u32) -> &mut Self;
    fn get_r16_stack(&mut self, ctx: &mut CodegenCtx, r16: R16Stack) -> &mut Self;
    fn set_r16_stack(&mut self, ctx: &mut CodegenCtx, r16: R16Stack) -> &mut Self;
    fn pop_byte(&mut self, ctx: &mut CodegenCtx) -> &mut Self;
    fn push_byte(&mut self, ctx: &mut CodegenCtx) -> &mut Self;
    fn clear_flags(&mut self, ctx: &mut CodegenCtx) -> &mut Self;
    // TODO: I should probably just reuse the Flag struct defined in the interpreter here.
    #[allow(clippy::fn_params_excessive_bools)]
    fn assign_flags(
        &mut self,
        ctx: &mut CodegenCtx,
        zero: bool,
        subtraction: bool,
        half_carry: bool,
        carry: bool,
    ) -> &mut Self;
    #[allow(clippy::fn_params_excessive_bools)]
    fn set_flags(
        &mut self,
        ctx: &mut CodegenCtx,
        zero: bool,
        subtraction: bool,
        half_carry: bool,
        carry: bool,
    ) -> &mut Self;
    fn set_flag(&mut self, ctx: &mut CodegenCtx, flag_bit: FlagBit) -> &mut Self;
    fn check_flag(&mut self, ctx: &mut CodegenCtx, flag_bit: FlagBit) -> &mut Self;
    fn insert_checkpoint(&mut self, ctx: &mut CodegenCtx) -> &mut Self;
    fn prologue(&mut self, registers_ptr: usize, regs_used: &HashMap<LocalReg, u32>) -> &mut Self;
    fn epilogue(&mut self, registers_ptr: usize, regs_used: &HashMap<LocalReg, u32>) -> &mut Self;
    fn read_byte_static(&mut self, ctx: &mut CodegenCtx, addr: u16) -> &mut Self;
    fn write_byte_static<'a: 'b, 'b, F>(
        &'a mut self,
        ctx: &'b mut CodegenCtx,
        addr: u16,
        f: F,
    ) -> &'a mut Self
    where
        F: FnOnce(&'a mut Self, &'b mut CodegenCtx) -> (&'a mut Self, &'b mut CodegenCtx);
    fn call_read_byte(&mut self, ctx: &mut CodegenCtx) -> &mut Self;
    fn call_write_byte(&mut self, ctx: &mut CodegenCtx) -> &mut Self;
}

impl Sm83Macros for InstructionSink<'_> {
    fn get_reg(&mut self, ctx: &mut CodegenCtx, reg: LocalReg) -> &mut Self {
        let index = reg.to_index(ctx);
        self.local_get(index)
    }

    fn set_reg(&mut self, ctx: &mut CodegenCtx, reg: LocalReg) -> &mut Self {
        let index = reg.to_index(ctx);
        self.local_set(index)
    }

    fn tee_reg(&mut self, ctx: &mut CodegenCtx, reg: LocalReg) -> &mut Self {
        let index = reg.to_index(ctx);
        self.local_tee(index)
    }

    /// Get the value of the specified 8-bit register.
    /// If R8 is [HL], `total_m_cycles` will increase by 1.
    /// # Signature
    /// ```
    /// () -> (value: i32)
    /// ```
    fn get_r8(&mut self, ctx: &mut CodegenCtx, r8: R8) -> &mut Self {
        match r8 {
            R8::IndirectHL => self.get_r16(ctx, R16::Hl).call_read_byte(ctx),
            _ => self.get_reg(ctx, r8.try_into().unwrap()),
        }
    }

    /// Set the value of the specified 8-bit register.
    /// If R8 is [HL], `total_m_cycles` will increase by 1.
    /// # Signature
    /// ```
    /// (value: i32) -> ()
    /// ```
    fn set_r8(&mut self, ctx: &mut CodegenCtx, r8: R8) -> &mut Self {
        match r8 {
            R8::IndirectHL => self.get_r16(ctx, R16::Hl).call_write_byte(ctx),
            _ => self.set_reg(ctx, r8.try_into().unwrap()),
        }
    }

    /// Get the value of the specified 16-bit register.
    /// # Signature
    /// ```
    /// () -> (r16: i32)
    /// ```
    fn get_r16(&mut self, ctx: &mut CodegenCtx, r16: R16) -> &mut Self {
        let (high_reg, low_reg) = match r16 {
            R16::Bc => (LocalReg::B, LocalReg::C),
            R16::De => (LocalReg::D, LocalReg::E),
            R16::Hl => (LocalReg::H, LocalReg::L),
            R16::Sp => unimplemented!("SP isn't in the JIT prelude/epilogue yet."),
        };

        self.get_reg(ctx, high_reg)
            .i32_const(8)
            .i32_shl()
            .get_reg(ctx, low_reg)
            .i32_or()
    }

    /// Set the value of the specified 16-bit register.
    /// The value passed in will effectively be truncated to 16-bits.
    /// This macro needs a temporary register to store the 16-bit value before writing each byte separately.
    /// The register specified in `temp_reg` will be clobbered.
    /// # Signature
    /// ```
    /// (value: i32) -> ()
    /// ```
    fn set_r16(&mut self, ctx: &mut CodegenCtx, r16: R16, temp_reg: u32) -> &mut Self {
        let (high_reg, low_reg) = match r16 {
            R16::Bc => (LocalReg::B, LocalReg::C),
            R16::De => (LocalReg::D, LocalReg::E),
            R16::Hl => (LocalReg::H, LocalReg::L),
            R16::Sp => return self.set_reg(ctx, LocalReg::SP),
        };

        self.local_tee(temp_reg)
            .i32_const(8)
            .i32_shr_u()
            .i32_const(0xFF)
            .i32_and()
            .set_reg(ctx, high_reg)
            .local_get(temp_reg)
            .i32_const(0xFF)
            .i32_and()
            .set_reg(ctx, low_reg)
    }

    fn set_r16_static(&mut self, ctx: &mut CodegenCtx, r16: R16, value: u16) -> &mut Self {
        let (high_reg, low_reg) = match r16 {
            R16::Bc => (LocalReg::B, LocalReg::C),
            R16::De => (LocalReg::D, LocalReg::E),
            R16::Hl => (LocalReg::H, LocalReg::L),
            R16::Sp => return self.i32_const(i32::from(value)).set_reg(ctx, LocalReg::SP),
        };

        let [high, low] = value.to_be_bytes();

        self.i32_const(i32::from(high))
            .set_reg(ctx, high_reg)
            .i32_const(i32::from(low))
            .set_reg(ctx, low_reg)
    }

    /// Get the value at the location in memory pointed to by the specified 16-bit memory register.
    /// The register specified in `temp_reg` will be clobbered.
    /// # Signature
    /// ```
    /// () -> (value: i32)
    /// ```
    /// # Side Effects
    /// Calls `call_read_byte`.
    fn get_r16_mem(&mut self, ctx: &mut CodegenCtx, r16_mem: R16Mem, temp_reg: u32) -> &mut Self {
        let (high_reg, low_reg) = match r16_mem {
            R16Mem::Bc => (LocalReg::B, LocalReg::C),
            R16Mem::De => (LocalReg::D, LocalReg::E),
            _ => (LocalReg::H, LocalReg::L),
        };

        self.get_reg(ctx, high_reg)
            .i32_const(8)
            .i32_shl()
            .get_reg(ctx, low_reg)
            .i32_or();

        if matches!(r16_mem, R16Mem::HlInc | R16Mem::HlDec) {
            self.local_tee(temp_reg);
        }

        self.call_read_byte(ctx);

        match r16_mem {
            R16Mem::HlInc => {
                self.local_get(temp_reg)
                    .i32_const(1)
                    .i32_add()
                    .set_r16(ctx, R16::Hl, temp_reg);
            }
            R16Mem::HlDec => {
                self.local_get(temp_reg)
                    .i32_const(1)
                    .i32_sub()
                    .set_r16(ctx, R16::Hl, temp_reg);
            }
            _ => (),
        }
        self
    }

    /// Set the value at the location in memory pointed to by the specified 16-bit memory register.
    /// The register specified in `temp_reg` will be clobbered.
    /// # Signature
    /// ```
    /// (value: i32) -> ()
    /// ```
    /// # Side Effects
    /// Calls `call_write_byte`.
    fn set_r16_mem(&mut self, ctx: &mut CodegenCtx, r16_mem: R16Mem, temp_reg: u32) -> &mut Self {
        let (high_reg, low_reg) = match r16_mem {
            R16Mem::Bc => (LocalReg::B, LocalReg::C),
            R16Mem::De => (LocalReg::D, LocalReg::E),
            _ => (LocalReg::H, LocalReg::L),
        };

        self.get_reg(ctx, high_reg)
            .i32_const(8)
            .i32_shl()
            .get_reg(ctx, low_reg)
            .i32_or();

        if matches!(r16_mem, R16Mem::HlInc | R16Mem::HlDec) {
            self.local_tee(temp_reg);
        }

        self.call_write_byte(ctx);

        match r16_mem {
            R16Mem::HlInc => {
                self.local_get(temp_reg)
                    .i32_const(1)
                    .i32_add()
                    .set_r16(ctx, R16::Hl, temp_reg);
            }
            R16Mem::HlDec => {
                self.local_get(temp_reg)
                    .i32_const(1)
                    .i32_sub()
                    .set_r16(ctx, R16::Hl, temp_reg);
            }
            _ => (),
        }
        self
    }

    /// Get the value of the specified 16-bit stack register.
    /// # Signature
    /// ```
    /// () -> (low_byte: i32, high_byte: i32)
    /// ```
    fn get_r16_stack(&mut self, ctx: &mut CodegenCtx, r16: R16Stack) -> &mut Self {
        let (high_reg, low_reg) = match r16 {
            R16Stack::Bc => (LocalReg::B, LocalReg::C),
            R16Stack::De => (LocalReg::D, LocalReg::E),
            R16Stack::Hl => (LocalReg::H, LocalReg::L),
            R16Stack::Af => (LocalReg::A, LocalReg::F),
        };

        self.get_reg(ctx, low_reg).get_reg(ctx, high_reg)
    }

    /// Set the value of the specified 16-bit stack register.
    /// The parameters to this macro are in reverse order compared to values returned by `get_r16_stack`.
    /// # Signature
    /// ```
    /// (high_byte: i32, low_byte: i32) -> ()
    /// ```
    fn set_r16_stack(&mut self, ctx: &mut CodegenCtx, r16: R16Stack) -> &mut Self {
        let (high_reg, low_reg) = match r16 {
            R16Stack::Bc => (LocalReg::B, LocalReg::C),
            R16Stack::De => (LocalReg::D, LocalReg::E),
            R16Stack::Hl => (LocalReg::H, LocalReg::L),
            // TODO: Don't set the lower nibble of F!!!
            R16Stack::Af => (LocalReg::A, LocalReg::F),
        };

        self.set_reg(ctx, high_reg).set_reg(ctx, low_reg)
    }

    /// Pop an 8-bit value from the stack.
    /// # Signature
    /// ```
    /// () -> (value: i32)
    /// ```
    /// # Side Effects
    /// Increments the system clock by 1 M-cycle after reading at SP.
    /// # Pseudocode
    /// ```
    /// mem[SP++]
    /// ```
    fn pop_byte(&mut self, ctx: &mut CodegenCtx) -> &mut Self {
        self.get_reg(ctx, LocalReg::SP)
            .call_read_byte(ctx)
            .get_reg(ctx, LocalReg::SP)
            .i32_const(1)
            .i32_add()
            .set_reg(ctx, LocalReg::SP)
    }

    /// Push an 8-bit value from the stack.
    /// # Signature
    /// ```
    /// (value: i32) -> ()
    /// ```
    /// # Side Effects
    /// Increments the system clock by 1 M-cycle after writing at SP.
    /// # Pseudocode
    /// ```
    /// mem[--SP] = val
    /// ```
    fn push_byte(&mut self, ctx: &mut CodegenCtx) -> &mut Self {
        // Pre-decrement SP.
        self.get_reg(ctx, LocalReg::SP)
            .i32_const(1)
            .i32_sub()
            .tee_reg(ctx, LocalReg::SP)
            .call_write_byte(ctx)
    }

    /// Clear all bits in the flag register.
    /// # Signature
    /// ```
    /// () -> ()
    /// ```
    /// # Pseudocode
    /// ```
    /// F = 0x00
    /// ```
    fn clear_flags(&mut self, ctx: &mut CodegenCtx) -> &mut Self {
        self.i32_const(0x00).set_reg(ctx, LocalReg::F)
    }

    /// Unconditionally set the specified flags to true, leaving the others unmodified.
    /// # Signature
    /// ```
    /// () -> ()
    /// ```
    /// # Pseudocode
    /// ```
    /// F |= specified_flags
    /// ```
    fn set_flags(
        &mut self,
        ctx: &mut CodegenCtx,
        zero: bool,
        subtraction: bool,
        half_carry: bool,
        carry: bool,
    ) -> &mut Self {
        let mut flags: u8 = 0b0000_0000;
        if zero {
            flags |= 1 << FlagBit::Zero as usize;
        }
        if subtraction {
            flags |= 1 << FlagBit::Subtraction as usize;
        }
        if half_carry {
            flags |= 1 << FlagBit::HalfCarry as usize;
        }
        if carry {
            flags |= 1 << FlagBit::Carry as usize;
        }

        self.get_reg(ctx, LocalReg::F)
            .i32_const(i32::from(flags))
            .i32_or()
            .set_reg(ctx, LocalReg::F)
    }

    /// Assign the bits in the flag register, overwriting any previous value.
    /// # Signature
    /// ```
    /// () -> ()
    /// ```
    /// # Pseudocode
    /// ```
    /// F = flag_bits
    /// ```
    fn assign_flags(
        &mut self,
        ctx: &mut CodegenCtx,
        zero: bool,
        subtraction: bool,
        half_carry: bool,
        carry: bool,
    ) -> &mut Self {
        let mut flags: u8 = 0b0000_0000;
        if zero {
            flags |= 1 << FlagBit::Zero as usize;
        }
        if subtraction {
            flags |= 1 << FlagBit::Subtraction as usize;
        }
        if half_carry {
            flags |= 1 << FlagBit::HalfCarry as usize;
        }
        if carry {
            flags |= 1 << FlagBit::Carry as usize;
        }

        self.i32_const(i32::from(flags)).set_reg(ctx, LocalReg::F)
    }

    /// Set the selected bit in the flag register. This will only change a 0 to a 1, not vice-versa.
    /// # Signature
    /// ```
    /// (bool: i32) -> ()
    /// ```
    /// # Pseudocode
    /// ```
    /// F |= (top_of_stack << flag_bit)
    /// ```
    fn set_flag(&mut self, ctx: &mut CodegenCtx, flag_bit: FlagBit) -> &mut Self {
        self.i32_const(flag_bit as i32)
            .i32_shl()
            .get_reg(ctx, LocalReg::F)
            .i32_or()
            .set_reg(ctx, LocalReg::F)
    }

    /// Check if the selected flag's bit is set in the flag register.
    /// # Signature
    /// ```
    /// () -> (bool: i32)
    /// ```
    /// # Pseudocode
    /// ```
    /// return (F >> flag_bit) & 1
    /// ```
    fn check_flag(&mut self, ctx: &mut CodegenCtx, flag_bit: FlagBit) -> &mut Self {
        self.get_reg(ctx, LocalReg::F)
            .i32_const(flag_bit as i32)
            .i32_shr_u()
            .i32_const(0x01)
            .i32_and()
    }

    /// Create a checkpoint.
    /// Abort the current block's execution if the next interrupt is scheduled to occur before the next checkpoint.
    /// # Signature
    /// ```
    /// () -> ()
    /// ```
    #[allow(clippy::cast_possible_truncation)]
    #[allow(clippy::cast_possible_wrap)]
    fn insert_checkpoint(&mut self, ctx: &mut CodegenCtx) -> &mut Self {
        ctx.needs_outer_block = true;

        let checkpoint_index = ctx.add_checkpoint();
        self.i32_const(checkpoint_index as i32)
            .i32_const(ctx.runtime_ptr as i32)
            .call(2)
            .br_if(0)
    }

    /// Load all of the registers to satisfy the calling convention.
    /// Usually this is the first macro in a block.
    /// # Signature
    /// ```
    /// () -> ()
    /// ```
    #[allow(clippy::cast_possible_truncation)]
    #[allow(clippy::cast_possible_wrap)]
    fn prologue(&mut self, registers_ptr: usize, regs_used: &HashMap<LocalReg, u32>) -> &mut Self {
        let register_mem_offsets = [
            LocalReg::F,
            LocalReg::A,
            LocalReg::C,
            LocalReg::B,
            LocalReg::E,
            LocalReg::D,
            LocalReg::L,
            LocalReg::H,
        ];
        for (reg, offset) in register_mem_offsets.iter().zip(0..) {
            if let Some(&reg_index) = regs_used.get(reg) {
                self.i32_const(registers_ptr as i32)
                    .i32_load8_u(MemArg {
                        offset,
                        align: 0,
                        memory_index: 0,
                    })
                    .local_set(reg_index);
            }
        }

        if let Some(&sp_index) = regs_used.get(&LocalReg::SP) {
            self.i32_const(registers_ptr as i32)
                .i32_load16_u(MemArg {
                    offset: 8,
                    align: 0,
                    memory_index: 0,
                })
                .local_set(sp_index);
        }
        self
    }

    /// Store all of the registers to satisfy the calling convention.
    /// Usually this is the final macro in a block.
    /// # Signature
    /// ```
    /// () -> ()
    /// ```
    #[allow(clippy::cast_possible_truncation)]
    #[allow(clippy::cast_possible_wrap)]
    fn epilogue(&mut self, registers_ptr: usize, regs_used: &HashMap<LocalReg, u32>) -> &mut Self {
        let register_mem_offsets = [
            LocalReg::F,
            LocalReg::A,
            LocalReg::C,
            LocalReg::B,
            LocalReg::E,
            LocalReg::D,
            LocalReg::L,
            LocalReg::H,
        ];
        for (reg, offset) in register_mem_offsets.iter().zip(0..) {
            if let Some(&reg_index) = regs_used.get(reg) {
                self.i32_const(registers_ptr as i32)
                    .local_get(reg_index)
                    .i32_store8(MemArg {
                        offset,
                        align: 0,
                        memory_index: 0,
                    });
            }
        }

        if let Some(&sp_index) = regs_used.get(&LocalReg::SP) {
            self.i32_const(registers_ptr as i32)
                .local_get(sp_index)
                .i32_store16(MemArg {
                    offset: 8,
                    align: 0,
                    memory_index: 0,
                });
        }
        self
    }

    /// Read a byte from the Game Boy's memory using a static address known at compile time.
    /// This can allow for optimisations such as reads from work RAM bypassing `call_read_byte`.
    /// # Signature
    /// ```
    /// () -> (value: i32)
    /// ```
    /// # Side Effects
    /// 1. Increments M-cycles by 1.
    #[allow(clippy::cast_possible_truncation)]
    #[allow(clippy::cast_possible_wrap)]
    fn read_byte_static(&mut self, ctx: &mut CodegenCtx, addr: u16) -> &mut Self {
        if (0xC000..0xE000).contains(&addr) {
            // If addr is in work RAM, emit a load instruction to read the byte directly.
            self.i32_const(ctx.work_ram_ptr as i32).i32_load8_u(MemArg {
                offset: u64::from(addr),
                align: 0,
                memory_index: 0,
            });
            ctx.increment_m_cycles(1);
            self
        } else {
            // Otherwise fall back to call_read_byte().
            self.i32_const(i32::from(addr)).call_read_byte(ctx)
        }
    }

    /// Write a byte to the Game Boy's memory using a static address known at compile time.
    /// This can allow for optimisations such as writes to work RAM bypassing `call_write_byte`.
    /// This function takes a closure, `f`, in which the caller is expected to put the 8-bit value to write on the stack.
    /// # Signature
    /// ```
    /// () -> ()
    /// ```
    /// # Side Effects
    /// 1. Increments M-cycles by 1.
    #[allow(clippy::cast_possible_truncation)]
    #[allow(clippy::cast_possible_wrap)]
    fn write_byte_static<'a: 'b, 'b, F>(
        &'a mut self,
        ctx: &'b mut CodegenCtx,
        addr: u16,
        f: F,
    ) -> &'a mut Self
    where
        F: FnOnce(&'a mut Self, &'b mut CodegenCtx) -> (&'a mut Self, &'b mut CodegenCtx),
    {
        if (0xC000..0xE000).contains(&addr) {
            self.i32_const(ctx.work_ram_ptr as i32);

            let (sink, ctx) = f(self, ctx);
            sink.i32_store8(MemArg {
                offset: u64::from(addr),
                align: 0,
                memory_index: 0,
            });
            ctx.increment_m_cycles(1);
            sink
        } else {
            let (sink, ctx) = f(self, ctx);
            sink.i32_const(i32::from(addr)).call_write_byte(ctx)
        }
    }

    /// Read a byte from the specified address in the Game Boy's memory.
    /// # Signature
    /// ```
    /// (addr: i32) -> (value: i32)
    /// ```
    /// # Side Effects
    /// 1. Increments M-cycles by 1.
    #[allow(clippy::cast_possible_truncation)]
    #[allow(clippy::cast_possible_wrap)]
    fn call_read_byte(&mut self, ctx: &mut CodegenCtx) -> &mut Self {
        self.local_tee(RW_ADDR_REG)
            .local_get(RW_ADDR_REG)
            .i32_const(0xE000)
            .i32_and()
            .i32_const(0xC000)
            .i32_eq()
            .if_(BlockType::FunctionType(4));
        {
            // If addr points to work RAM (0xC000..0xE000), there are no side effects so inline the memory read.
            self.i32_load8_u(MemArg {
                offset: ctx.work_ram_ptr as u64,
                align: 0,
                memory_index: 0,
            });
        }

        self.else_();
        {
            self.local_get(RW_ADDR_REG)
                .i32_const(i32::from(ROM_BANK_0_END))
                .i32_lt_u()
                .if_(BlockType::FunctionType(4));
            {
                // Or, if addr points to ROM bank 0 (..0x4000), inline that memory read.
                self.i32_load8_u(MemArg {
                    offset: ctx.rom_ptr as u64,
                    align: 0,
                    memory_index: 0,
                });
            }

            self.else_();
            {
                // Otherwise, fall back to invoking the read_byte function.
                self.i32_const(i32::from(ctx.total_m_cycles))
                    .i32_const(ctx.runtime_ptr as i32)
                    .call(0)
                    .end();
            }
            self.end();
        }
        self.end();

        ctx.increment_m_cycles(1);
        self
    }

    /// Write a byte to the specified address in the Game Boy's memory.
    /// # Signature
    /// ```
    /// (value: i32, addr: i32) -> ()
    /// ```
    /// # Side Effects
    /// 1. Increments M-cycles by 1.
    #[allow(clippy::cast_possible_truncation)]
    #[allow(clippy::cast_possible_wrap)]
    fn call_write_byte(&mut self, ctx: &mut CodegenCtx) -> &mut Self {
        self.local_set(RW_ADDR_REG)
            .local_set(WRITE_VAL_REG)
            .local_get(RW_ADDR_REG)
            .i32_const(0xE000)
            .i32_and()
            .i32_const(0xC000)
            .i32_eq();

        // If addr points to work RAM (0xC000..0xE000), there are no side effects so inline the memory store.
        self.if_(BlockType::Empty)
            .local_get(RW_ADDR_REG)
            .local_get(WRITE_VAL_REG)
            .i32_store8(MemArg {
                offset: ctx.work_ram_ptr as u64,
                align: 0,
                memory_index: 0,
            });

        // Otherwise, fall back to invoking the write_byte function.
        self.else_()
            .local_get(WRITE_VAL_REG)
            .local_get(RW_ADDR_REG)
            .i32_const(i32::from(ctx.total_m_cycles))
            .i32_const(ctx.runtime_ptr as i32)
            .call(1)
            .end();

        ctx.increment_m_cycles(1);
        self
    }
}
