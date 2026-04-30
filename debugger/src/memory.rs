use eframe::epaint::Color32;
use egui::{RichText, TextStyle, Ui};
use interpreter::cpu::Cpu;

pub fn draw_memory_table(ui: &mut Ui, dmg_state: &Cpu) {
    ui.style_mut().override_text_style = Some(TextStyle::Monospace);
    ui.horizontal(|ui| {
        ui.label("    ");

        for column_number in 0..16 {
            let column_label = format!("{column_number:02X}");
            ui.label(column_label);
        }
    });

    draw_memory_body(ui, dmg_state);
}

fn draw_memory_body(ui: &mut Ui, dmg_state: &Cpu) {
    egui::ScrollArea::vertical().show_rows(ui, 18.0, 0x10000 / 16, |ui, row_range| {
        for row in row_range {
            ui.horizontal(|ui| {
                let row_index = u16::try_from(row).unwrap() * 16;
                let row_label = format!("{:03X}0", row_index / 16);
                ui.label(row_label);

                for i in 0..16 {
                    let formatted_row =
                        format!("{:02X}", dmg_state.memory.buffer[(row_index + i) as usize]);

                    if row_index + i == dmg_state.registers.pc {
                        ui.strong(RichText::from(formatted_row).color(Color32::RED));
                    } else if row_index + i == dmg_state.registers.sp {
                        let sp_color = if ui.visuals().dark_mode {
                            Color32::LIGHT_BLUE
                        } else {
                            Color32::BLUE
                        };
                        ui.strong(RichText::from(formatted_row).color(sp_color));
                    } else {
                        ui.strong(formatted_row);
                    }
                }
            });
        }
    });
}
