//! A **typed, block-structured Eleven Rack patch** — the readable counterpart of
//! the raw [`tfx::Patch`] file model.
//!
//! [`Patch`] mirrors the GX-700's `typed::Patch`: a struct-per-block, named-field
//! view (`patch.distortion.drive`, `patch.reverb.decay`, …) that serialises to
//! JSON grouped by block, so the two products present patches the same way. It is
//! built from a parsed `.tfx` file with [`Patch::from_tfx`] and converts back with
//! [`Patch::to_tfx`].
//!
//! # What is and isn't decoded
//!
//! The `.tfx` **block layout** and the **global-block integers** are fully solved
//! (validated across the 1,243-patch ERUG corpus; see
//! `docs/eleven-rack-rig-format.adoc`). The **effect-block values are 32-bit
//! floats** whose *scaling into display units* (dB, %, ms) is still being
//! reverse-engineered — so every effect parameter is an `Option<f32>` carrying the
//! raw float, where `None` is the device's *unset / default* sentinel
//! (`0x7FFFFFFF` / `0x80000000`). Named fields cover the stable `FourCC` vocabulary;
//! any other tag a patch carries is preserved verbatim in the block's `extra` map,
//! so a round trip never drops a parameter.
#![allow(missing_docs)]

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::tfx::{self, Param};

// ---------------------------------------------------------------------------
// Value helpers. Global-block words are integers; effect-block words are floats
// with two "unset/default" sentinels that decode to `None`.
// ---------------------------------------------------------------------------

/// The two words the device uses to mark an effect parameter as unset / default.
const SENTINELS: [u32; 2] = [0x7FFF_FFFF, 0x8000_0000];
/// The sentinel written back for a `None` effect parameter.
const UNSET: u32 = 0x7FFF_FFFF;

/// Decode an effect-block word to a float, or `None` for the unset sentinel.
fn dec_f(word: u32) -> Option<f32> {
    if SENTINELS.contains(&word) {
        None
    } else {
        Some(f32::from_bits(word))
    }
}

/// Encode an optional effect float back to its 32-bit word (`None` -> sentinel).
fn enc_f(value: Option<f32>) -> u32 {
    value.map_or(UNSET, f32::to_bits)
}

/// Decode a global-block word to an integer, or `None` for the unset sentinel
/// (the global block uses the same `0x7FFFFFFF`/`0x80000000` "unset" words).
fn dec_i(word: u32) -> Option<i64> {
    if SENTINELS.contains(&word) {
        None
    } else {
        Some(i64::from(word))
    }
}

/// Encode an optional global integer back to its 32-bit word (`None` -> sentinel).
fn enc_i(value: Option<i64>) -> u32 {
    value.and_then(|v| u32::try_from(v).ok()).unwrap_or(UNSET)
}

/// A block's parameters as a tag -> raw-word map, for named-field extraction.
fn to_map(params: &[Param]) -> BTreeMap<String, u32> {
    params.iter().map(|p| (p.tag.clone(), p.value)).collect()
}

/// A boolean flag stored as an effect float (`bypa`): true when non-zero.
fn dec_flag(word: u32) -> bool {
    dec_f(word).is_some_and(|f| f != 0.0)
}

/// Encode a boolean flag back to a `0.0`/`1.0` float word.
fn enc_flag(on: bool) -> u32 {
    if on { 1.0_f32 } else { 0.0_f32 }.to_bits()
}

// ---------------------------------------------------------------------------
// Effect blocks. Each is generated with the same shape: a `bypass` flag, the
// named `Option<f32>` parameters of its stable FourCC vocabulary, and an `extra`
// map that preserves any other tag verbatim so nothing is lost.
// ---------------------------------------------------------------------------

