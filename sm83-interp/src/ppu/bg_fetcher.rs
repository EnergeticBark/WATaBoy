use hw_constants::MEM_MAP_SIZE;
use hw_constants::io_regs::{LCDC, SCX, SCY};

use super::lcd_control::LcdControl;
use super::palette::Palette;
use super::tiles;

#[derive(Copy, Clone)]
pub struct Pixel {
    pub low: bool,
    pub high: bool,
    pub palette: Palette,
    pub priority: bool,
}

#[derive(Debug)]
pub enum FetcherState {
    BeforeGetTile,
    GetTile,
    BeforeGetTileDataLow,
    GetTileDataLow,
    BeforeGetTileDataHigh,
    GetTileDataHigh,
    Push,
}

pub struct BackgroundFetcher {
    pub state: FetcherState,
    pub drawing_window: bool,
    pub warmup: bool,
    pub bg_fifo: Vec<Pixel>,
    tile_id: u8,
    tile_line: u8,
    tile_x: u8,
    tile_data_low: u8,
    tile_data_high: u8,
}

/* The BackgroundFetcher has no "clear" or "reset" method as it stands right now.
   When a fresh FIFO queue is needed just make a whole new BackgroundFetcher. As far as I can tell none
   of the state gets carried over anyway. Write some good comments if anything ends up contradicting
   this.
*/
impl BackgroundFetcher {
    // Shift out a pixel from the background FIFO.
    pub fn shift_out(&mut self) -> Option<Pixel> {
        self.bg_fifo.pop()
    }

    // Push a row of 8 pixels from a tile to the background FIFO, if its empty.
    fn push(&mut self) -> bool {
        if !self.bg_fifo.is_empty() {
            return false;
        }

        for nth_bit in 0..8 {
            let pixel = Pixel {
                low: (self.tile_data_low >> nth_bit) & 1 == 1,
                high: (self.tile_data_high >> nth_bit) & 1 == 1,
                palette: Palette::Bgp,
                priority: false,
            };

            self.bg_fifo.push(pixel);
        }
        self.tile_x += 1;
        true
    }

    fn get_tile(&mut self, memory: &[u8; MEM_MAP_SIZE], current_scanline: u8, window_y: u8) {
        let lcdc = LcdControl::from_bits(memory[LCDC as usize]);

        let bg_second_tile_map = lcdc.bg_tile_map() && !self.drawing_window;
        let window_second_tile_map = lcdc.window_tile_map() && self.drawing_window;
        let second_tile_map = bg_second_tile_map || window_second_tile_map;

        let tile_x = if self.drawing_window {
            self.tile_x
        } else {
            ((memory[SCX as usize] / 8) + self.tile_x) & 0x1F
        };

        let ly = if self.drawing_window {
            window_y
        } else {
            current_scanline.wrapping_add(memory[SCY as usize])
        };
        let tile_y = ly / 8;

        let tile_map = if second_tile_map {
            tiles::tile_map_1(memory)
        } else {
            tiles::tile_map_0(memory)
        };

        self.tile_id = tile_map[tile_y as usize * 32 + tile_x as usize];
        self.tile_line = ly % 8;

        // TODO: If VRAM is blocked tile index is 0xFF...
    }

    fn current_tile<'a>(&self, memory: &'a [u8; MEM_MAP_SIZE]) -> &'a [u8; 16] {
        let lcdc = LcdControl::from_bits(memory[LCDC as usize]);

        if lcdc.bg_and_window_tiles() {
            tiles::unsigned_nth_tile(memory, self.tile_id as usize)
        } else {
            tiles::signed_nth_tile(memory, self.tile_id.cast_signed() as isize)
        }
    }

    fn get_tile_data_low(&mut self, memory: &[u8; MEM_MAP_SIZE]) {
        let tile = self.current_tile(memory);
        self.tile_data_low = tile[self.tile_line as usize * 2];
    }

    fn get_tile_data_high(&mut self, memory: &[u8; MEM_MAP_SIZE]) {
        let tile = self.current_tile(memory);
        self.tile_data_high = tile[self.tile_line as usize * 2 + 1];
    }

    pub fn tick(&mut self, memory: &[u8; MEM_MAP_SIZE], current_scanline: u8, window_y: u8) {
        self.state = match self.state {
            FetcherState::BeforeGetTile => FetcherState::GetTile,
            FetcherState::GetTile => {
                self.get_tile(memory, current_scanline, window_y);
                FetcherState::BeforeGetTileDataLow
            }
            FetcherState::BeforeGetTileDataLow => FetcherState::GetTileDataLow,
            FetcherState::GetTileDataLow => {
                self.get_tile_data_low(memory);
                FetcherState::BeforeGetTileDataHigh
            }
            FetcherState::BeforeGetTileDataHigh => {
                if self.warmup {
                    // First fetch of the line. Restart and waste six cycles (5 here, plus 1 discarding). :)
                    self.warmup = false;

                    // Fill the FIFO with 8 transparent pixels for overscan.
                    // These will be merged and discarded with any sprite pixels that are past the left edge of the LCD.
                    for _ in 0..8 {
                        let pixel = Pixel {
                            low: false,
                            high: false,
                            palette: Palette::Bgp,
                            priority: false,
                        };

                        self.bg_fifo.push(pixel);
                    }

                    FetcherState::BeforeGetTile
                } else {
                    FetcherState::GetTileDataHigh
                }
            }
            FetcherState::GetTileDataHigh => {
                self.get_tile_data_high(memory);
                FetcherState::Push
            }
            FetcherState::Push => {
                if self.push() {
                    FetcherState::BeforeGetTile
                } else {
                    FetcherState::Push
                }
            }
        }
    }
}

impl Default for BackgroundFetcher {
    fn default() -> Self {
        Self {
            state: FetcherState::BeforeGetTile,
            drawing_window: false,
            warmup: true,
            bg_fifo: Vec::with_capacity(8),
            tile_id: 0,
            tile_line: 0,
            tile_x: 0,
            tile_data_low: 0,
            tile_data_high: 0,
        }
    }
}
