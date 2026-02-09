use crate::{MEM_MAP_SIZE, io_regs};

pub trait PostBoot: Sized {
    fn post_boot_dmg() -> Self;
}

// Values are from the "DMG / MGB" column of Pan Docs's table on hardware registers.
// See: https://gbdev.io/pandocs/Power_Up_Sequence.html#hardware-registers
#[allow(clippy::missing_panics_doc)]
#[must_use]
pub fn post_boot_hwio() -> Box<[u8; MEM_MAP_SIZE]> {
    let mut buffer: Box<[u8; MEM_MAP_SIZE]> = vec![0; MEM_MAP_SIZE].into_boxed_slice().try_into().unwrap();
    buffer[0xA000..0xC000].fill(0xFF);
    buffer[io_regs::JOYP as usize] = 0xCF;
    buffer[io_regs::SC as usize] = 0x7E;
    buffer[0xFF03] = 0xFF;
    buffer[io_regs::TAC as usize] = 0xF8;
    buffer[0xFF08..0xFF0F].fill(0xFF);
    buffer[io_regs::IF as usize] = 0xE1;
    // NRs
    buffer[io_regs::NR10 as usize] = 0x80;
    buffer[io_regs::NR11 as usize] = 0xBF;
    buffer[io_regs::NR12 as usize] = 0xF3;
    buffer[io_regs::NR13 as usize] = 0xFF;
    buffer[io_regs::NR14 as usize] = 0xBF;
    buffer[0xFF15] = 0xFF;
    buffer[io_regs::NR21 as usize] = 0x3F;
    buffer[io_regs::NR22 as usize] = 0x00;
    buffer[io_regs::NR23 as usize] = 0xFF;
    buffer[io_regs::NR24 as usize] = 0xBF;
    buffer[io_regs::NR30 as usize] = 0x7F;
    buffer[io_regs::NR31 as usize] = 0xFF;
    buffer[io_regs::NR32 as usize] = 0x9F;
    buffer[io_regs::NR33 as usize] = 0xFF;
    buffer[io_regs::NR34 as usize] = 0xBF;
    buffer[0xFF1F] = 0xFF;
    buffer[io_regs::NR41 as usize] = 0xFF;
    buffer[io_regs::NR42 as usize] = 0x00;
    buffer[io_regs::NR43 as usize] = 0x00;
    buffer[io_regs::NR44 as usize] = 0xBF;
    buffer[io_regs::NR50 as usize] = 0x77;
    buffer[io_regs::NR51 as usize] = 0xF3;
    buffer[io_regs::NR52 as usize] = 0xF1;
    buffer[0xFF27..0xFF40].fill(0xFF);

    buffer[io_regs::LCDC as usize] = 0x91;
    buffer[io_regs::STAT as usize] = 0x85;
    buffer[io_regs::BGP as usize] = 0xFC;
    buffer[io_regs::KEY0 as usize] = 0xFF;
    buffer[io_regs::KEY1 as usize] = 0xFF;
    buffer[0xFF4E] = 0xFF;
    buffer[io_regs::VBK as usize] = 0xFF;
    buffer[io_regs::BANK as usize] = 0xFF;
    buffer[io_regs::HDMA1 as usize] = 0xFF;
    buffer[io_regs::HDMA2 as usize] = 0xFF;
    buffer[io_regs::HDMA3 as usize] = 0xFF;
    buffer[io_regs::HDMA4 as usize] = 0xFF;
    buffer[io_regs::HDMA5 as usize] = 0xFF;
    buffer[io_regs::RP as usize] = 0xFF;
    buffer[0xFF57..0xFF68].fill(0xFF);
    buffer[io_regs::BCPS as usize] = 0xFF;
    buffer[io_regs::BCPD as usize] = 0xFF;
    buffer[io_regs::OCPS as usize] = 0xFF;
    buffer[io_regs::OCPD as usize] = 0xFF;
    buffer[0xFF6C..0xFF70].fill(0xFF);
    buffer[io_regs::SVBK as usize] = 0xFF;
    buffer[0xFF71..0xFFFF].fill(0xFF);

    buffer
}
