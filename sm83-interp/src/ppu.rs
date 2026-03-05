mod bg_fetcher;
pub mod oam;
mod obj_fetcher;
mod palette;
mod registers;
pub mod tiles;

pub use registers::{LcdStatus, StatMode};

use log::{info, trace};
use std::collections::VecDeque;

use hw_constants::io_regs::{IF, LCDC, LY, LYC, SCX, SCY, STAT, WX, WY};
use hw_constants::{
    MEM_MAP_SIZE, OAM_END, OAM_SIZE, OAM_START, PostBoot, SCREEN_SIZE, SCREEN_WIDTH, VRAM_END,
    VRAM_SIZE, VRAM_START,
};

use bg_fetcher::{BackgroundFetcher, FetcherState, Pixel};
use oam::Obj;
use obj_fetcher::{ObjectFetcher, TRANSPARENT};
use palette::Palette;
use registers::LcdControl;

use crate::addressable::Addressable;
use crate::ppu::registers::IoRegisters;

const SCANLINES_PER_FRAME: usize = 154;
const DOTS_PER_SCANLINE: usize = 456;
const DOTS_PER_FRAME: usize = DOTS_PER_SCANLINE * SCANLINES_PER_FRAME;

const OAM_SCAN_DOTS: usize = 80;

// OAM and VRAM access is never "read only", so we represent this state as a ternary value rather than 2 bools for readable and writable.
pub enum PpuMemAccess {
    ReadWrite,
    WriteOnly,
    Blocked,
}

#[derive(Debug, Copy, Clone)]
enum PpuMode {
    HBlank,
    VBlank,
    OamScan,
    Drawing,
}

pub struct Ppu {
    pub dot_counter: usize,
    mode: PpuMode,
    dots_in_mode: usize,
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
    // Only public for debugging reasons.
    pub vram: [u8; VRAM_SIZE as usize],
    pub oam: [u8; OAM_SIZE as usize],
    registers: IoRegisters,
    stat_interrupt_line: bool,
    stat_mode_for_interrupt: u8,
    ly_to_compare_lyc: Option<u8>,
    oam_access: PpuMemAccess,
    vram_access: PpuMemAccess,
    // Buffer of greyscale pixel values, i.e. what the PPU would output to the LCD.
    pub lcd_buffer: Vec<u8>,
    pub disabled: bool,
    just_enabled: bool,
}

fn drawing_window(memory: &[u8; MEM_MAP_SIZE], x: u8, y: u8) -> bool {
    let lcdc = LcdControl::from_bits(memory[LCDC as usize]);

    lcdc.window_enabled() && x + 7 == memory[WX as usize] && y >= memory[WY as usize]
}

fn mix_pixels(bg_pixel: Pixel, obj_pixel: Pixel) -> Pixel {
    let mut render_bg = false;
    render_bg |= !(obj_pixel.low || obj_pixel.high);
    render_bg |= obj_pixel.priority && (bg_pixel.low || bg_pixel.high);

    if render_bg { bg_pixel } else { obj_pixel }
}

impl Ppu {
    #[allow(clippy::cast_possible_truncation)]
    #[must_use]
    pub fn ly(&self) -> u8 {
        (self.dot_counter / DOTS_PER_SCANLINE) as u8
    }

    #[must_use]
    pub fn dots_this_line(&self) -> usize {
        self.dot_counter % DOTS_PER_SCANLINE
    }

    fn pop_next_obj(&mut self) -> Option<Obj> {
        self.obj_buffer.pop_front_if(|obj| {
            // Addition on the left might overflow so we cast to usize.
            obj.x_pos as usize + self.pixels_to_drop as usize <= self.x as usize + 8
        })
    }

    fn transition_hblank(&mut self) {
        self.mode = PpuMode::HBlank;
        self.dots_in_mode = 0;
        trace!(target: "ppu_hblank", "Set to Mode 0 on dot: {}, (Drew for {} dots)", self.dots_this_line(), self.dots_this_line() - OAM_SCAN_DOTS);

        self.x = 0;
        // Reset each of the fetchers.
        self.bg_fetcher = BackgroundFetcher::default();
        self.obj_fetcher = ObjectFetcher::default();
    }

