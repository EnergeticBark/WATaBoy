pub mod oam;
pub mod tiles;

mod bg_fetcher;
mod obj_fetcher;
mod palette;
mod registers;

use std::{array, assert_matches, collections::VecDeque};

pub use registers::{LcdControl, LcdStatus, StatMode};

use hw_constants::{
    OAM_SIZE, PostBoot, SCREEN_SIZE, VRAM_SIZE,
    io_regs::{LCDC, LY},
};
use palette::Palette;
use registers::IoRegisters;

use crate::ppu::{
    bg_fetcher::{ColorIndex, Pixel},
    palette::PaletteSelect,
};

const TILE_WIDTH: u8 = 8;
const TILE_HEIGHT: u8 = 8;
const TILE_BYTES: u8 = 16;
const TILE_ROW_BYTES: u8 = 2;

const TILEMAP_WIDTH: u8 = 32;
const TILEMAP_HEIGHT: u8 = 32;
const TILEMAP_CELL_BYTES: u8 = 1;

// This PPU implementation is heavily modelled off of DocBoy's with a few optimisations and a bit of refactoring to make it more rust-y.
// None of these timings or quirks were discovered by me, full credit belongs to: https://github.com/Docheinstein/docboy/

#[derive(Debug)]
enum TickSelector {
    OamScanAfterTurnOn,
    OamScanAfterTurnOn78,
    OamScanAfterTurnOn79,
    PixelTransferDummyLx0,
    PixelTransferDiscardLx0,
    PixelTransferDiscardLx0Wx0Scx,
    PixelTransferLx0,
    PixelTransferLx8,
    Hblank,
    Hblank452,
    Hblank453,
    Hblank454,
    Hblank455,
    HblankLastLine,
    HblankFirstLineAfterTurnOn,
    // (me) TODO!!!
}

// (me) TODO: Possibly refactor this out into a separate file once this is all working.
#[derive(Debug)]
enum FetcherTickSelector {
    BgWinPrefetcherGetTile0,
    BgPrefetcherGetTile0,
    BgPrefetcherGetTile1,
    BgPixelSliceFetcherGetTileDataLow0,
    BgPixelSliceFetcherGetTileDataLow1,
    BgPixelSliceFetcherGetTileDataHigh0,
    BgPixelSliceFetcherGetTileDataHigh1,
    WinPrefetcherGetTile1,
    ObjPrefetcherGetTile1,
    PixelSliceFetcherIdle,
    // (me) TODO!!!
}

struct OamScanEntry {
    number: u8, // [0, 40)
    y: u8,
}

struct InitialScx {
    to_discard: u8, // [0, 8)
    discarded: u8,  // [0, 8)
}

struct PixelTransfer {
    initial_scx: InitialScx,
}

// Window
struct Window {
    active_for_frame: bool, // window activated for the frame
    wly: u8,                // window line counter

    activating: bool,     // window hit but not activated yet
    active: bool,         // currently rendering window
    just_activated: bool, // first fetch of this window streak
}

struct InterruptedFetch {
    has_data: bool,
    tile_data_low: u8,
    tile_data_high: u8,
}

// Bg/Window Prefetcher
struct Bwf {
    lx: u8, // [0, 256), advances 8 by 8
    tilemap_tile_vram_addr: u16,

    tile_number: u8, // [0, 256)

    interrupted_fetch: InterruptedFetch,
    // (me) TODO!!!
}

// Window Prefetcher
struct Wf {
    tilemap_x: u8,
}

struct ObjPrefetcher {
    entry: OamScanEntry, // hit obj
    tile_number: u8,     // [0, 256)
    attributes: u8,
}

struct PixelSliceFetcher {
    tile_data_vram_address: u16,
    tile_data_low: u8,
    tile_data_high: u8,
}

pub struct VramBus {
    pub vram: Box<[u8; VRAM_SIZE as usize]>,
    acquired_by_ppu: bool,
    // (me) TODO!!!
}

impl VramBus {
    fn acquire(&mut self) {
        self.acquired_by_ppu = true;
    }

    fn release(&mut self) {
        self.acquired_by_ppu = false;
    }

    fn is_acquired_by_this(&self) -> bool {
        self.acquired_by_ppu
    }
}

pub struct OamBus {
    pub oam: Box<[u8; OAM_SIZE as usize]>,
    acquired_by_ppu: bool,
    // (me) TODO!!!
}

impl OamBus {
    fn acquire(&mut self) {
        self.acquired_by_ppu = true;
    }

    fn release(&mut self) {
        self.acquired_by_ppu = false;
    }

    fn is_acquired_by_this(&self) -> bool {
        self.acquired_by_ppu
    }
}

pub struct Ppu {
    pub registers: IoRegisters,
    tick_selector: TickSelector,
    fetcher_tick_selector: FetcherTickSelector,

    last_stat_irq: bool,
    enable_lyc_eq_ly_irq: bool,
    pending_stat_irq: bool,

    // Dots this scanline
    pub dots: u16, // [0, 456)
    lx: u8,        // LX=X+8, therefore [0, 168]

    pub mode: StatMode, // (me) Might be desynced from the mode in the STAT register?

    // (me) Registers that behave as if they were delayed 1 T-Cycle.
    last_ly: u8,
    last_lyc: u8,
    last_bgp: Palette,
    last_wx: u8,
    last_lcdc: LcdControl,

    // (me) TODO: Different to DocBoy's FillQueue and Queue, ensure this functions the same.
    // (me) TODO: Also, this is using my generic Pixel rather than BgPixel or ObjPixel.
    bg_fifo: VecDeque<Pixel>,
    obj_fifo: VecDeque<Pixel>,

    oam_entries: [Vec<OamScanEntry>; 168],

    pixel_slice_fetcher_push_enabled: bool,
    pending_bg_fifo_fill: bool,

    is_fetching_sprite: bool,

    is_glitched_line_0: bool,
    glitched_line_0_hblank_delay: u8, // [0, 2]

    pixel_transfer: PixelTransfer,

    w: Window,

    bwf: Bwf,

    wf: Wf,

    of: ObjPrefetcher,

    psf: PixelSliceFetcher,

