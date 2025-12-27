use super::{Chip8, Chip8Error, Chip8Result};
use crate::{u4, u12};
use std::collections::HashSet;

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

pub enum Chip8RunnerResult {
    HitBreakpoint,
    Ok,
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
    /// Returns early if a frame has to be rendered before the next CPU cycle.
    pub fn update(&mut self, dt: f32) -> Result<Chip8RunnerResult, Chip8Error> {
        self.update_with_breakpoints(dt, None)
    }

    /// Like `update` but checks for breakpoints after each CPU cycle.
    pub fn update_with_breakpoints(
        &mut self,
        dt: f32,
        breakpoints: Option<&HashSet<u12>>,
    ) -> Result<Chip8RunnerResult, Chip8Error> {
        self.cpu_dt_accumulator += dt;
        self.timer_dt_accumulator += dt;

        while self.timer_dt_accumulator >= TIMER_TIME_STEP {
            self.timer_dt_accumulator -= TIMER_TIME_STEP;
            self.chip8.timers_cycle();
        }

        while self.cpu_dt_accumulator >= CPU_TIME_STEP {
            self.cpu_dt_accumulator -= CPU_TIME_STEP;

            let cpu_result = self.chip8.cpu_cycle()?;

            if let Some(breakpoints) = &breakpoints
                && breakpoints.contains(&self.chip8.pc)
            {
                self.cpu_dt_accumulator = 0.0;
                return Ok(Chip8RunnerResult::HitBreakpoint);
            }

            match cpu_result {
                Chip8Result::WaitForNextFrame => {
                    // If we need to wait for the next frame we stop executing cycles.
                    // We clear the accumulator to avoid "catching up" in the next frame.
                    self.cpu_dt_accumulator = 0.0;
                    break;
                }
                Chip8Result::Continue => {}
            }
        }

        Ok(Chip8RunnerResult::Ok)
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

    pub fn chip8_ref(&self) -> &Chip8 {
        &self.chip8
    }

    pub fn chip8_mut(&mut self) -> &mut Chip8 {
        &mut self.chip8
    }
}
