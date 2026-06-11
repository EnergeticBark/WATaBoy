#[cfg(feature = "mbc-logging")]
use log::info;
use rkyv::{Archive, Deserialize, Serialize};

use hw_constants::{ROM_BANK_0_END, SRAM_END, SRAM_START};

const RAM_SIZE_ADDR: usize = 0x0149;

const RAM_BANK_SIZE: usize = 0x2000;

const MBC1_ROM_BANK_MASK: u8 = 0b0001_1111;
const MBC1_SRAM_BANK_MASK: u8 = 0b0000_0011;

#[derive(Archive, Deserialize, Serialize)]
pub(crate) struct Mbc1 {
    pub rom: Vec<u8>,
    pub sram: Vec<u8>,
    pub current_rom_bank: u8,
    current_rom_bank_start: usize,
    current_sram_bank: u8,
    sram_enabled: bool,
    banking_mode: bool,
}

impl Mbc1 {
    pub fn from_rom(rom: &[u8]) -> Self {
        let sram = match rom[RAM_SIZE_ADDR] {
            2 => vec![0; RAM_BANK_SIZE],
            3 => vec![0; RAM_BANK_SIZE * 4],
            _ => vec![],
        };

        Self {
            rom: rom.to_vec(),
            sram,
            ..Default::default()
        }
    }

    fn sram_size(&self) -> u8 {
        self.rom[RAM_SIZE_ADDR]
    }

    fn update_rom_bank(&mut self, bank_number: u8) {
        let mut bank_number = bank_number & MBC1_ROM_BANK_MASK;
        if bank_number == 0 {
            bank_number = 1;
        }
        #[cfg(feature = "mbc-logging")]
        info!(target: "mbc_events", "Switching to ROM bank #{bank_number}");

        self.current_rom_bank = bank_number;
        self.current_rom_bank_start = 0x4000 * (bank_number as usize - 1);
    }

    fn update_ram_bank(&mut self, bank_number: u8) {
        let mut bank_number = bank_number & MBC1_SRAM_BANK_MASK;
        #[cfg(feature = "mbc-logging")]
        info!(target: "mbc_events", "Switching to SRAM bank #{bank_number}");

        if self.sram_size() == 2 {
            bank_number = 0;
            #[cfg(feature = "mbc-logging")]
            info!(target: "mbc_events", "Only 1 SRAM bank, constraining to 0...");
        }

        self.current_sram_bank = bank_number;
    }

    // TODO: implement 1 MiB ROM support.
    // See: https://gbdev.io/pandocs/MBC1.html#40005fff--ram-bank-number--or--upper-bits-of-rom-bank-number-write-only
    fn nth_sram_bank(&self, bank_number: u8) -> &[u8; RAM_BANK_SIZE] {
        let start_addr = RAM_BANK_SIZE * bank_number as usize;
        let end_addr = start_addr + RAM_BANK_SIZE;
        self.sram[start_addr..end_addr].try_into().unwrap()
    }

    // TODO: implement 1 MiB ROM support.
    // See: https://gbdev.io/pandocs/MBC1.html#40005fff--ram-bank-number--or--upper-bits-of-rom-bank-number-write-only
    fn nth_sram_bank_mut(&mut self, bank_number: u8) -> &mut [u8; RAM_BANK_SIZE] {
        let start_addr = RAM_BANK_SIZE * bank_number as usize;
        let end_addr = start_addr + RAM_BANK_SIZE;
        (&mut self.sram[start_addr..end_addr]).try_into().unwrap()
    }

    fn set_banking_mode(&mut self, banking_mode: u8) {
        let banking_mode = banking_mode & 1 == 1;
        #[cfg(feature = "mbc-logging")]
        info!(target: "mbc_events", "Switching to banking mode {banking_mode}");

        self.banking_mode = banking_mode;
    }

    pub fn read_byte(&self, index: u16) -> u8 {
        match index {
            ..ROM_BANK_0_END => unsafe { *self.rom.get_unchecked(index as usize) },
            SRAM_START..SRAM_END => {
                // Only allow reads if SRAM has been enabled.
                if self.sram_enabled {
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
            // MBC1: Ram Enable
            0x0000..0x2000 => {
                if value & 0x0F == 0xA {
                    #[cfg(feature = "mbc-logging")]
                    info!(target: "mbc_events", "Enabling SRAM...");
                    self.sram_enabled = true;
                    return;
                }

                #[cfg(feature = "mbc-logging")]
                info!(target: "mbc_events", "Disabling SRAM...");
                self.sram_enabled = false;
            }
            // MBC1: ROM Bank Number
            0x2000..0x4000 => {
                #[cfg(feature = "mbc-logging")]
                info!(target: "mbc_events", "Switching ROM bank using value: {value}");

                self.update_rom_bank(value);
            }
            // MBC1: RAM Bank Number or Upper Bits of ROM bank number
            0x4000..0x6000 => {
                #[cfg(feature = "mbc-logging")]
                info!(target: "mbc_events", "Switching SRAM bank using value: {value}");

                if self.banking_mode {
                    self.update_ram_bank(value);
                } else {
                    #[cfg(feature = "mbc-logging")]
                    info!(target: "mbc_events", "Actually no, we're in simple mode!");
                }
            }
            // MBC1: Banking Mode Select
            0x6000..0x8000 => self.set_banking_mode(value),

            // MBC1: SRAM
            SRAM_START..SRAM_END => {
                // Only allow writes if the MBC RAM has been enabled.
                if self.sram_enabled {
                    let sram = self.nth_sram_bank_mut(self.current_sram_bank);
                    let sram_index = index as usize - 0xA000;
                    sram[sram_index] = value;
                }
            }
            _ => unreachable!(),
        }
    }
}

impl Default for Mbc1 {
    fn default() -> Self {
        Self {
            rom: Vec::new(),
            sram: Vec::new(),
            current_rom_bank: 1,
            current_rom_bank_start: 0,
            current_sram_bank: 0,
            sram_enabled: false,
            banking_mode: false,
        }
    }
}
