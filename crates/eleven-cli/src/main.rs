//! `rackctl-eleven` — command-line control for the Avid/Digidesign Eleven Rack.
//!
//! Scaffold: it can read a parameter by raw address (over the mock, or over a
//! connected unit with `--port`) and list the rawmidi ports. The full scan / dump
//! / rig commands land with the read-path work (see `docs/eleven-rack-roadmap.adoc`).
#![forbid(unsafe_code)]

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};

#[cfg(feature = "alsa")]
use rackctl_eleven::RawMidi;
use rackctl_eleven::{Eleven, MockTransport, Transport};

/// Control the Avid/Digidesign Eleven Rack over MIDI System Exclusive.
#[derive(Parser)]
#[command(name = "rackctl-eleven", version, about)]
struct Cli {
    /// ALSA rawmidi port (`hw:CARD,DEV`) of the "Eleven Rack Rig" port. Omit to
    /// use the in-memory mock (no hardware needed).
    #[arg(long, global = true)]
    port: Option<String>,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Read the parameter at a hex address, e.g. `read "11 21 0D"`.
    Read {
        /// Address bytes in hex, space- or comma-separated.
        addr: String,
    },
    /// List the available ALSA rawmidi ports.
    Ports,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match &cli.command {
        Command::Read { addr } => read(cli.port.as_deref(), addr),
        Command::Ports => list_ports(),
    }
}

/// Read one parameter and print its raw bytes and decoded word.
fn read(port: Option<&str>, addr: &str) -> Result<()> {
    let bytes = parse_addr(addr)?;
    let mut dev = open_device(port)?;
    let raw = dev.read_raw(&bytes)?;
    let hex: Vec<String> = raw.as_bytes().iter().map(|b| format!("{b:02X}")).collect();
    let word = raw.decode();
    println!(
        "{} -> {}  (word {word:#x} = {word})",
        addr.trim(),
        hex.join(" ")
    );
    Ok(())
}

/// Parse `"11 21 0D"` (or comma-separated, with optional `0x`) into address bytes.
fn parse_addr(s: &str) -> Result<Vec<u8>> {
    let bytes: Result<Vec<u8>> = s
        .split([' ', ','])
        .filter(|t| !t.is_empty())
        .map(|t| {
            let h = t.strip_prefix("0x").unwrap_or(t);
            u8::from_str_radix(h, 16).with_context(|| format!("invalid hex byte {t:?}"))
        })
        .collect();
    let bytes = bytes?;
    if bytes.is_empty() {
        anyhow::bail!("empty address");
    }
    Ok(bytes)
}

/// Open the device: the mock when `port` is `None`, else the hardware port.
fn open_device(port: Option<&str>) -> Result<Eleven<Box<dyn Transport>>> {
    match port {
        Some(p) => open_hardware(p),
        None => Ok(Eleven::new(Box::new(MockTransport::new()))),
    }
}

#[cfg(feature = "alsa")]
fn open_hardware(port: &str) -> Result<Eleven<Box<dyn Transport>>> {
    Ok(Eleven::new(Box::new(RawMidi::open(port)?)))
}

#[cfg(not(feature = "alsa"))]
fn open_hardware(_port: &str) -> Result<Eleven<Box<dyn Transport>>> {
    anyhow::bail!("built without the `alsa` feature; only the in-memory mock is available")
}

#[cfg(feature = "alsa")]
fn list_ports() -> Result<()> {
    for p in RawMidi::ports()? {
        println!("{p}");
    }
    Ok(())
}

#[cfg(not(feature = "alsa"))]
fn list_ports() -> Result<()> {
    anyhow::bail!("built without the `alsa` feature; no rawmidi ports available")
}
