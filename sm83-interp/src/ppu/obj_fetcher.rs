use std::collections::VecDeque;

use hw_constants::VRAM_SIZE;

use super::bg_fetcher::Pixel;
use super::oam::Obj;
use super::palette::Palette;
use super::registers::LcdControl;
use super::tiles;

// From ObjectFetcher's perspective, its pixel FIFO always contains 8 pixels.
// Before an object is pushed to the queue, transparent pixels are pushed to the back to maintain a length of 8 pixels.
// Any transparent pixels can be overwritten by opaque object pixels in the push() function.

pub const TRANSPARENT: Pixel = Pixel {
    low: false,
    high: false,
    palette: Palette::Obp0,
    priority: false,
};

#[derive(Debug)]
pub enum ObjectFetcherState {
    Idle,
    GetTile {
        ticks_remaining: u8,
        obj: Obj,
    },
    GetTileDataLow {
        ticks_remaining: u8,
        obj: Obj,
        obj_line: u8,
    },
    GetTileDataHigh {
        ticks_remaining: u8,
        obj: Obj,
        obj_line: u8,
    },
    Push {
        obj: Obj,
    },
}

pub struct ObjectFetcher {
    obj_buffer: VecDeque<Obj>,
    pub state: ObjectFetcherState,
    pub fifo: VecDeque<Pixel>,
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
        self.fifo.pop_front()
    }

    // Push a row of 8 pixels from a tile to the Obj FIFO.
    fn push(&mut self, obj: Obj) {
        if obj.attributes.x_flip() {
            self.push_bit_range(0..8, obj);
        } else {
            self.push_bit_range((0..8).rev(), obj);
        }
    }

    fn push_bit_range<T: Iterator<Item = u8>>(&mut self, bit_range: T, obj: Obj) {
        // Fill the back of the queue with transparent pixels.
        while self.fifo.len() < 8 {
            self.fifo.push_back(TRANSPARENT);
        }

        let old_pixels = self.fifo.iter_mut();
        let new_pixels = bit_range.map(|nth_bit| Pixel {
            low: (self.tile_data_low >> nth_bit) & 1 == 1,
            high: (self.tile_data_high >> nth_bit) & 1 == 1,
            palette: {
                if obj.attributes.palette() {
                    Palette::Obp1
                } else {
                    Palette::Obp0
                }
            },
            priority: obj.attributes.priority(),
        });
        // Replace any transparent pixels that are currently on the queue with the new pixels.
        for (old, new) in old_pixels.zip(new_pixels) {
            if !(old.low || old.high) {
                *old = new;
            }
        }
    }

    // Returns obj_line
    fn get_tile(current_scanline: u8, obj: Obj, obj_size: bool) -> u8 {
        let obj_line = current_scanline + 16 - obj.y_pos;
        // If the object isn't flipped vertically, just return the line.
        if !obj.attributes.y_flip() {
            return obj_line;
        }

        // If the object is flipped we need to subtract from its height - 1.
        if obj_size {
            15 - obj_line
        } else {
            7 - obj_line
        }
    }

    fn current_tile(
        vram: &[u8; VRAM_SIZE as usize],
        lcdc: LcdControl,
        obj: Obj,
        obj_line: u8,
    ) -> &[u8; 16] {
        let mut tile_index = obj.tile_index;
        if lcdc.obj_size() {
            // Override the first bit as described in PanDocs.
            // See: https://gbdev.io/pandocs/OAM.html#byte-2--tile-index
            if obj_line < 8 {
                tile_index &= !0b0000_0001;
            } else {
                tile_index |= 0b0000_0001;
            }
        }

        tiles::unsigned_nth_tile(vram, tile_index as usize)
    }

    fn get_tile_data_low(
        &mut self,
        vram: &[u8; VRAM_SIZE as usize],
        lcdc: LcdControl,
        obj: Obj,
        obj_line: u8,
    ) {
        let tile = Self::current_tile(vram, lcdc, obj, obj_line);
        let tile_line = obj_line % 8;
        self.tile_data_low = tile[tile_line as usize * 2];
    }

    fn get_tile_data_high(
        &mut self,
        vram: &[u8; VRAM_SIZE as usize],
        lcdc: LcdControl,
        obj: Obj,
        obj_line: u8,
    ) {
        let tile = Self::current_tile(vram, lcdc, obj, obj_line);
        let tile_line = obj_line % 8;
        self.tile_data_high = tile[tile_line as usize * 2 + 1];
    }

    pub fn tick(
        &mut self,
        vram: &[u8; VRAM_SIZE as usize],
        lcdc: LcdControl,
        current_scanline: u8,
    ) {
        match self.state {
            ObjectFetcherState::Idle => {
                if let Some(obj) = self.obj_buffer.pop_front() {
                    // This used to be ticks_remaining: 1. But Idle is taking its own tick, so it's
                    // probably more accurate to use ticks_remaining: 0 so GetTile finishes after
                    // two ticks instead of three.
                    self.state = ObjectFetcherState::GetTile {
                        ticks_remaining: 0,
                        obj,
                    };
                }
            }
            ObjectFetcherState::GetTile {
                ticks_remaining: 0,
                obj,
            } => {
                let obj_line = Self::get_tile(current_scanline, obj, lcdc.obj_size());
                self.state = ObjectFetcherState::GetTileDataLow {
                    ticks_remaining: 1,
                    obj,
                    obj_line,
                };
            }
            ObjectFetcherState::GetTileDataLow {
                ticks_remaining: 0,
                obj,
                obj_line,
            } => {
                self.get_tile_data_low(vram, lcdc, obj, obj_line);
                self.state = ObjectFetcherState::GetTileDataHigh {
                    ticks_remaining: 1,
                    obj,
                    obj_line,
                };
            }
            ObjectFetcherState::GetTileDataHigh {
                ticks_remaining: 0,
                obj,
                obj_line,
            } => {
                self.get_tile_data_high(vram, lcdc, obj, obj_line);
                self.state = ObjectFetcherState::Push { obj };
            }
            ObjectFetcherState::Push { obj } => {
                self.push(obj);
                self.state = if let Some(obj) = self.obj_buffer.pop_front() {
                    ObjectFetcherState::GetTile {
                        ticks_remaining: 0,
                        obj,
                    }
                } else {
                    ObjectFetcherState::Idle
                };
            }

            // Countdown
            ObjectFetcherState::GetTile {
                ref mut ticks_remaining,
                ..
            }
            | ObjectFetcherState::GetTileDataLow {
                ref mut ticks_remaining,
                ..
            }
            | ObjectFetcherState::GetTileDataHigh {
                ref mut ticks_remaining,
                ..
            } => *ticks_remaining -= 1,
        }
    }
}

impl Default for ObjectFetcher {
    fn default() -> Self {
        Self {
            obj_buffer: VecDeque::with_capacity(10),
            state: ObjectFetcherState::Idle,
            fifo: VecDeque::with_capacity(8),
            tile_data_low: 0,
            tile_data_high: 0,
        }
    }
}
