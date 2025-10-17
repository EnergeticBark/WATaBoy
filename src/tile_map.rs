use eframe::emath;
use eframe::emath::Rect;
use eframe::epaint::textures::TextureOptions;
use eframe::epaint::{ColorImage, TextureHandle};
use egui::{Color32, Frame, Stroke, StrokeKind, Ui, Vec2, pos2};
use ppu::tiles;
use sm83_interp::cpu::Cpu;

const TILE_MAP_SIZE: usize = 256;

pub fn draw_tile_map(
    ui: &mut Ui,
    ctx: &egui::Context,
    tile_map: &mut Option<TextureHandle>,
    dmg_state: &Cpu,
) {
    Frame::canvas(ui.style()).show(ui, |ui| {
        let (_, rect) = ui.allocate_space(Vec2::new(512.0, 512.0));
        let to_screen =
            emath::RectTransform::from_to(Rect::from_x_y_ranges(0.0..=255.0, 0.0..=255.0), rect);

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
                let greyscale =
                    ColorImage::from_gray([8, 8], &crate::tiles::greyscale_from_tile(tile_data));

                tile_map.set_partial([8 * column, 8 * row], greyscale, TextureOptions::NEAREST);
            }
        }

        egui::Image::from_texture(&*tile_map).paint_at(ui, rect);

        // Draw a red rectangle around the visible portion of the tile map.
        let sc_y = dmg_state.memory[0xFF42];
        let sc_x = dmg_state.memory[0xFF43];
        let bottom = ((sc_y as usize + 143) % 256) as f32;
        let right = ((sc_x as usize + 159) % 256) as f32;
        let visible = Rect::from_min_max(
            to_screen * pos2(right - 160.0, bottom - 144.0),
            to_screen * pos2(right, bottom),
        );

        ui.painter().rect_stroke(
            visible,
            0.0,
            Stroke::new(2.0, Color32::RED),
            StrokeKind::Middle,
        );
    });
}
