use hw_constants::{PostBoot, io_regs::DIV};
use rkyv::{Archive, Deserialize, Serialize};

use crate::addressable::Addressable;

#[derive(Default, Archive, Deserialize, Serialize)]
pub struct Timers {
    // Clock register incremented every T-Cycle.
    // Upper 8-bits exposed as the DIV register in memory.
    system_clock: u16,
    tima: u8,
    tma: u8,
    tima_enabled: bool,
    clock_select_bit: u8,
    tima_edge: bool,
    interrupt_queued: bool,
}

impl Timers {
    pub fn update_timer_counter(&mut self, tima: u8) {
        self.tima = tima;
    }

    pub fn update_timer_modulo(&mut self, tma: u8) {
        self.tma = tma;
    }

    pub fn update_timer_control(&mut self, tac: u8) {
        self.tima_enabled = tac & 0b0000_0100 == 0b0000_0100;
        self.clock_select_bit = match tac & 0b0000_0011 {
            0 => 9,
            1 => 3,
            2 => 5,
            3 => 7,
            _ => unreachable!(),
        };
    }

    pub fn increment(&mut self, m_cycles: u16) {
        // 1 TCycle = 4 MCycles
        let t_cycles = m_cycles * 4;

        let mask: u16 = 1 << self.clock_select_bit;

        for _ in 0..t_cycles {
            self.system_clock = self.system_clock.wrapping_add(1);

            let next_tima_edge = self.tima_enabled && self.system_clock & mask == mask;
            // If there was a falling edge, increment TIMA.
            if self.tima_edge && !next_tima_edge {
                let (next_tima, carry) = self.tima.overflowing_add(1);
                self.tima = next_tima;

                // Overflow, set timer counter to the timer modulo and request for a timer interrupt.
                if carry {
                    self.tima = self.tma;
                    self.interrupt_queued = true;
                }
            }
            self.tima_edge = next_tima_edge;
        }
    }

    fn div(&self) -> u8 {
        (self.system_clock >> 8) as u8
    }

    pub fn tima(&self) -> u8 {
        self.tima
    }

    pub fn process_interrupt(&mut self) -> bool {
        if self.interrupt_queued {
            self.interrupt_queued = false;
            return true;
        }
        false
    }
}

impl Addressable for Timers {
    fn read_byte(&self, index: u16) -> u8 {
        match index {
            DIV => self.div(),
            _ => unreachable!(),
        }
    }

    fn write_byte(&mut self, index: u16, _: u8, _: u64) {
        match index {
            // Writing any value to this register resets it to 0.
            // See: https://gbdev.io/pandocs/Timer_and_Divider_Registers.html#ff04--div-divider-register
            DIV => self.system_clock = 0,
            _ => unreachable!(),
        }
    }
}

impl PostBoot for Timers {
    fn post_boot_mgb() -> Self {
        Self {
            // Upper byte (0xAB) is based on documented value of the DIV register after an MGB boots.
            // See: https://gbdev.io/pandocs/Power_Up_Sequence.html#hardware-registers
            // Lower byte (0xCC) is just the lowest value that managed to pass boot_div-dmgABCmgb.gb
            system_clock: 0xABCC,
            ..Self::default()
        }
    }
}
