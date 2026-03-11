pub mod oam;
pub mod tiles;

mod bg_fetcher;
mod obj_fetcher;
mod palette;
mod registers;

pub use registers::{LcdControl, LcdStatus, StatMode};

use log::{info, trace};
use std::collections::VecDeque;

use hw_constants::io_regs::{BGP, LCDC, LY, LYC, OBP0, OBP1, SCX, SCY, STAT, WX, WY};
use hw_constants::{
    OAM_END, OAM_SIZE, OAM_START, PostBoot, SCREEN_SIZE, SCREEN_WIDTH, VRAM_END, VRAM_SIZE,
    VRAM_START,
};

use bg_fetcher::{BackgroundFetcher, FetcherState, Pixel};
use oam::Obj;
use obj_fetcher::{ObjectFetcher, TRANSPARENT};
use palette::PaletteSelect;

use crate::addressable::Addressable;
use crate::cpu::InterruptBits;
use crate::ppu::registers::IoRegisters;

const SCANLINES_PER_FRAME: u32 = 154;
const DOTS_PER_SCANLINE: u16 = 456;
const DOTS_PER_FRAME: u32 = DOTS_PER_SCANLINE as u32 * SCANLINES_PER_FRAME;

const OAM_SCAN_DOTS: u32 = 80;

// OAM and VRAM access is never "read only", so we represent this state as a ternary value rather than 2 bools for readable and writable.
pub enum PpuMemAccess {
    ReadWrite,
    WriteOnly,
    Blocked,
}

#[derive(Debug, Copy, Clone)]
enum PpuMode {
    Disabled,
    JustEnabled,
    JustEnabled2,
    JustEnabled3,
    JustEnabled4,
    JustEnabled5,
    JustEnabled6,
    HBlank,
    HBlank2,
    HBlank3,
    VBlank,
    VBlank2,
    VBlank3,
    LastLine,
    LastLine2,
    LastLine3,
    LastLine4,
    OamScan,
    OamScan2,
    OamScan3,
    OamScan4,
    Drawing,
    DrawingCoarse,
}

pub struct Ppu {
    dots_this_line: u16,
    line_number: u8,
    mode: PpuMode,
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
    pub clock: usize,
    pub next_vblank_interrupt: usize,
    pub next_lcd_interrupt: usize,
}

fn mix_pixels(bg_pixel: Pixel, obj_pixel: Pixel) -> Pixel {
    let mut render_bg = false;
    render_bg |= !(obj_pixel.low || obj_pixel.high);
    render_bg |= obj_pixel.priority && (bg_pixel.low || bg_pixel.high);

    if render_bg { bg_pixel } else { obj_pixel }
}

impl Ppu {
    // TODO: Implement drawing the window and sprites.
    // TODO: Fix jerky background scrolling.
    fn coarse_scanline(&mut self) {
        let ly = self.line_number.wrapping_add(self.registers.scy);

        let line_start = self.line_number as usize * SCREEN_WIDTH as usize;
        let line_end = line_start + SCREEN_WIDTH as usize;
        let scanline = &mut self.lcd_buffer[line_start..line_end];

        let mut tile_x = 0;

        let tile_y_idx = ly / 8;

        while tile_x * 8 < 168 {
            let tile_map = if self.registers.lcdc.bg_tile_map() {
                tiles::tile_map_1(&self.vram)
            } else {
                tiles::tile_map_0(&self.vram)
            };

            let tile_x_idx = ((self.registers.scx / 8) + tile_x) & 0x1F;

            let tile_id = tile_map[tile_y_idx as usize * 32 + tile_x_idx as usize];
            let tile_line = ly % 8;

            let tile_data = if self.registers.lcdc.bg_and_window_tiles() {
                tiles::unsigned_nth_tile(&self.vram, tile_id as usize)
            } else {
                tiles::signed_nth_tile(&self.vram, tile_id.cast_signed() as isize)
            };

            let tile_data_low = tile_data[tile_line as usize * 2];
            let tile_data_high = tile_data[tile_line as usize * 2 + 1];

            // Push
            for nth_bit in 0..8 {
                let pixel = Pixel {
                    low: (tile_data_low >> nth_bit) & 1 == 1,
                    high: (tile_data_high >> nth_bit) & 1 == 1,
                    palette: PaletteSelect::Bgp,
                    priority: false,
                };

                let mut funny_greyscale = 0;
                if pixel.low {
                    funny_greyscale |= 0b0000_0001;
                }
                if pixel.high {
                    funny_greyscale |= 0b0000_0010;
                }

                let color = palette::map_to_palette(self.registers.bgp, funny_greyscale);

                let scrolled_left = self.registers.scx & 7;
                let pixel_index =
                    (tile_x as usize * 8 + (7 - nth_bit)).saturating_sub(scrolled_left as usize);
                if pixel_index < SCREEN_WIDTH as usize {
                    // Get the colors in their correct greyscale values.
                    scanline[pixel_index] = 255 - color.into_bits() * 64;
                }
            }

            tile_x += 1;
        }
    }

