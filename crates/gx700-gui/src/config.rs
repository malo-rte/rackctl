//! Persisted GUI state and an on-disk cache of the patch bank, so a relaunch shows
//! the list instantly instead of re-reading 100 patches (~1 minute) every time.

use std::path::{Path, PathBuf};

use directories::ProjectDirs;
use serde::{Deserialize, Serialize};

/// Default interface zoom factor (egui zoom), used when no config exists.
pub(crate) const DEFAULT_ZOOM: f32 = 1.5;

/// GUI state saved between runs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct GuiConfig {
    /// Interface zoom factor, restored on startup.
    #[serde(default = "default_zoom")]
    pub zoom: f32,
    /// Saved window inner size in logical points (`[width, height]`), or `None` for
    /// the default size.
    #[serde(default)]
    pub window: Option<[f32; 2]>,
}

impl Default for GuiConfig {
    fn default() -> Self {
        Self {
            zoom: DEFAULT_ZOOM,
            window: None,
        }
    }
}

fn default_zoom() -> f32 {
    DEFAULT_ZOOM
}

/// One cached patch-list row (mirrors `PatchHeader`, made serializable).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct CachedRow {
    pub slot: u16,
    pub name: String,
    pub output_level: u8,
    pub chain: Vec<u8>,
}

/// The suite's per-device settings directory: `<config>/rackctl/gx700`. `None` if
/// no home directory can be determined.
pub(crate) fn settings_dir() -> Option<PathBuf> {
    Some(
        ProjectDirs::from("", "malo-rte", "rackctl")?
            .config_dir()
            .join("gx700"),
    )
}

fn config_path() -> Option<PathBuf> {
    settings_dir().map(|dir| dir.join("gui-config.json"))
}

fn cache_path() -> Option<PathBuf> {
    settings_dir().map(|dir| dir.join("bank-cache.json"))
}

/// Load the saved GUI config, falling back to defaults on any error.
pub(crate) fn load() -> GuiConfig {
    let Some(path) = config_path() else {
        return GuiConfig::default();
    };
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

/// Best-effort save of the GUI config; failures are ignored (not critical).
pub(crate) fn save(config: &GuiConfig) {
    write_json(config_path(), config);
}

/// Load the cached patch bank (empty if absent or unreadable).
pub(crate) fn load_cache() -> Vec<CachedRow> {
    let Some(path) = cache_path() else {
        return Vec::new();
    };
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

/// Best-effort save of the patch-bank cache.
pub(crate) fn save_cache(rows: &[CachedRow]) {
    write_json(cache_path(), rows);
}

fn write_json<T: Serialize + ?Sized>(path: Option<PathBuf>, value: &T) {
    let Some(path) = path else {
        return;
    };
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(text) = serde_json::to_string_pretty(value) {
        let _ = std::fs::write(&path, text);
    }
}

// ---- On-disk libraries: single patches, single blocks, and whole-bank scenes ----

/// Library directory for saved single patches (`<settings>/patches`).
pub(crate) fn patches_dir() -> Option<PathBuf> {
    settings_dir().map(|d| d.join("patches"))
}

/// Library directory for saved single effect blocks (`<settings>/blocks`).
pub(crate) fn blocks_dir() -> Option<PathBuf> {
    settings_dir().map(|d| d.join("blocks"))
}

/// Library directory for saved scenes — whole-bank snapshots (`<settings>/scenes`).
#[allow(dead_code)] // used by the scene editor (next)
pub(crate) fn scenes_dir() -> Option<PathBuf> {
    settings_dir().map(|d| d.join("scenes"))
}

/// Turn a user-entered name into a safe `.json` file stem.
pub(crate) fn sanitize(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || matches!(c, ' ' | '-' | '_' | '.') {
                c
            } else {
                '_'
            }
        })
        .collect();
    let trimmed = cleaned.trim();
    if trimmed.is_empty() {
        "untitled".to_owned()
    } else {
        trimmed.to_owned()
    }
}

/// Path of `name`.json inside `dir` (sanitised).
pub(crate) fn lib_path(dir: Option<PathBuf>, name: &str) -> Option<PathBuf> {
    dir.map(|d| d.join(format!("{}.json", sanitize(name))))
}

/// Sorted `.json` file stems in `dir` (empty if the directory is missing).
pub(crate) fn json_stems(dir: Option<PathBuf>) -> Vec<String> {
    let Some(dir) = dir else {
        return Vec::new();
    };
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut names: Vec<String> = entries
        .filter_map(std::result::Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "json"))
        .filter_map(|p| p.file_stem().map(|s| s.to_string_lossy().into_owned()))
        .collect();
    names.sort();
    names
}

/// Save `value` as pretty JSON to `path`, creating parent dirs. `Err` on failure.
pub(crate) fn save_json<T: Serialize>(path: &Path, value: &T) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let text = serde_json::to_string_pretty(value).map_err(|e| e.to_string())?;
    std::fs::write(path, text).map_err(|e| e.to_string())
}

/// Read a file to a string, or `None` if it can't be read.
pub(crate) fn read_text(path: &Path) -> Option<String> {
    std::fs::read_to_string(path).ok()
}

/// Delete a file. `Err` on failure.
pub(crate) fn delete_file(path: &Path) -> Result<(), String> {
    std::fs::remove_file(path).map_err(|e| e.to_string())
}
