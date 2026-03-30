//! Definition of the [`Whole`] type.

/// The whole-number part of a hexadecimal float literal, accumulated digit-by-digit.
///
/// This type represents the portion of a hexadecimal floating point literal
/// that precedes the hexadecimal point. It represents the numeric value
/// `mantissa * 2.powi(exponent)`.
///
/// This type is not simply a `u64`, because we need to represent trailing zeros
/// separately from the interesting part of the value, so that we can accept
/// input like this:
///
/// ```ignore
/// 12300000000000000000000
/// ```
///
/// This is `123` followed by 20 `0` digits. This value does not fit in a `u64`,
/// but it can be expressed as an exact `f32` value,
/// `351797413507857089839497216`, since it is just 0x123 multiplied by 2⁸⁰.
/// This input would be represented like so:
///
/// ```
/// # use hex_float::Whole;
/// assert_eq!(
///     Whole::from_str("12300000000000000000000"),
///     Ok(Whole {
///         mantissa: 0x123,
///         exponent: 20 * 4,
///         exact: true,
///     })
/// );
/// ```
///
/// If more significant digits are supplied than can be held in `mantissa`, then
/// `exact` is set to `false`, and `mantissa` retains the most significant
/// digits. For example, `123456789abcdef01` is seventeen digits long, whereas a
/// `u64` can only hold sixteen hex digits, so this would be represented like so:
///
/// ```
/// # use hex_float::Whole;
/// assert_eq!(
///     Whole::from_str("123456789abcdef01"),
///     Ok(Whole {
///         mantissa: 0x123456789abcdef,
///         exponent: 2 * 4,
///         exact: false,
///     })
/// );
/// ```
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub struct Whole {
    /// The value of the most significant non-zero digits in the input.
    ///
    /// If the input contains only zero digits, this is zero.
    /// Otherwise, the low bit is never zero. In other words, trailing
    /// zero bits are always represented by incrementing `exponent`.
    ///
    /// If the input includes more non-zero bits than can be stored
    /// here, then `rounded` is `true`.
    pub mantissa: u64,

    /// The power of two by which `mantissa` should be multiplied.
    ///
    /// If the input contains only zero digits, this is zero.
    pub exponent: u32,

    /// True if `mantissa` and `exponent` accurately capture all
    /// digits that were present in the input.
    pub exact: bool,
}

impl Whole {
    pub const fn zero() -> Whole {
        Whole {
            mantissa: 0,
            exponent: 0,
            exact: true,
        }
    }

    pub fn new(value: u64) -> Whole {
        if value == 0 {
            return Whole::zero();
        }

        // How many trailing zero hex digits must we push into the exponent?
        let exponent = value.trailing_zeros();
        Whole {
            // Ensure the mantissa's bottom nibble is non-zero.
            mantissa: value >> exponent,
            exponent,
            exact: true,
        }
    }

