/*! Building floating-point values from [`Parts`]. */

use crate::{BinaryFormat, Parts};

impl<S> Parts<S> {
    pub fn to_float<T: BinaryFormat>(&self) -> Assembled<T> {
        let mut mantissa = self.mantissa;

        if mantissa == 0 {
            return Assembled::Exact(self.zero());
        }

        // How many interesting bits are there in `mantissa`?
        let mut significant_bits = Self::MANTISSA_BITS - mantissa.leading_zeros();

        // What exponent would this require as a normal number? In IEEE, the
        // binary point comes at the left of the mantissa's bits, not the right.
        //
        // Note that in normal numbers, the leading `1` bit of the mantissa
        // becomes implicit.
        let normal_exponent = self
            .explicit_exponent
            .saturating_add(self.exponent)
            .saturating_add(significant_bits as i32 - 1);
        if normal_exponent > T::MAX_NORMAL_EXP {
            return Assembled::Infinity(self.infinity());
        }
        if normal_exponent < T::MIN_NORMAL_EXP {
            // TODO: implement subnormals
            return Assembled::Rounded(self.zero());
        }

        let exact;
        if significant_bits > T::MANTISSA_WIDTH + 1 {
            // We have too many bits to represent in the mantissa, even
            // including the implicit leading `1` bit. Drop bits off the bottom,
            // and return a rounded result.
            exact = false;
            mantissa = mantissa >> (significant_bits - (T::MANTISSA_WIDTH + 1));
            significant_bits = T::MANTISSA_WIDTH + 1;
        } else {
            exact = true;
        }

        // Since the mantissa now fits in `MANTISSA_WIDTH + 1` bits, putting its
        // leading `1` bit in the right place always entails a left shift, not a
        // right shift.
        mantissa = mantissa << T::MANTISSA_WIDTH + 1 - significant_bits;

        // Mask off the implicit leading `1` bit.
        mantissa &= !(1 << T::MANTISSA_WIDTH + 1);

        let biased_exponent = normal_exponent + T::EXPONENT_BIAS;
        assert!(biased_exponent >= 1);
        let f = T::from_bitfields(
            self.sign_bit(),
            biased_exponent as u32,
            mantissa,
        );

        if exact {
            Assembled::Exact(f)
        } else {
            Assembled::Rounded(f)
        }
    }

    // Return the infinity of `T` with the same sign as `self`.
    fn infinity<T: BinaryFormat>(&self) -> T {
        T::infinity(self.sign == -1)
    }

    // Return the zero of `T` with the same sign as `self`.
    fn zero<T: BinaryFormat>(&self) -> T {
        T::from_bitfields(self.sign_bit(), 0, 0)
    }

    fn sign_bit(&self) -> u32 {
        if self.sign == -1 { 1 } else { 0 }
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
