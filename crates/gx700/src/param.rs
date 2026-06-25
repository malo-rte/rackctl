//! The GX-700 parameter catalog.
//!
//! Each [`Param`] names one editable value in a GX-700 effect block, carrying a
//! value [`Kind`] (bool / bounded int / enum) and a provisional one-byte `SysEx`
//! address.
//!
//! # Units
//!
//! Every value in this catalog is a **raw 7-bit device unit** (`0..=127`). The
//! display-unit conversions the front panel shows (for example a tone control
//! displayed as `-50..+50`, or a delay time in milliseconds) are **deferred to
//! Stage 2**, where the catalog is verified against real hardware. Until then a
//! parameter's range is its raw device range, not its display range.
//!
//! # Provisional
//!
//! The blocks below are decoded from the byte layout in the reverse-engineered
//! `boss-gx-700-patch-parser`, but the addresses and ranges here are a
//! representative first draft. They are firmed up against the hardware in
//! Stage 2.

// PROVISIONAL: addresses/ranges verified against hardware in Stage 2.

/// The value kind of a parameter, with its raw device range/defaults.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Kind {
    /// An on/off switch (typically a block enable).
    Bool,
    /// A bounded integer (inclusive `min..=max`), in raw device units.
    Int {
        /// Inclusive minimum accepted by the hardware.
        min: i32,
        /// Inclusive maximum accepted by the hardware.
        max: i32,
        /// Power-on / reset default.
        default: i32,
    },
    /// An enumerated choice; the value is an index into `values`.
    Enum {
        /// Display labels in value order.
        values: &'static [&'static str],
        /// Default value index.
        default: i32,
    },
}

/// A concrete parameter value, tagged by kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Value {
    /// Boolean switch value.
    Bool(bool),
    /// Bounded integer value, in raw device units.
    Int(i32),
    /// Enum choice, as an index into the parameter's value list.
    Enum(i32),
}

/// Labels for the common overdrive/distortion model selector.
pub const OD_TYPE_VALUES: &[&str] = &["Overdrive", "Distortion", "Fuzz", "Turbo OD"];
/// Labels for the preamp model selector.
pub const PREAMP_TYPE_VALUES: &[&str] = &[
    "JC-120",
    "Clean Twin",
    "Match Drive",
    "BG Lead",
    "MS1959 I",
    "MS1959 II",
    "SLDN Lead",
    "Metal 5150",
    "Metal Lead",
];
/// Labels for the speaker-simulator cabinet selector.
pub const SPEAKER_TYPE_VALUES: &[&str] = &[
    "Small",
    "Middle",
    "JC-120",
    "Built-In 1",
    "Built-In 2",
    "BG Stack",
    "MS Stack",
    "Metal Stack",
];
/// Labels for the chorus mode selector.
pub const CHORUS_MODE_VALUES: &[&str] = &["Mono", "Stereo"];
/// Labels for the wah mode selector.
pub const WAH_MODE_VALUES: &[&str] = &["Manual", "Auto"];

/// One editable GX-700 parameter.
#[derive(Debug, Clone, Copy)]
pub struct Param {
    key: &'static str,
    block: &'static str,
    kind: Kind,
    addr: u8,
}

impl Param {
    /// The stable kebab-case key used by the CLI and patch files.
    #[must_use]
    pub const fn key(self) -> &'static str {
        self.key
    }

    /// The effect block this parameter belongs to, for grouping in listings.
    #[must_use]
    pub const fn block(self) -> &'static str {
        self.block
    }

    /// The parameter's value kind and raw range.
    #[must_use]
    pub const fn kind(self) -> Kind {
        self.kind
    }

    /// The parameter's provisional one-byte `SysEx` address.
    #[must_use]
    pub const fn addr(self) -> u8 {
        self.addr
    }

    /// Look up a parameter by its [`Self::key`].
    #[must_use]
    pub fn from_key(key: &str) -> Option<Param> {
        ALL.iter().copied().find(|p| p.key == key)
    }
}

/// Convenience constructor for a bool (block enable) parameter.
const fn b(key: &'static str, block: &'static str, addr: u8) -> Param {
    Param {
        key,
        block,
        kind: Kind::Bool,
        addr,
    }
}

/// Convenience constructor for a `0..=100` integer parameter, default 50.
const fn i100(key: &'static str, block: &'static str, addr: u8) -> Param {
    Param {
        key,
        block,
        kind: Kind::Int {
            min: 0,
            max: 100,
            default: 50,
        },
        addr,
    }
}

/// Convenience constructor for an enum parameter (default index 0).
const fn e(
    key: &'static str,
    block: &'static str,
    values: &'static [&'static str],
    addr: u8,
) -> Param {
    Param {
        key,
        block,
        kind: Kind::Enum { values, default: 0 },
        addr,
    }
}

