mod font;

pub const DISPLAY_X: usize = 64;
pub const DISPLAY_Y: usize = 32;

pub const CPU_HZ: u16 = 700;
pub const TIMER_HZ: u16 = 60;

pub const CPU_TIME_STEP: f32 = 1.0 / CPU_HZ as f32;
pub const TIMER_TIME_STEP: f32 = 1.0 / TIMER_HZ as f32;

pub type Display = [[bool; DISPLAY_X]; DISPLAY_Y];

const FONT_START_ADDRESS: usize = 0x50;
const ROM_START_ADDRESS: usize = 0x200;

pub struct Chip8 {
    pub memory: [u8; 4096],
    pub display: Display,

    pub pc: u16,
    pub i: u16,
    pub v: [u8; 16],
    pub stack: Vec<u16>,

    pub delay_timer: u8,
    pub sound_timer: u8,

    pub keypad: [bool; 16],
}

impl Default for Chip8 {
    fn default() -> Self {
        Self::new()
    }
}

impl Chip8 {
    pub fn new() -> Self {
        Chip8 {
            memory: [0; 4096],
            display: [[false; DISPLAY_X]; DISPLAY_Y],
            pc: ROM_START_ADDRESS as u16,
            i: 0,
            v: [0; 16],
            stack: Vec::new(),
            delay_timer: 0,
            sound_timer: 0,
            keypad: [false; 16],
        }
    }

    pub fn load_rom(&mut self, rom: &[u8]) {
        let font_end = FONT_START_ADDRESS + font::FONT.len();
        self.memory[FONT_START_ADDRESS..font_end].copy_from_slice(&font::FONT);

        let rom_end = ROM_START_ADDRESS + rom.len();
        self.memory[ROM_START_ADDRESS..rom_end].copy_from_slice(rom);

        self.pc = ROM_START_ADDRESS as u16;
    }

    // Should be about 700Hz
    pub fn cpu_cycle(&mut self) {
        let opcode = self.fetch();
        let decoded_opcode = self.decode(opcode);
        self.execute(decoded_opcode);
    }

    // Should be 60Hz
    pub fn timers_cycle(&mut self) {
        if self.delay_timer > 0 {
            self.delay_timer -= 1;
        }

        if self.sound_timer > 0 {
            self.sound_timer -= 1;
        }
    }

    pub fn should_beep(&self) -> bool {
        self.sound_timer > 0
    }

    fn fetch(&mut self) -> u16 {
        let opcode_slice = &self.memory[self.pc as usize..(self.pc + 2) as usize];
        self.pc += 2;

        u16::from_be_bytes(opcode_slice.try_into().unwrap())
    }

    fn decode(&self, opcode: u16) -> Opcode {
        let nibble = (
            ((opcode & 0xF000) >> 12) as u8,
            ((opcode & 0x0F00) >> 8) as u8,
            ((opcode & 0x00F0) >> 4) as u8,
            (opcode & 0x000F) as u8,
        );

        let x = nibble.1;
        let y = nibble.2;
        let n = nibble.3;
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
                op: match n {
                    0x0 => OpcodeALU::Set,
                    0x1 => OpcodeALU::Or,
                    0x2 => OpcodeALU::And,
                    0x3 => OpcodeALU::Xor,
                    0x4 => OpcodeALU::Add,
                    0x5 => OpcodeALU::Sub,
                    0x6 => OpcodeALU::ShiftRight,
                    0x7 => OpcodeALU::SubReverse,
                    0xE => OpcodeALU::ShiftLeft,
                    _ => OpcodeALU::Unknown,
                },
            },
            (0x9, _, _, 0x0) => Opcode::SkipRegNotEqualReg { x, y },
            (0xA, _, _, _) => Opcode::SetIndexImm { nnn },
            (0xB, _, _, _) => Opcode::JumpWithOffset { nnn },
            (0xC, _, _, _) => Opcode::Random { x, nn },
            (0xD, _, _, _) => Opcode::Draw { x, y, n },
            (0xE, _, 0x9, 0xE) => Opcode::SkipIfPressed { x },
            (0xE, _, 0xA, 0x1) => Opcode::SkipIfNotPressed { x },
            (0xF, _, 0x0, 0xA) => Opcode::GetKey { x },
            (0xF, _, 0x0, 0x7) => Opcode::ReadDelayTimer { x },
            (0xF, _, 0x1, 0x5) => Opcode::SetDelayTimer { x },
            (0xF, _, 0x1, 0x8) => Opcode::SetSoundTimer { x },
            (0xF, _, 0x1, 0xE) => Opcode::AddIndexReg { x },
            (0xF, _, 0x2, 0x9) => Opcode::FontChar { x },
            (0xF, _, 0x3, 0x3) => Opcode::BCD { x },
            (0xF, _, 0x5, 0x5) => Opcode::StoreRegs { x },
            (0xF, _, 0x6, 0x5) => Opcode::LoadRegs { x },

