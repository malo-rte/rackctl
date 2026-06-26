//! Generate the per-block patch-format tables (AsciiDoc) from the param catalog,
//! for the protocol reference appendix. Throwaway doc tool.
//! Run: cargo run -p rackctl-gx700 --example gen_patch_format
#![allow(
    clippy::unwrap_used,
    clippy::print_stdout,
    clippy::indexing_slicing,
    clippy::doc_markdown
)]

use rackctl_gx700::{Kind, param};

/// Escape AsciiDoc cell-delimiter `|` in a label.
fn cell(s: &str) -> String {
    s.replace('|', "\\|")
}

fn main() {
    let mut current = "";
    for &p in param::ALL {
        let block = p.block_label();
        if block != current {
            if !current.is_empty() {
                println!("|===");
                println!();
            }
            let sub = p.patch_offset()[2];
            println!("=== {block} -- sub-block `{sub:02X}`");
            println!();
            println!(r#"[cols="1,3,6",options="header"]"#);
            println!("|===");
            println!("| Offset | Parameter | Type / values");
            current = block;
        }
        let off = p.patch_offset()[3];
        let val = match p.kind() {
            Kind::Bool => "bool (`0`=off, `1`=on)".to_owned(),
            Kind::Int { min, max, .. } => format!("int `{min}`..`{max}`"),
            Kind::Enum { values, .. } => {
                let items: Vec<String> = values
                    .iter()
                    .enumerate()
                    .map(|(i, s)| format!("`{i}`={}", cell(s)))
                    .collect();
                format!("enum -- {}", items.join(", "))
            }
            _ => "?".to_owned(),
        };
        println!("| `{off:02X}` | `{}` | {val}", p.key());
    }
    println!("|===");
}
