use eframe::emath::Rect;
use eframe::epaint::textures::TextureOptions;
use eframe::epaint::{ColorImage, TextureHandle};
use egui::emath::RectTransform;
use egui::{Color32, Frame, Stroke, StrokeKind, Ui, Vec2, pos2};
use hw_constants::io_regs;
use ppu::{lcd, tiles};
use sm83_interp::cpu::Cpu;

fn draw_tile_map(
    ui: &mut Ui,
    rect: Rect,
    tile_map_texture: &mut TextureHandle,
    tile_map: &[u8; 0x0400],
    dmg_state: &Cpu,
) {
    let mut tile_map_bitmap: Vec<Vec<u8>> = vec![vec![0; 256]; 256];

    for row in 0..32 {
        for column in 0..32 {
            let tile_id = tile_map[row * 32 + column];

            let tile_data = if lcd::bg_and_window_tiles(&dmg_state.memory.buffer) {
                tiles::unsigned_nth_tile(&dmg_state.memory.buffer, tile_id as usize)
            } else {
                tiles::signed_nth_tile(&dmg_state.memory.buffer, tile_id.cast_signed() as isize)
            };

            let greyscale_tile = crate::tiles::greyscale_from_tile(tile_data);

            for (tile_row, chunk) in greyscale_tile.chunks_exact(8).enumerate() {
                tile_map_bitmap[row * 8 + tile_row]
                    .as_mut_array::<256>()
                    .unwrap()[column * 8..column * 8 + 8]
                    .copy_from_slice(chunk);
            }
        }
    }
    let greyscale_tilemap: Vec<_> = tile_map_bitmap.into_iter().flatten().collect();

    let greyscale = ColorImage::from_gray([256, 256], &greyscale_tilemap);
    tile_map_texture.set(greyscale, TextureOptions::NEAREST);

    egui::Image::from_texture(&*tile_map_texture).paint_at(ui, rect);
}

fn highlight_background(ui: &mut Ui, to_screen: RectTransform, dmg_state: &Cpu) {
    // Draw a red rectangle around the visible portion of the background tile map.
    let sc_y = dmg_state.memory[io_regs::SCY];
    let sc_x = dmg_state.memory[io_regs::SCX];
    let bottom = f32::from(sc_y.wrapping_add(143));
    let right = f32::from(sc_x.wrapping_add(159));
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
}

fn highlight_window(ui: &mut Ui, to_screen: RectTransform, dmg_state: &Cpu) {
    // Draw a blue rectangle around the visible portion of the window tile map.
    let wy = dmg_state.memory[io_regs::WY];
    let wx = dmg_state.memory[io_regs::WX].wrapping_sub(7);
    let bottom = f32::from(143u8.wrapping_sub(wy));
    let right = f32::from(159u8.wrapping_sub(wx));
    let visible = Rect::from_min_max(
        to_screen * pos2(right - 160.0, bottom - 144.0),
        to_screen * pos2(right, bottom),
    );

    ui.painter().rect_stroke(
        visible,
        0.0,
        Stroke::new(2.0, Color32::BLUE),
        StrokeKind::Middle,
    );
}

pub fn draw_tile_map_0(ui: &mut Ui, tile_map_0_texture: &mut TextureHandle, dmg_state: &Cpu) {
    ui.vertical(|ui| {
        ui.heading("Tile Map 0: 0x9800-0x9C00");

        let tile_map = tiles::tile_map_0(&dmg_state.memory.buffer);

        Frame::canvas(ui.style()).show(ui, |ui| {
            let (_, rect) = ui.allocate_space(Vec2::new(512.0, 512.0));
            let to_screen =
                RectTransform::from_to(Rect::from_x_y_ranges(0.0..=255.0, 0.0..=255.0), rect);

            draw_tile_map(ui, rect, tile_map_0_texture, tile_map, dmg_state);

            if lcd::bg_and_window_enabled(&dmg_state.memory.buffer) {
                if !lcd::bg_tile_map(&dmg_state.memory.buffer) {
                    highlight_background(ui, to_screen, dmg_state);
                }
                if !lcd::window_tile_map(&dmg_state.memory.buffer)
                    && lcd::window_enabled(&dmg_state.memory.buffer)
                {
                    highlight_window(ui, to_screen, dmg_state);
                }
            }
        });
    });
}

pub fn draw_tile_map_1(ui: &mut Ui, tile_map_1_texture: &mut TextureHandle, dmg_state: &Cpu) {
    ui.vertical(|ui| {
        ui.heading("Tile Map 1: 0x9C00-0xA000");

        let tile_map = tiles::tile_map_1(&dmg_state.memory.buffer);

        Frame::canvas(ui.style()).show(ui, |ui| {
            let (_, rect) = ui.allocate_space(Vec2::new(512.0, 512.0));
            let to_screen =
                RectTransform::from_to(Rect::from_x_y_ranges(0.0..=255.0, 0.0..=255.0), rect);

            draw_tile_map(ui, rect, tile_map_1_texture, tile_map, dmg_state);

            if lcd::bg_and_window_enabled(&dmg_state.memory.buffer) {
                if lcd::bg_tile_map(&dmg_state.memory.buffer) {
                    highlight_background(ui, to_screen, dmg_state);
                }
                if lcd::window_tile_map(&dmg_state.memory.buffer)
                    && lcd::window_enabled(&dmg_state.memory.buffer)
                {
                    highlight_window(ui, to_screen, dmg_state);
                }
            }
        });
    });
}
