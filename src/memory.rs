use eframe::epaint::Color32;
use egui::{RichText, Ui};
use sm83_interp::cpu::Cpu;

pub fn draw_memory_table(ui: &mut Ui, dmg_state: &Cpu) {
    ui.horizontal(|ui| {
        ui.monospace("    ");

        for column_number in 0..16 {
            let column_label = format!("{column_number:02X}");
            ui.monospace(column_label);
        }
    });

    draw_memory_body(ui, dmg_state);
}

fn draw_memory_body(ui: &mut Ui, dmg_state: &Cpu) {
    egui::ScrollArea::vertical().show_rows(ui, 18.0, 0x10000 / 16, |ui, row_range| {
        for row in row_range {
            ui.horizontal(|ui| {
                let row_index = row as u16 * 16;
                let row_label = format!("{:03X}0", row_index / 16);
                ui.monospace(row_label);

                for i in 0..16 {
                    let formatted_row = format!("{:02X}", dmg_state.memory[row_index + i]);

                    let label = RichText::from(formatted_row).strong();

                    let label = if row_index + i == dmg_state.registers.pc {
                        label.color(Color32::RED)
                    } else {
                        label
                    };

                    ui.monospace(label);
                }
            });
        }
    });
}
