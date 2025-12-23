use std::{sync::Arc, time::Instant};

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

use chip8_rust::{
    CPU_TIME_STEP, Chip8, Chip8Error, Chip8Result, DISPLAY_X, DISPLAY_Y, Display, TIMER_TIME_STEP,
    u4,
};

const DISPLAY_PHOSPHOR_RATE: f32 = 10.0;
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
    display_float: Display<f32>,

    _audio_stream: OutputStream,
    audio_sink: Sink,

    chip8: Chip8,
    last_frame_instant: Instant,
    cpu_dt_accumulator: f32,
    timer_dt_accumulator: f32,
}

impl App {
    fn new(rom: &[u8]) -> Self {
        let mut _audio_stream =
            OutputStreamBuilder::open_default_stream().expect("Failed to open audio output stream");
        _audio_stream.log_on_drop(false);

        let audio_sink = Sink::connect_new(_audio_stream.mixer());
        audio_sink.pause();
        audio_sink.append(SquareWave::new(440.0).amplify(0.5));

        let mut chip8 = Chip8::default();
        chip8.load_rom(rom).expect("Failed to load ROM");

        Self {
            pixels: None,
            window: None,
            display_float: [[0.0; DISPLAY_X]; DISPLAY_Y],

            _audio_stream,
            audio_sink,

            chip8,
            last_frame_instant: Instant::now(),
            cpu_dt_accumulator: 0.0,
            timer_dt_accumulator: 0.0,
        }
    }

    fn process_cpu(&mut self, dt: f32) -> Result<(), Chip8Error> {
        self.cpu_dt_accumulator += dt;
        self.timer_dt_accumulator += dt;

        while self.cpu_dt_accumulator >= CPU_TIME_STEP {
            self.cpu_dt_accumulator -= CPU_TIME_STEP;
            match self.chip8.cpu_cycle() {
                Ok(Chip8Result::NextFrame) => {
                    // Don't execute further cycles this frame
                    break;
                }
                Ok(Chip8Result::Continue) => {}
                Err(e) => {
                    return Err(e);
                }
            }
        }

        while self.timer_dt_accumulator >= TIMER_TIME_STEP {
            self.chip8.timers_cycle();
            self.timer_dt_accumulator -= TIMER_TIME_STEP;
        }

        if self.chip8.should_beep() {
            self.audio_sink.play();
        } else {
            self.audio_sink.pause();
        }

        Ok(())
    }

    fn process_display(&mut self, dt: f32) {
        let buff = self.pixels.as_mut().unwrap().frame_mut();

        for (i, pxl) in buff.chunks_exact_mut(4).enumerate() {
            let x = i % DISPLAY_X;
            let y = i / DISPLAY_X;

            self.display_float[y][x] = if self.chip8.get_display_pixel(y, x) {
                1.0
            } else {
                (self.display_float[y][x] - DISPLAY_PHOSPHOR_RATE * dt).max(0.0)
            };

            let rgba = [0, 0xff, 0, (self.display_float[y][x] * 255.0) as u8];
            pxl.copy_from_slice(&rgba);
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
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
                    .unwrap(),
            )
        };

        self.window = Some(window.clone());
        self.pixels = {
            let window_size = window.inner_size();
            let surface_texture =
                SurfaceTexture::new(window_size.width, window_size.height, window.clone());

            match Pixels::new(DISPLAY_X as u32, DISPLAY_Y as u32, surface_texture) {
                Ok(pixels) => {
                    window.request_redraw();
                    Some(pixels)
                }
                Err(err) => {
                    eprintln!("Error creating pixels surface: {}", err);
                    event_loop.exit();
                    None
                }
            }
        };

        // Avoid large dt on first frame
        self.last_frame_instant = Instant::now();
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
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
                if let Err(err) = self
                    .pixels
                    .as_mut()
                    .unwrap()
                    .resize_surface(size.width, size.height)
                {
                    eprintln!("Error resizing pixels surface: {}", err);
                    event_loop.exit();
                }
            }

            WindowEvent::RedrawRequested => {
                let now = Instant::now();
                let dt = (now - self.last_frame_instant).as_secs_f32();
                self.last_frame_instant = now;

                if let Err(e) = self.process_cpu(dt) {
                    eprintln!("Chip8 Error: {:?}", e);
                    event_loop.exit();
                    return;
                }

                self.process_display(dt);

                if let Err(e) = self.pixels.as_ref().unwrap().render() {
                    eprintln!("Pixels render error: {}", e);
                    event_loop.exit();
                    return;
                }

                self.window.as_ref().unwrap().request_redraw();
            }

            WindowEvent::KeyboardInput { event, .. } => match event.state {
                ElementState::Pressed => {
                    if let Some(key) = KEY_MAP.iter().position(|&k| k == event.physical_key) {
                        self.chip8.set_key(u4::new(key as u8), true);
                    }
                }
                ElementState::Released => {
                    if let Some(key) = KEY_MAP.iter().position(|&k| k == event.physical_key) {
                        self.chip8.set_key(u4::new(key as u8), false);
                    }
                }
            },

            _ => (),
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() != 2 || args[1] == "-h" {
        println!(
            "Usage:\n\t{} <rom_path>\n\n\
            Description:\n\tThis is a CHIP-8 emulator written in Rust.\n\n\
            Keybindings:\n\t1-4, Q-R, A-F, Z-V: Map to CHIP-8 keys\n\t\
            Escape: Exit the emulator\n",
            args[0]
        );
        std::process::exit(1);
    }

    let rom = std::fs::read(&args[1]).expect("Failed to read ROM file");

    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut app = App::new(&rom);
    event_loop.run_app(&mut app).expect("Error running app");
}
