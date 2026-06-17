//! Capturing and restoring mixer / channel-strip state as presets.
//!
//! A [`Preset`] is a serde-(de)serializable snapshot of control values. Two
//! granularities, distinguished by a `kind` tag so a file is self-describing:
//!
//! - **Strip** — one channel's per-channel controls (fader, mute, pan, phase,
//!   EQ, compressor), stored without a channel index so it can be applied to
//!   any channel.
//! - **Mixer** — the global master controls, the per-output routing, and all 16
//!   channel strips.
//!
//! File and format handling lives in the caller (e.g. the CLI); this module only
//! turns device state into a [`Preset`] and back.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::backend::Backend;
use crate::control::{Control, Kind, NUM_CHANNELS, NUM_OUTPUTS, Scope, Value};
use crate::device::Us16x08;
use crate::error::{Error, Result};

/// Schema version written into every preset, for forward compatibility.
pub const PRESET_VERSION: u32 = 1;

/// A single control value as stored in a preset. Enums are kept as their label
/// for readability; integers and booleans use their native JSON types.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Scalar {
    /// A boolean switch value.
    Bool(bool),
    /// An integer value.
    Int(i64),
    /// An enum value, stored as its label.
    Text(String),
}

/// Map of control key ([`Control::cli_key`]) to value.
type ControlMap = BTreeMap<String, Scalar>;

/// A saved snapshot of mixer state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Preset {
    /// One channel's per-channel controls, applicable to any channel.
    Strip {
        /// Schema version ([`PRESET_VERSION`]).
        version: u32,
        /// Per-channel control values, keyed by [`Control::cli_key`].
        controls: ControlMap,
    },
    /// The whole mixer: master globals, routing, and every channel strip.
    Mixer {
        /// Schema version ([`PRESET_VERSION`]).
        version: u32,
        /// Global master controls.
        master: ControlMap,
        /// Routing source per line output, keyed by output index (`"0".."7"`).
        route: BTreeMap<String, Scalar>,
        /// One control map per input channel.
        channels: Vec<ControlMap>,
    },
}

/// Outcome of applying a preset.
#[derive(Debug, Default, Clone)]
pub struct ApplyReport {
    /// How many control values were written.
    pub applied: usize,
    /// Keys that were skipped because the device lacks the control or the key is
    /// unknown (e.g. a preset from a different version).
    pub skipped: Vec<String>,
}

fn to_scalar(control: Control, value: Value) -> Scalar {
    match value {
        Value::Bool(b) => Scalar::Bool(b),
        Value::Int(i) => Scalar::Int(i64::from(i)),
        Value::Enum(i) => {
            if let Kind::Enum { values, .. } = control.kind() {
                if let Some(label) = usize::try_from(i).ok().and_then(|n| values.get(n)) {
                    return Scalar::Text((*label).to_owned());
                }
            }
            Scalar::Int(i64::from(i))
        }
    }
}

fn from_scalar(control: Control, scalar: &Scalar) -> Result<Value> {
    let key = control.cli_key();
    match control.kind() {
        Kind::Bool => match scalar {
            Scalar::Bool(b) => Ok(Value::Bool(*b)),
            _ => Err(Error::Preset(format!("{key}: expected a boolean"))),
        },
        Kind::Int { .. } => match scalar {
            Scalar::Int(n) => i32::try_from(*n)
                .map(Value::Int)
                .map_err(|_| Error::Preset(format!("{key}: value {n} out of range"))),
            _ => Err(Error::Preset(format!("{key}: expected an integer"))),
        },
        Kind::Enum { values, .. } => match scalar {
            Scalar::Text(s) => values
                .iter()
                .position(|v| v.eq_ignore_ascii_case(s))
                .and_then(|i| i32::try_from(i).ok())
                .map(Value::Enum)
                .ok_or_else(|| Error::Preset(format!("{key}: unknown value {s:?}"))),
            Scalar::Int(n) => i32::try_from(*n)
                .map(Value::Enum)
                .map_err(|_| Error::Preset(format!("{key}: value {n} out of range"))),
            Scalar::Bool(_) => Err(Error::Preset(format!("{key}: expected an enum value"))),
        },
        Kind::Meter => Err(Error::Preset(format!("{key}: meter is not settable"))),
    }
}

