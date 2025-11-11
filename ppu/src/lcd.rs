use hw_constants::io_regs;

pub fn lcd_and_ppu_enabled(memory: &[u8]) -> bool {
    memory[io_regs::LCDC as usize] & 0b1000_0000 == 0b1000_0000
}

pub fn window_tile_map(memory: &[u8]) -> bool {
    memory[io_regs::LCDC as usize] & 0b0100_0000 == 0b0100_0000
}

pub fn window_enabled(memory: &[u8]) -> bool {
    memory[io_regs::LCDC as usize] & 0b0010_0000 == 0b0010_0000
}

pub fn bg_and_window_tiles(memory: &[u8]) -> bool {
    memory[io_regs::LCDC as usize] & 0b0001_0000 == 0b0001_0000
}

pub fn bg_tile_map(memory: &[u8]) -> bool {
    memory[io_regs::LCDC as usize] & 0b0000_1000 == 0b0000_1000
}

pub fn bg_and_window_enabled(memory: &[u8]) -> bool {
    memory[io_regs::LCDC as usize] & 0b0000_0001 == 0b0000_0001
}