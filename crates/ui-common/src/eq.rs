//! Pure EQ-response math, shared by the device GUIs' curve plots.
//!
//! Each band is modelled as an RBJ "Audio EQ Cookbook" *digital biquad* — a low
//! shelf, a peaking filter (with Q), or a high shelf — evaluated at an assumed
//! 48 kHz DSP rate, so the filter shapes and the digital warping toward Nyquist
//! are faithful. Raw-control-value to dB/Hz/Q conversions are device-specific and
//! stay in each GUI; this module is just the band magnitude response.
#![allow(clippy::cast_precision_loss)]

use std::f64::consts::{SQRT_2, TAU};

/// Assumed internal DSP sample rate (Hz); only affects the response near Nyquist.
const SAMPLE_RATE: f64 = 48_000.0;

/// The filter shape of an EQ band.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BandType {
    /// Low band: a low-shelf filter.
    LowShelf,
    /// Mid band: a peaking filter with a Q control.
    Peaking,
    /// High band: a high-shelf filter.
    HighShelf,
}

/// One equalizer band.
#[derive(Debug, Clone, Copy)]
pub struct EqBand {
    /// The filter shape (shelf or peak).
    pub kind: BandType,
    /// Centre frequency (peaking) or corner frequency (shelf) in Hz.
    pub f0: f64,
    /// Quality factor (peaking bands only; ignored by the shelves).
    pub q: f64,
    /// Gain in dB (0 = flat).
    pub gain_db: f64,
}

/// Magnitude (dB) of one band's RBJ digital biquad at frequency `f`, evaluated at
/// the assumed 48 kHz DSP rate. A peaking band hits `gain_db` exactly at `f0`; a
/// zero-gain band of any shape contributes nothing.
#[must_use]
pub fn band_db(band: &EqBand, f: f64) -> f64 {
    biquad_db(band, f, SAMPLE_RATE)
}

/// Magnitude (dB) of `band`'s biquad at frequency `f`, sampled at `fs`.
fn biquad_db(band: &EqBand, f: f64, fs: f64) -> f64 {
    if band.f0 <= 0.0 || band.q <= 0.0 || fs <= 0.0 {
        return 0.0;
    }
    // RBJ cookbook coefficients. `a` is sqrt(linear gain); `w0` the band's
    // normalised angular frequency; `alpha` sets the bandwidth (Q for peaks, a
    // fixed unity-slope shelf for the shelves, which have no Q control).
    let a = 10f64.powf(band.gain_db / 40.0);
    let w0 = TAU * band.f0 / fs;
    let (sin_w0, cos_w0) = w0.sin_cos();
    let (b, den) = match band.kind {
        BandType::Peaking => {
            let alpha = sin_w0 / (2.0 * band.q);
            (
                [1.0 + alpha * a, -2.0 * cos_w0, 1.0 - alpha * a],
                [1.0 + alpha / a, -2.0 * cos_w0, 1.0 - alpha / a],
            )
        }
        BandType::LowShelf => {
            let alpha = sin_w0 / 2.0 * SQRT_2;
            let (ap1, am1) = (a + 1.0, a - 1.0);
            let tsa = 2.0 * a.sqrt() * alpha;
            (
                [
                    a * (ap1 - am1 * cos_w0 + tsa),
                    2.0 * a * (am1 - ap1 * cos_w0),
                    a * (ap1 - am1 * cos_w0 - tsa),
                ],
                [
                    ap1 + am1 * cos_w0 + tsa,
                    -2.0 * (am1 + ap1 * cos_w0),
                    ap1 + am1 * cos_w0 - tsa,
                ],
            )
        }
        BandType::HighShelf => {
            let alpha = sin_w0 / 2.0 * SQRT_2;
            let (ap1, am1) = (a + 1.0, a - 1.0);
            let tsa = 2.0 * a.sqrt() * alpha;
            (
                [
                    a * (ap1 + am1 * cos_w0 + tsa),
                    -2.0 * a * (am1 + ap1 * cos_w0),
                    a * (ap1 + am1 * cos_w0 - tsa),
                ],
                [
                    ap1 - am1 * cos_w0 + tsa,
                    2.0 * (am1 - ap1 * cos_w0),
                    ap1 - am1 * cos_w0 - tsa,
                ],
            )
        }
    };
    let w = TAU * f / fs;
    let (num_mag, den_mag) = (poly_mag(b, w), poly_mag(den, w));
    if num_mag <= 0.0 || den_mag <= 0.0 {
        return 0.0;
    }
    20.0 * (num_mag / den_mag).log10()
}

/// Magnitude of `c0 + c1 z^-1 + c2 z^-2` on the unit circle at angle `w`.
fn poly_mag(c: [f64; 3], w: f64) -> f64 {
    let (s1, c1) = w.sin_cos();
    let (s2, c2) = (2.0 * w).sin_cos();
    let re = c[0] + c[1] * c1 + c[2] * c2;
    let im = -(c[1] * s1 + c[2] * s2);
    re.hypot(im)
}

/// Summed EQ response (dB) of the band cascade at frequency `f`. Cascaded biquads
/// multiply in magnitude, i.e. add in dB.
#[must_use]
pub fn eq_response_db(bands: &[EqBand], f: f64) -> f64 {
    bands.iter().map(|b| band_db(b, f)).sum()
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-6
    }

    fn peaking(f0: f64, q: f64, gain_db: f64) -> EqBand {
        EqBand {
            kind: BandType::Peaking,
            f0,
            q,
            gain_db,
        }
    }

    #[test]
    fn flat_at_zero_gain_for_every_shape() {
        for kind in [BandType::LowShelf, BandType::Peaking, BandType::HighShelf] {
            let band = EqBand {
                kind,
                f0: 1000.0,
                q: 1.0,
                gain_db: 0.0,
            };
            assert!(close(band_db(&band, 1000.0), 0.0), "{kind:?} at f0");
            assert!(close(band_db(&band, 100.0), 0.0), "{kind:?} off f0");
        }
    }

    #[test]
    fn peaking_hits_gain_at_centre() {
        let band = peaking(1000.0, 2.0, 6.0);
        assert!((band_db(&band, 1000.0) - 6.0).abs() < 1e-6);
        assert!(band_db(&band, 20.0).abs() < 1.0);
    }

    #[test]
    fn low_shelf_lifts_the_bottom_only() {
        let band = EqBand {
            kind: BandType::LowShelf,
            f0: 200.0,
            q: 0.7,
            gain_db: 6.0,
        };
        assert!((band_db(&band, 10.0) - 6.0).abs() < 0.5);
        assert!(band_db(&band, 20_000.0).abs() < 0.5);
    }

    #[test]
    fn high_shelf_lifts_the_top_only() {
        let band = EqBand {
            kind: BandType::HighShelf,
            f0: 5000.0,
            q: 0.7,
            gain_db: 6.0,
        };
        assert!((band_db(&band, 23_000.0) - 6.0).abs() < 0.5);
        assert!(band_db(&band, 100.0).abs() < 0.5);
    }

    #[test]
    fn eq_response_sums_bands() {
        let bands = [peaking(100.0, 1.0, 4.0), peaking(100.0, 1.0, 3.0)];
        assert!((eq_response_db(&bands, 100.0) - 7.0).abs() < 1e-6);
    }
}
