const START_ADDRESS: usize = 0x200;
pub const DISPLAY_X: usize = 64;
pub const DISPLAY_Y: usize = 32;

pub struct Chip8 {
    pub memory: [u8; 4096],
    pub display: [bool; DISPLAY_X * DISPLAY_Y],

    pub pc: u16,
    pub i: u16,
    pub v: [u8; 16],
    pub stack: Vec<u16>,

    pub delay_timer: u8,
    pub sound_timer: u8,
}

impl Chip8 {
    pub fn new() -> Self {
        Chip8 {
            memory: [0; 4096],
            display: [false; DISPLAY_X * DISPLAY_Y],
            pc: START_ADDRESS as u16,
            i: 0,
            v: [0; 16],
            stack: Vec::new(),
            delay_timer: 0,
            sound_timer: 0,
        }
    }

    pub fn load_rom(&mut self, rom: &[u8]) {
        let copy_size = std::cmp::min(rom.len(), self.memory.len() - START_ADDRESS);
        self.memory[START_ADDRESS..START_ADDRESS + copy_size].copy_from_slice(&rom[..copy_size]);
    }

    pub fn cycle(&mut self) {
        let opcode = self.fetch();
        let decoded_opcode = self.decode(opcode);
        self.execute(decoded_opcode);
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
            (0x1, _, _, _) => Opcode::Jump { nnn },
            (0x6, _, _, _) => Opcode::Set { x, nn },
            (0x7, _, _, _) => Opcode::Add { x, nn },
            (0xA, _, _, _) => Opcode::SetI { nnn },
            (0xD, _, _, _) => Opcode::Draw { x, y, n },

            _ => Opcode::Unknown,
        }
    }

    fn execute(&mut self, opcode: Opcode) {
        match opcode {
            Opcode::ClearDisplay => {
                self.display.fill(false);
            }
            Opcode::Jump { nnn } => {
                self.pc = nnn;
            }
            Opcode::Set { x, nn } => {
                self.v[x as usize] = nn;
            }
            Opcode::Add { x, nn } => {
                self.v[x as usize] = self.v[x as usize].wrapping_add(nn);
            }
            Opcode::SetI { nnn } => {
                self.i = nnn;
            }
            Opcode::Draw { x, y, n } => {
                self.execute_draw(x, y, n);
            }
            Opcode::Unknown => {
                // Add debug print or sth here
            }
        }
    }

    fn execute_draw(&mut self, x: u8, y: u8, n: u8) {
        let x_pos = self.v[x as usize] as usize % DISPLAY_X;
        let y_pos = self.v[y as usize] as usize % DISPLAY_Y;

        let row_count = std::cmp::min(n as usize, DISPLAY_Y - y_pos);
        let col_count = std::cmp::min(8, DISPLAY_X - x_pos);
        for row in 0..row_count {
            let sprite_byte = self.memory[self.i as usize + row];

            for col in 0..col_count {
                // If current sprite bit is non-zero
                if (sprite_byte & (0x80 >> col)) != 0 {
                    let index = (y_pos + row) * DISPLAY_X + (x_pos + col);

                    // Flip the pixel
                    self.display[index] ^= true;

                    // If any pixel was erased, set VF to 1
                    if !self.display[index] {
                        self.v[0xF] = 1;
                    }
                }
            }
        }
    }
}

pub enum Opcode {
    ClearDisplay,
    Jump { nnn: u16 },
    Set { x: u8, nn: u8 },
    Add { x: u8, nn: u8 },
    SetI { nnn: u16 },
    Draw { x: u8, y: u8, n: u8 },

    Unknown,
}
