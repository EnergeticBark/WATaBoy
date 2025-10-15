use egui::Ui;
use egui_extras::{Column, TableBody, TableBuilder};
use sm83_interp::cpu::Cpu;

pub fn draw_register_table(ui: &mut Ui, dmg_state: &Cpu) {
    TableBuilder::new(ui)
        .striped(true)
        .column(Column::auto())
        .column(Column::remainder())
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

    let formatted: Vec<_> = reg_names_and_values.into_iter().map(|(name, value)| {
        (name, format!("{value:#06X}"))
    }).collect();

    body.rows(18.0, reg_names_and_values.len(), |mut row| {
        let row_index = row.index();
        let (name, value) = &formatted[row_index];

        row.col(|ui| {
            ui.label(*name);
        });
        row.col(|ui| {
            ui.label(value);
        });
    });
}
