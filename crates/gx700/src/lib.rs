//! Control-surface library for the **BOSS GX-700** guitar effects processor.
//!
//! The GX-700 is edited over DIN MIDI using Roland address-mapped System
//! Exclusive: `F0 41 <dev> 79 <cmd> <addr..> <data..> <checksum> F7`, where
//! `41` is Roland, `79` the GX-700 model id, `12` a DT1 (set) and `11` an RQ1
//! (request). This crate wraps that surface in a typed API: a parameter catalog
//! ([`param`]), a [`Transport`] seam with a mock and a real ALSA-rawmidi
//! implementation, a [`Gx700`] device facade, and a JSON [`Patch`] model.
//!
//! The `SysEx` codec ([`sysex`]) is split into a manufacturer-independent framer
//! and a Roland-specific builder/parser, so the generic half lifts cleanly into
//! a shared crate when more MIDI devices join the suite.
//!
//! # Backends
//!
//! All access goes through the [`Transport`] trait:
//! - [`RawMidi`] (feature `alsa`, on by default) talks to real hardware via
//!   ALSA rawmidi.
//! - [`MockTransport`] is an in-memory stand-in needing no MIDI port or
//!   `libasound`, for development and tests.
//!
//! # Example
//!
//! ```
//! use rackctl_gx700::{Gx700, MockTransport, Param, Value};
//!
//! let mut dev = Gx700::new(MockTransport::new());
//! let preamp_gain = Param::from_key("preamp-gain").expect("known parameter");
//!
//! dev.set(preamp_gain, Value::Int(80))?;
//! assert_eq!(dev.get(preamp_gain)?, Value::Int(80));
//! # Ok::<(), rackctl_gx700::Error>(())
//! ```
//!
//! # Stage status
//!
//! The parameter catalog's addresses and ranges are **provisional**: they are
//! decoded from a reverse-engineered patch parser and are verified against
//! hardware in Stage 2 (see [`param`]).

mod backend;
mod device;
mod error;
mod patch;

pub mod param;
pub mod sysex;

#[cfg(feature = "alsa")]
pub use backend::RawMidi;
pub use backend::{MockTransport, Transport};
pub use device::Gx700;
pub use error::{Error, Result};
pub use param::{Kind, Param, Value};
pub use patch::{PATCH_VERSION, Patch, Scalar};
pub use sysex::{Framer, RolandMessage};
