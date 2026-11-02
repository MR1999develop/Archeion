//! What differs between operating systems in the look of the window: the
//! interface font. The only place with `cfg(windows)` for the interface.

/// The two faces of the system interface font.
pub struct UiFonts {
    pub regular: Vec<u8>,
    pub semibold: Vec<u8>,
}

/// Segoe UI, read from the Windows fonts folder (never shipped: its
/// licence does not allow it); `None` elsewhere or when it is missing, and
/// egui's own font is used.
#[cfg(windows)]
pub fn ui_fonts() -> Option<UiFonts> {
    let dir = std::env::var_os("WINDIR")
        .map_or_else(|| "C:\\Windows".into(), std::path::PathBuf::from)
        .join("Fonts");
    let regular = std::fs::read(dir.join("segoeui.ttf")).ok()?;
    let semibold = std::fs::read(dir.join("seguisb.ttf")).ok()?;
    Some(UiFonts { regular, semibold })
}

#[cfg(not(windows))]
pub fn ui_fonts() -> Option<UiFonts> {
    None
}
