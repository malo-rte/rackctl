//! `tascam-mixer` — graphical mixer for the Tascam US-16x08.

mod app;
mod bridge;
mod channel;
mod config;
mod curves;
mod output;
mod routing;

use anyhow::Result;
use tascam_us16x08::{Backend, MockBackend, Us16x08};

#[cfg(feature = "alsa")]
use tascam_us16x08::AlsaBackend;

/// Open the device as a boxed backend: the in-memory mock, or real hardware.
fn open_device(mock: bool) -> Result<Us16x08<Box<dyn Backend>>> {
    if mock {
        return Ok(Us16x08::new(Box::new(MockBackend::new())));
    }
    #[cfg(feature = "alsa")]
    {
        Ok(Us16x08::new(Box::new(AlsaBackend::open()?)))
    }
    #[cfg(not(feature = "alsa"))]
    {
        anyhow::bail!("built without ALSA support; re-run with --mock")
    }
}

fn main() -> Result<()> {
    let mock = std::env::args().skip(1).any(|a| a == "--mock");
    let device = open_device(mock)?;

    // Restore the saved window size before creating the window; an absent size
    // falls back to eframe's default.
    let mut viewport = eframe::egui::ViewportBuilder::default();
    if let Some([w, h]) = config::load().window {
        viewport = viewport.with_inner_size([w, h]);
    }
    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };
    eframe::run_native(
        "Tascam US-16x08 Mixer",
        options,
        Box::new(move |cc| {
            let app = app::App::new(device, mock);
            // Apply the saved zoom; Ctrl +/- adjusts from here and Save default
            // remembers it.
            cc.egui_ctx.set_zoom_factor(app.zoom());
            // Uniform slider length so the editor's value boxes line up.
            cc.egui_ctx
                .style_mut(|style| style.spacing.slider_width = 120.0);
            Ok(Box::new(app))
        }),
    )
    .map_err(|e| anyhow::anyhow!("GUI error: {e}"))?;
    Ok(())
}
