#[cfg(feature = "mbc-logging")]
use log::info;
use rkyv::{Archive, Deserialize, Serialize};

use hw_constants::{MEM_MAP_SIZE, ROM_BANK_0_END};

const RAM_SIZE_ADDR: usize = 0x0149;

const RAM_BANK_SIZE: usize = 0x2000;

const MBC1_ROM_BANK_MASK: u8 = 0b0001_1111;
const MBC1_SRAM_BANK_MASK: u8 = 0b0000_0011;

#[derive(Archive, Deserialize, Serialize)]
pub(crate) struct Mbc1 {
    ram_enabled: bool,
    pub rom: Vec<u8>,
    ext_ram: Vec<u8>,
    pub current_rom_bank: u8,
    current_rom_bank_start: usize,
    current_ram_bank: u8,
    banking_mode: bool,
}

impl Mbc1 {
    pub fn from_rom(rom: &[u8]) -> Self {
        let ext_ram = match rom[RAM_SIZE_ADDR] {
            2 => vec![0; RAM_BANK_SIZE],
            3 => vec![0; RAM_BANK_SIZE * 4],
            _ => vec![],
        };

        Self {
            rom: rom.to_vec(),
            ext_ram,
            ..Default::default()
        }
    }

    fn ram_size(&self) -> u8 {
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

    // TODO: implement 1 MiB ROM support.
    // See: https://gbdev.io/pandocs/MBC1.html#40005fff--ram-bank-number--or--upper-bits-of-rom-bank-number-write-only
    fn nth_ram_bank(&mut self, bank_number: u8) -> &[u8; RAM_BANK_SIZE] {
        let mut bank_number = bank_number & MBC1_SRAM_BANK_MASK;
        #[cfg(feature = "mbc-logging")]
        info!(target: "mbc_events", "Switching to SRAM bank #{bank_number}");

        if self.ram_size() == 2 {
            bank_number = 0;
            #[cfg(feature = "mbc-logging")]
            info!(target: "mbc_events", "Only 1 SRAM bank, constraining to 0...");
        }

        self.current_ram_bank = bank_number;

        let start_addr = RAM_BANK_SIZE * bank_number as usize;
        let end_addr = start_addr + RAM_BANK_SIZE;
        self.ext_ram[start_addr..end_addr].try_into().unwrap()
    }

    fn write_ram_bank(&mut self, bank: &[u8; RAM_BANK_SIZE]) {
        let start_addr = RAM_BANK_SIZE * self.current_ram_bank as usize;
        let end_addr = start_addr + RAM_BANK_SIZE;
        self.ext_ram[start_addr..end_addr].clone_from_slice(bank);
    }

    fn set_banking_mode(&mut self, banking_mode: u8) {
        let banking_mode = banking_mode & 1 == 1;
        #[cfg(feature = "mbc-logging")]
        info!(target: "mbc_events", "Switching to banking mode {banking_mode}");

        self.banking_mode = banking_mode;
    }

    pub fn read_byte(&self, index: u16) -> u8 {
        let translated = match index {
            ..ROM_BANK_0_END => index as usize,
            _ => self.current_rom_bank_start + index as usize,
        };

        unsafe { *self.rom.get_unchecked(translated) }
    }

    pub fn write_byte(&mut self, memory: &mut [u8; MEM_MAP_SIZE], index: u16, value: u8) {
        match index {
            // MBC1: Ram Enable
            0x0000..0x2000 => {
                if value & 0x0F == 0xA && !self.ram_enabled {
                    let bank = self.nth_ram_bank(self.current_ram_bank);
                    memory[0xA000..0xC000].clone_from_slice(bank);

                    #[cfg(feature = "mbc-logging")]
                    info!(target: "mbc_events", "Enabling SRAM...");
                    self.ram_enabled = true;
                } else if value & 0x0F != 0xA && self.ram_enabled {
                    self.write_ram_bank(&memory[0xA000..0xC000].try_into().unwrap());
                    memory[0xA000..0xC000].fill(0xFF);

                    #[cfg(feature = "mbc-logging")]
                    info!(target: "mbc_events", "Disabling SRAM...");
                    self.ram_enabled = false;
                }
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
                    // Backup old bank... Ew, I know I can do better than this.
                    self.write_ram_bank(&memory[0xA000..0xC000].try_into().unwrap());

                    let bank = self.nth_ram_bank(value);
                    memory[0xA000..0xC000].clone_from_slice(bank);
                } else {
                    #[cfg(feature = "mbc-logging")]
                    info!(target: "mbc_events", "Actually no, we're in simple mode!");
                }
            }
            // MBC1: Banking Mode Select
            0x6000..0x8000 => self.set_banking_mode(value),

            // MBC1: SRAM
            0xA000..0xC000 => {
                // Only allow writes if the MBC RAM has been enabled.
                if self.ram_enabled {
                    memory[index as usize] = value;
                }
            }
            _ => unreachable!(),
        }
    }
}

impl Default for Mbc1 {
    fn default() -> Self {
        Self {
            ram_enabled: false,
            rom: Vec::new(),
            ext_ram: Vec::new(),
            current_rom_bank: 1,
            current_rom_bank_start: 0,
            current_ram_bank: 0,
            banking_mode: false,
        }
    }
}
