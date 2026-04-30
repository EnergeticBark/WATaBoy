use crate::dnd_rom::handle_dropped_rom;
use crate::memory::draw_memory_table;
use crate::oam::draw_oam_table;
use crate::registers::draw_register_table;
use crate::tile_map::{draw_tile_map_0, draw_tile_map_1};
use crate::tiles::draw_tile_table;
use crate::{interrupts, woke_ppu, woke_timers};
use eframe::epaint::textures::TextureOptions;
use eframe::epaint::{Color32, ColorImage};
use egui::{Key, Slider, TextureHandle, Ui};
use hw_constants::{SCREEN_HEIGHT, SCREEN_WIDTH, TILE_MAP_SIZE, TILE_SIZE};
use log::error;
use rkyv::deserialize;
use rkyv::rancor::Error;
use interpreter::cpu::opcodes::Opcode;
use interpreter::cpu::{ArchivedCpu, Cpu};
use interpreter::joypad::ButtonsHeld;
use std::fs::File;
use std::io::{Read, Write};

pub struct DebuggerApp {
    dmg_state: Cpu,
    tiles: Vec<TextureHandle>,
    tile_map_0: TextureHandle,
    tile_map_1: TextureHandle,
    screen: TextureHandle,
    step_by_cycles: u32,
    step_by_frames: u32,
    speed: f32,
    penalty: u64,
    play: bool,
    buttons_held: ButtonsHeld,
    logger_open: bool,
    interrupts_open: bool,
    woke_ppu_open: bool,
    woke_timers_open: bool,
}

impl DebuggerApp {
    /// Called once before the first frame.
    #[must_use]
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // This is also where you can customize the look and feel of egui using
        // `cc.egui_ctx.set_visuals` and `cc.egui_ctx.set_fonts`.

