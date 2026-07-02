//! `eleven-audio-probe` — a throwaway libusb diagnostic for the Eleven Rack.
//!
//! Purpose: test the leading theory that the unit accepts host *parameter writes*
//! only while its (proprietary, vendor-class) USB **audio session is active**. If so,
//! activating audio is the key that unlocks the whole GUI parameter editor. See
//! `docs/eleven-rack-audio-driver-scope.adoc`.
//!
//! The Eleven (`0dba:b011`) exposes MIDI on interface 2 (`hw:2,0`, what
//! `rackctl-eleven` uses) and two **vendor-class** audio-streaming interfaces:
//!   * interface 3 — playback (host→unit), ISO EP `0x03`, alt 1
//!   * interface 4 — capture (unit→host), ISO EP `0x83`, alt 1
//! Both are vendor class (`0xFF`), so `snd-usb-audio` ignores them — no ALSA PCM —
//! and libusb can claim them from userspace without fighting a kernel driver. This
//! coexists with MIDI on interface 2 (a different interface).
//!
//! Three escalating hypotheses for what the edit-gate needs (cheapest first):
//!   * **H1** — just claim interfaces 3/4 and select their streaming alt-setting.
//!     (this binary, default)
//!   * **H2** — additionally submit ISO packets (silence) at the right cadence.
//!     (`--iso`; not yet implemented — see [`stream_iso`])
//!   * **H3** — correctly formatted PCM + clock/sync (the full driver).
//!
//! Usage: run this to hold the session open, then in another terminal attempt a
//! `rackctl-eleven set …` / `dump` and check whether the write now *sticks*.
//!
//! NOTE: raw USB access typically needs root or a udev rule for `0dba:b011`
//! (`SUBSYSTEM=="usb", ATTR{idVendor}=="0dba", ATTR{idProduct}=="b011", MODE="0660",
//! TAG+="uaccess"`). Run with `sudo` if it can't open the device.

use std::thread::sleep;
use std::time::Duration;

use anyhow::{Context, Result, bail};
use clap::Parser;
use rusb::{DeviceHandle, GlobalContext};

/// Digidesign Eleven Rack.
const VID: u16 = 0x0dba;
const PID: u16 = 0xb011;
/// The vendor audio-control interface (target of the class SET_CUR writes below)
/// and the two audio-streaming interfaces (playback, capture).
const CONTROL_IFACE: u8 = 1;
const AUDIO_IFACES: [u8; 2] = [3, 4];
/// The alt-setting that exposes each interface's ISO endpoint (alt 0 = idle).
const ACTIVE_ALT: u8 = 1;

/// Timeout for control transfers.
const CTRL_TIMEOUT: Duration = Duration::from_millis(500);

/// The class-specific audio-control writes the Windows driver sends right after
/// activating the streaming interfaces, transcribed verbatim from the cold-connect
/// capture (`11/eleven-save-20260702-114348.pcapng`, t≈14.18 s). All are
/// host→device class-interface requests (`bmRequestType 0x21`) to interface 1.
/// `(bRequest, wValue, wIndex, data)`:
///   * SET_CUR entity 0x80 = 01            (enable?)
///   * SET_CUR entity 0x81 = 44 AC 00 00   (sample rate 0xAC44 = 44100 Hz)
///   * SET_CUR entity 0x80 = 01
///   * bReq 3  entity 0x20 = 02
const CONTROL_WRITES: &[(u8, u16, u16, &[u8])] = &[
    (0x01, 0x0100, 0x8001, &[0x01]),
    (0x01, 0x0100, 0x8101, &[0x44, 0xAC, 0x00, 0x00]),
    (0x01, 0x0100, 0x8001, &[0x01]),
    (0x03, 0x0000, 0x2001, &[0x02]),
];
/// ISO endpoints and packet size, for H2. OUT = playback, IN = capture; 416-byte
/// packets at interval 1 (8 packets/ms at high speed), 32-bit PCM.
const ISO_OUT_EP: u8 = 0x03;
const ISO_IN_EP: u8 = 0x83;
const ISO_PACKET_BYTES: usize = 416;

/// libusb probe to activate the Eleven Rack audio session and test the edit gate.
#[derive(Parser)]
#[command(name = "eleven-audio-probe", version, about)]
struct Cli {
    /// Seconds to hold the audio session active before releasing — keep it running
    /// while you test a MIDI param-write in another terminal.
    #[arg(long, default_value_t = 30)]
    hold: u64,
    /// Also stream ISO packets (hypothesis H2). Not yet implemented — see the
    /// doc-comment on `stream_iso` for how to add it (nusb or libusb1-sys async ISO).
    #[arg(long)]
    iso: bool,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let handle = rusb::open_device_with_vid_pid(VID, PID).with_context(|| {
        format!("Eleven Rack {VID:04x}:{PID:04x} not found (or no USB permission — try sudo / a udev rule)")
    })?;
    // Let libusb detach any kernel driver as we claim (harmless if none).
    handle.set_auto_detach_kernel_driver(true).ok();

