/*! Parsing of WGSL hexademical floats.

The functions in this module parse hexadecimal float and integer literals as
described in the [WGSL] specification. On success, they return a [`Parts`]
value, whose [`to_float`] and [`to_int`] methods return the literal's numeric
value as a Rust floating-point or integer type.

The syntax accepted follows WGSL's [`hex_float_literal`] and [`hex_int_literal`]
productions. The specification presents these as somewhat hairy regular
expressions, but in plain English, the syntax of a WGSL hexadecimal literal is:

- A prefix of `"0x"` or `"0X"`;

- Zero or more hexadecimal digits, indicating the whole-number portion of the literal;

- Optionally, a hexadecimal point `'.'` followed by zero or more hexadecimal
  digits, indicating the fractional portion of the literal;

- Optionally, an exponent introducer character, `'p'` or `'P'`, followed by an
  optional `'+'` or `'-'` sign, followed by one or more decimal digits,
  indicating the exponent: the power of two by which the preceding number is
  multiplied;

- And an optional type suffix letter, indicating a concrete numerical type.

The hexadecimal digit strings may contain any combination of upper- and
lower-case letters.

In hexadecimal float literals, either the fractional portion or the exponent
must be present, and there must be at least one digit present in either the
whole or fractional portion.

In hexadecimal integer literals, the whole-number portion must have at
least one digit, and the fractional and exponent portions must be
absent.

[WGSL]: https://www.w3.org/TR/WGSL/
[`to_float`]: Parts::to_float
[`to_int`]: Parts::to_int
[`hex_float_literal`]: https://gpuweb.github.io/gpuweb/wgsl/#syntax-hex_float_literal
[`hex_int_literal`]: https://gpuweb.github.io/gpuweb/wgsl/#syntax-hex_int_literal

*/

use crate::{PartFlags, Parts};

