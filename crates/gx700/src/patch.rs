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
