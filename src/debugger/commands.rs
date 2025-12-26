use clap::{Args, Parser, Subcommand};
use clap_num::{maybe_hex, maybe_hex_range};

use crate::chip8::Opcode;
use crate::u4;

#[derive(Parser)]
#[command(multicall = true)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Clone)]
pub enum Command {
    #[command(visible_alias = "r")]
    Run,

    #[command(visible_alias = "p")]
    Pause,

    #[command(visible_alias = "s")]
    Step,

    #[command(visible_alias = "q")]
    Quit,

    #[command(visible_alias = "b")]
    Breakpoint {
        #[command(subcommand)]
        action: BreakpointAction,
    },

    #[command(visible_alias = "m")]
    Mem {
        #[command(flatten)]
        args: MemArgs,
    },

    #[command(visible_alias = "d")]
    Disasm {
        #[command(flatten)]
        args: MemArgs,
    },

    #[command(visible_alias = "v")]
    SetV {
        #[arg(value_parser = u4_parse)]
        idx: u4,

        #[arg(value_parser = maybe_hex::<u8>)]
        value: u8,
    },

    #[command(visible_alias = "i")]
    SetI {
        #[arg(value_parser = u12_parse)]
        value: u16,
    },

    #[command(visible_alias = "pc")]
    SetPc {
        #[arg(value_parser = u12_parse)]
        value: u16,
    },

    #[command(visible_alias = "k")]
    SetKey {
        #[arg(value_parser = u4_parse)]
        key: u4,

        #[arg(action = clap::ArgAction::Set)]
        pressed: bool,
    },

    #[command(visible_alias = "dt")]
    SetDt {
        #[arg(value_parser = maybe_hex::<u8>)]
        value: u8,
    },

    #[command(visible_alias = "st")]
    SetSt {
        #[arg(value_parser = maybe_hex::<u8>)]
        value: u8,
    },

    #[command(visible_alias = "pu")]
    Push {
        #[arg(value_parser = u12_parse)]
        value: u16,
    },

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
    #[command(visible_alias = "s")]
    Set {
        #[arg(value_parser = u12_parse)]
        addr: u16,
    },

    #[command(visible_alias = "c")]
    Clear {
        #[arg(value_parser = u12_parse)]
        addr: u16,
    },

    #[command(visible_alias = "l")]
    List,

    #[command(visible_alias = "ca")]
    ClearAll,
}

#[derive(Args, Clone)]
pub struct MemArgs {
    #[arg(value_parser = u12_parse)]
    pub offset: u16,

    #[arg(default_value = "16", value_parser = u12_parse)]
    pub len: u16,
}

fn u12_parse(s: &str) -> Result<u16, String> {
    maybe_hex_range(s, 0, 0xFFF)
}

fn u4_parse(s: &str) -> Result<u4, String> {
    maybe_hex_range(s, 0, 0xF).map(u4::new)
}
