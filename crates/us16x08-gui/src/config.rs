//! Persisted GUI-only state. The stereo-link grouping is not a hardware control
//! (the driver has no link element), so it lives here rather than in the
//! device's JSON presets.

use std::path::{Path, PathBuf};

use directories::ProjectDirs;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

// The on-disk library format + path conventions come from the shared crates, so
// the GUI and CLI can't drift. The device id and envelope version are the
// US-16x08 library's.
pub(crate) use rackctl_us16x08::{DEVICE_ID, LIB_VERSION};

/// Default interface zoom factor (egui zoom), used when no config exists.
pub(crate) const DEFAULT_ZOOM: f32 = 1.5;

/// GUI state saved between runs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct GuiConfig {
    /// Stereo-link state for the eight adjacent channel pairs (0/1 .. 14/15).
    #[serde(default)]
    pub links: [bool; 8],
    /// Interface zoom factor, restored on startup and on `Load default`.
    #[serde(default = "default_zoom")]
    pub zoom: f32,
    /// Saved window inner size in logical points (`[width, height]`), or `None`
    /// to use the default size. Restored on startup and on `Load default`.
    #[serde(default)]
    pub window: Option<[f32; 2]>,
    /// User-given names for the 16 input channels (GUI-only), empty when unset.
    #[serde(default = "default_names")]
    pub names: [String; 16],
}

impl Default for GuiConfig {
    fn default() -> Self {
        Self {
            links: [false; 8],
            zoom: DEFAULT_ZOOM,
            window: None,
            names: default_names(),
        }
    }
}

fn default_zoom() -> f32 {
    DEFAULT_ZOOM
}

fn default_names() -> [String; 16] {
    std::array::from_fn(|_| String::new())
}

/// The suite's per-device settings directory: `<config>/rackctl/us16x08`, where
/// the default preset, scenes, and per-section presets live (shared with the
/// CLI's `default` command). Migrates a pre-rename `tascam-mixer` directory into
/// place on first use. `None` if no home directory can be determined.
pub(crate) fn settings_dir() -> Option<PathBuf> {
    let dir = rackctl_core::device_dir(DEVICE_ID)?;
    migrate_legacy_settings(&dir);
    Some(dir)
}

/// One-time move of the pre-rename settings (`<config>/tascam-mixer`, from when
/// the tools were named after the device) into the new suite location. Best
/// effort: if it cannot move, the app simply starts with fresh settings.
fn migrate_legacy_settings(new_dir: &Path) {
    if new_dir.exists() {
        return;
    }
    let Some(old) = ProjectDirs::from("de", "paraair", "tascam-mixer")
        .map(|dirs| dirs.config_dir().to_path_buf())
    else {
        return;
    };
    if !old.exists() {
        return;
    }
    if let Some(parent) = new_dir.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::rename(&old, new_dir);
}

fn config_path() -> Option<PathBuf> {
    settings_dir().map(|dir| dir.join("config.json"))
}

/// Path to the shared default-mixer preset, in the same settings directory the
/// CLI's `default` command uses. `None` if no home directory can be determined.
pub(crate) fn default_preset_path() -> Option<PathBuf> {
    settings_dir().map(|dir| dir.join("default-preset.json"))
}

/// Directory holding the user's saved scenes (whole-mixer presets).
pub(crate) fn scenes_dir() -> Option<PathBuf> {
    rackctl_core::library_dir(DEVICE_ID, "scenes")
}

/// Directory holding the user's saved channel presets (single-channel strips).
pub(crate) fn strips_dir() -> Option<PathBuf> {
    rackctl_core::library_dir(DEVICE_ID, "strips")
}

/// Directory holding the user's saved EQ presets.
pub(crate) fn eq_dir() -> Option<PathBuf> {
    rackctl_core::library_dir(DEVICE_ID, "eq")
}

/// Directory holding the user's saved compressor presets.
pub(crate) fn comp_dir() -> Option<PathBuf> {
    rackctl_core::library_dir(DEVICE_ID, "comp")
}

/// Save `payload` to `path` in the shared library envelope (format version +
/// device id), so the file is self-identifying and version-checked on load.
pub(crate) fn save_item<T: Serialize>(path: &Path, payload: &T) -> Result<(), String> {
    rackctl_core::save_item(path, DEVICE_ID, LIB_VERSION, payload)
}

/// Read a library item from envelope `text`: `None` if it is not one of our
/// envelopes (the caller may then try a bare/legacy parse); `Some(Err)` if from
/// another device or a newer format; `Some(Ok(payload))` otherwise.
pub(crate) fn load_item<T: DeserializeOwned>(text: &str) -> Option<Result<T, String>> {
    rackctl_core::decode_item(DEVICE_ID, LIB_VERSION, text)
}

/// Read a file to a string, or `None` if it can't be read.
pub(crate) fn read_text(path: &Path) -> Option<String> {
    rackctl_core::read_text(path)
}

/// Delete a file. `Err` on failure.
pub(crate) fn delete_file(path: &Path) -> Result<(), String> {
    rackctl_core::delete_file(path)
}

/// Load the saved config, falling back to defaults on any error.
pub(crate) fn load() -> GuiConfig {
    let Some(path) = config_path() else {
        return GuiConfig::default();
    };
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

/// Best-effort save; failures are ignored (GUI state is not critical).
pub(crate) fn save(config: &GuiConfig) {
    let Some(path) = config_path() else {
        return;
    };
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(text) = serde_json::to_string_pretty(config) {
        let _ = std::fs::write(&path, text);
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::indexing_slicing)]
    use super::GuiConfig;

    #[test]
    fn config_without_names_loads_with_empty_defaults() {
        // A config written before channel names existed must still load.
        let json = r#"{"links":[false,false,false,false,false,false,false,false],"zoom":1.5}"#;
        let cfg: GuiConfig = serde_json::from_str(json).unwrap();
        assert_eq!(cfg.names.len(), 16);
        assert!(cfg.names.iter().all(String::is_empty));
    }

    #[test]
    fn names_round_trip_through_json() {
        let mut cfg = GuiConfig::default();
        cfg.names[0] = "Kick".to_owned();
        cfg.names[15] = "Vox".to_owned();
        let text = serde_json::to_string(&cfg).unwrap();
        let back: GuiConfig = serde_json::from_str(&text).unwrap();
        assert_eq!(back.names[0], "Kick");
        assert_eq!(back.names[15], "Vox");
    }
}
