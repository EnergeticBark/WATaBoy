use egui::{Checkbox, Label, TextWrapMode};
use egui_extras::{Column, TableBody, TableBuilder};
use hw_constants::io_regs::STAT;
use hw_constants::io_regs::{IE, IF};
use interpreter::addressable::Addressable;
use interpreter::cpu::Cpu;
use interpreter::cpu::InterruptBits;
use interpreter::ppu::{LcdStatus, StatMode};

const ROW_HEIGHT: f32 = 18.0;

pub fn show(ui: &mut egui::Ui, dmg_state: &Cpu) {
    let name = "IE and IF";
    ui.heading(name);
    TableBuilder::new(ui)
        .id_salt(name)
        .striped(true)
        .column(Column::auto())
        .column(Column::auto())
        .column(Column::auto())
        .column(Column::remainder())
        .header(ROW_HEIGHT, |mut header| {
            for column_name in ["Interrupt", "Enabled", "Flagged", "Scheduled"] {
                header.col(|ui| _ = ui.label(column_name));
            }
        })
        .body(|body| {
            draw_ie_and_if_body(body, dmg_state);
        });

    ui.separator();

    let name = "STAT";
    ui.heading(name);
    TableBuilder::new(ui)
        .id_salt(name)
        .striped(true)
        .column(Column::auto())
        .column(Column::remainder())
        .header(ROW_HEIGHT, |mut header| {
            for column_name in ["Status", "Value"] {
                header.col(|ui| _ = ui.label(column_name));
            }
        })
        .body(|mut body| {
            draw_stat_body(&mut body, dmg_state);
        });

    ui.separator();

    let name = "TAC";
    ui.heading(name);
    TableBuilder::new(ui)
        .id_salt(name)
        .striped(true)
        .column(Column::auto())
        .column(Column::remainder())
        .header(ROW_HEIGHT, |mut header| {
            for column_name in ["Status", "Value"] {
                header.col(|ui| _ = ui.label(column_name));
            }
        })
        .body(|mut body| {
            draw_tac_body(&mut body, dmg_state);
        });
}

fn cycles_until_interrupt(next_interrupt: u64, current_clock: u64) -> Option<u64> {
    // We use u64::MAX to represent interrupts that will never happen, i.e., VBlank interrupt is disabled in IE.
    if next_interrupt == u64::MAX {
        None
    } else {
        Some(next_interrupt.saturating_sub(current_clock))
    }
}

fn draw_ie_and_if_body(body: TableBody<'_>, dmg_state: &Cpu) {
    let intr_enable = InterruptBits::from_bits(dmg_state.memory.buffer[IE as usize]);
    let intr_flag = InterruptBits::from_bits(dmg_state.memory.buffer[IF as usize]);

    let clock = dmg_state.memory.clock;
    let next_vblank = cycles_until_interrupt(dmg_state.memory.ppu.next_vblank_interrupt, clock);
    let next_lcd = cycles_until_interrupt(dmg_state.memory.ppu.next_lcd_interrupt, clock);
    let next_timers = cycles_until_interrupt(dmg_state.memory.timers.next_interrupt, clock);

    let values = [
        (
            "VBlank",
            intr_enable.vblank(),
            intr_flag.vblank(),
            next_vblank,
        ),
        ("LCD", intr_enable.lcd(), intr_flag.lcd(), next_lcd),
        ("Timer", intr_enable.timer(), intr_flag.timer(), next_timers),
        ("Serial", intr_enable.serial(), intr_flag.serial(), None),
        ("Joypad", intr_enable.joypad(), intr_flag.joypad(), None),
    ];

    body.rows(ROW_HEIGHT, values.len(), |mut row| {
        let row_index = row.index();
        let (interrupt, enabled, flagged, scheduled) = values[row_index];

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

        row.col(|ui| {
            if let Some(cycles) = scheduled {
                ui.monospace(cycles.to_string());
            } else {
                ui.label("N/A");
            }
        });
    });
}

fn draw_checkbox_row(body: &mut TableBody<'_>, status: &str, checked: bool) {
    body.row(ROW_HEIGHT, |mut row| {
        row.col(|ui| {
            // Prevent this label from wrapping so the column it's in gets extended.
            ui.add(Label::new(status).wrap_mode(TextWrapMode::Extend));
        });
        row.col(|ui| {
            let mut checked = checked;
            ui.add_enabled(false, Checkbox::new(&mut checked, ""));
        });
    });
}

fn draw_stat_body(body: &mut TableBody<'_>, dmg_state: &Cpu) {
    let stat = LcdStatus::from_bits(dmg_state.memory.ppu.read_byte(STAT, 0));

    body.row(ROW_HEIGHT, |mut row| {
        row.col(|ui| {
            ui.label("PPU Mode");
        });
        let stat_mode_name = match stat.mode() {
            StatMode::HBlank => "0 (HBlank)",
            StatMode::VBlank => "1 (VBlank)",
            StatMode::OamScan => "2 (OAM Scan)",
            StatMode::Drawing => "3 (Drawing)",
        };
        row.col(|ui| {
            ui.label(stat_mode_name);
        });
    });

    draw_checkbox_row(body, "LYC == LY", stat.coincidence());
    draw_checkbox_row(body, "Mode 0 Int Select", stat.mode0_int_select());
    draw_checkbox_row(body, "Mode 1 Int Select", stat.mode1_int_select());
    draw_checkbox_row(body, "Mode 2 Int Select", stat.mode2_int_select());
    draw_checkbox_row(body, "LYC Int Select", stat.lyc_int_select());
}

fn draw_tac_body(body: &mut TableBody<'_>, dmg_state: &Cpu) {
    let tac = dmg_state.memory.timers.tac;

    body.row(ROW_HEIGHT, |mut row| {
        row.col(|ui| {
            ui.label("Clock select");
        });
        let clock_select_name = format!("{}", tac.clock_select().into_bits());
        row.col(|ui| {
            ui.label(clock_select_name);
        });
    });

    body.row(ROW_HEIGHT, |mut row| {
        row.col(|ui| {
            ui.label("TIMA enabled");
        });
        row.col(|ui| {
            let mut checked = tac.tima_enabled();
            ui.add_enabled(false, Checkbox::new(&mut checked, ""));
        });
    });
}
