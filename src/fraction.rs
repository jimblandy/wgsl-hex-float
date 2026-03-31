//! Definition of the [`Fraction`] type.

/// The fractional part of a hexadecimal float literal, accumulated digit-by-digit.
///
/// This type represents the portion of a hexadecimal floating point literal
/// that follows the hexadecimal point, along with the information needed to add
/// new digits at the end. It represents the numeric value `mantissa *
/// 2**exponent`, where `exponent` is generally negative. This value is always
/// in the half-open range `[0, 1)`.
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
/// 2⁹². This input would be represented like so:
///
/// ```
/// # use hex_float::Fraction;
/// assert_eq!(
///     Fraction::from_str("0000000000000000000012300000000000000000000"),
///     Ok(Fraction {
///         mantissa: 0x123,
///         exponent: (20 + 3) * -4,
///         last_digit_exponent: (20 + 3 + 20) * -4,
///         exact: true,
///     })
/// );
/// ```
///
/// Since `Fraction` represents the exponent as a power of two, it can
/// represent some 17-digit hex fractional values exactly:
///
/// ```
/// # use hex_float::Fraction;
/// assert_eq!(
///     Fraction::from_str("3000000000000000c"),
///     Ok(Fraction {
///         mantissa: 0xc000000000000003,
///         exponent: 16 * -4 - 2,
///         last_digit_exponent: (16 + 1) * -4,
///         exact: true,
///     })
/// );
/// ```
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct Fraction {
    /// The most significant bits of the value.
    ///
    /// If the input contains only zero digits, this is zero. Otherwise, the
    /// least significant bit is never zero.
    pub mantissa: u64,

    /// The power of two by which `mantissa` should be multiplied. Zero or
    /// negative. If `mantissa` is zero, this is also zero.
    pub exponent: i32,

    /// The exponent of the power of two by which the most recently pushed
    /// digit was occupied. Initially zero.
    pub last_digit_exponent: i32,

    /// True if `mantissa` and `exponent` accurately capture all
    /// digits that were present in the input.
    pub exact: bool,
}

impl Fraction {
    /// Return the `Fraction` value representing zero.
    pub fn zero() -> Fraction {
        Fraction {
            mantissa: 0,
            exponent: 0,
            last_digit_exponent: 0,
            exact: true,
        }
    }

    /// Return the `Fraction` value representing `mantissa * 2**exponent)`.
    ///
    /// The resulting value must be less than one. This means that `exponent` is
    /// zero or negative, and `mantissa` must be less than `2**(-exponent)`.
    /// 
    /// The next digit pushed will be placed just to the right of where
    /// `exponent` places `mantissa`. For this to make much sense as a string of
    /// hex digits, you should pass a multiple of four for `mantissa`.
    ///
    /// ```
    /// # use hex_float::Fraction;
    /// let mut f = Fraction::with_exponent(0x100, -20);
    /// f.push_hex_digit(0xf);
    /// assert_eq!(f, Fraction::with_exponent(0x100f, -24));
    /// ```
    pub fn with_exponent(mantissa: u64, exponent: i32) -> Fraction {
        assert!(mantissa == 0 || exponent < 0);

        if mantissa == 0 {
            return Fraction {
                mantissa: 0,
                exponent: 0,
                last_digit_exponent: exponent,
                exact: true,
            }
        }

        let trailing_zeros = mantissa.trailing_zeros() as i32;

        Fraction {
            mantissa: mantissa >> trailing_zeros,
            exponent: exponent + trailing_zeros,
            last_digit_exponent: exponent,
            exact: true,
        }
    }

    /// Add a hexadecimal digit whose numeric value is `digit` to the right end
    /// of `self`.
    pub fn push_hex_digit(&mut self, mut digit: u32) {
        assert!(digit < 16);

        let Fraction {
            mantissa,
            exponent,
            ..
        } = *self;

        // Advance `last_digit_exponent` to this digit's exponent.
        self.last_digit_exponent = self.last_digit_exponent.checked_sub(4).unwrap();

        if digit == 0 {
            return;
        }

        let trailing_zeros = digit.trailing_zeros();

        // When the mantissa is zero, there's no need to shift it, so we can
        // always incorporate the new digit without a loss of precision.
        if mantissa == 0 {
            self.mantissa = digit as u64 >> trailing_zeros;
            self.exponent = self.last_digit_exponent + trailing_zeros as i32;
            return;
        }

        // How much space do we have in the mantissa for more precision?
        let max_shift = self.mantissa.leading_zeros();
        
        // The new digit always lands to the right of where the mantissa is
        // landing now. Compute the distance between those locations, which is
        // always positive.
        assert!(exponent >= self.last_digit_exponent);
        let gap = (exponent - self.last_digit_exponent) as u32;

        // How many bits would we need to shift in to `mantissa` to represent
        // the new digit exactly? Trailing zeros in the new digit must not go in
        // `mantissa`.
        //
        // Note that this assumes `digit` is non-zero, which we ensured above.
        let exact_shift = gap - trailing_zeros;

        // Handle the common case where we have room to incorporate `digit` exactly.
        if exact_shift <= max_shift {
            self.mantissa = mantissa << exact_shift | (digit >> trailing_zeros) as u64;
            self.exponent = exponent.checked_sub(exact_shift as i32).unwrap();
            return;
        }

        // Pity.
        self.exact = false;

        // Adjust the value of the digit as necessary.
        let bits_to_drop = exact_shift - max_shift;
        if bits_to_drop < 4 {
            digit = digit & !((1 << bits_to_drop) - 1);
        } else {
            digit = 0;
        }

        // The digit may have become zero, so we must repeat the earlier check.
        if digit == 0 {
            return;
        }
        
        // We need to recompute the shift, as losing the bottom bits may have
        // exposed more trailing zeros (rounding 9 to 8, say). But this time the
        // required shift always ought to fit --- otherwise we would have zeroed
        // out `digit` entirely above, and returned early.
        let trailing_zeros = digit.trailing_zeros();
        let shift = gap - trailing_zeros;
        debug_assert!(shift <= max_shift);

        self.mantissa = mantissa << shift | (digit >> trailing_zeros) as u64;
        self.exponent = exponent.checked_sub(exact_shift as i32).unwrap();
    }

