use interpreter::cpu::opcodes::parameters::{R8, R16, R16Mem, R16Stack};
use wasm_encoder::{InstructionSink, MemArg};

use crate::codegen::{
    CodegenCtx,
    registers::{A, B, C, D, E, F, H, L, SP, r8_to_reg_param},
};

pub(crate) enum FlagBit {
    Zero = 7,
    Subtraction = 6,
    HalfCarry = 5,
    Carry = 4,
}

pub(crate) trait Sm83Macros {
    fn get_r8(&mut self, ctx: &mut CodegenCtx, r8: R8) -> &mut Self;
    fn set_r8(&mut self, ctx: &mut CodegenCtx, r8: R8) -> &mut Self;
    fn get_r16(&mut self, r16: R16) -> &mut Self;
    fn set_r16(&mut self, r16: R16, temp_reg: u32) -> &mut Self;
    fn get_r16_mem(&mut self, ctx: &mut CodegenCtx, r16_mem: R16Mem, temp_reg: u32) -> &mut Self;
    fn set_r16_mem(&mut self, ctx: &mut CodegenCtx, r16: R16Mem, temp_reg: u32) -> &mut Self;
    fn get_r16_stack(&mut self, r16: R16Stack) -> &mut Self;
    fn set_r16_stack(&mut self, r16: R16Stack) -> &mut Self;
    fn pop_byte(&mut self, ctx: &mut CodegenCtx) -> &mut Self;
    fn push_byte(&mut self, ctx: &mut CodegenCtx) -> &mut Self;
    fn clear_flags(&mut self) -> &mut Self;
    // TODO: I should probably just reuse the Flag struct defined in the interpreter here.
    #[allow(clippy::fn_params_excessive_bools)]
    fn assign_flags(
        &mut self,
        zero: bool,
        subtraction: bool,
        half_carry: bool,
        carry: bool,
    ) -> &mut Self;
    #[allow(clippy::fn_params_excessive_bools)]
    fn set_flags(
        &mut self,
        zero: bool,
        subtraction: bool,
        half_carry: bool,
        carry: bool,
    ) -> &mut Self;
    fn set_flag(&mut self, flag_bit: FlagBit) -> &mut Self;
    fn check_flag(&mut self, flag_bit: FlagBit) -> &mut Self;
    fn insert_checkpoint(&mut self, ctx: &mut CodegenCtx) -> &mut Self;
    fn read_regs(&mut self, registers_ptr: usize) -> &mut Self;
    fn return_regs(&mut self, registers_ptr: usize) -> &mut Self;
    fn read_byte_static(&mut self, ctx: &mut CodegenCtx, addr: u16) -> &mut Self;
    fn call_read_byte(&mut self, ctx: &mut CodegenCtx) -> &mut Self;
    fn call_write_byte(&mut self, ctx: &mut CodegenCtx) -> &mut Self;
}

