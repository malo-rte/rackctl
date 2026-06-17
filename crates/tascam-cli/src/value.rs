//! Parsing and formatting control values for the command line.

use anyhow::{Result, anyhow, bail};
use tascam_us16x08::{Control, Kind, Value};

/// Parse a user-supplied string into a [`Value`] appropriate for `kind`.
pub(crate) fn parse_value(kind: Kind, input: &str) -> Result<Value> {
    match kind {
        Kind::Bool => Ok(Value::Bool(parse_bool(input)?)),
        Kind::Int { .. } => input
            .parse::<i32>()
            .map(Value::Int)
            .map_err(|_| anyhow!("expected an integer, got {input:?}")),
        Kind::Enum { values, .. } => Ok(Value::Enum(parse_enum(values, input)?)),
        Kind::Meter => bail!("this control is read-only; use the `meters` command"),
        _ => bail!("unsupported control kind"),
    }
}

fn parse_bool(input: &str) -> Result<bool> {
    match input.to_ascii_lowercase().as_str() {
        "on" | "true" | "1" | "yes" => Ok(true),
        "off" | "false" | "0" | "no" => Ok(false),
        _ => bail!("expected a boolean (on/off, true/false, 1/0, yes/no), got {input:?}"),
    }
}

fn parse_enum(values: &[&str], input: &str) -> Result<i32> {
    // Accept an integer index within range...
    if let Ok(n) = input.parse::<i32>() {
        let len = i32::try_from(values.len()).unwrap_or(i32::MAX);
        if n >= 0 && n < len {
            return Ok(n);
        }
        bail!("index {n} out of range 0..{}", values.len());
    }
    // ...or a label, matched case-insensitively.
    for (i, label) in values.iter().enumerate() {
        if label.eq_ignore_ascii_case(input) {
            return Ok(i32::try_from(i).unwrap_or(i32::MAX));
        }
    }
    bail!(
        "unknown value {input:?}; expected one of: {}",
        values.join(", ")
    )
}

/// Format a control's value for display, expanding enum indices to their label.
pub(crate) fn format_value(control: Control, value: Value) -> String {
    match value {
        Value::Bool(b) => b.to_string(),
        Value::Int(v) => v.to_string(),
        Value::Enum(v) => {
            if let Kind::Enum { values, .. } = control.kind() {
                let label = usize::try_from(v).ok().and_then(|i| values.get(i)).copied();
                return label.map_or_else(|| v.to_string(), |l| format!("{l} ({v})"));
            }
            v.to_string()
        }
        _ => String::from("?"),
    }
}
