use eframe::epaint::TextureHandle;
use egui::{Checkbox, Ui};
use egui_extras::{Column, TableBody, TableBuilder};
use sm83_interp::cpu::Cpu;
use sm83_interp::ppu::oam;

pub fn draw_oam_table(ui: &mut Ui, tiles: &mut [TextureHandle], dmg_state: &Cpu) {
    TableBuilder::new(ui)
        .id_salt("OAM View")
        .striped(true)
        .columns(Column::auto(), 9)
        .header(18.0, |mut header| {
            for column_name in [
                "Object #",
                "Y Pos",
                "X Pos",
                "Tile Index",
                "Tile",
                "Priority",
                "Y Flip",
                "X Flip",
                "Palette",
            ] {
                header.col(|ui| _ = ui.label(column_name));
            }
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

        let obj = oam::nth_obj(dmg_state.memory.ppu.oam.as_array().unwrap(), nth_row);

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
            let mut checked: bool = obj.attributes.priority();
            ui.add_enabled(false, Checkbox::new(&mut checked, ""));
        });
        row.col(|ui| {
            let mut checked: bool = obj.attributes.y_flip();
            ui.add_enabled(false, Checkbox::new(&mut checked, ""));
        });
        row.col(|ui| {
            let mut checked: bool = obj.attributes.x_flip();
            ui.add_enabled(false, Checkbox::new(&mut checked, ""));
        });
        row.col(|ui| {
            let mut checked: bool = obj.attributes.palette();
            ui.add_enabled(false, Checkbox::new(&mut checked, ""));
        });
    });
}