impl Sm83Macros for InstructionSink<'_> {
    /// Get the value of the specified 8-bit register.
    /// If R8 is [HL], `delta_m_cycles` will reset to 0 and `total_m_cycles` will increase by 1.
    /// # Signature
    /// ```
    /// () -> (value: i32)
    /// ```
    fn get_r8(&mut self, ctx: &mut CodegenCtx, r8: R8) -> &mut Self {
        match r8 {
            R8::IndirectHL => self.get_r16(R16::Hl).call_read_byte(ctx),
            _ => self.local_get(r8_to_reg_param(r8)),
        }
    }

    /// Set the value of the specified 8-bit register.
    /// If R8 is [HL], `delta_m_cycles` will reset to 0 and `total_m_cycles` will increase by 1.
    /// # Signature
    /// ```
    /// (value: i32) -> ()
    /// ```
    fn set_r8(&mut self, ctx: &mut CodegenCtx, r8: R8) -> &mut Self {
        match r8 {
            R8::IndirectHL => self.get_r16(R16::Hl).call_write_byte(ctx),
            _ => self.local_set(r8_to_reg_param(r8)),
        }
    }

    /// Get the value of the specified 16-bit register.
    /// # Signature
    /// ```
    /// () -> (r16: i32)
    /// ```
    fn get_r16(&mut self, r16: R16) -> &mut Self {
        let (high_reg, low_reg) = match r16 {
            R16::Bc => (R8::B, R8::C),
            R16::De => (R8::D, R8::E),
            R16::Hl => (R8::H, R8::L),
            R16::Sp => unimplemented!("SP isn't in the JIT prelude/epilogue yet."),
        };

        self.local_get(r8_to_reg_param(high_reg))
            .i32_const(8)
            .i32_shl()
            .local_get(r8_to_reg_param(low_reg))
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
    fn set_r16(&mut self, r16: R16, temp_reg: u32) -> &mut Self {
        let (high_reg, low_reg) = match r16 {
            R16::Bc => (R8::B, R8::C),
            R16::De => (R8::D, R8::E),
            R16::Hl => (R8::H, R8::L),
            R16::Sp => return self.local_set(SP),
        };

        self.local_tee(temp_reg)
            .i32_const(8)
            .i32_shr_u()
            .i32_const(0xFF)
            .i32_and()
            .local_set(r8_to_reg_param(high_reg))
            .local_get(temp_reg)
            .i32_const(0xFF)
            .i32_and()
            .local_set(r8_to_reg_param(low_reg))
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
            R16Mem::Bc => (R8::B, R8::C),
            R16Mem::De => (R8::D, R8::E),
            _ => (R8::H, R8::L),
        };

        self.local_get(r8_to_reg_param(high_reg))
            .i32_const(8)
            .i32_shl()
            .local_get(r8_to_reg_param(low_reg))
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
                    .set_r16(R16::Hl, temp_reg);
            }
            R16Mem::HlDec => {
                self.local_get(temp_reg)
                    .i32_const(1)
                    .i32_sub()
                    .set_r16(R16::Hl, temp_reg);
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
            R16Mem::Bc => (R8::B, R8::C),
            R16Mem::De => (R8::D, R8::E),
            _ => (R8::H, R8::L),
        };

        self.local_get(r8_to_reg_param(high_reg))
            .i32_const(8)
            .i32_shl()
            .local_get(r8_to_reg_param(low_reg))
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
                    .set_r16(R16::Hl, temp_reg);
            }
            R16Mem::HlDec => {
                self.local_get(temp_reg)
                    .i32_const(1)
                    .i32_sub()
                    .set_r16(R16::Hl, temp_reg);
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
    fn get_r16_stack(&mut self, r16: R16Stack) -> &mut Self {
        let (high_reg, low_reg) = match r16 {
            R16Stack::Bc => (B, C),
            R16Stack::De => (D, E),
            R16Stack::Hl => (H, L),
            R16Stack::Af => (A, F),
        };

        self.local_get(low_reg).local_get(high_reg)
    }

    /// Set the value of the specified 16-bit stack register.
    /// The parameters to this macro are in reverse order compared to values returned by `get_r16_stack`.
    /// # Signature
    /// ```
    /// (high_byte: i32, low_byte: i32) -> ()
    /// ```
    fn set_r16_stack(&mut self, r16: R16Stack) -> &mut Self {
        let (high_reg, low_reg) = match r16 {
            R16Stack::Bc => (B, C),
            R16Stack::De => (D, E),
            R16Stack::Hl => (H, L),
            // TODO: Don't set the lower nibble of F!!!
            R16Stack::Af => (A, F),
        };

        self.local_set(high_reg).local_set(low_reg)
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
        self.local_get(SP)
            .call_read_byte(ctx)
            .local_get(SP)
            .i32_const(1)
            .i32_add()
            .local_set(SP)
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
        self.local_get(SP)
            .i32_const(1)
            .i32_sub()
            .local_tee(SP)
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
    fn clear_flags(&mut self) -> &mut Self {
        self.i32_const(0x00).local_set(F)
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

        self.local_get(F)
            .i32_const(i32::from(flags))
            .i32_or()
            .local_set(F)
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

        self.i32_const(i32::from(flags)).local_set(F)
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
    fn set_flag(&mut self, flag_bit: FlagBit) -> &mut Self {
        self.i32_const(flag_bit as i32)
            .i32_shl()
            .local_get(F)
            .i32_or()
            .local_set(F)
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
    fn check_flag(&mut self, flag_bit: FlagBit) -> &mut Self {
        self.local_get(F)
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
    fn read_regs(&mut self, registers_ptr: usize) -> &mut Self {
        let register_mem_offsets = [F, A, C, B, E, D, L, H];
        for (reg, offset) in register_mem_offsets.iter().zip(0..) {
            self.i32_const(registers_ptr as i32)
                .i32_load8_u(MemArg {
                    offset,
                    align: 0,
                    memory_index: 0,
                })
                .local_set(*reg);
        }

        self.i32_const(registers_ptr as i32)
            .i32_load16_u(MemArg {
                offset: 8,
                align: 0,
                memory_index: 0,
            })
            .local_set(SP)
    }

    /// Store all of the registers to satisfy the calling convention.
    /// Usually this is the final macro in a block.
    /// # Signature
    /// ```
    /// () -> ()
    /// ```
    #[allow(clippy::cast_possible_truncation)]
    #[allow(clippy::cast_possible_wrap)]
    fn return_regs(&mut self, registers_ptr: usize) -> &mut Self {
        let register_mem_offsets = [F, A, C, B, E, D, L, H];
        for (reg, offset) in register_mem_offsets.iter().zip(0..) {
            self.i32_const(registers_ptr as i32)
                .local_get(*reg)
                .i32_store8(MemArg {
                    offset,
                    align: 0,
                    memory_index: 0,
                });
        }

        self.i32_const(registers_ptr as i32)
            .local_get(SP)
            .i32_store16(MemArg {
                offset: 8,
                align: 0,
                memory_index: 0,
            })
    }

    /// Read a byte from the Game Boy's memory using a static address known at compile time.
    /// # Signature
    /// ```
    /// () -> (value: i32)
    /// ```
    /// # Side Effects
    /// 1. Calls `call_read_byte`.
    #[allow(clippy::cast_possible_truncation)]
    #[allow(clippy::cast_possible_wrap)]
    fn read_byte_static(&mut self, ctx: &mut CodegenCtx, addr: u16) -> &mut Self {
        if (0xC000..0xE000).contains(&addr) {
            self.i32_const(ctx.work_ram_ptr as i32).i32_load8_u(MemArg {
                offset: u64::from(addr),
                align: 0,
                memory_index: 0,
            });
            ctx.increment_m_cycles(1);
        } else {
            self.i32_const(i32::from(addr)).call_read_byte(ctx);
        }
        self
    }

    /// Read a byte from the specified address in the Game Boy's memory.
    /// # Signature
    /// ```
    /// (addr: i32) -> (value: i32)
    /// ```
    /// # Side Effects
    /// 1. Resets `delta_m_cycles` to 0 because `read_byte_mem` will increment the timers before reading the byte from memory.
    /// 2. Increments M-cycles by 1.
    #[allow(clippy::cast_possible_truncation)]
    #[allow(clippy::cast_possible_wrap)]
    fn call_read_byte(&mut self, ctx: &mut CodegenCtx) -> &mut Self {
        self.i32_const(i32::from(ctx.delta_m_cycles))
            .i32_const(ctx.runtime_ptr as i32)
            .call(0);
        ctx.delta_m_cycles = 0;
        ctx.increment_m_cycles(1);
        self
    }

    /// Write a byte to the specified address in the Game Boy's memory.
    /// # Signature
    /// ```
    /// (value: i32, addr: i32) -> ()
    /// ```
    /// # Side Effects
    /// 1. Resets `delta_m_cycles` to 0 because `write_byte_mem` will increment the timers before writing the byte to memory.
    /// 2. Increments M-cycles by 1.
    #[allow(clippy::cast_possible_truncation)]
    #[allow(clippy::cast_possible_wrap)]
    fn call_write_byte(&mut self, ctx: &mut CodegenCtx) -> &mut Self {
        self.i32_const(i32::from(ctx.delta_m_cycles))
            .i32_const(ctx.runtime_ptr as i32)
            .call(1);
        ctx.delta_m_cycles = 0;
        ctx.increment_m_cycles(1);
        self
    }
}