    // --- H1: replay the Windows driver's audio-activation sequence ---
    // Claim the control interface (target of the class writes) and both streaming
    // interfaces. The MIDI interface (2, hw:2,0) is left alone, so rackctl-eleven
    // keeps working in parallel.
    let mut claimed: Vec<u8> = Vec::new();
    for &iface in &[CONTROL_IFACE, AUDIO_IFACES[0], AUDIO_IFACES[1]] {
        handle.detach_kernel_driver(iface).ok(); // ignore "no driver"
        handle.claim_interface(iface).with_context(|| {
            format!(
                "claiming interface {iface}. If this is `Resource busy`, another userspace \
                 process still holds the device — e.g. VirtualBox USB passthrough. Fully \
                 release the Eleven from any VM / USB proxy so Linux owns it, then retry."
            )
        })?;
        claimed.push(iface);
    }
    // Idle-reset then activate the streaming interfaces (mirrors the capture:
    // SET_INTERFACE(3,0)/(4,0) then (3,1)/(4,1)).
    for &iface in &AUDIO_IFACES {
        handle.set_alternate_setting(iface, 0).ok();
    }
    for &iface in &AUDIO_IFACES {
        handle
            .set_alternate_setting(iface, ACTIVE_ALT)
            .with_context(|| format!("interface {iface} -> alt {ACTIVE_ALT}"))?;
        println!("interface {iface}: alt {ACTIVE_ALT} (streaming endpoint active)");
    }
    // The class audio-control setup (sample rate 44100 + enable flags).
    for &(req, value, index, data) in CONTROL_WRITES {
        handle
            .write_control(0x21, req, value, index, data, CTRL_TIMEOUT)
            .with_context(|| format!("class write bReq={req} wIndex={index:#06x}"))?;
        println!("  class write bReq={req} wIndex={index:#06x} data={data:02x?} ok");
    }
    println!("\nH1 active: audio interfaces activated + class control setup replayed.");

    // --- H2 (optional): also push ISO packets so the unit sees a live stream ---
    if cli.iso {
        stream_iso(&handle)?;
    }

    println!(
        "\n>>> While this holds the session, test the edit gate in another terminal:\n\
         >>>   rackctl-eleven --port hw:2,0 --midi-log /tmp/g.log set '11 78 0d' 3f\n\
         >>>   # then re-read / `dump` and check whether the write STICKS.\n\
         Holding for {}s (Ctrl-C to stop early; cleanup still runs on normal exit)...",
        cli.hold
    );
    sleep(Duration::from_secs(cli.hold));

    // --- cleanup: idle alt, release, reattach any kernel driver ---
    for &iface in claimed.iter().rev() {
        handle.set_alternate_setting(iface, 0).ok();
        handle.release_interface(iface).ok();
        handle.attach_kernel_driver(iface).ok();
    }
    println!("released — audio session closed.");
    Ok(())
}

/// Hypothesis **H2**: stream ISO packets so the unit sees a live audio session.
///
/// NOT IMPLEMENTED — `rusb`'s safe API has no isochronous support. To add it:
///   * Preferred: use the `nusb` crate (pure-Rust, async, supports ISO transfers).
///   * Or drop to `libusb1-sys` in `unsafe`: `libusb_alloc_transfer(n_pkts)` +
///     `libusb_fill_iso_transfer` for EP `0x03` (OUT, silence buffer) and `0x83`
///     (IN), `libusb_set_iso_packet_lengths(t, 416)`, submit, then pump
///     `libusb_handle_events` in a loop.
///   * Cadence: interval 1 at high speed = 8 packets/ms; start with all-zero
///     (silence) payloads of [`ISO_PACKET_BYTES`] on [`ISO_OUT_EP`], and drain
///     [`ISO_IN_EP`]. If the gate opens on silence, the audio content never needs
///     decoding (that would only be H3 — a full driver).
fn stream_iso(_handle: &DeviceHandle<GlobalContext>) -> Result<()> {
    let _ = (ISO_OUT_EP, ISO_IN_EP, ISO_PACKET_BYTES);
    bail!(
        "--iso (H2) not implemented: rusb has no ISO API. Implement via nusb or \
         libusb1-sys async ISO — see the stream_iso() doc-comment and \
         docs/eleven-rack-audio-driver-scope.adoc"
    );
}
