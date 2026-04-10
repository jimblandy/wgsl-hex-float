/*! Descriptions of IEEE 754 binary floating point formats.

This module defines the [`BinaryFormat`] trait, which characterizes a floating
point type that follows the IEEE 754 binary conventions. This crate can parse
any type that implements this trait.

*/

/// A floating point type following the rules for a IEEE 754 binary format.
///
/// All IEEE 754 binary floating point formats have a common structure:
///
/// - The most significant bit is a sign bit, followed by some number
///   of exponent bits, with the rest of the value containing mantissa
///   bits.
///
/// - The sign bit is `0` for positive values, and `1` for negative values.
///   Below, let `s` be `1` when the sign bit is `0`, and `-1` when the sign bit
///   is `1`. [`BinaryFormat::SIGN_SHIFT`] gives the position of the sign bit
///   within a value; it's always one less than the total number of bits.
///
/// - The mantissa bits give the fractional digits of a binary number, where the
///   most significant bit has a value of 1/2, and the next most significant
///   bits have values of 1/4, 1/8, 1/16, and so on. Call this value between
///   zero and one `m`. (We'll cover the implicit leading `1` bit below.)
///   [`BinaryFormat::MANTISSA_WIDTH`] gives the width of the format's mantissa
///   in bits.
///
/// - The exponent field, whose width in bits is given by
///   [`BinaryFormat::EXPONENT_WIDTH`], usually provides an exponent, but if all
///   bits are clear, or all bits are set, those have special meanings:
///
///   - If the exponent is neither all zero bits nor all one bits, then
///     interpret it as an unsigned number `e`. The value of the floating point
///     number is then `s * (1 + m) * 2^(e - B)`, where `B` is a bias that is a
///     characteristic constant of the floating point format.
///     [`BinaryFormat::EXPONENT_BIAS`] gives the format's value for `B`.
///
///   - If the exponent is all zero bits, then the value is a "subnormal" value,
///     representing a value very close to zero, equal to `s * m * 2^(1 - B))`.
///     Note the use of `m` instead of `(1 + m)`; subnormals are the only way to
///     get a zero in an IEEE binary floating point format. Note also the use of
///     `1` as the pre-biased exponent, not zero as the bitfield would suggest.
///
///   - If the exponent is all one bits (that is, the largest value the bitfield
///     can hold), then:
///
///     - If the mantissa bits are zero, the value is an infinity, whose sign is
///       given by the sign bit.
///
///     - Otherwise, if the mantissa bits are non-zero, the value is a NaN, and
///       the mantissa bits carry some sort of diagnostic information whose
///       interpretation IEEE doesn't specify, and which is generally up to the
///       application.
///
/// The IEEE 754 `binary16`, `binary32`, `binary64`, and `binary128` formats all
/// satisfy these constraints. Rust`s `f16`, `f32`, `f64`, and `f128` types use
/// those formats.
pub trait BinaryFormat: Sized {
    /// The length of the mantissa field in bits.
    const MANTISSA_WIDTH: u32;

    /// The length of the exponent field in bits.
    ///
    /// This is, naturally, everything left over after you've placed the
    /// mantissa at the bottom and the sign bit at the top.
    const EXPONENT_WIDTH: u32 = std::mem::size_of::<Self>() as u32 * 8 - Self::MANTISSA_WIDTH - 1;

    /// The bit position of the sign bit.
    ///
    /// This is, naturally, one short of the total number of bits in the type.
    const SIGN_SHIFT: u32 = std::mem::size_of::<Self>() as u32 * 8 - 1;

    /// The exponent bias.
    ///
    /// If the exponent field is interpreted as an unsigned number, this is the
    /// number subtracted from that to produce the actual exponent for the power
    /// of two by which the mantissa is multipled.
    ///
    /// In principle, many different biases would work, but IEEE formats all
    /// choose this to be half the largest value the exponent field can hold,
    /// rounded down.
    const EXPONENT_BIAS: i32 = (1 << Self::EXPONENT_WIDTH - 1) - 1;

    /// The minimum unbiased exponent a normal, non-infinite `Self` value can have.
    const MIN_NORMAL_EXP: i32 = 1 - Self::EXPONENT_BIAS;

    /// The maximum unbiased exponent a normal, non-infinite `Self` value can have.
    const MAX_NORMAL_EXP: i32 = (1 << Self::EXPONENT_WIDTH) - 1 - Self::EXPONENT_BIAS;

