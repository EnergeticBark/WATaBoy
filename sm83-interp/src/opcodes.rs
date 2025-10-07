use arbitrary_int::{u2, u3};
use bitmatch::bitmatch;

use crate::parameters::*;

#[bitmatch]
fn decode(first_byte: u8) -> Result<Opcode, String> {
    let invalid_instruction_error = Err(String::from("Invalid instruction"));

    use Opcode::*;
    let op = #[bitmatch]
    match first_byte {
        // Block 0
        "0000_0000" => Nop,

        "00xx_0001" => LdRrNn { x: R16::from_bits(u2::new(x)) },
        "00xx_0010" => LdMemA { x: R16Mem::from_bits(u2::new(x)) },
        "00xx_1010" => LdAMem { x: R16Mem::from_bits(u2::new(x)) },
        "0000_1000" => LdNnSp,

        "00xx_0011" => IncRr { x: R16::from_bits(u2::new(x)) },
        "00xx_1011" => DecRr { x: R16::from_bits(u2::new(x)) },
        "00xx_1001" => AddHlRr { x: R16::from_bits(u2::new(x)) },

        "00xx_x100" => IncR { x: R8::from_bits(u3::new(x)) },
        "00xx_x101" => DecR { x: R8::from_bits(u3::new(x)) },

        "00xx_x110" => LdRN { x: R8::from_bits(u3::new(x)) },

        "0000_0111" => Rlca,
        "0000_1111" => Rrca,
        "0001_0111" => Rla,
        "0001_1111" => Rra,
        "0010_0111" => Daa,
        "0010_1111" => Cpl,
        "0011_0111" => Scf,
        "0011_1111" => Ccf,

        "0001_1000" => JrE,
        "001x_x000" => JrCcE { c: Condition::from_bits(u2::new(x)) },

        "0001_0000" => Stop,

        // Block 1
        "0111_0110" => Halt,
        "01xx_xyyy" => LdRR {
            x: R8::from_bits(u3::new(x)),
            y: R8::from_bits(u3::new(y)),
        },

        _ => invalid_instruction_error?,
    };

    Ok(op)
}

/* Instruction info and mnemonics sourced from: https://gekkio.fi/files/gb-docs/gbctr.pdf

  Opcodes that work with literal values from a second byte (PC + 1) are denoted with 'N'.
  It's up to the emulator to get that literal value from memory, not the opcode parser.
  Opcodes that use 16-bit literals/addresses will be denoted 'Nn'.
*/
#[derive(Debug, PartialEq, Eq)]
enum Opcode {
    // 8-bit load instructions.
    // Ld
    LdRR { x: R8, y: R8 }, // LD r, r'
    LdRN { x: R8 },        // LD r, n
    LdMemA { x: R16Mem },  // LD (BC|DE|HL+|HL-), A
    LdAMem { x: R16Mem },  // LD A, (BC|DE|HL+|HL-)
    LdANn,                 // LD A, (nn)
    LdNnA,                 // LD (nn), A
    // Ldh
    LdhAC, // LDH A, (C)
    LdhCA, // LDH (C), A
    LdhAN, // LDH A, (n)
    LdhNA, // LDH (n), A

    // 16-bit load instructions.
    LdRrNn { x: R16 }, // LD rr, nn
    LdNnSp,            // LD (nn), SP
    LdSpHl,            // LD SP, HL
    PushRr { x: u2 },  // PUSH rr
    PopRr { x: u2 },   // POP rr
    LdHlSpPlusE,       // LD HL, SP+e

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
    IncR { x: R8 }, // INC r
    IncHl,          // INC (HL)
    // Decrement
    DecR { x: R8 }, // DEC r
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
    IncRr { x: R16 },   // INC rr
    DecRr { x: R16 },   // DEC rr
    AddHlRr { x: R16 }, // ADD HL, rr
    AddSpE,             // ADD SP, e

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
    BitBR { b: u3, x: u3 }, // BIT b, r
    BitBHl { b: u3 },       // BIT b, (HL)
    ResBR { b: u3, x: u3 }, // RES b, r
    ResBHl { b: u3 },       // RES b, (HL)
    SetBR { b: u3, r: u3 }, // SET b, r
    SetBHl { b: u3 },       // SET b, (HL)

    // Control flow instructions
    JpNn,                   // JP nn
    JpHl,                   // JP HL
    JpCcNn { c: u2 },       // JP cc, nn
    JrE,                    // JR e
    JrCcE { c: Condition }, // JR cc, e
    CallNn,                 // CALL nn
    CallCcNn { c: u2 },     // CALL cc, nn
    Ret,                    // RET
    RetCc,                  // RET cc
    Reti,                   // RETI
    RstN { x: u3 },         // RST n

    // Miscellaneous instructions
    Halt, // HALT
    Stop, // STOP
    Di,   // DI
    Ei,   // EI
    Nop,  // NOP
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_load_b_indirect_hl() {
        let bytecode = 0b0100_0110;

        let opcode = decode(bytecode).unwrap();
        assert_eq!(opcode, Opcode::LdRR { x: R8::B, y: R8::IndirectHL });
    }

    #[test]
    fn decode_halt() {
        let bytecode = 0b0111_0110;

        let opcode = decode(bytecode).unwrap();
        assert_eq!(opcode, Opcode::Halt);
    }
}