    fn transition_vblank(&mut self) {
        self.mode = PpuMode::VBlank;
        self.dots_in_mode = 0;
        // Update LCD Y coordinate.
        self.update_ly_register();
    }

    fn transition_oam_scan(&mut self) {
        self.mode = PpuMode::OamScan;
        self.dots_in_mode = 0;
        trace!(target: "ppu_oamscan", "Set to Mode 2 on dot: {}", self.dots_this_line());
    }

    fn transition_drawing(&mut self, memory: &mut [u8; MEM_MAP_SIZE]) {
        self.mode = PpuMode::Drawing;
        self.dots_in_mode = 0;

        // This is the last cycle of the OAM scan, so lets actually do the OAM scan.
        let lcdc = LcdControl::from_bits(memory[LCDC as usize]);
        let ly = self.ly();
        oam::oam_scan(&mut self.obj_buffer, &self.oam, lcdc, ly);

        // Prepare for Drawing.
        self.pixels_to_drop = (memory[SCX as usize] & 7) + 8;
    }

    fn update_ly_register(&mut self) {
        self.registers.ly = self.ly();
    }

    fn update_stat_mode(memory: &mut [u8; MEM_MAP_SIZE], mode: StatMode) {
        let stat = LcdStatus::from_bits(memory[STAT as usize]);
        memory[STAT as usize] = stat.with_mode(mode).into();
    }

    pub fn update_stat_interrupt(&mut self, memory: &mut [u8; MEM_MAP_SIZE]) {
        let stat = LcdStatus::from_bits(memory[STAT as usize]);

        let coincidence = self
            .ly_to_compare_lyc
            .is_some_and(|x| x == memory[LYC as usize]);
        memory[STAT as usize] = stat.with_coincidence(coincidence).into();

        // STAT interrupt triggering.
        let lyc_int = coincidence && stat.lyc_int_select();
        let mode_int = match self.stat_mode_for_interrupt {
            0 => stat.mode0_int_select(),
            1 => stat.mode1_int_select(),
            2 => stat.mode2_int_select(),
            _ => false,
        };

        let prev_stat_line = self.stat_interrupt_line;
        self.stat_interrupt_line = lyc_int || mode_int;

        // Low to high transition on the STAT interrupt line.
        if !prev_stat_line && self.stat_interrupt_line {
            info!(target: "lcd_int", "LCD interrupt flag set on dot: {}", self.dots_this_line());
            // Request the LCD interrupt.
            memory[IF as usize] |= 0b0000_0010;
        }
    }

