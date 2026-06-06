mod mbc1;

#[cfg(feature = "mbc-logging")]
use log::info;
use rkyv::{Archive, Deserialize, Serialize};

use hw_constants::MEM_MAP_SIZE;
use hw_constants::io_regs::BANK;

use mbc1::Mbc1;

const MBC_TYPE_ADDR: usize = 0x0147;
const RAM_SIZE_ADDR: usize = 0x0149;

const MGB_BOOT_ROM: &[u8; 0x100] = include_bytes!("../bootix_mgb.bin");

// TODO: Add the other MBC types.
#[derive(Archive, Deserialize, Serialize, Default)]
enum Mbc {
    #[default]
    RomOnly,
    Mbc1(Mbc1),
    Mbc3,
}

#[derive(Archive, Deserialize, Serialize)]
pub struct MbcDispatcher {
    pub boot_rom_mounted: bool,
    under_boot_rom: Box<[u8; 0x100]>,
    mbc: Mbc,
}

impl MbcDispatcher {
    pub fn from_rom(rom: &[u8]) -> Self {
        // Based on this table: https://gbdev.io/pandocs/The_Cartridge_Header.html#0147--cartridge-type
        // TODO: Technically, 0x00 should probably be a RomOnly type.
        let mbc = match rom[MBC_TYPE_ADDR] {
            0x00..=0x03 => Mbc::Mbc1(Mbc1::from_rom(rom)),
            0x05..=0x06 => unimplemented!("MBC2"),
            0x11..=0x13 => unimplemented!("MBC3"),
            _ => unimplemented!(),
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
            _ => unimplemented!(),
        }

        dispatcher
    }

    pub fn read_byte(&self, index: u16) -> u8 {
        match &self.mbc {
            Mbc::Mbc1(mbc1) => mbc1.read_byte(index),
            _ => unreachable!(),
        }
    }

    pub fn write_byte(&mut self, memory: &mut [u8; MEM_MAP_SIZE], index: u16, value: u8) {
        if index == BANK {
            match self.mbc {
                Mbc::Mbc1(ref mut mbc1) => {
                    mbc1.rom[..0x100].copy_from_slice(&self.under_boot_rom[..0x100]);
                    self.boot_rom_mounted = false;
                    return;
                }
                _ => unimplemented!(),
            }
        }

        match &mut self.mbc {
            Mbc::Mbc1(mbc1) => mbc1.write_byte(memory, index, value),
            _ => unreachable!(),
        }
    }
}

impl Default for MbcDispatcher {
    fn default() -> Self {
        Self {
            boot_rom_mounted: true,
            under_boot_rom: vec![0; 0x100].into_boxed_slice().try_into().unwrap(),
            mbc: Mbc::RomOnly,
        }
    }
}
