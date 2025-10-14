use std::fmt::format;
use egui::Ui;
use egui_extras::{Column, TableBody, TableBuilder};
use sm83_interp::cpu::Cpu;

pub struct PPUViewApp {
    // Example stuff:
    dmg_state: Cpu,
}

impl Default for PPUViewApp {
    fn default() -> Self {
        Self {
            dmg_state: Cpu::default(),
        }
    }
}

impl PPUViewApp {
    /// Called once before the first frame.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // This is also where you can customize the look and feel of egui using
        // `cc.egui_ctx.set_visuals` and `cc.egui_ctx.set_fonts`.

        Default::default()
    }
}

impl eframe::App for PPUViewApp {
    /// Called each time the UI needs repainting, which may be many times per second.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            egui::MenuBar::new().ui(ui, |ui| {

                ui.menu_button("File", |ui| {
                    if ui.button("Load Bootrom (dmg0.bin)").clicked() {
                        self.dmg_state.load_boot_rom();
                    }
                });
            });
        });

        egui::SidePanel::right("Registers")
            .min_width(300.0)
            .show(ctx, |ui| {
                TableBuilder::new(ui)
                    .striped(true)
                    .column(Column::auto())
                    .column(Column::remainder())
                    .header(12.0, |mut header| {
                        header.col(|ui| {
                            ui.label("Register");
                        });
                        header.col(|ui| {
                            ui.label("Value");
                        });
                    })
                    .body(|body| {
                        draw_registers(body, &self.dmg_state);
                    });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Nothing");
            if ui.button("Step").clicked() {
                self.dmg_state.execute();
            }
        });
    }
}

fn draw_registers(body: TableBody<'_>, dmg_state: &Cpu) {
    let reg_names_and_values = [
        ("AF", dmg_state.registers.af.into_bits()),
        ("BC", dmg_state.registers.bc.into_bits()),
        ("DE", dmg_state.registers.de.into_bits()),
        ("HL", dmg_state.registers.hl.into_bits()),
        ("SP", dmg_state.registers.sp),
        ("PC", dmg_state.registers.pc),
    ];

    let formatted: Vec<_> = reg_names_and_values.iter().map(|(name, value)| {
        (*name, format!("{:#06X}", value))
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