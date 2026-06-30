//! The real Eleven Rack [`Transport`], layering Digidesign read/write `SysEx`
//! over the byte-level [`MidiPort`] link from `rackctl-midi`.
//!
//! This file owns only the *Digidesign-specific* protocol: building read/write
//! messages, framing replies with [`crate::sysex::Framer`], and matching a reply
//! to its request address. Opening the port, listing ports, the advisory lock and
//! the raw byte I/O all live in `rackctl-midi`.
//!
//! This path is exercised only on hardware; CI and tests use the mock.

use std::thread::sleep;
use std::time::Duration;

use rackctl_midi::MidiPort;

use super::Transport;
use crate::sysex::{self, Framer, READ_REPLY};
use rackctl_eleven_model::error::{Error, Result};
use rackctl_eleven_model::value::RawValue;

/// Pause between non-blocking read polls while waiting for a reply.
const POLL_INTERVAL: Duration = Duration::from_millis(1);

/// How many [`POLL_INTERVAL`] polls to wait for a read reply before giving up
/// (about 500 ms).
const REPLY_POLLS: u32 = 500;

/// Consecutive silent [`POLL_INTERVAL`] polls that end an input drain (~20 ms).
const DRAIN_QUIET_POLLS: u32 = 20;

/// Fold a byte-level link error into this crate's [`Error`]. (The shared `Error`
/// lives in `rackctl-eleven-model`, so a blanket `From<MidiError>` would be an
/// orphan impl; map it here at the protocol edge instead.)
fn midi_err(e: rackctl_midi::MidiError) -> Error {
    match e {
        rackctl_midi::MidiError::PortBusy(p) => Error::PortBusy(p),
        rackctl_midi::MidiError::PortNotFound(p) => Error::PortNotFound(p),
        rackctl_midi::MidiError::Io(s) => Error::Transport(s),
    }
}

/// A live connection to an Eleven Rack: the Digidesign protocol over a
/// [`MidiPort`] (the "Eleven Rack Rig" rawmidi port).
#[derive(Debug)]
pub struct RawMidi {
    port: MidiPort,
    device_id: u8,
}

impl RawMidi {
    /// Enumerate the ALSA rawmidi ports available on the system, as
    /// `hw:CARD,DEV` strings suitable for [`Self::open`].
    ///
    /// # Errors
    /// [`Error::Transport`] if ALSA reports an error while iterating ports.
    pub fn ports() -> Result<Vec<String>> {
        MidiPort::list_ports().map_err(midi_err)
    }

    /// Open the rawmidi port at `port` (a `hw:CARD,DEV` address) for both input
    /// and output.
    ///
    /// # Errors
    /// [`Error::PortBusy`] if another rackctl process already holds this port;
    /// [`Error::PortNotFound`] if the address is invalid;
    /// [`Error::Transport`] if ALSA cannot open the stream.
    pub fn open(port: &str) -> Result<Self> {
        Ok(Self {
            port: MidiPort::open(port).map_err(midi_err)?,
            device_id: sysex::DEFAULT_DEVICE_ID,
        })
    }

    /// Print every incoming complete `SysEx` message as hex, one per line, until
    /// interrupted. A reverse-engineering aid for mapping the addresses the unit
    /// emits when its knobs move.
    ///
    /// # Errors
    /// [`Error::Transport`] if a read fails for a reason other than no data yet.
    pub fn watch_sysex(&mut self) -> Result<()> {
        let mut framer = Framer::new();
        let mut buf = [0u8; 256];
        loop {
            match self.port.read(&mut buf).map_err(midi_err)? {
                0 => sleep(POLL_INTERVAL),
                n => {
                    for msg in framer.push(buf.get(..n).unwrap_or(&[])) {
                        let hex: Vec<String> = msg.iter().map(|b| format!("{b:02X}")).collect();
                        println!("{}", hex.join(" "));
                    }
                }
            }
        }
    }

    /// Discard any pending input, so a stale reply cannot be mistaken for the
    /// answer to the next request.
    fn drain_input(&mut self) {
        let mut buf = [0u8; 256];
        let mut quiet = 0u32;
        while quiet < DRAIN_QUIET_POLLS {
            match self.port.read(&mut buf) {
                Ok(n) if n > 0 => quiet = 0,
                _ => {
                    quiet = quiet.saturating_add(1);
                    sleep(POLL_INTERVAL);
                }
            }
        }
    }
}

impl Transport for RawMidi {
    fn read(&mut self, addr: &[u8]) -> Result<RawValue> {
        self.drain_input();
        let msg = sysex::build_read_request(self.device_id, addr);
        self.port.write_all(&msg).map_err(midi_err)?;

        let mut framer = Framer::new();
        let mut buf = [0u8; 256];
        for _ in 0..REPLY_POLLS {
            match self.port.read(&mut buf).map_err(midi_err)? {
                0 => sleep(POLL_INTERVAL),
                n => {
                    for reply in framer.push(buf.get(..n).unwrap_or(&[])) {
                        let Ok(parsed) = sysex::parse(&reply) else {
                            continue;
                        };
                        if parsed.opcode != READ_REPLY {
                            continue;
                        }
                        if let Some(value) = parsed.value_at(addr) {
                            return Ok(value);
                        }
                    }
                }
            }
        }
        Err(Error::Timeout)
    }

    fn write(&mut self, addr: &[u8], value: &RawValue) -> Result<()> {
        let msg = sysex::build_write(self.device_id, addr, value);
        self.port.write_all(&msg).map_err(midi_err)?;
        Ok(())
    }
}
