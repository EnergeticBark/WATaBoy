use crate::opcodes::Opcode;

// Loading from IndirectHL takes extra machine cycles.
// See: https://gekkio.fi/files/gb-docs/gbctr.pdf

#[allow(clippy::match_same_arms)]
pub fn m_cycles(opcode: Opcode) -> u16 {
    match opcode {
        // 8-bit load instructions.
        // Ld
        Opcode::LdRR { .. } => 1,
        Opcode::LdRN { .. } => 0, // CONTEXT DEPENDANT: May read from timer.
        Opcode::LdAMem { .. } | Opcode::LdMemA { .. } => 2,
        Opcode::LdANn | Opcode::LdNnA => 0, // CONTEXT DEPENDANT: May read from timer.
        // Ldh
        Opcode::LdhAC => 0, // CONTEXT DEPENDANT: May read from timer.
        Opcode::LdhCA => 2,
        Opcode::LdhAN | Opcode::LdhNA => 0, // CONTEXT DEPENDANT: May read from timer.

        // 16-bit load instructions.
        Opcode::LdRrNn { .. } => 3,
        Opcode::LdNnSp => 5,
        Opcode::LdSpHl => 2,
        Opcode::PushRr { .. } => 4,
        Opcode::PopRr { .. } => 3,
        Opcode::LdHlSpPlusE => 3,

        // 8-bit arithmetic and logical instructions.
        Opcode::AddR { .. } => 1,
        Opcode::AddN => 2,
        Opcode::AdcR { .. } => 1,
        Opcode::AdcN => 2,
        Opcode::SubR { .. } => 1,
        Opcode::SubN => 2,
        Opcode::SbcR { .. } => 1,
        Opcode::SbcN => 2,
        Opcode::CpR { .. } => 1,
        Opcode::CpN => 2,
        Opcode::IncR { .. } => 1,
        Opcode::DecR { .. } => 1,
        Opcode::AndR { .. } => 1,
        Opcode::AndN => 2,
        Opcode::OrR { .. } => 1,
        Opcode::OrN => 2,
        Opcode::XorR { .. } => 1,
        Opcode::XorN => 2,
        Opcode::Ccf | Opcode::Scf | Opcode::Daa | Opcode::Cpl => 1,

        // 16-bit arithmetic instructions.
        Opcode::IncRr { .. } | Opcode::DecRr { .. } => 2,
        Opcode::AddHlRr { .. } => 2,
        Opcode::AddSpE => 4,

        // Rotate instructions.
        Opcode::Rlca | Opcode::Rrca | Opcode::Rla | Opcode::Rra => 1,

        // Control flow instructions
        Opcode::JpNn => 4,
        Opcode::JpHl => 1,
        Opcode::JpCcNn { .. } => 0, // CONTEXT DEPENDANT: 4 if condition is true, otherwise 3.
        Opcode::JrE => 3,
        Opcode::JrCcE { .. } => 0, // CONTEXT DEPENDANT: 3 if condition is true, otherwise 2.
        Opcode::CallNn => 6,
        Opcode::CallCcNn { .. } => 0, // CONTEXT DEPENDANT: 6 if condition is true, otherwise 3.
        Opcode::Ret => 4,
        Opcode::RetCc { .. } => 0, // CONTEXT DEPENDANT: 5 if condition is true, otherwise 2.
        Opcode::Reti => 4,
        Opcode::RstN { .. } => 4,

        // Miscellaneous instructions
        // I'm not 100% sure about HALT, but it's listed as 1 MCycle on the gbdev.io optable.
        // See: https://gbdev.io/gb-opcodes/optables/
        Opcode::Halt => 1,
        Opcode::Stop => 0, // TODO: Unknown
        Opcode::Di => 1,
        Opcode::Ei => 1,
        Opcode::Nop => 1,
        Opcode::Prefix => 0,
    }
}
