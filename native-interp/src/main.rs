use std::collections::VecDeque;
use std::fs;
use std::sync::Arc;
use std::time::Instant;

use error_iter::ErrorIter as _;
use hw_constants::io_regs::IF;
use interpreter::joypad::ButtonsHeld;
use log::error;
use pixels::{Error, Pixels, SurfaceTexture};
use winit::dpi::LogicalSize;
use winit::event::{Event, WindowEvent};
use winit::event_loop::EventLoop;
use winit::keyboard::KeyCode;
use winit::window::WindowAttributes;
use winit_input_helper::WinitInputHelper;

use interpreter::cpu::Cpu;

const WIDTH: u32 = 160;
const HEIGHT: u32 = 144;

#[derive(Default)]
pub struct AppState {
    mgb_state: Cpu,
    buttons_held: ButtonsHeld,
    next_vblank: u64,
    prev_frametimes: VecDeque<u128>,
}

fn main() -> Result<(), Error> {
    env_logger::init();
    let event_loop = EventLoop::new().unwrap();
    let mut input = WinitInputHelper::new();
    let window = {
        let size = LogicalSize::new(WIDTH as f64, HEIGHT as f64);
        #[allow(deprecated)]
        Arc::new(
            event_loop
                .create_window(
                    WindowAttributes::new()
                        .with_title("wataboy native interp")
                        .with_inner_size(size)
                        .with_min_inner_size(size),
                )
                .unwrap(),
        )
    };

    let mut pixels = {
        let window_size = window.inner_size();
        let surface_texture = SurfaceTexture::new(window_size.width, window_size.height, &window);
        Pixels::new(WIDTH, HEIGHT, surface_texture)?
    };
    let mut app_state = AppState::default();
	
	let path_str = std::env::args().nth(1).expect("no path given");
	let path = std::path::PathBuf::from(path_str);
	let rom = fs::read(path).expect("failed to read file");
	
	app_state.mgb_state.memory.load_rom(&rom);

    let res = event_loop.run(|event, elwt| {
        match event {
            Event::Resumed => {}
            Event::NewEvents(_) => input.step(),
            Event::AboutToWait => input.end_step(),
            Event::DeviceEvent { event, .. } => {
                input.process_device_event(&event);
            }
            Event::WindowEvent { event, .. } => {
                // Handle input events
                if input.process_window_event(&event) {
                    // Close events
                    if input.key_pressed(KeyCode::Escape) || input.close_requested() {
                        elwt.exit();
                        return;
                    }
					
					app_state.buttons_held = ButtonsHeld {
						start: input.key_held(KeyCode::Enter),
						select: input.key_held(KeyCode::Backspace),
						b: input.key_held(KeyCode::KeyX),
						a: input.key_held(KeyCode::KeyZ),
						down: input.key_held(KeyCode::ArrowDown),
						up: input.key_held(KeyCode::ArrowUp),
						left: input.key_held(KeyCode::ArrowLeft),
						right: input.key_held(KeyCode::ArrowRight),
					};

                    // Resize the window
                    if let Some(size) = input.window_resized()
                        && size.width > 0
                        && size.height > 0
                        && let Err(err) = pixels.resize_surface(size.width, size.height)
                    {
                        log_error("pixels.resize_surface", err);
                        elwt.exit();
                        return;
                    }
                }

                // Draw the current frame
                if event == WindowEvent::RedrawRequested {
                    app_state.draw(pixels.frame_mut());
                    if let Err(err) = pixels.render() {
                        log_error("pixels.render", err);
                        elwt.exit();
                        return;
                    }

                    // Update internal state and request a redraw
                    let now = Instant::now();
                    for _ in 0..10 {
                        app_state.step_vblank();
						app_state.mgb_state.memory.buttons_held = app_state.buttons_held;
                    }
                    let frametime = now.elapsed().as_millis();
                    app_state.prev_frametimes.push_front(frametime);
                    if app_state.prev_frametimes.len() >= 100 {
                        app_state.prev_frametimes.pop_back();
                    }
					let avg = app_state.prev_frametimes.iter().sum::<u128>() / app_state.prev_frametimes.len() as u128;
					let min = app_state.prev_frametimes.iter().min().unwrap();
					let max = app_state.prev_frametimes.iter().max().unwrap();
                    // TODO: enable this with an argument.
                    // println!("Frametimes: latest = {frametime}ms avg = {avg}ms min = {min}ms max = {max}ms of last 100");

                    window.request_redraw();
                }
            }
            _ => {}
        }
    });
    res.map_err(|e| Error::UserDefined(Box::new(e)))
}

fn log_error<E: std::error::Error + 'static>(method_name: &str, err: E) {
    error!("{method_name}() failed: {err}");
    for source in err.sources().skip(1) {
        error!("  Caused by: {source}");
    }
}

impl AppState {
    /// Update the Game Boy's state, emulating a frame worth of cycles.
    fn step_vblank(&mut self) {
        self.next_vblank += 70224;
		while self.mgb_state.memory.ppu.line_number >= 144
			&& self.mgb_state.memory.clock < self.next_vblank
		{
			self.mgb_state.execute().unwrap();
		}
		
		while self.mgb_state.memory.ppu.line_number < 144
			&& self.mgb_state.memory.clock < self.next_vblank
		{
			self.mgb_state.execute().unwrap();
		}
		
        // Force catch-up the PPU.
		let mmu = &mut self.mgb_state.memory;
		mmu.ppu.catch_up(mmu.clock, &mut mmu.buffer[IF as usize]);
    }

    /// Draw the Game Boy's lcd buffer state to the frame buffer.
    ///
    /// Assumes the default texture format: `wgpu::TextureFormat::Rgba8UnormSrgb`
    fn draw(&self, frame: &mut [u8]) {
        let lcd_buf = &self.mgb_state.memory.ppu.lcd_buffer;

        for (i, pixel) in frame.chunks_exact_mut(4).enumerate() {
            let greyscale = lcd_buf[i];

            let rgba = [greyscale, greyscale, greyscale, 0xff];

            pixel.copy_from_slice(&rgba);
        }
    }
}