    pub fn catch_up(&mut self, cpu_clock: usize, interrupt_flags: &mut u8) {
        // Make the PPU catch up to the CPU!
        while self.clock < cpu_clock {
            match self.mode {
                PpuMode::Disabled => self.clock = cpu_clock,
                // Do evil initial line 0 shenanigans.
                // This timing matches GameRoy's PPU implementation.
                PpuMode::JustEnabled => {
                    // Observable 1.
                    self.stat_mode_for_interrupt = 0xFF;
                    self.update_stat_interrupt(interrupt_flags);

                    self.clock += 78;
                    self.dots_this_line += 78;
                    self.mode = PpuMode::JustEnabled2;
                }
                PpuMode::JustEnabled2 => {
                    // Observable 79.
                    self.oam_access = PpuMemAccess::Blocked;
                    self.vram_access = PpuMemAccess::Blocked;

                    self.update_stat_mode(StatMode::Drawing);
                    self.stat_mode_for_interrupt = 3;
                    self.update_stat_interrupt(interrupt_flags);

                    self.clock += 172;
                    self.dots_this_line += 172;
                    self.mode = PpuMode::JustEnabled3;
                }
                PpuMode::JustEnabled3 => {
                    // Observable 84.
                    // Skip 5 extra cycles, 84 will be observed as 89.
                    self.dots_this_line += 5;
                    self.mode = PpuMode::JustEnabled4;
                }
                PpuMode::JustEnabled4 => {
                    // Observable 251.
                    self.oam_access = PpuMemAccess::ReadWrite;
                    self.vram_access = PpuMemAccess::ReadWrite;

                    self.update_stat_mode(StatMode::HBlank);
                    self.clock += 198;
                    self.dots_this_line += 198;
                    self.mode = PpuMode::JustEnabled5;
                }
                PpuMode::JustEnabled5 => {
                    // Skip 3 extra cycles.
                    self.dots_this_line = 0;
                    self.line_number += 1;
                    self.mode = PpuMode::JustEnabled6;
                }
                PpuMode::JustEnabled6 => {
                    self.update_ly_register();
                    self.transition_oam_scan();
                    // TEMP: needed for mixed tick and catch up so we don't instantly go to OAM.
                    self.clock += 1;
                }

                PpuMode::OamScan => {
                    self.clock += 2;
                    self.dots_this_line += 2;

                    self.mode = PpuMode::OamScan2;
                }
                PpuMode::OamScan2 => {
                    // Observable 3.
                    self.oam_access = PpuMemAccess::WriteOnly;

                    // Mode 2 signals a mode interrupt 1-Tcycle *before* its bits change in STAT on line 1 onward.
                    // See: section 8.11.1 of TCAGBD.
                    // Also see cycles.txt based on SameBoy's timing.
                    if self.line_number == 0 {
                        self.stat_mode_for_interrupt = 0xFF;
                        self.ly_to_compare_lyc = Some(0);
                    } else {
                        self.stat_mode_for_interrupt = 2;
                        self.ly_to_compare_lyc = None;
                    }

                    self.update_stat_mode(StatMode::HBlank);
                    self.update_stat_interrupt(interrupt_flags);

                    self.clock += 1;
                    self.dots_this_line += 1;
                    self.mode = PpuMode::OamScan3;
                }
                PpuMode::OamScan3 => {
                    // Observable 4.
                    self.oam_access = PpuMemAccess::Blocked;

                    self.update_stat_mode(StatMode::OamScan);

                    self.ly_to_compare_lyc = Some(self.line_number);

                    self.stat_mode_for_interrupt = 2;
                    self.update_stat_interrupt(interrupt_flags);

                    self.stat_mode_for_interrupt = 0xFF;
                    self.update_stat_interrupt(interrupt_flags);

                    self.clock += 76;
                    self.dots_this_line += 76;
                    self.mode = PpuMode::OamScan4;
                }

                PpuMode::OamScan4 => {
                    // Observable 80.
                    self.oam_access = PpuMemAccess::WriteOnly;
                    self.vram_access = PpuMemAccess::WriteOnly;

                    self.clock += 1;
                    self.dots_this_line += 1;
                    self.transition_drawing();

                    // Using DOTS_PER_SCANLINE is wayyy too conservative, but it's a start.
                    if cpu_clock > self.clock && cpu_clock - self.clock > DOTS_PER_SCANLINE as usize
                    {
                        self.mode = PpuMode::DrawingCoarse;
                    }
                }
                PpuMode::Drawing => {
                    // Observable 84.
                    if self.dots_this_line == 83 {
                        self.oam_access = PpuMemAccess::Blocked;
                        self.vram_access = PpuMemAccess::Blocked;

                        self.update_stat_mode(StatMode::Drawing);

                        self.stat_mode_for_interrupt = 3;
                        self.update_stat_interrupt(interrupt_flags);
                    }

                    if let Some(obj) = self.pop_next_obj() {
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
                        self.bg_fetcher.tick(
                            &self.vram,
                            self.registers.lcdc,
                            self.registers.scx,
                            self.registers.scy,
                            self.line_number,
                            self.window_y,
                        );
                    } else {
                        self.obj_fetcher
                            .tick(&self.vram, self.registers.lcdc, self.line_number);
                    }

                    if self.obj_fetcher.idle_and_empty() {
                        // Combine FIFOs.
                        if let Some(bg_pixel) = self.bg_fetcher.shift_out() {
                            let obj_pixel = self.obj_fetcher.shift_out().unwrap_or(TRANSPARENT);

                            if self.pixels_to_drop > 0 {
                                self.pixels_to_drop -= 1;
                            } else {
                                // If the background/window is disabled, use a pixel with a value of 0.
                                // See: https://gbdev.io/pandocs/pixel_fifo.html#pixel-rendering
                                let mut pixel_to_render =
                                    if self.registers.lcdc.bg_and_window_enabled() {
                                        bg_pixel
                                    } else {
                                        Pixel {
                                            low: false,
                                            high: false,
                                            palette: PaletteSelect::Bgp,
                                            priority: false,
                                        }
                                    };

                                if self.registers.lcdc.obj_enabled() {
                                    pixel_to_render = mix_pixels(pixel_to_render, obj_pixel);
                                }

                                let mut funny_greyscale = 0;
                                if pixel_to_render.low {
                                    funny_greyscale |= 0b0000_0001;
                                }
                                if pixel_to_render.high {
                                    funny_greyscale |= 0b0000_0010;
                                }

                                let lcd_row = self.line_number as usize * SCREEN_WIDTH as usize;
                                let lcd_pixel_index = lcd_row + self.x as usize;
                                let palette = match pixel_to_render.palette {
                                    PaletteSelect::Bgp => self.registers.bgp,
                                    PaletteSelect::Obp0 => self.registers.obp0,
                                    PaletteSelect::Obp1 => self.registers.obp1,
                                };
                                let color = palette::map_to_palette(palette, funny_greyscale);

                                // Get the colors in their correct greyscale values.
                                self.lcd_buffer[lcd_pixel_index] = 255 - color.into_bits() * 64;

                                self.x += 1;
                            }
                        }
                    }

                    if self.drawing_window() && !self.bg_fetcher.drawing_window {
                        #[cfg(feature = "ppu-logging")]
                        trace!(target: "ppu_window", "Started drawing window at X {}", self.x);
                        self.window_y = self.window_y.wrapping_add(1);
                        self.bg_fetcher = BackgroundFetcher::default();
                        self.bg_fetcher.warmup = false;
                        self.bg_fetcher.drawing_window = true;
                        // Prevent the window from being scrolled by the background scroll (SCX).
                        // https://github.com/Ashiepaws/GBEDG/blob/master/ppu/index.md#scx-at-a-sub-tile-layer
                        self.pixels_to_drop = 0;
                    }

                    self.clock += 1;
                    self.dots_this_line += 1;
                    // If we've finished drawing this line, then transition to the HBlank state.
                    if self.x == SCREEN_WIDTH {
                        self.transition_hblank();
                    }
                    assert!(self.x <= SCREEN_WIDTH);
                }
                PpuMode::DrawingCoarse => {
                    self.oam_access = PpuMemAccess::Blocked;
                    self.vram_access = PpuMemAccess::Blocked;

                    self.update_stat_mode(StatMode::Drawing);

                    self.stat_mode_for_interrupt = 3;
                    self.update_stat_interrupt(interrupt_flags);

                    self.coarse_scanline();
                    // TODO: Actually compute this line length beforehand and use it to determine whether we can enter "DrawingCoarse" to begin with.
                    // Right now it's hard coded, which is very wrong.
                    self.clock += 172;
                    self.dots_this_line += 172;
                    self.transition_hblank();
                }

                PpuMode::HBlank => {
                    self.clock += 3;
                    self.dots_this_line += 3;
                    self.mode = PpuMode::HBlank2;
                }
                PpuMode::HBlank2 => {
                    // Observable 4 dots into HBlank, or 256 with the shortest mode 3.
                    self.oam_access = PpuMemAccess::ReadWrite;
                    self.vram_access = PpuMemAccess::ReadWrite;

                    self.update_stat_mode(StatMode::HBlank);
                    self.stat_mode_for_interrupt = 0;
                    self.update_stat_interrupt(interrupt_flags);

                    let dots_remaining_in_scanline = DOTS_PER_SCANLINE - self.dots_this_line;

                    self.clock += dots_remaining_in_scanline as usize - 1;
                    self.dots_this_line = 0;
                    self.line_number += 1;
                    self.mode = PpuMode::HBlank3;
                }
                PpuMode::HBlank3 => {
                    if self.line_number == 144 {
                        self.transition_vblank();
                        self.update_ly_register();
                        self.ly_to_compare_lyc = None;
                    } else {
                        // Update LCD Y coordinate.
                        self.update_ly_register();
                        self.transition_oam_scan();
                    }

                    // TEMP: needed for mixed tick and catch up so we don't instantly go to OAM.
                    self.clock += 1;
                }
                PpuMode::VBlank => {
                    self.clock += 3;
                    self.dots_this_line += 3;
                    self.mode = PpuMode::VBlank2;
                }
                // TODO: Observable 2.
                PpuMode::VBlank2 => {
                    // Observable 4.
                    self.ly_to_compare_lyc = Some(self.line_number);
                    if self.line_number == 144 {
                        self.update_stat_mode(StatMode::VBlank);
                        // Request the VBlank interrupt.
                        *interrupt_flags |= 0b0000_0001;

                        // A VBlank also triggers as an OAM Scan... for some reason?
                        // See: https://github.com/Gekkio/mooneye-test-suite/blob/main/acceptance/ppu/vblank_stat_intr-GS.s
                        self.stat_mode_for_interrupt = 2;
                        self.update_stat_interrupt(interrupt_flags);
                        self.stat_mode_for_interrupt = 1;
                    }
                    self.update_stat_interrupt(interrupt_flags);

                    self.clock += 452;
                    self.dots_this_line += 452;
                    self.mode = PpuMode::VBlank3;
                }
                PpuMode::VBlank3 => {
                    self.clock += 1;
                    self.dots_this_line = 0;
                    self.line_number += 1;
                    if self.line_number == 153 {
                        self.mode = PpuMode::LastLine;
                    } else {
                        self.mode = PpuMode::VBlank;
                    }
                    // Update LCD Y coordinate.
                    self.update_ly_register();
                }
                PpuMode::LastLine => {
                    self.clock += 5;
                    self.dots_this_line += 5;
                    self.mode = PpuMode::LastLine2;
                }
                PpuMode::LastLine2 => {
                    // Observable 6.
                    // Force LY I/O register to 0 early.
                    self.registers.ly = 0;
                    self.ly_to_compare_lyc = Some(153);
                    self.update_stat_interrupt(interrupt_flags);

                    self.clock += 6;
                    self.dots_this_line += 6;
                    self.mode = PpuMode::LastLine3;
                }
                PpuMode::LastLine3 => {
                    // Observable 12.
                    self.ly_to_compare_lyc = Some(0);
                    self.update_stat_interrupt(interrupt_flags);

                    self.clock += 445;
                    self.dots_this_line += 445;
                    self.mode = PpuMode::LastLine4;
                }
                PpuMode::LastLine4 => {
                    self.dots_this_line = 0;
                    self.line_number = 0;
                    self.window_y = 255;
                    self.transition_oam_scan();
                }
            }
        }
    }

