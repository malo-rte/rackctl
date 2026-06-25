//! Capturing and restoring the GX-700 patch buffer as a JSON [`Patch`].
//!
//! A [`Patch`] is a serde-(de)serializable snapshot of parameter values, keyed
//! by [`crate::Param::key`]. File and format handling lives in the caller (e.g. the
//! CLI); this module only turns device state into a [`Patch`] and back.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::backend::Transport;
use crate::device::Gx700;
use crate::error::{Error, Result};
use crate::param::{self, Kind, Value};

/// Schema version written into every patch, for forward compatibility.
pub const PATCH_VERSION: u32 = 1;

/// Number of characters in a GX-700 patch name.
pub const NAME_LEN: usize = 12;

/// Decode a GX-700 patch name: up to [`NAME_LEN`] 7-bit character codes (ASCII
/// over the printable range), with trailing padding trimmed.
#[must_use]
pub fn decode_name(bytes: &[u8]) -> String {
    let raw: String = bytes
        .iter()
        .take(NAME_LEN)
        .map(|&c| {
            if (0x20..0x7f).contains(&c) {
                char::from(c)
            } else {
                ' '
            }
        })
        .collect();
    raw.trim_end().to_owned()
}

/// Encode a patch name into [`NAME_LEN`] space-padded 7-bit character bytes.
#[must_use]
pub fn encode_name(name: &str) -> [u8; NAME_LEN] {
    let mut out = [0x20u8; NAME_LEN];
    for (slot, ch) in out.iter_mut().zip(name.chars().take(NAME_LEN)) {
        let code = u32::from(ch);
        if (0x20..0x7f).contains(&code) {
            *slot = u8::try_from(code).unwrap_or(0x20);
        }
    }
    out
}

/// The 4-byte base address of patch memory `slot`: user patches `1..=100`
/// (area `00`), preset patches `101..=200` (area `01`). `None` if out of range.
#[must_use]
pub fn patch_base(slot: u16) -> Option<[u8; 4]> {
    let (area, index) = match slot {
        1..=100 => (0x00u8, slot - 1),
        101..=200 => (0x01u8, slot - 101),
        _ => return None,
    };
    Some([area, u8::try_from(index).unwrap_or(0), 0x00, 0x00])
}

/// The header of a stored patch: name, output level, and effect-chain order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatchHeader {
    /// The patch name (trailing padding trimmed).
    pub name: String,
    /// Patch output level, raw `0..=100`.
    pub output_level: u8,
    /// The 13 effect-type bytes giving the block order in the signal chain.
    pub chain: Vec<u8>,
}

/// A single parameter value as stored in a patch. Enums are kept as their label
/// for readability; integers and booleans use their native JSON types.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Scalar {
    /// A boolean switch value.
    Bool(bool),
    /// An integer value, in raw device units.
    Int(i64),
    /// An enum value, stored as its label.
    Text(String),
}

/// A saved snapshot of a GX-700 patch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Patch {
    /// Schema version ([`PATCH_VERSION`]).
    pub version: u32,
    /// Parameter values, keyed by [`crate::Param::key`].
    pub params: BTreeMap<String, Scalar>,
}

fn to_scalar(param: param::Param, value: Value) -> Scalar {
    match value {
        Value::Bool(b) => Scalar::Bool(b),
        Value::Int(i) => Scalar::Int(i64::from(i)),
        Value::Enum(i) => {
            if let Kind::Enum { values, .. } = param.kind() {
                if let Some(label) = usize::try_from(i).ok().and_then(|n| values.get(n)) {
                    return Scalar::Text((*label).to_owned());
                }
            }
            Scalar::Int(i64::from(i))
        }
    }
}

fn from_scalar(param: param::Param, scalar: &Scalar) -> Result<Value> {
    let key = param.key();
    match param.kind() {
        Kind::Bool => match scalar {
            Scalar::Bool(b) => Ok(Value::Bool(*b)),
            _ => Err(Error::Patch(format!("{key}: expected a boolean"))),
        },
        Kind::Int { .. } => match scalar {
            Scalar::Int(n) => i32::try_from(*n)
                .map(Value::Int)
                .map_err(|_| Error::Patch(format!("{key}: value {n} out of range"))),
            _ => Err(Error::Patch(format!("{key}: expected an integer"))),
        },
        Kind::Enum { values, .. } => match scalar {
            Scalar::Text(s) => values
                .iter()
                .position(|v| v.eq_ignore_ascii_case(s))
                .and_then(|i| i32::try_from(i).ok())
                .map(Value::Enum)
                .ok_or_else(|| Error::Patch(format!("{key}: unknown value {s:?}"))),
            Scalar::Int(n) => i32::try_from(*n)
                .map(Value::Enum)
                .map_err(|_| Error::Patch(format!("{key}: value {n} out of range"))),
            Scalar::Bool(_) => Err(Error::Patch(format!("{key}: expected an enum value"))),
        },
    }
}

