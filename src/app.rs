use crate::dnd_rom::handle_dropped_rom;
use crate::memory::draw_memory_table;
use crate::oam::draw_oam_table;
use crate::registers::draw_register_table;
use crate::tile_map::{draw_tile_map_0, draw_tile_map_1};
use crate::tiles::draw_tile_table;
use eframe::epaint::textures::TextureOptions;
use eframe::epaint::{Color32, ColorImage};
use egui::{Key, Slider, TextureHandle};
use hw_constants::io_regs;
use log::error;
use ppu::ppu::Ppu;
use rkyv::deserialize;
use rkyv::rancor::Error;
use sm83_interp::common::post_boot::PostBoot;
use sm83_interp::cpu::{ArchivedCpu, Cpu};
use sm83_interp::joypad::ButtonsHeld;
use sm83_interp::opcodes::decode;
use std::fs::File;
use std::io::{Read, Write};

const NINTENDO_LOGO: &[u8; 48] = include_bytes!("../nintendo_logo.bin");

pub struct PPUViewApp {
    dmg_state: Cpu,
    ppu_state: Ppu,
    tiles: Vec<TextureHandle>,
    tile_map_0: TextureHandle,
    tile_map_1: TextureHandle,
    screen: TextureHandle,
    step_by_cycles: u32,
    step_by_frames: u32,
    play: bool,
    buttons_held: ButtonsHeld,
}

impl PPUViewApp {
    /// Called once before the first frame.
    #[must_use]
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // This is also where you can customize the look and feel of egui using
        // `cc.egui_ctx.set_visuals` and `cc.egui_ctx.set_fonts`.

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
            tiles: (0..384)
                .map(|tile_index| {
                    cc.egui_ctx.load_texture(
                        format!("Tile {tile_index}"),
                        ColorImage::filled(
                            [hw_constants::TILE_SIZE, hw_constants::TILE_SIZE],
                            Color32::BLACK,
                        ),
                        TextureOptions::NEAREST,
                    )
                })
                .collect(),
            tile_map_0: cc.egui_ctx.load_texture(
                "Tile Map 0",
                ColorImage::filled(
                    [hw_constants::TILE_MAP_SIZE, hw_constants::TILE_MAP_SIZE],
                    Color32::BLACK,
                ),
                TextureOptions::NEAREST,
            ),
            tile_map_1: cc.egui_ctx.load_texture(
                "Tile Map 1",
                ColorImage::filled(
                    [hw_constants::TILE_MAP_SIZE, hw_constants::TILE_MAP_SIZE],
                    Color32::BLACK,
                ),
                TextureOptions::NEAREST,
            ),
            screen: cc.egui_ctx.load_texture(
                "Screen",
                ColorImage::filled(
                    [hw_constants::SCREEN_WIDTH, hw_constants::SCREEN_HEIGHT],
                    Color32::BLACK,
                ),
                TextureOptions::NEAREST,
            ),
            step_by_cycles: 10000,
            step_by_frames: 1,
            play: false,
            buttons_held: ButtonsHeld::default(),
        }
    }
}

fn step_multiple(steps: u32, dmg_state: &mut Cpu, ppu_state: &mut Ppu, buttons_held: ButtonsHeld) {
    for _ in 0..steps {
        dmg_state.memory.update_joypad(buttons_held);
        match dmg_state.execute() {
            Err(message) => error!("{message}"),
            Ok(m_cycles) => {
                for _ in 0..m_cycles * 4 {
                    ppu_state.tick(&mut dmg_state.memory.buffer);
                }
            }
        }
        dmg_state.handle_interrupts();
    }
}

