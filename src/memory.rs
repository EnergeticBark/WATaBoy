use eframe::epaint::Color32;
use egui::{RichText, Ui};
use egui_extras::{Column, TableBody, TableBuilder};
use sm83_interp::cpu::Cpu;

pub fn draw_memory_table(ui: &mut Ui, dmg_state: &Cpu) {
    TableBuilder::new(ui)
        .id_salt("Memory View")
        .striped(true)
        .columns(Column::auto(), 17)
        .header(18.0, |mut header| {
            header.col(|ui| {
                ui.label("");
            });

            for column_number in 0..16 {
                header.col(|ui| {
                    let column_label = format!("{column_number:02X}");
                    ui.monospace(column_label);
                });
            }
        })
        .body(|body| {
            draw_memory_body(body, dmg_state);
        });
}

fn draw_memory_body(body: TableBody<'_>, dmg_state: &Cpu) {
    body.rows(18.0, 0x10000 / 16, |mut row| {
        let row_index = row.index() as u16 * 16;
        let row_label = format!("{:03X}0", row_index / 16);
        row.col(|ui| {
            ui.monospace(row_label);
        });

        for i in 0..16 {
            let formatted_row = format!("{:02X}", dmg_state.memory[row_index + i]);

            row.col(|ui| {
                let label = RichText::from(formatted_row).strong();

                let label = if row_index + i == dmg_state.registers.pc {
                    label.color(Color32::RED)
                } else {
                    label
                };

                ui.monospace(label);
            });
        }
    });
}