    #[must_use]
    pub fn predict_next_interrupt(&mut self, cpu_clock: usize, ie: InterruptBits) -> usize {
        self.next_vblank_interrupt = if ie.vblank() {
            // VBlank always happens on this dot.
            let vblank_dot = (DOTS_PER_SCANLINE as isize * 144) + 4;
            let mut dots_from_vblank = vblank_dot
                - (DOTS_PER_SCANLINE as isize * self.line_number as isize)
                + self.dots_this_line as isize;
            if dots_from_vblank.is_negative() {
                dots_from_vblank = DOTS_PER_FRAME as isize + dots_from_vblank;
            }
            cpu_clock + dots_from_vblank as usize
        } else {
            usize::MAX
        };
        self.next_lcd_interrupt = if ie.lcd() { cpu_clock } else { usize::MAX };

        self.next_vblank_interrupt.min(self.next_lcd_interrupt)
        // TODO: actual prediction...
    }

    #[must_use]
    pub fn is_disabled(&self) -> bool {
        matches!(self.mode, PpuMode::Disabled)
    }

    fn drawing_window(&self) -> bool {
        self.registers.lcdc.window_enabled()
            && self.x + 7 == self.registers.wx
            && self.line_number >= self.registers.wy
    }

    fn pop_next_obj(&mut self) -> Option<Obj> {
        self.obj_buffer.pop_front_if(|obj| {
            // Addition on the left might overflow so we cast to usize.
            obj.x_pos as usize + self.pixels_to_drop as usize <= self.x as usize + 8
        })
    }

