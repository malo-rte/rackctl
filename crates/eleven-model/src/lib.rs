//! The **Avid/Digidesign Eleven Rack data model** — pure data, no MIDI and no I/O.
//!
//! Everything here describes *what an Eleven Rack parameter value is*,
//! independently of how it is read from or written to the hardware (that is the
//! protocol crate, `rackctl-eleven`). A tool that only needs to read, edit,
//! validate or convert Eleven Rack data can depend on this crate alone — no ALSA,
//! no transport.
//!
//! * [`error`] — the shared error vocabulary for the whole Eleven Rack stack
//!   (model and protocol), so both layers use one [`Error`].
//! * [`value`] — the address-mapped parameter *value* codec: the five-MIDI-byte,
//!   little-endian 7-bit-packed wire word and its [`value::pack`] / [`value::unpack`]
//!   round trip. This is a confirmed protocol fact; what the word *means* per
//!   parameter (int / float / packed) is still being decoded.
//!
//! The typed per-block rig model and the parameter catalog land in later steps
//! (see `docs/eleven-rack-roadmap.adoc`); for now this crate carries the pieces
//! the protocol layer needs.
//!
//! NOTE: Eleven Rack, Digidesign and Avid are trademarks of Avid Technology, Inc.
//! This is an independent, unofficial project; the names identify the hardware.
#![forbid(unsafe_code)]

pub mod error;
pub mod tfx;
pub mod value;

pub use error::{Error, Result};
pub use tfx::{Block, Param, Rig};
pub use value::{RawValue, VALUE_LEN};

/// A confirmed parameter address: the amp **Gain** knob.
///
/// The Eleven Rack addresses parameters with a multi-byte key; this is the one
/// address verified byte-for-byte against hardware (firmware `0157`), and anchors
/// the parameter catalog built in a later step. See
/// `docs/eleven-rack-sysex-protocol.adoc`.
pub const AMP_GAIN: [u8; 3] = [0x11, 0x21, 0x0D];