/// Define an effect-block struct plus its `.tfx` block-id and decode/encode.
///
/// `None` in a named field means the tag was absent or held the unset sentinel;
/// the two are not distinguished (the corpus schema is stable, so an expected tag
/// is effectively always present). `re-encoding` a `None` writes the sentinel.
macro_rules! fx_block {
    ($name:ident = $id:literal { $($field:ident : $tag:literal),* $(,)? }) => {
        #[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
        pub struct $name {
            /// `bypa` — the block's bypass flag (true = bypassed).
            pub bypass: bool,
            $(
                #[doc = concat!("`", $tag, "` — raw effect float (`null` = unset).")]
                pub $field: Option<f32>,
            )*
            /// Any parameters the block carries beyond those named above, kept by
            /// `FourCC` tag so a round trip is lossless (`null` = the unset sentinel).
            #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
            pub extra: BTreeMap<String, Option<f32>>,
        }

        impl $name {
            /// This block's fixed `.tfx` signal-chain slot id.
            pub const ID: u8 = $id;

            fn decode(params: &[Param]) -> Self {
                let mut m = to_map(params);
                let bypass = m.remove("bypa").is_some_and(dec_flag);
                $( let $field = m.remove($tag).and_then(dec_f); )*
                let extra = m.into_iter().map(|(k, v)| (k, dec_f(v))).collect();
                Self { bypass, $( $field, )* extra }
            }

            fn encode(&self) -> tfx::Block {
                let mut params = vec![Param { tag: "bypa".into(), value: enc_flag(self.bypass) }];
                $( params.push(Param { tag: $tag.into(), value: enc_f(self.$field) }); )*
                for (tag, v) in &self.extra {
                    params.push(Param { tag: tag.clone(), value: enc_f(*v) });
                }
                tfx::Block { id: Self::ID, params }
            }
        }
    };
}

fx_block!(VolumePedal = 0x43 { volume: "Vol ", min: "Min ", taper: "Tapr" });
fx_block!(Wah = 0x44 { filter: "Filt", vox_cry: "VxCr" });
fx_block!(Distortion = 0x45 { drive: "Driv", tone: "Tone", level: "Levl" });
fx_block!(Eq = 0x46 {
    low_shelf: "LwSh", low_mid_gain: "LMGn", mid_gain: "MGn ",
    high_mid_gain: "HMGn", high_shelf: "HiSh", output: "Out ",
});
fx_block!(Modulation = 0x47 {
    speed: "Sped", depth: "Dpth", feedback: "Fdbk", pre_delay: "PDly", sync: "Sync",
});
fx_block!(Modulation2 = 0x48 { speed: "Sped", sync: "Sync" });
fx_block!(FxLoop = 0x4A { send: "send", ret: "rtrn", wet: "wetp" });
fx_block!(Delay = 0x4B {
    volume: "Vol ", sustain: "Sust", time: "EDly", sync: "Sync",
    rec: "Rec ", wow: "Wow ", tilt: "Tilt",
});
fx_block!(Reverb = 0x4C {
    decay: "Dcay", tone: "Tone", mix: "RMix", kind: "Type", pre_delay: "PDly",
});

// ---------------------------------------------------------------------------
// Global block (0x41) — integer values, fully decoded.
// ---------------------------------------------------------------------------

/// The rig-global block (`0x41`): master levels, tempo, expression and routing.
/// Values are integers (unlike the float effect blocks); `null` is the same
/// "unset / default" sentinel the effect blocks use.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Global {
    /// `RVol` — master rig volume (`null` = unset).
    pub master_volume: Option<i64>,
    /// `Vol1` — volume control 1 (`null` = unset).
    pub volume1: Option<i64>,
    /// `Vol2` — volume control 2 (`null` = unset).
    pub volume2: Option<i64>,
    /// `RMno` — mono flag.
    pub mono: bool,
    /// `Tmpo` — tempo in microseconds per beat (`500000` = 120 BPM).
    pub tempo_us: Option<i64>,
    /// `Msyc` — MIDI-clock sync flag.
    pub midi_sync: bool,
    /// `ExpT` — expression-pedal target.
    pub exp_target: Option<i64>,
    /// `PIGI` — input gain (`0..=127`).
    pub input_gain: Option<i64>,
    /// `FXc1..FXc4` — the four FX-chain slots (packed effect id + state; raw).
    pub fx_chain: [Option<i64>; 4],
    /// The `Wor*`/`Wst*` routing-graph nodes and any other global tag, kept by
    /// tag (the routing graph decode is still open; preserved so nothing is lost).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub routing: BTreeMap<String, Option<i64>>,
}

