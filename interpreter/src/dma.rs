// 1 M-cycle for the write + 1 delay + 160 for each byte of OAM.
// See comments: https://github.com/Gekkio/mooneye-test-suite/blob/443f6e1f2a8d83ad9da051cbb960311c5aaaea66/acceptance/oam_dma_start.s
#[derive(Clone, Copy, Default)]
pub enum DmaState {
    #[default]
    Idle,
    Initiated,
    Pending,
    // M-Cycles remaining
    Active(u8),
}

#[derive(Default)]
pub struct Dma {
    pub clock: u64,
    pub state: DmaState,
}

impl Dma {
    pub fn start(&mut self) {
        self.state = DmaState::Initiated;
    }

    pub fn is_active(&self) -> bool {
        matches!(self.state, DmaState::Active(_))
    }

    // Returns true on Pending -> Active transition.
    fn tick(&mut self) -> bool {
        match self.state {
            DmaState::Initiated => self.state = DmaState::Pending,
            DmaState::Pending => {
                self.state = DmaState::Active(159);
                return true;
            }
            DmaState::Active(0) => self.state = DmaState::Idle,
            DmaState::Active(remaining) => self.state = DmaState::Active(remaining - 1),
            DmaState::Idle => (),
        }

        false
    }

    // Returns true on Pending -> Active transition.
    pub fn catch_up(&mut self, cpu_clock: u64) -> bool {
        let catch_up_m_cycles = (cpu_clock - self.clock) / 4;
        self.clock = cpu_clock;

        let mut transition = false;
        for _ in 0..catch_up_m_cycles {
            if let DmaState::Idle = self.state {
                break;
            }
            transition |= self.tick();
        }
        transition
    }
}
