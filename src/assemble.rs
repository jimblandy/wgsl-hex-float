/*! Building floating-point values from [`Parts`]. */

use crate::{BinaryFormat, Parts};

impl<S> Parts<S> {
    /// Convert `self` to a Rust floating-point value.
    ///
    /// If `self` can be represented exactly as a value `v` of the type `T`,
    /// return `Assembled::Exact(v)`.
    ///
    /// If `self` is in range, but cannot be represented exactly in `T`, return
    /// `Assembled::Rounded(v)`, where `v` is the value of `self` rounded
    /// towards zero.
    ///
    /// If `self` is too large to be represented in `T`, return
    /// `Assembled::Infinity(i)`, where `i` is the infinity of `T` with the same
    /// sign as `self`.
    pub fn to_float<T: BinaryFormat>(&self) -> Assembled<T> {
        let mut mantissa = self.mantissa;

        if mantissa == 0 {
            return Assembled::Exact(self.zero());
        }

        // How many interesting bits are there in `mantissa`?
        let mut significant_bits = Self::MANTISSA_BITS - mantissa.leading_zeros();

        // What exponent would `self` require as a normal float? In IEEE, the
        // binary point comes at the left of the mantissa's bits, not the right.
        //
        // Note that in normal numbers, the leading `1` bit of the mantissa
        // becomes implicit.
        let unbiased_exponent = self
            .explicit_exponent
            .saturating_add(self.exponent)
            .saturating_add(significant_bits as i32 - 1);
        if unbiased_exponent > T::MAX_NORMAL_EXP {
            return Assembled::Infinity(self.infinity());
        }
        if unbiased_exponent < T::MIN_NORMAL_EXP {
            // TODO: implement subnormals
            return Assembled::Rounded(self.zero());
        }

        let mut exact = self.exact;
        if significant_bits > T::MANTISSA_WIDTH + 1 {
            // We have too many bits to represent in the mantissa, even
            // including the implicit leading `1` bit. Drop bits off the bottom,
            // and return a rounded result.

            // TODO: this should round, not truncate
            exact = false;
            mantissa = mantissa >> (significant_bits - (T::MANTISSA_WIDTH + 1));
            significant_bits = T::MANTISSA_WIDTH + 1;
        }

        // Since the mantissa now fits in `MANTISSA_WIDTH + 1` bits, putting its
        // leading `1` bit in the right place always entails a left shift, not a
        // right shift.
        mantissa = mantissa << T::MANTISSA_WIDTH + 1 - significant_bits;

        // Mask off the leading `1` bit, that will be implicit in `T`.
        debug_assert!(mantissa & (1 << T::MANTISSA_WIDTH) != 0);
        mantissa &= !(1 << T::MANTISSA_WIDTH);

        let biased_exponent = unbiased_exponent + T::EXPONENT_BIAS;
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
        if self.sign == -1 {
            T::NEG_INFINITY
        } else {
            T::INFINITY
        }
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
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum Assembled<T> {
    /// The input can be represented exactly as the given value.
    Exact(T),

    /// The input value cannot be represented exactly in the given type, but can
    /// be represented with rounding towards zero as the given value.
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

#[cfg(test)]
fn make(sign: i32, mantissa: u64, exponent: i32) -> Parts<()> {
    assert!(mantissa == 0 || mantissa & 1 == 1);
    Parts {
        mantissa,
        exponent,
        present: crate::PartFlags::empty(),
        sign,
        last_digit_exponent: 0,
        explicit_exponent: 0,
        exact: true,
        suffix: None,
    }
}

#[test]
fn simple() {
    assert_eq!(make(1, 0, 0).to_float(), Assembled::Exact(0.0));
    assert_eq!(make(-1, 0, 0).to_float(), Assembled::Exact(-0.0));
    assert_eq!(make(1, 1, 0).to_float(), Assembled::Exact(1.0));
    assert_eq!(make(1, 1, 1).to_float(), Assembled::Exact(2.0));
    assert_eq!(make(1, 1, -1).to_float(), Assembled::Exact(0.5));

    assert_eq!(make(1, 1, 50).to_float(), Assembled::Exact(f32::powi(2.0, 50)));
    assert_eq!(make(1, 1, -50).to_float(), Assembled::Exact(f32::powi(2.0, -50)));

    assert_eq!(make(1, 7, -1).to_float(), Assembled::Exact(7.0 / 2.0));
    assert_eq!(make(1, 7, -1).to_float(), Assembled::Exact(7.0 / 2.0));
    assert_eq!(make(1, 7, -20).to_float(), Assembled::Exact(7.0 / f32::powi(2.0, 20)));
    assert_eq!(make(1, 7, 20).to_float(), Assembled::Exact(7.0 * f32::powi(2.0, 20)));

    assert_eq!(make(1, 7, -1).to_float::<f64>(), Assembled::Exact(7.0 / 2.0));
    assert_eq!(make(1, 7, -1).to_float::<f64>(), Assembled::Exact(7.0 / 2.0));
    assert_eq!(make(1, 7, -20).to_float::<f64>(), Assembled::Exact(7.0 / f64::powi(2.0, 20)));
    assert_eq!(make(1, 7, 20).to_float::<f64>(), Assembled::Exact(7.0 * f64::powi(2.0, 20)));
}

#[test]
fn extrema() {
    assert_eq!(f32::MAX_NORMAL_EXP, 127);
    assert_eq!(make(1, 1, 127).to_float::<f32>(), Assembled::Exact(f32::powi(2.0, 127)));
    assert_eq!(make(1, 3, 127).to_float::<f32>(), Assembled::Infinity(f32::INFINITY));
    assert_eq!(make(1, 1, 128).to_float::<f32>(), Assembled::Infinity(f32::INFINITY));
    assert_eq!(make(-1, 1, 128).to_float::<f32>(), Assembled::Infinity(f32::NEG_INFINITY));

    assert_eq!(f32::MIN_NORMAL_EXP, -126);
    assert_eq!(make(1, 1, -126).to_float::<f32>(), Assembled::Exact(f32::powi(2.0, -126)));
    // Note: this should become a subnormal
    assert_eq!(make(1, 1, -127).to_float::<f32>(), Assembled::Rounded(0.0));

    assert_eq!(f64::MAX_NORMAL_EXP, 1023);
    assert_eq!(make(1, 1, 1023).to_float::<f64>(), Assembled::Exact(f64::powi(2.0, 1023)));
    assert_eq!(make(1, 3, 1023).to_float::<f64>(), Assembled::Infinity(f64::INFINITY));
    assert_eq!(make(-1, 1, 1024).to_float::<f64>(), Assembled::Infinity(f64::NEG_INFINITY));

    assert_eq!(f64::MIN_NORMAL_EXP, -1022);
    assert_eq!(make(1, 1, -1022).to_float::<f64>(), Assembled::Exact(f64::powi(2.0, -1022)));
    // Note: this should become a subnormal
    assert_eq!(make(1, 1, -1023).to_float::<f64>(), Assembled::Rounded(0.0));
}

#[test]
fn too_many_bits() {
    // This is 23 bits. Should fit.
    assert_eq!(make(1, 0x7fff0f, 0).to_float::<f32>(), Assembled::Exact(0x7fff0f as f32));
    // This is 24 bits, but that's okay, too: there's an implicit `1` bit.
    assert_eq!(make(1, 0xffff0f, 0).to_float::<f32>(), Assembled::Exact(0xffff0f as f32));
    // This is 25 bits, and must be rounded.
    assert_eq!(make(1, 0x1ffff0f, 0).to_float::<f32>(), Assembled::Rounded(0x1ffff0e as f32));

    // 52 bits. Should fit.
    assert_eq!(make(1, 0xf_ffff_0000_ffff, 0).to_float::<f64>(),
               Assembled::Exact(0xf_ffff_0000_ffff_u64 as f64));
    // This is 53 bits, but that's okay: there's an implicit `1` bit.
    assert_eq!(make(1, 0x1f_ffff_0000_ffff, 0).to_float::<f64>(),
               Assembled::Exact(0x1f_ffff_0000_ffff_u64 as f64));
    // This is 54 bits, and must be rounded.
    assert_eq!(make(1, 0x3f_ffff_0000_ffff, 0).to_float::<f64>(),
               Assembled::Rounded(0x3f_ffff_0000_fffe_u64 as f64));
}
