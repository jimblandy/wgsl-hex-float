/*! Parsing hexadecimal float values.

This crate provides `#[no_std]`-friendly functions for parsing hexadecimal float
values as described in the [WGSL specification][wgsl].

```ignore
# fn main() {
let f: f32 = wgsl_hex_float::parse("0x80.8p-5").unwrap().get();
assert_eq!(f, 4.015625);

// Parse a literal into its parts
let (p, rest) = wgsl_hex_float::parse_as_parts("-0xc.04p5 and more").unwrap();
assert_eq!(p, Parts {
    sign: -1,
    prefix: true,
    whole: Whole::whole(12),
    fraction: Fraction::with_exponent(4, -8), // 4 * 2^-8
    exponent: 5,
});
assert_eq!(rest, " and more");

// Assemble those parts into an `f64`.
let g: f64 = wgsl_hex_float::from_parts(p).unwrap().get();
assert_eq!(g, -384.5);
# }
```

This crate reports rounding and overflow, when the value of a hexadecimal
floating point literal cannot be expressed exactly in the target type. (WGSL
implementations must treat these cases as compile-time errors.)

This crate also provides entry points that accept pre-parsed sign, exponent,
whole part and fractional part values: if users have already performed their own
lexical analysis, this crate doesn't insist on repeating that work for them. In
this case, the crate still takes care of assembling the pieces into a Rust
floating point value, while checking for overflow and loss of precision.

Finally, this crate provides helper types for accumulating the whole-number and
fractional portions of hexadecimal float literals, which check for overflow,
properly fold leading and trailing zeros into the exponent, and so on.

[wgsl]: https://www.w3.org/TR/WGSL/

*/

mod format;
mod fraction;
mod parse;
mod shl_exact;
mod whole;

pub use format::BinaryFormat;
pub use fraction::Fraction;
pub use parse::{Parts, parse_parts};
pub use whole::Whole;

/// The result of assembling a hexadecimal float value.
pub enum Parsed<T> {
    /// The input can be represented exactly as the given value.
    Exact(T),

    /// The input value cannot be represented exactly in the given type,
    /// but can be represented with rounding as the given value.
    Rounded(T),

    /// The input overflowed, and is represented as the given infinity.
    Infinity(T),
}

impl<T> Parsed<T> {
    pub fn get(self) -> T {
        match self {
            Parsed::Exact(v) => v,
            Parsed::Rounded(v) => v,
            Parsed::Infinity(v) => v,
        }
    }
}
