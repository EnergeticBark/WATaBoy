use crate::lcd_control::{bg_tile_map, window_tile_map};
use crate::palette::Palette;
use crate::{lcd_control, tiles};

use hw_constants::{MEM_MAP_SIZE, io_regs};

#[derive(Copy, Clone)]
pub struct Pixel {
    pub low: bool,
    pub high: bool,
    pub palette: Palette,
    pub priority: bool,
}

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
    state: FetcherState,
    pub drawing_window: bool,
    pub warmup: bool,
    bg_fifo: Vec<Pixel>,
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

    fn get_tile(&mut self, memory: &[u8], current_scanline: u8, window_y: u8) {
        let bg_second_tile_map = bg_tile_map(memory) && !self.drawing_window;
        let window_second_tile_map = window_tile_map(memory) && self.drawing_window;
        let second_tile_map = bg_second_tile_map || window_second_tile_map;

        let tile_x = if self.drawing_window {
            self.tile_x
        } else {
            ((memory[io_regs::SCX as usize] / 8) + self.tile_x) & 0x1F
        };

        let ly = if self.drawing_window {
            window_y
        } else {
            current_scanline.wrapping_add(memory[io_regs::SCY as usize])
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

    fn current_tile<'a>(&self, memory: &'a [u8]) -> &'a [u8; 16] {
        if lcd_control::bg_and_window_tiles(memory) {
            tiles::unsigned_nth_tile(memory, self.tile_id as usize)
        } else {
            tiles::signed_nth_tile(memory, self.tile_id.cast_signed() as isize)
        }
    }

    fn get_tile_data_low(&mut self, memory: &[u8]) {
        let tile = self.current_tile(memory);
        self.tile_data_low = tile[self.tile_line as usize * 2];
    }

    fn get_tile_data_high(&mut self, memory: &[u8]) {
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
            FetcherState::BeforeGetTileDataHigh => FetcherState::GetTileDataHigh,
            FetcherState::GetTileDataHigh => {
                self.get_tile_data_high(memory);
                // First fetch of the line. Restart and waste six cycles for some reason. :)
                if self.warmup {
                    self.warmup = false;
                    FetcherState::BeforeGetTile
                } else {
                    FetcherState::Push
                }
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
