//! Ease curves for a skin that works in floats: [`ease_f32`] eases through
//! `shuyi_core`'s integer curves ([`shuyi_core::ease`]), so a curve is the
//! same on every machine.

#[cfg(not(feature = "std"))]
use crate::float::Float;
use shuyi_core::ease::Easing;

/// [`shuyi_core::ease::ONE`], the whole way, as an `f32`.
const ONE: f32 = 65_536.0;

/// `p` (0 to 1; clamped, not a number counts as 0) eased by `ease`: turned
/// into 1/65536, eased by the integer curve and turned back. A curve that
/// overshoots (`Back`, `Elastic`) gives values below 0 and above 1.
///
/// ```
/// use shuyi_core::{Ease, Family};
/// use shuyi_ui::ease::ease_f32;
///
/// assert_eq!(ease_f32(&Ease::Smooth, 0.5), 0.5);
/// assert!(ease_f32(&Ease::Out(Family::Back), 0.6) > 1.0);
/// ```
pub fn ease_f32(ease: &impl Easing, p: f32) -> f32 {
    let p = if p.is_nan() { 0.0 } else { p.clamp(0.0, 1.0) };
    // At most 65 536, which an `f32` and an `i32` hold exactly.
    #[expect(clippy::cast_possible_truncation, reason = "0 to 65 536")]
    let units = (p * ONE).round() as i32;
    #[expect(
        clippy::cast_precision_loss,
        reason = "every curve stays between -65 536 and 131 072"
    )]
    let eased = ease.ease(units) as f32;
    eased / ONE
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cinematic::ease_in_out;
    use shuyi_core::{Ease, Family};

    #[test]
    fn smooth_is_the_cinematics_ease_in_out() {
        for k in 0..=10_000_u16 {
            let p = f32::from(k) / 10_000.0;
            let off = (ease_f32(&Ease::Smooth, p) - ease_in_out(p)).abs();
            assert!(off <= 2.0 / 65_536.0, "{p}: {off}");
        }
    }

    #[test]
    fn not_a_number_and_input_out_of_range_are_clamped() {
        let curve = Ease::In(Family::Cubic);
        assert_eq!(ease_f32(&curve, f32::NAN), 0.0);
        assert_eq!(ease_f32(&curve, -3.0), 0.0);
        assert_eq!(ease_f32(&curve, f32::NEG_INFINITY), 0.0);
        assert_eq!(ease_f32(&curve, 7.0), 1.0);
        assert_eq!(ease_f32(&curve, f32::INFINITY), 1.0);
        assert_eq!(ease_f32(&curve, 0.5), 0.125);
    }

    #[test]
    fn an_overshoot_goes_past_0_and_1() {
        assert!(ease_f32(&Ease::In(Family::Back), 0.25) < 0.0);
        assert!(ease_f32(&Ease::Out(Family::Elastic), 0.1) > 1.0);
    }
}
