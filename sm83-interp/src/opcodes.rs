use std::fmt;

use arbitrary_int::u3;
use bitmatch::bitmatch;
use std::error::Error;

use crate::parameters::{Condition, R8, R16, R16Mem, R16Stack};

#[derive(Debug)]
struct InvalidOpcode;

impl fmt::Display for InvalidOpcode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "invalid instruction")
    }
}

impl Error for InvalidOpcode {}

/// # Errors
///
/// Will return an error if `first_byte` is not the first byte of a valid instruction.
/// The complete list of invalid first byte values is:
/// `0xD3`, `0xDB`, `0xDD`, `0xE3`, `0xE4`, `0xEB`, `0xEC`, `0xED`, `0xF4`, `0xFC`, and `0xFD`.
#[allow(clippy::too_many_lines)]
#[allow(clippy::verbose_bit_mask)] // Clippy warns about the bitmask from expanding the bitmatch macro. :(
#[bitmatch]
pub fn decode(first_byte: u8) -> Result<Opcode, Box<dyn Error>> {
    let op = #[bitmatch]
    match first_byte {
        // Block 0
        "0000_0000" => Opcode::Nop,

        "00xx_0001" => Opcode::LdRrNn {
            x: R16::from_bits(x),
        },
        "00xx_0010" => Opcode::LdMemA {
            x: R16Mem::from_bits(x),
        },
        "00xx_1010" => Opcode::LdAMem {
            x: R16Mem::from_bits(x),
        },
        "0000_1000" => Opcode::LdNnSp,

        "00xx_0011" => Opcode::IncRr {
            x: R16::from_bits(x),
        },
        "00xx_1011" => Opcode::DecRr {
            x: R16::from_bits(x),
        },
        "00xx_1001" => Opcode::AddHlRr {
            x: R16::from_bits(x),
        },

        "00xx_x100" => Opcode::IncR {
            x: R8::from_bits(x),
        },
        "00xx_x101" => Opcode::DecR {
            x: R8::from_bits(x),
        },

        "00xx_x110" => Opcode::LdRN {
            x: R8::from_bits(x),
        },

        "0000_0111" => Opcode::Rlca,
        "0000_1111" => Opcode::Rrca,
        "0001_0111" => Opcode::Rla,
        "0001_1111" => Opcode::Rra,
        "0010_0111" => Opcode::Daa,
        "0010_1111" => Opcode::Cpl,
        "0011_0111" => Opcode::Scf,
        "0011_1111" => Opcode::Ccf,

        "0001_1000" => Opcode::JrE,
        "001x_x000" => Opcode::JrCcE {
            c: Condition::from_bits(x),
        },

        "0001_0000" => Opcode::Stop,

        // Block 1
        "0111_0110" => Opcode::Halt,
        "01xx_xyyy" => Opcode::LdRR {
            x: R8::from_bits(x),
            y: R8::from_bits(y),
        },

        // Block 2
        "1000_0xxx" => Opcode::AddR {
            x: R8::from_bits(x),
        },
        "1000_1xxx" => Opcode::AdcR {
            x: R8::from_bits(x),
        },
        "1001_0xxx" => Opcode::SubR {
            x: R8::from_bits(x),
        },
        "1001_1xxx" => Opcode::SbcR {
            x: R8::from_bits(x),
        },
        "1010_0xxx" => Opcode::AndR {
            x: R8::from_bits(x),
        },
        "1010_1xxx" => Opcode::XorR {
            x: R8::from_bits(x),
        },
        "1011_0xxx" => Opcode::OrR {
            x: R8::from_bits(x),
        },
        "1011_1xxx" => Opcode::CpR {
            x: R8::from_bits(x),
        },

        // Block 3
        "1100_0110" => Opcode::AddN,
        "1100_1110" => Opcode::AdcN,
        "1101_0110" => Opcode::SubN,
        "1101_1110" => Opcode::SbcN,
        "1110_0110" => Opcode::AndN,
        "1110_1110" => Opcode::XorN,
        "1111_0110" => Opcode::OrN,
        "1111_1110" => Opcode::CpN,

        "110x_x000" => Opcode::RetCc {
            c: Condition::from_bits(x),
        },
        "1100_1001" => Opcode::Ret,
        "1101_1001" => Opcode::Reti,
        "110x_x010" => Opcode::JpCcNn {
            c: Condition::from_bits(x),
        },
        "1100_0011" => Opcode::JpNn,
        "1110_1001" => Opcode::JpHl,
        "110x_x100" => Opcode::CallCcNn {
            c: Condition::from_bits(x),
        },
        "1100_1101" => Opcode::CallNn,
        "11xx_x111" => Opcode::RstN { x: u3::new(x) },

        "11xx_0001" => Opcode::PopRr {
            x: R16Stack::from_bits(x),
        },
        "11xx_0101" => Opcode::PushRr {
            x: R16Stack::from_bits(x),
        },

        "1100_1011" => Opcode::Prefix,

        "1110_0010" => Opcode::LdhCA,
        "1110_0000" => Opcode::LdhNA,
        "1110_1010" => Opcode::LdNnA,
        "1111_0010" => Opcode::LdhAC,
        "1111_0000" => Opcode::LdhAN,
        "1111_1010" => Opcode::LdANn,

        "1110_1000" => Opcode::AddSpE,
        "1111_1000" => Opcode::LdHlSpPlusE,
        "1111_1001" => Opcode::LdSpHl,

        "1111_0011" => Opcode::Di,
        "1111_1011" => Opcode::Ei,

        _ => Err(InvalidOpcode)?,
    };

    Ok(op)
}

