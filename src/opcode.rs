use crate::u4;

/// CHIP-8 instruction opcodes
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
    UnknownALU(u16),
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

impl Opcode {
    /// Decode a 16-bit raw opcode into an Opcode enum variant
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