    // Advance the PPU by 1 dot.
    #[allow(clippy::too_many_lines)]
    // Only panics if internal assertions fail, and they never should.
    #[allow(clippy::missing_panics_doc)]
    pub fn tick(&mut self, memory: &mut [u8; MEM_MAP_SIZE]) {
        let lcdc = LcdControl::from_bits(memory[LCDC as usize]);
        if !lcdc.lcd_and_ppu_enabled() {
            if !self.disabled {
                info!(target: "ppu_disabled", "Disabled on dot: {}", self.dot_counter);

                // Reset the PPU state, preserving only VRAM, OAM, and the stat interrupt line.
                *self = Ppu {
                    vram: self.vram,
                    oam: self.oam,
                    stat_interrupt_line: self.stat_interrupt_line,
                    ..Default::default()
                };

                let stat = LcdStatus::from_bits(memory[STAT as usize]);
                memory[STAT as usize] = stat.with_mode(StatMode::HBlank).into();
                self.update_ly_register();
            }
            return;
        }
        if self.disabled {
            self.disabled = false;
            info!(target: "ppu_enabled", "Enabled");
        }

        // Do evil initial line 0 shenanigans.
        // This timing matches GameRoy's PPU implementation.
        if self.just_enabled {
            // Observable 1.
            if self.dot_counter == 0 {
                self.stat_mode_for_interrupt = 0xFF;
                self.update_stat_interrupt(memory);
            }

            // Observable 79.
            if self.dot_counter == 78 {
                self.oam_access = PpuMemAccess::Blocked;
                self.vram_access = PpuMemAccess::Blocked;

                Self::update_stat_mode(memory, StatMode::Drawing);
                self.stat_mode_for_interrupt = 3;
                self.update_stat_interrupt(memory);
            }

            // Observable 84.
            if self.dot_counter == 83 {
                // Skip 5 extra cycles, 84 will be observed as 89.
                self.dot_counter += 5;
            }

            // Observable 251.
            if self.dot_counter == 255 {
                self.oam_access = PpuMemAccess::ReadWrite;
                self.vram_access = PpuMemAccess::ReadWrite;

                Self::update_stat_mode(memory, StatMode::HBlank);
                // Skip 2 extra cycles.
                self.dot_counter += 2;
            }

            self.dot_counter += 1;
            if self.dot_counter == DOTS_PER_SCANLINE {
                self.update_ly_register();
                self.transition_oam_scan();
                self.just_enabled = false;
            }
            return;
        }

        match self.mode {
            PpuMode::OamScan => {
                // Mode 2 signals a mode interrupt 1-Tcycle *before* its bits change in STAT on line 1 onward.
                // See: section 8.11.1 of TCAGBD.
                // Also see cycles.txt based on SameBoy's timing.

                // Observable 3.
                if self.dots_this_line() == 2 {
                    self.oam_access = PpuMemAccess::WriteOnly;

                    if self.ly() == 0 {
                        self.stat_mode_for_interrupt = 0xFF;
                        self.ly_to_compare_lyc = Some(0);
                    } else {
                        self.stat_mode_for_interrupt = 2;
                        self.ly_to_compare_lyc = None;
                    }

                    Self::update_stat_mode(memory, StatMode::HBlank);
                    self.update_stat_interrupt(memory);
                }

                // Observable 4.
                if self.dots_this_line() == 3 {
                    self.oam_access = PpuMemAccess::Blocked;

                    Self::update_stat_mode(memory, StatMode::OamScan);

                    self.ly_to_compare_lyc = Some(self.ly());

                    self.stat_mode_for_interrupt = 2;
                    self.update_stat_interrupt(memory);

                    self.stat_mode_for_interrupt = 0xFF;
                    self.update_stat_interrupt(memory);
                }

                // Observable 80.
                if self.dots_this_line() == 79 {
                    self.oam_access = PpuMemAccess::WriteOnly;
                    self.vram_access = PpuMemAccess::WriteOnly;
                }

                self.dot_counter += 1;
                self.dots_in_mode += 1;
                if self.dots_this_line() == OAM_SCAN_DOTS {
                    self.transition_drawing(memory);
                }
            }
            PpuMode::Drawing => {
                // Observable 84.
                if self.dots_this_line() == 83 {
                    self.oam_access = PpuMemAccess::Blocked;
                    self.vram_access = PpuMemAccess::Blocked;

                    Self::update_stat_mode(memory, StatMode::Drawing);

                    self.stat_mode_for_interrupt = 3;
                    self.update_stat_interrupt(memory);
                }

                if let Some(obj) = self.pop_next_obj() {
                    //println!("DOT: {}, Push obj", self.dots_in_mode);
                    self.obj_fetcher.push_obj(obj);
                }

                if self.obj_fetcher.idle_and_empty()
                    || self.bg_fetcher.bg_fifo.is_empty()
                    || !matches!(
                        self.bg_fetcher.state,
                        FetcherState::BeforeGetTileDataHigh
                            | FetcherState::GetTileDataHigh
                            | FetcherState::Push
                    )
                {
                    let scx = memory[SCX as usize];
                    let scy = memory[SCY as usize];
                    self.bg_fetcher
                        .tick(&self.vram, lcdc, scx, scy, self.ly(), self.window_y);
                } else {
                    self.obj_fetcher.tick(&self.vram, lcdc, self.ly());
                }

                if self.obj_fetcher.idle_and_empty() {
                    // Combine FIFOs.
                    if let Some(bg_pixel) = self.bg_fetcher.shift_out() {
                        let obj_pixel = self.obj_fetcher.shift_out().unwrap_or(TRANSPARENT);

                        if self.pixels_to_drop > 0 {
                            self.pixels_to_drop -= 1;
                        } else {
                            let lcdc = LcdControl::from_bits(memory[LCDC as usize]);

                            // If the background/window is disabled, use a pixel with a value of 0.
                            // See: https://gbdev.io/pandocs/pixel_fifo.html#pixel-rendering
                            let mut pixel_to_render = if lcdc.bg_and_window_enabled() {
                                bg_pixel
                            } else {
                                Pixel {
                                    low: false,
                                    high: false,
                                    palette: Palette::Bgp,
                                    priority: false,
                                }
                            };

                            if lcdc.obj_enabled() {
                                pixel_to_render = mix_pixels(pixel_to_render, obj_pixel);
                            }

                            let mut funny_greyscale = 0;
                            if pixel_to_render.low {
                                funny_greyscale |= 0b0000_0001;
                            }
                            if pixel_to_render.high {
                                funny_greyscale |= 0b0000_0010;
                            }

                            let lcd_row = self.ly() as usize * SCREEN_WIDTH as usize;
                            let lcd_pixel_index = lcd_row + self.x as usize;
                            let color = match pixel_to_render.palette {
                                Palette::Bgp => palette::map_to_bgp(memory, funny_greyscale),
                                Palette::Obp0 => palette::map_to_obp0(memory, funny_greyscale),
                                Palette::Obp1 => palette::map_to_obp1(memory, funny_greyscale),
                            };

                            // Get the colors in their correct greyscale values.
                            self.lcd_buffer[lcd_pixel_index] = 255 - color.into_bits() * 64;

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

                self.dot_counter += 1;
                self.dots_in_mode += 1;
                // If we've finished drawing this line, then transition to the HBlank state.
                if self.x == SCREEN_WIDTH {
                    self.transition_hblank();
                }
                assert!(self.x <= SCREEN_WIDTH);
            }
            PpuMode::HBlank => {
                // Observable 4 dots into HBlank, or 256 with the shortest mode 3.
                if self.dots_in_mode == 3 {
                    self.oam_access = PpuMemAccess::ReadWrite;
                    self.vram_access = PpuMemAccess::ReadWrite;

                    Self::update_stat_mode(memory, StatMode::HBlank);
                    self.stat_mode_for_interrupt = 0;
                    self.update_stat_interrupt(memory);
                }

                self.dot_counter += 1;
                self.dots_in_mode += 1;
                if self.dot_counter.is_multiple_of(DOTS_PER_SCANLINE) {
                    if self.ly() == 144 {
                        self.transition_vblank();
                        self.update_ly_register();
                        self.ly_to_compare_lyc = None;
                    } else {
                        // Update LCD Y coordinate.
                        self.update_ly_register();
                        self.transition_oam_scan();
                    }
                }
            }
            PpuMode::VBlank => {
                // TODO: Observable 2.

                // Last line
                if self.ly() == 153 {
                    // Observable 6.
                    if self.dots_this_line() == 5 {
                        // Force LY I/O register to 0 early.
                        self.registers.ly = 0;
                        self.ly_to_compare_lyc = Some(153);
                        self.update_stat_interrupt(memory);
                    }

                    // Observable 12.
                    if self.dots_this_line() == 11 {
                        self.ly_to_compare_lyc = Some(0);
                        self.update_stat_interrupt(memory);
                    }
                } else {
                    // Observable 4.
                    if self.dots_this_line() == 3 {
                        self.ly_to_compare_lyc = Some(self.ly());
                        if self.ly() == 144 {
                            Self::update_stat_mode(memory, StatMode::VBlank);
                            // Request the VBlank interrupt.
                            memory[IF as usize] |= 0b0000_0001;

                            // A VBlank also triggers as an OAM Scan... for some reason?
                            // See: https://github.com/Gekkio/mooneye-test-suite/blob/main/acceptance/ppu/vblank_stat_intr-GS.s
                            self.stat_mode_for_interrupt = 2;
                            self.update_stat_interrupt(memory);
                            self.stat_mode_for_interrupt = 1;
                        }
                        self.update_stat_interrupt(memory);
                    }
                }

                self.dot_counter += 1;
                if self.dot_counter == DOTS_PER_FRAME {
                    self.dot_counter = 0;
                    self.window_y = 255;
                    self.transition_oam_scan();
                }
                if self.dots_this_line() == 0 {
                    // Update LCD Y coordinate.
                    self.update_ly_register();
                }
            }
        }

        match self.dots_this_line() {
            0 | 4 | 8 | 12 | 76 | 80 | 84 | 448 | 452 => {
                trace!(
                    target: "ppu_enabled",
                    "Clocks: {:3}, LY: {:3}, STAT Mode: {}, LY to compare LYC: {:?}, INT: {}",
                    self.dots_this_line(),
                    self.registers.ly,
                    LcdStatus::from_bits(memory[STAT as usize]).mode().into_bits(),
                    self.ly_to_compare_lyc,
                    self.stat_interrupt_line,
                );
            }
            _ => (),
        }
    }
}

impl Addressable for Ppu {
    fn read_byte(&self, index: u16) -> u8 {
        match index {
            VRAM_START..VRAM_END => match self.vram_access {
                PpuMemAccess::ReadWrite => self.vram[(index - VRAM_START) as usize],
                _ => 0xFF,
            },
            OAM_START..OAM_END => match self.oam_access {
                PpuMemAccess::ReadWrite => self.oam[(index - OAM_START) as usize],
                _ => 0xFF,
            },
            LY => self.registers.ly,
            _ => unreachable!(),
        }
    }

    fn write_byte(&mut self, index: u16, value: u8) {
        match index {
            // Ignore writes to VRAM when access is blocked.
            VRAM_START..VRAM_END => match self.vram_access {
                PpuMemAccess::Blocked => (),
                _ => self.vram[(index - VRAM_START) as usize] = value,
            },
            // Ignore writes to OAM when access is blocked.
            OAM_START..OAM_END => match self.oam_access {
                PpuMemAccess::Blocked => (),
                _ => self.oam[(index - OAM_START) as usize] = value,
            },
            _ => unreachable!(),
        }
    }
}

impl Default for Ppu {
    fn default() -> Self {
        Self {
            dot_counter: 0,
            mode: PpuMode::HBlank,
            dots_in_mode: 0,
            x: 0,
            pixels_to_drop: 0,
            window_y: 255,
            bg_fetcher: BackgroundFetcher::default(),
            obj_buffer: VecDeque::with_capacity(10),
            obj_fetcher: ObjectFetcher::default(),
            vram: [0; VRAM_SIZE as usize],
            oam: [0; OAM_SIZE as usize],
            registers: IoRegisters::default(),
            stat_interrupt_line: false,
            stat_mode_for_interrupt: 0xFF,
            ly_to_compare_lyc: Some(0),
            oam_access: PpuMemAccess::ReadWrite,
            vram_access: PpuMemAccess::ReadWrite,
            lcd_buffer: vec![0; SCREEN_SIZE],
            disabled: true,
            just_enabled: true,
        }
    }
}

impl PostBoot for Ppu {
    fn post_boot_dmg() -> Self {
        Self {
            dot_counter: DOTS_PER_FRAME - 54,
            mode: PpuMode::VBlank,
            dots_in_mode: 0,
            x: 0,
            pixels_to_drop: 0,
            window_y: 255,
            bg_fetcher: BackgroundFetcher::default(),
            obj_buffer: VecDeque::with_capacity(10),
            obj_fetcher: ObjectFetcher::default(),
            vram: [0; VRAM_SIZE as usize],
            oam: [0; OAM_SIZE as usize],
            registers: IoRegisters::default(),
            stat_interrupt_line: false,
            stat_mode_for_interrupt: 1,
            ly_to_compare_lyc: Some(0),
            oam_access: PpuMemAccess::ReadWrite,
            vram_access: PpuMemAccess::ReadWrite,
            lcd_buffer: vec![0; SCREEN_SIZE],
            disabled: false,
            just_enabled: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use hw_constants::io_regs::LCDC;

    use super::*;

    // Assert that the minimum Mode 3 length (172) with:
    // - unscrolled background tiles (0)
    // - no window (0)
    // - no objects (0)
    // is 172 dots.
    // See: https://gbdev.io/pandocs/Rendering.html#mode-3-length
    #[test]
    fn test_minimum_bg_mode_3_dots() {
        let mut ppu = Ppu::post_boot_dmg();
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
        let mut ppu = Ppu::post_boot_dmg();
        let mut memory = hw_constants::post_boot_hwio();
        memory[SCX as usize] = 7;

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
        let mut ppu = Ppu::post_boot_dmg();
        let mut memory = hw_constants::post_boot_hwio();

        // Enable the window.
        memory[LCDC as usize] |= 0b0010_0000;
        // Scroll it to x=50px
        memory[WX as usize] = 50 + 7;

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
        let mut ppu = Ppu::post_boot_dmg();
        let mut memory = hw_constants::post_boot_hwio();
        memory[SCX as usize] = 7;

        // Enable the window.
        memory[LCDC as usize] |= 0b0010_0000;
        // Scroll it to x=50px
        memory[WX as usize] = 50 + 7;

        while !matches!(ppu.mode, PpuMode::HBlank) {
            ppu.tick(&mut memory);
        }

        let mode_3_dots = ppu.dot_counter - OAM_SCAN_DOTS;
        assert_eq!(mode_3_dots, 185);
    }

    // Assert that the minimum Mode 3 length (172) with:
    // - unscrolled background tiles (0)
    // - no window (0)
    // - 1 object at position x=0 (11)
    // is 183 dots.
    // See: https://gbdev.io/pandocs/Rendering.html#mode-3-length
    #[test]
    fn test_bg_obj_x_0_mode_3_dots() {
        let mut ppu = Ppu::post_boot_dmg();
        let mut memory = hw_constants::post_boot_hwio();
        ppu.oam[0x00] = 16; // OBJ Y
        ppu.oam[0x01] = 0; // OBJ X
        memory[LCDC as usize] = 0x93; // Enable OBJs.

        while !matches!(ppu.mode, PpuMode::HBlank) {
            ppu.tick(&mut memory);
        }

        let mode_3_dots = ppu.dot_counter - OAM_SCAN_DOTS;
        assert_eq!(mode_3_dots, 183);
    }

    // Assert that the minimum Mode 3 length (172) with:
    // - unscrolled background tiles (0)
    // - no window (0)
    // - 2 object at position x=0 (11+6)
    // is 189 dots.
    // See: https://gbdev.io/pandocs/Rendering.html#mode-3-length
    #[test]
    fn test_bg_2_obj_x_0_mode_3_dots() {
        let mut ppu = Ppu::post_boot_dmg();
        let mut memory = hw_constants::post_boot_hwio();
        ppu.oam[0x00] = 16; // OBJ Y
        ppu.oam[0x01] = 0; // OBJ X
        ppu.oam[0x04] = 16; // OBJ Y
        ppu.oam[0x05] = 0; // OBJ X
        memory[LCDC as usize] = 0x93; // Enable OBJs.

        while !matches!(ppu.mode, PpuMode::HBlank) {
            ppu.tick(&mut memory);
        }

        let mode_3_dots = ppu.dot_counter - OAM_SCAN_DOTS;
        assert_eq!(mode_3_dots, 189);
    }

    // Assert that the minimum Mode 3 length (172) with:
    // - unscrolled background tiles (0)
    // - no window (0)
    // - 10 object at position x=1 (64)
    // is 236 dots.
    // See: https://gbdev.io/pandocs/Rendering.html#mode-3-length
    #[test]
    fn test_bg_10_obj_x_1_mode_3_dots() {
        let mut ppu = Ppu::post_boot_dmg();
        let mut memory = hw_constants::post_boot_hwio();
        for i in 0..10 {
            let obj_idx = i * 4;
            ppu.oam[obj_idx] = 16; // OBJ Y
            ppu.oam[obj_idx + 1] = 1; // OBJ X
        }
        memory[LCDC as usize] = 0x93; // Enable OBJs.

        while !matches!(ppu.mode, PpuMode::HBlank) {
            ppu.tick(&mut memory);
        }

        let mode_3_dots = ppu.dot_counter - OAM_SCAN_DOTS;
        assert_eq!(mode_3_dots, 236);
    }

    // Assert that the minimum Mode 3 length (172) with:
    // - unscrolled background tiles (0)
    // - no window (0)
    // - 1 object at position x=2 (9)
    // is 181 dots.
    // See: https://gbdev.io/pandocs/Rendering.html#mode-3-length
    #[test]
    fn test_bg_obj_x_2_mode_3_dots() {
        let mut ppu = Ppu::post_boot_dmg();
        let mut memory = hw_constants::post_boot_hwio();
        ppu.oam[0x00] = 16; // OBJ Y
        ppu.oam[0x01] = 2; // OBJ X
        memory[LCDC as usize] = 0x93; // Enable OBJs.

        while !matches!(ppu.mode, PpuMode::HBlank) {
            ppu.tick(&mut memory);
        }

        let mode_3_dots = ppu.dot_counter - OAM_SCAN_DOTS;
        assert_eq!(mode_3_dots, 181);
    }

    // Assert that the minimum Mode 3 length (172) with:
    // - unscrolled background tiles (0)
    // - no window (0)
    // - 1 object at position x=8 (11)
    // is 183 dots.
    // See: https://gbdev.io/pandocs/Rendering.html#mode-3-length
    #[test]
    fn test_bg_obj_x_8_mode_3_dots() {
        let mut ppu = Ppu::post_boot_dmg();
        let mut memory = hw_constants::post_boot_hwio();
        ppu.oam[0x00] = 16; // OBJ Y
        ppu.oam[0x01] = 8; // OBJ X
        memory[LCDC as usize] = 0x93; // Enable OBJs.

        while !matches!(ppu.mode, PpuMode::HBlank) {
            ppu.tick(&mut memory);
        }

        let mode_3_dots = ppu.dot_counter - OAM_SCAN_DOTS;
        assert_eq!(mode_3_dots, 183);
    }

    // Assert that the minimum Mode 3 length (172) with:
    // - unscrolled background tiles (0)
    // - no window (0)
    // - 1 object at position x=9 (10)
    // is 182 dots.
    // See: https://gbdev.io/pandocs/Rendering.html#mode-3-length
    #[test]
    fn test_bg_obj_x_9_mode_3_dots() {
        let mut ppu = Ppu::post_boot_dmg();
        let mut memory = hw_constants::post_boot_hwio();
        ppu.oam[0x00] = 16; // OBJ Y
        ppu.oam[0x01] = 9; // OBJ X
        memory[LCDC as usize] = 0x93; // Enable OBJs.

        while !matches!(ppu.mode, PpuMode::HBlank) {
            ppu.tick(&mut memory);
        }

        let mode_3_dots = ppu.dot_counter - OAM_SCAN_DOTS;
        assert_eq!(mode_3_dots, 182);
    }
}
