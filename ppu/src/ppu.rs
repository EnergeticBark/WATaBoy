use std::collections::VecDeque;

const SCANLINES_PER_FRAME: usize = 154;
const DOTS_PER_SCANLINE: usize = 456;
const DOT_PER_FRAME: usize = DOTS_PER_SCANLINE * SCANLINES_PER_FRAME;

enum PpuMode {
    HBlank,
    VBlank,
    OamScan,
    Drawing,
}

struct Ppu {
    mode: PpuMode,
    dot_counter: usize,
    background_fifo: VecDeque<u8>,
    object_fifo: VecDeque<u8>,
}

impl Ppu {
    fn tick(&mut self) {
        match self.mode {
            PpuMode::OamScan => {
                self.dot_counter += 1;
                if self.dot_counter % DOTS_PER_SCANLINE >= 80 {
                    self.mode = PpuMode::Drawing;
                }
            },
            PpuMode::Drawing => {
                self.dot_counter += 1;
                if self.dot_counter % DOTS_PER_SCANLINE >= 369 {
                    self.mode = PpuMode::HBlank;
                }
            },
            PpuMode::HBlank => {
                self.dot_counter += 1;
                if self.dot_counter.is_multiple_of(DOTS_PER_SCANLINE) {
                    if self.dot_counter / DOTS_PER_SCANLINE < 144 {
                        self.mode = PpuMode::OamScan;
                    } else {
                        self.mode = PpuMode::VBlank;
                    }
                }
            },
            PpuMode::VBlank => {
                self.dot_counter += 1;
                if self.dot_counter == DOT_PER_FRAME {
                    self.dot_counter = 0;
                    self.mode = PpuMode::HBlank;
                }
            },
        }
    }
}