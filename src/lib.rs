/*! Parsing hexadecimal float values.

This crate provides `#[no_std]`-friendly functions for parsing hexadecimal float
values as described in the [WGSL specification][wgsl].

```
let f: f32 = wgsl_hex_float::parse("0x80.8p-5");
assert_eq!(f, 4.015625);

// `0xc.04p5`, pre-parsed into its components
let g: f64 = wgsl_hex_float::from_parts(Parts {
    sign: -1,
    whole: Whole::from(12),
    fraction: Fraction::from(4, 2), // two digits, `04`
    exponent: 5,
});
assert_eq!(g, -384.5);
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

pub use format::BinaryFormat;
