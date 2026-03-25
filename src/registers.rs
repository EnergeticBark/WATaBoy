use egui::{Checkbox, Color32, RichText, Ui};
use egui_extras::{Column, TableBody, TableBuilder};
use sm83_interp::cpu::Cpu;

const ROW_HEIGHT: f32 = 18.0;

pub fn draw_register_table(ui: &mut Ui, dmg_state: &Cpu) {
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            let name = "CPU Registers";
            ui.heading(name);
            TableBuilder::new(ui)
                .id_salt(name)
                .striped(true)
                .column(Column::auto())
                .column(Column::auto())
                .header(18.0, |mut header| {
                    header.col(|ui| {
                        ui.label("Register");
                    });
                    header.col(|ui| {
                        ui.label("Value");
                    });
                })
                .body(|body| {
                    draw_registers_body(body, dmg_state);
                });
        });

        ui.separator();

        ui.vertical(|ui| {
            let name = "Misc. State";
            ui.heading(name);
            TableBuilder::new(ui)
                .id_salt(name)
                .striped(true)
                .column(Column::auto())
                .column(Column::remainder())
                .header(ROW_HEIGHT, |mut header| {
                    header.col(|ui| {
                        ui.label("State");
                    });
                    header.col(|ui| {
                        ui.label("Value");
                    });
                })
                .body(|mut body| {
                    draw_flags_body(&mut body, dmg_state);
                });
        });
    });
}

fn draw_registers_body(body: TableBody<'_>, dmg_state: &Cpu) {
    let reg_names_and_values = [
        ("AF", dmg_state.registers.af.into_bits()),
        ("BC", dmg_state.registers.bc.into_bits()),
        ("DE", dmg_state.registers.de.into_bits()),
        ("HL", dmg_state.registers.hl.into_bits()),
        ("SP", dmg_state.registers.sp),
        ("PC", dmg_state.registers.pc),
    ];

    let formatted: Vec<_> = reg_names_and_values
        .into_iter()
        .map(|(name, value)| (name, format!("{value:#06X}")))
        .collect();

    body.rows(ROW_HEIGHT, reg_names_and_values.len(), |mut row| {
        let row_index = row.index();
        let (name, value) = &formatted[row_index];

        row.col(|ui| {
            if *name == "PC" {
                ui.label(RichText::from(*name).color(Color32::RED));
            } else if *name == "SP" {
                let sp_color = if ui.visuals().dark_mode {
                    Color32::LIGHT_BLUE
                } else {
                    Color32::BLUE
                };
                ui.label(RichText::from(*name).color(sp_color));
            } else {
                ui.label(*name);
            }
        });
        row.col(|ui| {
            ui.strong(RichText::from(value).monospace());
        });
    });
}

fn draw_flags_body(body: &mut TableBody<'_>, dmg_state: &Cpu) {
    body.row(ROW_HEIGHT, |mut row| {
        row.col(|ui| _ = ui.label("HALTED"));
        let mut checked = dmg_state.halted;
        row.col(|ui| _ = ui.add_enabled(false, Checkbox::new(&mut checked, "")));
    });
    body.row(ROW_HEIGHT, |mut row| {
        row.col(|ui| _ = ui.label("IME"));
        let mut checked = dmg_state.ime;
        row.col(|ui| _ = ui.add_enabled(false, Checkbox::new(&mut checked, "")));
    });
    body.row(ROW_HEIGHT, |mut row| {
        row.col(|ui| _ = ui.label("CPU Clock"));
        let formatted = format!("{}", dmg_state.memory.clock);
        row.col(|ui| _ = ui.label(formatted));
    });
    body.row(ROW_HEIGHT, |mut row| {
        row.col(|ui| _ = ui.label("PPU Clock"));
        let formatted = format!("{}", dmg_state.memory.ppu.clock);
        row.col(|ui| _ = ui.label(formatted));
    });
    body.row(ROW_HEIGHT, |mut row| {
        row.col(|ui| _ = ui.label("Next VBlank"));
        let formatted = format!("{}", dmg_state.memory.ppu.next_vblank_interrupt);
        row.col(|ui| _ = ui.label(formatted));
    });
    body.row(ROW_HEIGHT, |mut row| {
        row.col(|ui| _ = ui.label("Next LCD"));
        let formatted = format!("{}", dmg_state.memory.ppu.next_lcd_interrupt);
        row.col(|ui| _ = ui.label(formatted));
    });
    body.row(ROW_HEIGHT, |mut row| {
        row.col(|ui| _ = ui.label("Timers Clock"));
        let formatted = format!("{}", dmg_state.memory.timers.clock);
        row.col(|ui| _ = ui.label(formatted));
    });
    body.row(ROW_HEIGHT, |mut row| {
        row.col(|ui| _ = ui.label("Next Timer"));
        let formatted = format!("{}", dmg_state.memory.timers.next_interrupt);
        row.col(|ui| _ = ui.label(formatted));
    });
    body.row(ROW_HEIGHT, |mut row| {
        row.col(|ui| _ = ui.label("Next Inter"));
        let formatted = format!("{}", dmg_state.memory.next_interrupt);
        row.col(|ui| _ = ui.label(formatted));
    });
}
