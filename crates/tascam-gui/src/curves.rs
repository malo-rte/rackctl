//! Pure math for the channel editor's EQ-response and compressor-transfer
//! curves, plus the raw-control-value to human-unit conversions.
//!
//! The compressor transfer is exact. The EQ response is *indicative*: it sums
//! analytic peaking-filter responses, and the per-band centre frequency is
//! mapped from the raw control index over a nominal range (only the LOW band's
//! 32 Hz-1.6 kHz span is confirmed by the manual; the others are approximate).
//! See `docs/signal-chain.adoc`.
#![allow(clippy::cast_precision_loss)]

/// One EQ band as a peaking filter.
#[derive(Debug, Clone, Copy)]
pub(crate) struct EqBand {
    /// Centre frequency in Hz.
    pub f0: f64,
    /// Quality factor.
    pub q: f64,
    /// Gain in dB (0 = flat).
    pub gain_db: f64,
}

/// Magnitude (dB) of one analog peaking filter at frequency `f`.
///
/// Uses the RBJ peaking biquad magnitude; at `f == f0` it equals `gain_db`, and
/// a zero-gain band contributes nothing.
pub(crate) fn peaking_db(band: &EqBand, f: f64) -> f64 {
    if band.gain_db.abs() < f64::EPSILON || band.f0 <= 0.0 || band.q <= 0.0 {
        return 0.0;
    }
    let a = 10f64.powf(band.gain_db / 40.0);
    let x = f / band.f0;
    let base = (1.0 - x * x).powi(2);
    let num = base + (a * x / band.q).powi(2);
    let den = base + (x / (a * band.q)).powi(2);
    10.0 * (num / den).log10()
}

/// Summed EQ response (dB) of all bands at frequency `f`.
pub(crate) fn eq_response_db(bands: &[EqBand], f: f64) -> f64 {
    bands.iter().map(|b| peaking_db(b, f)).sum()
}

/// Map a raw frequency-control index to Hz, log-spaced over `lo..=hi`.
pub(crate) fn log_freq(raw: i32, max_raw: i32, lo: f64, hi: f64) -> f64 {
    if max_raw <= 0 {
        return lo;
    }
    let t = (f64::from(raw) / f64::from(max_raw)).clamp(0.0, 1.0);
    lo * (hi / lo).powf(t)
}

/// Map a raw Q-control index (0..=6) to a Q factor, log-spaced over 0.25..=16.
pub(crate) fn q_value(raw: i32) -> f64 {
    0.25 * (16.0_f64 / 0.25).powf((f64::from(raw) / 6.0).clamp(0.0, 1.0))
}

/// EQ band gain in dB for a raw `*-volume` value (0..=24, 12 = 0 dB).
pub(crate) fn eq_gain_db(raw: i32) -> f64 {
    f64::from(raw - 12)
}

/// Parse a compressor ratio label (`"2.0:1"`, `"inf:1"`) into a numeric ratio.
pub(crate) fn ratio_from_label(label: &str) -> f64 {
    let head = label.split(':').next().unwrap_or("1");
    if head.eq_ignore_ascii_case("inf") {
        f64::INFINITY
    } else {
        head.parse().unwrap_or(1.0)
    }
}

/// Compressor output level (dB) for an input level (dB): unity below threshold,
/// `ratio`-compressed above it, then make-up gain.
pub(crate) fn comp_output_db(input_db: f64, threshold_db: f64, ratio: f64, makeup_db: f64) -> f64 {
    let shaped = if input_db <= threshold_db {
        input_db
    } else {
        threshold_db + (input_db - threshold_db) / ratio
    };
    shaped + makeup_db
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-6
    }

    #[test]
    fn peaking_is_flat_at_zero_gain() {
        let band = EqBand {
            f0: 1000.0,
            q: 1.0,
            gain_db: 0.0,
        };
        assert!(close(peaking_db(&band, 1000.0), 0.0));
        assert!(close(peaking_db(&band, 100.0), 0.0));
    }

    #[test]
    fn peaking_hits_gain_at_centre() {
        let band = EqBand {
            f0: 1000.0,
            q: 2.0,
            gain_db: 6.0,
        };
        assert!((peaking_db(&band, 1000.0) - 6.0).abs() < 1e-6);
        // Far from centre it returns toward flat.
        assert!(peaking_db(&band, 20.0).abs() < 1.0);
    }

    #[test]
    fn eq_response_sums_bands() {
        let bands = [
            EqBand {
                f0: 100.0,
                q: 1.0,
                gain_db: 4.0,
            },
            EqBand {
                f0: 100.0,
                q: 1.0,
                gain_db: 3.0,
            },
        ];
        assert!((eq_response_db(&bands, 100.0) - 7.0).abs() < 1e-6);
    }

    #[test]
    fn log_freq_spans_the_range() {
        assert!(close(log_freq(0, 31, 32.0, 1600.0), 32.0));
        assert!(close(log_freq(31, 31, 32.0, 1600.0), 1600.0));
    }

    #[test]
    fn ratio_parsing() {
        assert!(close(ratio_from_label("2.0:1"), 2.0));
        assert!(ratio_from_label("inf:1").is_infinite());
        assert!(close(ratio_from_label("1.0:1"), 1.0));
    }

    #[test]
    fn compressor_transfer() {
        // Below threshold: unity (no make-up).
        assert!(close(comp_output_db(-40.0, -20.0, 4.0, 0.0), -40.0));
        // 8 dB over a -20 dB threshold at 4:1 -> 2 dB over -> -18 dB.
        assert!(close(comp_output_db(-12.0, -20.0, 4.0, 0.0), -18.0));
        // inf:1 limits to the threshold.
        assert!(close(comp_output_db(0.0, -20.0, f64::INFINITY, 0.0), -20.0));
        // Make-up gain adds.
        assert!(close(comp_output_db(-40.0, -20.0, 4.0, 3.0), -37.0));
    }
}
