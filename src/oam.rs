use eframe::epaint::TextureHandle;
use egui::Ui;
use egui_extras::{Column, TableBody, TableBuilder};
use ppu::oam;
use sm83_interp::cpu::Cpu;

pub fn draw_oam_table(ui: &mut Ui, tiles: &mut [TextureHandle], dmg_state: &Cpu) {
    TableBuilder::new(ui)
        .id_salt("OAM View")
        .striped(true)
        .columns(Column::auto(), 6)
        .header(18.0, |mut header| {
            header.col(|ui| {
                ui.label("Object #");
            });
            header.col(|ui| {
                ui.label("Y Pos");
            });
            header.col(|ui| {
                ui.label("X Pos");
            });
            header.col(|ui| {
                ui.label("Tile Index");
            });
            header.col(|ui| {
                ui.label("Tile");
            });
            header.col(|ui| {
                ui.label("Attributes");
            });
        })
        .body(|body| {
            draw_oam_body(body, tiles, dmg_state);
        });
}

fn draw_oam_body(body: TableBody<'_>, tiles: &mut [TextureHandle], dmg_state: &Cpu) {
    body.rows(18.0, 40, |mut row| {
        let nth_row = row.index();

        row.col(|ui| {
            ui.label(format!("{nth_row}"));
        });

        let obj = oam::nth_obj(dmg_state.memory.buffer.as_slice(), nth_row);

        row.col(|ui| {
            ui.label(format!("{}", obj.y_pos));
        });

        row.col(|ui| {
            ui.label(format!("{}", obj.x_pos));
        });

        row.col(|ui| {
            ui.label(format!("{}: ", obj.tile_index));
        });

        row.col(|ui| {
            let tile = &tiles[obj.tile_index as usize];
            ui.add(egui::Image::from_texture(tile).fit_to_original_size(2.0));
        });

        row.col(|ui| {
            ui.label(format!("{}", obj.attributes));
        });
    });
}
