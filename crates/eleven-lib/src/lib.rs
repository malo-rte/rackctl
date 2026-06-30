//! Eleven Rack **library and rig-management** layer (scaffold).
//!
//! This is the device-specific half of the library stack: it sits on
//! [`rackctl_eleven`] (the device protocol + value model) and, once the rig work
//! lands, on `rackctl-core` (the device-neutral on-disk library), providing what
//! both a CLI and a GUI need but neither should own privately.
//!
//! It provides `.tfx` rig import and the named on-disk rig library (save / load /
//! list, via `rackctl-core`); device-touching management (the MIDI library
//! backup/restore) arrives with later steps (see `docs/eleven-rack-roadmap.adoc`).
//!
//! NOTE: Eleven Rack, Digidesign and Avid are trademarks of Avid Technology, Inc.
//! This is an independent, unofficial project.
#![forbid(unsafe_code)]

use std::path::{Path, PathBuf};

use rackctl_eleven::Rig;

// Re-export the protocol crate so a frontend can depend on this one crate.
pub use rackctl_eleven as device;

/// This device's stable id, stamped into every saved library item (the
/// rackctl-core envelope) so a file is matched to the Eleven Rack on load.
pub const DEVICE_ID: &str = "eleven";

/// Current on-disk library format version. Bump when the envelope or a payload
/// shape changes; older versions load (with migration), newer ones are refused.
pub const LIB_VERSION: u32 = 1;

/// On-disk library subdirectory for saved rigs.
const RIGS: &str = "rigs";

fn no_dir() -> String {
    "no config directory available".to_owned()
}

/// Parse an Eleven Rack `.tfx` rig file from disk into a typed [`Rig`].
///
/// # Errors
/// If the file cannot be read, or its contents are not a valid `.tfx` rig.
pub fn import_tfx(path: &Path) -> Result<Rig, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("read {}: {e}", path.display()))?;
    rackctl_eleven::tfx::parse(&bytes).map_err(|e| e.to_string())
}

/// Save `rig` to the rig library as `name`, returning the file path.
///
/// # Errors
/// If no config directory is available, or the write fails.
pub fn save_rig(name: &str, rig: &Rig) -> Result<PathBuf, String> {
    let file = rackctl_core::item_path(DEVICE_ID, RIGS, name).ok_or_else(no_dir)?;
    rackctl_core::save_item(&file, DEVICE_ID, LIB_VERSION, rig)?;
    Ok(file)
}

/// Load the named rig from the rig library.
///
/// # Errors
/// If the file is missing/unreadable, or matches no known format.
pub fn load_rig(name: &str) -> Result<Rig, String> {
    let file = rackctl_core::item_path(DEVICE_ID, RIGS, name).ok_or_else(no_dir)?;
    let text = rackctl_core::read_text(&file)
        .ok_or_else(|| format!("could not read {}", file.display()))?;
    rackctl_core::decode_item::<Rig>(DEVICE_ID, LIB_VERSION, &text)
        .unwrap_or_else(|| Err("unrecognised rig file".to_owned()))
}

/// The names of saved rigs, sorted.
#[must_use]
pub fn list_rigs() -> Vec<String> {
    rackctl_core::list_stems(DEVICE_ID, RIGS)
}
