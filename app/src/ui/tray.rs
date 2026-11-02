//! The icon in the Windows notification area. A left click (or "Open
//! Archeion" in its menu) shows the window; "Quit" closes Archeion. The
//! tooltip says when a run is going. Clicks arrive on the window's thread
//! and are passed on through a channel; each one wakes the window, which
//! runs even while hidden.

use crate::i18n::Tr;
use eframe::egui;

/// What the user asked from the icon.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrayCommand {
    Show,
    Quit,
}

#[cfg(windows)]
pub use windows::Tray;

#[cfg(not(windows))]
pub use other::Tray;

#[cfg(windows)]
mod windows {
    use super::{Tr, TrayCommand, egui};
    use std::sync::mpsc;
    use tray_icon::{
        Icon, MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent,
        menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem},
    };

    const OPEN_ID: &str = "archeion-open";
    const QUIT_ID: &str = "archeion-quit";
    const ICON_SIZE: u32 = 32;

    pub struct Tray {
        icon: TrayIcon,
        rx: mpsc::Receiver<TrayCommand>,
        busy: bool,
    }

    impl std::fmt::Debug for Tray {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("Tray").field("busy", &self.busy).finish_non_exhaustive()
        }
    }

    impl Tray {
        /// Adds the icon. Call once: the click handlers are process-wide.
        ///
        /// # Errors
        /// The icon could not be created (the caller keeps a normal window).
        pub fn new(ctx: &egui::Context, tr: &Tr) -> Result<Self, String> {
            let (tx, rx) = mpsc::channel();
            let send = {
                let ctx = ctx.clone();
                move |cmd| {
                    let _ = tx.send(cmd);
                    ctx.request_repaint();
                }
            };
            let on_menu = send.clone();
            MenuEvent::set_event_handler(Some(move |e: MenuEvent| {
                if e.id == OPEN_ID {
                    on_menu(TrayCommand::Show);
                } else if e.id == QUIT_ID {
                    on_menu(TrayCommand::Quit);
                }
            }));
            TrayIconEvent::set_event_handler(Some(move |e: TrayIconEvent| match e {
                TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                }
                | TrayIconEvent::DoubleClick { button: MouseButton::Left, .. } => send(TrayCommand::Show),
                _ => {}
            }));

            let menu = Menu::new();
            let open = MenuItem::with_id(OPEN_ID, tr.t("tray-open"), true, None);
            let quit = MenuItem::with_id(QUIT_ID, tr.t("tray-quit"), true, None);
            for item in [&open as &dyn tray_icon::menu::IsMenuItem, &PredefinedMenuItem::separator(), &quit] {
                menu.append(item).map_err(|e| e.to_string())?;
            }
            let icon = Icon::from_rgba(crate::ui::icon::rgba(ICON_SIZE), ICON_SIZE, ICON_SIZE)
                .map_err(|e| e.to_string())?;
            let icon = TrayIconBuilder::new()
                .with_icon(icon)
                .with_tooltip("Archeion")
                .with_menu(Box::new(menu))
                .with_menu_on_left_click(false)
                .build()
                .map_err(|e| e.to_string())?;
            Ok(Self { icon, rx, busy: false })
        }

        /// The commands received since the last call.
        pub fn commands(&self) -> Vec<TrayCommand> {
            self.rx.try_iter().collect()
        }

        pub fn set_visible(&self, visible: bool) {
            if let Err(e) = self.icon.set_visible(visible) {
                tracing::warn!(error = %e, "showing or hiding the tray icon");
            }
        }

        /// Tooltip: "Archeion", or what is running.
        pub fn set_running(&mut self, running: Option<&str>) {
            if self.busy == running.is_some() {
                return;
            }
            self.busy = running.is_some();
            let text = running.map_or_else(|| "Archeion".to_owned(), |r| format!("Archeion – {r}"));
            if let Err(e) = self.icon.set_tooltip(Some(text)) {
                tracing::warn!(error = %e, "updating the tray tooltip");
            }
        }
    }
}

/// No notification-area icon outside Windows: the window closes normally.
#[cfg(not(windows))]
mod other {
    use super::{Tr, TrayCommand, egui};

    #[derive(Debug)]
    pub struct Tray;

    impl Tray {
        pub fn new(_ctx: &egui::Context, _tr: &Tr) -> Result<Self, String> {
            Err("no notification area on this system".to_owned())
        }

        pub fn commands(&self) -> Vec<TrayCommand> {
            Vec::new()
        }

        pub fn set_visible(&self, _visible: bool) {}

        pub fn set_running(&mut self, _running: Option<&str>) {}
    }
}