    /// Construct a `Self` floating-point value, given explicit values for its fields.
    ///
    /// The `sign` argument must be either 0 (positive) or 1 (negative).
    ///
    /// The `exponent_bits` argument must fit in [`Self::EXPONENT_WIDTH`] bits,
    /// and gives the exact contents of the IEEE 754 binary format exponent field. For
    /// example, if `EXPONENT_WIDTH` is 8, then a value of `255` represents an
    /// infinity or NaN value, and a value of `127` means that the mantissa
    /// should be multiplied by 2**0, or 1.
    ///
    /// The `mantissa_bits` argument must fit in [`Self::MANTISSA_WIDTH`] bits,
    /// and gives the exact contents of the IEEE binary format mantissa field.
    /// For example, if `EXPONENT_WIDTH` is 8, then with a `sign` of 0
    /// (positive) and an `exponent_bits` argument of `127` (indicating an
    /// exponent of zero), a `mantissa_bits` argument of `0` represents the
    /// value `1`, due to the implicit leading `1` bit in normal IEEE binary
    /// manissa values.

    // It may seem like this should have a default definition based on a
    // `from_bits` method, but the trait bounds required are a cure worse than
    // the disease. This function is perfectly straightforward to implement
    // directly.
    fn from_bitfields(sign: u32, exponent_bits: u32, mantissa_bits: u64) -> Self;

    fn infinity(negative: bool) -> Self;
}

impl BinaryFormat for f32 {
    const MANTISSA_WIDTH: u32 = 23;

    fn from_bitfields(sign: u32, exponent_bits: u32, mantissa_bits: u64) -> Self {
        assert!(sign <= 1);
        assert!(exponent_bits < 1 << Self::EXPONENT_WIDTH);
        assert!(mantissa_bits < 1 << Self::MANTISSA_WIDTH);

        let bits =
            sign << Self::SIGN_SHIFT | exponent_bits << Self::MANTISSA_WIDTH | mantissa_bits as u32;
        f32::from_bits(bits)
    }

    fn infinity(negative: bool) -> Self {
        if negative {
            Self::NEG_INFINITY
        } else {
            Self::INFINITY
        }
    }
}

impl BinaryFormat for f64 {
    const MANTISSA_WIDTH: u32 = 52;

    fn from_bitfields(sign: u32, exponent_bits: u32, mantissa_bits: u64) -> Self {
        assert!(sign <= 1);
        assert!(exponent_bits < 1 << Self::EXPONENT_WIDTH);
        assert!(mantissa_bits < 1 << Self::MANTISSA_WIDTH);

        let bits = (sign as u64) << Self::SIGN_SHIFT
            | (exponent_bits as u64) << Self::MANTISSA_WIDTH
            | mantissa_bits;
        f64::from_bits(bits)
    }

    fn infinity(negative: bool) -> Self {
        if negative {
            Self::NEG_INFINITY
        } else {
            Self::INFINITY
        }
    }
}

#[test]
fn basic() {
    assert_eq!(f32::from_bitfields(0, 0, 0), 0.0);
    assert_eq!(f64::from_bitfields(0, 0, 0), 0.0);

    assert_eq!(f32::from_bitfields(0, 126, 0), 0.5);
    assert_eq!(f64::from_bitfields(0, 1022, 0), 0.5);
    assert_eq!(f32::from_bitfields(1, 126, 0), -0.5);
    assert_eq!(f64::from_bitfields(1, 1022, 0), -0.5);

    assert_eq!(f32::from_bitfields(0, 127, 0), 1.0);
    assert_eq!(f64::from_bitfields(0, 1023, 0), 1.0);
    assert_eq!(f32::from_bitfields(1, 127, 0), -1.0);
    assert_eq!(f64::from_bitfields(1, 1023, 0), -1.0);

    assert_eq!(f32::from_bitfields(0, 128, 0), 2.0);
    assert_eq!(f64::from_bitfields(0, 1024, 0), 2.0);
    assert_eq!(f32::from_bitfields(1, 128, 0), -2.0);
    assert_eq!(f64::from_bitfields(1, 1024, 0), -2.0);
}