    // Only public for debugging reasons.
    pub vram: VramBus,
    pub oam: OamBus,
    // Buffer of greyscale pixel values, i.e. what the PPU would output to the LCD.
    pub lcd_buffer: Vec<u8>,

    pub cycles: u64,
}

impl Ppu {
    // (me) TODO!!!
    fn exec_tick_selector(&mut self) {
        match &self.tick_selector {
            TickSelector::OamScanAfterTurnOn => {
                assert!(self.dots < 78);
                assert!(!self.oam.is_acquired_by_this());

                // First OAM Scan after PPU turn on does nothing.

                self.dots += 1;
                if self.dots == 78 {
                    self.tick_selector = TickSelector::OamScanAfterTurnOn78;
                }
            }

            TickSelector::OamScanAfterTurnOn78 => {
                assert_eq!(self.dots, 78);

                // Mode is updated to Pixel Transfer one dot earlier it actually comes in.
                assert_matches!(self.mode, StatMode::OamScan);
                self.mode = StatMode::PixelTransfer;

                self.tick_selector = TickSelector::OamScanAfterTurnOn79;

                self.dots = 79;
            }

            TickSelector::OamScanAfterTurnOn79 => {
                assert_eq!(self.dots, 79);

                self.dots = 80;

                self.enter_pixel_transfer();
            }

            TickSelector::PixelTransferDummyLx0 => {
                assert_eq!(self.lx, 0);
                assert!(!self.w.active);
                assert!(self.oam.is_acquired_by_this());
                assert!(self.vram.is_acquired_by_this());

                self.dots += 1;
                if self.dots == 83 {
                    // The first tile fetch is used only to make the initial SCX % 8
                    // alignment possible, but this data will be thrown away in any case.
                    // So filling this with junk is not a problem (even for window with WX=0).
                    // (me) TODO: Is this equivalent?
                    /*self.bg_fifo.fill();*/
                    self.bg_fifo = VecDeque::from([Pixel::default(); 8]);

                    // Initial SCX is not read after the beginning of Pixel Transfer.
                    // It is read some T-cycles after: around here, after the first BG fetch.
                    // (TODO: verify precise T-Cycle timing).
                    // [mealybug/m3_scx_low_3_bits]
                    self.pixel_transfer.initial_scx.to_discard = self.registers.scx % 8;

                    if self.pixel_transfer.initial_scx.to_discard > 0 {
                        self.pixel_transfer.initial_scx.discarded = 0;

                        // When SCX % 8 is > 0, window can be activated immediately before BG.
                        self.check_window_activation();

                        self.tick_selector = if self.w.activating {
                            TickSelector::PixelTransferDiscardLx0Wx0Scx
                        } else {
                            TickSelector::PixelTransferDiscardLx0
                        };
                    } else {
                        self.tick_selector = TickSelector::PixelTransferLx0;
                    }
                }
            }

            TickSelector::PixelTransferLx0 => {
                assert!(self.lx < 8);
                assert!(
                    self.pixel_transfer.initial_scx.to_discard == 0
                        || self.pixel_transfer.initial_scx.discarded
                            == self.pixel_transfer.initial_scx.to_discard
                );
                assert!(self.oam.is_acquired_by_this());
                assert!(self.vram.is_acquired_by_this());

                self.handle_pending_window_activation();

                self.handle_pending_bg_fifo_fill();

                let mut inc_lx = false;

                // For LX € [0, 8) just pop the pixels but do not push them to LCD
                if self.is_bg_fifo_ready_to_be_popped() {
                    self.bg_fifo.pop_front();

                    if !self.obj_fifo.is_empty() {
                        self.obj_fifo.pop_front();
                    }

                    inc_lx = true;

                    if self.lx + 1 == 8 {
                        self.tick_selector = TickSelector::PixelTransferLx8;
                    }

                    self.check_window_activation();
                }

                self.tick_fetcher();

                self.handle_pixel_slice_fetcher_push();

                if inc_lx {
                    self.increase_lx();
                }

                self.dots += 1;
            }

            TickSelector::PixelTransferLx8 => {
                assert!(self.lx >= 8);
                assert!(self.oam.is_acquired_by_this());
                assert!(self.vram.is_acquired_by_this());

                self.handle_pending_window_activation();

                self.handle_pending_bg_fifo_fill();

                let mut inc_lx = false;

                // Push a new pixel to the LCD if the BG fifo contains
                // some pixels and it's not blocked by a sprite fetch
                if self.is_bg_fifo_ready_to_be_popped() {
                    const NO_COLOR: u8 = 4;
                    let mut color = NO_COLOR;

                    // Pop out a pixel from BG fifo
                    let bg_pixel = self.bg_fifo.pop_front().unwrap();

                    // On DMG, it seems that there is a 1 T-cycle delay between the real LCDC
                    // and the LCDC the PPU sees, both for LCDC.OBJ_ENABLE and LCDC.BG_WIN_ENABLE.
                    // LX == 8 seems to be an exception to this rule.
                    // [mealybug/m3_lcdc_bg_en_change, mealybug/m3_lcdc_obj_en_change].
                    // Verified: LX == 8 is not a special case on CGB.
                    let lcdc_ = if self.lx == 8 {
                        self.registers.lcdc
                    } else {
                        self.last_lcdc
                    };

                    if !self.obj_fifo.is_empty() {
                        let obj_pixel = self.obj_fifo.pop_front();

                        // Take OBJ pixel instead of the BG pixel only if all are satisfied:
                        // - LCDC.OBJ_ENABLE is 1
                        // - OBJ pixel is opaque
                        // - either one of these is satisfied:
                        //   > the BG color is 0
                        //   > LCDC.BG_WIN_ENABLE is disabled
                        //   > BG_OVER_OBJ is disabled [and BG priority is disabled as well, in CGB mode]
                        // (me) TODO: Actually implement this.
                        /*const bool show_obj =
                        lcdc_.obj_enable && is_obj_opaque(obj_pixel.color_index) &&
                        (!lcdc_.bg_win_enable || bg_pixel.color_index == 0 ||
                         (
                             !test_bit<Specs::Bits::OAM::Attributes::BG_OVER_OBJ>(obj_pixel.attributes)));

                        if (show_obj) {
                            // Resolve OBJ color against palette.
                            color = resolve_color(obj_pixel.color_index,
                                                  test_bit<Specs::Bits::OAM::Attributes::DMG_PALETTE>(obj_pixel.attributes) ? obp1
                                                                                                                            : obp0);
                        }*/
                    }

                    if color == NO_COLOR {
                        // Resolve BG color against palette.
                        // On DMG there is 1 T-cycle delay between BGP and the BGP value the PPU sees.
                        // During this T-Cycle, the PPU sees the last BGP ORed with the new BGP.
                        // Verified: it does not happen in CGB in DMG Mode.
                        // [mealybug/m3_bgp_change]
                        let bgp_: u8 = u8::from(self.registers.bgp) | u8::from(self.last_bgp);
                        // (me) TODO!!!
                        /*color = if lcdc_.bg_and_window_enabled() {
                            resolve_color(bg_pixel.color_index(), bgp_)
                        } else {
                            0
                        };*/
                    }

                    // (me) TODO!!!
                    /*ASSERT(color < NUMBER_OF_COLORS);

                    lcd.push_pixel(color);*/

                    inc_lx = true;

                    if self.lx + 1 == 168 {
                        self.increase_lx();
                        self.dots += 1;
                        self.enter_hblank();
                        return;
                    }

                    self.check_window_activation();
                }

                self.tick_fetcher();

                self.handle_pixel_slice_fetcher_push();

                if inc_lx {
                    self.increase_lx();
                }

                self.dots += 1;
            }

            TickSelector::Hblank => {
                assert_eq!(self.lx, 168);
                assert!(self.registers.ly < 143);

                self.dots += 1;
                if self.dots == 452 {
                    self.tick_selector = TickSelector::Hblank452;
                }
            }

            TickSelector::Hblank452 => {
                assert_eq!(self.dots, 452);

                // Eventually raise OAM interrupt
                self.update_stat_irq_for_oam_mode();

                self.begin_increase_ly();

                self.dots = 453;

                self.tick_selector = TickSelector::Hblank453;
            }

            unimplemented => println!("Tick selector: {unimplemented:?}"),
        }
    }

