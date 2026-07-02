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

use std::sync::atomic::{AtomicBool, AtomicI32, AtomicUsize, Ordering};
use std::thread::sleep;
use std::time::Duration;

use anyhow::{Context, Result};
use clap::Parser;
use libusb1_sys as ffi;
use rusb::{DeviceHandle, GlobalContext, UsbContext};

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

    println!(
        "\n>>> While this holds the session, test the edit gate in another terminal:\n\
         >>>   rackctl-eleven --port hw:2,0 --midi-log /tmp/g.log set '11 78 0d' 3f\n\
         >>>   # then re-read / `dump` and check whether the write STICKS.\n\
         Holding for {}s (Ctrl-C to stop early; cleanup still runs on normal exit)...",
        cli.hold
    );

    // --- H2 (optional): stream ISO packets so the unit sees a live audio session;
    // this also does the "hold" (it pumps libusb events for `hold` seconds) ---
    if cli.iso {
        println!("H2: streaming ISO silence on EP {ISO_OUT_EP:#04x}/{ISO_IN_EP:#04x} ...");
        stream_iso(&handle, cli.hold);
    } else {
        sleep(Duration::from_secs(cli.hold));
    }

    // --- cleanup: idle alt, release, reattach any kernel driver ---
    for &iface in claimed.iter().rev() {
        handle.set_alternate_setting(iface, 0).ok();
        handle.release_interface(iface).ok();
        handle.attach_kernel_driver(iface).ok();
    }
    println!("released — audio session closed.");
    Ok(())
}

/// Isochronous packets per transfer, and outstanding transfers per direction.
const ISO_PKTS: usize = 8;
const ISO_DEPTH: usize = 8;

/// True while streaming; the completion callback resubmits transfers until this
/// clears, then lets them drain.
static STREAMING: AtomicBool = AtomicBool::new(true);
/// Count of transfers still owned by libusb (submitted, not yet finally completed).
static IN_FLIGHT: AtomicUsize = AtomicUsize::new(0);
/// Diagnostics: total completions seen, and completions whose status != COMPLETED.
static COMPLETED: AtomicUsize = AtomicUsize::new(0);
static ERRORED: AtomicUsize = AtomicUsize::new(0);
static LAST_STATUS: AtomicI32 = AtomicI32::new(0);

/// libusb completion callback: resubmit while streaming, else account it drained.
extern "system" fn on_iso(transfer: *mut ffi::libusb_transfer) {
    // SAFETY: `transfer` is a live libusb transfer libusb hands back on completion.
    unsafe {
        COMPLETED.fetch_add(1, Ordering::Relaxed);
        let status = (*transfer).status;
        if status != ffi::constants::LIBUSB_TRANSFER_COMPLETED {
            ERRORED.fetch_add(1, Ordering::Relaxed);
            LAST_STATUS.store(status, Ordering::Relaxed);
        }
        if STREAMING.load(Ordering::Relaxed) && ffi::libusb_submit_transfer(transfer) == 0 {
            return;
        }
    }
    IN_FLIGHT.fetch_sub(1, Ordering::Relaxed);
}

/// Hypothesis **H2**: stream ISO packets (silence out, drain in) so the unit sees a
/// live audio session, pumping libusb events for `hold_secs`. If the edit gate opens
/// while this runs, the audio *content* never needs decoding — that would be H3.
fn stream_iso(handle: &DeviceHandle<GlobalContext>, hold_secs: u64) {
    let raw = handle.as_raw();
    // Keep the packet buffers alive for the whole stream (their pointers live in the
    // transfers). Silence = zeros for OUT; a scratch sink for IN.
    let mut buffers: Vec<Vec<u8>> = Vec::new();
    let mut transfers: Vec<*mut ffi::libusb_transfer> = Vec::new();

    for &(endpoint, _label) in &[(ISO_OUT_EP, "out"), (ISO_IN_EP, "in")] {
        for _ in 0..ISO_DEPTH {
            let mut buf = vec![0u8; ISO_PKTS * ISO_PACKET_BYTES];
            // SAFETY: alloc a transfer with room for ISO_PKTS packet descriptors and
            // fill the fields libusb needs (the C `fill_iso_transfer` inline).
            let t = unsafe { ffi::libusb_alloc_transfer(ISO_PKTS as i32) };
            assert!(!t.is_null(), "libusb_alloc_transfer failed");
            unsafe {
                (*t).dev_handle = raw;
                (*t).endpoint = endpoint;
                (*t).transfer_type = ffi::constants::LIBUSB_TRANSFER_TYPE_ISOCHRONOUS;
                (*t).timeout = 1000;
                (*t).buffer = buf.as_mut_ptr();
                (*t).length = (ISO_PKTS * ISO_PACKET_BYTES) as i32;
                (*t).num_iso_packets = ISO_PKTS as i32;
                (*t).callback = on_iso;
                let descs =
                    std::slice::from_raw_parts_mut((*t).iso_packet_desc.as_mut_ptr(), ISO_PKTS);
                for d in descs {
                    d.length = ISO_PACKET_BYTES as u32;
                }
            }
            buffers.push(buf);
            transfers.push(t);
        }
    }

    // Submit them all, then pump events for the hold window.
    for &t in &transfers {
        // SAFETY: `t` is a filled, not-yet-submitted transfer.
        if unsafe { ffi::libusb_submit_transfer(t) } == 0 {
            IN_FLIGHT.fetch_add(1, Ordering::Relaxed);
        }
    }
    println!("  {} ISO transfers submitted", IN_FLIGHT.load(Ordering::Relaxed));

    let ctx = handle.context().as_raw();
    let start = std::time::Instant::now();
    while start.elapsed() < Duration::from_secs(hold_secs) {
        let tv = libc::timeval { tv_sec: 0, tv_usec: 100_000 };
        // SAFETY: valid context + timeval; drives the completion callbacks.
        unsafe { ffi::libusb_handle_events_timeout_completed(ctx, &tv, std::ptr::null_mut()) };
    }

    // Stop: cancel outstanding transfers and drain their final callbacks, then free.
    STREAMING.store(false, Ordering::Relaxed);
    for &t in &transfers {
        unsafe { ffi::libusb_cancel_transfer(t) };
    }
    let drain = std::time::Instant::now();
    while IN_FLIGHT.load(Ordering::Relaxed) > 0 && drain.elapsed() < Duration::from_secs(2) {
        let tv = libc::timeval { tv_sec: 0, tv_usec: 50_000 };
        unsafe { ffi::libusb_handle_events_timeout_completed(ctx, &tv, std::ptr::null_mut()) };
    }
    for &t in &transfers {
        unsafe { ffi::libusb_free_transfer(t) };
    }
    drop(buffers);
    println!(
        "  ISO stream stopped. completions={} errored={} last_status={}",
        COMPLETED.load(Ordering::Relaxed),
        ERRORED.load(Ordering::Relaxed),
        LAST_STATUS.load(Ordering::Relaxed),
    );
}
