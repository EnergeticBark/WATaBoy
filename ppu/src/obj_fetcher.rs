use crate::bg_fetcher::Pixel;
use crate::oam::Obj;
use crate::tiles;

use std::collections::VecDeque;
use crate::palette::Palette;

// The ObjectFetcher's pixel FIFO always contains 8 pixels.
// Each time a pixel is popped from the queue, a transparent pixel is pushed to the back of the
// queue to maintain a constant length of 8 pixels.
// Any transparent pixels can be overwritten by opaque object pixels in the push() function.

const TRANSPARENT: Pixel = Pixel {
    low: false,
    high: false,
    palette: Palette::OBP0,
    priority: false,
};

enum ObjectFetcherState {
    Idle,
    GetTile { ticks_remaining: u8, obj: Obj },
    GetTileDataLow { ticks_remaining: u8, obj: Obj, tile_line: u8 },
    GetTileDataHigh { ticks_remaining: u8, obj: Obj, tile_line: u8 },
    Push { obj: Obj },
}

pub struct ObjectFetcher {
    obj_buffer: VecDeque<Obj>,
    state: ObjectFetcherState,
    fifo: VecDeque<Pixel>,
    tile_data_low: u8,
    tile_data_high: u8,
}

/* The ObjectFetcher has no "clear" or "reset" method as it stands right now.
   When a fresh FIFO queue is needed just make a whole new ObjectFetcher. As far as I can tell none
   of the state gets carried over anyway. Write some good comments if anything ends up contradicting
   this.
*/
impl ObjectFetcher {
    pub fn push_obj(&mut self, obj: Obj) {
        self.obj_buffer.push_back(obj);
    }

    pub fn idle_and_empty(&self) -> bool {
        matches!(self.state, ObjectFetcherState::Idle) && self.obj_buffer.is_empty()
    }

    // Shift out a pixel from the Obj FIFO.
    pub fn shift_out(&mut self) -> Option<Pixel> {
        let front = self.fifo.pop_front();
        self.fifo.push_back(TRANSPARENT);

        front
    }

    // Push a row of 8 pixels from a tile to the Obj FIFO.
    fn push(&mut self, obj: Obj) {
        if obj.x_flip() {
            self.push_bit_range(0..8, obj)
        } else {
            self.push_bit_range((0..8).rev(), obj)
        };
    }

    fn push_bit_range<T: Iterator<Item = u8>>(&mut self, bit_range: T, obj: Obj) {
        let old_pixels = self.fifo.iter_mut();
        let new_pixels = bit_range.map(|nth_bit| {
            Pixel {
                low: (self.tile_data_low >> nth_bit) & 1 == 1,
                high: (self.tile_data_high >> nth_bit) & 1 == 1,
                palette: {
                    if obj.palette() {
                        Palette::OBP1
                    } else {
                        Palette::OBP0
                    }
                },
                priority: obj.priority(),
            }
        });
        // Replace any transparent pixels that are currently on the queue with the new pixels.
        for (old, new) in old_pixels.zip(new_pixels) {
            if !(old.low || old.high) {
                *old = new
            }
        }
    }

    // Returns tile_line
    fn get_tile(&self, current_scanline: u8, obj: Obj) -> u8 {
        let tile_line = (current_scanline + 16 - obj.y_pos) % 8;
        // Handle vertical object flipping.
        if !obj.y_flip() {
            tile_line
        } else {
            7 - tile_line
        }
    }

    fn current_tile<'a>(&self, memory: &'a [u8], obj: Obj) -> &'a [u8; 16] {
        tiles::unsigned_nth_tile(memory, obj.tile_index as usize)
    }

    fn get_tile_data_low(&mut self, memory: &[u8], obj: Obj, tile_line: u8) {
        let tile = self.current_tile(memory, obj);
        self.tile_data_low = tile[tile_line as usize * 2];
    }

    fn get_tile_data_high(&mut self, memory: &[u8], obj: Obj, tile_line: u8) {
        let tile = self.current_tile(memory, obj);
        self.tile_data_high = tile[tile_line as usize * 2 + 1];
    }

    pub fn tick(&mut self, memory: &[u8], current_scanline: u8) {
        match self.state {
            ObjectFetcherState::Idle => {
                if let Some(obj) = self.obj_buffer.pop_front() {
                    // This used to be ticks_remaining: 1. But Idle is taking its own tick, so it's
                    // probably more accurate to use ticks_remaining: 0 so GetTile finishes after
                    // two ticks instead of three.
                    self.state = ObjectFetcherState::GetTile { ticks_remaining: 0, obj };
                }
            }
            ObjectFetcherState::GetTile { ticks_remaining: 0, obj } => {
                let tile_line = self.get_tile(current_scanline, obj);
                self.state = ObjectFetcherState::GetTileDataLow { ticks_remaining: 1, obj, tile_line };
            }
            ObjectFetcherState::GetTileDataLow { ticks_remaining: 0, obj, tile_line } => {
                self.get_tile_data_low(memory, obj, tile_line);
                self.state = ObjectFetcherState::GetTileDataHigh { ticks_remaining: 1, obj, tile_line };
            }
            ObjectFetcherState::GetTileDataHigh { ticks_remaining: 0, obj, tile_line } => {
                self.get_tile_data_high(memory, obj, tile_line);
                self.state = ObjectFetcherState::Push { obj };
            }
            ObjectFetcherState::Push { obj } => {
                self.push(obj);
                self.state = ObjectFetcherState::Idle;
            }

            // Countdown
            ObjectFetcherState::GetTile { ref mut ticks_remaining, .. } => *ticks_remaining -= 1,
            ObjectFetcherState::GetTileDataLow { ref mut ticks_remaining, .. } => *ticks_remaining -= 1,
            ObjectFetcherState::GetTileDataHigh { ref mut ticks_remaining, .. } => *ticks_remaining -= 1,
        }
    }
}

impl Default for ObjectFetcher {
    fn default() -> Self {
        Self {
            obj_buffer: VecDeque::with_capacity(10),
            state: ObjectFetcherState::Idle,
            fifo: VecDeque::from([TRANSPARENT; 8]),
            tile_data_low: 0,
            tile_data_high: 0,
        }
    }
}