    /// Push all hexadecimal digits at the start of `digits` onto `self`.
    ///
    /// Return the remaining portion of `digits`, which is either empty, or
    /// starts with a character that is not a hexadecimal digit.
    pub fn consume_hex_digits<'d>(&mut self, mut digits: &'d str) -> &'d str {
        let mut chars = digits.chars();
        while let Some(digit) = chars.next().and_then(|ch| ch.to_digit(16)) {
            digits = chars.as_str();
            self.push_hex_digit(digit);
        }

        digits
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
fn consume_hex_digits() {
    let mut f = Fraction::zero();
    assert_eq!(
        f.consume_hex_digits("fp+2"),
        "p+2"
    );
    assert_eq!(
        f,
        Fraction {
            mantissa: 0xf,
            exponent: -4,
            last_digit_exponent: -4,
            exact: true,
        }
    );
}

#[test]
fn from_str() {
    assert_eq!(Fraction::from_str(""), Ok(Fraction::zero()));
    assert_eq!(
        Fraction::from_str("0"),
        Ok(Fraction {
            mantissa: 0,
            exponent: 0,
            last_digit_exponent: -4,
            exact: true,
        })
    );
    assert_eq!(
        Fraction::from_str("00"),
        Ok(Fraction {
            mantissa: 0,
            exponent: 0,
            last_digit_exponent: -8,
            exact: true,
        })
    );
    assert_eq!(
        Fraction::from_str("0000000000000000000000000000000000000000"),
        Ok(Fraction {
            mantissa: 0,
            exponent: 0,
            last_digit_exponent: 40 * -4,
            exact: true,
        })
    );
    assert_eq!(
        Fraction::from_str("00000000000000000000000000000000000000001"),
        Ok(Fraction {
            mantissa: 1,
            exponent: 41 * -4,
            last_digit_exponent: 41 * -4,
            exact: true,
        })
    );
    assert_eq!(
        Fraction::from_str("0000000000000000f"),
        Ok(Fraction {
            mantissa: 0xf,
            exponent: (16 + 1) * -4, 
            last_digit_exponent: (16 + 1) * -4, 
            exact: true,
        })
    );
    assert_eq!(
        Fraction::from_str("0000000000000000f000000000000000"),
        Ok(Fraction {
            mantissa: 0xf,
            exponent: (16 + 1) * -4,
            last_digit_exponent: (16 + 1 + 15) * -4,
            exact: true,
        })
    );
    assert_eq!(
        Fraction::from_str("0000000000000000f000000000000001"),
        Ok(Fraction {
            mantissa: 0xf000000000000001,
            exponent: (16 + 1 + 14 + 1) * -4,
            last_digit_exponent: (16 + 1 + 14 + 1) * -4,
            exact: true,
        })
    );
    assert_eq!(
        Fraction::from_str("0000000000000000f0000000000000001"),
        Ok(Fraction {
            mantissa: 0xf,
            exponent: (16 + 1) * -4,
            last_digit_exponent: (16 + 1 + 15 + 1) * -4,
            exact: false,
        })
    );
    assert_eq!(
        Fraction::from_str("0000000000000000000012300000000000000000000"),
        Ok(Fraction {
            mantissa: 0x123,
            exponent: (20 + 3) * -4,
            last_digit_exponent: (20 + 3 + 20) * -4,
            exact: true,
        })
    );
    assert_eq!(
        Fraction::from_str("3000000000000000c"),
        Ok(Fraction {
            mantissa: 0xc000000000000003,
            exponent: 16 * -4 - 2,
            last_digit_exponent: (16 + 1) * -4,
            exact: true,
        })
    );
}

#[test]
fn with_exponent() {
    assert_eq!(
        Fraction::with_exponent(0, 0),
        Fraction { mantissa: 0, exponent: 0, last_digit_exponent: 0, exact: true }
    );
    assert_eq!(
        Fraction::with_exponent(0, -20),
        Fraction { mantissa: 0, exponent: 0, last_digit_exponent: -20, exact: true }
    );
    assert_eq!(
        Fraction::with_exponent(0x1, -20),
        Fraction { mantissa: 0x1, exponent: -20, last_digit_exponent: -20, exact: true }
    );
    assert_eq!(
        Fraction::with_exponent(0x100, -20),
        Fraction { mantissa: 0x1, exponent: -12, last_digit_exponent: -20, exact: true }
    );
    assert_eq!(
        Fraction::with_exponent(0x1000000000000001, -64),
        Fraction { mantissa: 0x1000000000000001, exponent: -64, last_digit_exponent: -64, exact: true }
    );
    assert_eq!(
        Fraction::with_exponent(0x1000000010000000, -64),
        Fraction { mantissa: 0x100000001, exponent: -64 + 7 * 4, last_digit_exponent: -64, exact: true }
    );
}
