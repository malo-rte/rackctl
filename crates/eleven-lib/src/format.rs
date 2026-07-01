//! Saved-file formats and parsing for the whole-device **scene**.
//!
//! A scene is the User patch bank captured as one file, so a whole arrangement can
//! be saved and re-flashed. Each saved item is the rackctl-core envelope around the
//! payload, shared by the CLI and GUI.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use rackctl_eleven::PatchBackup;

use crate::{DEVICE_ID, LIB_VERSION};

/// A whole-device **scene**: the User patch bank as `slot -> `[`PatchBackup`]. The
/// map is sparse, so a partial capture (a read failed, or the bank wrapped early)
/// survives rather than silently filling the gap. `name` is the library filename,
/// kept in the payload too so the file is self-describing.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Scene {
    /// The scene's library name.
    #[serde(default)]
    pub name: String,
    /// Captured User patches by slot (0-based).
    pub patches: BTreeMap<u8, PatchBackup>,
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

/// Parse a saved **scene** file: a rackctl-core envelope around the canonical
/// [`Scene`].
///
/// # Errors
/// If the text is not such an envelope, or it is from another device / a newer
/// format version.
pub fn parse_scene(text: &str) -> Result<Scene, String> {
    rackctl_core::decode_item::<Scene>(DEVICE_ID, LIB_VERSION, text)
        .unwrap_or_else(|| Err("unrecognised scene file".to_owned()))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    use rackctl_eleven::BlockData;

    #[test]
    fn scene_round_trips_through_the_envelope() {
        let mut scene = Scene::new("live-set");
        scene.patches.insert(
            0,
            PatchBackup::new(
                "rhythm",
                vec![BlockData {
                    id: 0x05,
                    bytes: b"rhythm".to_vec(),
                }],
            ),
        );
        scene.patches.insert(7, PatchBackup::new("lead", vec![]));

        let text = rackctl_core::encode_item(DEVICE_ID, LIB_VERSION, &scene).unwrap();
        assert_eq!(parse_scene(&text).unwrap(), scene);
    }
}
