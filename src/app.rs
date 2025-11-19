use crate::dnd_rom::handle_dropped_rom;
use crate::memory::draw_memory_table;
use crate::oam::draw_oam_table;
use crate::registers::draw_register_table;
use crate::tile_map::{draw_tile_map_0, draw_tile_map_1};
use crate::tiles::draw_tile_table;

use eframe::epaint::textures::TextureOptions;
use eframe::epaint::{Color32, ColorImage};
use egui::{Slider, TextureHandle};
use log::error;
use ppu::ppu::Ppu;
use sm83_interp::common::post_boot::PostBoot;
use sm83_interp::cpu::Cpu;
use sm83_interp::opcodes::decode;

const NINTENDO_LOGO: &[u8; 48] = include_bytes!("../nintendo_logo.bin");

pub struct PPUViewApp {
    dmg_state: Cpu,
    ppu_state: Ppu,
    tiles: Vec<Option<TextureHandle>>,
    tile_map_0: Option<TextureHandle>,
    tile_map_1: Option<TextureHandle>,
    funny_buffer_texture: Option<TextureHandle>,
    step_by_cycles: u32,
    step_by_frames: u32,
}

impl Default for PPUViewApp {
    fn default() -> Self {
        Self {
            dmg_state: {
                let mut cpu = Cpu::default();
                cpu.memory.buffer[0x0104..0x0134].copy_from_slice(NINTENDO_LOGO);
                /* Our rom header is all zeros, so just hardcode the checksum of those zeros to make
                   the bootrom happy.
                   See: https://gbdev.io/pandocs/The_Cartridge_Header.html#014d--header-checksum
                */
                cpu.memory.buffer[0x014D] = 0xE7;
                cpu
            },
            ppu_state: Ppu::default(),
            tiles: vec![None; 384],
            tile_map_0: None,
            tile_map_1: None,
            funny_buffer_texture: None,
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

fn step_multiple(steps: u32, dmg_state: &mut Cpu, ppu_state: &mut Ppu) {
    for _ in 0..steps {
        // Set joypad value such that no buttons are held.
        dmg_state.memory.write_byte(0xFF00, 0x0F);
        if let Err(message) = dmg_state.execute() {
            error!("{message}");
        }
        for _ in 0..4 {
            ppu_state.tick(&dmg_state.memory.buffer);
            dmg_state.memory.buffer[0xFF44] = ppu_state.ly();
        }
        dmg_state.handle_interrupts();
    }
}

impl eframe::App for PPUViewApp {
    /// Called each time the UI needs repainting, which may be many times per second.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::Window::new("THE SCREEN Ahhh :)").show(ctx, |ui| {
            let funny_buffer_texture = self.funny_buffer_texture.get_or_insert_with(|| {
                ctx.load_texture(
                    "Screen",
                    ColorImage::filled([160, 144], Color32::BLACK),
                    TextureOptions::NEAREST,
                )
            });

            funny_buffer_texture.set(
                ColorImage::from_gray([160, 144], &self.ppu_state.funny_buffer_test),
                TextureOptions::NEAREST,
            );

            ui.add(egui::Image::from_texture(&*funny_buffer_texture).fit_to_original_size(2.0));
        });

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
            ui.horizontal(|ui| {
                draw_tile_map_0(ui, ctx, &mut self.tile_map_0, &self.dmg_state);
                draw_tile_map_1(ui, ctx, &mut self.tile_map_1, &self.dmg_state);
            });

            if ui.button("Step once").clicked() {
                println!(
                    "{:?}",
                    decode(self.dmg_state.memory[self.dmg_state.registers.pc])
                );
                if let Err(message) = self.dmg_state.execute() {
                    error!("{message}");
                }
            }

            ui.horizontal(|ui| {
                if ui.button("Step multiple").clicked() {
                    step_multiple(
                        self.step_by_cycles,
                        &mut self.dmg_state,
                        &mut self.ppu_state,
                    );
                }

                ui.add(Slider::new(&mut self.step_by_cycles, 0..=100_000));
            });

            ui.horizontal(|ui| {
                if ui.button("Request VBlank and step multiple").clicked() {
                    for _ in 0..self.step_by_frames {
                        self.dmg_state.memory.buffer[0xFF0F] |= 0b0000_0001;
                        step_multiple(
                            self.step_by_cycles,
                            &mut self.dmg_state,
                            &mut self.ppu_state,
                        );
                    }
                }

                ui.add(Slider::new(&mut self.step_by_frames, 0..=100));
            });

            draw_oam_table(ui, ctx, &mut self.tiles, &self.dmg_state);
        });

        ctx.input(|i| {
            if !i.raw.dropped_files.is_empty() {
                let dropped_file = i.raw.dropped_files.first().unwrap().clone();
                self.dmg_state = Cpu::post_boot_dmg();
                handle_dropped_rom(dropped_file, &mut self.dmg_state);
            }
        });
    }
}
