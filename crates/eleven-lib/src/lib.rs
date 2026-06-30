//! Eleven Rack **library and rig-management** layer (scaffold).
//!
//! This is the device-specific half of the library stack: it sits on
//! [`rackctl_eleven`] (the device protocol + value model) and, once the rig work
//! lands, on `rackctl-core` (the device-neutral on-disk library), providing what
//! both a CLI and a GUI need but neither should own privately.
//!
//! For now it carries only the on-disk identity constants; the `.tfx` rig parser,
//! the named rig libraries, and device-touching management arrive with later
//! steps (see `docs/eleven-rack-roadmap.adoc`).
//!
//! NOTE: Eleven Rack, Digidesign and Avid are trademarks of Avid Technology, Inc.
//! This is an independent, unofficial project.
#![forbid(unsafe_code)]

// Re-export the protocol crate so a frontend can depend on this one crate.
pub use rackctl_eleven as device;

/// This device's stable id, stamped into every saved library item (the
/// rackctl-core envelope) so a file is matched to the Eleven Rack on load.
pub const DEVICE_ID: &str = "eleven";

/// Current on-disk library format version. Bump when the envelope or a payload
/// shape changes; older versions load (with migration), newer ones are refused.
pub const LIB_VERSION: u32 = 1;