/// Parse `input` as a WGSL hexadecimal floating-point or integer literal.
///
/// Consume a WGSL [`hex_float_literal`] or [`hex_int_literal`] from the front
/// of `input`. On success, return a [`Parts`] value presentin the parsed form of
/// the literal, and the unconsumed portion of `input`.
///
/// Integral and floating-point literals can be distinguished by checking for
/// the presence of either a fractional part or an exponent, as returned in
/// [`Parts::present`].
///
/// Note that this also allows a sign to be present at the beginning of the
/// literal. This is not part of the official WGSL literal syntax. If you would
/// ilke to forbid the sign, use [`parse_with_allowed_parts`] instead.
pub fn parse(input: &str) -> Result<(Parts<Suffix>, &str), Error> {
    let (parts, rest) = parse_with_allowed_parts(input, WGSL_FLOAT_OR_INT_PARTS | PartFlags::SIGN)?;

    if !parts.present.contains(PartFlags::PREFIX) {
        return Err(Error::MissingPrefix);
    }

    match parts.suffix {
        None => {
            // There have to be some mantissa digits present somewhere.
            if !parts
                .present
                .intersects(PartFlags::WHOLE | PartFlags::FRACTION)
            {
                return Err(Error::
            }
        }
        Some(Suffix::I32 | Suffix::U32) => {
            // The grammar does not include `i` or `u` suffixes if an exponent
            // or fractional part was present. The parser should have left them
            // in the input, unconsumed.
            assert!(
                !parts
                    .present
                    .intersects(PartFlags::FRACTION | PartFlags::EXPONENT)
            );

            // There have to be some mantissa digits present somewhere.
            if !parts.present.intersects(PartFlags::WHOLE) {
                return Err(Error::MissingWhole);
            }
        }
        Some(Suffix::F16 | Suffix::F32) => {
            // Since there was a floating-point type suffix, there must have
            // been an exponent.
            assert!(parts.present.contains(PartFlags::EXPONENT));

            // Even with the exponent present, there must have been some whole
            // digits or fractional digits.
            if !parts
                .present
                .intersects(PartFlags::WHOLE | PartFlags::FRACTION)
            {
                return Err(Error::MissingWholeAndFraction);
            }
        }
    }

    Ok((parts, rest))
}

/// Parts that may be present in a WGSL hexadecimal floating-point or integer literal.
pub const WGSL_FLOAT_OR_INT_PARTS: PartFlags = PartFlags::PREFIX
    .union(PartFlags::WHOLE)
    .union(PartFlags::POINT)
    .union(PartFlags::FRACTION)
    .union(PartFlags::EXPONENT)
    .union(PartFlags::TYPE_SUFFIX);

/// Parts that may be present in a WGSL hexadecimal integer literal.
pub const INT_PARTS: PartFlags = PartFlags::PREFIX.union(PartFlags::WHOLE);

/// Parse `input` as a WGSL hexadecimal floating-point or integer literal.
///
/// Consume a WGSL [`hex_float_literal`] or [`hex_int_literal`] from the front
/// of `input`, accepting those parts of the syntax included in `allow`. On
/// success, return a `Parts` value representing the parsed form of the literal,
/// and the unconsumed portion of `input`.
///
/// The `allow` argument indicates which parts of the literal syntax
/// should be recognized. In particular:
///
/// - If `allow` contains [`SIGN`], then `input` may start with a `'+'` or `'-'`
///   character, indicating the sign of the literal. This is not part of the
///   WGSL hexadecimal literal syntax, since WGSL treats signs as operators, not
///   parts of the literal, but it seems useful for other applications.
///
/// - If `allow` contains [`PREFIX`], then `input` may optionally start with a
///   `"0x"` or `"0X"` sequence (following the sign, if allowed and present). An
///   actual WGSL front end may have already consumed this prefix, in which case
///   this flag can be omitted. If you want to require a prefix, you must check
///   the [`present`] field of the returned [`Parts`] value.
///   
/// [`hex_float_literal`]: https://gpuweb.github.io/gpuweb/wgsl/#syntax-hex_float_literal
/// [`hex_int_literal`]: https://gpuweb.github.io/gpuweb/wgsl/#syntax-hex_int_literal
/// [`SIGN`]: PartFlags::SIGN
/// [`PREFIX`]: PartFlags::PREFIX_ALLOWED
/// [`present`]: Parts::present
pub fn parse_with_allowed_parts(
    mut input: &str,
    allow: PartFlags,
) -> Result<(Parts<Suffix>, &str), Error> {
    assert!(
        allow.contains(PartFlags::FRACTION) == allow.contains(PartFlags::POINT),
        "If `allow' contains `FRACTION`, it must also contain `POINT`, and vice versa"
    );
    let mut result = Parts::<Suffix>::new();

    // Parse a sign.
    if allow.contains(PartFlags::SIGN) {
        if let Some(present_sign) = parse_sign(&mut input) {
            result.present.insert(PartFlags::SIGN);
            result.sign = present_sign;
        } else {
            result.sign = 1;
        }
    };

    // Parse a prefix.
    if allow.contains(PartFlags::PREFIX) {
        if let Some(rest) = input
            .strip_prefix("0x")
            .or_else(|| input.strip_prefix("0X"))
        {
            result.present.insert(PartFlags::PREFIX);
            input = rest;
        }
    }

    // Parse a whole number portion.
    if allow.contains(PartFlags::WHOLE) {
        input = result.consume_whole_digits(input);
    }

    // Parse a fractional portion.
    if allow.contains(PartFlags::POINT) {
        if let Some(after_point) = input.strip_prefix('.') {
            result.present.insert(PartFlags::POINT);
            // We asserted that `FRACTION` is also allowed.
            input = result.consume_fractional_digits(after_point);
        }
    }

    // Parse an exponent.
    if allow.contains(PartFlags::EXPONENT) {
        if let Some(rest) = input.strip_prefix(&['p', 'P'][..]) {
            input = rest;
            let exponent_sign = parse_sign(&mut input).unwrap_or(1);
            let mut exponent: i32 = 0;
            let mut chars = input.chars();
            while let Some(digit) = chars.next().and_then(|ch| ch.to_digit(10)) {
                input = chars.as_str();
                result.present.insert(PartFlags::EXPONENT);
                // To handle `i32::MIN` correctly, multiply the sign into each
                // digit as we incorporate it, rather than doing the multiply
                // once at the end. The code is simpler this way, and multiplies
                // are cheap nowadays. This is a little ridiculous, as no actual
                // floating point format could represent such an exponent
                // anyway, but it's not that hard for this code to at least do
                // its job correctly.
                exponent = exponent
                    .checked_mul(10)
                    .and_then(|exponent| exponent.checked_add(digit as i32 * exponent_sign))
                    .ok_or(Error::ExponentOverflow)?;
            }
            if !result.present.contains(PartFlags::EXPONENT) {
                return Err(Error::ExponentMissingDigits);
            }
            result.explicit_exponent = exponent;
        }
    }

    // Parse a type suffix.
    if allow.contains(PartFlags::TYPE_SUFFIX) {
        let mut chars = input.chars();

        // If we've seen an exponent, then floating-point suffixes are allowed.
        //
        // If we've seen no evidence of a float literal (either fractional
        // digits or an exponent), then integer literals are allowed.
        //
        // Unacceptable suffixes are simply left in `input` unconsumed. In WGSL,
        // these rules are not post-parsing consistency checks; they're built
        // into the grammar, so the syntax for literals just doesn't include
        // them. I don't think there's any place in WGSL where an identifier can
        // directly follow a literal, so it'll trigger an error anyway.
        result.suffix = if result.present.contains(PartFlags::EXPONENT) {
            match chars.next() {
                Some('h') => Some(Suffix::F16),
                Some('f') => Some(Suffix::F32),
                _ => None,
            }
        } else if !result
            .present
            .intersects(PartFlags::POINT | PartFlags::EXPONENT)
        {
            match chars.next() {
                Some('u') => Some(Suffix::U32),
                Some('i') => Some(Suffix::I32),
                _ => None,
            }
        } else {
            None
        };

        if result.suffix.is_some() {
            result.present.insert(PartFlags::TYPE_SUFFIX);
            input = chars.as_str();
        }
    }

    Ok((result, input))
}

fn parse_sign(input: &mut &str) -> Option<i32> {
    if let Some(rest) = input.strip_prefix('+') {
        *input = rest;
        Some(1)
    } else if let Some(rest) = input.strip_prefix('-') {
        *input = rest;
        Some(-1)
    } else {
        None
    }
}

/// A WGSL type suffix, indicating that the literal has a concrete type.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Suffix {
    /// The suffix `'i'`, indicating a 32-bit signed integer literal.
    I32,

    /// The suffix `'u'`, indicating a 32-bit unsigned integer literal.
    U32,

    /// The suffix `'h'`, indicating a 16-bit floating-point literal.
    F16,

    /// The suffix `'f'`, indicating a 32-bit floating-ponit literal.
    F32,
}

