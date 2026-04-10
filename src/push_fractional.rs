//! Implementation of [`Parts::push_fractional_digit`] and friends.

use crate::{PartFlags, Parts};

impl<S> Parts<S> {
    /// Append a hexadecimal digit whose numeric value is `digit` to the
    /// fractional portion of `self`.
    ///
    /// For example:
    ///
    /// - If `self` currently represents `0x12`, then pushing `3` would
    ///   change it to represent `0x12.3`.
    ///
    /// - Then, pushing `4` would change it to represent `0x12.34`.
    ///
    /// This adjusts [`last_digit_exponent`], to track where the next digit
    /// should fall, and adds [`FRACTION`] to [`present`].
    ///
    /// [`last_digit_exponent`]: Self::last_digit_exponent
    /// [`FRACTION`]: PartFlags::FRACTION
    /// [`present`]: Self::present
    pub fn push_fractional_digit(&mut self, mut digit: u32) {
        assert!(digit < 16);
        assert!(self.last_digit_exponent <= 0);

        // Note that at least one fractional digit was present.
        self.present.insert(PartFlags::FRACTION);

        // Advance `last_digit_exponent` to this digit's exponent.
        self.last_digit_exponent = self.last_digit_exponent.checked_sub(4).unwrap();

        if digit == 0 {
            return;
        }

        let Parts {
            mantissa, exponent, ..
        } = *self;

        let trailing_zeros = digit.trailing_zeros();

        // If we don't need to round, then `digit`, will wind up at the right
        // end of `mantissa`, so `last_digit_exponent` tells us the new
        // `exponent`. But adjust for trailing zeros, which mustn't go into
        // `mantissa`.
        self.exponent = self.last_digit_exponent + trailing_zeros as i32;

        // When the mantissa is zero, we can always incorporate a new digit,
        // even if `last_digit_exponent` would imply a very large shift. Handle
        // this case early to avoid needless shift calculations.
        if mantissa == 0 {
            self.mantissa = digit as u64 >> trailing_zeros;
            return;
        }

        // The new digit always lands to the right of where the mantissa is
        // landing now. Compute the distance between those locations, which is
        // always positive.
        assert!(exponent >= self.last_digit_exponent);
        let gap = (exponent - self.last_digit_exponent) as u32;

        // How many bits would we need to shift into `mantissa` to represent the
        // new digit exactly? Don't count the new digit's trailing zeros.
        //
        // Note that this assumes `digit` is non-zero, which we ensured above.
        let new_digit_shift = gap - trailing_zeros;

        // How much space do we actually have in the mantissa for more precision?
        let max_shift = self.mantissa.leading_zeros();

        // Handle the common case where we can incorporate `digit` exactly.
        if new_digit_shift <= max_shift {
            self.mantissa = mantissa << new_digit_shift | (digit >> trailing_zeros) as u64;
            return;
        }

        // Oh well.
        self.exact = false;

        // We'll need to drop bits off the less significant end of `digit`.
        let bits_to_drop = new_digit_shift - max_shift;
        if bits_to_drop < 4 {
            digit = digit & !((1 << bits_to_drop) - 1);
        } else {
            digit = 0;
        }

        // Dropping bits may have exposed new trailing zeros (when rounding 9 to
        // 8, for example), or zeroed out `digit` altogether, so we must repeat
        // our earlier checks.
        if digit == 0 {
            // Restore the original exponent.
            self.exponent = exponent;
            return;
        }

        let trailing_zeros = digit.trailing_zeros();
        self.exponent = self.last_digit_exponent + trailing_zeros as i32;

        // Recompute the shift. This time it should always fit: otherwise we
        // would have zeroed out `digit` entirely above, and returned early.
        let new_digit_shift = gap - trailing_zeros;
        debug_assert!(new_digit_shift <= max_shift);

        self.mantissa = mantissa << new_digit_shift | (digit as u64 >> trailing_zeros);
    }

