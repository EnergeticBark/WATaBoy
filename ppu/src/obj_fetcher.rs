use crate::bg_fetcher::{FetcherState, Pixel};
use crate::oam::Obj;
use crate::tiles;

use std::collections::VecDeque;

pub struct ObjectFetcher {
    current_obj: Option<Obj>,
    pub done: bool,
    pub state: FetcherState,
    ticks: u8,
    pub fifo: VecDeque<Pixel>,
    tile_id: u8,
    tile_line: u8,
    tile_data_low: u8,
    tile_data_high: u8,
}

/* The ObjectFetcher has no "clear" or "reset" method as it stands right now.
   When a fresh FIFO queue is needed just make a whole new ObjectFetcher. As far as I can tell none
   of the state gets carried over anyway. Write some good comments if anything ends up contradicting
   this.
*/
impl ObjectFetcher {
    // Shift out a pixel from the background FIFO, if it contains more than 8 pixels.
    pub fn shift_out(&mut self) -> Option<Pixel> {
        self.fifo.pop_front()
    }

    // Push a row of 8 pixels from a tile to the background FIFO, if its empty.
    fn push(&mut self) {
        if self.current_obj.unwrap().x_flip() {
            self.push_bit_range(0..8)
        } else {
            self.push_bit_range((0..8).rev())
        };
    }

    fn push_bit_range<T: Iterator<Item = u8>>(&mut self, bit_range: T) {
        for nth_bit in bit_range.skip(self.fifo.len()) {
            let pixel = Pixel {
                low: (self.tile_data_low >> nth_bit) & 1 == 1,
                high: (self.tile_data_high >> nth_bit) & 1 == 1,
                priority: self.current_obj.unwrap().priority(),
            };

            self.fifo.push_back(pixel);
        }
    }

    fn get_tile(&mut self, current_scanline: u8, obj: Obj) {
        self.tile_id = obj.tile_index;
        self.tile_line = (current_scanline + 16 - obj.y_pos) % 8;
    }

    fn current_tile<'a>(&self, memory: &'a [u8]) -> &'a [u8; 16] {
        tiles::unsigned_nth_tile(memory, self.tile_id as usize)
    }

    fn get_tile_data_low(&mut self, memory: &[u8]) {
        let tile = self.current_tile(memory);
        self.tile_data_low = tile[self.tile_line as usize * 2];
    }

    fn get_tile_data_high(&mut self, memory: &[u8]) {
        let tile = self.current_tile(memory);
        self.tile_data_high = tile[self.tile_line as usize * 2 + 1];
    }

    pub fn tick(&mut self, memory: &[u8], current_scanline: u8, obj: Obj) {
        if self.current_obj.is_none_or(|prev_obj| prev_obj != obj) {
            self.current_obj = Some(obj);
            self.state = FetcherState::GetTile;
            self.done = false;
            self.ticks = 0;
        }
        if self.done {
            return;
        }
        self.ticks += 1;

        if let FetcherState::Push = self.state {
            self.push();
            self.done = true;
        }

        if self.ticks >= 2 {
            self.ticks = 0;
            match self.state {
                FetcherState::GetTile => {
                    self.get_tile(current_scanline, obj);
                    self.state = FetcherState::GetTileDataLow;
                }
                FetcherState::GetTileDataLow => {
                    self.get_tile_data_low(memory);
                    self.state = FetcherState::GetTileDataHigh;
                }
                FetcherState::GetTileDataHigh => {
                    self.get_tile_data_high(memory);
                    self.state = FetcherState::Push;
                }
                _ => {}
            }
        }
    }
}

impl Default for ObjectFetcher {
    fn default() -> Self {
        Self {
            current_obj: None,
            done: true,
            state: FetcherState::GetTile,
            ticks: 0,
            fifo: VecDeque::with_capacity(8),
            tile_id: 0,
            tile_line: 0,
            tile_data_low: 0,
            tile_data_high: 0,
        }
    }
}
