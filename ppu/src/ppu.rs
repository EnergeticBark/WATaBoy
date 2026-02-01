use std::collections::VecDeque;
use crate::bg_fetcher::{BackgroundFetcher, Pixel};
use crate::oam::Obj;
use crate::obj_fetcher::ObjectFetcher;
use crate::{lcd_control, oam, lcd_status, palette};

use hw_constants::io_regs;
use log::{trace, warn};
use crate::palette::Palette;

const SCANLINES_PER_FRAME: usize = 154;
const DOTS_PER_SCANLINE: usize = 456;
const DOTS_PER_FRAME: usize = DOTS_PER_SCANLINE * SCANLINES_PER_FRAME;

const OAM_SCAN_DOTS: usize = 80;

enum PpuMode {
    HBlank,
    VBlank,
    OamScan,
    Drawing,
}

pub struct Ppu {
    mode: PpuMode,
    pub dot_counter: usize,
    x: u8,
    pixels_to_drop: u8,
    window_y: u8,
    bg_fetcher: BackgroundFetcher,
    // Why is this here *and* in ObjectFetcher? Because the ObjectFetcher doesn't know the current x
    // coordinate the LCD is rendering.
    // The PPU decides *when* to fetch an object, whereas the ObjectFetcher just mindlessly fetches
    // whatever the PPU puts into its queue.
    obj_buffer: VecDeque<Obj>,
    obj_fetcher: ObjectFetcher,
    stat_interrupt_line: bool,
    pub funny_buffer_test: Vec<u8>,
    disabled: bool,
    update_stat_on_dot: usize,
    ly_to_compare_lyc: u8,
}

fn drawing_window(memory: &[u8], x: u8, y: u8) -> bool {
    lcd_control::window_enabled(memory)
        && x + 7 == memory[io_regs::WX as usize]
        && y >= memory[io_regs::WY as usize]
}

fn mix_pixels(bg_pixel: Pixel, obj_pixel: Pixel) -> Pixel {
    let mut render_bg = false;
    render_bg |= !(obj_pixel.low || obj_pixel.high);
    render_bg |= obj_pixel.priority && (bg_pixel.low || bg_pixel.high);

    if render_bg { bg_pixel } else { obj_pixel }
}

impl Ppu {
    pub fn ly(&self) -> u8 {
        (self.dot_counter / DOTS_PER_SCANLINE) as u8
    }

    fn pop_next_obj(&mut self) -> Option<Obj> {
        // Discard any fully off-screen sprites.
        if self.obj_buffer.pop_front_if(|obj| obj.x_pos == 0).is_some() {
            return None;
        }

        self.obj_buffer.pop_front_if(|obj| obj.intersects_x(self.x))
    }

    fn transition_hblank(&mut self) {
        self.mode = PpuMode::HBlank;
        self.update_stat_on_dot = self.dot_counter + 4;

        trace!(target: "ppu_hblank", "Set to Mode 0 on dot: {}, (Drew for {} dots)", self.dot_counter % DOTS_PER_SCANLINE, (self.dot_counter % DOTS_PER_SCANLINE) - OAM_SCAN_DOTS);

        self.x = 0;
        // Reset each of the fetchers.
        self.bg_fetcher = BackgroundFetcher::default();
        self.obj_fetcher = ObjectFetcher::default();
    }

    fn transition_vblank(&mut self, memory: &mut [u8]) {
        self.mode = PpuMode::VBlank;
        self.update_stat_on_dot = self.dot_counter + 4;

        // Request the VBlank interrupt.
        memory[io_regs::IF as usize] |= 0b0000_0001;

        // A VBlank triggering also the OAM being triggered... for some reason?
        // See: https://github.com/Gekkio/mooneye-test-suite/blob/main/acceptance/ppu/vblank_stat_intr-GS.s
        if !self.stat_interrupt_line && lcd_status::mode2_int_select(memory) {
            self.stat_interrupt_line = true;
            // Request the LCD interrupt.
            memory[io_regs::IF as usize] |= 0b0000_0010;
        }
    }

    fn transition_oam_scan(&mut self) {
        self.mode = PpuMode::OamScan;
        self.update_stat_on_dot = self.dot_counter + 4;

        trace!(target: "ppu_hblank", "Set to Mode 2 on dot: {}", self.dot_counter % DOTS_PER_SCANLINE);
    }

    fn transition_drawing(&mut self, memory: &mut [u8]) {
        self.mode = PpuMode::Drawing;
        self.update_stat_on_dot = self.dot_counter + 4;

        // This is the last cycle of the OAM scan, so lets actually do the OAM scan.
        self.obj_buffer = oam::oam_scan(memory, self.ly());

        // Prepare for Drawing.
        self.pixels_to_drop = memory[io_regs::SCX as usize] & 7;
    }

