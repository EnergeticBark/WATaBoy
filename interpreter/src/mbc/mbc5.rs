#[cfg(feature = "mbc-logging")]
use log::info;
use rkyv::{Archive, Deserialize, Serialize};

use hw_constants::{ROM_BANK_0_END, SRAM_END, SRAM_START};

const SRAM_SIZE_ADDR: usize = 0x0149;

const ROM_BANK_SIZE: usize = 0x4000;
const SRAM_BANK_SIZE: usize = 0x2000;

const MBC5_SRAM_BANK_MASK: u8 = 0b0000_1111;

#[derive(Archive, Deserialize, Serialize)]
pub(crate) struct Mbc5 {
    ram_enabled: bool,
    pub rom: Vec<u8>,
    sram: Vec<u8>,
    pub current_rom_bank: u16,
    current_rom_bank_start: usize,
    current_ram_bank: u8,
}

impl Mbc5 {
    pub fn from_rom(rom: &[u8]) -> Self {
        let sram = match rom[SRAM_SIZE_ADDR] {
            2 => vec![0; SRAM_BANK_SIZE],
            3 => vec![0; SRAM_BANK_SIZE * 4],
            _ => vec![],
        };

        Self {
            rom: rom.to_vec(),
            sram,
            ..Default::default()
        }
    }

    fn ram_size(&self) -> u8 {
        self.rom[SRAM_SIZE_ADDR]
    }

    fn update_rom_bank(&mut self, mut bank_number: u16) {
        // ROM bank 0 is allowed to be mapped.

        // Constrain the bank number to the number of rom banks.
        let num_rom_banks = u16::try_from(self.rom.len() / ROM_BANK_SIZE).unwrap();
        bank_number %= num_rom_banks;

        println!("Switching to ROM bank #{bank_number}");

        #[cfg(feature = "mbc-logging")]
        info!(target: "mbc_events", "Switching to ROM bank #{bank_number}");

        self.current_rom_bank = bank_number;
        self.current_rom_bank_start = 0x4000 * bank_number as usize;
    }

    fn update_ram_bank(&mut self, bank_number: u8) {
        let mut bank_number = bank_number & MBC5_SRAM_BANK_MASK;
        #[cfg(feature = "mbc-logging")]
        info!(target: "mbc_events", "Switching to SRAM bank #{bank_number}");

        if self.ram_size() == 2 {
            bank_number = 0;
            #[cfg(feature = "mbc-logging")]
            info!(target: "mbc_events", "Only 1 SRAM bank, constraining to 0...");
        }

        self.current_ram_bank = bank_number;
    }

    fn nth_ram_bank(&self, bank_number: u8) -> &[u8; SRAM_BANK_SIZE] {
        let start_addr = SRAM_BANK_SIZE * bank_number as usize;
        let end_addr = start_addr + SRAM_BANK_SIZE;
        self.sram[start_addr..end_addr].try_into().unwrap()
    }

    fn nth_ram_bank_mut(&mut self, bank_number: u8) -> &mut [u8; SRAM_BANK_SIZE] {
        let start_addr = SRAM_BANK_SIZE * bank_number as usize;
        let end_addr = start_addr + SRAM_BANK_SIZE;
        (&mut self.sram[start_addr..end_addr]).try_into().unwrap()
    }

    pub fn read_byte(&self, index: u16) -> u8 {
        match index {
            ..ROM_BANK_0_END => unsafe { *self.rom.get_unchecked(index as usize) },
            SRAM_START..SRAM_END => {
                // Only allow reads if SRAM has been enabled.
                if self.ram_enabled {
                    let sram = self.nth_ram_bank(self.current_ram_bank);
                    let sram_index = index as usize - 0xA000;
                    sram[sram_index]
                } else {
                    0xFF
                }
            }
            _ => {
                let translated = self.current_rom_bank_start + (index - 0x4000) as usize;
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
                    self.ram_enabled = true;
                    return;
                }

                #[cfg(feature = "mbc-logging")]
                info!(target: "mbc_events", "Disabling SRAM...");
                self.ram_enabled = false;
            }
            // MBC5: Lower 8 Bits of ROM Bank Number
            0x2000..0x3000 => {
                let prev_8th_bit = self.current_rom_bank & (1 << 8);
                let bank_number = prev_8th_bit | u16::from(value);

                #[cfg(feature = "mbc-logging")]
                info!(target: "mbc_events", "Switching ROM bank using value: {bank_number}");

                self.update_rom_bank(bank_number);
            }
            // MBC5: 8th Bit of ROM Bank Number
            0x3000..0x4000 => {
                let prev_lo_byte = self.current_rom_bank & 0xFF;
                let bank_number = prev_lo_byte | ((u16::from(value) & 1) << 8);

                #[cfg(feature = "mbc-logging")]
                info!(target: "mbc_events", "Switching ROM bank using value: {bank_number}");

                self.update_rom_bank(bank_number);
            }
            // MBC1: RAM Bank Number or Upper Bits of ROM bank number
            0x4000..0x6000 => {
                #[cfg(feature = "mbc-logging")]
                info!(target: "mbc_events", "Switching SRAM bank using value: {value}");

                self.update_ram_bank(value);
            }
            0x6000..0x8000 => (),

            // MBC1: SRAM
            SRAM_START..SRAM_END => {
                // Only allow writes if the MBC RAM has been enabled.
                if self.ram_enabled {
                    let sram = self.nth_ram_bank_mut(self.current_ram_bank);
                    let sram_index = index as usize - 0xA000;
                    sram[sram_index] = value;
                }
            }
            _ => unreachable!(),
        }
    }
}

impl Default for Mbc5 {
    fn default() -> Self {
        Self {
            ram_enabled: false,
            rom: Vec::new(),
            sram: Vec::new(),
            current_rom_bank: 1,
            current_rom_bank_start: 0x4000,
            current_ram_bank: 0,
        }
    }
}
