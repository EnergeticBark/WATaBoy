use crate::lcd;
use crate::pixel_fetcher::PixelFetcher;

const SCANLINES_PER_FRAME: usize = 154;
const DOTS_PER_SCANLINE: usize = 456;
const DOTS_PER_FRAME: usize = DOTS_PER_SCANLINE * SCANLINES_PER_FRAME;

const SCX: usize = 0xFF43;

const WY: usize = 0xFF4A;
const WX: usize = 0xFF4B;

enum PpuMode {
    HBlank,
    VBlank,
    OamScan,
    Drawing,
}

pub struct Ppu {
    mode: PpuMode,
    dot_counter: usize,
    x: u8,
    window_y: u8,
    fetcher: PixelFetcher,
    //object_fifo: VecDeque<u8>,
    pub funny_buffer_test: Vec<u8>,
}

fn drawing_window(memory: &[u8], x: u8, y: u8) -> bool {
    lcd::window_enabled(memory) && lcd::bg_and_window_enabled(memory) && (x + 7) >= memory[WX] && y >= memory[WY]
}

impl Ppu {
    fn ly(&self) -> usize {
        self.dot_counter / DOTS_PER_SCANLINE
    }

    // Advance the PPU by 1 dot.
    pub fn tick(&mut self, memory: &[u8]) {
        match self.mode {
            PpuMode::OamScan => {
                self.dot_counter += 1;
                if self.dot_counter % DOTS_PER_SCANLINE >= 80 {
                    self.mode = PpuMode::Drawing;
                }
            },
            PpuMode::Drawing => {
                self.dot_counter += 1;

                self.fetcher.tick(memory, self.ly() as u8, self.window_y);
                let pixels_to_drop = 8 + (memory[SCX] & 7);

                if let Some(pixel) = self.fetcher.shift_out() {
                    if self.x >= pixels_to_drop {
                        let funny_index = self.ly() * 160 + self.x as usize - pixels_to_drop as usize;
                        let mut funny_greyscale = 0;
                        if pixel.low {
                            funny_greyscale |= 0b0000_0001;
                        }
                        if pixel.high {
                            funny_greyscale |= 0b0000_0010;
                        }
                        self.funny_buffer_test[funny_index] = funny_greyscale * 64;
                    }

                    self.x += 1;
                }

                if drawing_window(memory, self.x, self.ly() as u8) && !self.fetcher.drawing_window {
                    self.window_y += 1;
                    self.fetcher = PixelFetcher::default();
                    self.fetcher.drawing_window = true;
                }

                if self.x >= 160 + pixels_to_drop {
                    self.x = 0;
                    self.fetcher = PixelFetcher::default();
                    self.mode = PpuMode::HBlank;
                }
            },
            PpuMode::HBlank => {
                self.dot_counter += 1;
                if self.dot_counter.is_multiple_of(DOTS_PER_SCANLINE) {
                    if self.ly() < 144 {
                        self.mode = PpuMode::OamScan;
                    } else {
                        self.mode = PpuMode::VBlank;
                    }
                }
            },
            PpuMode::VBlank => {
                self.dot_counter += 1;
                if self.dot_counter == DOTS_PER_FRAME {
                    self.dot_counter = 0;
                    self.window_y = 0;
                    self.mode = PpuMode::HBlank;
                }
            },
        }
    }
}

impl Default for Ppu {
    fn default() -> Self {
        Self {
            mode: PpuMode::OamScan,
            dot_counter: 0,
            x: 0,
            window_y: 0,
            fetcher: PixelFetcher::default(),
            funny_buffer_test: vec![0; 160 * 144],
        }
    }
}