    fn transition_hblank(&mut self) {
        self.mode = PpuMode::HBlank;
        #[cfg(feature = "ppu-logging")]
        trace!(target: "ppu_hblank", "Set to Mode 0 on dot: {}, (Drew for {} dots)", self.dots_this_line(), self.dots_this_line() - OAM_SCAN_DOTS);

        self.x = 0;
        // Reset each of the fetchers.
        self.bg_fetcher = BackgroundFetcher::default();
        self.obj_fetcher = ObjectFetcher::default();
    }

    fn transition_vblank(&mut self) {
        self.mode = PpuMode::VBlank;
        // Update LCD Y coordinate.
        self.update_ly_register();
    }

    fn transition_oam_scan(&mut self) {
        self.mode = PpuMode::OamScan;
        #[cfg(feature = "ppu-logging")]
        trace!(target: "ppu_oamscan", "Set to Mode 2 on dot: {}", self.dots_this_line());
    }

    fn transition_drawing(&mut self) {
        self.mode = PpuMode::Drawing;

        // This is the last cycle of the OAM scan, so lets actually do the OAM scan.
        let ly = self.line_number;
        oam::oam_scan(&mut self.obj_buffer, &self.oam, self.registers.lcdc, ly);

        // Prepare for Drawing.
        self.pixels_to_drop = (self.registers.scx & 7) + 8;
    }

