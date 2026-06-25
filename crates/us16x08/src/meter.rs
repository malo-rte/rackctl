//! Level-meter readout.
//!
//! The `Level Meter` control returns a block of 34 integers. The layout is
//! documented from how `OMainWnd::on_notification_from_worker_thread` indexes
//! `alsa->meters[]`:
//!
//! | index   | meaning                                   |
//! |---------|-------------------------------------------|
//! | `0..16` | per-channel input levels                  |
//! | `16`    | master output level, left                 |
//! | `17`    | master output level, right                |
//! | `18..34`| per-channel compressor gain reduction     |
//!
//! Raw samples are linear `0..=32767`; the `*_db` accessors apply
//! [`crate::convert::meter_scale`].

use crate::control::NUM_CHANNELS;
use crate::convert::meter_scale;

/// Number of integers in the `Level Meter` block.
pub const METER_COUNT: usize = 34;

const MASTER_LEFT: usize = 16;
const MASTER_RIGHT: usize = 17;
const REDUCTION_BASE: usize = 18;

/// A snapshot of the level meters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Meters {
    raw: [i32; METER_COUNT],
}

impl Meters {
    /// Wrap a raw 34-integer meter block.
    #[must_use]
    pub const fn from_raw(raw: [i32; METER_COUNT]) -> Self {
        Self { raw }
    }

    /// The underlying raw block.
    #[must_use]
    pub const fn raw(&self) -> &[i32; METER_COUNT] {
        &self.raw
    }

    /// Raw input level for `channel` (`0..16`), if in range.
    #[must_use]
    pub fn channel_raw(&self, channel: u32) -> Option<i32> {
        if channel >= NUM_CHANNELS {
            return None;
        }
        self.raw.get(channel as usize).copied()
    }

    /// Scaled (dB-curve) input level for `channel` (`0..16`), if in range.
    #[must_use]
    pub fn channel_db(&self, channel: u32) -> Option<i32> {
        self.channel_raw(channel).map(meter_scale)
    }

    /// Raw compressor meter for `channel` (`0..16`), if in range. This index is
    /// the compressor's gain *coefficient* (full scale = unity gain, i.e. no
    /// reduction); see [`Self::gain_reduction`] for the reduction itself.
    #[must_use]
    pub fn reduction_raw(&self, channel: u32) -> Option<i32> {
        if channel >= NUM_CHANNELS {
            return None;
        }
        self.raw.get(REDUCTION_BASE + channel as usize).copied()
    }

    /// Compressor gain reduction for `channel` as a `0.0..=1.0` fraction (0 = no
    /// reduction, 1 = full).
    ///
    /// The device reports the compressor's current *gain coefficient* at this
    /// meter index (full scale is unity gain, i.e. no reduction), so the
    /// reduction is its complement.
    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    pub fn gain_reduction(&self, channel: u32) -> Option<f32> {
        self.reduction_raw(channel)
            .map(|raw| (1.0 - raw.max(0) as f32 / 32768.0).clamp(0.0, 1.0))
    }

    /// Raw master output level (left, right).
    #[must_use]
    pub fn master_raw(&self) -> (i32, i32) {
        (
            self.raw.get(MASTER_LEFT).copied().unwrap_or(0),
            self.raw.get(MASTER_RIGHT).copied().unwrap_or(0),
        )
    }

    /// Scaled master output level (left, right).
    #[must_use]
    pub fn master_db(&self) -> (i32, i32) {
        let (l, r) = self.master_raw();
        (meter_scale(l), meter_scale(r))
    }
}

impl Default for Meters {
    fn default() -> Self {
        Self::from_raw([0; METER_COUNT])
    }
}
