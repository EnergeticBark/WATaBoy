use std::collections::VecDeque;
use crate::{lcd, tiles};
use crate::lcd::{bg_tile_map, window_tile_map};

const SCY: usize = 0xFF42;
const SCX: usize = 0xFF43;

const WX: usize = 0xFF4B;

pub struct Pixel {
    pub low: bool,
    pub high: bool,
}

enum FetcherState {
    GetTile,
    GetTileDataLow,
    GetTileDataHigh,
    Sleep,
    Push,
}

pub struct PixelFetcher {
    state: FetcherState,
    pub drawing_window: bool,
    ticks: u8,
    bg_fifo: VecDeque<Pixel>,
    tile_id: u8,
    tile_line: u8,
    tile_x: u8,
    second_tile_map: bool,
    tile_data_low: u8,
    tile_data_high: u8,
}

impl PixelFetcher {
    // Shift out a pixel from the background FIFO, if it contains more than 8 pixels.
    pub fn shift_out(&mut self) -> Option<Pixel> {
        if self.bg_fifo.len() < 8 {
            None
        } else {
            Some(self.bg_fifo.pop_front()?)
        }
    }

    // Push a row of 8 pixels from a tile to the background FIFO, if its empty.
    fn push(&mut self) -> bool {
        if self.bg_fifo.len() > 8 {
            return false;
        }

        for nth_bit in (0..8).rev() {
            let pixel = Pixel {
                low: (self.tile_data_low >> nth_bit) & 1 == 1,
                high: (self.tile_data_high >> nth_bit) & 1 == 1,
            };

            self.bg_fifo.push_back(pixel);
        }
        true
    }

    pub fn clear(&mut self) {
        self.bg_fifo.clear();
    }

    fn get_tile(&mut self, memory: &[u8], x_coord: u8, current_scanline: u8, window_y: u8) {
        let bg_second_tile_map = bg_tile_map(memory) && !self.drawing_window;
        let window_second_tile_map = window_tile_map(memory) && self.drawing_window;
        self.second_tile_map = bg_second_tile_map || window_second_tile_map;

        let tile_x = if self.drawing_window {
            (x_coord - memory[WX]) / 8
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
        self.tile_x += 1;

        // TODO: If VRAM is blocked tile index is 0xFF...
    }

    fn get_tile_data_low(&mut self, memory: &[u8]) {
        let tile = if lcd::bg_and_window_tiles(memory) {
            tiles::unsigned_nth_tile(memory, self.tile_id as usize)
        } else {
            tiles::signed_nth_tile(memory, self.tile_id.cast_signed() as isize)
        };

        self.tile_data_low = tile[self.tile_line as usize * 2];
    }

    fn get_tile_data_high(&mut self, memory: &[u8]) {
        let tile = if lcd::bg_and_window_tiles(memory) {
            tiles::unsigned_nth_tile(memory, self.tile_id as usize)
        } else {
            tiles::signed_nth_tile(memory, self.tile_id.cast_signed() as isize)
        };

        self.tile_data_high = tile[self.tile_line as usize * 2 + 1];
    }

    pub fn tick(&mut self, memory: &[u8], x_coord: u8, current_scanline: u8, window_y: u8) {
        self.ticks += 1;
        if self.ticks < 2 {
            return;
        }
        self.ticks = 0;

        match self.state {
            FetcherState::GetTile => {
                self.get_tile(memory, x_coord, current_scanline, window_y);
                self.state = FetcherState::GetTileDataLow;
            }
            FetcherState::GetTileDataLow => {
                self.get_tile_data_low(memory);
                self.state = FetcherState::GetTileDataHigh;
            }
            FetcherState::GetTileDataHigh => {
                self.get_tile_data_high(memory);
                self.push();
                self.state = FetcherState::Sleep;
            }
            FetcherState::Sleep => self.state = FetcherState::Push,
            FetcherState::Push => {
                // TODO: Do this every tick instead of every 2
                if self.push() {
                    self.state = FetcherState::GetTile;
                }
            }
        }
    }
}

impl Default for PixelFetcher {
    fn default() -> Self {
        Self {
            state: FetcherState::GetTile,
            drawing_window: false,
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