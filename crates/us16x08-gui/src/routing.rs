//! The output routing tab: each physical line output picks one source.
#![allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]

use eframe::egui;
use rackctl_us16x08::{Control, Kind, NUM_OUTPUTS, Value};

use crate::app::App;

/// Render the 8-output source matrix.
pub(crate) fn show(app: &mut App, ui: &mut egui::Ui) {
    ui.heading("Output routing");
    ui.label("Each physical line output carries one source.");

    let Kind::Enum { values, .. } = Control::LineOutRoute.kind() else {
        return;
    };

    for out in 0..NUM_OUTPUTS {
        let current = app.cached_int(Control::LineOutRoute, out);
        let mut selected = current;
        let text = usize::try_from(current)
            .ok()
            .and_then(|i| values.get(i))
            .copied()
            .unwrap_or("?");
        ui.horizontal(|ui| {
            ui.label(format!("Line out {}", out + 1));
            egui::ComboBox::from_id_salt(("route", out))
                .selected_text(text)
                .show_ui(ui, |ui| {
                    for (i, name) in values.iter().enumerate() {
                        ui.selectable_value(&mut selected, i as i32, *name);
                    }
                });
        });
        if selected != current {
            app.set(Control::LineOutRoute, out, Value::Enum(selected));
        }
    }
}
