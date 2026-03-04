use egui::Checkbox;
use egui_extras::{Column, TableBody, TableBuilder};
use hw_constants::io_regs::{IE, IF};
use sm83_interp::cpu::Cpu;
use sm83_interp::cpu::InterruptBits;

pub fn show(ui: &mut egui::Ui, dmg_state: &Cpu) {
    TableBuilder::new(ui)
        .striped(true)
        .column(Column::auto())
        .column(Column::auto())
        .column(Column::remainder())
        .header(18.0, |mut header| {
            header.col(|ui| {
                ui.label("Interrupt");
            });
            header.col(|ui| {
                ui.label("Enabled");
            });
            header.col(|ui| {
                ui.label("Flagged");
            });
        })
        .body(|body| {
            draw_interrupts_body(body, dmg_state);
        });
}

fn draw_interrupts_body(body: TableBody<'_>, dmg_state: &Cpu) {
    let intr_enable = InterruptBits::from_bits(dmg_state.memory.buffer[IE as usize]);
    let intr_flag = InterruptBits::from_bits(dmg_state.memory.buffer[IF as usize]);

    let values = [
        ("VBlank", intr_enable.vblank(), intr_flag.vblank()),
        ("LCD", intr_enable.lcd(), intr_flag.lcd()),
        ("Timer", intr_enable.timer(), intr_flag.timer()),
        ("Serial", intr_enable.serial(), intr_flag.serial()),
        ("Joypad", intr_enable.joypad(), intr_flag.joypad()),
    ];

    body.rows(18.0, values.len(), |mut row| {
        let row_index = row.index();
        let (interrupt, enabled, flagged) = values[row_index];

        row.col(|ui| {
            ui.label(interrupt);
        });

        // Check boxes are only used to show the bool values, so interaction is disabled.
        row.col(|ui| {
            let mut enabled: bool = enabled;
            ui.add_enabled(false, Checkbox::new(&mut enabled, ""));
        });
        row.col(|ui| {
            let mut flagged: bool = flagged;
            ui.add_enabled(false, Checkbox::new(&mut flagged, ""));
        });
    });
}
