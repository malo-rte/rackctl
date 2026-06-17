//! Persisted GUI-only state. The stereo-link grouping is not a hardware control
//! (the driver has no link element), so it lives here rather than in the
//! device's JSON presets.

use std::path::PathBuf;

use directories::ProjectDirs;
use serde::{Deserialize, Serialize};

/// GUI state saved between runs.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub(crate) struct GuiConfig {
    /// Stereo-link state for the eight adjacent channel pairs (0/1 .. 14/15).
    #[serde(default)]
    pub links: [bool; 8],
}

fn config_path() -> Option<PathBuf> {
    ProjectDirs::from("de", "paraair", "tascam-mixer")
        .map(|dirs| dirs.config_dir().join("config.json"))
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
