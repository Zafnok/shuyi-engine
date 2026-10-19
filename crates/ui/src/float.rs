//! The float methods `core` lacks, from `libm`, for the `no_std` build
//! (ADR-0070 §3): import [`Float`] and [`Round`] only without `std`. With
//! `std` the standard methods are called, so the numbers stay exactly what
//! they were; the tests below hold the two together. A game built on the
//! engine without `std` imports them too.

/// The `f32` methods the frame and the game call that need `std`.
pub trait Float: Sized {
    /// Nearest integer, halves away from zero.
    #[must_use]
    fn round(self) -> Self;
    /// Largest integer not above `self`.
    #[must_use]
    fn floor(self) -> Self;
    /// Sine, in radians.
    #[must_use]
    fn sin(self) -> Self;
    /// Cosine, in radians.
    #[must_use]
    fn cos(self) -> Self;
    /// `sqrt(self² + other²)` without overflow.
    #[must_use]
    fn hypot(self, other: Self) -> Self;
    /// The least non-negative remainder of `self / rhs`.
    #[must_use]
    fn rem_euclid(self, rhs: Self) -> Self;
}

impl Float for f32 {
    fn round(self) -> Self {
        libm::roundf(self)
    }
    fn floor(self) -> Self {
        libm::floorf(self)
    }
    fn sin(self) -> Self {
        libm::sinf(self)
    }
    fn cos(self) -> Self {
        libm::cosf(self)
    }
    fn hypot(self, other: Self) -> Self {
        libm::hypotf(self, other)
    }
    fn rem_euclid(self, rhs: Self) -> Self {
        // As `std` writes it.
        let r = self % rhs;
        if r < 0.0 { r + rhs.abs() } else { r }
    }
}

/// The one `f64` method the game calls that needs `std`.
pub trait Round {
    /// Nearest integer, halves away from zero.
    #[must_use]
    fn round(self) -> Self;
}

impl Round for f64 {
    fn round(self) -> Self {
        libm::round(self)
    }
}

#[cfg(test)]
mod tests {
    use super::{Float, Round};
    use proptest::prelude::*;

    /// The very same number, down to the sign of a zero.
    fn same(a: f32, b: f32) -> bool {
        a.to_bits() == b.to_bits()
    }

    /// `sin`, `cos` and `hypot` may differ from the platform's in the last
    /// bit; rounding and remainders must not differ at all.
    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() <= 1e-6 * b.abs().max(1.0)
    }

    #[test]
    fn rounding_matches_std_at_halves_and_signs() {
        for v in [-2.5_f32, -1.5, -0.5, -0.4, 0.0, 0.4, 0.5, 1.5, 2.5, 7.99] {
            assert!(same(Float::round(v), v.round()), "round {v}");
            assert!(same(Float::floor(v), v.floor()), "floor {v}");
            let w = f64::from(v);
            assert_eq!(Round::round(w).to_bits(), w.round().to_bits(), "round {v}");
        }
    }

    #[test]
    fn rem_euclid_matches_std_at_signs_and_zero() {
        for (v, w) in [
            (-1.0_f32, 4.0),
            (-1.0, -4.0),
            (5.0, 4.0),
            (-4.0, 4.0),
            (0.5, 0.25),
        ] {
            assert!(same(Float::rem_euclid(v, w), v.rem_euclid(w)), "{v} % {w}");
        }
    }

    proptest! {
        #[test]
        fn matches_std(v in -1000.0_f32..1000.0, w in -1000.0_f32..1000.0) {
            prop_assert!(same(Float::round(v), v.round()));
            prop_assert!(same(Float::floor(v), v.floor()));
            let x = f64::from(v) * 3.7;
            prop_assert_eq!(Round::round(x).to_bits(), x.round().to_bits());
            prop_assert!(close(Float::sin(v / 100.0), (v / 100.0).sin()));
            prop_assert!(close(Float::cos(v / 100.0), (v / 100.0).cos()));
            prop_assert!(close(Float::hypot(v, w), v.hypot(w)));
            if w != 0.0 {
                prop_assert!(same(Float::rem_euclid(v, w), v.rem_euclid(w)));
            }
        }
    }
}
