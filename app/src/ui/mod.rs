//! The window: profile list, profile editor, restore browser, status bar.

mod activity;
mod app;
mod backups;
mod editor;
mod help;
mod history;
mod icon;
mod load;
mod platform;
mod restore;
mod settings;
mod shell;
mod theme;
mod tray;
mod tree;
mod widgets;

use crate::{instance::Primary, storage::Storage};

/// Window icon size, in pixels.
const WINDOW_ICON: u32 = 64;

/// Opens the window. `start_hidden`: started with Windows, it waits in the
/// notification area. `primary`: this window's single-instance claim, which
/// lets a second launch bring it forward.
pub fn run(storage: Storage, start_hidden: bool, primary: Option<Primary>) -> anyhow::Result<()> {
    let icon =
        eframe::egui::IconData { rgba: icon::rgba(WINDOW_ICON), width: WINDOW_ICON, height: WINDOW_ICON };
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title("Archeion")
            .with_icon(icon)
            .with_inner_size([960.0, 640.0])
            .with_min_inner_size([760.0, 480.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Archeion",
        options,
        Box::new(move |cc| {
            theme::apply(&cc.egui_ctx);
            Ok(Box::new(app::ArcheionApp::new(storage, &cc.egui_ctx, start_hidden, primary)))
        }),
    )
    .map_err(|e| anyhow::anyhow!("{e}"))
}
