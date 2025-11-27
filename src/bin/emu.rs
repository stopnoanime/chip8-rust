use macroquad::prelude::*;

use chip8_rust::{Chip8, DISPLAY_X, DISPLAY_Y};

#[macroquad::main("64x32 Display")]
async fn main() {
    let mut chip8 = Chip8::new();

    const ROM_DATA: &[u8] = include_bytes!("../../ibm-logo.ch8");
    chip8.load_rom(ROM_DATA);

    loop {
        chip8.cpu_cycle();

        // Scale to fit the screen
        let scale_x = screen_width() / DISPLAY_X as f32;
        let scale_y = screen_height() / DISPLAY_Y as f32;
        let scale = scale_x.min(scale_y).floor().max(1.0);

        // Offset to center the display
        let offset_x = ((screen_width() - (DISPLAY_X as f32 * scale)) / 2.0).round();
        let offset_y = ((screen_height() - (DISPLAY_Y as f32 * scale)) / 2.0).round();

        // Update the display
        clear_background(BLACK);
        for (y, row) in chip8.display.iter().enumerate() {
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

        next_frame().await
    }
}
