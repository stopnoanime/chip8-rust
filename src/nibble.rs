use std::ops::{Index, IndexMut};

/// A 4-bit unsigned integer (nibble).
#[derive(Clone, Copy, Debug)]
#[allow(non_camel_case_types)]
pub struct u4(u8);

impl u4 {
    /// Creates a new `u4` from a `u8`.
    ///
    /// Panics if the value is greater than 0x0F.
    pub const fn new(value: u8) -> Self {
        assert!(value <= 0x0F, "u4 value must be in range 0x0-0xF");
        Self(value)
    }
}

impl From<u4> for usize {
    fn from(v: u4) -> usize {
        v.0 as usize
    }
}

impl<T> Index<u4> for [T; 16] {
    type Output = T;

    fn index(&self, index: u4) -> &Self::Output {
        &self[index.0 as usize]
    }
}

impl<T> IndexMut<u4> for [T; 16] {
    fn index_mut(&mut self, index: u4) -> &mut Self::Output {
        &mut self[index.0 as usize]
    }
}
