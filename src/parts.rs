/*! Types representing parsed literals, ready to turn into a Rust value.

The main type of interest here is [`Parts`].

*/

use crate::{BinaryFormat, Fraction, Whole};

/// A parsed hexadecimal literal, ready to be converted into a Rust value.
///
/// This struct represents a hexadecimal numeric literal, either integral or
/// floating-point, that has been parsed and is ready to be converted into a
/// Rust value via the [`to_float`] and [`to_int`] methods.
///
/// The type parameter `S` gives the type of the language-specific suffix.
/// For parsing WGSL literals, this would be [`wgsl::Suffix`].
///
/// [`to_float`]: Self::to_float
/// [`to_int`]: Self::to_int
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Parts<S> {
    pub present: PartFlags,
    pub sign: i32,
    pub whole: Whole,
    pub fraction: Fraction,
    pub exponent: i32,
    pub suffix: Option<S>,
}

bitflags::bitflags! {
    /// The parts that a hexadecimal floating point literal might have.
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub struct PartFlags: u32 {
        /// An optional leading `'+'` or `'-'` character before the prefix, indicating the sign.
        const SIGN = 1 << 0;

        /// A `"0x"` or `"0X"` prefix for the rest of the number,
        /// indicating a hexadecimal literal.
        const PREFIX = 1 << 1;

        /// One or more digits of whole-number portion, preceding any hexadecimal point present.
        const WHOLE = 1 << 2;

        /// A hexadecimal point, introducing the fractional digits.
        const POINT = 1 << 3;

        /// One or more fractional digits, preceded by a hexadecimal point. 
        const FRACTION = 1 << 4;

        /// An exponent, introduced by a `'p'` or `'P'` character.
        const EXPONENT = 1 << 5;
    }
}

impl<S> Parts<S> {
    pub fn to_float<T: BinaryFormat>(&self) -> Result<Assembled<T>, Error> {
        todo!()
    }
}

/// The result of assembling a hexadecimal literal value.
pub enum Assembled<T> {
    /// The input can be represented exactly as the given value.
    Exact(T),

    /// The input value cannot be represented exactly in the given type,
    /// but can be represented with rounding as the given value.
    Rounded(T),

    /// The input overflowed, and is represented as the given infinity.
    Infinity(T),
}

impl<T> Assembled<T> {
    pub fn get(self) -> T {
        match self {
            Self::Exact(v) => v,
            Self::Rounded(v) => v,
            Self::Infinity(v) => v,
        }
    }
}

#[derive(Clone, Debug, thiserror::Error, Eq, PartialEq)]
pub enum Error {
}
