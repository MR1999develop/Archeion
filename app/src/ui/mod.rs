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
mod symbol;
mod theme;
mod tray;
mod tree;
mod widgets;

use crate::{instance::Primary, storage::Storage};
use archeion_core::WindowSize;

/// Window icon size, in pixels.
const WINDOW_ICON: u32 = 64;
/// First size of the window, and the smallest it can be made.
const DEFAULT_SIZE: [f32; 2] = [960.0, 640.0];
const MIN_SIZE: [f32; 2] = [760.0, 480.0];

/// The size to open the window with: the saved one, never below the
/// minimum.
fn start_size(saved: Option<WindowSize>) -> [f32; 2] {
    saved
        .map_or(DEFAULT_SIZE, |s| [f32::from(s.width).max(MIN_SIZE[0]), f32::from(s.height).max(MIN_SIZE[1])])
}

/// The size to save for a window seen with this inner size: `None` while
/// it is minimized or not known yet. A maximized window keeps the size it
/// had before (`saved`), to come back to it.
pub(crate) fn seen_size(
    inner: Option<eframe::egui::Vec2>,
    maximized: bool,
    minimized: bool,
    saved: Option<WindowSize>,
) -> Option<WindowSize> {
    if minimized {
        return None;
    }
    if maximized {
        let before =
            saved.map_or((DEFAULT_SIZE[0], DEFAULT_SIZE[1]), |s| (f32::from(s.width), f32::from(s.height)));
        return Some(WindowSize { width: pixels(before.0), height: pixels(before.1), maximized: true });
    }
    let inner = inner.filter(|v| v.x >= 1.0 && v.y >= 1.0)?;
    Some(WindowSize { width: pixels(inner.x), height: pixels(inner.y), maximized: false })
}

#[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss, reason = "rounded and clamped to u16")]
fn pixels(v: f32) -> u16 {
    v.round().clamp(0.0, f32::from(u16::MAX)) as u16
}

/// Opens the window. `start_hidden`: started with Windows, it waits in the
/// notification area. `primary`: this window's single-instance claim, which
/// lets a second launch bring it forward.
pub fn run(storage: Storage, start_hidden: bool, primary: Option<Primary>) -> anyhow::Result<()> {
    let icon =
        eframe::egui::IconData { rgba: icon::rgba(WINDOW_ICON), width: WINDOW_ICON, height: WINDOW_ICON };
    // Unreadable settings are reported by the window itself.
    let saved = storage.load().ok().and_then(|s| s.window_size);
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title("Archeion")
            .with_icon(icon)
            .with_inner_size(start_size(saved))
            .with_min_inner_size(MIN_SIZE)
            .with_maximized(saved.is_some_and(|s| s.maximized)),
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

#[cfg(test)]
mod tests {
    use super::*;
    use eframe::egui::vec2;

    const SAVED: WindowSize = WindowSize { width: 1200, height: 800, maximized: false };

    #[test]
    fn the_window_opens_at_the_saved_size_never_below_the_minimum() {
        assert_eq!(start_size(None), DEFAULT_SIZE);
        assert_eq!(start_size(Some(SAVED)), [1200.0, 800.0]);
        let tiny = WindowSize { width: 10, height: 10, maximized: false };
        assert_eq!(start_size(Some(tiny)), MIN_SIZE);
    }

    #[test]
    fn the_size_seen_is_rounded_and_a_maximized_window_keeps_the_one_before() {
        let seen = seen_size(Some(vec2(1023.6, 700.2)), false, false, None);
        assert_eq!(seen, Some(WindowSize { width: 1024, height: 700, maximized: false }));
        let max = seen_size(Some(vec2(1920.0, 1040.0)), true, false, Some(SAVED));
        assert_eq!(max, Some(WindowSize { maximized: true, ..SAVED }));
        assert_eq!(seen_size(Some(vec2(1024.0, 700.0)), false, true, Some(SAVED)), None, "minimized");
        assert_eq!(seen_size(None, false, false, Some(SAVED)), None, "not known yet");
    }
}
