use crate::{Chip8, Chip8Error, Chip8Result, u4};

const CPU_HZ: f32 = 700.0;
const TIMER_HZ: f32 = 60.0;

const CPU_TIME_STEP: f32 = 1.0 / CPU_HZ;
const TIMER_TIME_STEP: f32 = 1.0 / TIMER_HZ;

/// High-level emulator runner that manages timing internally.
pub struct Chip8Runner {
    chip8: Chip8,
    cpu_dt_accumulator: f32,
    timer_dt_accumulator: f32,
}

impl Chip8Runner {
    pub fn new(chip8: Chip8) -> Self {
        Self {
            chip8,
            cpu_dt_accumulator: 0.0,
            timer_dt_accumulator: 0.0,
        }
    }

    /// Update emulator by delta time, handles both CPU and timer cycles.
    ///
    /// Runs as many CPU cycles and timer updates as needed based on the elapsed time `dt`.
    /// Returns early if a frame has to be rendered before the next CPU cycle (Chip8Result::WaitForNextFrame).
    pub fn update(&mut self, dt: f32) -> Result<Chip8Result, Chip8Error> {
        self.cpu_dt_accumulator += dt;
        self.timer_dt_accumulator += dt;

        while self.timer_dt_accumulator >= TIMER_TIME_STEP {
            self.chip8.timers_cycle();
            self.timer_dt_accumulator -= TIMER_TIME_STEP;
        }

        while self.cpu_dt_accumulator >= CPU_TIME_STEP {
            self.cpu_dt_accumulator -= CPU_TIME_STEP;
            match self.chip8.cpu_cycle()? {
                Chip8Result::WaitForNextFrame => {
                    // If we need to wait for the next frame we stop executing cycles.
                    // We also clear the accumulator to avoid "catching up" too fast in the next frame.
                    self.cpu_dt_accumulator = 0.0;
                    return Ok(Chip8Result::WaitForNextFrame);
                }
                Chip8Result::Continue => {}
            }
        }

        Ok(Chip8Result::Continue)
    }

    /// Returns true if the sound timer is active, indicating a beep should be played.
    pub fn should_beep(&self) -> bool {
        self.chip8.should_beep()
    }

    /// Set the state of a key on the keypad.
    pub fn set_key(&mut self, key: u4, pressed: bool) {
        self.chip8.set_key(key, pressed)
    }

    /// Get the state of a pixel on the display (true = on, false = off).
    pub fn get_display_pixel(&self, y: usize, x: usize) -> bool {
        self.chip8.get_display_pixel(y, x)
    }
}
