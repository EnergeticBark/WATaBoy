use arbitrary_int::{u2, u3};
use bitmatch::bitmatch;

#[bitmatch]
fn decode(first_byte: u8) -> Result<Opcode, String> {
    let invalid_instruction_error = Err(String::from("Invalid instruction"));

    use Opcode::*;
    let op = #[bitmatch]
    match first_byte {
        // 8-bit load instructions.
        /* TODO: This is overriding LdRHl. Manually order these from most to least specific
        (reverse order?). */
        "01xx_xyyy" => LdRR {
            x: u3::new(x),
            y: u3::new(y),
        },
        "00xx_x110" => LdRN { x: u3::new(x) },
        "01xx_x110" => LdRHl { x: u3::new(x) },
        "0111_0xxx" => LdHlR { x: u3::new(x) },
        "0011_0110" => LdHlN,
        "0000_1010" => LdABc,
        "0001_1010" => LdADe,
        "0000_0010" => LdBcA,
        "0001_0010" => LdDeA,
        "1111_1010" => LdANn,
        "1110_1010" => LdNnA,
        "1111_0010" => LdhAC,
        _ => invalid_instruction_error?,
    };

    Ok(op)
}

/* Instruction info and mnemonics sourced from: https://gekkio.fi/files/gb-docs/gbctr.pdf

  Opcodes that work with literal values from a second byte (PC + 1) are denoted with 'N'.
  It's up to the emulator to get that literal value from memory, not the opcode parser.
  Opcodes that use 16-bit literals/addresses will be denoted 'Nn'.
*/
#[derive(Debug, Eq, PartialEq)]
enum Opcode {
    // 8-bit load instructions.
    LdRR { x: u3, y: u3 },
    LdRN { x: u3 },
    LdRHl { x: u3 },
    LdHlR { x: u3 },
    LdHlN,
    LdABc,
    LdADe,
    LdBcA,
    LdDeA,
    LdANn,
    LdNnA,
    LdhAC,    // LDH A, (C)
    LdhCA,    // LDH (C), A
    LdhAN,    // LDH A, (n)
    LdhNA,    // LDH (n), A
    LdAHlDec, // LD A, (HL-)
    LdHlDecA, // LD (HL-), A
    LdAHlInc, // LD A, (HL+)
    LdHlIncA, // LD (HL+), A

    // 16-bit load instructions.
    LdRrNn { x: u2 }, // LD rr, nn
    LdNnSp,           // LD (nn), SP
    LdSpHl,           // LD SP, HL
    PushRr { x: u2 }, // PUSH rr
    PopRr { x: u2 },  // POP rr
    LdHlSpPlusE,      // LD HL, SP+e

    // 8-bit arithmetic and logical instructions.
    // Add
    AddR { x: u3 }, // ADD r
    AddHl,          // ADD (HL)
    AddN,           // ADD n
    AdcR { x: u3 }, // ADC r
    AdcHl,          // ADC (HL)
    AdcN,           // ADC n
    // Subtract
    SubR { x: u3 }, // SUB r
    SubHl,          // SUB (HL)
    SubN,           // SUB n
    SbcR { x: u3 }, // SBC r
    SbcHl,          // SBC (HL)
    SbcN,           // SBC n
    // Compare
    CpR { x: u3 }, // CP r
    CpHl,          // CP (HL)
    CpN,           // CP n
    // Increment
    IncR { x: u3 }, // INC r
    IncHl,          // INC (HL)
    // Decrement
    DecR { x: u3 }, // DEC r
    DecHl,          // DEC (HL)
    // And
    AndR { x: u3 }, // AND r
    AndHl,          // AND (HL)
    AndN,           // AND n
    // Or
    OrR { x: u3 }, // OR r
    OrHl,          // OR (HL)
    OrN,           // OR n
    // Xor
    XorR { x: u3 }, // XOR r
    XorHl,          // XOR (HL)
    XorN,           // XOR n
    // Flags
    Ccf, // CCF
    Scf, // SCF
    Daa, // DAA
    Cpl, // CPL

    // 16-bit arithmetic instructions.
    IncRr { x: u2 },   // INC rr
    DecRr { x: u2 },   // DEC rr
    AddHlRr { x: u2 }, // ADD HL, rr
    AddSpE,            // ADD SP, e

    // Rotate, shift, and bit-operation instructions.
    // Rotate
    Rlca,           // RLCA
    Rrca,           // RRCA
    Rla,            // RLA
    Rra,            // RRA
    RlcR { x: u3 }, // RLC r
    RlcHl,          // RLC (HL)
    RrcR { x: u3 }, // RRC r
    RrcHl,          // RRC (HL)
    RlR { x: u3 },  // RL r
    RlHl,           // RL (HL)
    RrR { x: u3 },  // RR r
    RrHl,           // RR (HL)
    // Arithmetic shift
    SlaR { x: u3 }, // SLA r
    SlaHl,          // SLA (HL)
    SraR { x: u3 }, // SRA r
    SraHl,          // SRA (HL)
    // Swap
    SwapR { x: u3 }, // SWAP r
    SwapHl,          // SWAP (HL)
    // Logical shift
    SrlR { x: u3 }, // SRL r
    SrlHl,          // SRL (HL)
    // Bit
    // TODO: the documentation for this one is fucked. Double check this later.
    BitBR { b: u3, x: u3 }, // BIT b, r
    BitBHl { b: u3 },       // BIT b, (HL)
    ResBR { b: u3, x: u3 }, // RES b, r
    ResBHl { b: u3 },       // RES b, (HL)
    SetBR { b: u3, r: u3 }, // SET b, r
    SetBHl { b: u3 },       // SET b, (HL)

    // Control flow instructions
    JpNn,               // JP nn
    JpHl,               // JP HL
    JpCcNn { c: u2 },   // JP cc, nn
    JrE,                // JR e
    JrCcE { c: u2 },    // JR cc, e
    CallNn,             // CALL nn
    CallCcNn { c: u2 }, // CALL cc, nn
    Ret,                // RET
    RetCc,              // RET cc
    Reti,               // RETI
    RstN { x: u3 },     // RST n

    // Miscellaneous instructions
    Halt, // HALT
    Stop, // STOP
    Di,   // DI
    Ei,   // EI
    Nop,  // NOP
}

#[cfg(test)]
mod tests {
    use crate::opcodes::{Opcode, decode};
    use arbitrary_int::u3;

    #[test]
    fn decode_load_a_indirect_hl() {
        let bytecode = 0b01000110;

        let opcode = decode(bytecode).unwrap();
        assert_eq!(opcode, Opcode::LdRHl { x: u3::new(0) });
    }
}