        Self {
            dmg_state: Cpu::default(),
            tiles: (0..384)
                .map(|tile_index| {
                    cc.egui_ctx.load_texture(
                        format!("Tile {tile_index}"),
                        ColorImage::filled(
                            [TILE_SIZE as usize, TILE_SIZE as usize],
                            Color32::BLACK,
                        ),
                        TextureOptions::NEAREST,
                    )
                })
                .collect(),
            tile_map_0: cc.egui_ctx.load_texture(
                "Tile Map 0",
                ColorImage::filled(
                    [TILE_MAP_SIZE as usize, TILE_MAP_SIZE as usize],
                    Color32::BLACK,
                ),
                TextureOptions::NEAREST,
            ),
            tile_map_1: cc.egui_ctx.load_texture(
                "Tile Map 1",
                ColorImage::filled(
                    [TILE_MAP_SIZE as usize, TILE_MAP_SIZE as usize],
                    Color32::BLACK,
                ),
                TextureOptions::NEAREST,
            ),
            screen: cc.egui_ctx.load_texture(
                "Screen",
                ColorImage::filled(
                    [SCREEN_WIDTH as usize, SCREEN_HEIGHT as usize],
                    Color32::BLACK,
                ),
                TextureOptions::NEAREST,
            ),
            step_by_cycles: 10000,
            step_by_frames: 1,
            speed: 0.0,
            penalty: 0,
            play: false,
            buttons_held: ButtonsHeld::default(),
            logger_open: false,
            interrupts_open: false,
            woke_ppu_open: false,
            woke_timers_open: false,
        }
    }

    fn draw_menu_bar(&mut self, ui: &mut Ui) {
        egui::MenuBar::new().ui(ui, |ui| {
            ui.menu_button("File", |ui| {
                if ui.button("Load Boot ROM").clicked() {
                    // self.dmg_state.load_boot_rom();
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

            ui.menu_button("Tools", |ui| {
                self.logger_open |= ui.button("Show Logger").clicked();
                self.interrupts_open |= ui.button("Show Interrupts").clicked();
                self.woke_ppu_open |= ui.button("Show Woke PPU").clicked();
                self.woke_timers_open |= ui.button("Show Woke Timers").clicked();
            });
        });
    }
}

fn step_once(dmg_state: &mut Cpu, buttons_held: ButtonsHeld) {
    if let Err(message) = dmg_state.execute() {
        error!("{message}");
    }
    dmg_state.memory.buttons_held = buttons_held;
}

fn step_multiple(steps: u32, dmg_state: &mut Cpu, buttons_held: ButtonsHeld) {
    for _ in 0..steps {
        step_once(dmg_state, buttons_held);
    }
}

fn step_vblank(dmg_state: &mut Cpu, buttons_held: ButtonsHeld) {
    for _ in 0..70224 {
        step_once(dmg_state, buttons_held);
    }
}

impl eframe::App for DebuggerApp {
    /// Called each time the UI needs repainting, which may be many times per second.
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::Window::new("Log")
            .open(&mut self.logger_open)
            .show(ui, |ui| {
                // Draws the logger UI.
                egui_logger::logger_ui().show(ui);
            });
        egui::Window::new("Interrupts")
            .open(&mut self.interrupts_open)
            .show(ui, |ui| {
                interrupts::show(ui, &self.dmg_state);
            });
        egui::Window::new("Woke PPU")
            .open(&mut self.woke_ppu_open)
            .show(ui, |ui| {
                woke_ppu::show(ui, &self.dmg_state);
            });
        egui::Window::new("Woke Timers")
            .open(&mut self.woke_timers_open)
            .show(ui, |ui| {
                woke_timers::show(ui, &self.dmg_state);
            });

        egui::Window::new("PPU Output").show(ui, |ui| {
            self.screen.set(
                ColorImage::from_gray(
                    [SCREEN_WIDTH as usize, SCREEN_HEIGHT as usize],
                    &self.dmg_state.memory.ppu.lcd_buffer,
                ),
                TextureOptions::NEAREST,
            );

            ui.add(egui::Image::from_texture(&self.screen).fit_to_original_size(2.0));
        });

        egui::Panel::top("top_panel").show_inside(ui, |ui| {
            self.draw_menu_bar(ui);
        });

        egui::Panel::right("Registers")
            .min_size(300.0)
            .show_inside(ui, |ui| {
                draw_register_table(ui, &self.dmg_state);

                ui.separator();

                draw_memory_table(ui, &self.dmg_state);
            });

        egui::Panel::left("Tiles")
            .min_size(300.0)
            .show_inside(ui, |ui| {
                draw_tile_table(ui, &mut self.tiles, &self.dmg_state);
            });

        egui::CentralPanel::default().show_inside(ui, |ui| {
            ui.horizontal(|ui| {
                draw_tile_map_0(ui, &mut self.tile_map_0, &self.dmg_state.memory.ppu);
                draw_tile_map_1(ui, &mut self.tile_map_1, &self.dmg_state.memory.ppu);
            });

            if ui.button("Step once").clicked() {
                println!(
                    "{:?}",
                    Opcode::decode(self.dmg_state.memory.read_byte(self.dmg_state.registers.pc))
                );
                if let Err(message) = self.dmg_state.execute() {
                    error!("{message}");
                }
            }

            ui.horizontal(|ui| {
                if ui.button("Step multiple").clicked() {
                    step_multiple(self.step_by_cycles, &mut self.dmg_state, self.buttons_held);
                }

                ui.add(Slider::new(&mut self.step_by_cycles, 0..=1_000_000));
            });

            ui.horizontal(|ui| {
                if ui.button("Step vblank").clicked() {
                    for _ in 0..self.step_by_frames {
                        step_vblank(&mut self.dmg_state, self.buttons_held);
                    }
                }

                ui.add(Slider::new(&mut self.step_by_frames, 0..=100));
            });

            ui.horizontal(|ui| {
                if ui.button("Play/pause").clicked() {
                    self.play = !self.play;
                }

                ui.add(Slider::new(&mut self.speed, -2.0..=2.0));
            });

            draw_oam_table(ui, &mut self.tiles, &self.dmg_state);
        });

        ui.input(|i| {
            if !i.raw.dropped_files.is_empty() {
                let dropped_file = i.raw.dropped_files.first().unwrap().clone();
                self.dmg_state = Cpu::default();
                //self.dmg_state = Cpu::post_boot_mgb();
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
            };

            if self.play {
                let dmg_clock_speed = 2.0_f32.powi(22);

                let cycles_to_execute =
                    i.unstable_dt * dmg_clock_speed * (10.0_f32.powf(self.speed));
                #[allow(clippy::cast_possible_truncation)]
                #[allow(clippy::cast_sign_loss)]
                let target_cycle =
                    (self.dmg_state.memory.clock + cycles_to_execute as u64) - self.penalty;
                self.penalty = 0;
                while self.dmg_state.memory.clock < target_cycle {
                    step_once(&mut self.dmg_state, self.buttons_held);
                }
                self.penalty += self.dmg_state.memory.clock - target_cycle;
            }
        });

        ui.request_repaint();
    }
}
