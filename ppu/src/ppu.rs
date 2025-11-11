use crate::{lcd, oam};
use crate::bg_fetcher::{BackgroundFetcher, Pixel};
use crate::oam::Obj;
use crate::obj_fetcher::ObjectFetcher;

const SCANLINES_PER_FRAME: usize = 154;
const DOTS_PER_SCANLINE: usize = 456;
const DOTS_PER_FRAME: usize = DOTS_PER_SCANLINE * SCANLINES_PER_FRAME;

const OAM_SCAN_DOTS: usize = 80;

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
    pixels_to_drop: u8,
    window_y: u8,
    bg_fetcher: BackgroundFetcher,
    obj_buffer: Vec<Obj>,
    obj_fetcher: ObjectFetcher,
    pub funny_buffer_test: Vec<u8>,
}

fn drawing_window(memory: &[u8], x: u8, y: u8) -> bool {
    lcd::window_enabled(memory) && x + 7 == memory[WX] && y >= memory[WY]
}

fn mix_pixels(bg_pixel: Pixel, obj_pixel: Pixel) -> Pixel {
    let mut render_bg = false;
    render_bg |= !(obj_pixel.low || obj_pixel.high);
    render_bg |= obj_pixel.priority && (bg_pixel.low || bg_pixel.high);

    if render_bg {
        bg_pixel
    } else {
        obj_pixel
    }
}

impl Ppu {
    pub fn ly(&self) -> u8 {
        (self.dot_counter / DOTS_PER_SCANLINE) as u8
    }

    fn current_obj(&self) -> Option<Obj> {
        self.obj_buffer.iter().filter(|obj| obj.intersects_x(self.x)).cloned().next()
    }