    /// Add a hexadecimal digit whose numeric value is `digit` to the right end
    /// of `self`.
    pub fn push_hex_digit(&mut self, mut digit: u32) {
        assert!(digit < 16);

        let Whole {
            mantissa,
            exponent,
            exact: _,
        } = *self;

        // All trailing zero bits need to be covered by `exponent`; if it's
        // non-zero at all, the mantissa must always have its bottom bit set.
        debug_assert!((mantissa == 0 && exponent == 0) || mantissa & 1 == 1);

        // We can always accommodate a new trailing zero by adjusting the
        // exponent. The rest of the code is simpler if we can assume the
        // mantissa and the new digit are non-zero.
        if digit == 0 {
            if mantissa != 0 {
                self.exponent = exponent.checked_add(4).unwrap();
            }
            
            return;
        }

        // Incorporate the new digit into the mantissa. There are two wrinkles:
        //
        // - The new digit's bits must fall to the *right* of any trailing zeros
        //   currently represented in `exponent`. We'll need to shift those
        //   zeros into `mantissa`, and then drop the new digit's bits into
        //   place after them.
        //
        // - The `exponent` field covers *all* trailing zeros, such that the
        //   mantissa's bottom bit is always one (unless the whole thing is
        //   zero). So if the new digit itself has trailing zeros, those need to
        //   end up covered by the exponent, never the mantissa.
        //
        //   For example, if `exponent` is initially zero, then a new digit of
        //   `8` would only shift a single `1` bit onto the bottom of the
        //   mantissa, and set `exponent` to `3`. In doing so, we mustn't
        //   over-estimate how far we need to shift the mantissa, since that
        //   might cause us to think we have to drop bits when we actually
        //   don't.
        //
        // For example, if our current mantissa bits are `MMMMMMMMMMMMMM1`, the
        // exponent is 13, and the new digit's bits are `DD10`, the mantissa
        // we're aiming for looks like this:
        //
        //     0000MMMMMMMMMMMMMM10000000000000DD1
        //                                     --- Bits from the new digit,
        //                                         trailing zeros dropped
        //
        //                        -------------    13 trailing zeros previously
        //                                         held in the exponent
        //
        //         ---------------                 Previous mantissa bits
        //
        //     ----                                Remaining mantissa space
        //
        // And the new exponent would be 1, to cover the trailing zero in
        // `DD10`.

        // How much space do we have in the mantissa for more precision?
        let max_shift = mantissa.leading_zeros();

        // How many bits would we need to shift in to represent the new digit
        // exactly? Trailing zeros in the new digit don't need to (must not) go
        // in the mantissa.
        //
        // Note that this assumes `digit` is non-zero, which we ensured above.
        let trailing_zeros = digit.trailing_zeros();
        let exact_shift = exponent + (4 - trailing_zeros);

        // Handle the common case where we have room to incorporate `digit` exactly.
        if exact_shift <= max_shift {
            self.mantissa = mantissa << exact_shift | (digit >> trailing_zeros) as u64;
            self.exponent = trailing_zeros;
            return;
        }

        // Oh well.
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
            if mantissa != 0 {
                self.exponent = exponent.checked_add(4).unwrap();
            }

            return;
        }
        
        // We need to recompute the shift, as losing the bottom bits may have
        // exposed more trailing zeros (rounding 9 to 8, say). But this time the
        // required shift always ought to fit --- otherwise we would have zeroed
        // out `digit` entirely above, and returned early.
        let trailing_zeros = digit.trailing_zeros();
        let shift = exponent + (4 - trailing_zeros);
        debug_assert!(shift <= max_shift);

        self.mantissa = mantissa << shift | (digit >> trailing_zeros) as u64;
        self.exponent = trailing_zeros;
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

