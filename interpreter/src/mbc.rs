mod mbc1;
mod mbc2;
mod mbc3;
mod mbc5;

#[cfg(feature = "mbc-logging")]
use log::info;

use hw_constants::io_regs::BANK;

use mbc1::Mbc1;
use mbc2::Mbc2;
use mbc3::Mbc3;
use mbc5::Mbc5;

const MBC_TYPE_ADDR: usize = 0x0147;
#[cfg(feature = "mbc-logging")]
const RAM_SIZE_ADDR: usize = 0x0149;

const MGB_BOOT_ROM: &[u8; 0x100] = include_bytes!("../bootix_mgb.bin");

// TODO: Add the other MBC types.
#[derive(Default)]
enum Mbc {
    #[default]
    RomOnly,
    Mbc1(Mbc1),
    Mbc2(Mbc2),
    Mbc3(Mbc3),
    Mbc5(Mbc5),
}

pub struct MbcDispatcher {
    pub rom: Vec<u8>,
    pub boot_rom_mounted: bool,
    under_boot_rom: Box<[u8; 0x100]>,
    mbc: Mbc,
}

impl MbcDispatcher {
    pub fn from_rom(rom: &[u8]) -> Self {
        // Based on this table: https://gbdev.io/pandocs/The_Cartridge_Header.html#0147--cartridge-type
        // TODO: Technically, 0x00 should probably be a RomOnly type.
        let mbc_type = rom[MBC_TYPE_ADDR];
        let mbc = match mbc_type {
            0x00..=0x03 => Mbc::Mbc1(Mbc1::from_rom(rom)),
            0x05..=0x06 => Mbc::Mbc2(Mbc2::from_rom(rom)),
            0x0F..=0x13 => Mbc::Mbc3(Mbc3::from_rom(rom)),
            0x19..=0x1E => Mbc::Mbc5(Mbc5::from_rom(rom)),
            _ => unimplemented!("Unsupported MBC type: {mbc_type}"),
        };

        #[cfg(feature = "mbc-logging")]
        {
            const ROM_SIZE_ADDR: usize = 0x0148;

            info!(target: "mbc_events", "MBC Type: {}", rom[MBC_TYPE_ADDR]);
            info!(target: "mbc_events",
                "ROM size: {}, Banks: {}",
                rom[ROM_SIZE_ADDR],
                2 << rom[ROM_SIZE_ADDR]
            );
            info!(target: "mbc_events", "SRAM size: {}", rom[RAM_SIZE_ADDR]);
        }

        let mut dispatcher = Self {
            mbc,
            ..Default::default()
        };

        // Backup the first 0x100 bytes and mount the boot ROM.
        match dispatcher.mbc {
            Mbc::Mbc1(ref mut mbc1) => {
                dispatcher
                    .under_boot_rom
                    .copy_from_slice(&mbc1.rom[..0x100]);
                mbc1.rom[..0x100].copy_from_slice(MGB_BOOT_ROM);
            }
            Mbc::Mbc2(ref mut mbc2) => {
                dispatcher
                    .under_boot_rom
                    .copy_from_slice(&mbc2.rom[..0x100]);
                mbc2.rom[..0x100].copy_from_slice(MGB_BOOT_ROM);
            }
            Mbc::Mbc3(ref mut mbc3) => {
                dispatcher
                    .under_boot_rom
                    .copy_from_slice(&mbc3.rom[..0x100]);
                mbc3.rom[..0x100].copy_from_slice(MGB_BOOT_ROM);
            }
            Mbc::Mbc5(ref mut mbc5) => {
                dispatcher
                    .under_boot_rom
                    .copy_from_slice(&mbc5.rom[..0x100]);
                mbc5.rom[..0x100].copy_from_slice(MGB_BOOT_ROM);
            }
            Mbc::RomOnly => unimplemented!(),
        }

        dispatcher
    }

    pub fn read_byte(&self, index: u16) -> u8 {
        match &self.mbc {
            Mbc::Mbc1(mbc1) => mbc1.read_byte(index),
            Mbc::Mbc2(mbc2) => mbc2.read_byte(index),
            Mbc::Mbc3(mbc3) => mbc3.read_byte(index),
            Mbc::Mbc5(mbc5) => mbc5.read_byte(index),
            Mbc::RomOnly => unreachable!(),
        }
    }

    pub fn write_byte(&mut self, index: u16, value: u8) {
        if index == BANK {
            match self.mbc {
                Mbc::Mbc1(ref mut mbc1) => {
                    mbc1.rom[..0x100].copy_from_slice(&self.under_boot_rom[..0x100]);
                    self.boot_rom_mounted = false;
                }
                Mbc::Mbc2(ref mut mbc2) => {
                    mbc2.rom[..0x100].copy_from_slice(&self.under_boot_rom[..0x100]);
                    self.boot_rom_mounted = false;
                }
                Mbc::Mbc3(ref mut mbc3) => {
                    mbc3.rom[..0x100].copy_from_slice(&self.under_boot_rom[..0x100]);
                    self.boot_rom_mounted = false;
                }
                Mbc::Mbc5(ref mut mbc5) => {
                    mbc5.rom[..0x100].copy_from_slice(&self.under_boot_rom[..0x100]);
                    self.boot_rom_mounted = false;
                }
                Mbc::RomOnly => unimplemented!(),
            }
            return;
        }

        match &mut self.mbc {
            Mbc::Mbc1(mbc1) => mbc1.write_byte(index, value),
            Mbc::Mbc2(mbc2) => mbc2.write_byte(index, value),
            Mbc::Mbc3(mbc3) => mbc3.write_byte(index, value),
            Mbc::Mbc5(mbc5) => mbc5.write_byte(index, value),
            Mbc::RomOnly => unreachable!(),
        }
    }

    pub fn rom_base_ptr(&self) -> *const u8 {
        match &self.mbc {
            Mbc::Mbc1(mbc1) => mbc1.rom.as_ptr(),
            Mbc::Mbc2(mbc2) => mbc2.rom.as_ptr(),
            Mbc::Mbc3(mbc3) => mbc3.rom.as_ptr(),
            Mbc::Mbc5(mbc5) => mbc5.rom.as_ptr(),
            Mbc::RomOnly => unimplemented!(),
        }
    }

    pub fn current_rom_bank(&self) -> u16 {
        match &self.mbc {
            Mbc::Mbc1(mbc1) => u16::from(mbc1.current_rom_bank),
            Mbc::Mbc2(mbc2) => u16::from(mbc2.current_rom_bank),
            Mbc::Mbc3(mbc3) => u16::from(mbc3.current_rom_bank),
            Mbc::Mbc5(mbc5) => mbc5.current_rom_bank,
            Mbc::RomOnly => unimplemented!(),
        }
    }
}

impl Default for MbcDispatcher {
    fn default() -> Self {
        Self {
            rom: Vec::new(),
            boot_rom_mounted: true,
            under_boot_rom: vec![0; 0x100].into_boxed_slice().try_into().unwrap(),
            mbc: Mbc::RomOnly,
        }
    }
}
