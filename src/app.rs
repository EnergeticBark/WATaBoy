use crate::memory::draw_memory_table;
use crate::registers::draw_register_table;
use crate::tiles::draw_tile_table;
use egui::TextureHandle;
use sm83_interp::cpu::Cpu;

const NINTENDO_LOGO: &[u8; 48] = include_bytes!("../nintendo_logo.bin");

pub struct PPUViewApp {
    dmg_state: Cpu,
    tiles: Vec<Option<TextureHandle>>,
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

impl eframe::App for PPUViewApp {
    /// Called each time the UI needs repainting, which may be many times per second.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            egui::MenuBar::new().ui(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Load Bootrom (dmg.bin)").clicked() {
                        self.dmg_state.load_boot_rom();
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
            if ui.button("Step").clicked() {
                self.dmg_state.execute();
            }

            if ui.button("Step 10,000x").clicked() {
                for _ in 0..10000 {
                    /* Cycle the LCD Y coordinate so the bootrom doesn't get stuck waiting for a v-blank.
                      Once I actually implement the PPU alongside the CPU, I'll want to do this with proper
                      timing. See: https://gbdev.io/pandocs/Rendering.html
                    */
                    self.dmg_state.memory[0xFF44] = (self.dmg_state.memory[0xFF44] + 1) % 154;
                    self.dmg_state.execute();
                }
            }
        });
    }
}
