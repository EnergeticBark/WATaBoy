use std::ops::{Index, Range};
use crate::timers::Timers;

const MEM_MAP_SIZE: usize = 0x10000;

pub struct AddressBus {
    pub buffer: [u8; MEM_MAP_SIZE],
    pub timers: Timers,
}

impl AddressBus {
    pub fn write_byte(&mut self, index: u16, value: u8) {
        match index {
            0x0000..0x8000 => (),
            0xFF04 => self.timers.system_clock = 0,
            _ => self.buffer[index as usize] = value
        }
    }

    pub fn increment_timers(&mut self, m_cycles: u16) {
        // Memory mapped timer register addresses
        const DIV: u16 = 0xFF04;
        const TIMA: u16 = 0xFF05;
        const TMA: u16 = 0xFF06;
        const TAC: u16 = 0xFF07;

        self.timers.update_timer_counter(self.buffer[TIMA as usize]);
        self.timers.update_timer_modulo(self.buffer[TMA as usize]);
        self.timers.update_timer_control(self.buffer[TAC as usize]);

        self.timers.increment(m_cycles);

        self.buffer[DIV as usize] = self.timers.div();
        self.buffer[TIMA as usize] = self.timers.tima();

        if self.timers.process_interrupt() {
            self.buffer[0xFF0F] |= 0b0000_0100;
        }
    }
}

impl Default for AddressBus {
    fn default() -> Self {
        Self {
            buffer: [0; MEM_MAP_SIZE],
            timers: Timers::default(),
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
