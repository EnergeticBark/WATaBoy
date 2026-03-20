mod registers;

use hw_constants::{
    PostBoot,
    io_regs::{DIV, TAC, TIMA, TMA},
};
use rkyv::{Archive, Deserialize, Serialize};

use registers::TimerControl;

use crate::{addressable::Addressable, cpu::InterruptBits};

#[derive(Archive, Deserialize, Serialize)]
enum TimaOverflowState {
    Cancelable,
    IgnoringWrites,
}

// TODO: On monochrome consoles, disabling the timer if the currently selected bit is set, will send a “Timer tick” once.
// See: https://gbdev.io/pandocs/Timer_Obscure_Behaviour.html#relation-between-timer-and-divider-register
// Is there a test ROM for this functionality?
#[derive(Archive, Deserialize, Serialize)]
pub struct Timers {
    // Clock register incremented every T-Cycle.
    // Upper 8-bits exposed as the DIV register in memory.
    system_clock: u16,
    tima: u8,
    // Timer modulo
    tma: u8,
    pub tac: TimerControl,
    tima_edge: bool,
    // TMA being copied and the interrupt being fired are both delayed by 1 M-Cycles.
    // See: https://gbdev.io/pandocs/Timer_Obscure_Behaviour.html#timer-overflow-behavior
    tima_overflow_state: Option<TimaOverflowState>,
    interrupt_queued: bool,
    pub clock: u64,
    pub next_interrupt: u64,
}

impl Timers {
    pub fn predict_next_interrupt(&mut self, ie: InterruptBits) -> u64 {
        if !ie.timer() {
            self.next_interrupt = u64::MAX;
            return self.next_interrupt;
        }

        if matches!(
            self.tima_overflow_state,
            Some(TimaOverflowState::Cancelable)
        ) {
            // The interrupt from TIMA overflowing will occur even if TIMA was disabled.
            self.next_interrupt = self.clock + 4;
            return self.next_interrupt;
        }

        if !self.tac.tima_enabled() {
            self.next_interrupt = u64::MAX;
            return self.next_interrupt;
        }

        // TODO: Actually calculate this
        self.next_interrupt = self.clock;
        self.next_interrupt
    }

    pub fn catch_up(&mut self, cpu_clock: u64) {
        let clock_delta = cpu_clock - self.clock;
        let m_cycles = clock_delta / 4;

        self.increment(m_cycles);
        self.clock += m_cycles * 4;
    }

    fn reset_divider_register(&mut self) {
        self.system_clock = 0;
        self.try_ticking_tima();
    }

    fn update_timer_counter(&mut self, tima: u8) {
        self.tima = tima;
        // Writing to TIMA when an overflow update is queued cancels the update.
        if let Some(TimaOverflowState::Cancelable) = self.tima_overflow_state {
            self.tima_overflow_state = None;
        }
    }

    fn update_timer_control(&mut self, tac: u8) {
        self.tac = TimerControl::from_bits(tac);
        self.try_ticking_tima();
    }

    fn try_ticking_tima(&mut self) {
        let mask = self.tac.clock_select().mask();
        let next_tima_edge = self.tac.tima_enabled() && self.system_clock & mask == mask;
        // If there was a falling edge, increment TIMA.
        if self.tima_edge && !next_tima_edge {
            let (next_tima, carry) = self.tima.overflowing_add(1);
            self.tima = next_tima;

            // Overflow, queue setting the timer counter to the timer modulo and requesting for a timer interrupt.
            if carry {
                self.tima_overflow_state = Some(TimaOverflowState::Cancelable);
            }
        }
        self.tima_edge = next_tima_edge;
    }

    fn increment(&mut self, m_cycles: u64) {
        for _ in 0..m_cycles {
            self.system_clock = self.system_clock.wrapping_add(4);

            self.tima_overflow_state = match self.tima_overflow_state {
                Some(TimaOverflowState::Cancelable) => {
                    self.tima = self.tma;
                    self.interrupt_queued = true;
                    Some(TimaOverflowState::IgnoringWrites)
                }
                Some(TimaOverflowState::IgnoringWrites) => {
                    // The timer modulo's value is constantly being copied until tima_overflow_state is None.
                    // See: https://gbdev.io/pandocs/Timer_Obscure_Behaviour.html#timer-overflow-behavior
                    self.tima = self.tma;
                    None
                }
                None => None,
            };

            self.try_ticking_tima();
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
            TIMA => self.tima,
            TMA => self.tma,
            TAC => self.tac.into_bits(),
            _ => unreachable!(),
        }
    }

    fn write_byte(&mut self, index: u16, value: u8, _: u64) {
        match index {
            // Writing any value to this register resets it to 0.
            // See: https://gbdev.io/pandocs/Timer_and_Divider_Registers.html#ff04--div-divider-register
            DIV => self.reset_divider_register(),
            TIMA => self.update_timer_counter(value),
            TMA => self.tma = value,
            TAC => self.update_timer_control(value),
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

impl Default for Timers {
    fn default() -> Self {
        Self {
            system_clock: 8,
            tima: 0,
            tma: 0,
            tac: TimerControl::default(),
            tima_edge: false,
            tima_overflow_state: None,
            interrupt_queued: false,
            clock: 0,
            next_interrupt: 0,
        }
    }
}