    fn update_ly_register(&mut self) {
        self.registers.ly = self.line_number;
    }

    fn update_stat_mode(&mut self, mode: StatMode) {
        self.registers.stat.set_mode(mode);
    }

    pub fn update_stat_interrupt(&mut self, interrupt_flags: &mut u8) {
        let coincidence = self
            .ly_to_compare_lyc
            .is_some_and(|x| x == self.registers.lyc);
        self.registers.stat.set_coincidence(coincidence);

        // STAT interrupt triggering.
        let lyc_int = coincidence && self.registers.stat.lyc_int_select();
        let mode_int = match self.stat_mode_for_interrupt {
            0 => self.registers.stat.mode0_int_select(),
            1 => self.registers.stat.mode1_int_select(),
            2 => self.registers.stat.mode2_int_select(),
            _ => false,
        };

        let prev_stat_line = self.stat_interrupt_line;
        self.stat_interrupt_line = lyc_int || mode_int;

        // Low to high transition on the STAT interrupt line.
        if !prev_stat_line && self.stat_interrupt_line {
            #[cfg(feature = "ppu-logging")]
            info!(target: "lcd_int", "LCD interrupt flag set on dot: {}", self.dots_this_line());
            // Request the LCD interrupt.
            *interrupt_flags |= 0b0000_0010;
        }
    }