    // ------- PPU states helpers ---------

    fn enter_pixel_transfer(&mut self) {
        assert_eq!(self.mode, StatMode::PixelTransfer);
        assert_eq!(self.dots, 80);
        assert!(self.registers.ly < 144);

        // ASSERT_CODE({
        //     ASSERT(oam_scan.count <= 10);
        //     ASSERT(oam_entries_count <= oam_scan.count);
        //
        //     uint8_t count {};
        //     for (uint32_t i = 0; i < array_size(oam_entries); i++) {
        //         count += oam_entries[i].size();
        //     }
        //     ASSERT(count == oam_entries_count);
        // });
        //
        // #ifdef ENABLE_DEBUGGER
        //     timings.oam_scan = dots;
        // #endif

        self.reset_fetcher();

        self.tick_selector = TickSelector::PixelTransferDummyLx0;

        self.vram.acquire();
        self.oam.acquire();

        // (me) TODO: Call this if it ever exists.
        // lcd.reset_row_cursor();
    }

    fn enter_hblank(&mut self) {
        assert_eq!(self.lx, 168);
        assert!(self.registers.ly < 144);

        // clang-format off
        /*ASSERT_CODE({
            // Pixel Transfer Timing.

            // Factors always affecting timing:
            //    REASON                                 | DOTS
            // 1) first dummy fetch                      | 3
            // 2) push first tile (that is discarded)    | 8
            // 3) push 160 pixels to LCD                 | 8 * 20

            // Factors eventually affecting timing:
            //    REASON                                 | DOTS
            // 4) initial SCX % 8 shifting               | SCX % 8
            // 5) each window triggers reset             | 6 * #(win triggers)
            //    the fetcher and takes 6 dots           |
            // 6) WX = 0 and SCX > 0 takes an extra dot  | 1 if WX=0 and SCX > 0

            // 7) from 6 to 11 for sprite fetch          | 6 to 11

            uint16_t lb = 80; // OAM Scan

            // 1) First dummy fetch
            lb += 3;

            // 2) First tile push
            lb += 8;

            // 3) 20 tiles push
            lb += 20 * 8;

            // 4) Initial SCX % 8
            lb += pixel_transfer.initial_scx.to_discard;

            // 5) Window triggers
            uint8_t window_triggers_timing = 6 * w.line_triggers.size();

            // Consider the window activations as part of the LB only on DMG,
            // as in CGB window can be aborted and fetcher restarted if it is
            // disabled during the first fetch: the lower bound can't be guaranteed.
            lb += window_triggers_timing;

            // 6) WX = 0 and SCX > 0
            bool window_triggered_at_wx0 = false;
            for (uint8_t i = 0; i < w.line_triggers.size(); i++) {
                if (w.line_triggers[i] == 0) {
                    window_triggered_at_wx0 = true;
                    break;
                }
            }
            lb += window_triggered_at_wx0 && pixel_transfer.initial_scx.to_discard > 0;

            const uint8_t oam_entries_served_count = oam_entries_count - oam_entries_not_served_count;

            // 7) Sprite fetch
            lb += 6 * oam_entries_served_count /* use served count for LB */;

            uint16_t ub = lb;

            // A precise Upper Bound is difficult to predict due to sprite timing,
            // I'll let test roms verify this precisely.
            // 7) Sprite fetch
            ub += (11 - 6 /* already considered in LB */) * oam_entries_count /* use total count for UB */;

            ASSERT(dots >= lb);
            ASSERT(dots <= ub);
        });*/
        // clang-format on

        /*#ifdef ENABLE_DEBUGGER
            timings.pixel_transfer = dots - timings.oam_scan;
        #endif*/

        if self.is_glitched_line_0 {
            // HBlank mode is delayed by 2 dots the first line PPU is turned on.
            self.tick_selector = TickSelector::HblankFirstLineAfterTurnOn;

            self.glitched_line_0_hblank_delay = 0;

            // VRAM and OAM buses are released the next dot.
        } else {
            self.mode = StatMode::HBlank;

            self.tick_selector = if self.registers.ly == 143 {
                TickSelector::HblankLastLine
            } else {
                TickSelector::Hblank
            };

            self.vram.release();
            self.oam.release();
        }
    }

    pub fn tick(&mut self) {
        // Do nothing if PPU is off
        if self.registers.lcdc.enabled() {
            // Tick PPU by one dot
            self.exec_tick_selector();

            // Check for window activation (the state holds for entire frame).
            // The activation conditions are checked every dot,
            // (not only during pixel transfer or at LY == WX).
            // This is also reflected in the fact that window glitches for
            // the rest of the frame by pushing 00 pixels if WX == LX during the fetched
            // push stage if at some point in the entire frame the condition
            // WY = LY was satisfied.
            self.tick_window();

            // Update STAT's LYC_EQ_LY and eventually raise STAT interrupt.
            self.raise_stat_irq();

            // On DMG there is 1 T-cycle delay between BGP and the BGP value the PPU sees.
            // During this T-Cycle, the PPU sees the last BGP ORed with the new one.
            // Therefore, we store the last BGP value here so that we can use (BGP | LAST_BGP)
            // for resolving BG color.
            // [mealybug/m3_bgp_change]
            // Verified: CGB is not affected.
            self.last_bgp = self.registers.bgp;

            // On DMG, this is needed for:
            // 1) LCDC.WIN_ENABLE, because it seems that the t-cycle window is turned on
            //    (and it was off), window is activated also for LX == WX + 1, not only for LX == WX.
            //    [mealybug/m3_lcdc_win_en_change_multiple_wx]
            // 2) LCDC.BG_WIN_ENABLE, because it seems that there is a 1 T-cycle delay for this bit
            //    [mealybug/m3_lcdc_bg_en_change]
            // 3) LCDC.OBJ_ENABLE, because it seems that there is a 1 T-cycle delay for this bit
            //    [mealybug/m3_lcdc_obj_en_change]
            // Verified: CGB is not affected (it sees LCDC delayed by 2 T-Cycles by default)
            // On CGB, this is needed for:
            // 1) LCDC.BG_WIN_TILE_DATA selector change glitch.
            //    [mealybug/m3_lcdc_tile_sel_change (DMG mode)]
            self.last_lcdc = self.registers.lcdc;

            // It seems that there is a 1 T-cycle delay for the LY value used for the comparison LYC == LY.
            // This is supported by the fact that for LY = 153, LYC_EQ_LY STAT flag is raised for LYC = 153
            // the very same T-cycle LY is 0 (but was 153 the T-cycle before).
            self.last_ly = self.registers.ly;

            // It seems that there is a 1 T-cycle delay for the LYC value used for the comparison LYC == LY.
            self.last_lyc = self.registers.lyc;

            //#ifdef ENABLE_DEBUGGER
            self.cycles += 1;
            //#endif

            // Writes to PPU registers have delayed effects.
            // This happens mostly on CGB, where some registers are seen with delay of 2 T-Cycles.
            self.tick_pending_write();

            // Update STAT's mode (usually it matches the "internal" PPU mode, but there are few exceptions).
            self.tick_stat();

            assert!(self.dots < 456);
        }
    }

    // ------- PPU helpers -------

    fn turn_on(&mut self) {
        assert_eq!(self.registers.ly, 0);

        self.last_lyc = self.registers.lyc;

        // STAT's LYC_EQ_LY is updated properly (but interrupt is not raised)
        self.registers.stat.set_coincidence(self.is_lyc_eq_ly());

        // (me) TODO: Call this if it ever exists.
        // lcd.turn_on_ppu();
    }

    fn turn_off(&mut self) {
        self.dots = 0;
        self.registers.ly = 0;
        self.last_ly = 0;

        // Clear oam entries eventually still there
        // (me) TODO!!!
        /*for (uint32_t i = 0; i < array_size(oam_entries); i++) {
            oam_entries[i].clear();
        }*/

        self.reset_fetcher();

        /*
        #ifdef ENABLE_ASSERTS
            oam_entries_count = 0;
            oam_entries_not_served_count = 0;
        #endif

        #ifdef ENABLE_DEBUGGER
            scanline_oam_entries.clear();
        #endif
        */

        // The first line after PPU is turn on behaves differently.
        // STAT's mode remains in HBLANK during OAM Scan, OAM Bus is not acquired,
        // OAM Scan does not check for any sprite and STAT's update for HBLANK is late by 2 T-cycles.
        self.mode = StatMode::OamScan;

        self.tick_selector = TickSelector::OamScanAfterTurnOn;

        self.is_glitched_line_0 = true;
        self.glitched_line_0_hblank_delay = 0;

        self.enable_lyc_eq_ly_irq = true;

        self.vram.release();
        self.oam.release();

        // (me) TODO: Call this if it ever exists.
        // lcd.turn_off_ppu();
    }

    fn is_lyc_eq_ly(&self) -> bool {
        // In addition to the condition (LYC == LY), LYC_EQ_LY IRQ might be disabled for several reasons.
        // Remarkably:
        // * LYC_EQ_LY is always 0 at dot 454
        // * The last scanline (LY = 153 = 0) the LY values differs from what LYC_EQ_LY reads and from the related IRQ.
        // [mooneye/lcdon_timing, daid/ppu_scanline_bgp]
        (self.last_lyc == self.last_ly) && self.enable_lyc_eq_ly_irq
    }

    fn raise_stat_irq(&mut self) {
        // (me) TODO!!!
    }

    fn tick_pending_write(&mut self) {
        // (me) TODO!!!
    }

    fn tick_stat(&mut self) {
        // (me) TODO!!!
    }

    fn update_stat_irq_for_oam_mode(&mut self) {
        assert!(self.enable_lyc_eq_ly_irq);
        assert_eq!(self.last_ly, self.registers.ly);

        // OAM mode interrupt is not checked every dot: it is checked only here during mode transition.
        // Furthermore, it seems that having a pending LYC_EQ_LY signal high prevents the STAT IRQ to go low.
        // [daid/ppu_scanline_bgp]
        let lyc_eq_ly_irq = self.registers.stat.lyc_int_select() && (self.last_lyc == self.last_ly);
        let lyc_eq_next_ly_irq =
            self.registers.stat.lyc_int_select() && (self.last_lyc == self.last_ly + 1);
        let irq = self.registers.stat.mode2_int_select() || lyc_eq_ly_irq;

        self.pending_stat_irq = self.last_stat_irq < irq;

        // Actually, from the tests it seems that the only case in which the STAT IRQ is reset
        // (and consequently there's an opportunity to trigger an interrupt on raising edge)
        // is when LY_EQ_LYC is true for the next LY.
        // TODO: This is bizarre by the way, find a more reasonable way to represent this oddity.
        if lyc_eq_next_ly_irq {
            self.last_stat_irq = irq;
        }
    }

    fn begin_increase_ly(&self) {
        assert!(self.registers.ly < 153);
    }

    fn tick_window(&mut self) {
        // Window is considered active for the rest of the frame
        // if at some point in it happens that window is enabled while LY matches WY.
        // Note that this does not mean that window will always be rendered:
        // the WX == LX condition will be checked again during pixel transfer
        // (on the other hand WY is not checked again).
        let ly = self.registers.ly;
        self.w.active_for_frame = self.w.active_for_frame
            || (self.registers.lcdc.window_enabled() && ly == self.registers.wy);
    }

    fn tick_fetcher(&mut self) {
        match &self.fetcher_tick_selector {
            FetcherTickSelector::BgWinPrefetcherGetTile0 => {
                assert!(!self.is_fetching_sprite);

                // Check whether fetch the tile for the window or for the BG
                if self.w.active {
                    if self.registers.lcdc.window_enabled()
                    /* TODO: other conditions checked? WY?*/
                    {
                        // Proceed to fetch window
                        self.win_prefetcher_get_tile_0();
                        return;
                    }

                    // Abort window fetch and proceed with BG fetch
                    self.w.active = false;
                }

                self.bg_prefetcher_get_tile_0();
            }

            FetcherTickSelector::BgPrefetcherGetTile1 => {
                assert!(!self.is_fetching_sprite);
                self.fetcher_tick_selector =
                    FetcherTickSelector::BgPixelSliceFetcherGetTileDataLow0;
            }

            FetcherTickSelector::BgPixelSliceFetcherGetTileDataLow0 => {
                assert!(!self.is_fetching_sprite);

                // Note that fetcher determinate the tile data address to read both in
                // GetTileDataLow and GetTileDataHigh just before access it from VRAM.
                // Therefore, changes to SCY or BG_WIN_TILE_DATA during these phases may
                // have bitplane desync effects
                // (e.g. read the low byte from a tile and the high byte from a different one).
                // [mealybug/m3_scy_change, mealybug/m3_lcdc_tile_sel_change]
                self.setup_bg_pixel_slice_fetcher_tile_data_address();

                // Read tile data from VRAM.
                {
                    self.read_bgwin_tile_data_low();
                }

                self.fetcher_tick_selector =
                    FetcherTickSelector::BgPixelSliceFetcherGetTileDataLow1;
            }

            FetcherTickSelector::BgPixelSliceFetcherGetTileDataLow1 => {
                assert!(!self.is_fetching_sprite);

                self.fetcher_tick_selector =
                    FetcherTickSelector::BgPixelSliceFetcherGetTileDataHigh0;
            }

            FetcherTickSelector::BgPixelSliceFetcherGetTileDataHigh0 => {
                assert!(!self.is_fetching_sprite);

                // Note that fetcher determinate the tile data address to read both in
                // GetTileDataLow and GetTileDataHigh just before access it from VRAM.
                // Therefore, changes to SCY or BG_WIN_TILE_DATA during these phases may
                // have bitplane desync effects
                // (e.g. read the low byte from a tile and the high byte from a different one).
                // [mealybug/m3_scy_change, mealybug/m3_lcdc_tile_sel_change]
                self.setup_bg_pixel_slice_fetcher_tile_data_address();

                {
                    // Read tile data from VRAM.
                    self.read_bgwin_tile_data_high();
                }

                self.fetcher_tick_selector =
                    FetcherTickSelector::BgPixelSliceFetcherGetTileDataHigh1;
            }

            FetcherTickSelector::BgPixelSliceFetcherGetTileDataHigh1 => {
                self.bgwin_pixel_slice_fetcher_get_tile_data_high_1();
            }

            selector => println!("Fetcher: {selector:?}"),
        }
    }

    fn reset_fetcher(&mut self) {
        self.lx = 0;
        self.bg_fifo.clear();
        self.obj_fifo.clear();
        self.pixel_slice_fetcher_push_enabled = false;
        self.pending_bg_fifo_fill = false;
        self.is_fetching_sprite = false;

        self.w.activating = false;
        self.w.active = false;
        self.w.just_activated = false;
        // #if defined(ENABLE_DEBUGGER) || defined(ENABLE_ASSERTS)
        //     w.line_triggers.clear();
        // #endif
        self.bwf.lx = 0;
        self.bwf.tile_number = 0;
        self.bwf.interrupted_fetch.has_data = false;
        self.wf.tilemap_x = 0;
        self.fetcher_tick_selector = FetcherTickSelector::BgWinPrefetcherGetTile0;
    }

    fn increase_lx(&mut self) {
        /*
            ASSERT(oam_entries_count >= oam_entries[lx].size());
        #ifdef ENABLE_ASSERTS
            oam_entries_not_served_count += oam_entries[lx].size(); // update with the real served oam entries count.
        #endif
            ASSERT(oam_entries_not_served_count <= oam_entries_count);
            */

        // Clear oam entries of current LX (it might contain OBJs not served).
        self.oam_entries[self.lx as usize].clear();

        // Increase LX.
        self.lx += 1;
    }

    fn is_bg_fifo_ready_to_be_popped(&mut self) -> bool {
        // The condition for which the bg fifo can be popped are:
        // - the bg fifo is not empty
        // - the fetcher is not fetching a sprite
        // - there are no more oam entries for this LX to be fetched or obj are disabled in LCDC
        !self.bg_fifo.is_empty()
            && !self.is_fetching_sprite
            && (self.oam_entries[self.lx as usize].is_empty() || !self.registers.lcdc.obj_enabled())
    }

    fn is_obj_ready_to_be_fetched(&mut self) -> bool {
        // The condition for which a obj fetch can be served are:
        // - there is at least a pending OAM entry for the current LX
        // - obj are enabled in LCDC
        !self.oam_entries[self.lx as usize].is_empty() && self.registers.lcdc.obj_enabled()
    }

    fn fill_bg_fifo(&mut self) {
        // (me) TODO: is this equivalent?
        /*bg_fifo.fill(&TILE_ROW_DATA_TO_ROW_PIXELS[concat(psf.tile_data_high, psf.tile_data_low)]);*/
        for nth_bit in 0..8 {
            let pixel = Pixel::new()
                .with_color_index(
                    ColorIndex::new()
                        .with_low((self.psf.tile_data_low >> nth_bit) & 1 == 1)
                        .with_high((self.psf.tile_data_high >> nth_bit) & 1 == 1),
                )
                .with_palette(PaletteSelect::Bgp)
                .with_priority(false);

            self.bg_fifo.push_back(pixel);
        }
    }

    fn handle_pixel_slice_fetcher_push(&mut self) {
        // Push can't happen while we're fetching sprites.
        if self.pixel_slice_fetcher_push_enabled && !self.is_fetching_sprite {
            // clang-format off
            /*
             * As far as I understand, there are 4 possible cases:
             *
             * [can_push_to_bg_fifo] | [is_obj_ready_to_be_fetched]
             * ---------------------------------------------------------
             * bg_fifo.is_empty()    | oam_entries[lx].is_not_empty() &&
             *                       | lcdc.obj_enable
             * ---------------------------------------------------------
             *        0              |           0              | wait (nop)
             *        0              |           1              | cache bg fetch and start obj prefetcher (tick now)
             *        1              |           0              | push to bg fifo and prepare for bg/win prefetcher (tick next dot)
             *        1              |           1              | push to bg fifo and start obj prefetcher (tick now)
             */
            // clang-format on
            // The pixels can be pushed only if the bg fifo is empty,
            // otherwise wait in push state until bg fifo is emptied
            let can_push_to_bg_fifo = self.bg_fifo.is_empty();
            if can_push_to_bg_fifo {
                // If window activation conditions were met in the frame, then a
                // glitch can occur: if this push stage happens when LX == WX,
                // then a 00 pixel is pushed into the bg fifo instead of the fetched tile.
                // Therefore, the push of the tile that was about to happen is postponed by 1 dot.
                // The first window tile after the window activation seems to be an exception.
                // Verified: this happens also on CGB, although the conditions that make
                // this happen are slightly different. In particular, on CGB the glitch seems to
                // happen only if we are fetching a window tile right now.
                if self.w.active_for_frame && self.lx == self.registers.wx && !self.w.just_activated
                {
                    self.bg_fifo.push_back(Pixel::new());
                    return;
                }

                // Push pixels into bg fifo.
                self.pending_bg_fifo_fill = true;

                // Disable pixel slice fetcher push phase.
                self.disable_pixel_slice_fetcher_push();

                self.fetcher_tick_selector = FetcherTickSelector::BgWinPrefetcherGetTile0;

                // Sprite fetches are ignored just after a window tile push.
                // TODO: w.just_activated or w.active? Test needed!
                if self.w.just_activated {
                    self.w.just_activated = false;
                    return;
                }
            }

            // If there is a pending obj hit, discard the fetched bg pixels
            // and restart the obj prefetcher with the obj hit.
            // Note: the first obj prefetcher tick overlaps this tick.
            if self.is_obj_ready_to_be_fetched() {
                if !can_push_to_bg_fifo {
                    // The bg/win tile fetched is not thrown away: instead, it is
                    // cached so that the fetcher can start with it after the sprite
                    // has been merged into obj fifo.
                    self.cache_bg_win_fetch();
                }

                // Now fetching sprite.
                // Implicitly prevent pixel slice fetcher to push to bg fifo.
                self.is_fetching_sprite = true;

                /*self.of.entry = self.oam_entries[self.lx as usize].pull_back();*/
                self.of.entry = self.oam_entries[self.lx as usize].pop().unwrap(); // (me) TODO: Is this the same?
                self.obj_prefetcher_get_tile_0();
            }
        }
    }

    fn handle_pending_bg_fifo_fill(&mut self) {
        if self.pending_bg_fifo_fill {
            // Re-fill the entire bg fifo with pixel slice fetcher's data.
            assert!(self.bg_fifo.is_empty());
            self.pending_bg_fifo_fill = false;
            // (me) TODO!!!
            /*bg_fifo.set_size(8);*/
            self.fill_bg_fifo();
        }
    }

    fn handle_pending_window_activation(&mut self) {
        if self.w.activating {
            {
                // Standard case: actually activate window.
                self.activate_window();
            }

            self.w.activating = false;

            self.fetcher_tick_selector = FetcherTickSelector::BgWinPrefetcherGetTile0;
        }
    }

    fn activate_window(&mut self) {
        // Throw away the pixels in the BG fifo.
        self.bg_fifo.clear();

        // Activate window.
        self.w.active = true;
        self.w.just_activated = true;

        /*
        #if defined(ENABLE_DEBUGGER) || defined(ENABLE_ASSERTS)
            w.line_triggers.push_back(last_wx);
        #endif
        */

        // Reset the window tile counter.
        self.wf.tilemap_x = 0;
    }

    fn check_window_activation(&mut self) {
        // The conditions for which the window can be triggered are:
        // - at some point in the frame LY was equal to WY
        // - window is enabled in LCDC
        // - pixel transfer LX matches WX
        // Furthermore, on DMG there's a glitch that causes window to be
        // triggered even late by 1 dot (i.e. a BG/OBJ pixel is pushed instead and
        // all the window pixels are shifted right by 1 pixel).
        // This happens if LCDC.WIN_ENABLE was off and is turned on while LX == WX + 1.
        // [mealybug/m3_lcdc_win_en_change_multiple_wx]
        // Verified: this glitch does not happen on CGB.
        if self.w.active_for_frame
            && !self.w.active
            && self.registers.lcdc.window_enabled()
            && (self.lx == self.last_wx
                || (self.lx == self.last_wx + 1 && !self.last_lcdc.window_enabled()/* DMG glitch: window late by 1 dot case */))
        {
            self.setup_fetcher_for_window();
        }
    }

    fn setup_fetcher_for_window(&mut self) {
        assert!(!self.w.active);

        self.w.activating = true;

        // Increase the window line counter.
        self.w.wly += 1;

        // Do not allow pixel slice fetcher to push to bg fifo.
        self.disable_pixel_slice_fetcher_push();

        // The next fetcher tick is skipped.
        self.fetcher_tick_selector = FetcherTickSelector::PixelSliceFetcherIdle;
    }

    // -------- Fetcher states ---------

    // ----------- bg / win ------------

    fn bg_prefetcher_get_tile_0(&mut self) {
        assert!(!self.is_fetching_sprite);

        // The prefetcher computes at this phase only the tile base address.
        // based on the tilemap X and Y coordinate and the tile map to use.
        // [mealybug/m3_lcdc_bg_map_change].
        self.setup_bg_pixel_slice_fetcher_tilemap_tile_address();

        {
            // Read the tile number (and the attributes, in CGB mode) from VRAM.
            self.read_bgwin_tile_number_and_attributes();
        }

        self.fetcher_tick_selector = FetcherTickSelector::BgPrefetcherGetTile1;
    }

    fn win_prefetcher_get_tile_0(&mut self) {
        assert!(!self.is_fetching_sprite);
        assert!(self.w.active_for_frame);
        assert!(self.w.active);

        // If the window is turned off, the fetcher switches back to the BG fetcher.
        if !self.registers.lcdc.window_enabled() {
            self.bg_prefetcher_get_tile_0();
            return;
        }

        // The prefetcher computes at this phase only the tile base address.
        // based on the tilemap X and Y coordinate and the tile map to use.
        // [mealybug/m3_lcdc_win_map_change].
        self.setup_win_pixel_slice_fetcher_tilemap_tile_address();

        // On CGB there's a glitch that is triggered if the condition LX == WX
        // is ever met (i.e. WX has been changed since window activation).
        // If this happens at GetTile0 phase, the tile number is not read,
        // instead the tile of the last fetch is used instead.
        // This also happens when WX == LX == 0. Indeed, the first window tile
        // fetch for WX ==  and SCX > 0 does not read the proper tile data,
        // instead it reads the tile data from tile number 0.
        // [mealybug/m3_wx_4_change (DMG mode)]
        {
            // Read the tile number (and the attributes, in CGB mode) from VRAM.
            self.read_bgwin_tile_number_and_attributes();
        }

        self.fetcher_tick_selector = FetcherTickSelector::WinPrefetcherGetTile1;
    }

    fn bgwin_pixel_slice_fetcher_get_tile_data_high_1(&mut self) {
        assert!(!self.is_fetching_sprite);

        // The bg/win fetcher tile is increased only at this step.
        // Therefore, if the window aborts a bg fetch before this step,
        // the bg tile does not increment: it increments only if the
        // fetcher reaches the push stage (or a sprite aborts it).
        // The first window fetch does not increase the bg tile.
        if !self.w.just_activated {
            self.bwf.lx += TILE_WIDTH;
        }

        // Allow pixel slice fetcher to push to bg fifo.
        self.fetcher_tick_selector = FetcherTickSelector::PixelSliceFetcherIdle;
        self.enable_pixel_slice_fetcher_push();
    }

    // ------------- obj ---------------

    fn obj_prefetcher_get_tile_0(&mut self) {
        assert!(self.is_fetching_sprite);

        // Read two bytes from OAM (Tile Number and Attributes).
        // Note that if DMA transfer is in progress conflicts can occur and
        // we might end up reading from the OAM address the DMA is writing to
        // (but only if DMA writing request is pending, i.e. it is in T0 or T1).
        // [hacktix/strikethrough]

        // TODO: figure out exact timing of read request and read flush for obj prefetcher.
        // (me) TODO: figure out some way to implement this. WATaBoy has no notion of a "read_word_request".
        /*oam.read_word_request(
            Specs::MemoryLayout::OAM::START
                + OAM_ENTRY_BYTES * of.entry.number
                + Specs::Bytes::OAM::TILE_NUMBER,
        );*/

        self.fetcher_tick_selector = FetcherTickSelector::ObjPrefetcherGetTile1;
    }

    // ------- Fetcher states helpers ---------

    fn setup_bg_pixel_slice_fetcher_tilemap_tile_address(&mut self) {
        let tilemap_x: u8 = ((self.bwf.lx + self.registers.scx) / TILE_WIDTH) % TILEMAP_WIDTH;
        let tilemap_y: u8 =
            ((self.registers.ly + self.registers.scy) / TILE_HEIGHT) % TILEMAP_HEIGHT;

        let tilemap_vram_addr_base: u16 = if self.registers.lcdc.bg_tile_map() {
            0x1C00
        } else {
            0x1800
        }; // 0x9800 or 0x9C00 (global)
        let tilemap_vram_addr_slack: u16 =
            (u16::from(TILEMAP_WIDTH) * u16::from(TILEMAP_CELL_BYTES) * u16::from(tilemap_y))
                + u16::from(tilemap_x);
        self.bwf.tilemap_tile_vram_addr = tilemap_vram_addr_base + tilemap_vram_addr_slack;

        /*#ifdef ENABLE_DEBUGGER
            bwf.tilemap_x = tilemap_x;
            bwf.tilemap_y = tilemap_y;
            bwf.tilemap_vram_addr = tilemap_vram_addr_base;
        #endif*/
    }

    fn setup_bg_pixel_slice_fetcher_tile_data_address(&mut self) {
        let vram_tile_addr: u16 = if self.registers.lcdc.bg_and_window_tiles() {
            // unsigned addressing mode with 0x8000 as (global) base address
            0x0000 + u16::from(TILE_BYTES) * u16::from(self.bwf.tile_number)
        } else {
            // signed addressing mode with 0x9000 as (global) base address
            (0x1000 + i16::from(TILE_BYTES) * i16::from(self.bwf.tile_number.cast_signed()))
                .cast_unsigned()
        };

        let tile_y = (self.registers.ly + self.registers.scy) % TILE_HEIGHT;

        assert!(tile_y < 8);

        self.psf.tile_data_vram_address =
            vram_tile_addr + u16::from(TILE_ROW_BYTES) * u16::from(tile_y);
    }

    fn setup_win_pixel_slice_fetcher_tilemap_tile_address(&mut self) {
        // The window prefetcher has its own internal counter to determine the tile to fetch
        // (me) TODO: is this equivalent?
        /*const uint8_t tilemap_x = wf.tilemap_x++;*/
        let tilemap_x = self.wf.tilemap_x;
        self.wf.tilemap_x += 1;
        let tilemap_y = self.w.wly / TILE_HEIGHT;

        let tilemap_vram_addr_base: u16 = if self.registers.lcdc.window_tile_map() {
            0x1C00
        } else {
            0x1800
        }; // 0x9800 or 0x9C00 (global)
        let tilemap_vram_addr_slack: u16 =
            (u16::from(TILEMAP_WIDTH) * u16::from(TILEMAP_CELL_BYTES) * u16::from(tilemap_y))
                + u16::from(tilemap_x);
        self.bwf.tilemap_tile_vram_addr = tilemap_vram_addr_base + tilemap_vram_addr_slack;

        /*#ifdef ENABLE_DEBUGGER
            bwf.tilemap_x = tilemap_x;
            bwf.tilemap_y = tilemap_y;
            bwf.tilemap_vram_addr = tilemap_vram_addr_base;
        #endif*/
    }

    fn read_bgwin_tile_number_and_attributes(&mut self) {
        // (me) TODO: is this equivalent?
        /*bwf.tile_number = vram.read<0>(bwf.tilemap_tile_vram_addr);*/
        self.bwf.tile_number = self.vram.vram[self.bwf.tilemap_tile_vram_addr as usize];
    }

    fn read_bgwin_tile_data_low(&mut self) {
        self.psf.tile_data_low = self.vram.vram[self.psf.tile_data_vram_address as usize];
    }

    fn read_bgwin_tile_data_high(&mut self) {
        self.psf.tile_data_high = self.vram.vram[self.psf.tile_data_vram_address as usize + 1];
    }

    fn enable_pixel_slice_fetcher_push(&mut self) {
        self.pixel_slice_fetcher_push_enabled = true;
    }

    fn disable_pixel_slice_fetcher_push(&mut self) {
        self.pixel_slice_fetcher_push_enabled = false;
    }

    fn cache_bg_win_fetch(&mut self) {
        self.bwf.interrupted_fetch.tile_data_low = self.psf.tile_data_low;
        self.bwf.interrupted_fetch.tile_data_high = self.psf.tile_data_high;
        assert!(!self.bwf.interrupted_fetch.has_data);
        self.bwf.interrupted_fetch.has_data = true;
    }

    fn write_lcdc(&mut self, value: u8) {
        let new_lcdc = LcdControl::from_bits(value);

        let en = new_lcdc.enabled();
        if en != self.registers.lcdc.enabled() {
            // Enabling or disabling the PPU happen instantly both on DMG and CGB.
            if en {
                self.turn_on();
            } else {
                self.turn_off();
            }
        }

        self.registers.lcdc = new_lcdc;
    }

    pub(super) fn read_byte(&self, index: u16) -> u8 {
        match index {
            LY => self.registers.ly,
            _ => unreachable!(),
        }
    }

    pub(super) fn write_byte(&mut self, index: u16, value: u8) {
        match index {
            LCDC => self.write_lcdc(value),
            _ => unreachable!(),
        }
    }

    // Side-effect free reads.
    #[must_use]
    pub fn debug_read_byte(&self, index: u16) -> u8 {
        match index {
            LY => self.registers.ly,
            _ => unreachable!(),
        }
    }
}

