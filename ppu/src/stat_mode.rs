const DOTS_PER_SCANLINE: usize = 456;

pub(crate) struct StatMachine {
    mode: Mode,
    ticks_remaining: usize,
    line_number: usize,
}

#[derive(Debug, Copy, Clone)]
pub enum Mode {
    OamScan,
    Drawing,
    HBlank,
    VBlank,
}

impl StatMachine {
    pub fn tick(&mut self) {
        if self.ticks_remaining == 0 {
            match self.mode {
                Mode::OamScan => {
                    self.mode = Mode::Drawing;
                    self.ticks_remaining = 172; // Variable
                },
                Mode::Drawing => {
                    self.mode = Mode::HBlank;
                    self.ticks_remaining = 204; // Variable
                },
                Mode::HBlank => {
                    if self.line_number == 143 {
                        self.mode = Mode::VBlank;
                        self.ticks_remaining = DOTS_PER_SCANLINE * 10;

                        self.line_number = 0;
                    } else {
                        self.mode = Mode::OamScan;
                        self.ticks_remaining = 80;

                        self.line_number += 1;
                    }
                },
                Mode::VBlank => {
                    self.mode = Mode::OamScan;
                    self.ticks_remaining = 80;
                },
            }
        }

        self.ticks_remaining -= 1;
    }

    pub fn mode(&self) -> Mode {
        self.mode
    }
}

impl Default for StatMachine {
    fn default() -> Self {
        StatMachine {
            mode: Mode::VBlank,
            ticks_remaining: 4,
            line_number: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn skip_lines(stat_machine: &mut StatMachine, number_of_lines: usize) {
        for _ in 0..DOTS_PER_SCANLINE * number_of_lines {
            stat_machine.tick();
        }
    }

    #[test]
    fn test_line_1_mode_0() {
        let mut stat_machine = StatMachine::default();
        for _ in 0..DOTS_PER_SCANLINE + 4 {
            stat_machine.tick();
            //println!("Dot: {dot} = {:?}", &stat_machine.mode);
        }
        stat_machine.tick();
        assert!(matches!(stat_machine.mode, Mode::OamScan));
    }
}