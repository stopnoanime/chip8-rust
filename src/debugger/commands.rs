use clap::{Parser, Subcommand};
use clap_num::maybe_hex;

use crate::u4;

#[derive(Parser)]
#[command(multicall = true)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    #[command(visible_alias = "r")]
    Run,

    #[command(visible_alias = "p")]
    Pause,

    #[command(visible_alias = "s")]
    Step,

    #[command(visible_alias = "b")]
    Breakpoint {
        #[command(subcommand)]
        action: BreakpointAction,
    },

    Set {
        #[arg(value_parser = parse_set_target)]
        target: SetTarget,
        #[arg(value_parser = maybe_hex::<u16>)]
        value: u16,
    },

    // #[command(visible_alias = "m")]
    // Mem {
    //     #[arg(default_value = "0", value_parser = maybe_hex::<u16>)]
    //     start: u16,
    //     #[arg(default_value = "16", value_parser = maybe_hex::<u16>)]
    //     len: u16,
    // },

    // #[command(visible_alias = "d")]
    // Disasm {
    //     #[arg(default_value = "0", value_parser = maybe_hex::<u16>)]
    //     start: u16,
    //     #[arg(default_value = "16", value_parser = maybe_hex::<u16>)]
    //     len: u16,
    // },
    #[command(visible_alias = "q")]
    Quit,
}

pub enum CommandResult {
    Ok,
    BreakpointList { breakpoints: Vec<u16> },
    Quit,
}

#[derive(Debug, thiserror::Error)]
pub enum CommandError {
    #[error("Error while executing cpu instruction: {0}")]
    Chip8Error(#[from] crate::chip8::Chip8Error),
    #[error("Value out of range")]
    ValueOutOfRange,
}

#[derive(Subcommand)]
pub enum BreakpointAction {
    #[command(visible_alias = "s")]
    Set {
        #[arg(value_parser = maybe_hex::<u16>)]
        addr: u16,
    },

    #[command(visible_alias = "c")]
    Clear {
        #[arg(value_parser = maybe_hex::<u16>)]
        addr: u16,
    },

    #[command(visible_alias = "l")]
    List,

    #[command(visible_alias = "ca")]
    ClearAll,
}

#[derive(Clone)]
pub enum SetTarget {
    V(u4),
    I,
    Pc,
}

fn parse_set_target(s: &str) -> Result<SetTarget, String> {
    let lower = s.to_lowercase();

    match lower.as_str() {
        "index" | "i" => Ok(SetTarget::I),
        "pc" => Ok(SetTarget::Pc),

        _ if lower.starts_with('v') => {
            let hex_str = &lower[1..];
            match u8::from_str_radix(hex_str, 16) {
                Ok(val) if val < 16 => Ok(SetTarget::V(u4::new(val))),
                _ => Err(format!("Invalid register: '{}'", s)),
            }
        }

        _ => Err(format!("Unknown set target: '{}'", s)),
    }
}
