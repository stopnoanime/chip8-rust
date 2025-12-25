use crate::u4;

/// CHIP-8 instruction opcodes.
///
/// The fields (x, y, n, nn, nnn) correspond to the operands encoded in the opcode.
pub enum Opcode {
    /// 1nnn - Jump to location nnn.
    Jump { nnn: u16 },
    /// Bnnn - Jump to location nnn + V0.
    JumpWithOffset { nnn: u16 },

    /// 2nnn - Call subroutine at nnn.
    Call { nnn: u16 },
    /// 00EE - Return from a subroutine.
    Return,

    /// 3xnn - Skip next instruction if Vx == nn.
    SkipRegEqualImm { x: u4, nn: u8 },
    /// 4xnn - Skip next instruction if Vx != nn.
    SkipRegNotEqualImm { x: u4, nn: u8 },
    /// 5xy0 - Skip next instruction if Vx == Vy.
    SkipRegEqualReg { x: u4, y: u4 },
    /// 9xy0 - Skip next instruction if Vx != Vy.
    SkipRegNotEqualReg { x: u4, y: u4 },

    /// 6xnn - Set Vx = nn.
    SetRegImm { x: u4, nn: u8 },
    /// 7xnn - Set Vx = Vx + nn.
    AddRegImm { x: u4, nn: u8 },
    /// Annn - Set I = nnn.
    SetIndexImm { nnn: u16 },
    /// Fx1E - Set I = I + Vx.
    AddIndexReg { x: u4 },

    /// 8xyN - ALU operations
    ALU { x: u4, y: u4, op: OpcodeALU },
    /// Cxnn - Set Vx = random byte AND nn.
    Random { x: u4, nn: u8 },

    /// 00E0 - Clear the display.
    ClearDisplay,
    /// Dxyn - Display sprite.
    Draw { x: u4, y: u4, n: u4 },

    /// Ex9E - Skip next instruction if key with the value of Vx is pressed.
    SkipIfPressed { x: u4 },
    /// ExA1 - Skip next instruction if key with the value of Vx is not pressed.
    SkipIfNotPressed { x: u4 },
    /// Fx0A - Wait for a key press and release, store the value of the key in Vx.
    WaitForKey { x: u4 },

    /// Fx07 - Set Vx = delay timer value.
    ReadDelayTimer { x: u4 },
    /// Fx15 - Set delay timer = Vx.
    SetDelayTimer { x: u4 },
    /// Fx18 - Set sound timer = Vx.
    SetSoundTimer { x: u4 },

    /// Fx29 - Set I = location of sprite for digit Vx.
    FontChar { x: u4 },
    /// Fx33 - Store BCD representation of Vx in memory locations I, I+1, and I+2.
    BCD { x: u4 },

    /// Fx55 - Store registers V0 through Vx in memory starting at location I.
    StoreRegs { x: u4 },
    /// Fx65 - Read registers V0 through Vx from memory starting at location I.
    LoadRegs { x: u4 },

    /// Represents an unknown opcode.
    Unknown(u16),
    /// Represents an unknown ALU operation (8xyN where N is invalid).
    UnknownALU(u16),
}

/// ALU operations for the 8xyN instruction.
pub enum OpcodeALU {
    /// 8xy0 - Vx = Vy
    Set,
    /// 8xy1 - Vx = Vx OR Vy
    Or,
    /// 8xy2 - Vx = Vx AND Vy
    And,
    /// 8xy3 - Vx = Vx XOR Vy
    Xor,
    /// 8xy4 - Vx = Vx + Vy
    Add,
    /// 8xy5 - Vx = Vx - Vy
    Sub,
    /// 8xy6 - Vx = Vx SHR 1
    ShiftRight,
    /// 8xy7 - Vx = Vy - Vx
    SubReverse,
    /// 8xyE - Vx = Vx SHL 1
    ShiftLeft,
}

impl Opcode {
    /// Decode a 16-bit raw opcode into an `Opcode` enum variant.
    pub fn decode(opcode: u16) -> Self {
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
                    _ => return Opcode::UnknownALU(opcode),
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
}
