use hw_constants::io_regs;
fn nth_bit(value: u8, n: u8) -> bool {
    let mask = 0b0000_0001 << n;
    value & mask == mask
}

pub fn lcd_and_ppu_enabled(memory: &[u8]) -> bool {
    nth_bit(memory[io_regs::LCDC as usize], 7)
}

pub fn window_tile_map(memory: &[u8]) -> bool {
    nth_bit(memory[io_regs::LCDC as usize], 6)
}

pub fn window_enabled(memory: &[u8]) -> bool {
    nth_bit(memory[io_regs::LCDC as usize], 5)
}

pub fn bg_and_window_tiles(memory: &[u8]) -> bool {
    nth_bit(memory[io_regs::LCDC as usize], 4)
}

pub fn bg_tile_map(memory: &[u8]) -> bool {
    nth_bit(memory[io_regs::LCDC as usize], 3)
}

pub fn obj_size(memory: &[u8]) -> bool {
    nth_bit(memory[io_regs::LCDC as usize], 2)
}

pub fn obj_enabled(memory: &[u8]) -> bool {
    nth_bit(memory[io_regs::LCDC as usize], 1)
}

pub fn bg_and_window_enabled(memory: &[u8]) -> bool {
    nth_bit(memory[io_regs::LCDC as usize], 0)
}
