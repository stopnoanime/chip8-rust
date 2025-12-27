use std::{
    path::PathBuf,
    time::{Duration, Instant},
};

use anyhow::Context;
use clap::{Parser, error::ErrorKind};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use ratatui::{
    DefaultTerminal, Frame,
    buffer::Buffer,
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span, Text},
    widgets::{
        Block, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, StatefulWidget, Widget,
        Wrap,
    },
};

use chip8_rust::{
    debugger::{Cli, Command, Executor},
    emu::{Chip8, Chip8Runner, Chip8RunnerResult, DISPLAY_X, DISPLAY_Y},
    u4,
};

const KEY_MAP: [KeyCode; 16] = [
    KeyCode::Char('x'), // 0x0
    KeyCode::Char('1'), // 0x1
    KeyCode::Char('2'), // 0x2
    KeyCode::Char('3'), // 0x3
    KeyCode::Char('q'), // 0x4
    KeyCode::Char('w'), // 0x5
    KeyCode::Char('e'), // 0x6
    KeyCode::Char('a'), // 0x7
    KeyCode::Char('s'), // 0x8
    KeyCode::Char('d'), // 0x9
    KeyCode::Char('z'), // 0xA
    KeyCode::Char('c'), // 0xB
    KeyCode::Char('4'), // 0xC
    KeyCode::Char('r'), // 0xD
    KeyCode::Char('f'), // 0xE
    KeyCode::Char('v'), // 0xF
];

// Key release events are not fired in terminals on Linux.
// To handle this, we implement a timeout after which we consider a key released.
const KEY_RELEASE_TIMEOUT: Duration = Duration::from_millis(50);

/// A widget for displaying output messages with scrolling support.
struct OutputBox {
    content: String,
    is_error: bool,
    scroll_state: ScrollbarState,
}

impl OutputBox {
    fn new(content: String) -> Self {
        Self {
            scroll_state: ScrollbarState::new(content.lines().count()),
            is_error: false,
            content,
        }
    }

    fn set(&mut self, content: String, is_error: bool) {
        *self = Self::new(content);
        self.is_error = is_error;
    }

    fn set_str(&mut self, content: &str, is_error: bool) {
        self.set(content.to_string(), is_error);
    }

    /// Scroll up.
    fn up(&mut self) {
        self.scroll_state.prev();
    }

    /// Scroll down.
    fn down(&mut self) {
        self.scroll_state.next();
    }
}

impl Widget for &OutputBox {
    fn render(self, area: Rect, buf: &mut Buffer) {
        Paragraph::new(self.content.as_str())
            .wrap(Wrap { trim: true })
            .scroll((self.scroll_state.get_position() as u16, 0))
            .block(
                Block::bordered()
                    .border_style(Style::new().fg(if self.is_error {
                        Color::Red
                    } else {
                        Color::White
                    }))
                    .title(" Output "),
            )
            .render(area, buf);

        Scrollbar::new(ScrollbarOrientation::VerticalRight).render(
            area,
            buf,
            &mut self.scroll_state.clone(),
        );
    }
}

struct App {
    executor: Executor,
    input: String,
    output: OutputBox,
    should_quit: bool,
    last_tick: Instant,
    last_command: Option<Command>,
    key_press_times: [Option<Instant>; 16],
}

impl App {
    fn new(rom: &[u8]) -> anyhow::Result<Self> {
        let mut chip8 = Chip8::default();
        chip8
            .load(rom)
            .context("Failed to load ROM into CHIP-8 memory")?;

        Ok(Self {
            executor: Executor::new(Chip8Runner::new(chip8)),
            input: String::new(),
            output: OutputBox::new("Enter 'help' for a list of commands.".to_string()),
            should_quit: false,
            last_tick: Instant::now(),
            last_command: None,
            key_press_times: [None; 16],
        })
    }

    /// Run the application loop.
    fn run(&mut self, terminal: &mut DefaultTerminal) -> anyhow::Result<()> {
        while !self.should_quit {
            let dt = self.last_tick.elapsed().as_secs_f32();
            self.last_tick = Instant::now();

            // Handles execution when debugger is in running mode
            match self.executor.poll(dt) {
                Ok(Chip8RunnerResult::HitBreakpoint) => {
                    self.output.set_str("Hit breakpoint", false)
                }
                Err(e) => self.output.set(e.to_string(), true),
                _ => {}
            }

            terminal.draw(|frame| self.draw(frame))?;

            self.check_key_timeout();

            if event::poll(Duration::from_millis(16))?
                && let Event::Key(key) = event::read()?
            {
                // We got a key event
                self.handle_key_event(key);
            }
        }

        Ok(())
    }

    fn draw(&self, frame: &mut Frame) {
        frame.render_widget(self, frame.area());
    }

    /// Checks for key release timeouts and updates keypad state accordingly.
    ///
    /// This is necessary because terminals do not send key release events.
    fn check_key_timeout(&mut self) {
        let now = Instant::now();

        for (idx, press_time) in self.key_press_times.iter_mut().enumerate() {
            if let Some(time) = press_time
                && now.duration_since(*time) > KEY_RELEASE_TIMEOUT
            {
                *press_time = None;
                self.executor
                    .runner_mut()
                    .set_key(u4::new(idx as u8), false);
            }
        }
    }

