/*! Definition of [`Parts`]. */

/// A hexadecimal literal in the process of being parsed.
///
/// This struct represents a hexadecimal numeric literal, either integral or
/// floating-point, that is in the midst of being parsed. Once a `Parts` value
/// is fully initialized, it can be converted into a Rust value via the
/// [`to_float`] and [`to_int`] methods.
///
/// For example:
///
/// ```
/// # use hex_float::{Parts, PartFlags};
/// let (p, rest) = hex_float::wgsl::parse("0x12.345p-678f").unwrap();
///
/// assert_eq!(p, Parts {
///     present:
///     PartFlags::PREFIX       // "0x"
///     | PartFlags::WHOLE      // "12"
///     | PartFlags::POINT      // "."
///     | PartFlags::FRACTION   // "345"
///     | PartFlags::EXPONENT,  // "p-687"
///     sign: 1,                // the default sign
///     mantissa: 0x12345,
///     exponent: -12,          // does not include explicit exponent
///     last_digit_exponent: -12,
///     explicit_exponent: -678,
///     exact: true,            // no rounding was needed
///     suffix: Some(hex_float::wgsl::Suffix::F32) // "f"
/// });
/// assert_eq!(rest, "");
/// ```
///
/// Note that trailing zeros are never stored in `mantissa`:
///
/// ```
/// # use hex_float::Parts;
/// let (p, rest) = hex_float::wgsl::parse("0x20p9").unwrap();
/// assert!(matches!(
///     p,
///     Parts {
///         mantissa: 0x1,
///         exponent: 5,
///         last_digit_exponent: 0,
///         explicit_exponent: 9,
///         ..
///     }
/// ));
/// assert_eq!(rest, "");
///
/// let (p, rest) = hex_float::wgsl::parse("0x0.060").unwrap();
/// assert!(matches!(
///     p,
///     Parts {
///         mantissa: 0x3,
///         exponent: -7,
///         last_digit_exponent: -12,
///         explicit_exponent: 0,
///         ..
///     }
/// ));
/// assert_eq!(rest, "");
/// ```
///
/// If you are performing your own lexical analysis, you can build `Parts`
/// values digit by digit, and then use their `to_int` or `to_float` methods to
/// construct the corresponding Rust value:
///
/// ```
/// # use hex_float::Parts;
/// let mut p = Parts::new();
/// p.push_whole_digit(0xa);
/// p.push_whole_digit(0xb);
///
/// p.push_fractional_digit(0xc);
/// p.push_fractional_digit(0xd);
/// p.push_fractional_digit(0x0);
///
/// assert!(matches!(
///     p,
///     Parts {
///         mantissa: 0xabcd,
///         exponent: -8,
///         last_digit_exponent: -12,
///         ..
///     }
/// ));
///
/// assert_eq!(p.to_float::<f32>(), Assembled::Exact(0xabcd0 as f32 / 64.0));
/// ```
///
/// Once you've pushed a fractional digit onto a `Parts` value, you may not push
/// any more whole-number digits onto it: the whole-building and
/// fraction-building phases must be carried out in that order.
///
/// The type parameter `S` gives the type of the language-specific suffix.
/// For parsing WGSL literals, this would be [`wgsl::Suffix`].
///
/// [`to_float`]: Self::to_float
/// [`to_int`]: Self::to_int
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Parts<S> {
    /// Which parts of the literal were present in the input.
    pub present: PartFlags,

    /// The sign of the literal, either `1` or `-1`.
    ///
    /// If no sign was parsed or present, this is `1`.
    pub sign: i32,

    /// The most significant bits of the value.
    ///
    /// If the input contains only zero digits, this is zero. Otherwise, the
    /// least significant bit is always `1`, and any trailing zeros are
    /// expressed in `exponent`.
    pub mantissa: u64,

    /// The power of two by which `mantissa` should be multiplied to yield the
    /// value of the whole-number and fractional parts, ignoring any explicit
    /// suffix.
    ///
    /// For example, to represent `0x12.345p100`, `mantissa` would be `0x12345`,
    /// and `exponent` would be `-12`. The `100` would appear in
    /// `explicit_exponent`.
    ///
    /// If `mantissa` is zero, this is also zero.
    pub exponent: i32,

    /// The exponent of the power of two giving the place value of the most
    /// recently pushed digit. was multiplied before incorporation. This remains
    /// at zero while we're pushing whole-number digits, since those are being
    /// inserted just before the hexadecimal point. For each fractional digit we
    /// push, this is decremented by four.
    pub last_digit_exponent: i32,

    /// The exponent written explicitly in the literal, or zero if no exponent
    /// is parsed or present.
    ///
    /// This must be added to `exponent` to obtain the literal's full value.
    pub explicit_exponent: i32,

    /// True if `mantissa` and `exponent` accurately capture all
    /// digits that have been presented.
    pub exact: bool,

    /// A language-specific type suffix.
    ///
    /// For example, in WGSL, an `h` suffix marks the literal as an `f16` value.
    /// This would be represented by [`wgsl::Suffix::F16`]. But other languages
    /// would supply different suffix types here.
    ///
    /// [`wgsl::Suffix::F16`]: crate::wgsl::Suffix::F16.
    pub suffix: Option<S>,
}

impl<S> Parts<S> {
    pub const MANTISSA_BITS: u32 = std::mem::size_of_val(&Parts::<()>::new().mantissa) as u32 * 8;
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
