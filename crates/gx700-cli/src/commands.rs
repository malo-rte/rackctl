//! Command handlers, generic over the [`Transport`] so the same logic drives
//! the mock and the real ALSA rawmidi device.

use std::fs;

use anyhow::{Context, Result};
use rackctl_gx700::{Gx700, Kind, Param, Transport, param};

use crate::value::{format_value, parse_value};

/// Print the full parameter catalog. Backend-independent.
pub(crate) fn list() {
    println!("{:<22} {:<18} {:<24} PATCH OFFSET", "KEY", "BLOCK", "KIND");
    for &p in param::ALL {
        println!(
            "{:<22} {:<18} {:<24} {}",
            p.key(),
            p.block_label(),
            kind_str(p),
            hex4(p.patch_offset()),
        );
    }
}

/// Format a 4-byte address as space-separated hex.
fn hex4(addr: [u8; 4]) -> String {
    addr.iter()
        .map(|b| format!("{b:02X}"))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Print detailed metadata for one parameter. Backend-independent.
pub(crate) fn info(key: &str) -> Result<()> {
    let p = resolve(key)?;
    println!("{}  ({})", p.key(), p.block_label());
    println!("  patch offset: {}", hex4(p.patch_offset()));
    println!("  live address: {}", hex4(p.address()));
    match p.kind() {
        Kind::Bool => println!("  kind:  bool (on/off/true/false/1/0/yes/no)"),
        Kind::Int { min, max, default } => {
            println!("  kind:  int (raw device units)");
            println!("  range: {min}..={max} (default {default})");
        }
        Kind::Enum { values, default } => {
            println!("  kind:  enum (default {default})");
            let listed: Vec<String> = values
                .iter()
                .enumerate()
                .map(|(i, v)| format!("{i}={v}"))
                .collect();
            println!("  values: {}", listed.join("  "));
        }
        _ => println!("  kind:  ?"),
    }
    Ok(())
}

/// Read and print one parameter's value.
pub(crate) fn get<T: Transport>(dev: &mut Gx700<T>, key: &str) -> Result<()> {
    let p = resolve(key)?;
    let value = dev.get(p)?;
    println!("{}", format_value(p, value));
    Ok(())
}

/// Parse and write one parameter's value. Silent on success.
pub(crate) fn set<T: Transport>(dev: &mut Gx700<T>, key: &str, raw_value: &str) -> Result<()> {
    let p = resolve(key)?;
    let value = parse_value(p, raw_value)?;
    dev.set(p, value)?;
    Ok(())
}

/// Capture the patch buffer to a JSON file, or to standard output if `path` is
/// `None`.
pub(crate) fn dump<T: Transport>(dev: &mut Gx700<T>, file: Option<&str>) -> Result<()> {
    let patch = dev.capture_patch()?;
    let json = serde_json::to_string_pretty(&patch).context("serializing patch")?;
    match file {
        Some(file) => fs::write(file, json).with_context(|| format!("writing {file:?}"))?,
        None => println!("{json}"),
    }
    Ok(())
}

/// Load a JSON patch file and apply it to the device.
pub(crate) fn load<T: Transport>(dev: &mut Gx700<T>, file: &str) -> Result<()> {
    let text = fs::read_to_string(file).with_context(|| format!("reading {file:?}"))?;
    let patch = serde_json::from_str(&text).with_context(|| format!("parsing {file:?}"))?;
    let applied = dev.apply_patch(&patch)?;
    eprintln!("applied {applied} parameter(s)");
    Ok(())
}

/// Select a patch memory by Program Change.
pub(crate) fn select<T: Transport>(dev: &mut Gx700<T>, n: u8) -> Result<()> {
    dev.select_patch(n)?;
    Ok(())
}

fn resolve(key: &str) -> Result<Param> {
    Param::from_key(key).with_context(|| format!("unknown parameter {key:?} (try `list`)"))
}

fn kind_str(p: Param) -> String {
    match p.kind() {
        Kind::Bool => "bool".to_owned(),
        Kind::Int { min, max, .. } => format!("int {min}..={max}"),
        Kind::Enum { values, .. } => format!("enum[{}]", values.len()),
        _ => "?".to_owned(),
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;
    use rackctl_gx700::MockTransport;

    fn dev() -> Gx700<MockTransport> {
        Gx700::new(MockTransport::new())
    }

    #[test]
    fn set_then_get_round_trips() {
        let mut d = dev();
        set(&mut d, "preamp-volume", "77").unwrap();
        // get() prints; assert via the device directly.
        let p = Param::from_key("preamp-volume").unwrap();
        assert_eq!(format_value(p, d.get(p).unwrap()), "77");
    }

    #[test]
    fn unknown_param_errors() {
        assert!(resolve("nonsuch").is_err());
    }
}
