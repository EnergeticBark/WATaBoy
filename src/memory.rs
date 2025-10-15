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
                    let column_label = format!("{:02X}", column_number);
                    ui.monospace(RichText::from(column_label).strong());
                });
            }
        })
        .body(|body| {
            draw_memory_body(body, &dmg_state);
        });
}

fn draw_memory_body(body: TableBody<'_>, dmg_state: &Cpu) {
    body.rows(18.0, dmg_state.memory.len() / 16, |mut row| {
        let row_index = row.index() * 16;
        let row_label = format!("{:03X}0", row_index / 16);
        row.col(|ui| {
            ui.monospace(RichText::from(row_label).strong());
        });

        for i in 0..16 {
            let formatted_row = format!("{:02X}", dmg_state.memory[row_index + i]);

            row.col(|ui| {
                if row_index + i == dmg_state.registers.pc as usize {
                    ui.monospace(RichText::from(formatted_row).color(Color32::RED));
                } else {
                    ui.monospace(formatted_row);
                }
            });
        }
    });
}
