//! Saved-file formats and parsing.
//!
//! Every saved item is the rackctl-core envelope around the typed payload (the
//! documented format), shared by the CLI and GUI. The one extra patch form
//! accepted is a *bare* typed patch — what `dump --json` emits, for hand-editing
//! and piping.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use rackctl_gx700::typed::{BlockData, Patch};

use crate::{DEVICE_ID, LIB_VERSION};

/// A whole-device **scene**: the user patch bank as `slot -> patch`. The map is
/// sparse, so a partial capture (a read failed mid-scene) survives rather than
/// silently filling the gap. `name` is the library filename, kept in the payload
/// too so the file is self-describing.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Scene {
    /// The scene's library name.
    #[serde(default)]
    pub name: String,
    /// Captured user patches by slot (`1..=100`).
    pub patches: BTreeMap<u16, Patch>,
}

impl Scene {
    /// An empty scene named `name`.
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            patches: BTreeMap::new(),
        }
    }
}

/// Parse a saved **patch** file: a rackctl-core envelope around the typed patch,
/// or a bare typed patch (the `dump --json` form).
///
/// # Errors
/// If the text matches neither form, or an envelope is from another device / a
/// newer format.
pub fn parse_patch(text: &str) -> Result<Patch, String> {
    if let Some(res) = rackctl_core::decode_item::<Patch>(DEVICE_ID, LIB_VERSION, text) {
        return res;
    }
    serde_json::from_str::<Patch>(text).map_err(|_| "unrecognised patch file".to_owned())
}

/// Parse a saved **scene** file: a rackctl-core envelope around the canonical
/// [`Scene`].
///
/// # Errors
/// If the text is not such an envelope, or it is from another device / a newer
/// format.
pub fn parse_scene(text: &str) -> Result<Scene, String> {
    rackctl_core::decode_item::<Scene>(DEVICE_ID, LIB_VERSION, text)
        .unwrap_or_else(|| Err("unrecognised scene file".to_owned()))
}

/// Parse a saved single-**block** preset file: a rackctl-core envelope around the
/// block data.
///
/// # Errors
/// If the text is not such an envelope, or it is from another device / a newer
/// format.
pub fn parse_block(text: &str) -> Result<BlockData, String> {
    rackctl_core::decode_item::<BlockData>(DEVICE_ID, LIB_VERSION, text)
        .unwrap_or_else(|| Err("unrecognised block file".to_owned()))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;

    fn json_eq<T: Serialize>(a: &T, b: &T) -> bool {
        serde_json::to_value(a).unwrap() == serde_json::to_value(b).unwrap()
    }

    #[test]
    fn patch_envelope_round_trips() {
        let p = Patch::init();
        let text = rackctl_core::encode_item(DEVICE_ID, LIB_VERSION, &p).unwrap();
        assert!(json_eq(&parse_patch(&text).unwrap(), &p));
    }

    #[test]
    fn patch_reads_bare_typed_dump_json() {
        // The `dump --json` form: a bare typed patch (no envelope).
        let p = Patch::init();
        let text = serde_json::to_string(&p).unwrap();
        assert!(json_eq(&parse_patch(&text).unwrap(), &p));
    }

    #[test]
    fn scene_envelope_round_trips() {
        let mut scene = Scene::new("gig");
        scene.patches.insert(1, Patch::init());
        scene.patches.insert(7, Patch::init());
        let text = rackctl_core::encode_item(DEVICE_ID, LIB_VERSION, &scene).unwrap();
        let got = parse_scene(&text).unwrap();
        assert_eq!(got.name, "gig");
        assert!(got.patches.contains_key(&1) && got.patches.contains_key(&7));
        // A non-envelope (legacy) scene is no longer accepted.
        assert!(parse_scene("[]").is_err());
    }

    #[test]
    fn block_envelope_round_trips() {
        let data = BlockData::from_patch(&Patch::init(), rackctl_gx700::Block::Reverb).unwrap();
        let text = rackctl_core::encode_item(DEVICE_ID, LIB_VERSION, &data).unwrap();
        assert!(json_eq(&parse_block(&text).unwrap(), &data));
    }

    #[test]
    fn other_device_envelope_is_rejected() {
        let text = rackctl_core::encode_item("us16x08", LIB_VERSION, &Patch::init()).unwrap();
        assert!(parse_patch(&text).is_err());
    }
}