            _ => Opcode::Unknown,
        }
    }

    fn execute(&mut self, opcode: Opcode) {
        match opcode {
            Opcode::ClearDisplay => {
                self.display = [[false; DISPLAY_X]; DISPLAY_Y];
            }
            Opcode::Jump { nnn } => {
                self.pc = nnn;
            }
            Opcode::JumpWithOffset { nnn } => {
                self.pc = nnn + self.v[0] as u16;
            }
            Opcode::Call { nnn } => {
                self.stack.push(self.pc);
                self.pc = nnn;
            }
            Opcode::Return => {
                if let Some(address) = self.stack.pop() {
                    self.pc = address;
                }
            }
            Opcode::SkipRegEqualImm { x, nn } => {
                if self.v[x as usize] == nn {
                    self.pc += 2;
                }
            }
            Opcode::SkipRegNotEqualImm { x, nn } => {
                if self.v[x as usize] != nn {
                    self.pc += 2;
                }
            }
            Opcode::SkipRegEqualReg { x, y } => {
                if self.v[x as usize] == self.v[y as usize] {
                    self.pc += 2;
                }
            }
            Opcode::SkipRegNotEqualReg { x, y } => {
                if self.v[x as usize] != self.v[y as usize] {
                    self.pc += 2;
                }
            }
            Opcode::SetRegImm { x, nn } => {
                self.v[x as usize] = nn;
            }
            Opcode::AddRegImm { x, nn } => {
                self.v[x as usize] = self.v[x as usize].wrapping_add(nn);
            }
            Opcode::ALU { x, y, op } => {
                self.execute_alu(x, y, op);
            }
            Opcode::Random { x, nn } => {
                let rand_byte: u8 = rand::random();
                self.v[x as usize] = rand_byte & nn;
            }
            Opcode::SetIndexImm { nnn } => {
                self.i = nnn;
            }
            Opcode::AddIndexReg { x } => {
                self.i = self.i.wrapping_add(self.v[x as usize] as u16);
            }
            Opcode::Draw { x, y, n } => {
                self.execute_draw(x, y, n);
            }
            Opcode::SkipIfPressed { x } => {
                if self.keypad[self.v[x as usize] as usize] {
                    self.pc += 2;
                }
            }
            Opcode::SkipIfNotPressed { x } => {
                if !self.keypad[self.v[x as usize] as usize] {
                    self.pc += 2;
                }
            }
            Opcode::GetKey { x } => {
                for (key, &pressed) in self.keypad.iter().enumerate() {
                    if pressed {
                        self.v[x as usize] = key as u8;
                        return;
                    }
                }

                // If no key is pressed, loop on this instruction
                self.pc -= 2;
            }
            Opcode::ReadDelayTimer { x } => {
                self.v[x as usize] = self.delay_timer;
            }
            Opcode::SetDelayTimer { x } => {
                self.delay_timer = self.v[x as usize];
            }
            Opcode::SetSoundTimer { x } => {
                self.sound_timer = self.v[x as usize];
            }
            Opcode::FontChar { x } => {
                let digit = self.v[x as usize] & 0x0F;
                self.i = FONT_START_ADDRESS as u16 + digit as u16 * 5;
            }
            Opcode::BCD { x } => {
                let value = self.v[x as usize];
                self.memory[self.i as usize] = value / 100;
                self.memory[self.i as usize + 1] = (value / 10) % 10;
                self.memory[self.i as usize + 2] = value % 10;
            }
            Opcode::StoreRegs { x } => {
                for reg_index in 0..=x as usize {
                    self.memory[self.i as usize + reg_index] = self.v[reg_index];
                }
            }
            Opcode::LoadRegs { x } => {
                for reg_index in 0..=x as usize {
                    self.v[reg_index] = self.memory[self.i as usize + reg_index];
                }
            }
            Opcode::Unknown => {
                // Add debug print or sth here
            }
        }
    }

    fn execute_alu(&mut self, x: u8, y: u8, op: OpcodeALU) {
        match op {
            OpcodeALU::Set => self.v[x as usize] = self.v[y as usize],
            OpcodeALU::Or => {
                self.v[x as usize] |= self.v[y as usize];
                self.v[0xF] = 0;
            }
            OpcodeALU::And => {
                self.v[x as usize] &= self.v[y as usize];
                self.v[0xF] = 0;
            }
            OpcodeALU::Xor => {
                self.v[x as usize] ^= self.v[y as usize];
                self.v[0xF] = 0;
            }
            OpcodeALU::Add => {
                let (res, overflow) = self.v[x as usize].overflowing_add(self.v[y as usize]);
                self.v[x as usize] = res;
                self.v[0xF] = if overflow { 1 } else { 0 };
            }
            OpcodeALU::Sub => {
                let (res, borrow) = self.v[x as usize].overflowing_sub(self.v[y as usize]);
                self.v[x as usize] = res;
                self.v[0xF] = if borrow { 0 } else { 1 }; // Notice that borrow is inverted
            }
            OpcodeALU::SubReverse => {
                let (res, borrow) = self.v[y as usize].overflowing_sub(self.v[x as usize]);
                self.v[x as usize] = res;
                self.v[0xF] = if borrow { 0 } else { 1 };
            }
            OpcodeALU::ShiftRight => {
                let lsb = self.v[y as usize] & 1;
                self.v[x as usize] = self.v[y as usize] >> 1;
                self.v[0xF] = lsb;
            }
            OpcodeALU::ShiftLeft => {
                let msb = (self.v[y as usize] >> 7) & 1;
                self.v[x as usize] = self.v[y as usize] << 1;
                self.v[0xF] = msb;
            }
            OpcodeALU::Unknown => {
                // Add debug print or sth here
            }
        }
    }

    fn execute_draw(&mut self, x: u8, y: u8, n: u8) {
        let x_pos = self.v[x as usize] as usize % DISPLAY_X;
        let y_pos = self.v[y as usize] as usize % DISPLAY_Y;

        // Don't draw out of bounds
        let row_count = std::cmp::min(n as usize, DISPLAY_Y - y_pos);
        let col_count = std::cmp::min(8, DISPLAY_X - x_pos);

        let mut any_erased = false;
        for row in 0..row_count {
            let sprite_byte = self.memory[self.i as usize + row];

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
    }
}

pub enum Opcode {
    Jump { nnn: u16 },
    JumpWithOffset { nnn: u16 },

    Call { nnn: u16 },
    Return,

    SkipRegEqualImm { x: u8, nn: u8 },
    SkipRegNotEqualImm { x: u8, nn: u8 },
    SkipRegEqualReg { x: u8, y: u8 },
    SkipRegNotEqualReg { x: u8, y: u8 },

    SetRegImm { x: u8, nn: u8 },
    AddRegImm { x: u8, nn: u8 },
    SetIndexImm { nnn: u16 },
    AddIndexReg { x: u8 },

    ALU { x: u8, y: u8, op: OpcodeALU },
    Random { x: u8, nn: u8 },

    ClearDisplay,
    Draw { x: u8, y: u8, n: u8 },

    SkipIfPressed { x: u8 },
    SkipIfNotPressed { x: u8 },
    GetKey { x: u8 },

    ReadDelayTimer { x: u8 },
    SetDelayTimer { x: u8 },
    SetSoundTimer { x: u8 },

    FontChar { x: u8 },
    BCD { x: u8 },

    StoreRegs { x: u8 },
    LoadRegs { x: u8 },

    Unknown,
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
    Unknown,
}
