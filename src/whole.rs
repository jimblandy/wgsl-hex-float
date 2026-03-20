//! Definition of the [`Whole`] type.

use crate::shl_exact::shl_exact;

/// The whole-number part of a hexadecimal float literal, accumulated digit-by-digit.
///
/// This type represents the portion of a hexadecimal floating point literal
/// that precedes the fraction point. It represents the numeric value
/// `mantissa * 16.ipow(exponent_16)`.
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
/// `351797413507857089839497216`, since it is just 0x123 multiplied by 16²⁰.
/// This input would be represented like so:
///
/// ```
/// # use wgsl_hex_float::Whole;
/// assert_eq!(
///     Whole::from_str("12300000000000000000000"),
///     Ok(Whole {
///         mantissa: 0x123,
///         exponent_16: 20,
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
/// # use wgsl_hex_float::Whole;
/// assert_eq!(
///     Whole::from_str("123456789abcdef01"),
///     Ok(Whole {
///         mantissa: 0x123456789abcdef,
///         exponent_16: 2,
///         exact: false,
///     })
/// );
/// ```
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub struct Whole {
    /// The value of the most significant non-zero digits in the input.
    ///
    /// If the input contains only zero digits, this is zero. Otherwise,
    /// the low four bits are never zero. In other words, trailing zeros are
    /// always represented by `exponent_16`.
    ///
    /// If the input includes more non-zero digits than can be stored here, then
    /// `rounded` is `true`.
    pub mantissa: u64,

    /// The power of 16 by which `mantissa` should be multiplied.
    ///
    /// If the input contains only zero digits, this is zero.
    pub exponent_16: u32,

    /// True if `mantissa` and `exponent_16` accurately capture all
    /// digits that were present in the input.
    pub exact: bool,
}

impl Whole {
    pub fn zero() -> Whole {
        Whole {
            mantissa: 0,
            exponent_16: 0,
            exact: true,
        }
    }

    /// Add a hexadecimal digit whose numeric value is `digit` to the right end
    /// of `self`.
    pub fn push_hex_digit(&mut self, digit: u32) {
        assert!(digit < 16);

        let Whole {
            mantissa,
            exponent_16,
            exact: _,
        } = *self;

        // We can always accommodate a new trailing zero by adjusting the
        // exponent.
        if digit == 0 {
            if mantissa == 0 {
                return;
            }

            self.exponent_16 = exponent_16.checked_add(1).unwrap();
            return;
        }

        // Shift `mantissa` to make room for the new digit, making any prior
        // trailing zeros explicit, and checking for bit loss.
        let shift = exponent_16.checked_add(1).unwrap().checked_mul(4).unwrap();
        let Some(mantissa) = shl_exact(mantissa, shift) else {
            // The shift would lose bits, so we can't fit `digit` exactly.
            self.exact = false;
            self.exponent_16 = exponent_16.checked_add(1).unwrap();
            return;
        };

        // Now that we've applied the shift to `mantissa` directly, take it off
        // the books.
        self.exponent_16 = 0;

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
fn from_str() {
    assert_eq!(
        Whole::from_str("0"),
        Ok(Whole {
            mantissa: 0,
            exponent_16: 0,
            exact: true
        })
    );
    assert_eq!(
        Whole::from_str("00"),
        Ok(Whole {
            mantissa: 0,
            exponent_16: 0,
            exact: true
        })
    );
    assert_eq!(
        Whole::from_str("000"),
        Ok(Whole {
            mantissa: 0,
            exponent_16: 0,
            exact: true
        })
    );

    assert_eq!(
        Whole::from_str("1"),
        Ok(Whole {
            mantissa: 0x1,
            exponent_16: 0,
            exact: true
        })
    );
    assert_eq!(
        Whole::from_str("2f"),
        Ok(Whole {
            mantissa: 0x2f,
            exponent_16: 0,
            exact: true
        })
    );
    assert_eq!(
        Whole::from_str("38e"),
        Ok(Whole {
            mantissa: 0x38e,
            exponent_16: 0,
            exact: true
        })
    );

    assert_eq!(
        Whole::from_str("10"),
        Ok(Whole {
            mantissa: 0x1,
            exponent_16: 1,
            exact: true
        })
    );
    assert_eq!(
        Whole::from_str("2f00"),
        Ok(Whole {
            mantissa: 0x2f,
            exponent_16: 2,
            exact: true
        })
    );
    assert_eq!(
        Whole::from_str("38e000"),
        Ok(Whole {
            mantissa: 0x38e,
            exponent_16: 3,
            exact: true
        })
    );

    assert_eq!(
        Whole::from_str("101"),
        Ok(Whole {
            mantissa: 0x101,
            exponent_16: 0,
            exact: true
        })
    );
    assert_eq!(
        Whole::from_str("200f"),
        Ok(Whole {
            mantissa: 0x200f,
            exponent_16: 0,
            exact: true
        })
    );
    assert_eq!(
        Whole::from_str("38000e"),
        Ok(Whole {
            mantissa: 0x38000e,
            exponent_16: 0,
            exact: true
        })
    );

    assert_eq!(
        Whole::from_str("123456789abcdef1"),
        Ok(Whole {
            mantissa: 0x123456789abcdef1,
            exponent_16: 0,
            exact: true
        })
    );
    assert_eq!(
        Whole::from_str("123456789abcdef12"),
        Ok(Whole {
            mantissa: 0x123456789abcdef1,
            exponent_16: 1,
            exact: false
        })
    );
    assert_eq!(
        Whole::from_str("1000000000000000"),
        Ok(Whole {
            mantissa: 0x1,
            exponent_16: 15,
            exact: true
        })
    );
    assert_eq!(
        Whole::from_str("10000000000000000"),
        Ok(Whole {
            mantissa: 0x1,
            exponent_16: 16,
            exact: true
        })
    );
    assert_eq!(
        Whole::from_str("10000000000000001"),
        Ok(Whole {
            mantissa: 0x1,
            exponent_16: 16,
            exact: false
        })
    );
    assert_eq!(
        Whole::from_str("100000000000000000000"),
        Ok(Whole {
            mantissa: 0x1,
            exponent_16: 20,
            exact: true
        })
    );
    assert_eq!(
        Whole::from_str("100000000000000000001"),
        Ok(Whole {
            mantissa: 0x1,
            exponent_16: 20,
            exact: false
        })
    );
}
