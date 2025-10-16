use eframe::epaint::textures::TextureOptions;
use eframe::epaint::{ColorImage, TextureHandle};
use egui::Ui;
use egui_extras::{Column, TableBody, TableBuilder};
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

fn greyscale_from_tile(buffer: &[u8]) -> Vec<u8> {
    // Each tile consists of 16 bytes.
    assert_eq!(buffer.len(), 16);
    let (chunks, _) = buffer.as_chunks::<2>();
    chunks
        .iter()
        .flat_map(|&[least_significant, most_significant]| {
            (0..8)
                .map(move |nth_bit| {
                    let mut palette_index = 0;
                    if least_significant >> nth_bit & 1 == 1 {
                        palette_index += 1;
                    }
                    if most_significant >> nth_bit & 1 == 1 {
                        palette_index += 2;
                    }
                    palette_index
                })
                .rev()
        })
        // Bring the range of values from 0-3 to 0-255.
        .map(|palette_index| palette_index * 64)
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
                        ColorImage::from_gray([TILE_SIZE, TILE_SIZE], &[0; 64]),
                        TextureOptions::NEAREST,
                    )
                });

                let start_byte = 0x8000 + (row_index + i) * 16;
                let end_byte = start_byte + 16;
                tile.set(
                    ColorImage::from_gray(
                        [TILE_SIZE, TILE_SIZE],
                        &greyscale_from_tile(&dmg_state.memory[start_byte..end_byte]),
                    ),
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
