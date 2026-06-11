use std::time::{Duration, SystemTime};

use bitfield_struct::bitfield;
#[cfg(feature = "mbc-logging")]
use log::info;
use rkyv::{Archive, Deserialize, Serialize};

use hw_constants::{ROM_BANK_0_END, SRAM_END, SRAM_START};

const RAM_SIZE_ADDR: usize = 0x0149;

const RAM_BANK_SIZE: usize = 0x2000;

// For Mbc3, all 7 bits are written directly.
// See: https://gbdev.io/pandocs/MBC3.html#2000-3fff---rom-bank-number-write-only
const MBC3_ROM_BANK_MASK: u8 = 0b0111_1111;

// Structs to support the RTC.
#[bitfield(u8, order = Msb)]
#[derive(Archive, Deserialize, Serialize)]
struct DaysHi {
    days_carry: bool,
    halt: bool,
    #[bits(5)]
    __: u8,
    day_msb: bool,
}

// The day counter is 9 bits, with the lower 8 in `days_lo` and the most significant in `days_hi`.
#[derive(Default, Archive, Deserialize, Serialize)]
struct RtcRegs {
    seconds: u8,
    minutes: u8,
    hours: u8,
    days_lo: u8,
    days_hi: DaysHi,
}

impl RtcRegs {
    fn update(&mut self, secs: u64) {
        let minutes = secs / 60;
        let hours = minutes / 60;
        let days = hours / 24;

        self.seconds = (secs % 60) as u8;
        self.minutes = (minutes % 60) as u8;
        self.hours = (hours % 24) as u8;
        self.days_lo = (days & 0xFF) as u8;

        // Set `day_msb` to the 8th bit of `days`.
        self.days_hi.set_day_msb(days & (1 << 8) != 0);
    }
}

struct Rtc {
    duration: Duration,
    last_ticked: SystemTime,
    regs: RtcRegs,
}

impl Default for Rtc {
    fn default() -> Self {
        Self {
            duration: Duration::default(),
            last_ticked: SystemTime::now(),
            regs: RtcRegs::default(),
        }
    }
}

impl Rtc {
    fn update(&mut self) {
        let new_time = SystemTime::now();
        let delta = new_time
            .duration_since(self.last_ticked)
            .expect("Clock went backwards...");
        self.duration += delta;

        // Update the register values if the RTC isn't halted.
        if !self.regs.days_hi.halt() {
            self.regs.update(self.duration.as_secs());
        }

        self.last_ticked = new_time;
    }

    fn read(&self, selected: u8) -> u8 {
        match selected {
            0x08 => self.regs.seconds,
            0x09 => self.regs.minutes,
            0x0A => self.regs.hours,
            0x0B => self.regs.days_lo,
            0x0C => self.regs.days_hi.into_bits(),
            _ => unreachable!(),
        }
    }

    fn write(&mut self, selected: u8, value: u8) {
        match selected {
            0x08 => self.regs.seconds = value,
            0x09 => self.regs.minutes = value,
            0x0A => self.regs.hours = value,
            0x0B => self.regs.days_lo = value,
            0x0C => {
                let new_days_hi = DaysHi::from_bits(value);
                if self.regs.days_hi.halt() && !new_days_hi.halt() {
                    self.last_ticked = SystemTime::now();
                }

                self.regs.days_hi = new_days_hi;
            }
            _ => unreachable!(),
        }
    }
}

pub(crate) struct Mbc3 {
    ram_and_rtc_enabled: bool,
    pub rom: Vec<u8>,
    pub sram: Vec<u8>,
    rtc: Rtc,
    rtc_latch: bool,
    pub current_rom_bank: u8,
    current_rom_bank_start: usize,
    sram_bank_or_rtc_reg: u8,
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

    fn sram_size(&self) -> u8 {
        self.rom[RAM_SIZE_ADDR]
    }

    fn update_rom_bank(&mut self, bank_number: u8) {
        #[cfg(feature = "mbc-logging")]
        info!(target: "mbc_events", "Switching ROM bank using value: {bank_number}");

        let mut bank_number = bank_number & MBC3_ROM_BANK_MASK;
        if bank_number == 0 {
            bank_number = 1;
        }
        #[cfg(feature = "mbc-logging")]
        info!(target: "mbc_events", "Switching to ROM bank #{bank_number}");

        self.current_rom_bank = bank_number;
        self.current_rom_bank_start = 0x4000 * (bank_number as usize - 1);
    }

    fn update_sram_bank_or_rtc_reg(&mut self, mut value: u8) {
        #[cfg(feature = "mbc-logging")]
        info!(target: "mbc_events", "Switching to SRAM bank or RTC reg using value #{value}");

        if value < 0x8 && self.sram_size() == 2 {
            value = 0;
            #[cfg(feature = "mbc-logging")]
            info!(target: "mbc_events", "Only 1 SRAM bank, constraining to 0...");
        }

        self.sram_bank_or_rtc_reg = value;
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
                // Only allow reads if SRAM or RTC has been enabled.
                if self.ram_and_rtc_enabled {
                    match self.sram_bank_or_rtc_reg {
                        0x0..0x8 => {
                            let sram = self.nth_sram_bank(self.sram_bank_or_rtc_reg);
                            let sram_index = index as usize - 0xA000;
                            sram[sram_index]
                        }
                        0x8..0xD => self.rtc.read(self.sram_bank_or_rtc_reg),
                        _ => unreachable!(),
                    }
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
                self.ram_and_rtc_enabled = value & 0x0F == 0xA;

                #[cfg(feature = "mbc-logging")]
                if self.ram_and_rtc_enabled {
                    info!(target: "mbc_events", "Enabling SRAM...");
                } else {
                    info!(target: "mbc_events", "Disabling SRAM...");
                }
            }
            // MBC3: ROM Bank Number
            0x2000..0x4000 => self.update_rom_bank(value),
            // MBC3: SRAM Bank Number or RTC Register Select
            0x4000..0x6000 => self.update_sram_bank_or_rtc_reg(value),
            // MBC3: RTC Latch
            0x6000..0x8000 => match value {
                0 => self.rtc_latch = false,
                1 => {
                    // Low to high transition.
                    if !self.rtc_latch {
                        // Update the RTC.
                        self.rtc.update();
                    }

                    self.rtc_latch = true;
                }
                _ => (),
            },

            // MBC3: SRAM
            SRAM_START..SRAM_END => {
                // Only allow writes if the MBC RAM has been enabled.
                if self.ram_and_rtc_enabled {
                    match self.sram_bank_or_rtc_reg {
                        0x0..0x8 => {
                            let sram = self.nth_sram_bank_mut(self.sram_bank_or_rtc_reg);
                            let sram_index = index as usize - 0xA000;
                            sram[sram_index] = value;
                        }
                        0x8..0xD => self.rtc.write(self.sram_bank_or_rtc_reg, value),
                        _ => unreachable!(),
                    }
                }
            }
            _ => unreachable!(),
        }
    }
}

impl Default for Mbc3 {
    fn default() -> Self {
        Self {
            ram_and_rtc_enabled: false,
            rom: Vec::new(),
            sram: Vec::new(),
            rtc: Rtc::default(),
            rtc_latch: true,
            current_rom_bank: 1,
            current_rom_bank_start: 0,
            sram_bank_or_rtc_reg: 0,
        }
    }
}
