#[cfg(feature = "mbc-logging")]
use log::info;
use rkyv::{Archive, Deserialize, Serialize};

use hw_constants::{ROM_BANK_0_END, SRAM_END, SRAM_START};

const RAM_SIZE_ADDR: usize = 0x0149;

const RAM_BANK_SIZE: usize = 0x2000;

// For Mbc3, all 7 bits are written directly.
// See: https://gbdev.io/pandocs/MBC3.html#2000-3fff---rom-bank-number-write-only
const MBC3_ROM_BANK_MASK: u8 = 0b0111_1111;

#[derive(Archive, Deserialize, Serialize)]
pub(crate) struct Mbc3 {
    ram_enabled: bool,
    pub rom: Vec<u8>,
    sram: Vec<u8>,
    pub current_rom_bank: u8,
    current_rom_bank_start: usize,
    current_sram_bank: u8,
}

impl Mbc3 {
    pub fn from_rom(rom: &[u8]) -> Self {
        let sram = match rom[RAM_SIZE_ADDR] {
            2 => vec![0; RAM_BANK_SIZE],
            3 => vec![0; RAM_BANK_SIZE * 4],
            5 => vec![0; RAM_BANK_SIZE * 8],
            _ => vec![],
        };

        Self {
            rom: rom.to_vec(),
            sram,
            ..Default::default()
        }
    }

    fn ram_size(&self) -> u8 {
        self.rom[RAM_SIZE_ADDR]
    }

    fn update_rom_bank(&mut self, bank_number: u8) {
        let mut bank_number = bank_number & MBC3_ROM_BANK_MASK;
        if bank_number == 0 {
            bank_number = 1;
        }
        #[cfg(feature = "mbc-logging")]
        info!(target: "mbc_events", "Switching to ROM bank #{bank_number}");

        self.current_rom_bank = bank_number;
        self.current_rom_bank_start = 0x4000 * (bank_number as usize - 1);
    }

    fn update_sram_bank(&mut self, bank_number: u8) {
        let mut bank_number = bank_number;
        #[cfg(feature = "mbc-logging")]
        info!(target: "mbc_events", "Switching to SRAM bank #{bank_number}");

        if self.ram_size() == 2 {
            bank_number = 0;
            #[cfg(feature = "mbc-logging")]
            info!(target: "mbc_events", "Only 1 SRAM bank, constraining to 0...");
        }

        self.current_sram_bank = bank_number;
    }

    fn nth_sram_bank(&self, bank_number: u8) -> &[u8; RAM_BANK_SIZE] {
        let start_addr = RAM_BANK_SIZE * bank_number as usize;
        let end_addr = start_addr + RAM_BANK_SIZE;
        self.sram[start_addr..end_addr].try_into().unwrap()
    }

    fn nth_sram_bank_mut(&mut self, bank_number: u8) -> &mut [u8; RAM_BANK_SIZE] {
        let start_addr = RAM_BANK_SIZE * bank_number as usize;
        let end_addr = start_addr + RAM_BANK_SIZE;
        (&mut self.sram[start_addr..end_addr]).try_into().unwrap()
    }

    pub fn read_byte(&self, index: u16) -> u8 {
        match index {
            ..ROM_BANK_0_END => unsafe { *self.rom.get_unchecked(index as usize) },
            SRAM_START..SRAM_END => {
                // Only allow reads if SRAM has been enabled.
                if self.ram_enabled {
                    let sram = self.nth_sram_bank(self.current_sram_bank);
                    let sram_index = index as usize - 0xA000;
                    sram[sram_index]
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
            // MBC3: Ram Enable
            0x0000..0x2000 => {
                if value & 0x0F == 0xA {
                    #[cfg(feature = "mbc-logging")]
                    info!(target: "mbc_events", "Enabling SRAM...");
                    self.ram_enabled = true;
                    return;
                }

                #[cfg(feature = "mbc-logging")]
                info!(target: "mbc_events", "Disabling SRAM...");
                self.ram_enabled = false;
            }
            // MBC3: ROM Bank Number
            0x2000..0x4000 => {
                #[cfg(feature = "mbc-logging")]
                info!(target: "mbc_events", "Switching ROM bank using value: {value}");

                self.update_rom_bank(value);
            }
            // MBC3: SRAM Bank Number or RTC Register Select
            0x4000..0x6000 => {
                #[cfg(feature = "mbc-logging")]
                info!(target: "mbc_events", "Switching SRAM bank using value: {value}");

                match value {
                    0x0..0x8 => self.update_sram_bank(value),
                    0x8..0xD => unimplemented!("RTC Register Select"),
                    _ => unreachable!(),
                }
            }

            0x6000..0x8000 => unimplemented!("Latch Clock Data"),

            // MBC3: SRAM
            SRAM_START..SRAM_END => {
                // Only allow writes if the MBC RAM has been enabled.
                if self.ram_enabled {
                    let sram = self.nth_sram_bank_mut(self.current_sram_bank);
                    let sram_index = index as usize - 0xA000;
                    sram[sram_index] = value;
                }
            }
            _ => unreachable!(),
        }
    }
}

impl Default for Mbc3 {
    fn default() -> Self {
        Self {
            ram_enabled: false,
            rom: Vec::new(),
            sram: Vec::new(),
            current_rom_bank: 1,
            current_rom_bank_start: 0,
            current_sram_bank: 0,
        }
    }
}