    fn handle_key_event(&mut self, key: KeyEvent) {
        // Handle Ctrl+C globally
        if key.code == KeyCode::Char('c') && key.modifiers.contains(event::KeyModifiers::CONTROL) {
            self.should_quit = true;
            return;
        }

        // Handles keyboard input based on the current state
        // - Running: Keys are mapped to the CHIP-8 keypad. Esc pauses.
        // - Paused: Keys control the debugger UI (scrolling, typing commands).
        if self.executor.is_running() {
            match key.code {
                KeyCode::Esc => {
                    self.executor.pause();
                    self.output.set_str("Paused", false);
                }
                _ => {
                    if let Some(idx) = KEY_MAP.iter().position(|&k| k == key.code) {
                        self.executor.runner_mut().set_key(u4::new(idx as u8), true);
                        self.key_press_times[idx] = Some(Instant::now());
                    }
                }
            }
        } else if key.kind == KeyEventKind::Press {
            match key.code {
                KeyCode::Esc => {
                    self.should_quit = true;
                }
                KeyCode::Enter => {
                    self.handle_enter();
                }
                KeyCode::Up => {
                    self.output.up();
                }
                KeyCode::Down => {
                    self.output.down();
                }
                KeyCode::Char(c) => {
                    self.input.push(c);
                }
                KeyCode::Backspace => {
                    self.input.pop();
                }
                _ => {}
            }
        }
    }

    fn handle_enter(&mut self) {
        if self.input.is_empty() && self.last_command.is_some() {
            self.execute_command(self.last_command.clone().unwrap());
        } else {
            match Cli::try_parse_from(self.input.split_whitespace()) {
                Ok(cli) => {
                    self.last_command = Some(cli.command.clone());
                    self.execute_command(cli.command);
                }
                Err(e) => {
                    self.last_command = None;
                    self.output
                        .set(e.to_string(), e.kind() != ErrorKind::DisplayHelp);
                }
            }
        }

        self.input.clear();
    }

    fn execute_command(&mut self, command: Command) {
        match self.executor.execute(command) {
            Ok(result) => match result {
                chip8_rust::debugger::CommandResult::Ok => {
                    self.output.set_str("OK", false);
                }
                chip8_rust::debugger::CommandResult::Quit => {
                    self.should_quit = true;
                }
                chip8_rust::debugger::CommandResult::Breakpoints(breakpoints) => {
                    if breakpoints.is_empty() {
                        self.output.set_str("No breakpoints set", false);
                    } else {
                        self.output.set(
                            breakpoints
                                .iter()
                                .map(|b| format!("Breakpoint: {b:#05X}\n"))
                                .collect(),
                            false,
                        )
                    };
                }
                chip8_rust::debugger::CommandResult::MemDump { data, offset } => {
                    self.output.set(
                        data.iter()
                            .enumerate()
                            .map(|(i, byte)| {
                                if i % 16 == 0 {
                                    format!("\n{:03X}: {byte:02X} ", offset.wrapping_add(i as u16))
                                } else {
                                    format!("{byte:02X} ")
                                }
                            })
                            .collect(),
                        false,
                    );
                }
                chip8_rust::debugger::CommandResult::Disasm {
                    instructions,
                    offset,
                } => {
                    self.output.set(
                        instructions
                            .iter()
                            .enumerate()
                            .map(|(i, (ins, opcode))| {
                                format!(
                                    "{:03X}: {ins:04X} - {opcode:X?}\n",
                                    offset.wrapping_add((i * 2) as u16)
                                )
                            })
                            .collect(),
                        false,
                    );
                }
            },
            Err(e) => {
                self.output.set(e.to_string(), true);
            }
        }
    }
}

impl Widget for &App {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Check if we have enough space
        const MIN_WIDTH: u16 = DISPLAY_X as u16 + 2 + 15 + 2;
        const MIN_HEIGHT: u16 = DISPLAY_Y as u16 + 2 + 1 + 2 + 1 + 2;
        if area.width < MIN_WIDTH || area.height < MIN_HEIGHT {
            let center = area.centered(Constraint::Length(45), Constraint::Length(3));

            Paragraph::new(format!(
                "Terminal is too small ({}x{} min)",
                MIN_WIDTH, MIN_HEIGHT
            ))
            .style(Style::default().fg(Color::Red))
            .alignment(Alignment::Center)
            .block(Block::bordered())
            .render(center, buf);

            return;
        }

        let [left, right] = Layout::horizontal([
            Constraint::Min(DISPLAY_X as u16 + 2),
            Constraint::Length(15 + 2),
        ])
        .areas(area);

        let [display, output, input] = Layout::vertical([
            Constraint::Length(DISPLAY_Y as u16 + 2),
            Constraint::Min(1 + 2),
            Constraint::Length(1 + 2),
        ])
        .areas(left);

        let [state, registers, keypad, stack] = Layout::vertical([
            Constraint::Length(1 + 2),
            Constraint::Length(11 + 2),
            Constraint::Length(4 + 2),
            Constraint::Min(1 + 2),
        ])
        .areas(right);

