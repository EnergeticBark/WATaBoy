use eframe::epaint::textures::TextureOptions;
use eframe::epaint::{ColorImage, TextureHandle};
use egui::{Color32, Ui};
use egui_extras::{Column, TableBody, TableBuilder};
use ppu::tiles;
use sm83_interp::cpu::Cpu;

const TILE_SIZE: usize = 8;
const TILE_SCALE: usize = 4;

pub fn draw_tile_table(
    ui: &mut Ui,
    ctx: &egui::Context,
    tiles: &mut [Option<TextureHandle>],
    dmg_state: &Cpu,
) {
    ui.heading("Tile Data: 0x8000-0x9800");
    TableBuilder::new(ui)
        .id_salt("Tile View")
        .striped(true)
        .columns(Column::auto(), 8)
        .body(|body| {
            draw_tiles_body(body, ctx, tiles, dmg_state);
        });
}

pub fn greyscale_from_tile(tile: &[u8; 16]) -> Vec<u8> {
    tiles::tile_to_palette_indices(tile)
        .into_iter()
        // Bring the range of values from 0-3 to 0-255.
        .map(|palette_index| palette_index.value() * 64)
        .collect()
}

fn draw_tiles_body(
    body: TableBody<'_>,
    ctx: &egui::Context,
    tiles: &mut [Option<TextureHandle>],
    dmg_state: &Cpu,
) {
    body.rows(
        (TILE_SIZE * TILE_SCALE) as f32,
        tiles.len() / 8,
        |mut row| {
            let row_index = row.index() * 8;

            for i in 0..8 {
                let tile = tiles[row_index + i].get_or_insert_with(|| {
                    ctx.load_texture(
                        format!("Tile {}", row_index + i),
                        ColorImage::filled([TILE_SIZE, TILE_SIZE], Color32::BLACK),
                        TextureOptions::NEAREST,
                    )
                });

                let tile_data = tiles::unsigned_nth_tile(&dmg_state.memory.buffer, row_index + i);
                tile.set(
                    ColorImage::from_gray([TILE_SIZE, TILE_SIZE], &greyscale_from_tile(tile_data)),
                    TextureOptions::NEAREST,
                );

                row.col(|ui| {
                    ui.add(
                        egui::Image::from_texture(&*tile).fit_to_original_size(TILE_SCALE as f32),
                    );
                });
            }
        },
    );
}
