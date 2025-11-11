use std::collections::VecDeque;
use crate::bg_fetcher::{FetcherState, Pixel};
use crate::tiles;
use crate::oam::Obj;

pub struct ObjectFetcher {
    prev_obj: Option<Obj>,
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
    fn push(&mut self) -> bool {
        let space_remaining = self.fifo.capacity() - self.fifo.len();

        for nth_bit in (0..space_remaining).rev() {
            let pixel = Pixel {
                low: (self.tile_data_low >> nth_bit) & 1 == 1,
                high: (self.tile_data_high >> nth_bit) & 1 == 1,
            };

            self.fifo.push_back(pixel);
        }
        true
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
        if self.prev_obj.is_none_or(|prev_obj| prev_obj != obj) {
            self.prev_obj = Some(obj);
            self.state = FetcherState::GetTile;
            self.done = false;
            self.ticks = 0;
        }
        if self.done {
            return;
        }
        self.ticks += 1;

        if let FetcherState::Push = self.state && self.push() {
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
                _ => {},
            }
        }
    }
}

impl Default for ObjectFetcher {
    fn default() -> Self {
        Self {
            prev_obj: None,
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