impl<T: Transport> Gx700<T> {
    /// Read every cataloged parameter into a [`Patch`].
    ///
    /// # Errors
    /// Propagates transport read errors.
    pub fn capture_patch(&mut self) -> Result<Patch> {
        let mut params = BTreeMap::new();
        for &p in param::ALL {
            let value = self.get(p)?;
            params.insert(p.key().to_owned(), to_scalar(p, value));
        }
        Ok(Patch {
            version: PATCH_VERSION,
            params,
        })
    }

    /// Read the header (name, output level, chain order) of stored patch memory
    /// `slot` (`1..=100` user, `101..=200` preset).
    ///
    /// One RQ1 to the patch base returns its Level/Chain block, whose first 26
    /// bytes are the output level, the 13 chain bytes, and the 12-char name.
    ///
    /// # Errors
    /// [`Error::Patch`] if `slot` is out of range; transport errors otherwise.
    pub fn read_patch_header(&mut self, slot: u16) -> Result<PatchHeader> {
        let base = patch_base(slot)
            .ok_or_else(|| Error::Patch(format!("patch slot {slot} out of range (1..=200)")))?;
        let data = self.transport_mut().request(&base, 26)?;
        Ok(PatchHeader {
            output_level: data.first().copied().unwrap_or(0),
            chain: data.get(1..14).unwrap_or(&[]).to_vec(),
            name: decode_name(data.get(14..26).unwrap_or(&[])),
        })
    }

    /// Apply a [`Patch`], writing every parameter the patch holds that this
    /// build recognises. Keys it does not recognise are skipped; the count of
    /// applied parameters is returned.
    ///
    /// # Errors
    /// [`Error::Patch`] if a stored value does not fit its parameter; otherwise
    /// transport write errors.
    pub fn apply_patch(&mut self, patch: &Patch) -> Result<usize> {
        let mut applied = 0;
        for (key, scalar) in &patch.params {
            let Some(p) = param::Param::from_key(key) else {
                continue;
            };
            let value = from_scalar(p, scalar)?;
            self.set(p, value)?;
            applied += 1;
        }
        Ok(applied)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;
    use crate::backend::MockTransport;
    use crate::param::Param;

    fn dev() -> Gx700<MockTransport> {
        Gx700::new(MockTransport::new())
    }

    fn p(key: &str) -> Param {
        Param::from_key(key).unwrap()
    }

    #[test]
    fn name_decodes_and_round_trips() {
        let bytes = [
            0x4E, 0x20, 0x52, 0x4F, 0x44, 0x47, 0x45, 0x52, 0x53, 0x3F, 0x20, 0x20,
        ];
        assert_eq!(decode_name(&bytes), "N RODGERS?");
        assert_eq!(encode_name("N RODGERS?"), bytes);
        // Over-long names are truncated; short ones space-padded to 12.
        assert_eq!(decode_name(&encode_name("JAZZ TONE")), "JAZZ TONE");
        assert_eq!(encode_name("WAY TOO LONG A NAME").len(), NAME_LEN);
    }

    #[test]
    fn patch_base_addresses() {
        assert_eq!(patch_base(1), Some([0x00, 0x00, 0x00, 0x00]));
        assert_eq!(patch_base(2), Some([0x00, 0x01, 0x00, 0x00]));
        assert_eq!(patch_base(100), Some([0x00, 0x63, 0x00, 0x00]));
        assert_eq!(patch_base(101), Some([0x01, 0x00, 0x00, 0x00]));
        assert_eq!(patch_base(200), Some([0x01, 0x63, 0x00, 0x00]));
        assert_eq!(patch_base(0), None);
        assert_eq!(patch_base(201), None);
    }

    #[test]
    fn patch_round_trips_through_a_fresh_device() {
        let mut a = dev();
        a.set(p("preamp-volume"), Value::Int(90)).unwrap();
        a.set(p("comp-enable"), Value::Bool(true)).unwrap();
        a.set(p("dist-type"), Value::Enum(2)).unwrap();

        let patch = a.capture_patch().unwrap();

        let mut b = dev();
        let applied = b.apply_patch(&patch).unwrap();
        assert_eq!(applied, param::ALL.len());
        assert_eq!(b.get(p("preamp-volume")).unwrap(), Value::Int(90));
        assert_eq!(b.get(p("comp-enable")).unwrap(), Value::Bool(true));
        assert_eq!(b.get(p("dist-type")).unwrap(), Value::Enum(2));
    }

    #[test]
    fn serde_json_round_trip() {
        let patch = dev().capture_patch().unwrap();
        let json = serde_json::to_string(&patch).unwrap();
        let back: Patch = serde_json::from_str(&json).unwrap();
        assert_eq!(patch, back);
    }

    #[test]
    fn unknown_keys_are_skipped() {
        let mut params = BTreeMap::new();
        params.insert("comp-enable".to_owned(), Scalar::Bool(true));
        params.insert("not-a-param".to_owned(), Scalar::Int(1));
        let patch = Patch {
            version: PATCH_VERSION,
            params,
        };
        let mut d = dev();
        assert_eq!(d.apply_patch(&patch).unwrap(), 1);
    }
}
