use hw_constants::{
    PostBoot,
    io_regs::{DIV, TIMA},
};
use rkyv::{Archive, Deserialize, Serialize};

use crate::addressable::Addressable;

#[derive(Archive, Deserialize, Serialize)]
enum TimaOverflowState {
    Cancelable,
    IgnoringWrites,
}

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
    // TMA being copied and the interrupt being fired are both delayed by 1 M-Cycles.
    // See: https://gbdev.io/pandocs/Timer_Obscure_Behaviour.html#timer-overflow-behavior
    tima_overflow_state: Option<TimaOverflowState>,
    interrupt_queued: bool,
}

impl Timers {
    fn update_timer_counter(&mut self, tima: u8) {
        println!("Write {:02X} to TIMA at SYS: {}", tima, self.system_clock);
        if let Some(TimaOverflowState::IgnoringWrites) = self.tima_overflow_state {
            return;
        }
        self.tima = tima;
        // Writing to TIMA when an overflow update is queued cancels the update.
        self.tima_overflow_state = None;
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

        let mask: u16 = 1 << self.clock_select_bit;

        for _ in 0..m_cycles {
            self.system_clock = self.system_clock.wrapping_add(4);

            self.tima_overflow_state = match self.tima_overflow_state {
                Some(TimaOverflowState::Cancelable) => {
                    println!(
                        "Overflow with {:02X} at SYS: {}",
                        self.tma, self.system_clock
                    );
                    self.tima = self.tma;
                    self.interrupt_queued = true;
                    Some(TimaOverflowState::IgnoringWrites)
                }
                Some(TimaOverflowState::IgnoringWrites) | None => None,
            };

            let next_tima_edge = self.tima_enabled && self.system_clock & mask == mask;
            // If there was a falling edge, increment TIMA.
            if self.tima_edge && !next_tima_edge {
                let (next_tima, carry) = self.tima.overflowing_add(1);
                self.tima = next_tima;

                // Overflow, queue setting the timer counter to the timer modulo and requesting for a timer interrupt.
                if carry {
                    println!("Queue overflow at SYS: {}", self.system_clock);
                    self.tima_overflow_state = Some(TimaOverflowState::Cancelable);
                }
            }
            self.tima_edge = next_tima_edge;
        }
    }

    fn div(&self) -> u8 {
        (self.system_clock >> 8) as u8
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
            TIMA => {
                println!(
                    "Reading TIMA {:02X} at SYS: {}",
                    self.tima, self.system_clock
                );
                self.tima
            }
            _ => unreachable!(),
        }
    }

    fn write_byte(&mut self, index: u16, value: u8, _: u64) {
        match index {
            // Writing any value to this register resets it to 0.
            // See: https://gbdev.io/pandocs/Timer_and_Divider_Registers.html#ff04--div-divider-register
            DIV => self.system_clock = 0,
            TIMA => self.update_timer_counter(value),
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
