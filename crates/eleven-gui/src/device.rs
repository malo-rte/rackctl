//! The Eleven Rack device, behind a boxed `Send` [`ElevenDevice`] so it can move to
//! the background bank-loader/writer threads and be swapped for the in-memory mock.

use std::path::Path;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use anyhow::Result;
use rackctl_eleven::{ElevenDevice, MockEleven};

#[cfg(feature = "alsa")]
use rackctl_eleven::{MidiPortInfo, RawMidi};

/// The Eleven Rack's ALSA card name, matched to auto-detect its port when none
/// was given.
#[cfg(feature = "alsa")]
const ELEVEN_MATCH: &str = "Eleven Rack";

/// The device, shared between the UI thread and the background threads. Each access
/// locks it briefly, so a slow bank read interleaves with UI actions.
pub(crate) type Device = Box<dyn ElevenDevice + Send>;
pub(crate) type SharedDevice = Arc<Mutex<Device>>;

/// Open the Eleven Rack: the in-memory mock, or a real ALSA rawmidi connection.
/// `port` is a `hw:CARD,DEV` address or a device-name substring; when `None`, the
/// Eleven Rack is auto-detected by name. When `midi_log` is set, a real connection
/// logs every MIDI byte in/out to that file (the `--midi-log` flag; ignored for
/// the mock).
pub(crate) fn open(mock: bool, port: Option<&str>, midi_log: Option<&Path>) -> Result<Device> {
    if mock {
        return Ok(Box::new(MockEleven::new()));
    }
    #[cfg(feature = "alsa")]
    {
        let addr = resolve_port(port)?;
        let mut dev = RawMidi::open(&addr).map_err(|e| anyhow::anyhow!("{e}"))?;
        if let Some(path) = midi_log {
            dev.enable_midi_log(path)
                .map_err(|e| anyhow::anyhow!("{e}"))?;
        }
        Ok(Box::new(dev))
    }
    #[cfg(not(feature = "alsa"))]
    {
        let _ = (port, midi_log);
        anyhow::bail!("built without ALSA support; re-run with --mock")
    }
}

/// Resolve `port` to a concrete `hw:CARD,DEV` address: an explicit `hw:…` address
/// is used verbatim; anything else is a device-name substring; `None` auto-detects
/// the Eleven Rack. If more than one device matches, the candidates are listed and
/// the user is asked to pick one with `--port` — nothing is chosen silently.
#[cfg(feature = "alsa")]
fn resolve_port(port: Option<&str>) -> Result<String> {
    if let Some(spec) = port {
        if spec.starts_with("hw:") {
            return Ok(spec.to_owned());
        }
        return Ok(select_one(find(spec)?, spec)?.addr);
    }
    Ok(select_one(find(ELEVEN_MATCH)?, ELEVEN_MATCH)?.addr)
}

/// Find the ports matching `spec`, mapping the MIDI error into `anyhow`.
#[cfg(feature = "alsa")]
fn find(spec: &str) -> Result<Vec<MidiPortInfo>> {
    RawMidi::find(spec).map_err(|e| anyhow::anyhow!("{e}"))
}

/// Pick exactly one port from the matches, or fail with guidance: none found, or
/// several found (list them and ask the user to select with `--port`).
#[cfg(feature = "alsa")]
fn select_one(mut matches: Vec<MidiPortInfo>, spec: &str) -> Result<MidiPortInfo> {
    match matches.len() {
        0 => anyhow::bail!(
            "no MIDI device matches {spec:?} (run `rackctl-eleven ports`, or use --mock)"
        ),
        1 => Ok(matches.remove(0)),
        _ => {
            let list = matches
                .iter()
                .map(|p| format!("  {}  {}", p.addr, p.name))
                .collect::<Vec<_>>()
                .join("\n");
            anyhow::bail!(
                "several devices match {spec:?}; select one with --port <hw:CARD,DEV>:\n{list}"
            )
        }
    }
}

/// A never-touched placeholder device for the disconnected/offline state, so the
/// app always holds a [`SharedDevice`] and can retry the real open.
pub(crate) fn placeholder() -> Device {
    Box::new(MockEleven::new())
}

/// Lock the shared device, recovering from a poisoned mutex rather than panicking.
pub(crate) fn lock(device: &SharedDevice) -> MutexGuard<'_, Device> {
    device.lock().unwrap_or_else(PoisonError::into_inner)
}