    // Advance the PPU by 1 dot.
    pub fn tick(&mut self, memory: &[u8]) {
        self.dot_counter += 1;
        match self.mode {
            PpuMode::OamScan => {
                if self.dot_counter % DOTS_PER_SCANLINE >= OAM_SCAN_DOTS {
                    // This is the last cycle of the OAM scan, so lets actually do the OAM scan.
                    self.obj_buffer = oam::oam_scan(memory, self.ly());

                    // Prepare for Drawing.
                    self.pixels_to_drop = memory[SCX] & 7;
                    self.mode = PpuMode::Drawing;
                }
            },
            PpuMode::Drawing => {
                if let Some(obj) = self.current_obj() {
                    self.obj_fetcher.tick(memory, self.ly(), obj);
                }

                if self.obj_fetcher.done {
                    self.bg_fetcher.tick(memory, self.ly(), self.window_y);
                    //println!("Dot: {}, X: {}, FIFO: {}", (self.dot_counter % DOTS_PER_SCANLINE) - OAM_SCAN_DOTS, self.x, self.bg_fetcher.bg_fifo.len());

                    // TODO: Combine FIFOs correctly.
                    if let Some(bg_pixel) = self.bg_fetcher.shift_out() {
                        if self.pixels_to_drop > 0 {
                            self.pixels_to_drop -= 1
                        } else {
                            let mut pixel_to_render = bg_pixel;
                            if let Some(obj_pixel) = self.obj_fetcher.shift_out() {
                                pixel_to_render = mix_pixels(bg_pixel, obj_pixel);
                            }

                            let funny_index = self.ly() as usize * 160 + self.x as usize;
                            let mut funny_greyscale = 0;
                            if pixel_to_render.low {
                                funny_greyscale |= 0b0000_0001;
                            }
                            if pixel_to_render.high {
                                funny_greyscale |= 0b0000_0010;
                            }
                            self.funny_buffer_test[funny_index] = funny_greyscale * 64;

                            self.x += 1;
                        }
                    }
                }


                if drawing_window(memory, self.x, self.ly()) && !self.bg_fetcher.drawing_window {
                    self.window_y = self.window_y.wrapping_add(1);
                    self.bg_fetcher = BackgroundFetcher::default();
                    self.bg_fetcher.warmup = false;
                    self.bg_fetcher.drawing_window = true;
                }

                if self.x >= 160 {
                    println!("Drew for {} dots", (self.dot_counter % DOTS_PER_SCANLINE) - OAM_SCAN_DOTS);
                    self.x = 0;
                    self.bg_fetcher = BackgroundFetcher::default();
                    self.obj_fetcher = ObjectFetcher::default();
                    self.mode = PpuMode::HBlank;
                }
            },
            PpuMode::HBlank => {
                if self.dot_counter.is_multiple_of(DOTS_PER_SCANLINE) {
                    if self.ly() < 144 {
                        self.mode = PpuMode::OamScan;
                    } else {
                        self.mode = PpuMode::VBlank;
                    }
                }
            },
            PpuMode::VBlank => {
                if self.dot_counter == DOTS_PER_FRAME {
                    self.dot_counter = 0;
                    self.window_y = 255;
                    self.mode = PpuMode::OamScan;
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
            pixels_to_drop: 0,
            window_y: 255,
            bg_fetcher: BackgroundFetcher::default(),
            obj_buffer: Vec::with_capacity(10),
            obj_fetcher: ObjectFetcher::default(),
            funny_buffer_test: vec![0; 160 * 144],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Assert that the minimum Mode 3 length (172) with:
    // - unscrolled background tiles (0)
    // - no window (0)
    // - no objects (0)
    // is 172 dots.
    // See: https://gbdev.io/pandocs/Rendering.html#mode-3-length
    #[test]
    fn test_minimum_bg_mode_3_dots() {
        let mut ppu = Ppu::default();
        let memory = [0; 0x10000];

        while !matches!(ppu.mode, PpuMode::HBlank) {
            ppu.tick(&memory);
        }

        let mode_3_dots = ppu.dot_counter - OAM_SCAN_DOTS;
        assert_eq!(mode_3_dots, 172);
    }

    // Assert that the minimum Mode 3 length (172) with:
    // - scrolled background tiles (SCX % 8 = 7)
    // - no window (0)
    // - no objects (0)
    // is 179 dots.
    // See: https://gbdev.io/pandocs/Rendering.html#mode-3-length
    #[test]
    fn test_scrolled_bg_mode_3_dots() {
        let mut ppu = Ppu::default();
        let mut memory = [0; 0x10000];
        memory[SCX] = 7;

        while !matches!(ppu.mode, PpuMode::HBlank) {
            ppu.tick(&memory);
        }

        let mode_3_dots = ppu.dot_counter - OAM_SCAN_DOTS;
        assert_eq!(mode_3_dots, 179);
    }

    // Assert that the minimum Mode 3 length (172) with:
    // - unscrolled background tiles (0)
    // - a window (6)
    // - no objects (0)
    // is 178 dots.
    // See: https://gbdev.io/pandocs/Rendering.html#mode-3-length
    #[test]
    fn test_minimum_bg_window_mode_3_dots() {
        let mut ppu = Ppu::default();
        let mut memory = [0; 0x10000];

        // Enable the window.
        const LCDC: usize = 0xFF40;
        memory[LCDC] |= 0b0010_0000;
        // Scroll it to x=50px
        memory[WX] = 50 + 7;

        while !matches!(ppu.mode, PpuMode::HBlank) {
            ppu.tick(&memory);
        }

        let mode_3_dots = ppu.dot_counter - OAM_SCAN_DOTS;
        assert_eq!(mode_3_dots, 178);
    }

    // Assert that the minimum Mode 3 length (172) with:
    // - scrolled background tiles (SCX % 8 = 7)
    // - a window (6)
    // - no objects (0)
    // is 185 dots.
    // See: https://gbdev.io/pandocs/Rendering.html#mode-3-length
    #[test]
    fn test_scrolled_bg_window_mode_3_dots() {
        let mut ppu = Ppu::default();
        let mut memory = [0; 0x10000];
        memory[SCX] = 7;

        // Enable the window.
        const LCDC: usize = 0xFF40;
        memory[LCDC] |= 0b0010_0000;
        // Scroll it to x=50px
        memory[WX] = 50 + 7;

        while !matches!(ppu.mode, PpuMode::HBlank) {
            ppu.tick(&memory);
        }

        let mode_3_dots = ppu.dot_counter - OAM_SCAN_DOTS;
        assert_eq!(mode_3_dots, 185);
    }
}