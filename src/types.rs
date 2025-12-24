/// Result type for CHIP-8 CPU cycle execution
pub enum Chip8Result {
    /// Continue executing instructions in the current frame.
    Continue,
    /// Wait for the next frame before continuing
    /// (e.g., after a draw instruction to limit the display update rate to the frame rate).
    WaitForNextFrame,
}

/// Error types that can occur during CHIP-8 emulation
#[derive(Debug, thiserror::Error)]
pub enum Chip8Error {
    #[error("ROM is too large ({size} bytes), max size is {max_size} bytes")]
    RomLoadError { size: usize, max_size: usize },

    #[error("Memory access out of bounds at address {address:#06X}")]
    MemoryOutOfBounds { address: u16 },

    #[error("Stack underflow: attempted to return from a subroutine with empty call stack")]
    StackUnderflow,

    #[error("Unknown opcode: {opcode:#06X}")]
    UnknownOpcode { opcode: u16 },

    #[error("Unknown ALU operation at opcode: {opcode:#06X}")]
    UnknownALUOpcode { opcode: u16 },
}

pub const DISPLAY_X: usize = 64;
pub const DISPLAY_Y: usize = 32;
/// A type alias for the CHIP-8 display buffer representation
pub type Display<T> = [[T; DISPLAY_X]; DISPLAY_Y];
