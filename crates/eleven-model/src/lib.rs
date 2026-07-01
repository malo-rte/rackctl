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
//! * [`param`] — the **parameter catalog**: every amp model's and effect's named
//!   controls, their MIDI CC / device index, and value semantics (knob / switch /
//!   stepped), transcribed from the User Guide's MIDI chapter. The amp section's
//!   wire addressing (`11 21 <cc>`) is hardware-confirmed; see [`param`].
//!
//! * [`patch`] — the **typed, block-structured patch model** ([`patch::Patch`]):
//!   a struct-per-block, named-field view built from a parsed `.tfx` file, grouped
//!   the same way as the GX-700's `typed::Patch` so both products present a patch
//!   identically. Effect values stay raw floats until their unit scaling is
//!   decoded; see the module and `docs/eleven-rack-rig-format.adoc`.
//!
//! NOTE: Eleven Rack, Digidesign and Avid are trademarks of Avid Technology, Inc.
//! This is an independent, unofficial project; the names identify the hardware.
#![forbid(unsafe_code)]

pub mod backup;
pub mod error;
pub mod param;
pub mod patch;
pub mod tfx;
pub mod value;

pub use backup::{BlockData, ParamRecord, PatchBackup, RestoreAction};
pub use error::{Error, Result};
/// The public `Patch` is the **typed, block-structured** model ([`patch::Patch`]);
/// the raw `.tfx` file form remains available as [`tfx::Patch`].
pub use patch::Patch;
pub use tfx::{Block, Param};
pub use value::{RawValue, VALUE_LEN};

/// A confirmed `SysEx` parameter address: the amp **Gain** knob, on the one amp
/// model whose block was captured at `0x21` (firmware `0157`).
///
/// The Eleven Rack addresses parameters with a multi-byte key `11 <block> <index>`
/// where the block byte is *model/slot-specific* and the index is a small
/// sequential offset — a **different namespace** from the MIDI CC numbers in
/// [`param`] (here the index `0x0D` and Gain's CC `13` happen to coincide, but in
/// general they do not). This is the one address verified byte-for-byte against
/// hardware. See the "Parameter catalog" section of
/// `docs/eleven-rack-sysex-protocol.adoc`.
pub const AMP_GAIN: [u8; 3] = [0x11, 0x21, 0x0D];