    fn disable(&mut self) {
        if !self.is_disabled() {
            #[cfg(feature = "ppu-logging")]
            info!(target: "ppu_disabled", "Disabled on dot: {}", self.dot_counter);

            // Reset the PPU state, preserving only some of its state.
            *self = Ppu {
                vram: self.vram,
                oam: self.oam,
                registers: self.registers,
                stat_interrupt_line: self.stat_interrupt_line,
                clock: self.clock,
                next_vblank_interrupt: self.clock,
                next_lcd_interrupt: self.next_lcd_interrupt,
                ..Default::default()
            };

            self.registers.stat.set_mode(StatMode::HBlank);
            self.update_ly_register();
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
            LCDC => self.registers.lcdc.into(),
            STAT => self.registers.stat.into(),
            SCY => self.registers.scy,
            SCX => self.registers.scx,
            LY => self.registers.ly,
            LYC => self.registers.lyc,
            BGP => self.registers.bgp.into(),
            OBP0 => self.registers.obp0.into(),
            OBP1 => self.registers.obp1.into(),
            WY => self.registers.wy,
            WX => self.registers.wx,
            _ => unreachable!(),
        }
    }

    fn write_byte(&mut self, index: u16, value: u8, cpu_clock: usize) {
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
            LCDC => {
                self.registers.lcdc = value.into();
                if self.registers.lcdc.lcd_and_ppu_enabled() {
                    if self.is_disabled() {
                        self.mode = PpuMode::JustEnabled;
                        // The PPU may have had a pending state change later than the current CPU clock.
                        // Bring the PPU clock back to the CPU's clock.
                        self.clock = cpu_clock;
                    }
                } else {
                    self.disable();
                }
            }
            STAT => {
                let stat = self.registers.stat.into_bits() & 0b1000_0111;
                let masked_value = value & 0b0111_1000;
                self.registers.stat = (stat | masked_value).into();
            }
            SCY => self.registers.scy = value,
            SCX => self.registers.scx = value,
            LYC => self.registers.lyc = value,
            BGP => self.registers.bgp = value.into(),
            OBP0 => self.registers.obp0 = value.into(),
            OBP1 => self.registers.obp1 = value.into(),
            WY => self.registers.wy = value,
            WX => self.registers.wx = value,
            _ => unreachable!(),
        }
    }
}

impl Default for Ppu {
    fn default() -> Self {
        Self {
            dots_this_line: 0,
            line_number: 0,
            mode: PpuMode::Disabled,
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
            clock: 0,
            next_vblank_interrupt: 0,
            next_lcd_interrupt: 0,
        }
    }
}

