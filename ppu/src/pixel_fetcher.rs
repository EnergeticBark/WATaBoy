use std::collections::VecDeque;
use crate::{lcd, tiles};
use crate::lcd::{bg_tile_map, window_tile_map};

const SCY: usize = 0xFF42;
const SCX: usize = 0xFF43;

pub struct Pixel {
    pub low: bool,
    pub high: bool,
}

enum FetcherState {
    GetTile,
    GetTileDataLow,
    GetTileDataHigh,
    Push,
}

pub struct PixelFetcher {
    state: FetcherState,
    pub drawing_window: bool,
    pub warmup: bool,
    ticks: u8,
    pub bg_fifo: VecDeque<Pixel>,
    tile_id: u8,
    tile_line: u8,
    tile_x: u8,
    second_tile_map: bool,
    tile_data_low: u8,
    tile_data_high: u8,
}

/* The PixelFetcher has no "clear" or "reset" method as it stands right now.
   When a fresh FIFO queue is needed just make a whole new PixelFetcher. As far as I can tell none
   of the state gets carried over anyway. Write some good comments if anything ends up contradicting
   this.
 */
impl PixelFetcher {
    // Shift out a pixel from the background FIFO, if it contains more than 8 pixels.
    pub fn shift_out(&mut self) -> Option<Pixel> {
        self.bg_fifo.pop_front()
    }

    // Push a row of 8 pixels from a tile to the background FIFO, if its empty.
    fn push(&mut self) -> bool {
        if !self.bg_fifo.is_empty() {
            return false;
        }

        for nth_bit in (0..8).rev() {
            let pixel = Pixel {
                low: (self.tile_data_low >> nth_bit) & 1 == 1,
                high: (self.tile_data_high >> nth_bit) & 1 == 1,
            };

            self.bg_fifo.push_back(pixel);
        }
        self.tile_x += 1;
        true
    }

    fn get_tile(&mut self, memory: &[u8], current_scanline: u8, window_y: u8) {
        let bg_second_tile_map = bg_tile_map(memory) && !self.drawing_window;
        let window_second_tile_map = window_tile_map(memory) && self.drawing_window;
        self.second_tile_map = bg_second_tile_map || window_second_tile_map;

        let tile_x = if self.drawing_window {
            self.tile_x
        } else {
            ((memory[SCX] / 8) + self.tile_x) & 0x1F
        };

        let ly = if self.drawing_window {
            window_y
        } else {
            current_scanline + memory[SCY]
        };
        let tile_y = ly / 8;

        let tile_map = if self.second_tile_map {
            tiles::tile_map_1(memory)
        } else {
            tiles::tile_map_0(memory)
        };

        self.tile_id = tile_map[tile_y as usize * 32 + tile_x as usize];
        self.tile_line = ly % 8;

        // TODO: If VRAM is blocked tile index is 0xFF...
    }

    fn current_tile<'a>(&self, memory: &'a [u8]) -> &'a [u8; 16] {
        if lcd::bg_and_window_tiles(memory) {
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

    pub fn tick(&mut self, memory: &[u8], current_scanline: u8, window_y: u8) {
        self.ticks += 1;

        if let FetcherState::Push = self.state && self.push() {
            self.ticks = 0;
            self.state = FetcherState::GetTile;
        }

        if self.ticks >= 2 {
            self.ticks = 0;
            match self.state {
                FetcherState::GetTile => {
                    self.get_tile(memory, current_scanline, window_y);
                    self.state = FetcherState::GetTileDataLow;
                }
                FetcherState::GetTileDataLow => {
                    self.get_tile_data_low(memory);
                    self.state = FetcherState::GetTileDataHigh;
                }
                FetcherState::GetTileDataHigh => {
                    self.get_tile_data_high(memory);
                    // First fetch of the line. Restart and waste six cycles for some reason. :)
                    if self.warmup {
                        self.state = FetcherState::GetTile;
                        self.warmup = false;
                    } else {
                        self.state = FetcherState::Push;
                    }
                }
                _ => {},
            }
        }
    }
}

impl Default for PixelFetcher {
    fn default() -> Self {
        Self {
            state: FetcherState::GetTile,
            drawing_window: false,
            warmup: true,
            ticks: 0,
            bg_fifo: VecDeque::with_capacity(16),
            tile_id: 0,
            tile_line: 0,
            tile_x: 0,
            second_tile_map: false,
            tile_data_low: 0,
            tile_data_high: 0,
        }
    }
}