//! The eframe application shell: device ownership and top-level layout.

use eframe::egui;
use tascam_us16x08::{Backend, Control, Us16x08, Value};

/// The running mixer application. Owns the device on the UI thread.
pub(crate) struct App {
    device: Us16x08<Box<dyn Backend>>,
    source: &'static str,
}

impl App {
    /// Build the app around an opened device. `mock` only affects the label.
    pub(crate) fn new(device: Us16x08<Box<dyn Backend>>, mock: bool) -> Self {
        Self {
            device,
            source: if mock {
                "mock device"
            } else {
                "US-16x08 (ALSA)"
            },
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("title").show(ctx, |ui| {
            ui.heading("Tascam US-16x08 mixer");
        });

        egui::TopBottomPanel::bottom("status").show(ctx, |ui| {
            ui.label(format!("device: {}", self.source));
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            // Minimal live read so M1 proves the device is open and responding;
            // the real bridge/editor arrive in later milestones.
            match self.device.get(Control::MasterVolume, 0) {
                Ok(Value::Int(v)) => ui.label(format!("master volume: {v}")),
                Ok(other) => ui.label(format!("master volume: {other:?}")),
                Err(e) => ui.colored_label(egui::Color32::RED, format!("read error: {e}")),
            };
            ui.weak("(meter bridge and channel editor arrive in later milestones)");
        });
    }
}
