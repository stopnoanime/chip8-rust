use std::ops::{Index, IndexMut};

#[derive(Clone, Copy)]
#[allow(non_camel_case_types)]
pub struct u4(u8);

impl u4 {
    pub const fn new(value: u8) -> Self {
        Self(value & 0x0F)
    }
}

impl From<u4> for usize {
    fn from(v: u4) -> usize {
        v.0 as usize
    }
}

impl Index<u4> for [u8; 16] {
    type Output = u8;

    fn index(&self, index: u4) -> &Self::Output {
        &self[index.0 as usize]
    }
}

impl IndexMut<u4> for [u8; 16] {
    fn index_mut(&mut self, index: u4) -> &mut Self::Output {
        &mut self[index.0 as usize]
    }
}