    /// Push all hexadecimal digits at the start of `digits` onto `self`.
    ///
    /// Return the remaining portion of `digits`, which is either empty, or
    /// starts with a character that is not a hexadecimal digit.
    ///
    /// If any hexadecimal digits are indeed present at the start of `digits`,
    /// then mark [`present`] as containing [`FRACTION`].
    ///
    /// [`present`]: Self::present
    /// [`FRACTION`]: PartFlags::FRACTION
    pub fn consume_fractional_digits<'d>(&mut self, mut digits: &'d str) -> &'d str {
        let mut chars = digits.chars();
        while let Some(digit) = chars.next().and_then(|ch| ch.to_digit(16)) {
            digits = chars.as_str();
            self.push_fractional_digit(digit);
        }

        digits
    }

    /// Construct a [`Mantissa`] value from the entire contents of `digits`.
    ///
    /// If `digits` does not consist entirely of hexadecimal digits, return an
    /// error.
    pub fn from_fractional_str(digits: &str) -> Result<Self, core::num::IntErrorKind> {
        let mut w = Parts::new();
        let rest = w.consume_fractional_digits(digits);
        if !rest.is_empty() {
            // We were stopped by a character that is not a hex digit, not by
            // reaching the end of the string.
            return Err(core::num::IntErrorKind::InvalidDigit);
        }
        Ok(w)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    type P = Parts<()>;

    #[test]
    fn consume_fractional_digits() {
        let mut f = P::new();
        assert_eq!(f.consume_fractional_digits("fp+2"), "p+2");
        assert_eq!(
            f,
            P {
                present: PartFlags::FRACTION,
                sign: 1,
                mantissa: 0xf,
                exponent: -4,
                last_digit_exponent: -4,
                explicit_exponent: 0,
                exact: true,
                suffix: None,
            }
        );
    }

    #[test]
    fn from_str() {
        assert_eq!(P::from_fractional_str(""), Ok(P::new()));
        assert_eq!(
            P::from_fractional_str("0"),
            Ok(P {
                present: PartFlags::FRACTION,
                sign: 1,
                mantissa: 0,
                exponent: 0,
                last_digit_exponent: -4,
                explicit_exponent: 0,
                exact: true,
                suffix: None,
            })
        );
        assert_eq!(
            P::from_fractional_str("00"),
            Ok(P {
                present: PartFlags::FRACTION,
                sign: 1,
                mantissa: 0,
                exponent: 0,
                last_digit_exponent: -8,
                explicit_exponent: 0,
                exact: true,
                suffix: None,
            })
        );
        assert_eq!(
            P::from_fractional_str("0000000000000000000000000000000000000000"),
            Ok(P {
                present: PartFlags::FRACTION,
                sign: 1,
                mantissa: 0,
                exponent: 0,
                last_digit_exponent: 40 * -4,
                explicit_exponent: 0,
                exact: true,
                suffix: None,
            })
        );
        assert_eq!(
            P::from_fractional_str("00000000000000000000000000000000000000001"),
            Ok(P {
                present: PartFlags::FRACTION,
                sign: 1,
                mantissa: 1,
                exponent: 41 * -4,
                last_digit_exponent: 41 * -4,
                explicit_exponent: 0,
                exact: true,
                suffix: None,
            })
        );
        assert_eq!(
            P::from_fractional_str("0000000000000000f"),
            Ok(P {
                present: PartFlags::FRACTION,
                sign: 1,
                mantissa: 0xf,
                exponent: (16 + 1) * -4,
                last_digit_exponent: (16 + 1) * -4,
                explicit_exponent: 0,
                exact: true,
                suffix: None,
            })
        );
        assert_eq!(
            P::from_fractional_str("0000000000000000f000000000000000"),
            Ok(P {
                present: PartFlags::FRACTION,
                sign: 1,
                mantissa: 0xf,
                exponent: (16 + 1) * -4,
                last_digit_exponent: (16 + 1 + 15) * -4,
                explicit_exponent: 0,
                exact: true,
                suffix: None,
            })
        );
        assert_eq!(
            P::from_fractional_str("0000000000000000f000000000000001"),
            Ok(P {
                present: PartFlags::FRACTION,
                sign: 1,
                mantissa: 0xf000000000000001,
                exponent: (16 + 1 + 14 + 1) * -4,
                last_digit_exponent: (16 + 1 + 14 + 1) * -4,
                explicit_exponent: 0,
                exact: true,
                suffix: None,
            })
        );
        assert_eq!(
            P::from_fractional_str("0000000000000000f0000000000000001"),
            Ok(P {
                present: PartFlags::FRACTION,
                sign: 1,
                mantissa: 0xf,
                exponent: (16 + 1) * -4,
                last_digit_exponent: (16 + 1 + 15 + 1) * -4,
                explicit_exponent: 0,
                exact: false,
                suffix: None,
            })
        );
        assert_eq!(
            P::from_fractional_str("0000000000000000000012300000000000000000000"),
            Ok(P {
                present: PartFlags::FRACTION,
                sign: 1,
                mantissa: 0x123,
                exponent: (20 + 3) * -4,
                last_digit_exponent: (20 + 3 + 20) * -4,
                explicit_exponent: 0,
                exact: true,
                suffix: None,
            })
        );
        assert_eq!(
            P::from_fractional_str("3000000000000000c"),
            Ok(P {
                present: PartFlags::FRACTION,
                sign: 1,
                mantissa: 0xc000000000000003,
                exponent: 16 * -4 - 2,
                last_digit_exponent: (16 + 1) * -4,
                explicit_exponent: 0,
                exact: true,
                suffix: None,
            })
        );
    }

    #[test]
    fn with_exponent() {
        assert_eq!(
            P::with_exponent(0, 0),
            P {
                present: PartFlags::empty(),
                sign: 1,
                mantissa: 0,
                exponent: 0,
                last_digit_exponent: 0,
                explicit_exponent: 0,
                exact: true,
                suffix: None
            }
        );
        assert_eq!(
            P::with_exponent(0x1, -20),
            P {
                present: PartFlags::empty(),
                sign: 1,
                mantissa: 0x1,
                exponent: -20,
                last_digit_exponent: -20,
                explicit_exponent: 0,
                exact: true,
                suffix: None
            }
        );
        assert_eq!(
            P::with_exponent(0x100, -20),
            P {
                present: PartFlags::empty(),
                sign: 1,
                mantissa: 0x1,
                exponent: -12,
                last_digit_exponent: -20,
                explicit_exponent: 0,
                exact: true,
                suffix: None
            }
        );
        assert_eq!(
            P::with_exponent(0x1000000000000001, -64),
            P {
                present: PartFlags::empty(),
                sign: 1,
                mantissa: 0x1000000000000001,
                exponent: -64,
                last_digit_exponent: -64,
                explicit_exponent: 0,
                exact: true,
                suffix: None
            }
        );
        assert_eq!(
            P::with_exponent(0x1000000010000000, -64),
            P {
                present: PartFlags::empty(),
                sign: 1,
                mantissa: 0x100000001,
                exponent: -64 + 7 * 4,
                last_digit_exponent: -64,
                explicit_exponent: 0,
                exact: true,
                suffix: None
            }
        );
    }
}
