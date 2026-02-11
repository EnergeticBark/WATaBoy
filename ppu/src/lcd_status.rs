use hw_constants::io_regs;

pub fn lyc_int_select(memory: &[u8]) -> bool {
    memory[io_regs::STAT as usize] & 0b0100_0000 == 0b0100_0000
}

pub fn mode2_int_select(memory: &[u8]) -> bool {
    memory[io_regs::STAT as usize] & 0b0010_0000 == 0b0010_0000
}

pub fn mode1_int_select(memory: &[u8]) -> bool {
    memory[io_regs::STAT as usize] & 0b0001_0000 == 0b0001_0000
}

pub fn mode0_int_select(memory: &[u8]) -> bool {
    memory[io_regs::STAT as usize] & 0b0000_1000 == 0b0000_1000
}

// Set the LYC == LY bit in the STATUS register.
pub fn set_coincidence(memory: &mut [u8], value: bool) {
    const COINCIDENCE_BIT: u8 = 0b0000_0100;
    if value {
        memory[io_regs::STAT as usize] |= COINCIDENCE_BIT;
    } else {
        memory[io_regs::STAT as usize] &= !COINCIDENCE_BIT;
    }
}

// Set the PPU mode in the STATUS register.
// Values: 0 - HBlank or LCD off, 1 - VBlank, 2 - OAM Scan, 3 - Drawing.
// This should really only take a 2-bit value, but what can you do.
pub fn set_ppu_mode(memory: &mut [u8], value: u8) {
    const MASK: u8 = 0b0000_0011;
    let value = value & MASK;
    memory[io_regs::STAT as usize] &= !MASK;
    memory[io_regs::STAT as usize] |= value;
}

pub fn ppu_mode(memory: &[u8]) -> u8 {
    memory[io_regs::STAT as usize] & 0b0000_0011
}