// PROVISIONAL: addresses are sequential placeholders; ranges are raw 7-bit
// guesses. Both are verified against hardware in Stage 2.
/// Every cataloged parameter, in a stable order. Used for enumeration, mock
/// seeding, CLI listings, and patch capture/apply.
pub const ALL: &[Param] = &[
    // Compressor.
    b("comp-enable", "Compressor", 0x00),
    i100("comp-sustain", "Compressor", 0x01),
    i100("comp-attack", "Compressor", 0x02),
    i100("comp-level", "Compressor", 0x03),
    // Wah.
    b("wah-enable", "Wah", 0x04),
    e("wah-mode", "Wah", WAH_MODE_VALUES, 0x05),
    i100("wah-freq", "Wah", 0x06),
    i100("wah-level", "Wah", 0x07),
    // Overdrive / Distortion.
    b("od-enable", "Overdrive/Distortion", 0x08),
    e("od-type", "Overdrive/Distortion", OD_TYPE_VALUES, 0x09),
    i100("od-drive", "Overdrive/Distortion", 0x0A),
    i100("od-tone", "Overdrive/Distortion", 0x0B),
    i100("od-level", "Overdrive/Distortion", 0x0C),
    // Preamp.
    b("preamp-enable", "Preamp", 0x0D),
    e("preamp-type", "Preamp", PREAMP_TYPE_VALUES, 0x0E),
    i100("preamp-gain", "Preamp", 0x0F),
    i100("preamp-bass", "Preamp", 0x10),
    i100("preamp-middle", "Preamp", 0x11),
    i100("preamp-treble", "Preamp", 0x12),
    i100("preamp-presence", "Preamp", 0x13),
    i100("preamp-level", "Preamp", 0x14),
    // Speaker simulator.
    b("speaker-enable", "Speaker Sim", 0x15),
    e("speaker-type", "Speaker Sim", SPEAKER_TYPE_VALUES, 0x16),
    i100("speaker-mic", "Speaker Sim", 0x17),
    i100("speaker-level", "Speaker Sim", 0x18),
    // EQ.
    b("eq-enable", "EQ", 0x19),
    i100("eq-low", "EQ", 0x1A),
    i100("eq-low-mid", "EQ", 0x1B),
    i100("eq-high-mid", "EQ", 0x1C),
    i100("eq-high", "EQ", 0x1D),
    i100("eq-level", "EQ", 0x1E),
    // Noise suppressor.
    b("ns-enable", "Noise Suppressor", 0x1F),
    i100("ns-threshold", "Noise Suppressor", 0x20),
    i100("ns-release", "Noise Suppressor", 0x21),
    // Chorus.
    b("chorus-enable", "Chorus", 0x22),
    e("chorus-mode", "Chorus", CHORUS_MODE_VALUES, 0x23),
    i100("chorus-rate", "Chorus", 0x24),
    i100("chorus-depth", "Chorus", 0x25),
    i100("chorus-level", "Chorus", 0x26),
    // Flanger.
    b("flanger-enable", "Flanger", 0x27),
    i100("flanger-rate", "Flanger", 0x28),
    i100("flanger-depth", "Flanger", 0x29),
    i100("flanger-resonance", "Flanger", 0x2A),
    // Phaser.
    b("phaser-enable", "Phaser", 0x2B),
    i100("phaser-rate", "Phaser", 0x2C),
    i100("phaser-depth", "Phaser", 0x2D),
    i100("phaser-resonance", "Phaser", 0x2E),
    // Pitch shifter.
    b("pitch-enable", "Pitch Shifter", 0x2F),
    i100("pitch-shift", "Pitch Shifter", 0x30),
    i100("pitch-fine", "Pitch Shifter", 0x31),
    i100("pitch-level", "Pitch Shifter", 0x32),
    // Delay.
    b("delay-enable", "Delay", 0x33),
    i100("delay-time", "Delay", 0x34),
    i100("delay-feedback", "Delay", 0x35),
    i100("delay-level", "Delay", 0x36),
    // Reverb.
    b("reverb-enable", "Reverb", 0x37),
    i100("reverb-time", "Reverb", 0x38),
    i100("reverb-tone", "Reverb", 0x39),
    i100("reverb-level", "Reverb", 0x3A),
    // Tremolo / Pan.
    b("tremolo-enable", "Tremolo/Pan", 0x3B),
    i100("tremolo-rate", "Tremolo/Pan", 0x3C),
    i100("tremolo-depth", "Tremolo/Pan", 0x3D),
];

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn keys_are_unique() {
        let mut seen = HashSet::new();
        for p in ALL {
            assert!(seen.insert(p.key()), "duplicate key: {}", p.key());
        }
    }

    #[test]
    fn addresses_are_unique() {
        let mut seen = HashSet::new();
        for p in ALL {
            assert!(seen.insert(p.addr()), "duplicate address for {}", p.key());
        }
    }

    #[test]
    fn key_round_trips_through_from_key() {
        for p in ALL {
            assert_eq!(Param::from_key(p.key()).map(Param::key), Some(p.key()));
        }
        assert!(Param::from_key("nonsuch").is_none());
    }
}
