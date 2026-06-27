//! Display-unit formatting for parameter values.
//!
//! Turns a raw device [`Value`] into the human-readable form shown to the user.
//! Most parameters display as their raw number or enum label; many carry a unit or
//! an offset (a percentage, a signed gain in dB, a time in ms, a pitch in
//! semitones), applied here. The catalog ranges stay raw -- this is the
//! presentation layer.

use crate::param::{Kind, Param, Value};

/// Format `value` for `param` in display units.
#[must_use]
pub fn display(param: Param, value: Value) -> String {
    match value {
        Value::Bool(on) => if on { "on" } else { "off" }.to_owned(),
        Value::Int(raw) => format_int(param.key(), raw),
        Value::Enum(index) => enum_label(param, index),
    }
}

/// Format an integer value, applying the parameter's display unit/offset.
fn format_int(key: &str, raw: i32) -> String {
    // Harmonist scale (36 bytes) and interval voices: an interval in semitones,
    // raw 0..48 centred at 24.
    if key.starts_with("mod-hr-scale-")
        || matches!(
            key,
            "mod-harmonist-interval1" | "mod-harmonist-interval2" | "mod-harmonist-interval3"
        )
    {
        return signed(raw - 24);
    }
    match key {
        // Percentages: output level and the delay L/R times (% of the centre tap).
        "output-level" | "delay-time-l" | "delay-time-r" => format!("{raw}%"),
        // Centred at 50 (raw 0..100 shown as -50..+50): tones and pitch-shifter fine.
        "comp-tone" | "dist-bass" | "dist-treble" | "mod-ps-fine1" | "mod-ps-fine2"
        | "mod-ps-fine3" => signed(raw - 50),
        // EQ gains: raw 0..40 = -20..+20 dB.
        "eq-low-gain" | "eq-mid-gain" | "eq-high-gain" | "eq-level" => {
            format!("{} dB", signed(raw - 20))
        }
        // Delay high damp: raw 0..50 = -50..0 dB.
        "delay-high-damp" => format!("{} dB", signed(raw - 50)),
        // Delay centre time, in milliseconds.
        "delay-time-c" => format!("{raw} ms"),
        // Delay tempo: raw 0..250 = 50..300 BPM.
        "delay-tempo" => format!("{} BPM", raw + 50),
        // Reverb time: raw 1..100 = 0.1..10.0 s (tenths of a second).
        "reverb-time" => format!("{}.{} s", raw / 10, raw % 10),
        // Chorus pre-delay: raw 0..100 in half-millisecond steps (0.0..50.0 ms).
        "chorus-pre-delay" => format!("{}.{} ms", raw / 2, (raw % 2) * 5),
        _ => raw.to_string(),
    }
}

/// A signed value with an explicit sign, but a bare `0` for the centre.
fn signed(v: i32) -> String {
    if v == 0 {
        "0".to_owned()
    } else {
        format!("{v:+}")
    }
}

/// The label for an enum `index`, or the bare index if out of range / not an enum.
fn enum_label(param: Param, index: i32) -> String {
    if let Kind::Enum { values, .. } = param.kind()
        && let Some(label) = usize::try_from(index).ok().and_then(|i| values.get(i))
    {
        return (*label).to_owned();
    }
    index.to_string()
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]
    use super::*;
    use crate::param::Param;

    fn fmt(key: &str, raw: i32) -> String {
        display(Param::from_key(key).expect("known key"), Value::Int(raw))
    }

    #[test]
    fn display_units_cover_the_families() {
        assert_eq!(fmt("output-level", 80), "80%");
        assert_eq!(fmt("comp-tone", 50), "0"); // centre
        assert_eq!(fmt("comp-tone", 70), "+20");
        assert_eq!(fmt("comp-tone", 30), "-20");
        assert_eq!(fmt("eq-low-gain", 20), "0 dB");
        assert_eq!(fmt("eq-mid-gain", 40), "+20 dB");
        assert_eq!(fmt("delay-high-damp", 0), "-50 dB");
        assert_eq!(fmt("delay-time-c", 1234), "1234 ms");
        assert_eq!(fmt("delay-time-l", 200), "200%");
        assert_eq!(fmt("delay-tempo", 70), "120 BPM");
        assert_eq!(fmt("reverb-time", 50), "5.0 s");
        assert_eq!(fmt("chorus-pre-delay", 3), "1.5 ms");
        assert_eq!(fmt("mod-harmonist-interval1", 24), "0");
        assert_eq!(fmt("mod-hr-scale-c1", 36), "+12");
        assert_eq!(fmt("mod-hr-scale-b3", 12), "-12");
        // A plain 0..100 value keeps its raw form.
        assert_eq!(fmt("preamp-volume", 75), "75");
    }
}
