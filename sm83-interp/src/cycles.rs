use crate::opcodes::{Opcode, PrefixOpcode};

// Loading from IndirectHL takes extra machine cycles.
// See: https://gekkio.fi/files/gb-docs/gbctr.pdf

pub fn m_cycles(opcode: Opcode) -> u16 {
    use Opcode::*;

    match opcode {
        // 8-bit load instructions.
        // Ld
        LdRR { .. } => 1,
        LdRN { .. } => 0, // CONTEXT DEPENDANT: May read from timer.
        LdAMem { .. } | LdMemA { .. } => 2,
        LdANn | LdNnA => 0, // CONTEXT DEPENDANT: May read from timer.
        // Ldh
        LdhAC => 0, // CONTEXT DEPENDANT: May read from timer.
        LdhCA => 2,
        LdhAN | LdhNA => 0, // CONTEXT DEPENDANT: May read from timer.

        // 16-bit load instructions.
        LdRrNn { .. } => 3,
        LdNnSp => 5,
        LdSpHl => 2,
        PushRr { .. } => 4,
        PopRr { .. } => 3,
        LdHlSpPlusE => 3,

        // 8-bit arithmetic and logical instructions.
        AddR { .. } => 1,
        AddN => 2,
        AdcR { .. } => 1,
        AdcN => 2,
        SubR { .. } => 1,
        SubN => 2,
        SbcR { .. } => 1,
        SbcN => 2,
        CpR { .. } => 1,
        CpN => 2,
        IncR { .. } => 1,
        DecR { .. } => 1,
        AndR { .. } => 1,
        AndN => 2,
        OrR { .. } => 1,
        OrN => 2,
        XorR { .. } => 1,
        XorN => 2,
        Ccf | Scf | Daa | Cpl => 1,

        // 16-bit arithmetic instructions.
        IncRr { .. } | DecRr { .. } => 2,
        AddHlRr { .. } => 2,
        AddSpE => 4,

        // Rotate instructions.
        Rlca | Rrca | Rla | Rra => 1,

        // Control flow instructions
        JpNn => 4,
        JpHl => 1,
        JpCcNn { .. } => 0, // CONTEXT DEPENDANT: 4 if condition is true, otherwise 3.
        JrE => 3,
        JrCcE { .. } => 0, // CONTEXT DEPENDANT: 3 if condition is true, otherwise 2.
        CallNn => 6,
        CallCcNn { .. } => 0, // CONTEXT DEPENDANT: 6 if condition is true, otherwise 3.
        Ret => 4,
        RetCc { .. } => 0, // CONTEXT DEPENDANT: 5 if condition is true, otherwise 2.
        Reti => 4,
        RstN { .. } => 4,

        // Miscellaneous instructions
        Halt => 1, // TODO: Unknown
        Stop => 0, // TODO: Unknown
        Di => 1,
        Ei => 1,
        Nop => 1,
        Prefix => 0,
    }
}

// Get the number of MCycles it takes to execute a 0xCB prefixed instruction.
// Returned value does not include the extra 1 MCycle taken to decode the prefix itself.
pub fn prefix_m_cycles(opcode: PrefixOpcode) -> u16 {
    use PrefixOpcode::*;

    match opcode {
        RlcR { .. }
        | RrcR { .. }
        | RlR { .. }
        | RrR { .. }
        | SlaR { .. }
        | SraR { .. }
        | SwapR { .. }
        | SrlR { .. }
        | BitBR { .. }
        | ResBR { .. }
        | SetBR { .. } => 1,
    }
}
