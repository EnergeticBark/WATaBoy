#[cfg(feature = "mbc-logging")]
use log::info;
use rkyv::{Archive, Deserialize, Serialize};

use hw_constants::{ROM_BANK_0_END, SRAM_END, SRAM_START};

const SRAM_ADDR_MASK: u16 = 0x01FF;

#[derive(Archive, Deserialize, Serialize)]
pub(crate) struct Mbc2 {
    pub rom: Vec<u8>,
    pub sram: Vec<u8>,
    pub current_rom_bank: u8,
    current_rom_bank_start: usize,
    sram_enabled: bool,
}

impl Mbc2 {
    pub fn from_rom(rom: &[u8]) -> Self {
        Self {
            rom: rom.to_vec(),
            ..Default::default()
        }
    }

    fn update_rom_bank(&mut self, bank_number: u8) {
        let bank_number = bank_number.max(1);
        #[cfg(feature = "mbc-logging")]
        info!(target: "mbc_events", "Switching to ROM bank #{bank_number}");

        self.current_rom_bank = bank_number;
        self.current_rom_bank_start = 0x4000 * (bank_number as usize - 1);
    }

    pub fn read_byte(&self, index: u16) -> u8 {
        match index {
            ..ROM_BANK_0_END => unsafe { *self.rom.get_unchecked(index as usize) },
            SRAM_START..SRAM_END => {
                // Only allow reads if SRAM has been enabled.
                if self.sram_enabled {
                    let sram_index = index & SRAM_ADDR_MASK;

                    // MBC2 SRAM is only made up of half bytes, so pull the upper 4 bits high.
                    self.sram[sram_index as usize] | 0xF0
                } else {
                    0xFF
                }
            }
            _ => {
                let translated = self.current_rom_bank_start + index as usize;
                unsafe { *self.rom.get_unchecked(translated) }
            }
        }
    }

    pub fn write_byte(&mut self, index: u16, value: u8) {
        match index {
            // MBC2: SRAM enable / ROM bank number.
            0x0000..0x4000 => {
                let lower_val_nibble = value & 0x0F;

                // If the 8th bit of index is 0, the value controls whether SRAM is enabled.
                // See: https://gbdev.io/pandocs/MBC2.html#when-bit-8-is-clear
                if index & (1 << 8) == 0 {
                    self.sram_enabled = lower_val_nibble == 0xA;

                    #[cfg(feature = "mbc-logging")]
                    if self.sram_enabled {
                        info!(target: "mbc_events", "Enabling SRAM...");
                    } else {
                        info!(target: "mbc_events", "Disabling SRAM...");
                    }

                    return;
                }

                // Otherwise, if the 8th bit is 1, the value controls the ROM bank.
                self.update_rom_bank(lower_val_nibble);
            }

            0x4000..0x8000 => (),

            // MBC2: SRAM
            SRAM_START..SRAM_END => {
                // Only allow writes if SRAM has been enabled.
                if self.sram_enabled {
                    let sram_index = index & SRAM_ADDR_MASK;
                    self.sram[sram_index as usize] = value;
                }
            }
            _ => unreachable!(),
        }
    }
}

impl Default for Mbc2 {
    fn default() -> Self {
        Self {
            rom: Vec::new(),
            sram: vec![0; 512],
            current_rom_bank: 1,
            current_rom_bank_start: 0,
            sram_enabled: false,
        }
    }
}
