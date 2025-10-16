use eframe::epaint::{ColorImage, TextureHandle};
use eframe::epaint::textures::TextureOptions;
use egui::{Color32, Ui};
use ppu::tiles;
use sm83_interp::cpu::Cpu;

const TILE_MAP_SIZE: usize = 256;

fn greyscale_from_tile(tile: &[u8; 16]) -> Vec<u8> {
    tiles::tile_to_palette_indices(tile)
        .into_iter()
        // Bring the range of values from 0-3 to 0-255.
        .map(|palette_index| palette_index.value() * 64)
        .collect()
}

pub fn draw_tile_map(ui: &mut Ui, ctx: &egui::Context, tile_map: &mut Option<TextureHandle>, dmg_state: &Cpu) {
    let tile_map = tile_map.get_or_insert_with(|| {
        ctx.load_texture(
            "Tile Map",
            ColorImage::filled([TILE_MAP_SIZE, TILE_MAP_SIZE], Color32::BLACK),
            TextureOptions::NEAREST,
        )
    });


    for row in 0..32 {
        for column in 0..32 {
            let tile_id = tiles::tile_map(&dmg_state.memory)[row * 32 + column];
            let tile_data = tiles::unsigned_nth_tile(&dmg_state.memory, tile_id as usize);
            let greyscale = ColorImage::from_gray(
                [8, 8],
                &greyscale_from_tile(tile_data),
            );

            tile_map.set_partial(
                [8 * column, 8 * row],
                greyscale,
                TextureOptions::NEAREST,
            );
        }
    }

    ui.add(
        egui::Image::from_texture(&*tile_map).fit_to_original_size(2.0),
    );
}