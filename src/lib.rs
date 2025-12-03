mod font;
mod nibble;

use nibble::u4;

pub const DISPLAY_X: usize = 64;
pub const DISPLAY_Y: usize = 32;

pub const CPU_HZ: f32 = 700.0;
pub const TIMER_HZ: f32 = 60.0;

pub const CPU_TIME_STEP: f32 = 1.0 / CPU_HZ;
pub const TIMER_TIME_STEP: f32 = 1.0 / TIMER_HZ;

pub type Display = [[bool; DISPLAY_X]; DISPLAY_Y];
pub type IsKeyPressed = dyn Fn(u8) -> bool;

const FONT_START_ADDRESS: usize = 0x50;
const ROM_START_ADDRESS: usize = 0x200;
const MEMORY_SIZE: usize = 4096;

pub struct Chip8 {
    pub memory: [u8; MEMORY_SIZE],
    pub display: Display,

    pub pc: u16,
    pub i: u16,
    pub v: [u8; 16],
    pub stack: Vec<u16>,

    pub delay_timer: u8,
    pub sound_timer: u8,

    pub wait_release_key: Option<u8>,
    pub is_key_pressed: Box<IsKeyPressed>,
}

impl Chip8 {
    pub fn new(is_key_pressed: Box<IsKeyPressed>) -> Self {
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
            is_key_pressed,
        }
    }

    pub fn load_rom(&mut self, rom: &[u8]) -> Result<(), Chip8Error> {
        let font_end = FONT_START_ADDRESS + font::FONT.len();
        self.memory[FONT_START_ADDRESS..font_end].copy_from_slice(&font::FONT);

        let rom_end = ROM_START_ADDRESS + rom.len();
        if rom_end > MEMORY_SIZE {
            return Err(Chip8Error::RomLoadError {
                size: rom.len(),
                max_size: MEMORY_SIZE - ROM_START_ADDRESS,
            });
        }

        self.memory[ROM_START_ADDRESS..rom_end].copy_from_slice(rom);
        self.pc = ROM_START_ADDRESS as u16;

        Ok(())
    }

    pub fn cpu_cycle(&mut self) -> Result<Chip8Result, Chip8Error> {
        let opcode = self.fetch()?;
        let decoded_opcode = self.decode(opcode);
        self.execute(decoded_opcode)
    }

    pub fn timers_cycle(&mut self) {
        self.delay_timer = self.delay_timer.saturating_sub(1);
        self.sound_timer = self.sound_timer.saturating_sub(1);
    }

    pub fn should_beep(&self) -> bool {
        self.sound_timer > 0
    }

    pub fn fetch(&mut self) -> Result<u16, Chip8Error> {
        let high = *self.mem_get(self.pc)?;
        let low = *self.mem_get(self.pc.wrapping_add(1))?;

        Ok(u16::from_be_bytes([high, low]))
    }

    pub fn decode(&self, opcode: u16) -> Opcode {
        let nibble = (
            ((opcode & 0xF000) >> 12) as u8,
            ((opcode & 0x0F00) >> 8) as u8,
            ((opcode & 0x00F0) >> 4) as u8,
            (opcode & 0x000F) as u8,
        );

        let x = u4::new(nibble.1);
        let y = u4::new(nibble.2);
        let n = u4::new(nibble.3);
        let nn = (opcode & 0x00FF) as u8;
        let nnn = opcode & 0x0FFF;

        match (nibble.0, nibble.1, nibble.2, nibble.3) {
            (0x0, 0x0, 0xE, 0x0) => Opcode::ClearDisplay,
            (0x0, 0x0, 0xE, 0xE) => Opcode::Return,
            (0x1, _, _, _) => Opcode::Jump { nnn },
            (0x2, _, _, _) => Opcode::Call { nnn },
            (0x3, _, _, _) => Opcode::SkipRegEqualImm { x, nn },
            (0x4, _, _, _) => Opcode::SkipRegNotEqualImm { x, nn },
            (0x5, _, _, 0x0) => Opcode::SkipRegEqualReg { x, y },
            (0x6, _, _, _) => Opcode::SetRegImm { x, nn },
            (0x7, _, _, _) => Opcode::AddRegImm { x, nn },
            (0x8, _, _, _) => Opcode::ALU {
                x,
                y,
                op: match nibble.3 {
                    0x0 => OpcodeALU::Set,
                    0x1 => OpcodeALU::Or,
                    0x2 => OpcodeALU::And,
                    0x3 => OpcodeALU::Xor,
                    0x4 => OpcodeALU::Add,
                    0x5 => OpcodeALU::Sub,
                    0x6 => OpcodeALU::ShiftRight,
                    0x7 => OpcodeALU::SubReverse,
                    0xE => OpcodeALU::ShiftLeft,
                    _ => return Opcode::Unknown(opcode),
                },
            },
            (0x9, _, _, 0x0) => Opcode::SkipRegNotEqualReg { x, y },
            (0xA, _, _, _) => Opcode::SetIndexImm { nnn },
            (0xB, _, _, _) => Opcode::JumpWithOffset { nnn },
            (0xC, _, _, _) => Opcode::Random { x, nn },
            (0xD, _, _, _) => Opcode::Draw { x, y, n },
            (0xE, _, 0x9, 0xE) => Opcode::SkipIfPressed { x },
            (0xE, _, 0xA, 0x1) => Opcode::SkipIfNotPressed { x },
            (0xF, _, 0x0, 0xA) => Opcode::WaitForKey { x },
            (0xF, _, 0x0, 0x7) => Opcode::ReadDelayTimer { x },
            (0xF, _, 0x1, 0x5) => Opcode::SetDelayTimer { x },
            (0xF, _, 0x1, 0x8) => Opcode::SetSoundTimer { x },
            (0xF, _, 0x1, 0xE) => Opcode::AddIndexReg { x },
            (0xF, _, 0x2, 0x9) => Opcode::FontChar { x },
            (0xF, _, 0x3, 0x3) => Opcode::BCD { x },
            (0xF, _, 0x5, 0x5) => Opcode::StoreRegs { x },
            (0xF, _, 0x6, 0x5) => Opcode::LoadRegs { x },

            _ => Opcode::Unknown(opcode),
        }
    }

    pub fn execute(&mut self, opcode: Opcode) -> Result<Chip8Result, Chip8Error> {
        self.pc = self.pc.wrapping_add(2);

        match opcode {
            Opcode::ClearDisplay => {
                self.display = [[false; DISPLAY_X]; DISPLAY_Y];
            }
            Opcode::Jump { nnn } => {
                self.pc = nnn;
            }
            Opcode::JumpWithOffset { nnn } => {
                self.pc = nnn.wrapping_add(self.v[0].into());
            }
            Opcode::Call { nnn } => {
                self.stack.push(self.pc);
                self.pc = nnn;
            }
            Opcode::Return => {
                self.pc = self.stack.pop().ok_or(Chip8Error::StackUnderflow)?;
            }
            Opcode::SkipRegEqualImm { x, nn } => {
                if self.v[x] == nn {
                    self.pc = self.pc.wrapping_add(2);
                }
            }
            Opcode::SkipRegNotEqualImm { x, nn } => {
                if self.v[x] != nn {
                    self.pc = self.pc.wrapping_add(2);
                }
            }
            Opcode::SkipRegEqualReg { x, y } => {
                if self.v[x] == self.v[y] {
                    self.pc = self.pc.wrapping_add(2);
                }
            }
            Opcode::SkipRegNotEqualReg { x, y } => {
                if self.v[x] != self.v[y] {
                    self.pc = self.pc.wrapping_add(2);
                }
            }
            Opcode::SetRegImm { x, nn } => {
                self.v[x] = nn;
            }
            Opcode::AddRegImm { x, nn } => {
                self.v[x] = self.v[x].wrapping_add(nn);
            }
            Opcode::ALU { x, y, op } => {
                self.execute_alu(x, y, op);
            }
            Opcode::Random { x, nn } => {
                let rand_byte: u8 = rand::random();
                self.v[x] = rand_byte & nn;
            }
            Opcode::SetIndexImm { nnn } => {
                self.i = nnn;
            }
            Opcode::AddIndexReg { x } => {
                self.i = self.i.wrapping_add(self.v[x].into());
            }
            Opcode::Draw { x, y, n } => {
                return self.execute_draw(x, y, n);
            }
            Opcode::SkipIfPressed { x } => {
                if (self.is_key_pressed)(self.v[x]) {
                    self.pc = self.pc.wrapping_add(2);
                }
            }
            Opcode::SkipIfNotPressed { x } => {
                if !(self.is_key_pressed)(self.v[x]) {
                    self.pc = self.pc.wrapping_add(2);
                }
            }
            Opcode::WaitForKey { x } => {
                return Ok(self.execute_wait_for_key(x));
            }
            Opcode::ReadDelayTimer { x } => {
                self.v[x] = self.delay_timer;
            }
            Opcode::SetDelayTimer { x } => {
                self.delay_timer = self.v[x];
            }
            Opcode::SetSoundTimer { x } => {
                self.sound_timer = self.v[x];
            }
            Opcode::FontChar { x } => {
                let digit = self.v[x] & 0x0F;
                self.i = FONT_START_ADDRESS as u16 + digit as u16 * 5;
            }
            Opcode::BCD { x } => {
                let value = self.v[x];
                *self.mem_get(self.i)? = value / 100;
                *self.mem_get(self.i.wrapping_add(1))? = (value / 10) % 10;
                *self.mem_get(self.i.wrapping_add(2))? = value % 10;
            }
            Opcode::StoreRegs { x } => {
                for reg_index in 0..=usize::from(x) {
                    *self.mem_get(self.i)? = self.v[reg_index];
                    self.i = self.i.wrapping_add(1);
                }
            }
            Opcode::LoadRegs { x } => {
                for reg_index in 0..=usize::from(x) {
                    self.v[reg_index] = *self.mem_get(self.i)?;
                    self.i = self.i.wrapping_add(1);
                }
            }
            Opcode::Unknown(opcode) => {
                return Err(Chip8Error::UnknownOpcode { opcode });
            }
        };

        Ok(Chip8Result::Continue)
    }

    fn execute_alu(&mut self, x: u4, y: u4, op: OpcodeALU) {
        match op {
            OpcodeALU::Set => self.v[x] = self.v[y],
            OpcodeALU::Or => {
                self.v[x] |= self.v[y];
                self.v[0xF] = 0;
            }
            OpcodeALU::And => {
                self.v[x] &= self.v[y];
                self.v[0xF] = 0;
            }
            OpcodeALU::Xor => {
                self.v[x] ^= self.v[y];
                self.v[0xF] = 0;
            }
            OpcodeALU::Add => {
                let (res, overflow) = self.v[x].overflowing_add(self.v[y]);
                self.v[x] = res;
                self.v[0xF] = if overflow { 1 } else { 0 };
            }
            OpcodeALU::Sub => {
                let (res, borrow) = self.v[x].overflowing_sub(self.v[y]);
                self.v[x] = res;
                self.v[0xF] = if borrow { 0 } else { 1 }; // Notice that borrow is inverted
            }
            OpcodeALU::SubReverse => {
                let (res, borrow) = self.v[y].overflowing_sub(self.v[x]);
                self.v[x] = res;
                self.v[0xF] = if borrow { 0 } else { 1 };
            }
            OpcodeALU::ShiftRight => {
                let lsb = self.v[y] & 1;
                self.v[x] = self.v[y] >> 1;
                self.v[0xF] = lsb;
            }
            OpcodeALU::ShiftLeft => {
                let msb = (self.v[y] >> 7) & 1;
                self.v[x] = self.v[y] << 1;
                self.v[0xF] = msb;
            }
        }
    }

    fn execute_draw(&mut self, x: u4, y: u4, n: u4) -> Result<Chip8Result, Chip8Error> {
        let x_pos = self.v[x] as usize % DISPLAY_X;
        let y_pos = self.v[y] as usize % DISPLAY_Y;

        // Don't draw out of bounds
        let row_count = std::cmp::min(usize::from(n), DISPLAY_Y - y_pos);
        let col_count = std::cmp::min(8, DISPLAY_X - x_pos);

        let mut any_erased = false;
        for row in 0..row_count {
            let sprite_byte = *self.mem_get(self.i.wrapping_add(row as u16))?;

            for col in 0..col_count {
                // If current sprite bit is non-zero
                if (sprite_byte & (0x80 >> col)) != 0 {
                    let pixel = &mut self.display[y_pos + row][x_pos + col];

                    // Flip the pixel
                    *pixel ^= true;

                    if !*pixel {
                        any_erased = true;
                    }
                }
            }
        }

        self.v[0xF] = if any_erased { 1 } else { 0 };
        Ok(Chip8Result::NextFrame)
    }

    fn execute_wait_for_key(&mut self, x: u4) -> Chip8Result {
        if let Some(key) = self.wait_release_key
            && !(self.is_key_pressed)(key)
        {
            // The key we were waiting for has been released
            self.v[x] = key;
            self.wait_release_key = None;
            return Chip8Result::Continue;
        }

        if self.wait_release_key.is_none() {
            // Not waiting for a key release yet, check all keys
            for key in 0..16 {
                if (self.is_key_pressed)(key) {
                    self.wait_release_key = Some(key);
                    break;
                }
            }
        }

        // Repeat this instruction until a key is released
        self.pc = self.pc.wrapping_sub(2);
        Chip8Result::NextFrame
    }

    fn mem_get(&mut self, addr: u16) -> Result<&mut u8, Chip8Error> {
        self.memory
            .get_mut(addr as usize)
            .ok_or(Chip8Error::MemoryOutOfBounds { address: addr })
    }
}

