const MBC_TYPE_ADDR: usize = 0x0147;
const ROM_SIZE_ADDR: usize = 0x0148;
const RAM_SIZE_ADDR: usize = 0x0149;

const RAM_BANK_SIZE: usize = 0x2000;


// TODO: Add the other MBC types.
#[derive(Default)]
enum MbcKind {
    #[default]
    Mbc1,
    Mbc3,
}

impl MbcKind {
    fn from_bits(value: u8) -> Self {
        // Based on this table: https://gbdev.io/pandocs/The_Cartridge_Header.html#0147--cartridge-type
        match value {
            0x01 => MbcKind::Mbc1,
            0x11..0x14 => MbcKind::Mbc3,
            _ => unimplemented!(),
        }
    }
}

#[derive(Default)]
pub struct Mbc {
    kind: MbcKind,
    pub ram_enabled: bool,
    pub rom: Vec<u8>,
    pub ext_ram: Vec<u8>,
    pub current_ram_bank: u8,
    pub banking_mode: bool,
}

impl Mbc {
    pub fn rom_size(&self) -> u8 {
        self.rom[ROM_SIZE_ADDR]
    }

    pub fn ram_size(&self) -> u8 {
        self.rom[RAM_SIZE_ADDR]
    }

    pub fn load_rom(&mut self, rom: &[u8]) {
        self.rom = rom.to_vec();
        println!("MBC Type: {}", self.rom[MBC_TYPE_ADDR]);
        self.kind = MbcKind::from_bits(self.rom[MBC_TYPE_ADDR]);
        println!(
            "ROM size: {}, Banks: {}",
            self.rom_size(),
            2 << self.rom_size()
        );
        println!("RAM size: {}", self.ram_size());

        match self.ram_size() {
            2 => self.ext_ram = vec![0; RAM_BANK_SIZE],
            3 => self.ext_ram = vec![0; RAM_BANK_SIZE * 4],
            _ => (),
        }
    }

    pub fn enable_ram(&mut self) {
        println!("Enabling ram...");
        self.ram_enabled = true;
    }

    pub fn disable_ram(&mut self) {
        println!("Disabling ram...");
        self.ram_enabled = false;
    }

    pub fn nth_rom_bank(&self, bank_number: u8) -> &[u8; 0x4000] {
        let mask = match self.kind {
            MbcKind::Mbc1 => 0b0001_1111,
            // For Mbc3, all 7 bits are written directly.
            // See: https://gbdev.io/pandocs/MBC3.html#2000-3fff---rom-bank-number-write-only
            MbcKind::Mbc3 => 0b0111_1111,
        };

        let mut bank_number = bank_number & mask;
        if bank_number == 0 {
            bank_number = 1;
        }
        println!("Switching to ROM bank #{bank_number}");

        let start_addr = 0x4000 * bank_number as usize;
        let end_addr = start_addr + 0x4000;
        self.rom[start_addr..end_addr].try_into().unwrap()
    }

    pub fn nth_ram_bank(&mut self, bank_number: u8) -> &[u8; RAM_BANK_SIZE] {
        let mut bank_number = bank_number & 0b0000_0011;
        println!("Switching to RAM bank #{bank_number}");

        if self.ram_size() == 2 {
            bank_number = 0;
            println!("Only 1 bank, constraining to 0...");
        }

        self.current_ram_bank = bank_number;

        let start_addr = RAM_BANK_SIZE * bank_number as usize;
        let end_addr = start_addr + RAM_BANK_SIZE;
        self.ext_ram[start_addr..end_addr].try_into().unwrap()
    }

    pub fn write_ram_bank(&mut self, bank: &[u8; RAM_BANK_SIZE]) {
        let start_addr = RAM_BANK_SIZE * self.current_ram_bank as usize;
        let end_addr = start_addr + RAM_BANK_SIZE;
        self.ext_ram[start_addr..end_addr].clone_from_slice(bank);
    }

    pub fn set_banking_mode(&mut self, banking_mode: u8) {
        let banking_mode = banking_mode & 1 == 1;
        println!("Switching to banking mode {banking_mode}");

        self.banking_mode = banking_mode;
    }
}