impl Default for Ppu {
    fn default() -> Self {
        Self {
            registers: IoRegisters::default(),
            tick_selector: TickSelector::OamScanAfterTurnOn,
            fetcher_tick_selector: FetcherTickSelector::BgPrefetcherGetTile0,

            last_stat_irq: false,
            enable_lyc_eq_ly_irq: true,
            pending_stat_irq: false,

            dots: 0,
            lx: 0,

            // TODO: ensure this is OAM_SCAN
            mode: IoRegisters::default().stat.mode(),

            last_ly: IoRegisters::default().ly,
            last_lyc: IoRegisters::default().lyc,
            last_bgp: IoRegisters::default().bgp,
            last_wx: IoRegisters::default().wx,
            last_lcdc: IoRegisters::default().lcdc,

            bg_fifo: VecDeque::with_capacity(8),
            obj_fifo: VecDeque::with_capacity(8),

            oam_entries: array::from_fn(|_| Vec::with_capacity(10)),

            pixel_slice_fetcher_push_enabled: false,
            pending_bg_fifo_fill: false,

            is_fetching_sprite: false,

            is_glitched_line_0: false,
            glitched_line_0_hblank_delay: 0,

            pixel_transfer: PixelTransfer {
                initial_scx: InitialScx {
                    to_discard: 0,
                    discarded: 0,
                },
            },

            w: Window {
                active_for_frame: false,
                wly: u8::MAX,

                activating: false,
                active: false,
                just_activated: false,
            },

            bwf: Bwf {
                lx: 0,
                tilemap_tile_vram_addr: 0,
                tile_number: 0,
                interrupted_fetch: InterruptedFetch {
                    has_data: false,
                    tile_data_low: 0,
                    tile_data_high: 0,
                },
            },

            wf: Wf { tilemap_x: 0 },

            of: ObjPrefetcher {
                entry: OamScanEntry { number: 0, y: 0 },
                tile_number: 0,
                attributes: 0,
            },

            psf: PixelSliceFetcher {
                tile_data_vram_address: 0,
                tile_data_low: 0,
                tile_data_high: 0,
            },

            vram: VramBus {
                vram: vec![0; VRAM_SIZE as usize]
                    .into_boxed_slice()
                    .try_into()
                    .unwrap(),
                acquired_by_ppu: false,
            },
            oam: OamBus {
                oam: vec![0; OAM_SIZE as usize]
                    .into_boxed_slice()
                    .try_into()
                    .unwrap(),
                acquired_by_ppu: false,
            },
            // Buffer of greyscale pixel values, i.e. what the PPU would output to the LCD.
            lcd_buffer: vec![0; SCREEN_SIZE],
            cycles: 0,
        }
    }
}

impl PostBoot for Ppu {
    fn post_boot_mgb() -> Self {
        Self {
            // TODO: Actually fill in these values
            ..Default::default()
        }
    }
}
