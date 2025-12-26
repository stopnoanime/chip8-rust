use clap::{Args, Parser, Subcommand};
use clap_num::{maybe_hex, maybe_hex_range};

use crate::emu::Opcode;
use crate::u4;

/// CHIP-8 Debugger Command Line Interface
#[derive(Parser)]
#[command(multicall = true, disable_help_flag = true, name = "")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Clone)]
pub enum Command {
    /// Start execution
    #[command(visible_alias = "r")]
    Run,

    /// Pause execution
    #[command(visible_alias = "p")]
    Pause,

    /// Execute a single instruction
    #[command(visible_alias = "s")]
    Step,

    /// Exit the debugger
    #[command(visible_alias = "q")]
    Quit,

    /// Manage breakpoints
    #[command(visible_alias = "b")]
    Breakpoint {
        #[command(subcommand)]
        action: BreakpointAction,
    },

    /// Display memory contents
    #[command(visible_alias = "m")]
    Mem {
        #[command(flatten)]
        args: MemArgs,
    },

    /// Disassemble memory
    #[command(visible_alias = "d")]
    Disasm {
        #[command(flatten)]
        args: MemArgs,
    },

    /// Set a V register value
    #[command(visible_alias = "v")]
    SetV {
        /// Register index
        #[arg(value_parser = u4_parse)]
        idx: u4,

        /// The value
        #[arg(value_parser = maybe_hex::<u8>)]
        value: u8,
    },

    /// Set the I register
    #[command(visible_alias = "i")]
    SetI {
        /// The value
        #[arg(value_parser = u12_parse)]
        value: u16,
    },

    /// Set the program counter
    #[command(visible_alias = "pc")]
    SetPc {
        /// The value
        #[arg(value_parser = u12_parse)]
        value: u16,
    },

    /// Set key state
    #[command(visible_alias = "k")]
    SetKey {
        /// Key index
        #[arg(value_parser = u4_parse)]
        key: u4,

        /// The value (true/false)
        #[arg(action = clap::ArgAction::Set)]
        pressed: bool,
    },

    /// Set delay timer
    #[command(visible_alias = "dt")]
    SetDt {
        /// The value
        #[arg(value_parser = maybe_hex::<u8>)]
        value: u8,
    },

    /// Set sound timer
    #[command(visible_alias = "st")]
    SetSt {
        /// The value
        #[arg(value_parser = maybe_hex::<u8>)]
        value: u8,
    },

    /// Push value onto the stack
    #[command(visible_alias = "pu")]
    Push {
        /// The value
        #[arg(value_parser = u12_parse)]
        value: u16,
    },

    /// Pop value from the stack
    #[command(visible_alias = "po")]
    Pop,
}

pub enum CommandResult {
    Ok,
    Breakpoints(Vec<u16>),
    MemDump {
        data: Vec<u8>,
        offset: u16,
    },
    Disasm {
        instructions: Vec<(u16, Opcode)>,
        offset: u16,
    },
    Quit,
}

#[derive(Subcommand, Clone)]
pub enum BreakpointAction {
    /// Set a breakpoint at an address
    #[command(visible_alias = "s")]
    Set {
        /// The address
        #[arg(value_parser = u12_parse)]
        addr: u16,
    },

    /// Clear a breakpoint at an address
    #[command(visible_alias = "c")]
    Clear {
        /// The address
        #[arg(value_parser = u12_parse)]
        addr: u16,
    },

    /// List all breakpoints
    #[command(visible_alias = "l")]
    List,

    /// Clear all breakpoints
    #[command(visible_alias = "ca")]
    ClearAll,
}

#[derive(Args, Clone)]
pub struct MemArgs {
    /// Starting memory address
    #[arg(value_parser = u12_parse)]
    pub offset: u16,

    /// Number of bytes to display
    #[arg(default_value = "32", value_parser = u12_parse)]
    pub len: u16,
}

fn u12_parse(s: &str) -> Result<u16, String> {
    maybe_hex_range(s, 0, 0xFFF)
}

fn u4_parse(s: &str) -> Result<u4, String> {
    maybe_hex_range(s, 0, 0xF).map(u4::new)
}