        self.render_display(display, buf);
        self.render_state(state, buf);
        self.render_registers(registers, buf);
        self.render_keypad(keypad, buf);
        self.render_stack(stack, buf);
        self.render_input(input, buf);
        self.output.render(output, buf);
    }
}

impl App {
    fn render_display(&self, area: Rect, buf: &mut Buffer) {
        let text: Vec<Line> = self
            .executor
            .get_display()
            .iter()
            .map(|row| {
                row.iter()
                    .map(|pixel| {
                        Span::styled(if *pixel { "█" } else { " " }, Style::default().green())
                    })
                    .collect()
            })
            .collect();

        Paragraph::new(text)
            .alignment(Alignment::Center)
            .block(Block::bordered().title(" Display "))
            .render(area, buf);
    }

    fn render_registers(&self, area: Rect, buf: &mut Buffer) {
        let mut lines = Vec::new();

        lines.push(Line::from(format!(
            "PC: {:03X}  I: {:03X}",
            self.executor.get_pc(),
            self.executor.get_i()
        )));
        lines.push(Line::from(format!(
            "DT: {:02X}   ST: {:02X}",
            self.executor.get_delay_timer(),
            self.executor.get_sound_timer()
        )));
        lines.push(Line::from(""));

        let v = self.executor.get_v();
        for idx in 0..8 {
            lines.push(Line::from(format!(
                "V{:X}: {:02X}   V{:X}: {:02X}",
                idx,
                v[idx],
                idx + 8,
                v[idx + 8]
            )));
        }

        Paragraph::new(lines)
            .block(Block::bordered().title(" Registers "))
            .render(area, buf);
    }

    fn render_stack(&self, area: Rect, buf: &mut Buffer) {
        let max_lines = area.height as usize - 2;

        let mut lines: Vec<Line> = self
            .executor
            .get_stack()
            .iter()
            .enumerate()
            .map(|(i, val)| Line::from(format!("{:02}: {:03X}", i, val)))
            .collect();

        if lines.is_empty() {
            lines.push(Line::from("Empty"));
        }

        if lines.len() > max_lines {
            // Display only the last `max_lines - 1` items with "..." at the top
            lines = std::iter::once(Line::from("..."))
                .chain(lines.into_iter().rev().take(max_lines - 1).rev())
                .collect();
        }

        Paragraph::new(lines)
            .alignment(Alignment::Center)
            .block(Block::bordered().title(" Stack "))
            .render(area, buf);
    }

    fn render_input(&self, area: Rect, buf: &mut Buffer) {
        Paragraph::new(self.input.as_str())
            .block(Block::bordered().title(" Command "))
            .render(area, buf);
    }

    fn render_state(&self, area: Rect, buf: &mut Buffer) {
        let (text, color) = if self.executor.is_running() {
            ("RUNNING", Color::Green)
        } else {
            ("PAUSED", Color::Yellow)
        };

        Paragraph::new(Text::styled(text, Style::default().fg(color)))
            .alignment(Alignment::Center)
            .block(Block::bordered().title(" State "))
            .render(area, buf);
    }

    fn render_keypad(&self, area: Rect, buf: &mut Buffer) {
        let keypad = self.executor.get_keypad();
        let layout = [
            [0x1, 0x2, 0x3, 0xC],
            [0x4, 0x5, 0x6, 0xD],
            [0x7, 0x8, 0x9, 0xE],
            [0xA, 0x0, 0xB, 0xF],
        ];

        let lines = layout
            .iter()
            .map(|row| {
                row.iter()
                    .map(|key| {
                        let key_str = format!("{:X}", key);

                        Span::styled(
                            key_str,
                            if keypad[*key] {
                                Style::default().fg(Color::Black).bg(Color::White)
                            } else {
                                Style::default()
                            },
                        )
                    })
                    .flat_map(|s| [s, Span::raw(" ")]) // Add space between keys
                    .take(row.len() * 2 - 1) // And remove the last space
                    .collect()
            })
            .collect::<Vec<Line>>();

        Paragraph::new(lines)
            .alignment(Alignment::Center)
            .block(Block::bordered().title(" Keypad "))
            .render(area, buf);
    }
}

/// TUI debugger for CHIP-8
///
/// Throughout the program all values are displayed in hex.
/// Input values can be decimal or hex (with 0x prefix).
/// When paused, Escape or Ctrl+C is used to exit the debugger and up arrow/down arrow to scroll output.
/// When paused, enter executes the last command if input is empty.
/// When running, keys 1-4, Q-R, A-F, Z-V map to CHIP-8 keys, and Escape pauses the debugger.
#[derive(Parser)]
#[command(max_term_width = 100)]
struct Args {
    /// Path to the CHIP-8 ROM file
    rom_path: PathBuf,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    let rom = std::fs::read(&args.rom_path).context("Failed to read ROM file")?;
    let mut app = App::new(&rom).context("Failed to initialize application")?;

    let mut terminal = ratatui::init();
    let app_result = app.run(&mut terminal);
    ratatui::restore();

    app_result
}
