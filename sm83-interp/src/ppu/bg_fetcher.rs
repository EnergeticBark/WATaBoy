use hw_constants::VRAM_SIZE;

use super::palette::PaletteSelect;
use super::registers::LcdControl;
use super::tiles;

use bitfield_struct::bitfield;

#[bitfield(u8, order = Msb)]
pub struct ColorIndex {
    #[bits(6)]
    __: u8, // Padding
    #[bits(1)]
    pub high: bool,
    #[bits(1)]
    pub low: bool,
}

#[bitfield(u8, order = Msb)]
pub struct Pixel {
    #[bits(3)]
    __: u8, // Padding
    #[bits(1)]
    pub priority: bool,
    #[bits(2)]
    pub palette: PaletteSelect,
    #[bits(2)]
    pub color_index: ColorIndex,
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

impl BackgroundFetcher {
    // Having a reset function is quicker than calling default, because nothing will be reallocated.
    pub fn reset(&mut self) {
        self.state = FetcherState::BeforeGetTile;
        self.drawing_window = false;
        self.warmup = true;
        self.bg_fifo.clear();
        self.tile_id = 0;
        self.tile_line = 0;
        self.tile_x = 0;
        self.tile_data_low = 0;
        self.tile_data_high = 0;
    }

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
            let pixel = Pixel::new()
                .with_color_index(
                    ColorIndex::new()
                        .with_low((self.tile_data_low >> nth_bit) & 1 == 1)
                        .with_high((self.tile_data_high >> nth_bit) & 1 == 1),
                )
                .with_palette(PaletteSelect::Bgp)
                .with_priority(false);

            self.bg_fifo.push(pixel);
        }
        self.tile_x += 1;
        true
    }

    fn get_tile(
        &mut self,
        vram: &[u8; VRAM_SIZE as usize],
        lcdc: LcdControl,
        scx: u8,
        scy: u8,
        current_scanline: u8,
        window_y: u8,
    ) {
        let bg_second_tile_map = lcdc.bg_tile_map() && !self.drawing_window;
        let window_second_tile_map = lcdc.window_tile_map() && self.drawing_window;
        let second_tile_map = bg_second_tile_map || window_second_tile_map;

        let tile_x = if self.drawing_window {
            self.tile_x
        } else {
            (scx / 8 + self.tile_x) & 0x1F
        };

        let ly = if self.drawing_window {
            window_y
        } else {
            current_scanline.wrapping_add(scy)
        };
        let tile_y = ly / 8;

        let tile_map = if second_tile_map {
            tiles::tile_map_1(vram)
        } else {
            tiles::tile_map_0(vram)
        };

        self.tile_id = tile_map[tile_y as usize * 32 + tile_x as usize];
        self.tile_line = ly % 8;

        // TODO: If VRAM is blocked tile index is 0xFF...
    }

    fn current_tile<'a>(
        &self,
        vram: &'a [u8; VRAM_SIZE as usize],
        lcdc: LcdControl,
    ) -> &'a [u8; 16] {
        if lcdc.bg_and_window_tiles() {
            tiles::unsigned_nth_tile(vram, self.tile_id as usize)
        } else {
            tiles::signed_nth_tile(vram, self.tile_id.cast_signed() as isize)
        }
    }

    fn get_tile_data_low(&mut self, vram: &[u8; VRAM_SIZE as usize], lcdc: LcdControl) {
        let tile = self.current_tile(vram, lcdc);
        self.tile_data_low = tile[self.tile_line as usize * 2];
    }

    fn get_tile_data_high(&mut self, vram: &[u8; VRAM_SIZE as usize], lcdc: LcdControl) {
        let tile = self.current_tile(vram, lcdc);
        self.tile_data_high = tile[self.tile_line as usize * 2 + 1];
    }

    pub fn tick(
        &mut self,
        vram: &[u8; VRAM_SIZE as usize],
        lcdc: LcdControl,
        scx: u8,
        scy: u8,
        current_scanline: u8,
        window_y: u8,
    ) {
        self.state = match self.state {
            FetcherState::BeforeGetTile => FetcherState::GetTile,
            FetcherState::GetTile => {
                self.get_tile(vram, lcdc, scx, scy, current_scanline, window_y);
                FetcherState::BeforeGetTileDataLow
            }
            FetcherState::BeforeGetTileDataLow => FetcherState::GetTileDataLow,
            FetcherState::GetTileDataLow => {
                self.get_tile_data_low(vram, lcdc);
                FetcherState::BeforeGetTileDataHigh
            }
            FetcherState::BeforeGetTileDataHigh => {
                if self.warmup {
                    // First fetch of the line. Restart and waste six cycles (5 here, plus 1 discarding). :)
                    self.warmup = false;

                    // Fill the FIFO with 8 transparent pixels for overscan.
                    // These will be merged and discarded with any sprite pixels that are past the left edge of the LCD.
                    for _ in 0..8 {
                        let pixel = Pixel::new()
                            .with_color_index(ColorIndex::from_bits(0))
                            .with_palette(PaletteSelect::Bgp)
                            .with_priority(false);

                        self.bg_fifo.push(pixel);
                    }

                    FetcherState::BeforeGetTile
                } else {
                    FetcherState::GetTileDataHigh
                }
            }
            FetcherState::GetTileDataHigh => {
                self.get_tile_data_high(vram, lcdc);
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
