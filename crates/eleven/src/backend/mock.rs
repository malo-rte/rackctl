//! An in-memory [`Transport`] for development and tests.

use std::collections::BTreeMap;

use super::Transport;
use rackctl_eleven_model::error::Result;
use rackctl_eleven_model::value::RawValue;

/// In-memory stand-in for a real Eleven Rack over MIDI.
///
/// A write stores the value at its address; a read returns the stored value, or
/// the all-zero value (decoding to `0`) for an address never written. No
/// parameter catalog is seeded yet, so the store starts empty.
#[derive(Debug, Clone, Default)]
pub struct MockTransport {
    store: BTreeMap<Vec<u8>, RawValue>,
}

impl MockTransport {
    /// Build an empty mock.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

impl Transport for MockTransport {
    fn read(&mut self, addr: &[u8]) -> Result<RawValue> {
        Ok(self.store.get(addr).copied().unwrap_or_default())
    }

    fn write(&mut self, addr: &[u8], value: &RawValue) -> Result<()> {
        self.store.insert(addr.to_vec(), *value);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;

    #[test]
    fn write_then_read_round_trips() {
        let mut mock = MockTransport::new();
        let addr = [0x11, 0x21, 0x0D];
        let value = RawValue::encode(0x6D);
        mock.write(&addr, &value).unwrap();
        assert_eq!(mock.read(&addr).unwrap(), value);
    }

    #[test]
    fn unwritten_address_reads_zero() {
        let mut mock = MockTransport::new();
        assert_eq!(mock.read(&[0x00, 0x00, 0x00]).unwrap().decode(), 0);
    }
}
