use std::{path::PathBuf, sync::Arc, time::Instant};

use anyhow::Context;
use clap::Parser;
use pixels::{Pixels, SurfaceTexture};
use rodio::{OutputStream, OutputStreamBuilder, Sink, Source, source::SquareWave};
use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::{ElementState, KeyEvent, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::{Key, KeyCode, NamedKey},
    window::{Window, WindowId},
};

use chip8_rust::{Chip8, Chip8Runner, DISPLAY_X, DISPLAY_Y, Display, u4};

/// The rate at which pixels fade out (phosphor decay).
const DISPLAY_PHOSPHOR_RATE: f32 = 10.0;

/// Mapping from physical keyboard keys to CHIP-8 hex keypad (0x0-0xF).
const KEY_MAP: [KeyCode; 16] = [
    KeyCode::KeyX,   // 0x00
    KeyCode::Digit1, // 0x01
    KeyCode::Digit2, // 0x02
    KeyCode::Digit3, // 0x03
    KeyCode::KeyQ,   // 0x04
    KeyCode::KeyW,   // 0x05
    KeyCode::KeyE,   // 0x06
    KeyCode::KeyA,   // 0x07
    KeyCode::KeyS,   // 0x08
    KeyCode::KeyD,   // 0x09
    KeyCode::KeyZ,   // 0x0A
    KeyCode::KeyC,   // 0x0B
    KeyCode::Digit4, // 0x0C
    KeyCode::KeyR,   // 0x0D
    KeyCode::KeyF,   // 0x0E
    KeyCode::KeyV,   // 0x0F
];

struct App {
    pixels: Option<Pixels<'static>>,
    window: Option<Arc<Window>>,
    /// Stores the brightness of each pixel (0.0 to 1.0) to implement phosphor decay.
    display_float: Display<f32>,

    /// Audio output stream (must be kept alive).
    _audio_stream: OutputStream,
    audio_sink: Sink,

    runner: Chip8Runner,
    /// Used for delta time calculation.
    last_frame_instant: Instant,

    /// Stores the result of the application to be returned from main.
    exit_result: anyhow::Result<()>,
}

impl App {
    fn new(rom: &[u8]) -> anyhow::Result<Self> {
        // Initialize audio
        let mut _audio_stream = OutputStreamBuilder::open_default_stream()
            .context("Failed to open audio output stream")?;
        _audio_stream.log_on_drop(false);

        let audio_sink = Sink::connect_new(_audio_stream.mixer());
        audio_sink.pause();
        audio_sink.append(SquareWave::new(440.0).amplify(0.5));

        // Initialize CHIP-8
        let mut chip8 = Chip8::default();
        chip8
            .load(rom)
            .context("Failed to load ROM into CHIP-8 memory")?;
        let runner = Chip8Runner::new(chip8);

        Ok(Self {
            pixels: None,
            window: None,
            display_float: [[0.0; DISPLAY_X]; DISPLAY_Y],

            _audio_stream,
            audio_sink,

            runner,
            last_frame_instant: Instant::now(),
            exit_result: Ok(()),
        })
    }

    fn process_display(&mut self, dt: f32) {
        let buff = self.pixels.as_mut().unwrap().frame_mut();

        for (i, pxl) in buff.chunks_exact_mut(4).enumerate() {
            let x = i % DISPLAY_X;
            let y = i / DISPLAY_X;

            // We use display_float to track the "brightness" of each pixel over time.
            // This allows us to implement a phosphor decay effect where pixels fade out
            // slowly instead of turning off instantly.
            self.display_float[y][x] = if self.runner.get_display_pixel(y, x) {
                // Pixel is currently on, set to full brightness
                1.0
            } else {
                // Pixel is off, but we decay the previous brightness value based on elapsed time
                (self.display_float[y][x] - DISPLAY_PHOSPHOR_RATE * dt).max(0.0)
            };

            let rgba = [0, 0xff, 0, (self.display_float[y][x] * 255.0) as u8];
            pxl.copy_from_slice(&rgba);
        }
    }

    fn try_resumed(&mut self, event_loop: &ActiveEventLoop) -> anyhow::Result<()> {
        let window = {
            let size = LogicalSize::new(DISPLAY_X as u32 * 10, DISPLAY_Y as u32 * 10);
            let min_size = LogicalSize::new(DISPLAY_X as u32, DISPLAY_Y as u32);

            Arc::new(
                event_loop
                    .create_window(
                        Window::default_attributes()
                            .with_title("chip8-rust")
                            .with_inner_size(size)
                            .with_min_inner_size(min_size),
                    )
                    .context("Failed to create window")?,
            )
        };

        self.window = Some(window.clone());
        self.pixels = {
            let window_size = window.inner_size();
            let surface_texture =
                SurfaceTexture::new(window_size.width, window_size.height, window.clone());

            let pixels = Pixels::new(DISPLAY_X as u32, DISPLAY_Y as u32, surface_texture)
                .context("Failed to create pixels surface")?;

            window.request_redraw();
            Some(pixels)
        };

        // Avoid large dt on first frame
        self.last_frame_instant = Instant::now();
        Ok(())
    }

    fn try_window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        event: WindowEvent,
    ) -> anyhow::Result<()> {
        match event {
            WindowEvent::CloseRequested
            | WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        logical_key: Key::Named(NamedKey::Escape),
                        ..
                    },
                ..
            } => {
                event_loop.exit();
            }

            WindowEvent::Resized(size) => {
                self.pixels
                    .as_mut()
                    .unwrap()
                    .resize_surface(size.width, size.height)
                    .context("Failed to resize pixels surface")?;
            }

            WindowEvent::RedrawRequested => {
                let now = Instant::now();
                let dt = (now - self.last_frame_instant).as_secs_f32();
                self.last_frame_instant = now;

                self.runner.update(dt).context("Chip8 Execution error")?;

                if self.runner.should_beep() {
                    self.audio_sink.play();
                } else {
                    self.audio_sink.pause();
                }

                self.process_display(dt);

                self.pixels
                    .as_ref()
                    .unwrap()
                    .render()
                    .context("Pixels render error")?;

                self.window.as_ref().unwrap().request_redraw();
            }

            WindowEvent::KeyboardInput { event, .. } => match event.state {
                ElementState::Pressed => {
                    if let Some(key) = KEY_MAP.iter().position(|&k| k == event.physical_key) {
                        self.runner.set_key(u4::new(key as u8), true);
                    }
                }
                ElementState::Released => {
                    if let Some(key) = KEY_MAP.iter().position(|&k| k == event.physical_key) {
                        self.runner.set_key(u4::new(key as u8), false);
                    }
                }
            },

            _ => (),
        }
        Ok(())
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if let Err(e) = self.try_resumed(event_loop) {
            self.exit_result = Err(e);
            event_loop.exit();
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        if let Err(e) = self.try_window_event(event_loop, event) {
            self.exit_result = Err(e);
            event_loop.exit();
        }
    }
}

/// CHIP-8 emulator written in Rust.
///
/// Keys 1-4, Q-R, A-F, Z-V map to CHIP-8 keys.
/// Escape is used to exit the emulator.
#[derive(Parser, Debug)]
#[command(about)]
struct Args {
    /// Path to the CHIP-8 ROM file
    rom_path: PathBuf,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    let rom = std::fs::read(&args.rom_path).context("Failed to read ROM file")?;

    let event_loop = EventLoop::new().context("Failed to create event loop")?;
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut app = App::new(&rom).context("Failed to initialize application")?;
    event_loop
        .run_app(&mut app)
        .context("Error occurred during event loop execution")?;

    // Return the result captured during the event loop
    app.exit_result
}
