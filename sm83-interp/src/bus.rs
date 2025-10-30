use crate::hw_addrs;
use crate::timers::Timers;
use std::ops::{Index, Range};
use crate::mbc::Mbc;

const MEM_MAP_SIZE: usize = 0x10000;

pub struct AddressBus {
    pub buffer: [u8; MEM_MAP_SIZE],
    pub timers: Timers,
    pub mbc: Mbc,
}

impl AddressBus {
    pub fn load_rom(&mut self, rom: &[u8]) {
        self.mbc.load_rom(rom);
    }
    
    pub fn write_byte(&mut self, index: u16, value: u8) {
        match index {
            // Handle writes to ROM address space.
            0x0000..0x2000 => {
                if value & 0x0F == 0xA {
                    println!("Enabling ram...");
                    self.mbc.ram_enabled = true;
                } else {
                    println!("Disabling ram...");
                    self.mbc.ram_enabled = false;
                }
            },
            0x2000..0x4000 => {
                println!("Switching ROM bank using value: {value}");
                let bank = self.mbc.nth_rom_bank(value);
                self.buffer[0x4000..0x8000].clone_from_slice(bank);
            },
            0x4000..0x6000 => {
                println!("Switching RAM bank using value: {value}");
                if self.mbc.banking_mode {
                    // Backup old bank... Ew, I know I can do better than this.
                    self.mbc.write_ram_bank(&self.buffer[0xA000..0xC000].try_into().unwrap());

                    let bank = self.mbc.nth_ram_bank(value);
                    self.buffer[0xA000..0xC000].clone_from_slice(bank);
                } else {
                    println!("Actually no, we're in simple mode!!!");
                }
            },
            0x6000..0x8000 => self.mbc.set_banking_mode(value),
            0xA000..0xC000 => {
                if self.mbc.ram_enabled {
                    self.buffer[index as usize] = value;
                }
            }

            // Certain I/O addresses only use certain bits. Bits which go unused are pulled high.
            // See Appendix B: https://gekkio.fi/files/gb-docs/gbctr.pdf
            hw_addrs::JOYP | hw_addrs::NR41 => self.buffer[index as usize] = value | 0b1100_0000,
            hw_addrs::SC => self.buffer[index as usize] = value | 0b0111_1110,
            hw_addrs::TAC => self.buffer[index as usize] = value | 0b1111_1000,
            hw_addrs::DIV => self.timers.system_clock = 0,
            hw_addrs::IF => self.buffer[index as usize] = value | 0b1110_0000,
            hw_addrs::STAT | hw_addrs::NR10 => self.buffer[index as usize] = value | 0b1000_0000,
            hw_addrs::NR30 => self.buffer[index as usize] = value | 0b0111_1111,
            hw_addrs::NR32 => self.buffer[index as usize] = value | 0b1001_1111,
            hw_addrs::NR44 => self.buffer[index as usize] = value | 0b0011_1111,
            hw_addrs::NR52 => self.buffer[index as usize] = value | 0b0111_0000,

            // There is *nothing* at these addresses, so they don't have names.
            // Their bits are always pulled high.
            0xFF03 | 0xFF08..0xFF0F | 0xFF15 | 0xFF1F | 0xFF27..0xFF30 | 0xFF4C..0xFF80 => {
                self.buffer[index as usize] = value | 0b1111_1111;
            }
            _ => self.buffer[index as usize] = value,
        }
    }

    pub fn increment_timers(&mut self, m_cycles: u16) {
        self.timers
            .update_timer_counter(self.buffer[hw_addrs::TIMA as usize]);
        self.timers
            .update_timer_modulo(self.buffer[hw_addrs::TMA as usize]);
        self.timers
            .update_timer_control(self.buffer[hw_addrs::TAC as usize]);

        self.timers.increment(m_cycles);

        self.buffer[hw_addrs::DIV as usize] = self.timers.div();
        self.buffer[hw_addrs::TIMA as usize] = self.timers.tima();

        if self.timers.process_interrupt() {
            self.buffer[hw_addrs::IF as usize] |= 0b0000_0100;
        }
    }
}

impl Default for AddressBus {
    fn default() -> Self {
        Self {
            buffer: [0; MEM_MAP_SIZE],
            timers: Timers::default(),
            mbc: Mbc::default(),
        }
    }
}

impl Index<u16> for AddressBus {
    type Output = u8;

    fn index(&self, index: u16) -> &Self::Output {
        &self.buffer[index as usize]
    }
}

impl Index<Range<u16>> for AddressBus {
    type Output = [u8];

    fn index(&self, index: Range<u16>) -> &Self::Output {
        &self.buffer[index.start as usize..index.end as usize]
    }
}