#[derive(thiserror::Error, Debug, Eq, PartialEq)]
pub enum Error {
    #[error("hexadecimal literal was missing `0x` or `0X` prefix")]
    MissingPrefix,

    #[error("hexadecimal literal exponent is too large")]
    ExponentOverflow,

    #[error("hexadecimal literal exponent must contain at least one digit")]
    ExponentMissingDigits,

    #[error("hexadecimal integer literal must contain at least one whole digit")]
    MissingWhole,

    #[error("hexadecimal literal must contain at least one whole or fractional digit")]
    MissingWholeAndFraction,
}

#[test]
fn spec_examples() {
    use PartFlags as Pf;

    assert_eq!(
        parse("0x123*"),
        Ok((
            Parts {
                present: Pf::PREFIX | Pf::WHOLE,
                sign: 1,
                mantissa: 0x123,
                exponent: 0,
                last_digit_exponent: 0,
                explicit_exponent: 0,
                exact: true,
                suffix: None
            },
            "*"
        ))
    );

    assert_eq!(
        parse("0xa.fp+2 "),
        Ok((
            Parts {
                present: Pf::PREFIX | Pf::WHOLE | Pf::POINT | Pf::FRACTION | Pf::EXPONENT,
                sign: 1,
                mantissa: 0xaf,
                exponent: -4,
                last_digit_exponent: -4,
                explicit_exponent: 2,
                exact: true,
                suffix: None
            },
            " "
        ))
    );

    assert_eq!(
        parse("0x1P+4f "),
        Ok((
            Parts {
                present: Pf::PREFIX | Pf::WHOLE | Pf::EXPONENT | Pf::TYPE_SUFFIX,
                sign: 1,
                mantissa: 0x1,
                exponent: 0,
                last_digit_exponent: 0,
                explicit_exponent: 4,
                exact: true,
                suffix: Some(Suffix::F32),
            },
            " "
        ))
    );

    assert_eq!(
        parse("0X.3."),
        Ok((
            Parts {
                present: Pf::PREFIX | Pf::POINT | Pf::FRACTION,
                sign: 1,
                mantissa: 0x3,
                exponent: -4,
                last_digit_exponent: -4,
                explicit_exponent: 0,
                exact: true,
                suffix: None,
            },
            "."
        ))
    );

    assert_eq!(
        parse("0x3p+2h0"),
        Ok((
            Parts {
                present: Pf::PREFIX | Pf::WHOLE | Pf::EXPONENT | Pf::TYPE_SUFFIX,
                sign: 1,
                mantissa: 0x3,
                exponent: 0,
                last_digit_exponent: 0,
                explicit_exponent: 2,
                exact: true,
                suffix: Some(Suffix::F16),
            },
            "0"
        ))
    );

    assert_eq!(
        parse("0X1.fp-4-"),
        Ok((
            Parts {
                present: Pf::PREFIX | Pf::WHOLE | Pf::POINT | Pf::FRACTION | Pf::EXPONENT,
                sign: 1,
                mantissa: 0x1f,
                exponent: -4,
                last_digit_exponent: -4,
                explicit_exponent: -4,
                exact: true,
                suffix: None,
            },
            "-"
        ))
    );

    assert_eq!(
        parse("0x3.2p+2hx"),
        Ok((
            Parts {
                present: Pf::PREFIX
                    | Pf::WHOLE
                    | Pf::POINT
                    | Pf::FRACTION
                    | Pf::EXPONENT
                    | Pf::TYPE_SUFFIX,
                sign: 1,
                mantissa: 0x19,
                exponent: -3,
                last_digit_exponent: -4,
                explicit_exponent: 2,
                exact: true,
                suffix: Some(Suffix::F16),
            },
            "x"
        ))
    );
}

