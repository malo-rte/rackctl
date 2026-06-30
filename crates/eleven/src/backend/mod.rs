//! Transport abstraction: the [`Transport`] trait and its implementations.
//!
//! [`Transport`] is the narrow seam between the typed device API and the actual
//! MIDI link. It works at the address/value level (read a value at an address,
//! write a value at an address), not at the raw `SysEx` byte level, so the device
//! layer never builds frames itself. Keeping it a trait lets the whole library
//! run against an in-memory [`MockTransport`] with no MIDI hardware or `libasound`
//! present (rust-coding-rules RS-80).

mod mock;
pub use mock::MockTransport;

#[cfg(feature = "alsa")]
mod rawmidi;
#[cfg(feature = "alsa")]
pub use rawmidi::RawMidi;

use rackctl_eleven_model::error::Result;
use rackctl_eleven_model::value::RawValue;

/// Address-mapped access to the Eleven Rack.
///
/// Implementors translate these calls into Digidesign `SysEx` (or, for the mock,
/// into an in-memory map). `addr` is the raw address bytes; the catalog's address
/// width is still being firmed up, so the seam takes a slice.
pub trait Transport {
    /// Read the value at `addr` (a read request, awaiting the reply).
    fn read(&mut self, addr: &[u8]) -> Result<RawValue>;

    /// Write `value` at `addr`.
    ///
    /// CAUTION: the Eleven Rack write opcode is *unconfirmed*; on hardware this
    /// should be used only as set + read-back + restore until verified.
    fn write(&mut self, addr: &[u8], value: &RawValue) -> Result<()>;
}

/// Lets an `Eleven<Box<dyn Transport>>` hold either transport chosen at runtime
/// (e.g. mock vs hardware behind a command-line flag) without a wrapper enum.
/// Generic over the boxed type, so it also covers `Box<dyn Transport + Send>`.
impl<T: Transport + ?Sized> Transport for Box<T> {
    fn read(&mut self, addr: &[u8]) -> Result<RawValue> {
        (**self).read(addr)
    }
    fn write(&mut self, addr: &[u8], value: &RawValue) -> Result<()> {
        (**self).write(addr, value)
    }
}
