pub mod oam;
pub mod tiles;

mod bg_fetcher;
mod obj_fetcher;
mod palette;
mod registers;

pub use registers::{LcdControl, LcdStatus, StatMode};

use hw_constants::{OAM_SIZE, PostBoot, SCREEN_SIZE, VRAM_SIZE};
use palette::Palette;
use registers::IoRegisters;

// This PPU implementation is heavily modelled off of DocBoy's with a few optimisations and a bit of refactoring to make it more rust-y.
// None of these timings or quirks were discovered by me, full credit belongs to: https://github.com/Docheinstein/docboy/

enum TickSelector {
    OamScanAfterTurnOn,
    // (me) TODO!!!
}

// (me) TODO: Possibly refactor this out into a separate file once this is all working.
enum FetcherTickSelector {
    BgPrefetcherGetTile0, // (me) TODO!!!
}

struct Window {
    active_for_frame: bool,
    // (me) TODO!!!
}

pub struct Ppu {
    registers: IoRegisters,
    tick_selector: TickSelector,
    fetcher_tick_selector: FetcherTickSelector,
    // Dots this scanline
    dots: u16, // [0, 456)
    lx: u8,    // LX=X+8, therefore [0, 168]

    mode: StatMode, // (me) Might be desynced from the mode in the STAT register?

    // (me) Registers that behave as if they were delayed 1 T-Cycle.
    last_ly: u8,
    last_lyc: u8,
    last_bgp: Palette,
    last_wx: u8,
    last_lcdc: LcdControl,

    w: Window,

    // Only public for debugging reasons.
    pub vram: Box<[u8; VRAM_SIZE as usize]>,
    pub oam: Box<[u8; OAM_SIZE as usize]>,
    // Buffer of greyscale pixel values, i.e. what the PPU would output to the LCD.
    pub lcd_buffer: Vec<u8>,
}

impl Ppu {
    // (me) TODO!!!
    fn exec_tick_selector(&mut self) {
        match self.tick_selector {
            _ => (),
        }
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

    fn raise_stat_irq(&mut self) {
        // (me) TODO!!!
    }

    fn tick_pending_write(&mut self) {
        // (me) TODO!!!
    }

    fn tick_stat(&mut self) {
        // (me) TODO!!!
    }

    fn tick(&mut self) {
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

            /*#ifdef ENABLE_DEBUGGER
                    ++cycles;
            #endif*/

            // Writes to PPU registers have delayed effects.
            // This happens mostly on CGB, where some registers are seen with delay of 2 T-Cycles.
            self.tick_pending_write();

            // Update STAT's mode (usually it matches the "internal" PPU mode, but there are few exceptions).
            self.tick_stat();

            assert!(self.dots < 456);
        }
    }
}

impl Default for Ppu {
    fn default() -> Self {
        Self {
            registers: IoRegisters::default(),
            tick_selector: TickSelector::OamScanAfterTurnOn,
            fetcher_tick_selector: FetcherTickSelector::BgPrefetcherGetTile0,
            dots: 0,
            lx: 0,

            // TODO: ensure this is OAM_SCAN
            mode: IoRegisters::default().stat.mode(),

            last_ly: IoRegisters::default().ly,
            last_lyc: IoRegisters::default().lyc,
            last_bgp: IoRegisters::default().bgp,
            last_wx: IoRegisters::default().wx,
            last_lcdc: IoRegisters::default().lcdc,

            w: Window {
                active_for_frame: false,
            },

            vram: vec![0; VRAM_SIZE as usize]
                .into_boxed_slice()
                .try_into()
                .unwrap(),
            oam: vec![0; OAM_SIZE as usize]
                .into_boxed_slice()
                .try_into()
                .unwrap(),
            // Buffer of greyscale pixel values, i.e. what the PPU would output to the LCD.
            lcd_buffer: vec![0; SCREEN_SIZE],
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