impl<B: Backend> Us16x08<B> {
    /// Capture one channel's per-channel controls as a [`Preset::Strip`].
    ///
    /// # Errors
    /// Propagates backend read errors.
    pub fn capture_strip(&self, channel: u32) -> Result<Preset> {
        Ok(Preset::Strip {
            version: PRESET_VERSION,
            controls: self.strip_map(channel)?,
        })
    }

    /// Capture the whole mixer (master globals, routing, all channel strips) as a
    /// [`Preset::Mixer`].
    ///
    /// # Errors
    /// Propagates backend read errors.
    pub fn capture_mixer(&self) -> Result<Preset> {
        let mut master = ControlMap::new();
        for &control in Control::ALL {
            if control.scope() == Scope::Global
                && !matches!(control.kind(), Kind::Meter)
                && self.is_present(control)
            {
                master.insert(
                    control.cli_key().to_owned(),
                    to_scalar(control, self.get(control, 0)?),
                );
            }
        }

        let mut route = BTreeMap::new();
        if self.is_present(Control::LineOutRoute) {
            for out in 0..NUM_OUTPUTS {
                let value = self.get(Control::LineOutRoute, out)?;
                route.insert(out.to_string(), to_scalar(Control::LineOutRoute, value));
            }
        }

        let mut channels = Vec::with_capacity(NUM_CHANNELS as usize);
        for ch in 0..NUM_CHANNELS {
            channels.push(self.strip_map(ch)?);
        }

        Ok(Preset::Mixer {
            version: PRESET_VERSION,
            master,
            route,
            channels,
        })
    }

    /// Apply a preset. A [`Preset::Strip`] requires a target `channel`; a
    /// [`Preset::Mixer`] must not be given one.
    ///
    /// Controls absent from this device, and keys it does not recognise, are
    /// skipped and recorded in the returned [`ApplyReport`].
    ///
    /// # Errors
    /// [`Error::Preset`] on a kind/channel mismatch or a value that does not fit
    /// its control; otherwise backend write errors.
    pub fn apply(&mut self, preset: &Preset, channel: Option<u32>) -> Result<ApplyReport> {
        let mut report = ApplyReport::default();
        match preset {
            Preset::Strip { controls, .. } => {
                let ch = channel.ok_or_else(|| {
                    Error::Preset("strip preset requires a target channel".to_owned())
                })?;
                self.apply_map(controls, ch, &mut report)?;
            }
            Preset::Mixer {
                master,
                route,
                channels,
                ..
            } => {
                if channel.is_some() {
                    return Err(Error::Preset(
                        "mixer preset cannot target a single channel".to_owned(),
                    ));
                }
                self.apply_map(master, 0, &mut report)?;
                for (out_key, scalar) in route {
                    if self.is_present(Control::LineOutRoute) {
                        let out: u32 = out_key.parse().map_err(|_| {
                            Error::Preset(format!("invalid route index {out_key:?}"))
                        })?;
                        let value = from_scalar(Control::LineOutRoute, scalar)?;
                        self.set(Control::LineOutRoute, out, value)?;
                        report.applied += 1;
                    } else {
                        report.skipped.push(format!("route[{out_key}]"));
                    }
                }
                for (i, map) in channels.iter().enumerate() {
                    let ch = u32::try_from(i)
                        .map_err(|_| Error::Preset("too many channels in preset".to_owned()))?;
                    self.apply_map(map, ch, &mut report)?;
                }
            }
        }
        Ok(report)
    }

    /// The present, settable per-channel controls at `channel`, as a map.
    fn strip_map(&self, channel: u32) -> Result<ControlMap> {
        let mut map = ControlMap::new();
        for &control in Control::ALL {
            if control.scope() == Scope::Channel
                && !matches!(control.kind(), Kind::Meter)
                && self.is_present(control)
            {
                let value = self.get(control, channel)?;
                map.insert(control.cli_key().to_owned(), to_scalar(control, value));
            }
        }
        Ok(map)
    }

