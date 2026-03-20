//! Definition of the [`Fraction`] type.

use crate::shl_exact::shl_exact;

/// The fractional part of a hexadecimal float literal, accumulated digit-by-digit.
///
/// This type represents the portion of a hexadecimal floating point literal
/// that follows the fraction point, along with the information needed to add
/// new digits at the end. It represents the numeric value
/// `mantissa / 16.ipow(exponent_16)`.
///
/// This type is not simply a `u64`, because we need to represent leading and
/// trailing zeros separately from the interesting part of the value, so that we
/// can accept input like this:
///
/// ```ignore
/// 0.0000000000000000000012300000000000000000000
/// ```
///
/// This is `123` preceded and followed by 20 `0` digits. This value can be
/// expressed as an exact `f32` value, since it is just 0x123 divided by
/// 16²³. This input would be represented like so:
///
/// ```
/// # use wgsl_hex_float::Fraction;
/// assert_eq!(
///     Fraction::from_str("0000000000000000000012300000000000000000000"),
///     Ok(Fraction {
///         mantissa: 0x123,
///         exponent_16: 23,
///         trailing_zeros: 20,
///         exact: true,
///     })
/// );
/// ```
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct Fraction {
    /// The value of the most significant non-zero digits in the input.
    ///
    /// If the input contains only zero digits, this is zero. Otherwise, the low
    /// four bits are never zero.
    pub mantissa: u64,

    /// The power of 16 by which `mantissa` should be divided.
    ///
    /// If the input contains only zero digits, this is zero.
    pub exponent_16: u32,

    /// The number of trailing zeros not included in `mantissa` or `exponent_16`.
    ///
    /// As trailing zeros are pushed, we increase this without affecting the
    /// rest of the value.
    ///
    /// This is zero when no digits have been pushed.
    pub trailing_zeros: u32,

    /// True if `mantissa` and `exponent_16` accurately capture all
    /// digits that were present in the input.
    pub exact: bool,
}

impl Fraction {
    pub fn zero() -> Fraction {
        Fraction {
            mantissa: 0,
            exponent_16: 0,
            trailing_zeros: 0,
            exact: true,
        }
    }

    /// Add a hexadecimal digit whose numeric value is `digit` to the right end
    /// of `self`.
    pub fn push_hex_digit(&mut self, digit: u32) {
        assert!(digit < 16);

        // A new trailing zero affects neither the value nor the exactness.
        if digit == 0 {
            self.trailing_zeros = self.trailing_zeros.checked_add(1).unwrap();
            return;
        }

        // Shift `mantissa` to make room for the new digit, making any prior
        // trailing zeros explicit, and checking for bit loss.
        let shift_digits = self.trailing_zeros.checked_add(1).unwrap();
        let Some(mantissa) = shl_exact(self.mantissa, shift_digits.checked_mul(4).unwrap()) else {
            // The shift would lose bits, so we can't fit `digit` exactly.
            self.exact = false;
            // Count this non-zero digit as if we had added a trailing zero
            // instead. This isn't necessary, but it seems less confusing to at
            // least keep the place value of the next digit accurate.
            self.trailing_zeros = self.trailing_zeros.checked_add(1).unwrap();
            return;
        };

        // Now that we've shifted the trailing zeros into `mantissa`, take them
        // off the books.
        self.trailing_zeros = 0;

        // Adjust the exponent to balance out the shift applied to the mantissa,
        // so that all extant digits retain their original place value.
        self.exponent_16 = self.exponent_16.checked_add(shift_digits).unwrap();

        // Incorporate `digit` into the mantissa.
        self.mantissa = mantissa | digit as u64;
    }

    /// Push all hexadecimal digits at the start of `digits` onto `self`.
    ///
    /// Return the remaining portion of `digits`, which is either empty, or
    /// starts with a character that is not a hexadecimal digit.
    pub fn consume_hex_digits<'d>(&mut self, digits: &'d str) -> &'d str {
        let mut chars = digits.chars();
        while let Some(digit) = chars.next().and_then(|ch| ch.to_digit(16)) {
            self.push_hex_digit(digit);
        }

        chars.as_str()
    }

    /// Construct a [`Fraction`] value from the entire contents of `digits`.
    ///
    /// If `digits` does not consist entirely of hexadecimal digits, return an
    /// error.
    pub fn from_str(digits: &str) -> Result<Fraction, core::num::IntErrorKind> {
        let mut w = Fraction::zero();
        let rest = w.consume_hex_digits(digits);
        if !rest.is_empty() {
            // We were stopped by a character that is not a hex digit, not by
            // reaching the end of the string.
            return Err(core::num::IntErrorKind::InvalidDigit);
        }
        Ok(w)
    }
}

#[test]
fn from_str() {
    assert_eq!(Fraction::from_str(""), Ok(Fraction::zero()));
    assert_eq!(
        Fraction::from_str("0"),
        Ok(Fraction {
            mantissa: 0,
            exponent_16: 0,
            trailing_zeros: 1,
            exact: true,
        })
    );
    assert_eq!(
        Fraction::from_str("00"),
        Ok(Fraction {
            mantissa: 0,
            exponent_16: 0,
            trailing_zeros: 2,
            exact: true,
        })
    );
    assert_eq!(
        Fraction::from_str("0000000000000000000000000000000000000000"),
        Ok(Fraction {
            mantissa: 0,
            exponent_16: 0,
            trailing_zeros: 40,
            exact: true,
        })
    );
    assert_eq!(
        Fraction::from_str("00000000000000000000000000000000000000001"),
        Ok(Fraction {
            mantissa: 1,
            exponent_16: 41,
            trailing_zeros: 0,
            exact: true,
        })
    );
    assert_eq!(
        Fraction::from_str("0000000000000000f"),
        Ok(Fraction {
            mantissa: 0xf,
            exponent_16: 17,
            trailing_zeros: 0,
            exact: true,
        })
    );
    assert_eq!(
        Fraction::from_str("0000000000000000f000000000000000"),
        Ok(Fraction {
            mantissa: 0xf,
            exponent_16: 17,
            trailing_zeros: 15,
            exact: true,
        })
    );
    assert_eq!(
        Fraction::from_str("0000000000000000f000000000000001"),
        Ok(Fraction {
            mantissa: 0xf000000000000001,
            exponent_16: 32,
            trailing_zeros: 0,
            exact: true,
        })
    );
    assert_eq!(
        Fraction::from_str("0000000000000000f0000000000000001"),
        Ok(Fraction {
            mantissa: 0xf,
            exponent_16: 17,
            trailing_zeros: 16,
            exact: false,
        })
    );
    assert_eq!(
        Fraction::from_str("0000000000000000000012300000000000000000000"),
        Ok(Fraction {
            mantissa: 0x123,
            exponent_16: 23,
            trailing_zeros: 20,
            exact: true,
        })
    );
}