#[test]
fn type_suffix() {
    use PartFlags as Pf;

    assert_eq!(
        parse("0x1.0f "), // hex digit, suffix
        Ok((
            Parts {
                present: Pf::PREFIX | Pf::WHOLE | Pf::POINT | Pf::FRACTION,
                sign: 1,
                mantissa: 0x10f,
                exponent: -8,
                last_digit_exponent: -8,
                explicit_exponent: 0,
                exact: true,
                suffix: None,
            },
            " "
        )),
    );

    assert_eq!(
        parse("0x1.0h "), // no exponent, suffix not parsed
        Ok((
            Parts {
                present: Pf::PREFIX | Pf::WHOLE | Pf::POINT | Pf::FRACTION,
                sign: 1,
                mantissa: 0x1,
                exponent: 0,
                last_digit_exponent: -4,
                explicit_exponent: 0,
                exact: true,
                suffix: None,
            },
            "h "
        )),
    );

    assert_eq!(
        parse("0x1h"), // no exponent, suffix not parsed
        Ok((
            Parts {
                present: Pf::PREFIX | Pf::WHOLE,
                sign: 1,
                mantissa: 0x1,
                exponent: 0,
                last_digit_exponent: 0,
                explicit_exponent: 0,
                exact: true,
                suffix: None
            },
            "h"
        )),
    );

    assert_eq!(
        parse("0x1uf"),
        Ok((
            Parts {
                present: Pf::PREFIX | Pf::WHOLE | Pf::TYPE_SUFFIX,
                sign: 1,
                mantissa: 0x1,
                exponent: 0,
                last_digit_exponent: 0,
                explicit_exponent: 0,
                exact: true,
                suffix: Some(Suffix::U32),
            },
            "f"
        )),
    );

    assert_eq!(
        parse("0x1.0u"), // fraction present, integer suffix not parsed
        Ok((
            Parts {
                present: Pf::PREFIX | Pf::WHOLE | Pf::POINT | Pf::FRACTION,
                sign: 1,
                mantissa: 0x1,
                exponent: 0,
                last_digit_exponent: -4,
                explicit_exponent: 0,
                exact: true,
                suffix: None,
            },
            "u"
        )),
    );

    assert_eq!(
        parse("0x1p4u"), // exponent present, integer suffix not parsed
        Ok((
            Parts {
                present: Pf::PREFIX | Pf::WHOLE | Pf::EXPONENT,
                sign: 1,
                mantissa: 0x1,
                exponent: 0,
                last_digit_exponent: 0,
                explicit_exponent: 4,
                exact: true,
                suffix: None
            },
            "u"
        )),
    );
}

#[test]
fn exponent_overflow() {
    use PartFlags as Pf;
    assert_eq!(
        parse("0x1p2147483647"),
        Ok((
            Parts {
                present: Pf::PREFIX | Pf::WHOLE | Pf::EXPONENT,
                sign: 1,
                mantissa: 0x1,
                exponent: 0,
                last_digit_exponent: 0,
                explicit_exponent: 2147483647,
                exact: true,
                suffix: None
            },
            ""
        )),
    );
    assert_eq!(parse("0x1p2147483648"), Err(Error::ExponentOverflow));
    assert_eq!(
        parse("0x1p-2147483648"),
        Ok((
            Parts {
                present: Pf::PREFIX | Pf::WHOLE | Pf::EXPONENT,
                sign: 1,
                mantissa: 0x1,
                exponent: 0,
                last_digit_exponent: 0,
                explicit_exponent: -2147483648,
                exact: true,
                suffix: None
            },
            ""
        )),
    );
    assert_eq!(parse("0x1p-2147483649"), Err(Error::ExponentOverflow));
}

#[test]
fn errors() {
    assert_eq!(parse("0"), Err(Error::MissingPrefix));
    assert_eq!(parse("0x.p0"), Err(Error::MissingWholeAndFraction));
    assert_eq!(parse("0xu"), Err(Error::MissingWhole));
    assert_eq!(parse("0x.p4f"), Err(Error::MissingWholeAndFraction));
    assert_eq!(parse("0x1pa"), Err(Error::ExponentMissingDigits));
    assert_eq!(parse("0x.h"), Err(Error::MissingWholeAndFraction));
}