    fn apply_map(&mut self, map: &ControlMap, index: u32, report: &mut ApplyReport) -> Result<()> {
        for (key, scalar) in map {
            let Some(control) = Control::from_key(key) else {
                report.skipped.push(key.clone());
                continue;
            };
            if !self.is_present(control) {
                report.skipped.push(key.clone());
                continue;
            }
            let value = from_scalar(control, scalar)?;
            self.set(control, index, value)?;
            report.applied += 1;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;
    use crate::MockBackend;

    fn dev() -> Us16x08<MockBackend> {
        Us16x08::new(MockBackend::new())
    }

    #[test]
    fn mixer_round_trips_through_a_fresh_device() {
        let mut a = dev();
        a.set(Control::MasterVolume, 0, Value::Int(120)).unwrap();
        a.set(Control::MasterMute, 0, Value::Bool(true)).unwrap();
        a.set(Control::LineOutRoute, 3, Value::Enum(5)).unwrap();
        a.set(Control::EqLowVolume, 7, Value::Int(20)).unwrap();
        a.set(Control::CompRatio, 7, Value::Enum(9)).unwrap();

        let preset = a.capture_mixer().unwrap();

        let mut b = dev();
        let report = b.apply(&preset, None).unwrap();
        assert!(report.skipped.is_empty());
        assert!(report.applied > 0);

        assert_eq!(b.get(Control::MasterVolume, 0).unwrap(), Value::Int(120));
        assert_eq!(b.get(Control::MasterMute, 0).unwrap(), Value::Bool(true));
        assert_eq!(b.get(Control::LineOutRoute, 3).unwrap(), Value::Enum(5));
        assert_eq!(b.get(Control::EqLowVolume, 7).unwrap(), Value::Int(20));
        assert_eq!(b.get(Control::CompRatio, 7).unwrap(), Value::Enum(9));
    }

    #[test]
    fn strip_applies_to_a_different_channel() {
        let mut a = dev();
        a.set(Control::Pan, 2, Value::Int(200)).unwrap();
        a.set(Control::MuteSwitch, 2, Value::Bool(true)).unwrap();
        let strip = a.capture_strip(2).unwrap();

        let mut b = dev();
        b.apply(&strip, Some(5)).unwrap();
        assert_eq!(b.get(Control::Pan, 5).unwrap(), Value::Int(200));
        assert_eq!(b.get(Control::MuteSwitch, 5).unwrap(), Value::Bool(true));
        // Channel 2 on the fresh device is untouched (still the default).
        assert_eq!(b.get(Control::Pan, 2).unwrap(), Value::Int(127));
    }

    #[test]
    fn serde_json_round_trip() {
        let preset = dev().capture_mixer().unwrap();
        let json = serde_json::to_string(&preset).unwrap();
        let back: Preset = serde_json::from_str(&json).unwrap();
        assert_eq!(preset, back);
        assert!(json.contains("\"kind\":\"mixer\""));
    }

    #[test]
    fn unknown_keys_are_skipped_not_fatal() {
        let mut controls = BTreeMap::new();
        controls.insert("mute".to_owned(), Scalar::Bool(true));
        controls.insert("not-a-control".to_owned(), Scalar::Int(1));
        let preset = Preset::Strip {
            version: PRESET_VERSION,
            controls,
        };

        let mut d = dev();
        let report = d.apply(&preset, Some(0)).unwrap();
        assert_eq!(report.applied, 1);
        assert_eq!(report.skipped, vec!["not-a-control".to_owned()]);
    }

    #[test]
    fn kind_and_channel_mismatches_error() {
        let mut d = dev();
        let strip = d.capture_strip(0).unwrap();
        let mixer = d.capture_mixer().unwrap();
        assert!(d.apply(&strip, None).is_err());
        assert!(d.apply(&mixer, Some(0)).is_err());
    }
}
