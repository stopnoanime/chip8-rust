use macroquad::prelude::*;
use rodio::{OutputStreamBuilder, Sink, Source, source::SquareWave};

use chip8_rust::{
    CPU_TIME_STEP, Chip8, CycleResult, DISPLAY_X, DISPLAY_Y, Display, TIMER_TIME_STEP,
};

const KEY_MAP: [KeyCode; 16] = [
    KeyCode::X,    // 0x00
    KeyCode::Key1, // 0x01
    KeyCode::Key2, // 0x02
    KeyCode::Key3, // 0x03
    KeyCode::Q,    // 0x04
    KeyCode::W,    // 0x05
    KeyCode::E,    // 0x06
    KeyCode::A,    // 0x07
    KeyCode::S,    // 0x08
    KeyCode::D,    // 0x09
    KeyCode::Z,    // 0x0A
    KeyCode::C,    // 0x0B
    KeyCode::Key4, // 0x0C
    KeyCode::R,    // 0x0D
    KeyCode::F,    // 0x0E
    KeyCode::V,    // 0x0F
];

fn update_keypad(chip8: &mut Chip8) {
    for (i, &key) in KEY_MAP.iter().enumerate() {
        chip8.keypad[i] = is_key_down(key);
    }
}

fn draw_display(display: &Display) {
    // Scale to fit the screen
    let scale_x = screen_width() / DISPLAY_X as f32;
    let scale_y = screen_height() / DISPLAY_Y as f32;
    let scale = scale_x.min(scale_y).floor().max(1.0);

    // Offset to center the display
    let offset_x = ((screen_width() - (DISPLAY_X as f32 * scale)) / 2.0).round();
    let offset_y = ((screen_height() - (DISPLAY_Y as f32 * scale)) / 2.0).round();

    clear_background(BLACK);
    for (y, row) in display.iter().enumerate() {
        for (x, &pixel) in row.iter().enumerate() {
            if pixel {
                draw_rectangle(
                    offset_x + (x as f32 * scale),
                    offset_y + (y as f32 * scale),
                    scale,
                    scale,
                    WHITE,
                );
            }
        }
    }
}

#[macroquad::main("64x32 Display")]
async fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() != 2 {
        eprintln!("Usage: {} <rom_path>", args[0]);
        std::process::exit(1);
    }

    let rom = std::fs::read(&args[1]).expect("Failed to read ROM file");

    let mut chip8 = Chip8::new();
    chip8.load_rom(&rom);

    let mut audio_stream =
        OutputStreamBuilder::open_default_stream().expect("Failed to open audio output stream");
    audio_stream.log_on_drop(false);

    let audio_sink = Sink::connect_new(audio_stream.mixer());
    audio_sink.pause();
    audio_sink.append(SquareWave::new(440.0).amplify(0.5));

    let mut cpu_dt_accumulator = 0.0;
    let mut timer_dt_accumulator = 0.0;

    loop {
        if is_key_pressed(KeyCode::Escape) {
            break;
        }

        let dt = get_frame_time();
        cpu_dt_accumulator += dt;
        timer_dt_accumulator += dt;

        update_keypad(&mut chip8);

        while cpu_dt_accumulator >= CPU_TIME_STEP {
            cpu_dt_accumulator -= CPU_TIME_STEP;

            match chip8.cpu_cycle() {
                CycleResult::Draw => {
                    // Limit to one draw per frame
                    break;
                }
                CycleResult::UnknownOpcode { addr, opcode } => {
                    println!("Unknown opcode at {:04X}: {:04X}", addr, opcode);
                }
                CycleResult::Continue => {}
            }
        }

        while timer_dt_accumulator >= TIMER_TIME_STEP {
            chip8.timers_cycle();
            timer_dt_accumulator -= TIMER_TIME_STEP;
        }

        if chip8.should_beep() {
            audio_sink.play();
        } else {
            audio_sink.pause();
        }

        draw_display(&chip8.display);
        draw_fps();

        next_frame().await
    }
}
