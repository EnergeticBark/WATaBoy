use std::collections::HashMap;

use egui_extras::{Column, TableBody, TableBuilder};
use hw_constants::{
    IE,
    io_regs::{DIV, IF, TAC, TIMA, TMA},
};
use sm83_interp::cpu::Cpu;

const ROW_HEIGHT: f32 = 18.0;

pub fn show(ui: &mut egui::Ui, dmg_state: &Cpu) {
    let name = "Waking Reads";
    ui.heading(name);
    TableBuilder::new(ui)
        .id_salt(name)
        .striped(true)
        .column(Column::auto())
        .column(Column::remainder())
        .header(ROW_HEIGHT, |mut header| {
            for column_name in ["Address (Range)", "Count"] {
                header.col(|ui| _ = ui.label(column_name));
            }
        })
        .body(|body| {
            draw_woke_timers_body(body, &dmg_state.memory.woke_timers_reads.0);
        });

    let name = "Waking Writes";
    ui.heading(name);
    TableBuilder::new(ui)
        .id_salt(name)
        .striped(true)
        .column(Column::auto())
        .column(Column::remainder())
        .header(ROW_HEIGHT, |mut header| {
            for column_name in ["Address (Range)", "Count"] {
                header.col(|ui| _ = ui.label(column_name));
            }
        })
        .body(|body| {
            draw_woke_timers_body(body, &dmg_state.memory.woke_timers_writes.0);
        });
}

fn draw_woke_timers_body(body: TableBody<'_>, ppu_accesses: &HashMap<u16, u64>) {
    let kv_pairs = ppu_accesses.iter().collect::<Vec<_>>();

    body.rows(ROW_HEIGHT, kv_pairs.len(), |mut row| {
        let row_index = row.index();
        let (&key, value) = kv_pairs[row_index];

        let key_name = match key {
            DIV => "DIV",
            TIMA => "TIMA",
            TMA => "TMA",
            TAC => "TAC",
            IF => "IF",
            IE => "IE",
            _ => unreachable!(),
        };

        row.col(|ui| {
            ui.label(key_name);
        });
        row.col(|ui| {
            ui.label(value.to_string());
        });
    });
}
