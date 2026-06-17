//! The OUTPUT panel: master meters/fader/mute and the global DSP switches.
//!
//! The dB readout truncates the control value to an int; the loss is irrelevant.
#![allow(clippy::cast_possible_truncation)]

use eframe::egui;
use tascam_us16x08::{Control, Value};

use crate::app::App;
use crate::bridge::{METER_HEIGHT, fraction, meter_bar};

pub(crate) fn show(app: &mut App, ui: &mut egui::Ui) {
    ui.heading("Output");

    ui.label("Master");
    // Meters on the left; the fader column (vol label, fader, mute) on the right.
    ui.horizontal_top(|ui| {
        let (l, r) = app.meters().master_db();
        meter_bar(ui, fraction(l));
        meter_bar(ui, fraction(r));

        ui.vertical(|ui| {
            ui.label("vol");
            let mut volume = app.cached_int(Control::MasterVolume, 0);
            ui.spacing_mut().slider_width = METER_HEIGHT;
            let fader = egui::Slider::new(&mut volume, 0..=133)
                .vertical()
                .custom_formatter(|n, _| format!("{:+} dB", n as i32 - 127));
            if ui.add(fader).changed() {
                app.set(Control::MasterVolume, 0, Value::Int(volume));
            }

            let muted = app.cached_bool(Control::MasterMute, 0);
            if ui.selectable_label(muted, "MUTE").clicked() {
                app.set(Control::MasterMute, 0, Value::Bool(!muted));
            }
        });
    });

    ui.separator();

    let bypass = app.cached_bool(Control::DspBypass, 0);
    if ui.selectable_label(bypass, "DSP bypass").clicked() {
        app.set(Control::DspBypass, 0, Value::Bool(!bypass));
    }
    let buss = app.cached_bool(Control::BussOut, 0);
    if ui.selectable_label(buss, "Buss out").clicked() {
        app.set(Control::BussOut, 0, Value::Bool(!buss));
    }
}