fn step_vblank(dmg_state: &mut Cpu, ppu_state: &mut Ppu, buttons_held: ButtonsHeld) {
    loop {
        dmg_state.memory.update_joypad(buttons_held);
        match dmg_state.execute() {
            Err(message) => error!("{message}"),
            Ok(m_cycles) => {
                for _ in 0..m_cycles * 4 {
                    ppu_state.tick(&mut dmg_state.memory.buffer);
                }
            }
        }
        let vblank_happened = (dmg_state.memory.buffer[io_regs::IF as usize] & 0b0000_0001
            == 0b0000_0001)
            && (dmg_state.memory.buffer[hw_constants::IE as usize] & 0b0000_0001 == 0b0000_0001)
            && dmg_state.ime;
        dmg_state.handle_interrupts();
        if vblank_happened {
            return;
        }
    }
}

impl eframe::App for PPUViewApp {
    /// Called each time the UI needs repainting, which may be many times per second.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::Window::new("Log").show(ctx, |ui| {
            // Draws the logger UI.
            egui_logger::logger_ui().show(ui);
        });

        egui::Window::new("PPU Output").show(ctx, |ui| {
            self.screen.set(
                ColorImage::from_gray(
                    [hw_constants::SCREEN_WIDTH, hw_constants::SCREEN_HEIGHT],
                    &self.ppu_state.funny_buffer_test,
                ),
                TextureOptions::NEAREST,
            );

            ui.add(egui::Image::from_texture(&self.screen).fit_to_original_size(2.0));
        });

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            egui::MenuBar::new().ui(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Load Bootrom (dmg.bin)").clicked() {
                        self.dmg_state.load_boot_rom();
                    }

                    if ui.button("Load State").clicked() {
                        let mut file = File::open("./savestate.bin").unwrap();
                        let mut bytes = Vec::new();
                        file.read_to_end(&mut bytes).unwrap();
                        let archived = rkyv::access::<ArchivedCpu, Error>(&bytes).unwrap();
                        self.dmg_state = deserialize::<Cpu, Error>(archived).unwrap();
                    }

                    if ui.button("Save State").clicked() {
                        let bytes = rkyv::to_bytes::<Error>(&self.dmg_state).unwrap();
                        let mut file = File::create("./savestate.bin").unwrap();
                        file.write_all(&bytes).unwrap();
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
                draw_tile_table(ui, &mut self.tiles, &self.dmg_state);
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                draw_tile_map_0(ui, &mut self.tile_map_0, &self.dmg_state);
                draw_tile_map_1(ui, &mut self.tile_map_1, &self.dmg_state);
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
                        self.buttons_held,
                    );
                }

                ui.add(Slider::new(&mut self.step_by_cycles, 0..=1_000_000));
            });

            ui.horizontal(|ui| {
                if ui.button("Step vblank").clicked() {
                    for _ in 0..self.step_by_frames {
                        step_vblank(&mut self.dmg_state, &mut self.ppu_state, self.buttons_held);
                    }
                }

                ui.add(Slider::new(&mut self.step_by_frames, 0..=100));
            });

            if ui.button("Play/pause").clicked() {
                self.play = !self.play;
            }
            if self.play {
                step_vblank(&mut self.dmg_state, &mut self.ppu_state, self.buttons_held);
            }

            draw_oam_table(ui, &mut self.tiles, &self.dmg_state);
        });

        ctx.input(|i| {
            if !i.raw.dropped_files.is_empty() {
                let dropped_file = i.raw.dropped_files.first().unwrap().clone();
                self.dmg_state = Cpu::post_boot_dmg();
                handle_dropped_rom(dropped_file, &mut self.dmg_state);
            }

            self.buttons_held = ButtonsHeld {
                start: i.key_down(Key::Enter),
                select: i.key_down(Key::Backspace),
                b: i.key_down(Key::X),
                a: i.key_down(Key::Z),
                down: i.key_down(Key::ArrowDown),
                up: i.key_down(Key::ArrowUp),
                left: i.key_down(Key::ArrowLeft),
                right: i.key_down(Key::ArrowRight),
            }
        });

        ctx.request_repaint();
    }
}