/* Instruction info and mnemonics sourced from: https://gekkio.fi/files/gb-docs/gbctr.pdf

  Opcodes that work with literal values from a second byte (PC + 1) are denoted with 'N'.
  It's up to the emulator to get that literal value from memory, not the opcode parser.
  Opcodes that use 16-bit literals/addresses will be denoted 'Nn'.
*/
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Opcode {
    // 8-bit load instructions.
    // Ld
    LdRR { x: R8, y: R8 }, // LD r, r'
    LdRN { x: R8 },        // LD r, n
    LdAMem { x: R16Mem },  // LD A, (BC|DE|HL+|HL-)
    LdMemA { x: R16Mem },  // LD (BC|DE|HL+|HL-), A
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
    Halt,   // HALT
    Stop,   // STOP
    Di,     // DI
    Ei,     // EI
    Nop,    // NOP
    Prefix, // Prefix opcode
}

#[must_use]
#[bitmatch]
pub fn decode_prefix(second_byte: u8) -> PrefixOpcode {
    #[bitmatch]
    match second_byte {
        "0000_0xxx" => PrefixOpcode::RlcR {
            x: R8::from_bits(x),
        },
        "0000_1xxx" => PrefixOpcode::RrcR {
            x: R8::from_bits(x),
        },
        "0001_0xxx" => PrefixOpcode::RlR {
            x: R8::from_bits(x),
        },
        "0001_1xxx" => PrefixOpcode::RrR {
            x: R8::from_bits(x),
        },
        "0010_0xxx" => PrefixOpcode::SlaR {
            x: R8::from_bits(x),
        },
        "0010_1xxx" => PrefixOpcode::SraR {
            x: R8::from_bits(x),
        },
        "0011_0xxx" => PrefixOpcode::SwapR {
            x: R8::from_bits(x),
        },
        "0011_1xxx" => PrefixOpcode::SrlR {
            x: R8::from_bits(x),
        },

        "01bb_bxxx" => PrefixOpcode::BitBR {
            b: u3::new(b),
            x: R8::from_bits(x),
        },
        "10bb_bxxx" => PrefixOpcode::ResBR {
            b: u3::new(b),
            x: R8::from_bits(x),
        },
        "11bb_bxxx" => PrefixOpcode::SetBR {
            b: u3::new(b),
            x: R8::from_bits(x),
        },
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum PrefixOpcode {
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
        assert_eq!(
            opcode,
            Opcode::LdRR {
                x: R8::B,
                y: R8::IndirectHL
            }
        );
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
            0xD3, 0xDB, 0xDD, 0xE3, 0xE4, 0xEB, 0xEC, 0xED, 0xF4, 0xFC, 0xFD,
        ];

        for bytecode in invalid_instructions {
            let result = decode(bytecode);
            assert!(result.is_err());
        }
    }
}
