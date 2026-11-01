//! The window: profile list, profile editor, restore browser, status bar.

mod activity;
mod app;
mod editor;
mod history;
mod restore;
mod theme;
mod tree;

use crate::storage::Storage;

pub fn run(storage: Storage) -> anyhow::Result<()> {
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title("Archeion")
            .with_inner_size([960.0, 640.0])
            .with_min_inner_size([760.0, 480.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Archeion",
        options,
        Box::new(move |cc| {
            theme::apply(&cc.egui_ctx);
            Ok(Box::new(app::ArcheionApp::new(storage)))
        }),
    )
    .map_err(|e| anyhow::anyhow!("{e}"))
}
