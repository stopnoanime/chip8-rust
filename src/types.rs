/// Result type for CHIP-8 CPU cycle execution
pub enum Chip8Result {
    /// Continue executing instructions
    Continue,
    /// Wait for the next frame before continuing (e.g., after a draw instruction)
    WaitForNextFrame,
}

/// Error types that can occur during CHIP-8 emulation
#[derive(Debug)]
pub enum Chip8Error {
    /// ROM is too large to fit in available memory
    RomLoadError { size: usize, max_size: usize },
    /// Attempted to access memory outside valid range
    MemoryOutOfBounds { address: u16 },
    /// Attempted to return from a subroutine with empty call stack
    StackUnderflow,
    /// Encountered an unknown/invalid opcode
    UnknownOpcode { opcode: u16 },
}

pub const DISPLAY_X: usize = 64;
pub const DISPLAY_Y: usize = 32;
pub type Display<T> = [[T; DISPLAY_X]; DISPLAY_Y];
