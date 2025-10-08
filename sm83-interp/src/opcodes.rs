use arbitrary_int::{u2, u3};
use bitmatch::bitmatch;

use crate::parameters::*;

#[bitmatch]
pub fn decode(first_byte: u8) -> Result<Opcode, String> {
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

        // Block 2
        "1000_0xxx" => AddR { x: R8::from_bits(u3::new(x)) },
        "1000_1xxx" => AdcR { x: R8::from_bits(u3::new(x)) },
        "1001_0xxx" => SubR { x: R8::from_bits(u3::new(x)) },
        "1001_1xxx" => SbcR { x: R8::from_bits(u3::new(x)) },
        "1010_0xxx" => AndR { x: R8::from_bits(u3::new(x)) },
        "1010_1xxx" => XorR { x: R8::from_bits(u3::new(x)) },
        "1011_0xxx" => OrR { x: R8::from_bits(u3::new(x)) },
        "1011_1xxx" => CpR { x: R8::from_bits(u3::new(x)) },

        // Block 3
        "1100_0110" => AddN,
        "1100_1110" => AdcN,
        "1101_0110" => SubN,
        "1101_1110" => SbcN,
        "1110_0110" => AndN,
        "1110_1110" => XorN,
        "1111_0110" => OrN,
        "1111_1110" => CpN,

        "110x_x000" => RetCc { c: Condition::from_bits(u2::new(x)) },
        "1100_1001" => Ret,
        "1101_1001" => Reti,
        "110x_x010" => JpCcNn { c: Condition::from_bits(u2::new(x))},
        "1100_0011" => JpNn,
        "1110_1001" => JpHl,
        "110x_x100" => CallCcNn { c: Condition::from_bits(u2::new(x)) },
        "1100_1101" => CallNn,
        "11xx_x111" => RstN { x: u3::new(x) },

        "11xx_0001" => PopRr { x: R16Stack::from_bits(u2::new(x)) },
        "11xx_0101" => PushRr { x: R16Stack::from_bits(u2::new(x)) },

        "1100_1011" => todo!("prefix byte!!!"),

        "1110_0010" => LdhCA,
        "1110_0000" => LdhNA,
        "1110_1010" => LdNnA,
        "1111_0010" => LdhAC,
        "1111_0000" => LdhAN,
        "1111_1010" => LdANn,

        "1110_1000" => AddSpE,
        "1111_1000" => LdHlSpPlusE,
        "1111_1001" => LdSpHl,

        "1111_0011" => Di,
        "1111_1011" => Ei,

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
pub enum Opcode {
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
    LdRrNn { x: R16 },      // LD rr, nn
    LdNnSp,                 // LD (nn), SP
    LdSpHl,                 // LD SP, HL
    PushRr { x: R16Stack }, // PUSH rr
    PopRr { x: R16Stack },  // POP rr
    LdHlSpPlusE,            // LD HL, SP+e

    // 8-bit arithmetic and logical instructions.
    AddR { x: R8 }, // ADD r
    AddN,           // ADD n
    AdcR { x: R8 }, // ADC r
    AdcN,           // ADC n
    SubR { x: R8 }, // SUB r
    SubN,           // SUB n
    SbcR { x: R8 }, // SBC r
    SbcN,           // SBC n
    CpR { x: R8 },  // CP r
    CpN,            // CP n
    IncR { x: R8 }, // INC r
    DecR { x: R8 }, // DEC r
    AndR { x: R8 }, // AND r
    AndN,           // AND n
    OrR { x: R8 },  // OR r
    OrN,            // OR n
    XorR { x: R8 }, // XOR r
    XorN,           // XOR n
    Ccf,            // CCF
    Scf,            // SCF
    Daa,            // DAA
    Cpl,            // CPL

    // 16-bit arithmetic instructions.
    IncRr { x: R16 },   // INC rr
    DecRr { x: R16 },   // DEC rr
    AddHlRr { x: R16 }, // ADD HL, rr
    AddSpE,             // ADD SP, e

    // Rotate instructions.
    Rlca, // RLCA
    Rrca, // RRCA
    Rla,  // RLA
    Rra,  // RRA

    // Control flow instructions
    JpNn,                      // JP nn
    JpHl,                      // JP HL
    JpCcNn { c: Condition },   // JP cc, nn
    JrE,                       // JR e
    JrCcE { c: Condition },    // JR cc, e
    CallNn,                    // CALL nn
    CallCcNn { c: Condition }, // CALL cc, nn
    Ret,                       // RET
    RetCc { c: Condition },    // RET cc
    Reti,                      // RETI
    RstN { x: u3 },            // RST n

    // Miscellaneous instructions
    Halt, // HALT
    Stop, // STOP
    Di,   // DI
    Ei,   // EI
    Nop,  // NOP
}

enum PrefixOpcode {
    RlcR { x: R8 },         // RLC r
    RrcR { x: R8 },         // RRC r
    RlR { x: R8 },          // RL r
    RrR { x: R8 },          // RR r
    SlaR { x: R8 },         // SLA r
    SraR { x: R8 },         // SRA r
    SwapR { x: R8 },        // SWAP r
    SrlR { x: R8 },         // SRL r
    BitBR { b: u3, x: R8 }, // BIT b, r
    ResBR { b: u3, x: R8 }, // RES b, r
    SetBR { b: u3, x: R8 }, // SET b, r
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
        /* Halt's opcode follows the same pattern as LdRR with both R8 parameters set to IndirectHL.
           Because of this, the Halt instruction must always take precedence over LdRR when parsing.
         */
        let bytecode = 0b0111_0110;

        let opcode = decode(bytecode).unwrap();
        assert_eq!(opcode, Opcode::Halt);
    }

    #[test]
    fn decode_invalid() {
        // Assert that decoding each possible invalid instruction results in an error.
        let invalid_instructions = [
            0xD3,
            0xDB,
            0xDD,
            0xE3,
            0xE4,
            0xEB,
            0xEC,
            0xED,
            0xF4,
            0xFC,
            0xFD,
        ];

        for bytecode in invalid_instructions {
            let result = decode(bytecode);
            assert!(result.is_err());
        }
    }
}