impl Global {
    /// This block's fixed `.tfx` slot id.
    pub const ID: u8 = 0x41;

    fn decode(params: &[Param]) -> Self {
        let mut m = to_map(params);
        let take = |m: &mut BTreeMap<String, u32>, tag: &str| m.remove(tag).and_then(dec_i);
        let master_volume = take(&mut m, "RVol");
        let volume1 = take(&mut m, "Vol1");
        let volume2 = take(&mut m, "Vol2");
        let mono = m.remove("RMno").is_some_and(|v| v != 0);
        let tempo_us = take(&mut m, "Tmpo");
        let midi_sync = m.remove("Msyc").is_some_and(|v| v != 0);
        let exp_target = take(&mut m, "ExpT");
        let input_gain = take(&mut m, "PIGI");
        let fx_chain = [
            take(&mut m, "FXc1"),
            take(&mut m, "FXc2"),
            take(&mut m, "FXc3"),
            take(&mut m, "FXc4"),
        ];
        let routing = m.into_iter().map(|(k, v)| (k, dec_i(v))).collect();
        Self {
            master_volume,
            volume1,
            volume2,
            mono,
            tempo_us,
            midi_sync,
            exp_target,
            input_gain,
            fx_chain,
            routing,
        }
    }

    fn encode(&self) -> tfx::Block {
        let mut params = vec![
            Param {
                tag: "RVol".into(),
                value: enc_i(self.master_volume),
            },
            Param {
                tag: "Vol1".into(),
                value: enc_i(self.volume1),
            },
            Param {
                tag: "Vol2".into(),
                value: enc_i(self.volume2),
            },
            Param {
                tag: "RMno".into(),
                value: u32::from(self.mono),
            },
            Param {
                tag: "Tmpo".into(),
                value: enc_i(self.tempo_us),
            },
            Param {
                tag: "Msyc".into(),
                value: u32::from(self.midi_sync),
            },
            Param {
                tag: "ExpT".into(),
                value: enc_i(self.exp_target),
            },
            Param {
                tag: "PIGI".into(),
                value: enc_i(self.input_gain),
            },
        ];
        for (i, slot) in self.fx_chain.iter().enumerate() {
            params.push(Param {
                tag: format!("FXc{}", i + 1),
                value: enc_i(*slot),
            });
        }
        for (tag, v) in &self.routing {
            params.push(Param {
                tag: tag.clone(),
                value: enc_i(*v),
            });
        }
        tfx::Block {
            id: Self::ID,
            params,
        }
    }
}

// ---------------------------------------------------------------------------
// Amp block (0x49) — model-specific knob "sliders", kept by tag.
// ---------------------------------------------------------------------------

/// The amp block (`0x49`). Its knobs are stored as generic `sld1..sldO` slider
/// slots whose mapping to a named knob is *model-specific* and still being
/// matched from the live `SysEx` sweep, so they are kept by tag (`null` = unset).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Amp {
    /// The amp model's knob sliders (`sld1..sldO`), by `FourCC` tag.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub sliders: BTreeMap<String, Option<f32>>,
}

impl Amp {
    /// This block's fixed `.tfx` slot id.
    pub const ID: u8 = 0x49;

    fn decode(params: &[Param]) -> Self {
        Self {
            sliders: params
                .iter()
                .map(|p| (p.tag.clone(), dec_f(p.value)))
                .collect(),
        }
    }

