use eframe::emath::Rect;
use eframe::epaint::textures::TextureOptions;
use eframe::epaint::{ColorImage, TextureHandle};
use egui::emath::RectTransform;
use egui::{Color32, Frame, Stroke, StrokeKind, Ui, Vec2, pos2};
use hw_constants::TILE_MAP_SIZE;
use hw_constants::io_regs::{LCDC, SCX, SCY, WX, WY};
use interpreter::addressable::Addressable;
use interpreter::ppu::tiles;
use interpreter::ppu::{LcdControl, Ppu};

const TILE_MAP_SCALE: f32 = 1.0;

fn draw_tile_map(
    ui: &mut Ui,
    rect: Rect,
    tile_map_texture: &mut TextureHandle,
    tile_map: &[u8; 0x0400],
    ppu: &Ppu,
) {
    let mut tile_map_bitmap: Vec<u8> = vec![0; 256 * 256];

    for row in 0..32 {
        for column in 0..32 {
            let tile_id = tile_map[row * 32 + column];

            let lcdc = LcdControl::from_bits(ppu.read_byte(LCDC, ppu.clock));

            let vram = ppu.vram.as_array().unwrap();
            let tile_data = if lcdc.bg_and_window_tiles() {
                tiles::unsigned_nth_tile(vram, tile_id as usize)
            } else {
                tiles::signed_nth_tile(vram, tile_id.cast_signed() as isize)
            };

            let greyscale_tile = crate::tiles::greyscale_from_tile(tile_data);

            for (tile_row, chunk) in greyscale_tile.chunks_exact(8).enumerate() {
                let index = 256 * (row * 8 + tile_row) + column * 8;
                tile_map_bitmap[index..index + 8].copy_from_slice(chunk);
            }
        }
    }

    let greyscale = ColorImage::from_gray([256, 256], &tile_map_bitmap);
    tile_map_texture.set(greyscale, TextureOptions::NEAREST);

    egui::Image::from_texture(&*tile_map_texture).paint_at(ui, rect);
}

fn highlight_background(ui: &mut Ui, to_screen: RectTransform, ppu: &Ppu) {
    // Draw a red rectangle around the visible portion of the background tile map.
    let sc_y = ppu.read_byte(SCY, ppu.clock);
    let sc_x = ppu.read_byte(SCX, ppu.clock);
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

fn highlight_window(ui: &mut Ui, to_screen: RectTransform, ppu: &Ppu) {
    // Draw a blue rectangle around the visible portion of the window tile map.
    let wy = ppu.read_byte(WY, ppu.clock);
    let wx = ppu.read_byte(WX, ppu.clock).wrapping_sub(7);
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

pub fn draw_tile_map_0(ui: &mut Ui, tile_map_0_texture: &mut TextureHandle, ppu: &Ppu) {
    ui.vertical(|ui| {
        ui.heading("Tile Map 0: 0x9800-0x9C00");

        let vram = ppu.vram.as_array().unwrap();
        let tile_map = tiles::tile_map_0(vram);

        Frame::canvas(ui.style()).show(ui, |ui| {
            let (_, rect) = ui.allocate_space(Vec2::new(
                f32::from(TILE_MAP_SIZE) * TILE_MAP_SCALE,
                f32::from(TILE_MAP_SIZE) * TILE_MAP_SCALE,
            ));
            let to_screen = RectTransform::from_to(
                Rect::from_x_y_ranges(
                    0.0..=f32::from(TILE_MAP_SIZE - 1),
                    0.0..=f32::from(TILE_MAP_SIZE - 1),
                ),
                rect,
            );

            draw_tile_map(ui, rect, tile_map_0_texture, tile_map, ppu);

            let lcdc = LcdControl::from_bits(ppu.read_byte(LCDC, ppu.clock));
            if lcdc.bg_and_window_enabled() {
                if !lcdc.bg_tile_map() {
                    highlight_background(ui, to_screen, ppu);
                }
                if !lcdc.window_tile_map() && lcdc.window_enabled() {
                    highlight_window(ui, to_screen, ppu);
                }
            }
        });
    });
}

pub fn draw_tile_map_1(ui: &mut Ui, tile_map_1_texture: &mut TextureHandle, ppu: &Ppu) {
    ui.vertical(|ui| {
        ui.heading("Tile Map 1: 0x9C00-0xA000");

        let vram = ppu.vram.as_array().unwrap();
        let tile_map = tiles::tile_map_1(vram);

        Frame::canvas(ui.style()).show(ui, |ui| {
            let (_, rect) = ui.allocate_space(Vec2::new(
                f32::from(TILE_MAP_SIZE) * TILE_MAP_SCALE,
                f32::from(TILE_MAP_SIZE) * TILE_MAP_SCALE,
            ));
            let to_screen = RectTransform::from_to(
                Rect::from_x_y_ranges(
                    0.0..=f32::from(TILE_MAP_SIZE - 1),
                    0.0..=f32::from(TILE_MAP_SIZE - 1),
                ),
                rect,
            );

            draw_tile_map(ui, rect, tile_map_1_texture, tile_map, ppu);

            let lcdc = LcdControl::from_bits(ppu.read_byte(LCDC, ppu.clock));
            if lcdc.bg_and_window_enabled() {
                if lcdc.bg_tile_map() {
                    highlight_background(ui, to_screen, ppu);
                }
                if lcdc.window_tile_map() && lcdc.window_enabled() {
                    highlight_window(ui, to_screen, ppu);
                }
            }
        });
    });
}
