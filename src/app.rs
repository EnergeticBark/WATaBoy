use crate::memory::draw_memory_table;
use crate::registers::draw_register_table;
use crate::tile_map::draw_tile_map;
use crate::tiles::draw_tile_table;
use egui::{Slider, TextureHandle};
use log::error;
use sm83_interp::cpu::Cpu;
use sm83_interp::opcodes::decode;
use crate::dnd_rom::handle_dropped_rom;

const NINTENDO_LOGO: &[u8; 48] = include_bytes!("../nintendo_logo.bin");

pub struct PPUViewApp {
    dmg_state: Cpu,
    tiles: Vec<Option<TextureHandle>>,
    tile_map: Option<TextureHandle>,
    step_by_cycles: u32,
    step_by_frames: u32
}

impl Default for PPUViewApp {
    fn default() -> Self {
        Self {
            dmg_state: {
                let mut cpu = Cpu::default();
                cpu.memory[0x0104..0x0134].copy_from_slice(NINTENDO_LOGO);
                /* Our rom header is all zeros, so just hardcode the checksum of those zeros to make
                   the bootrom happy.
                   See: https://gbdev.io/pandocs/The_Cartridge_Header.html#014d--header-checksum
                */
                cpu.memory[0x014D] = 0xE7;
                cpu
            },
            tiles: vec![None; 384],
            tile_map: None,
            step_by_cycles: 10000,
            step_by_frames: 1,
        }
    }
}

impl PPUViewApp {
    /// Called once before the first frame.
    #[must_use]
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // This is also where you can customize the look and feel of egui using
        // `cc.egui_ctx.set_visuals` and `cc.egui_ctx.set_fonts`.

        PPUViewApp::default()
    }
}

fn step_multiple(steps: u32, dmg_state: &mut Cpu) {
    for _ in 0..steps {
        /* Cycle the LCD Y coordinate so the bootrom doesn't get stuck waiting for a v-blank.
          Once I actually implement the PPU alongside the CPU, I'll want to do this with proper
          timing. See: https://gbdev.io/pandocs/Rendering.html
        */
        dmg_state.memory[0xFF44] = (dmg_state.memory[0xFF44] + 1) % 154;
        // Set joypad value such that no buttons are held.
        dmg_state.memory[0xFF00] = 0x0F;
        if let Err(message) =dmg_state.execute() {
            error!("{message}");
        }
        dmg_state.handle_interrupts();
    }
}

impl eframe::App for PPUViewApp {
    /// Called each time the UI needs repainting, which may be many times per second.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            egui::MenuBar::new().ui(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Load Bootrom (dmg.bin)").clicked() {
                        self.dmg_state.load_boot_rom();
                    }

                    if ui.button("Reset").clicked() {
                        self.dmg_state = Cpu::default();
                    }
                });
            });
        });

        egui::SidePanel::right("Registers")
            .min_width(300.0)
            .show(ctx, |ui| {
                draw_register_table(ui, &self.dmg_state);

                ui.separator();

                draw_memory_table(ui, &self.dmg_state);
            });

        egui::SidePanel::left("Tiles")
            .min_width(300.0)
            .show(ctx, |ui| {
                draw_tile_table(ui, ctx, &mut self.tiles, &self.dmg_state);
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            draw_tile_map(ui, ctx, &mut self.tile_map, &self.dmg_state);

            if ui.button("Step once").clicked() {
                println!("{:?}", decode(self.dmg_state.memory[self.dmg_state.registers.pc]));
                if let Err(message) = self.dmg_state.execute() {
                    error!("{message}");
                }
            }

            ui.horizontal(|ui| {
                if ui.button("Step multiple").clicked() {
                    step_multiple(self.step_by_cycles, &mut self.dmg_state);
                }

                ui.add(Slider::new(&mut self.step_by_cycles, 0..=100_000));
            });

            ui.horizontal(|ui| {
                if ui.button("Request VBlank and step multiple").clicked() {
                    for _ in 0..self.step_by_frames {
                        self.dmg_state.memory[0xFF0F] |= 0b0000_0001;
                        step_multiple(self.step_by_cycles, &mut self.dmg_state);
                    }
                }

                ui.add(Slider::new(&mut self.step_by_frames, 0..=100));
            });
            ui.horizontal(|ui| {
                if ui.button("Timer and step multiple").clicked() {
                    for _ in 0..self.step_by_frames {
                        self.dmg_state.memory[0xFF0F] |= 0b0000_0100;
                        step_multiple(self.step_by_cycles, &mut self.dmg_state);
                    }
                }

                ui.add(Slider::new(&mut self.step_by_frames, 0..=100));
            });
        });

        ctx.input(|i| {
            if !i.raw.dropped_files.is_empty() {
                let dropped_file = i.raw.dropped_files.first().unwrap().clone();
                handle_dropped_rom(dropped_file, &mut self.dmg_state);
            }
        });
    }
}
