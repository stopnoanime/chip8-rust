use super::commands::{BreakpointAction, Command, CommandError, CommandResult, SetTarget};
use crate::chip8::{Chip8Error, Chip8Runner, Chip8RunnerResult, Display};
use std::collections::HashSet;

pub struct Executor {
    is_running: bool,
    runner: Chip8Runner,
    breakpoints: HashSet<u16>,
}

impl Executor {
    pub fn new(runner: Chip8Runner) -> Self {
        Self {
            is_running: false,
            runner,
            breakpoints: HashSet::new(),
        }
    }

    pub fn poll(&mut self, dt: f32) -> Result<Chip8RunnerResult, Chip8Error> {
        if !self.is_running {
            return Ok(Chip8RunnerResult::Ok);
        }

        let result = self
            .runner
            .update_with_breakpoints(dt, Some(&self.breakpoints));

        if matches!(result, Err(_) | Ok(Chip8RunnerResult::HitBreakpoint)) {
            self.is_running = false;
        }

        result
    }

    pub fn execute(&mut self, command: Command) -> Result<CommandResult, CommandError> {
        match command {
            Command::Run => {
                self.execute_run();
                Ok(CommandResult::Ok)
            }
            Command::Pause => {
                self.execute_pause();
                Ok(CommandResult::Ok)
            }
            Command::Step => self.execute_step(),
            Command::Breakpoint { action } => self.handle_breakpoint(action),
            Command::Set { target, value } => self.handle_set(target, value),
            Command::Quit => Ok(CommandResult::Quit),
        }
    }

    pub fn execute_run(&mut self) {
        self.is_running = true;
    }

    pub fn execute_pause(&mut self) {
        self.is_running = false;
    }

    pub fn execute_step(&mut self) -> Result<CommandResult, CommandError> {
        self.runner.chip8_mut().cpu_cycle()?;
        Ok(CommandResult::Ok)
    }

    pub fn is_running(&self) -> bool {
        self.is_running
    }

    pub fn get_display(&self) -> &Display<bool> {
        &self.runner.chip8_ref().display
    }

    pub fn get_pc(&self) -> u16 {
        self.runner.chip8_ref().pc
    }

    pub fn get_i(&self) -> u16 {
        self.runner.chip8_ref().i
    }

    pub fn get_v(&self) -> &[u8; 16] {
        &self.runner.chip8_ref().v
    }

    pub fn get_stack(&self) -> &Vec<u16> {
        &self.runner.chip8_ref().stack
    }

    pub fn get_delay_timer(&self) -> u8 {
        self.runner.chip8_ref().delay_timer
    }

    pub fn get_sound_timer(&self) -> u8 {
        self.runner.chip8_ref().sound_timer
    }

    pub fn get_keypad(&self) -> &[bool; 16] {
        &self.runner.chip8_ref().keypad
    }

    pub fn runner_mut(&mut self) -> &mut Chip8Runner {
        &mut self.runner
    }

    fn handle_breakpoint(
        &mut self,
        action: BreakpointAction,
    ) -> Result<CommandResult, CommandError> {
        match action {
            BreakpointAction::Set { addr } => {
                self.breakpoints.insert(addr);
            }
            BreakpointAction::Clear { addr } => {
                self.breakpoints.remove(&addr);
            }
            BreakpointAction::ClearAll => {
                self.breakpoints.clear();
            }
            BreakpointAction::List => {
                return Ok(CommandResult::BreakpointList {
                    breakpoints: {
                        let mut bps: Vec<u16> = self.breakpoints.iter().cloned().collect();
                        bps.sort();
                        bps
                    },
                });
            }
        };

        Ok(CommandResult::Ok)
    }

    fn handle_set(&mut self, target: SetTarget, value: u16) -> Result<CommandResult, CommandError> {
        let chip8 = self.runner.chip8_mut();

        match target {
            SetTarget::V(reg) => {
                chip8.v[reg] = value as u8;
            }
            SetTarget::I => {
                chip8.i = value;
            }
            SetTarget::Pc => {
                chip8.pc = value;
            }
        }

        Ok(CommandResult::Ok)
    }
}
