//! Parsing and formatting parameter values for the command line.
//!
//! Values are raw device units for now; display-unit conversion is deferred to
//! Stage 2 (see [`rackctl_gx700::param`]).

use anyhow::{Result, anyhow, bail};
use rackctl_gx700::{Kind, Param, Value};

/// Parse a user-supplied string into a [`Value`] for `param`.
///
/// Booleans accept `on`/`off`/`true`/`false`/`1`/`0`/`yes`/`no`. Integers parse
/// as a bare number. Enums accept an in-range index or a case-insensitive label.
pub(crate) fn parse_value(param: Param, input: &str) -> Result<Value> {
    match param.kind() {
        Kind::Bool => Ok(Value::Bool(parse_bool(input)?)),
        Kind::Int { .. } => input.parse::<i32>().map(Value::Int).map_err(|_| {
            anyhow!(
                "could not parse {input:?} as an integer for {}",
                param.key()
            )
        }),
        Kind::Enum { values, .. } => Ok(Value::Enum(parse_enum(values, input)?)),
        _ => bail!("unsupported parameter kind"),
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
    if let Ok(n) = input.parse::<i32>() {
        let len = i32::try_from(values.len()).unwrap_or(i32::MAX);
        if n >= 0 && n < len {
            return Ok(n);
        }
        bail!("index {n} out of range 0..{}", values.len());
    }
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

/// Format a parameter's value for display, expanding enum indices to their
/// label. Integers print as their raw device value.
pub(crate) fn format_value(param: Param, value: Value) -> String {
    match value {
        Value::Bool(b) => b.to_string(),
        Value::Int(_) => rackctl_gx700::units::display(param, value),
        Value::Enum(v) => {
            if let Kind::Enum { values, .. } = param.kind() {
                let label = usize::try_from(v).ok().and_then(|i| values.get(i)).copied();
                return label.map_or_else(|| v.to_string(), |l| format!("{l} ({v})"));
            }
            v.to_string()
        }
        _ => String::from("?"),
    }
}
