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

    // Skip (delta_m_cycles - 1) in one big chunk, then increment the timer once.
    // TODO: This implementation needs more unit testing, but it passes the test ROMs.
    // Is there any way that the wrong IF might be read if TIMA overflowed multiple times?
    #[allow(clippy::cast_possible_truncation)]
    pub fn catch_up_coarse(&mut self, cpu_clock: u64, interrupt_flags: &mut u8) {
        let clock_delta = cpu_clock - self.clock;
        let delta_m_cycles = clock_delta / 4;

        if delta_m_cycles == 0 {
            return;
        }

        let skip_m_cycles = delta_m_cycles.saturating_sub(1);
        if skip_m_cycles > 0 {
            self.tima_overflow_state = match self.tima_overflow_state {
                Some(TimaOverflowState::Cancelable) => {
                    self.tima = self.tma;
                    *interrupt_flags |= 0b0000_0100;

                    if skip_m_cycles == 1 {
                        Some(TimaOverflowState::IgnoringWrites)
                    } else {
                        None
                    }
                }
                Some(TimaOverflowState::IgnoringWrites) => {
                    // The timer modulo's value is constantly being copied until tima_overflow_state is None.
                    // See: https://gbdev.io/pandocs/Timer_Obscure_Behaviour.html#timer-overflow-behavior
                    self.tima = self.tma;
                    None
                }
                None => None,
            };

            if self.tac.tima_enabled() {
                let tima_phase =
                    u64::from((self.system_clock / 4) % self.tac.clock_select().period());
                // TODO: Shrink this from u64 to the actual minimum it can be.
                let delta_tima =
                    (tima_phase + skip_m_cycles) / u64::from(self.tac.clock_select().period());
                let tima_rem_m_cycles =
                    (tima_phase + skip_m_cycles) % u64::from(self.tac.clock_select().period());

                let (next_tima, carry) = self.tima.overflowing_add(delta_tima as u8);
                self.tima = next_tima;
                if carry {
                    if self.tma > 0 {
                        self.tima = self.tma + (next_tima % 0_u8.wrapping_sub(self.tma));
                    }

                    match tima_rem_m_cycles {
                        0 => {
                            // Overflow just happened this M-Cycle
                            self.tima_overflow_state = Some(TimaOverflowState::Cancelable);
                        }
                        1 => {
                            *interrupt_flags |= 0b0000_0100;
                            self.tima_overflow_state = Some(TimaOverflowState::IgnoringWrites);
                        }
                        _ => {
                            *interrupt_flags |= 0b0000_0100;
                            self.tima_overflow_state = None;
                        }
                    }
                }
            }

            self.system_clock =
                (self.system_clock).wrapping_add((skip_m_cycles as u16).wrapping_mul(4));

            let mask = self.tac.clock_select().mask();
            self.tima_edge = self.tac.tima_enabled() && self.system_clock & mask == mask;
        }

        self.increment(1, interrupt_flags);

        self.clock += delta_m_cycles * 4;
    }

    pub fn catch_up(&mut self, cpu_clock: u64, interrupt_flags: &mut u8) {
        /*let clock_delta = cpu_clock - self.clock;
        let m_cycles = clock_delta / 4;

        self.increment(m_cycles, interrupt_flags);
        self.clock += m_cycles * 4;*/
        self.catch_up_coarse(cpu_clock, interrupt_flags);
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

    fn increment(&mut self, m_cycles: u64, interrupt_flags: &mut u8) {
        for _ in 0..m_cycles {
            self.system_clock = self.system_clock.wrapping_add(4);

            self.tima_overflow_state = match self.tima_overflow_state {
                Some(TimaOverflowState::Cancelable) => {
                    self.tima = self.tma;
                    *interrupt_flags |= 0b0000_0100;
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
}

impl Addressable for Timers {
    fn read_byte(&self, index: u16, _: u64) -> u8 {
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
            clock: 0,
            next_interrupt: 0,
        }
    }
}
