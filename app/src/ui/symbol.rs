//! The icons of the interface, from the Windows icon font Segoe MDL2
//! Assets (on Windows 10 and 11), so they match the system and each other.
//! Use only these constants; each code point was checked by drawing it.

pub const FOLDER: &str = "\u{F12B}";
pub const FILE: &str = "\u{E7C3}";
pub const DELETED: &str = "\u{E74D}";
/// Left out by an exclusion added after it was backed up.
pub const EXCLUDED: &str = "\u{E738}";
pub const REMOVE: &str = "\u{E711}";
pub const REFRESH: &str = "\u{E72C}";
pub const RUNNING: &str = "\u{E895}";
pub const WARNING: &str = "\u{E7BA}";
pub const ERROR: &str = "\u{EA39}";
pub const DONE: &str = "\u{E73E}";
pub const LOCKED: &str = "\u{E72E}";
pub const KEY: &str = "\u{E8D7}";
pub const TAG: &str = "\u{E8EC}";
pub const PATTERN: &str = "\u{E71C}";
pub const WAIT: &str = "\u{E823}";
/// Steps of a run: still to come, in progress (done is [`DONE`]).
pub const PENDING: &str = "\u{EA3A}";
pub const CURRENT: &str = "\u{EA3B}";

/// Every icon, for the test that each has a glyph.
#[cfg(test)]
pub const ALL: &[&str] = &[
    FOLDER, FILE, DELETED, EXCLUDED, REMOVE, REFRESH, RUNNING, WARNING, ERROR, DONE, LOCKED, KEY, TAG,
    PATTERN, WAIT, PENDING, CURRENT,
];

/// `icon` and `text` with the gap the interface uses between them.
pub fn with(icon: &str, text: impl std::fmt::Display) -> String {
    format!("{icon}  {text}")
}

/// The icons texts name (`{ $deleted-icon }` in "Files marked … deleted"),
/// as arguments of [`crate::i18n::Tr::t_args`].
pub fn args() -> [(&'static str, fluent_templates::fluent_bundle::FluentValue<'static>); 1] {
    [("deleted-icon", DELETED.into())]
}
