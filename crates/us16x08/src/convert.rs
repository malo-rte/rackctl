//! Value conversions between hardware units and human units.
//!
//! Ported verbatim from `OAlsa::sliderTodB` / `OAlsa::dBToSlider` and the
//! meter-scaling expression in `OMainWnd::on_notification_from_worker_thread`.
//! The hardware fader/volume controls store a value on a logarithmic curve
//! rather than a linear dB scale.

// The conversions intentionally truncate toward zero to match the original C
// `int` casts; the values are small and well within i32.
#![allow(clippy::cast_possible_truncation)]

/// Convert a 0..=127 fader/slider position to the hardware dB value.
///
/// Mirrors `OAlsa::sliderTodB`: `146.2 - 146.3 / 10^(pos/127)`.
#[must_use]
pub fn slider_to_db(pos: i32) -> i32 {
    (146.2 - 146.3 / 10f64.powf(f64::from(pos) / 127.0)) as i32
}

/// Inverse of [`slider_to_db`]: convert a hardware dB value back to a slider
/// position.
///
/// Mirrors `OAlsa::dBToSlider`: `127 * log10(146.3 / (146.2 - dB))`.
#[must_use]
pub fn db_to_slider(db: i32) -> i32 {
    (127.0 * (146.3 / (146.2 - f64::from(db))).log10()) as i32
}

/// Scale a raw level-meter sample (0..=32767, linear) onto the logarithmic
/// display range used by the original meters.
///
/// Mirrors `sliderTodB(raw / 32768 * 133) / 133 * 32768` from `OMainWnd`.
#[must_use]
pub fn meter_scale(raw: i32) -> i32 {
    let pos = ((f64::from(raw) / 32768.0) * 133.0) as i32;
    let db = slider_to_db(pos);
    ((f64::from(db) / 133.0) * 32768.0) as i32
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;

    #[test]
    fn slider_to_db_is_monotonic_nondecreasing() {
        let mut prev = slider_to_db(0);
        for pos in 1..=127 {
            let v = slider_to_db(pos);
            assert!(v >= prev, "pos={pos} v={v} prev={prev}");
            prev = v;
        }
    }

    #[test]
    fn slider_to_db_matches_reference_points() {
        // Hand-computed from `146.2 - 146.3 / 10^(pos/127)`, truncated to int --
        // these lock the port to the original `OAlsa::sliderTodB`.
        assert_eq!(slider_to_db(0), 0);
        assert_eq!(slider_to_db(32), 64);
        assert_eq!(slider_to_db(64), 100);
        assert_eq!(slider_to_db(96), 120);
        assert_eq!(slider_to_db(127), 131);
    }

    #[test]
    fn db_to_slider_matches_reference_points() {
        // Hand-computed from `127 * log10(146.3 / (146.2 - dB))`, truncated.
        assert_eq!(db_to_slider(0), 0);
        assert_eq!(db_to_slider(64), 31);
        assert_eq!(db_to_slider(100), 63);
    }

    #[test]
    fn slider_db_roundtrip_stays_bounded() {
        // The curve steepens sharply toward the top, where many slider
        // positions collapse onto the same integer dB; round-tripping is only
        // approximate there. Guard against gross errors, not exact recovery.
        for pos in 1..=127 {
            let back = db_to_slider(slider_to_db(pos));
            assert!((back - pos).abs() <= 6, "pos={pos} -> back={back}");
        }
    }

    #[test]
    fn meter_scale_is_monotonic_nondecreasing() {
        let mut prev = meter_scale(0);
        for raw in (0..=32767).step_by(257) {
            let v = meter_scale(raw);
            assert!(v >= prev, "raw={raw} v={v} prev={prev}");
            prev = v;
        }
    }
}