    fn update_ly_register(&self, memory: &mut [u8]) {
        memory[io_regs::LY as usize] = self.ly();
        trace!(target: "ppu_ly", "Updating LY on dot: {}", self.dot_counter % DOTS_PER_SCANLINE);
    }

    // Advance the PPU by 1 dot.
    pub fn tick(&mut self, memory: &mut [u8]) {
        if !lcd_control::lcd_and_ppu_enabled(memory) {
            if !self.disabled {
                warn!(target: "ppu_disabled", "Disabled on dot: {}", self.dot_counter);

                // Reset the PPU state.
                *self = Ppu::default();
                lcd_status::set_ppu_mode(memory, 0);
                self.update_ly_register(memory);
            }
            return;
        }
        if self.disabled {
            self.disabled = false;
            warn!(target: "ppu_enabled", "Enabled");
        }

        self.dot_counter += 1;
        match self.mode {
            PpuMode::OamScan => {
                if self.dot_counter == self.update_stat_on_dot {
                    lcd_status::set_ppu_mode(memory, 2);
                    self.update_stat_on_dot = usize::MAX;
                }

                if self.dot_counter % DOTS_PER_SCANLINE >= OAM_SCAN_DOTS {
                    self.transition_drawing(memory);
                }
            }
            PpuMode::Drawing => {
                if self.dot_counter == self.update_stat_on_dot {
                    lcd_status::set_ppu_mode(memory, 3);
                    self.update_stat_on_dot = usize::MAX;
                }

                if let Some(obj) = self.pop_next_obj() {
                    self.obj_fetcher.push_obj(obj);
                }
                self.obj_fetcher.tick(memory, self.ly());

                if self.obj_fetcher.idle_and_empty() {
                    self.bg_fetcher.tick(memory, self.ly(), self.window_y);
                    //println!("Dot: {}, X: {}, FIFO: {}", (self.dot_counter % DOTS_PER_SCANLINE) - OAM_SCAN_DOTS, self.x, self.bg_fetcher.bg_fifo.len());

                    // TODO: Combine FIFOs correctly.
                    if let Some(bg_pixel) = self.bg_fetcher.shift_out() {
                        if self.pixels_to_drop > 0 {
                            self.pixels_to_drop -= 1
                        } else {
                            // If the background/window is disabled, use a pixel with a value of 0.
                            // See: https://gbdev.io/pandocs/pixel_fifo.html#pixel-rendering
                            let mut pixel_to_render = if lcd_control::bg_and_window_enabled(memory) {
                                bg_pixel
                            } else {
                                Pixel {
                                    low: false,
                                    high: false,
                                    palette: Palette::BGP,
                                    priority: false,
                                }
                            };

                            if let Some(obj_pixel) = self.obj_fetcher.shift_out() && lcd_control::obj_enabled(memory) {
                                pixel_to_render = mix_pixels(pixel_to_render, obj_pixel);
                            }

                            let mut funny_greyscale = 0;
                            if pixel_to_render.low {
                                funny_greyscale |= 0b0000_0001;
                            }
                            if pixel_to_render.high {
                                funny_greyscale |= 0b0000_0010;
                            }

                            let funny_index = self.ly() as usize * 160 + self.x as usize;
                            let color = match pixel_to_render.palette {
                                Palette::BGP => palette::map_to_bgp(memory, funny_greyscale),
                                Palette::OBP0 => palette::map_to_obp0(memory, funny_greyscale),
                                Palette::OBP1 => palette::map_to_obp1(memory, funny_greyscale),
                            };
                            
                            // Get the colors in their correct greyscale values.
                            self.funny_buffer_test[funny_index] = 255 - color.into_bits() * 64;

                            self.x += 1;
                        }
                    }
                }

                if drawing_window(memory, self.x, self.ly()) && !self.bg_fetcher.drawing_window {
                    trace!(target: "ppu_window", "Started drawing window at X {}", self.x);
                    self.window_y = self.window_y.wrapping_add(1);
                    self.bg_fetcher = BackgroundFetcher::default();
                    self.bg_fetcher.warmup = false;
                    self.bg_fetcher.drawing_window = true;
                    // Prevent the window from being scrolled by the background scroll (SCX).
                    // https://github.com/Ashiepaws/GBEDG/blob/master/ppu/index.md#scx-at-a-sub-tile-layer
                    self.pixels_to_drop = 0;
                }

                if self.x >= 160 {
                    self.transition_hblank();
                }
            }
            PpuMode::HBlank => {
                if self.dot_counter == self.update_stat_on_dot {
                    lcd_status::set_ppu_mode(memory, 0);
                    self.update_stat_on_dot = usize::MAX;
                }

                if self.dot_counter.is_multiple_of(DOTS_PER_SCANLINE) {
                    if self.ly() < 144 {
                        self.transition_oam_scan();
                    } else {
                        self.transition_vblank(memory);
                    }
                }
            }
            PpuMode::VBlank => {
                if self.dot_counter == self.update_stat_on_dot {
                    lcd_status::set_ppu_mode(memory, 1);
                    self.update_stat_on_dot = usize::MAX;
                }

                if self.dot_counter == DOTS_PER_FRAME {
                    self.dot_counter = 0;
                    self.window_y = 255;
                    self.transition_oam_scan();
                }
            }
        }

        // At the start of every scanline (even offscreen scanlines).
        if self.dot_counter.is_multiple_of(DOTS_PER_SCANLINE) {
            // Update LCD Y coordinate.
            self.update_ly_register(memory);
            self.ly_to_compare_lyc = 255;
        }

        if self.dot_counter % DOTS_PER_SCANLINE == 4 {
            self.ly_to_compare_lyc = memory[io_regs::LY as usize];
        }

        let coincidence = self.ly_to_compare_lyc == memory[io_regs::LYC as usize];
        lcd_status::set_coincidence(memory, coincidence);

        // STAT interrupt triggering.
        let lcy_int = coincidence && lcd_status::lyc_int_select(memory);
        let mode_int = match self.mode {
            PpuMode::HBlank => lcd_status::mode0_int_select(memory),
            PpuMode::VBlank => lcd_status::mode1_int_select(memory),
            PpuMode::OamScan  => lcd_status::mode2_int_select(memory),
            PpuMode::Drawing => false,
        };
        let prev_stat_line = self.stat_interrupt_line;
        self.stat_interrupt_line = lcy_int || mode_int;

        // Low to high transition on the STAT interrupt line.
        if !prev_stat_line && self.stat_interrupt_line {
            // Request the LCD interrupt.
            memory[io_regs::IF as usize] |= 0b0000_0010;
        }

        let dots_this_line = self.dot_counter % DOTS_PER_SCANLINE;
        match dots_this_line {
            0 | 4 | 8 | 12 | 76 | 80 | 84 | 448 | 452 => {
                warn!(
                    target: "ppu_enabled",
                    "Clocks: {}, LY: {}, STAT Mode: {}, LY to compare LYC: {}, Mode INT: {}",
                    self.dot_counter % DOTS_PER_SCANLINE,
                    memory[io_regs::LY as usize],
                    lcd_status::ppu_mode(memory),
                    self.ly_to_compare_lyc,
                    mode_int,
                );
            }
            _ => ()
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
            obj_buffer: VecDeque::with_capacity(10),
            obj_fetcher: ObjectFetcher::default(),
            stat_interrupt_line: false,
            funny_buffer_test: vec![0; 160 * 144],
            disabled: true,
            update_stat_on_dot: usize::MAX,
            ly_to_compare_lyc: 0,
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
        let mut memory = hw_constants::post_boot_hwio();

        while !matches!(ppu.mode, PpuMode::HBlank) {
            ppu.tick(&mut memory);
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
        let mut memory = hw_constants::post_boot_hwio();
        memory[io_regs::SCX as usize] = 7;

        while !matches!(ppu.mode, PpuMode::HBlank) {
            ppu.tick(&mut memory);
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
        let mut memory = hw_constants::post_boot_hwio();

        // Enable the window.
        memory[io_regs::LCDC as usize] |= 0b0010_0000;
        // Scroll it to x=50px
        memory[io_regs::WX as usize] = 50 + 7;

        while !matches!(ppu.mode, PpuMode::HBlank) {
            ppu.tick(&mut memory);
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
        let mut memory = hw_constants::post_boot_hwio();
        memory[io_regs::SCX as usize] = 7;

        // Enable the window.
        memory[io_regs::LCDC as usize] |= 0b0010_0000;
        // Scroll it to x=50px
        memory[io_regs::WX as usize] = 50 + 7;

        while !matches!(ppu.mode, PpuMode::HBlank) {
            ppu.tick(&mut memory);
        }

        let mode_3_dots = ppu.dot_counter - OAM_SCAN_DOTS;
        assert_eq!(mode_3_dots, 185);
    }
}
