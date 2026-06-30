//! The Eleven Rack parameter **value** codec.
//!
//! Every parameter value travels as a fixed [`VALUE_LEN`]-byte field inside the
//! `SysEx` frame: a word packed as five little-endian 7-bit groups (`b0` holds the
//! least-significant 7 bits). Five groups carry 35 bits, so the natural Rust
//! carrier is [`u64`].
//!
//! This *packing* is confirmed against hardware (e.g. the bytes `7F 7F 7F 7F 0F`
//! reconstruct to `0xFFFF_FFFF`). What the reconstructed word *means* for a given
//! parameter — a small integer knob position, a 32-bit float, or a packed
//! multi-field value — is **not yet uniform** and is still being decoded
//! (`docs/eleven-rack-sysex-protocol.adoc`). So this module deals only in the
//! lossless wire <-> word transform; interpretation belongs to the parameter
//! catalog built later.

/// Number of MIDI bytes in a packed parameter value.
pub const VALUE_LEN: usize = 5;

/// Number of value bits the five 7-bit groups carry (`5 * 7`).
pub const VALUE_BITS: u32 = 35;

/// The raw, on-the-wire form of a parameter value: five 7-bit MIDI data bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RawValue(pub [u8; VALUE_LEN]);

impl RawValue {
    /// Wrap five MIDI bytes as a raw value. The high bit of each byte is masked
    /// off, so the result is always valid MIDI data.
    #[must_use]
    pub fn from_bytes(bytes: [u8; VALUE_LEN]) -> Self {
        let mut b = bytes;
        for byte in &mut b {
            *byte &= 0x7f;
        }
        Self(b)
    }

    /// The five MIDI bytes, in transmission order (`b0` first).
    #[must_use]
    pub fn as_bytes(&self) -> &[u8; VALUE_LEN] {
        &self.0
    }

    /// Reconstruct the 35-bit word from the five 7-bit groups.
    #[must_use]
    pub fn decode(&self) -> u64 {
        unpack(&self.0)
    }

    /// Pack a 35-bit word into the five 7-bit groups. Bits above
    /// [`VALUE_BITS`] are discarded.
    #[must_use]
    pub fn encode(word: u64) -> Self {
        Self(pack(word))
    }
}

/// Pack a word into five little-endian 7-bit groups (`b0` = least significant).
///
/// Only the low [`VALUE_BITS`] bits are represented; higher bits are dropped.
#[must_use]
pub fn pack(word: u64) -> [u8; VALUE_LEN] {
    [
        low7(word),
        low7(word >> 7),
        low7(word >> 14),
        low7(word >> 21),
        low7(word >> 28),
    ]
}

/// Reconstruct the word from five little-endian 7-bit groups. Each byte's high
/// bit is ignored, so stray status bits cannot corrupt the result.
#[must_use]
pub fn unpack(bytes: &[u8; VALUE_LEN]) -> u64 {
    bytes
        .iter()
        .enumerate()
        .fold(0u64, |acc, (i, &b)| acc | (u64::from(b & 0x7f) << (7 * i)))
}

/// The low 7 bits of `word`, as a byte. The mask guarantees the value fits, so
/// the conversion never fails.
fn low7(word: u64) -> u8 {
    u8::try_from(word & 0x7f).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;

    #[test]
    fn known_hardware_vectors() {
        // Captured live from a unit (amp block scan).
        assert_eq!(unpack(&[0x40, 0, 0, 0, 0]), 0x40);
        assert_eq!(unpack(&[0x7f, 0x7f, 0x7f, 0x7f, 0x0f]), 0xFFFF_FFFF);
        // Amp Gain at one point: low 7 bits = 0x6D, plus a high tag bit (b4=0x10).
        assert_eq!(unpack(&[0x6d, 0, 0, 0, 0x10]), 0x6d | (0x10u64 << 28));
    }

    #[test]
    fn round_trips_word_then_bytes() {
        for &w in &[
            0u64,
            1,
            0x40,
            0x7f,
            0x80,
            0xFFFF_FFFF,
            (1 << VALUE_BITS) - 1,
        ] {
            assert_eq!(unpack(&pack(w)), w, "word {w:#x}");
        }
    }

    #[test]
    fn bits_above_capacity_are_dropped() {
        let w = 1u64 << VALUE_BITS; // first bit beyond the 35-bit field
        assert_eq!(unpack(&pack(w)), 0);
    }

    #[test]
    fn from_bytes_masks_high_bits() {
        // High bits set on every byte are stripped to valid MIDI data.
        let v = RawValue::from_bytes([0xFF, 0xFF, 0xFF, 0xFF, 0xFF]);
        assert_eq!(v.as_bytes(), &[0x7f, 0x7f, 0x7f, 0x7f, 0x7f]);
        assert_eq!(v.decode(), (1 << VALUE_BITS) - 1);
    }

    #[test]
    fn raw_value_encode_decode_round_trips() {
        let v = RawValue::encode(0x1234_5678);
        assert_eq!(v.decode(), 0x1234_5678);
    }
}
