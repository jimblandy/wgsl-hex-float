//! Construction methods for [`Parts`].

use crate::{PartFlags, Parts};

impl<S> Parts<S> {
    /// Construct a new `Parts` value.
    ///
    /// The returned value:
    /// - has no part flags set;
    /// - has sign, exponent, and mantissa representing positive zero;
    /// - is ready to accept digits from the whole number portion of the literal; and
    /// - has no type suffix.
    pub fn new() -> Parts<S> {
        Parts {
            present: PartFlags::empty(),
            sign: 1,
            mantissa: 0,
            exponent: 0,
            last_digit_exponent: 0,
            explicit_exponent: 0,
            exact: true,
            suffix: None,
        }
    }

    /// Return the `Parts` value representing `mantissa * 2**exponent`.
    ///
    /// The next digit pushed will be placed just to the right of where
    /// `exponent` places `mantissa`. For this to make much sense as a string of
    /// hex digits, you should pass a multiple of four for `mantissa`.
    ///
    /// ```
    /// # use hex_float::{Parts, PartFlags};
    /// let mut f: Parts<()> = Parts::with_exponent(0x100, -20);
    /// f.push_fractional_digit(0xf);
    /// 
    /// let mut explicit = Parts::with_exponent(0x100f, -24);
    /// explicit.present = PartFlags::FRACTION;
    /// assert_eq!(f, explicit);
    /// ```
    pub fn with_exponent(mantissa: u64, exponent: i32) -> Self {
        assert!(mantissa != 0 || exponent == 0);

        let mut parts = Parts::new();

        if mantissa != 0 {
            let trailing_zeros = mantissa.trailing_zeros() as i32;

            parts.mantissa = mantissa >> trailing_zeros;
            parts.exponent = match exponent.checked_add(trailing_zeros) {
                Some(e) => e,
                None => {
                    parts.exact = false;
                    i32::MAX
                }
            };

            if exponent < 0 {
                parts.last_digit_exponent = exponent;
            }
        }

        parts
    }
}