impl PostBoot for Ppu {
    fn post_boot_dmg() -> Self {
        Self {
            dots_this_line: DOTS_PER_SCANLINE - 54,
            line_number: 153,
            mode: PpuMode::LastLine3,
            x: 0,
            pixels_to_drop: 0,
            window_y: 255,
            bg_fetcher: BackgroundFetcher::default(),
            obj_buffer: VecDeque::with_capacity(10),
            obj_fetcher: ObjectFetcher::default(),
            vram: [0; VRAM_SIZE as usize],
            oam: [0; OAM_SIZE as usize],
            registers: IoRegisters::post_boot_dmg(),
            stat_interrupt_line: false,
            stat_mode_for_interrupt: 1,
            ly_to_compare_lyc: Some(0),
            oam_access: PpuMemAccess::ReadWrite,
            vram_access: PpuMemAccess::ReadWrite,
            lcd_buffer: vec![0; SCREEN_SIZE],
            clock: 0,
            next_vblank_interrupt: 0,
            next_lcd_interrupt: 0,
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
    /*#[test]
    fn test_minimum_bg_mode_3_dots() {
        let mut ppu = Ppu::post_boot_dmg();
        let mut interrupt_flags = 0;

        while !matches!(ppu.mode, PpuMode::HBlank) {
            ppu.tick(&mut interrupt_flags);
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
        ppu.registers.scx = 7;

        let mut interrupt_flags = 0;
        while !matches!(ppu.mode, PpuMode::HBlank) {
            ppu.tick(&mut interrupt_flags);
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
        // Enable the window.
        ppu.registers.lcdc.set_window_enabled(true);
        // Scroll it to x=50px
        ppu.registers.wx = 50 + 7;

        let mut interrupt_flags = 0;
        while !matches!(ppu.mode, PpuMode::HBlank) {
            ppu.tick(&mut interrupt_flags);
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
        ppu.registers.scx = 7;

        // Enable the window.
        ppu.registers.lcdc.set_window_enabled(true);
        // Scroll it to x=50px
        ppu.registers.wx = 50 + 7;

        let mut interrupt_flags = 0;
        while !matches!(ppu.mode, PpuMode::HBlank) {
            ppu.tick(&mut interrupt_flags);
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
        ppu.oam[0x00] = 16; // OBJ Y
        ppu.oam[0x01] = 0; // OBJ X
        ppu.registers.lcdc = 0x93.into(); // Enable OBJs.

        let mut interrupt_flags = 0;
        while !matches!(ppu.mode, PpuMode::HBlank) {
            ppu.tick(&mut interrupt_flags);
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
        ppu.oam[0x00] = 16; // OBJ Y
        ppu.oam[0x01] = 0; // OBJ X
        ppu.oam[0x04] = 16; // OBJ Y
        ppu.oam[0x05] = 0; // OBJ X
        ppu.registers.lcdc = 0x93.into(); // Enable OBJs.

        let mut interrupt_flags = 0;
        while !matches!(ppu.mode, PpuMode::HBlank) {
            ppu.tick(&mut interrupt_flags);
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
        for i in 0..10 {
            let obj_idx = i * 4;
            ppu.oam[obj_idx] = 16; // OBJ Y
            ppu.oam[obj_idx + 1] = 1; // OBJ X
        }
        ppu.registers.lcdc = 0x93.into(); // Enable OBJs.

        let mut interrupt_flags = 0;
        while !matches!(ppu.mode, PpuMode::HBlank) {
            ppu.tick(&mut interrupt_flags);
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
        ppu.oam[0x00] = 16; // OBJ Y
        ppu.oam[0x01] = 2; // OBJ X
        ppu.registers.lcdc = 0x93.into(); // Enable OBJs.

        let mut interrupt_flags = 0;
        while !matches!(ppu.mode, PpuMode::HBlank) {
            ppu.tick(&mut interrupt_flags);
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
        ppu.oam[0x00] = 16; // OBJ Y
        ppu.oam[0x01] = 8; // OBJ X
        ppu.registers.lcdc = 0x93.into(); // Enable OBJs.

        let mut interrupt_flags = 0;
        while !matches!(ppu.mode, PpuMode::HBlank) {
            ppu.tick(&mut interrupt_flags);
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
        ppu.oam[0x00] = 16; // OBJ Y
        ppu.oam[0x01] = 9; // OBJ X
        ppu.registers.lcdc = 0x93.into(); // Enable OBJs.

        let mut interrupt_flags = 0;
        while !matches!(ppu.mode, PpuMode::HBlank) {
            ppu.tick(&mut interrupt_flags);
        }

        let mode_3_dots = ppu.dot_counter - OAM_SCAN_DOTS;
        assert_eq!(mode_3_dots, 182);
    }*/

    use std::fs::File;
    use std::io::Write;

    #[test]
    fn lcd_on_vars() {
        let mut ppu = Ppu::default();
        ppu.write_byte(LCDC, 0x81, 0); // Enable the LCD

        let filename = "my_lcd_on_vars.csv";
        let mut file = File::create(filename).unwrap();
        writeln!(
			&mut file,
			"Dot, LY, LY for LYC, STAT Mode, OAM R Blocked, OAM W Blocked, VRAM R Blocked, VRAM W Blocked"
		)
		.unwrap();

        // TWO FRAMES
        let mut previous_line_sans_dot = String::new();
        for dot in 0..DOTS_PER_FRAME as usize * 2 {
            let output_line = format!(
                "{dot}, {}, {}, {}, {}, {}, {}, {}",
                ppu.read_byte(LY),
                ppu.ly_to_compare_lyc.unwrap_or(0xFF),
                ppu.read_byte(STAT) & 0b0000_0011,
                matches!(
                    ppu.oam_access,
                    PpuMemAccess::Blocked | PpuMemAccess::WriteOnly
                ),
                matches!(ppu.oam_access, PpuMemAccess::Blocked),
                matches!(
                    ppu.vram_access,
                    PpuMemAccess::Blocked | PpuMemAccess::WriteOnly
                ),
                matches!(ppu.vram_access, PpuMemAccess::Blocked),
            );
            if let Some((_, line_sans_dot)) = output_line.split_once(", ")
                && line_sans_dot != previous_line_sans_dot
            {
                previous_line_sans_dot = line_sans_dot.into();
                writeln!(&mut file, "{output_line}").unwrap();
            }

            let mut interrupt_flags = 0;
            ppu.catch_up(dot + 1, &mut interrupt_flags);
        }
    }

    #[test]
    fn vars() {
        let mut ppu = Ppu::post_boot_dmg();

        let filename = "my_vars.csv";
        let mut file = File::create(filename).unwrap();
        writeln!(
			&mut file,
			"Dot, LY, LY for LYC, STAT Mode, OAM R Blocked, OAM W Blocked, VRAM R Blocked, VRAM W Blocked"
		)
		.unwrap();

        let initial_clock = DOTS_PER_SCANLINE as usize - 65;
        ppu.mode = PpuMode::LastLine3;

        // TWO FRAMES
        let mut previous_line_sans_dot = String::new();
        for dot in 0..DOTS_PER_FRAME as usize * 2 {
            let output_line = format!(
                "{dot}, {}, {}, {}, {}, {}, {}, {}",
                ppu.read_byte(LY),
                ppu.ly_to_compare_lyc.unwrap_or(0xFF),
                ppu.read_byte(STAT) & 0b0000_0011,
                matches!(
                    ppu.oam_access,
                    PpuMemAccess::Blocked | PpuMemAccess::WriteOnly
                ),
                matches!(ppu.oam_access, PpuMemAccess::Blocked),
                matches!(
                    ppu.vram_access,
                    PpuMemAccess::Blocked | PpuMemAccess::WriteOnly
                ),
                matches!(ppu.vram_access, PpuMemAccess::Blocked),
            );
            if let Some((_, line_sans_dot)) = output_line.split_once(", ")
                && line_sans_dot != previous_line_sans_dot
            {
                previous_line_sans_dot = line_sans_dot.into();
                writeln!(&mut file, "{output_line}").unwrap();
            }

            let mut interrupt_flags = 0;
            ppu.catch_up(initial_clock + dot + 1, &mut interrupt_flags);
        }
    }
}