    /// Construct a [`Whole`] value from the entire contents of `digits`.
    ///
    /// If `digits` does not consist entirely of hexadecimal digits, return an
    /// error.
    pub fn from_str(digits: &str) -> Result<Whole, core::num::IntErrorKind> {
        let mut w = Whole::zero();
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
    let mut w = Whole::zero();
    assert_eq!(
        w.consume_hex_digits("a.fp+2"),
        ".fp+2"
    );
    assert_eq!(
        w,
        Whole {
            mantissa: 0x5,
            exponent: 1,
            exact: true,
        }
    );
}

#[test]
fn from_str() {
    assert_eq!(
        Whole::from_str("0"),
        Ok(Whole {
            mantissa: 0,
            exponent: 0,
            exact: true
        })
    );
    assert_eq!(
        Whole::from_str("00"),
        Ok(Whole {
            mantissa: 0,
            exponent: 0,
            exact: true
        })
    );
    assert_eq!(
        Whole::from_str("000"),
        Ok(Whole {
            mantissa: 0,
            exponent: 0,
            exact: true
        })
    );

    assert_eq!(
        Whole::from_str("1"),
        Ok(Whole {
            mantissa: 0x1,
            exponent: 0,
            exact: true
        })
    );
    assert_eq!(
        Whole::from_str("2"),
        Ok(Whole {
            mantissa: 0x1,
            exponent: 1,
            exact: true
        })
    );
    assert_eq!(
        Whole::from_str("2f"),
        Ok(Whole {
            mantissa: 0x2f,
            exponent: 0,
            exact: true
        })
    );
    assert_eq!(
        Whole::from_str("38e"),
        Ok(Whole {
            mantissa: 0x38e >> 1,
            exponent: 1,
            exact: true
        })
    );

    assert_eq!(
        Whole::from_str("10"),
        Ok(Whole {
            mantissa: 0x1,
            exponent: 4,
            exact: true
        })
    );
    assert_eq!(
        Whole::from_str("2f00"),
        Ok(Whole {
            mantissa: 0x2f,
            exponent: 8,
            exact: true
        })
    );
    assert_eq!(
        Whole::from_str("38e000"),
        Ok(Whole {
            mantissa: 0x38e >> 1,
            exponent: 13,
            exact: true
        })
    );

    assert_eq!(
        Whole::from_str("101"),
        Ok(Whole {
            mantissa: 0x101,
            exponent: 0,
            exact: true
        })
    );
    assert_eq!(
        Whole::from_str("200f"),
        Ok(Whole {
            mantissa: 0x200f,
            exponent: 0,
            exact: true
        })
    );
    assert_eq!(
        Whole::from_str("38000e"),
        Ok(Whole {
            mantissa: 0x38000e >> 1,
            exponent: 1,
            exact: true
        })
    );

    assert_eq!(
        Whole::from_str("123456789abcdef1"),
        Ok(Whole {
            mantissa: 0x123456789abcdef1,
            exponent: 0,
            exact: true
        })
    );
    assert_eq!(
        Whole::from_str("123456789abcdef11"),
        Ok(Whole {
            mantissa: 0x123456789abcdef1,
            exponent: 4,
            exact: false,
        })
    );
    assert_eq!(
        Whole::from_str("123456789abcdef12"),
        Ok(Whole {
            mantissa: 0x123456789abcdef1 << 3 | 2 >> 1,
            exponent: 1,
            exact: true
        })
    );
    assert_eq!(
        Whole::from_str("123456789abcdef13"),
        Ok(Whole {
            mantissa: 0x123456789abcdef1 << 3 | 3 >> 1,
            exponent: 1,
            exact: false,
        })
    );
    assert_eq!(
        Whole::from_str("123456789abcdef18"),
        Ok(Whole {
            mantissa: 0x123456789abcdef1 << 1 | 8 >> 3,
            exponent: 3,
            exact: true,
        })
    );
    assert_eq!(
        Whole::from_str("123456789abcdef19"),
        Ok(Whole {
            mantissa: 0x123456789abcdef1 << 1 | 9 >> 3,
            exponent: 3,
            exact: false,
        })
    );
    assert_eq!(
        Whole::from_str("1000000000000000"),
        Ok(Whole {
            mantissa: 0x1,
            exponent: 15 * 4,
            exact: true
        })
    );
    assert_eq!(
        Whole::from_str("10000000000000000"),
        Ok(Whole {
            mantissa: 0x1,
            exponent: 16 * 4,
            exact: true
        })
    );
    assert_eq!(
        Whole::from_str("10000000000000001"),
        Ok(Whole {
            mantissa: 0x1,
            exponent: 16 * 4,
            exact: false
        })
    );
    assert_eq!(
        Whole::from_str("100000000000000000000"),
        Ok(Whole {
            mantissa: 0x1,
            exponent: 20 * 4,
            exact: true
        })
    );
    assert_eq!(
        Whole::from_str("100000000000000000001"),
        Ok(Whole {
            mantissa: 0x1,
            exponent: 20 * 4,
            exact: false
        })
    );
    assert_eq!(
        Whole::from_str("100000000000000000008"),
        Ok(Whole {
            mantissa: 0x1,
            exponent: 20 * 4,
            exact: false
        })
    );
}
