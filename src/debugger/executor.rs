use super::commands::{BreakpointAction, Command, CommandResult};
use crate::chip8::{Chip8Error, Chip8Runner, Chip8RunnerResult, Display, MEMORY_SIZE, Opcode};
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

    pub fn execute(&mut self, command: Command) -> Result<CommandResult, Chip8Error> {
        match command {
            Command::Run => self.run(),
            Command::Pause => self.pause(),
            Command::Step => return self.step(),
            Command::Quit => return Ok(CommandResult::Quit),
            Command::Breakpoint { action } => return Ok(self.handle_breakpoint(action)),
            Command::Mem { args } => return Ok(self.handle_mem(args.offset, args.len)),
            Command::Disasm { args } => return Ok(self.handle_disasm(args.offset, args.len)),
            Command::SetV { idx, value } => self.runner.chip8_mut().v[idx] = value,
            Command::SetI { value } => self.runner.chip8_mut().i = value,
            Command::SetPc { value } => self.runner.chip8_mut().pc = value,
            Command::SetKey { key, pressed } => self.runner.chip8_mut().keypad[key] = pressed,
            Command::SetDt { value } => self.runner.chip8_mut().delay_timer = value,
            Command::SetSt { value } => self.runner.chip8_mut().sound_timer = value,
            Command::Push { value } => self.runner.chip8_mut().stack.push(value),
            Command::Pop => {
                self.runner.chip8_mut().stack.pop();
            }
        };

        Ok(CommandResult::Ok)
    }

    pub fn run(&mut self) {
        self.is_running = true;
    }

    pub fn pause(&mut self) {
        self.is_running = false;
    }

    pub fn step(&mut self) -> Result<CommandResult, Chip8Error> {
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

    fn handle_breakpoint(&mut self, action: BreakpointAction) -> CommandResult {
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
                return CommandResult::Breakpoints({
                    let mut bps: Vec<u16> = self.breakpoints.iter().cloned().collect();
                    bps.sort();
                    bps
                });
            }
        };

        CommandResult::Ok
    }

    fn handle_mem(&self, offset: u16, len: u16) -> CommandResult {
        let end = MEMORY_SIZE.min(offset as usize + len as usize);
        let data = self.runner.chip8_ref().memory[offset as usize..end].to_vec();

        CommandResult::MemDump { data, offset }
    }

    fn handle_disasm(&self, offset: u16, len: u16) -> CommandResult {
        let end = MEMORY_SIZE.min(offset as usize + len as usize);
        let mut instructions = Vec::new();
        let mut pc = offset as usize;

        while pc < end {
            let value = u16::from_be_bytes(
                self.runner.chip8_ref().memory[pc..pc + 2]
                    .try_into()
                    .unwrap(),
            );

            let opcode = Opcode::decode(value);

            instructions.push((value, opcode));
            pc = pc + 2;
        }

        CommandResult::Disasm {
            instructions,
            offset,
        }
    }
}
