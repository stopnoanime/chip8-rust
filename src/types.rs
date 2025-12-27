use std::fmt;
use std::ops::{Index, IndexMut};

macro_rules! define_uint {
    (
        $(#[$meta:meta])*
        $name:ident,    // The name of the struct
        $repr:ty,       // The backing type
        $max:expr       // The maximum valid value
    ) => {
        $(#[$meta])*
        #[derive(Clone, Copy, Eq, Hash, PartialEq, PartialOrd, Ord)]
        #[allow(non_camel_case_types)]
        pub struct $name($repr);

        impl $name {
            /// Creates a new instance. Panics if the value exceeds the maximum.
            pub const fn new(value: $repr) -> Self {
                assert!(value <= $max, concat!(stringify!($name), " value out of range"));
                Self(value)
            }

            pub const fn wrapping_add(self, rhs: $repr) -> Self {
                Self((self.0.wrapping_add(rhs)) & $max)
            }

            pub const fn wrapping_sub(self, rhs: $repr) -> Self {
                Self((self.0.wrapping_sub(rhs)) & $max)
            }
        }

        impl From<$name> for usize {
            fn from(v: $name) -> usize {
                usize::from(v.0)
            }
        }

        impl<T> Index<$name> for [T; $max + 1] {
            type Output = T;

            fn index(&self, index: $name) -> &Self::Output {
                &self[usize::from(index)]
            }
        }

        impl<T> IndexMut<$name> for [T; $max + 1] {
            fn index_mut(&mut self, index: $name) -> &mut Self::Output {
                &mut self[usize::from(index)]
            }
        }

        impl fmt::Debug for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                fmt::UpperHex::fmt(&self.0, f)
            }
        }

        impl fmt::UpperHex for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                fmt::UpperHex::fmt(&self.0, f)
            }
        }
    };
}

define_uint!(
    /// A 4-bit unsigned integer.
    u4, u8, 0x0F
);

define_uint!(
    /// A 12-bit unsigned integer.
    u12, u16, 0x0FFF
);