    fn encode(&self) -> tfx::Block {
        tfx::Block {
            id: Self::ID,
            params: self
                .sliders
                .iter()
                .map(|(tag, v)| Param {
                    tag: tag.clone(),
                    value: enc_f(*v),
                })
                .collect(),
        }
    }
}

// ---------------------------------------------------------------------------
// The whole patch.
// ---------------------------------------------------------------------------

/// A typed, block-structured Eleven Rack patch. Built from a raw [`tfx::Patch`]
/// with [`Patch::from_tfx`]; the fields are grouped by signal-chain block so the
/// JSON reads like the GX-700's.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Patch {
    /// The patch name.
    pub name: String,
    pub global: Global,
    pub volume_pedal: VolumePedal,
    pub wah: Wah,
    pub distortion: Distortion,
    pub eq: Eq,
    pub modulation: Modulation,
    pub modulation2: Modulation2,
    pub amp: Amp,
    pub fx_loop: FxLoop,
    pub delay: Delay,
    pub reverb: Reverb,
    /// Any block whose id is not one of the eleven known signal-chain slots,
    /// preserved verbatim so an unrecognised block is never dropped.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub unknown_blocks: Vec<tfx::Block>,
}

impl Patch {
    /// Decode a parsed `.tfx` file into the typed model.
    #[must_use]
    pub fn from_tfx(raw: &tfx::Patch) -> Self {
        let params = |id: u8| -> &[Param] {
            raw.blocks
                .iter()
                .find(|b| b.id == id)
                .map_or(&[][..], |b| b.params.as_slice())
        };
        let known = [
            Global::ID,
            VolumePedal::ID,
            Wah::ID,
            Distortion::ID,
            Eq::ID,
            Modulation::ID,
            Modulation2::ID,
            Amp::ID,
            FxLoop::ID,
            Delay::ID,
            Reverb::ID,
        ];
        let unknown_blocks = raw
            .blocks
            .iter()
            .filter(|b| !known.contains(&b.id))
            .cloned()
            .collect();
        Self {
            name: raw.name.clone(),
            global: Global::decode(params(Global::ID)),
            volume_pedal: VolumePedal::decode(params(VolumePedal::ID)),
            wah: Wah::decode(params(Wah::ID)),
            distortion: Distortion::decode(params(Distortion::ID)),
            eq: Eq::decode(params(Eq::ID)),
            modulation: Modulation::decode(params(Modulation::ID)),
            modulation2: Modulation2::decode(params(Modulation2::ID)),
            amp: Amp::decode(params(Amp::ID)),
            fx_loop: FxLoop::decode(params(FxLoop::ID)),
            delay: Delay::decode(params(Delay::ID)),
            reverb: Reverb::decode(params(Reverb::ID)),
            unknown_blocks,
        }
    }

    /// Re-encode to the raw `.tfx` block model. Blocks are emitted in signal-chain
    /// order followed by any preserved unknown blocks. Effect floats round-trip
    /// exactly; the two unset sentinels both collapse to `0x7FFFFFFF`.
    #[must_use]
    pub fn to_tfx(&self) -> tfx::Patch {
        let mut blocks = vec![
            self.global.encode(),
            self.volume_pedal.encode(),
            self.wah.encode(),
            self.distortion.encode(),
            self.eq.encode(),
            self.modulation.encode(),
            self.modulation2.encode(),
            self.amp.encode(),
            self.fx_loop.encode(),
            self.delay.encode(),
            self.reverb.encode(),
        ];
        blocks.extend(self.unknown_blocks.iter().cloned());
        tfx::Patch {
            name: self.name.clone(),
            blocks,
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::float_cmp, clippy::indexing_slicing)]
    use super::*;

