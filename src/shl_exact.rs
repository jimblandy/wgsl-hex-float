//! Implementation of `shl_exact`, and tests.

// Shift `value` left by `bits` bits, returning `None` if any 1-bits would be lost.
//
// (If `exact_bitshifts` were stable, we could use `shl_exact` from the standard
// library.)
pub fn shl_exact(value: u64, bits: u32) -> Option<u64> {
    // We can always shift zero as far as you like.
    if value == 0 {
        return Some(0);
    }

    // Since `value` is non-zero, shifting it more than 64 bits would definitely
    // lose something.
    if bits >= 64 {
        return None;
    }

    // Rotate left, so that the bits shifted off the top end up at the less
    // significant end, where we can check them conveniently.
    let value = value.rotate_left(bits);

    // Check for non-zero bits that were shifted off the top and rotated back to
    // the bottom.
    let mask = (1 << bits) - 1;
    if value & mask != 0 {
        return None;
    }

    // Since all the bits rotated around to the bottom are zero, the rotate is
    // the same as the shift we wanted.
    Some(value)
}

#[test]
fn zero() {
    assert_eq!(shl_exact(0, 0), Some(0));
    assert_eq!(shl_exact(0, 63), Some(0));
    assert_eq!(shl_exact(0, 64), Some(0));
    assert_eq!(shl_exact(0, 1000), Some(0));
}

#[test]
fn one() {
    assert_eq!(shl_exact(1, 0), Some(1));
    assert_eq!(shl_exact(1, 4), Some(16));
    assert_eq!(shl_exact(1, 63), Some(1 << 63));
    assert_eq!(shl_exact(1, 64), None);
    assert_eq!(shl_exact(1, 1000), None);
}

#[test]
fn large() {
    assert_eq!(shl_exact(1 << 63, 0), Some(1 << 63));
    assert_eq!(shl_exact(1 << 63, 1), None);
    assert_eq!(shl_exact(15 << 60, 0), Some(15 << 60));
    assert_eq!(shl_exact(15 << 60, 1), None);
    assert_eq!(shl_exact(15 << 60, 4), None);
}
