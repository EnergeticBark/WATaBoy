use crate::opcodes::{Opcode, PrefixOpcode};
use crate::parameters::R8;

// Loading from IndirectHL takes extra machine cycles.
// See: https://gekkio.fi/files/gb-docs/gbctr.pdf

pub fn m_cycles(opcode: Opcode) -> u16 {
    use Opcode::*;

    match opcode {
        // 8-bit load instructions.
        // Ld
        LdRR { x: R8::IndirectHL, .. } | LdRR { y: R8::IndirectHL, .. }  => 2,
        LdRR { .. } => 1,
        LdRN { x: R8::IndirectHL } => 3,
        LdRN { .. } => 2,
        LdAMem { .. } | LdMemA { .. } => 2,
        LdANn | LdNnA => 4,
        // Ldh
        LdhAC | LdhCA => 2,
        LdhAN | LdhNA => 3,

        // 16-bit load instructions.
        LdRrNn { .. } => 3,
        LdNnSp => 5,
        LdSpHl => 2,
        PushRr { .. } => 4,
        PopRr { .. } => 3,
        LdHlSpPlusE => 3,

        // 8-bit arithmetic and logical instructions.
        AddR { x: R8::IndirectHL } => 2,
        AddR { .. } => 1,
        AddN => 2,
        AdcR { x: R8::IndirectHL } => 2,
        AdcR { .. } => 1,
        AdcN => 2,
        SubR { x: R8::IndirectHL } => 2,
        SubR { .. } => 1,
        SubN => 2,
        SbcR { x: R8::IndirectHL } => 2,
        SbcR { .. } => 1,
        SbcN => 2,
        CpR { x: R8::IndirectHL } => 2,
        CpR { .. } => 1,
        CpN => 2,
        IncR { x: R8::IndirectHL } => 3,
        IncR { .. } => 1,
        DecR { x: R8::IndirectHL } => 3,
        DecR { .. } => 1,
        AndR { x: R8::IndirectHL } => 2,
        AndR { .. } => 1,
        AndN => 2,
        OrR { x: R8::IndirectHL } => 2,
        OrR { .. } => 1,
        OrN => 2,
        XorR { x: R8::IndirectHL } => 2,
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
        JpCcNn { .. } => 0,   // CONTEXT DEPENDANT: 4 if condition is true, otherwise 3.
        JrE => 3,
        JrCcE { .. } => 0,    // CONTEXT DEPENDANT: 3 if condition is true, otherwise 2.
        CallNn => 6,
        CallCcNn { .. } => 0, // CONTEXT DEPENDANT: 6 if condition is true, otherwise 3.
        Ret => 4,
        RetCc { .. } => 0,    // CONTEXT DEPENDANT: 5 if condition is true, otherwise 2.
        Reti => 4,
        RstN { .. } => 4,

        // Miscellaneous instructions
        Halt => 0, // TODO: Unknown
        Stop => 0, // TODO: Unknown
        Di => 1,
        Ei => 1,
        Nop => 1,
        Prefix => 0,
    }
}

pub fn prefix_m_cycles(opcode: PrefixOpcode) -> u16 {
    use PrefixOpcode::*;

    match opcode {
        RlcR { x: R8::IndirectHL } => 4,
        RlcR { .. } => 2,
        RrcR { x: R8::IndirectHL } => 4,
        RrcR { .. } => 2,
        RlR { x: R8::IndirectHL } => 4,
        RlR { .. } => 2,
        RrR { x: R8::IndirectHL } => 4,
        RrR { .. } => 2,
        SlaR { x: R8::IndirectHL } => 4,
        SlaR { .. } => 2,
        SraR { x: R8::IndirectHL } => 4,
        SraR { .. } => 2,
        SwapR { x: R8::IndirectHL } => 4,
        SwapR { .. } => 2,
        SrlR { x: R8::IndirectHL } => 4,
        SrlR { .. } => 2,
        BitBR { x: R8::IndirectHL, .. } => 3,
        BitBR { .. } => 2,
        ResBR { x: R8::IndirectHL, .. } => 4,
        ResBR { .. } => 2,
        SetBR { x: R8::IndirectHL, .. } => 4, // SET b, r
        SetBR { .. } => 2,
    }
}