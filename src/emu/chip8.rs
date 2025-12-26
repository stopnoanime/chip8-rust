use super::{
    Chip8Error, Chip8Result, DISPLAY_X, DISPLAY_Y, Display, FONT, FONT_END_ADDRESS,
    FONT_START_ADDRESS, Opcode,
};
use crate::u4;

// The constants are specified by the CHIP-8 specification
const ROM_START_ADDRESS: usize = 0x200;
pub(crate) const MEMORY_SIZE: usize = 4096;

/// CHIP-8 virtual machine state
pub struct Chip8 {
    /// 4KB memory array
    pub(crate) memory: [u8; MEMORY_SIZE],
    /// Display buffer: 64x32 monochrome pixels
    pub(crate) display: Display<bool>,

    /// Program counter: address of the next instruction to execute
    pub(crate) pc: u16,
    /// Index register: used for memory operations
    pub(crate) i: u16,
    /// General-purpose registers V0-VF (VF is used as a flag register)
    pub(crate) v: [u8; 16],
    /// Call stack for subroutine returns
    pub(crate) stack: Vec<u16>,

    /// Delay timer: decrements at 60Hz until it reaches 0
    pub(crate) delay_timer: u8,
    /// Sound timer: decrements at 60Hz, beeps while non-zero
    pub(crate) sound_timer: u8,

    /// Tracks which key is waiting to be released for the FX0A instruction
    pub(crate) wait_release_key: Option<u8>,
    /// Keypad state: 16 keys mapped as booleans (true = pressed)
    pub(crate) keypad: [bool; 16],
}

impl Chip8 {
    pub fn new() -> Self {
        Chip8 {
            memory: [0; MEMORY_SIZE],
            display: [[false; DISPLAY_X]; DISPLAY_Y],
            pc: ROM_START_ADDRESS as u16,
            i: 0,
            v: [0; 16],
            stack: Vec::new(),
            delay_timer: 0,
            sound_timer: 0,
            wait_release_key: None,
            keypad: [false; 16],
        }
    }

    /// Loads a ROM into memory and initializes the font set.
    pub fn load(&mut self, rom: &[u8]) -> Result<(), Chip8Error> {
        // Load font into memory
        self.memory[FONT_START_ADDRESS..FONT_END_ADDRESS].copy_from_slice(&FONT);

        // Load ROM into memory
        let rom_end = ROM_START_ADDRESS + rom.len();
        self.memory
            .get_mut(ROM_START_ADDRESS..rom_end)
            .ok_or(Chip8Error::RomLoadError {
                size: rom.len(),
                max_size: MEMORY_SIZE - ROM_START_ADDRESS,
            })?
            .copy_from_slice(rom);

        // Set program counter to start of ROM
        self.pc = ROM_START_ADDRESS as u16;

        Ok(())
    }

    /// Executes a single CPU cycle (fetch, decode, execute).
    pub fn cpu_cycle(&mut self) -> Result<Chip8Result, Chip8Error> {
        let opcode = self.fetch()?;
        let decoded_opcode = Opcode::decode(opcode);
        self.execute(decoded_opcode)
    }

    /// Updates the delay and sound timers. Should be called at 60Hz.
    pub fn timers_cycle(&mut self) {
        self.delay_timer = self.delay_timer.saturating_sub(1);
        self.sound_timer = self.sound_timer.saturating_sub(1);
    }

    /// Returns true if the sound timer is greater than zero, indicating a beep should be played.
    pub fn should_beep(&self) -> bool {
        self.sound_timer > 0
    }

    /// Set the state of a key on the keypad.
    pub fn set_key(&mut self, key: u4, pressed: bool) {
        self.keypad[key] = pressed;
    }

    /// Get the state of a pixel on the display (true = on, false = off).
    pub fn get_display_pixel(&self, y: usize, x: usize) -> bool {
        self.display[y][x]
    }

    /// Fetches the next 16-bit opcode from memory.
    fn fetch(&mut self) -> Result<u16, Chip8Error> {
        let high = *self.mem_get(self.pc)?;
        let low = *self.mem_get(self.pc.wrapping_add(1))?;

        Ok(u16::from_be_bytes([high, low]))
    }

    /// Helper to get a mutable reference to a memory location with bounds checking.
    pub(crate) fn mem_get(&mut self, addr: u16) -> Result<&mut u8, Chip8Error> {
        self.memory
            .get_mut(addr as usize)
            .ok_or(Chip8Error::MemoryOutOfBounds { address: addr })
    }
}

impl Default for Chip8 {
    fn default() -> Self {
        Self::new()
    }
}