/// Tests for values with interesting mantissas, like fractions whose
/// denominator is not a power of two.
#[test]
fn interesting_normals() {
    // 1 + 1/4 + 1/16 + 1/64 + ... = 4/3. Consider a geometric series with a
    // common ratio of 1/4.
    //
    // If we were to write this out using a "binary point" (instead of a
    // "decimal point"), this value would be `1.01010101010101010101010...`
    //
    // The leading `1.` is captured by IEEE's implicit leading `1` bit. We don't
    // need to include it in our mantissa value.
    //
    // For `f32`:
    //
    // Truncating the fractional part to 23 bits, we would get:
    // `0.01010101010101010101010`.
    //
    // But rather than truncating, we should round to the nearest value we can
    // represent. The continuation of the truncated version would be
    // `0._______________________1010101010...`, so we should round that last
    // bit up, giving a fractional part of `0.01010101010101010101011`.
    //
    // For `f64`, everything is similar, except that since the mantissa is 52
    // bits long, the leading digit of the residue is zero, so we don't round
    // up.
    //
    // Note that it is fine to write `4.0 / 3.0` here: IEEE 754 requires that
    // division produce correctly rounded results, so Rust has no leeway in
    // which bit pattern to produce.
    assert_eq!(
        f32::from_bitfields(0, 127, 0b01010101010101010101011),
        4.0 / 3.0
    );
    assert_eq!(
        f64::from_bitfields(
            0,
            1023,
            0b0101010101010101010101010101010101010101010101010101
        ),
        4.0 / 3.0
    );

    // In "binary point": `1.010 * 2**3 == 10`.
    assert_eq!(f32::from_bitfields(0, 130, 0b01000000000000000000000), 10.0);
    assert_eq!(
        f64::from_bitfields(
            0,
            1026,
            0b0100000000000000000000000000000000000000000000000000
        ),
        10.0
    );

    // 1 + 3/16 + 3/256 + 3/4096 + ... = 6/5. Consider a geometric series with a
    // common ratio of 1/16.
    //
    // If we were to write this out using a "binary point" (instead of a
    // "decimal point"), this value would be `1.0011001100110011...`
    //
    // The leading `1.` is captured by IEEE's implicit leading `1` bit. We don't
    // need to include it in our mantissa value.
    //
    // For `f32`:
    //
    // Truncating the fractional part to 23 bits, we would get:
    // `0.00110011001100110011001`.
    //
    // But rather than truncating, we should round to the nearest value we can
    // represent. The continuation of the truncated version would be
    // `0._______________________100110011001100...`, so we should round that
    // last bit up, giving a fractional part of `0.00110011001100110011010`.
    //
    // For `f64`, everything is similar, except that since the mantissa is 52
    // bits long, the leading digit of the residue is zero, so we don't round
    // up.
    assert_eq!(
        f32::from_bitfields(0, 127, 0b00110011001100110011010),
        6.0 / 5.0
    );
    assert_eq!(
        f64::from_bitfields(
            0,
            1023,
            0b0011001100110011001100110011001100110011001100110011
        ),
        6.0 / 5.0
    );
}

/*
#[cfg(test)]
struct Unpacked<T>(T);

#[cfg(test)]
impl std::fmt::Display for Unpacked<f32> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let bits = self.0.to_bits();
        let sign = bits >> 31;
        let exponent = bits >> 23 & 0xff;
        let mantissa = bits & ((1 << 23) - 1);

        write!(f, "{{ sign: {sign}, exponent: {exponent}, mantissa: 0b{mantissa:023b} }}")
    }
}
*/

#[test]
fn f32_subnormals() {
    // Unlike f32::powi, the results of all the f32 arithmetic operations here
    // are exactly specified by IEEE 754.
    let two_p20: f32 = (1 << 20) as _;
    let two_p40 = two_p20 * two_p20;
    let two_p80 = two_p40 * two_p40;
    let two_p120 = two_p80 * two_p40;
    // Anything above 2**127 is out of range. So the subnormals can represent
    // values whose reciprocals cannot be represented!

    // The smallest non-subnormal value.
    assert_eq!(
        f32::from_bitfields(0, 1, 0b00000000000000000000000),
        1.0 / two_p120 / 64.0
    );

    // The largest subnormal power of two.
    //
    // Note: compared to the value above, we decreased the exponent by one,
    // *and* shifted the effective mantissa right by one, but the value is only
    // halved, not quartered. The effective exponent for subnormals is
    // `1-EXPONENT_BIAS`, even though the exponent field is `0`.
    assert_eq!(
        f32::from_bitfields(0, 0, 0b10000000000000000000000),
        1.0 / two_p120 / 128.0
    );

    assert_eq!(
        f32::from_bitfields(0, 0, 0b00001000000000000000000),
        1.0 / two_p120 / 128.0 / 16.0
    );

    // The smallest subnormal, and thus the smallest `f32`.
    assert_eq!(
        f32::from_bitfields(0, 0, 0b00000000000000000000001),
        1.0 / two_p120 / two_p20 / 512.0
    );
    assert_eq!(
        f32::from_bitfields(0, 0, 0b00000000000000000000001),
        f32::next_up(0.0)
    );
}