pub enum Opcode {
    Jump { nnn: u16 },
    JumpWithOffset { nnn: u16 },

    Call { nnn: u16 },
    Return,

    SkipRegEqualImm { x: u4, nn: u8 },
    SkipRegNotEqualImm { x: u4, nn: u8 },
    SkipRegEqualReg { x: u4, y: u4 },
    SkipRegNotEqualReg { x: u4, y: u4 },

    SetRegImm { x: u4, nn: u8 },
    AddRegImm { x: u4, nn: u8 },
    SetIndexImm { nnn: u16 },
    AddIndexReg { x: u4 },

    ALU { x: u4, y: u4, op: OpcodeALU },
    Random { x: u4, nn: u8 },

    ClearDisplay,
    Draw { x: u4, y: u4, n: u4 },

    SkipIfPressed { x: u4 },
    SkipIfNotPressed { x: u4 },
    WaitForKey { x: u4 },

    ReadDelayTimer { x: u4 },
    SetDelayTimer { x: u4 },
    SetSoundTimer { x: u4 },

    FontChar { x: u4 },
    BCD { x: u4 },

    StoreRegs { x: u4 },
    LoadRegs { x: u4 },

    Unknown(u16),
}

pub enum OpcodeALU {
    Set,
    Or,
    And,
    Xor,
    Add,
    Sub,
    ShiftRight,
    SubReverse,
    ShiftLeft,
}

pub enum Chip8Result {
    Continue,
    NextFrame,
}

#[derive(Debug)]
pub enum Chip8Error {
    RomLoadError { size: usize, max_size: usize },
    MemoryOutOfBounds { address: u16 },
    StackUnderflow,
    UnknownOpcode { opcode: u16 },
}
