use std::collections::HashMap;

use egui_extras::{Column, TableBody, TableBuilder};
use hw_constants::{
    IE,
    io_regs::{DIV, IF, TAC, TIMA, TMA},
};
use interpreter::cpu::Cpu;

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
            for column_name in ["Address", "Count"] {
                header.col(|ui| _ = ui.label(column_name));
            }
        })
        .body(|body| {
            draw_waking_timers_body(body, &dmg_state.memory.waking_reads.0);
        });

    let name = "Waking Writes";
    ui.heading(name);
    TableBuilder::new(ui)
        .id_salt(name)
        .striped(true)
        .column(Column::auto())
        .column(Column::remainder())
        .header(ROW_HEIGHT, |mut header| {
            for column_name in ["Address", "Count"] {
                header.col(|ui| _ = ui.label(column_name));
            }
        })
        .body(|body| {
            draw_waking_timers_body(body, &dmg_state.memory.waking_writes.0);
        });
}

fn draw_waking_timers_body(body: TableBody<'_>, waking_accesses: &HashMap<u16, u64>) {
    let filter_timer_addresses = |(&key, &val)| {
        let key_name = match key {
            DIV => "DIV",
            TIMA => "TIMA",
            TMA => "TMA",
            TAC => "TAC",
            IF => "IF",
            IE => "IE",
            _ => return None,
        };
        Some((key_name, val))
    };

    let kv_pairs = waking_accesses
        .iter()
        .filter_map(filter_timer_addresses)
        .collect::<Vec<(&'static str, u64)>>();

    body.rows(ROW_HEIGHT, kv_pairs.len(), |mut row| {
        let row_index = row.index();
        let (key, value) = kv_pairs[row_index];

        row.col(|ui| {
            ui.label(key);
        });
        row.col(|ui| {
            ui.label(value.to_string());
        });
    });
}