    /// A small synthetic `.tfx` patch covering the global block, one effect block
    /// with a real value + an unset sentinel + an unknown tag, the amp sliders,
    /// and an unknown block.
    fn sample() -> tfx::Patch {
        tfx::Patch {
            name: "Test Rig".into(),
            blocks: vec![
                tfx::Block {
                    id: 0x41,
                    params: vec![
                        Param {
                            tag: "RVol".into(),
                            value: 100,
                        },
                        Param {
                            tag: "Vol1".into(),
                            value: 0x8000_0000, // unset sentinel
                        },
                        Param {
                            tag: "Tmpo".into(),
                            value: 500_000,
                        },
                        Param {
                            tag: "RMno".into(),
                            value: 1,
                        },
                        Param {
                            tag: "WorB".into(),
                            value: 42,
                        }, // routing -> extra
                    ],
                },
                tfx::Block {
                    id: 0x45, // distortion
                    params: vec![
                        Param {
                            tag: "bypa".into(),
                            value: 0.0_f32.to_bits(),
                        },
                        Param {
                            tag: "Driv".into(),
                            value: 0.75_f32.to_bits(),
                        },
                        Param {
                            tag: "Tone".into(),
                            value: UNSET,
                        }, // unset
                        Param {
                            tag: "Xtra".into(),
                            value: 0.5_f32.to_bits(),
                        }, // -> extra
                    ],
                },
                tfx::Block {
                    id: 0x49, // amp
                    params: vec![
                        Param {
                            tag: "sld1".into(),
                            value: 0.25_f32.to_bits(),
                        },
                        Param {
                            tag: "sld2".into(),
                            value: UNSET,
                        },
                    ],
                },
                tfx::Block {
                    id: 0x7E, // not a known signal-chain slot
                    params: vec![Param {
                        tag: "Zzzz".into(),
                        value: 7,
                    }],
                },
            ],
        }
    }

    #[test]
    fn decodes_global_ints_and_routing() {
        let p = Patch::from_tfx(&sample());
        assert_eq!(p.name, "Test Rig");
        assert_eq!(p.global.master_volume, Some(100));
        assert_eq!(p.global.volume1, None); // the unset sentinel
        assert_eq!(p.global.tempo_us, Some(500_000));
        assert!(p.global.mono);
        assert_eq!(p.global.routing.get("WorB"), Some(&Some(42)));
    }

    #[test]
    fn decodes_effect_floats_sentinel_and_extra() {
        let p = Patch::from_tfx(&sample());
        assert!(!p.distortion.bypass);
        assert_eq!(p.distortion.drive, Some(0.75));
        assert_eq!(p.distortion.tone, None); // the unset sentinel
        assert_eq!(p.distortion.extra.get("Xtra"), Some(&Some(0.5)));
    }

    #[test]
    fn keeps_amp_sliders_and_unknown_blocks() {
        let p = Patch::from_tfx(&sample());
        assert_eq!(p.amp.sliders.get("sld1"), Some(&Some(0.25)));
        assert_eq!(p.amp.sliders.get("sld2"), Some(&None));
        assert_eq!(p.unknown_blocks.len(), 1);
        assert_eq!(p.unknown_blocks[0].id, 0x7E);
    }

    #[test]
    fn typed_round_trip_is_idempotent_and_lossless() {
        // A parameter is never dropped: the second decode equals the first, so
        // to_tfx -> from_tfx is a fixed point (the sentinel collapse settles).
        let once = Patch::from_tfx(&sample());
        let twice = Patch::from_tfx(&once.to_tfx());
        assert_eq!(once, twice);
        // Every source block id survives the round trip.
        let ids: Vec<u8> = once.to_tfx().blocks.iter().map(|b| b.id).collect();
        for id in [0x41, 0x45, 0x49, 0x7E] {
            assert!(ids.contains(&id), "block {id:#x} lost");
        }
    }

    #[test]
    fn serialises_grouped_by_block() {
        let p = Patch::from_tfx(&sample());
        let json = serde_json::to_value(&p).unwrap();
        assert!(
            json.get("distortion")
                .and_then(|b| b.get("drive"))
                .is_some()
        );
        assert!(json.get("global").and_then(|b| b.get("tempo_us")).is_some());
        assert_eq!(json.get("name").and_then(|n| n.as_str()), Some("Test Rig"));
    }
}
