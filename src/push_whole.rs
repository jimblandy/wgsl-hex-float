//! Implementation of [`Parts::push_whole_digit`] and friends.
#![allow(unused)]

use crate::{PartFlags, Parts};

impl<S> Parts<S> {
    /// Append a hexadecimal digit whose numeric value is `digit` to the
    /// whole-number portion of `self`.
    ///
    /// For example, if `self` currently represents `0x12`, then
    /// pushing `3` would change it to represent `0x123`.
    ///
    /// This adds [`WHOLE`] to [`present`].
    ///
    /// Once you've pushed fractional digits onto a `Parts`, you may no longer
    ///
    /// [`WHOLE`]: PartFlags::WHOLE
    /// [`present`]: Self::present
    pub fn push_whole_digit(&mut self, mut digit: u32) {
        assert!(digit < 16);
        assert!(
            self.exponent >= 0 && self.last_digit_exponent == 0,
            "Cannot push whole-number digits after fractional digits have been pushed"
        );

        // Note that at least one fractional digit was present.
        self.present.insert(PartFlags::WHOLE);

        // If the new digit is zero, we can adjust the exponent and go home.
        if digit == 0 {
            if self.mantissa != 0 {
                self.exponent = self.exponent.checked_add(4).unwrap();
            }
            return;
        }

        let trailing_zeros = digit.trailing_zeros() as i32;

        // How far would we need to shift the mantissa to incorporate
        // this digit exactly?
        let new_digit_shift = self.exponent.checked_add(4 - trailing_zeros).unwrap();

        // How much room for new bits do we actually have in the mantissa?
        let max_shift = self.mantissa.leading_zeros() as i32;

        // Handle the common case where we can incorporate `digit` exactly.
        if new_digit_shift <= max_shift {
            self.mantissa = (self.mantissa << new_digit_shift) | (digit as u64 >> trailing_zeros);
            self.exponent = trailing_zeros;
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
            if self.mantissa != 0 {
                self.exponent = self.exponent.checked_add(4).unwrap();
            }
            return;
        }

        let trailing_zeros = digit.trailing_zeros() as i32;

        // Recompute the shift. This time it should always fit: otherwise we
        // would have zeroed out `digit` entirely above, and returned early.
        let new_digit_shift = self.exponent.checked_add(4 - trailing_zeros).unwrap();
        debug_assert!(new_digit_shift <= max_shift);

        self.mantissa = (self.mantissa << new_digit_shift) | (digit as u64 >> trailing_zeros);
        self.exponent = trailing_zeros;
    }

    pub fn consume_whole_digits<'d>(&mut self, mut digits: &'d str) -> &'d str {
        let mut chars = digits.chars();
        while let Some(digit) = chars.next().and_then(|ch| ch.to_digit(16)) {
            digits = chars.as_str();
            self.push_whole_digit(digit);
        }

        digits
    }

    pub fn from_whole_str(digits: &str) -> Result<Self, core::num::IntErrorKind> {
        let mut w = Parts::new();
        let rest = w.consume_whole_digits(digits);
        if !rest.is_empty() {
            // We were stopped by a character that is not a hex digit, not by
            // reaching the end of the string.
            return Err(core::num::IntErrorKind::InvalidDigit);
        }
        Ok(w)
    }
}
