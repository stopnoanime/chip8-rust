use crate::{
    Chip8, Chip8Error, Chip8Result, DISPLAY_X, DISPLAY_Y, Opcode, OpcodeALU,
    font::FONT_START_ADDRESS, u4,
};

impl Chip8 {
    pub(crate) fn execute(&mut self, opcode: Opcode) -> Result<Chip8Result, Chip8Error> {
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
                if self.keypad[self.v[x] as usize & 0x0F] {
                    self.pc = self.pc.wrapping_add(2);
                }
            }
            Opcode::SkipIfNotPressed { x } => {
                if !self.keypad[self.v[x] as usize & 0x0F] {
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
        Ok(Chip8Result::WaitForNextFrame)
    }

    fn execute_wait_for_key(&mut self, x: u4) -> Chip8Result {
        if let Some(key) = self.wait_release_key
            && !self.keypad[key as usize]
        {
            // The key we were waiting for has been released
            self.v[x] = key;
            self.wait_release_key = None;
            return Chip8Result::Continue;
        }

        if self.wait_release_key.is_none() {
            // Not waiting for a key release yet, check all keys
            for key in 0..16 {
                if self.keypad[key as usize] {
                    self.wait_release_key = Some(key);
                    break;
                }
            }
        }

        // Repeat this instruction until a key is released
        self.pc = self.pc.wrapping_sub(2);
        Chip8Result::WaitForNextFrame
    }
}
