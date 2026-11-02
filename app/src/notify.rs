//! Windows notifications about automatic backups.
//!
//! Two cases ([`archeion_core::Alerts`]): a scheduled or event backup that
//! fails, reported by the worker at the end of the run; and a profile with
//! no finished backup for N days, found by `archeion watch`, which Task
//! Scheduler starts every day and after logon. A profile is warned about
//! staleness at most once a day (`alerts.json` in the data folder).
//!
//! The toast is shown by Windows PowerShell through the WinRT notification
//! API, so no extra library is needed; until an installer registers an
//! application id with a Start menu shortcut, Windows shows it as coming
//! from "Windows PowerShell".

use crate::{history, i18n::Tr, i18n::arg, storage::Storage};
use archeion_core::{Profile, ProfileId};
use std::{collections::BTreeMap, fs, path::Path};

const STATE_FILE: &str = "alerts.json";
const DAY_MS: i64 = 24 * 60 * 60 * 1000;

/// Shows a notification; problems are logged, never raised: a missing
/// toast must not fail a backup.
pub fn toast(title: &str, body: &str) {
    if let Err(e) = show(title, body) {
        tracing::warn!(error = %e, "showing a notification");
    }
}

/// Text of the notification for a failed automatic backup of `profile`.
pub fn failure_text(tr: &Tr, profile: &Profile, problem: &str) -> (String, String) {
    let title = tr.t_args("alert-failed-title", &[("name", arg(profile.name.clone()))]);
    (title, problem.to_owned())
}

/// `archeion watch`: warns about every automatic profile whose last
/// finished backup is older than its `stale_days`. Profiles never backed
/// up are left alone (a new profile has not had its first run yet).
pub fn watch(storage: &Storage, tr: &Tr, profiles: &[Profile], now_ms: i64) {
    let runs = history::list(storage.dir(), history::KEEP_LOGS);
    let mut state = load_state(storage.dir());
    let mut changed = false;
    for p in profiles {
        let Some(last) = history::last_backup(&runs, p.id) else { continue };
        let Some(days) = stale_for(p, last, now_ms) else { continue };
        if state.get(&p.id).is_some_and(|&t| now_ms.saturating_sub(t) < DAY_MS) {
            continue;
        }
        let title = tr.t_args("alert-stale-title", &[("name", arg(p.name.clone()))]);
        let body =
            tr.t_args("alert-stale-body", &[("days", arg(days)), ("when", arg(history::local_time(last)))]);
        tracing::info!(profile = %p.id, days, "no recent backup: notifying");
        toast(&title, &body);
        state.insert(p.id, now_ms);
        changed = true;
    }
    if changed {
        save_state(storage.dir(), &state);
    }
}

/// Whole days since `last_ms` when that is at least the profile's limit
/// and the profile backs up automatically; `None` otherwise.
fn stale_for(p: &Profile, last_ms: i64, now_ms: i64) -> Option<i64> {
    if p.alerts.stale_days == 0 || !p.backs_up_automatically() {
        return None;
    }
    let days = now_ms.saturating_sub(last_ms) / DAY_MS;
    (days >= i64::from(p.alerts.stale_days)).then_some(days)
}

/// Time of the last staleness warning of each profile.
fn load_state(dir: &Path) -> BTreeMap<ProfileId, i64> {
    fs::read_to_string(dir.join(STATE_FILE))
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

fn save_state(dir: &Path, state: &BTreeMap<ProfileId, i64>) {
    let text = serde_json::to_string_pretty(state).unwrap_or_default();
    if let Err(e) = fs::write(dir.join(STATE_FILE), text) {
        tracing::warn!(error = %e, "saving the notification state");
    }
}

/// PowerShell script that shows the toast; the texts come from environment
/// variables, so they need no quoting.
const SCRIPT: &str = r#"
$ErrorActionPreference = 'Stop'
[Windows.UI.Notifications.ToastNotificationManager, Windows.UI.Notifications, ContentType = WindowsRuntime] | Out-Null
[Windows.Data.Xml.Dom.XmlDocument, Windows.Data.Xml.Dom.XmlDocument, ContentType = WindowsRuntime] | Out-Null
$title = [Security.SecurityElement]::Escape($env:ARCHEION_TOAST_TITLE)
$body = [Security.SecurityElement]::Escape($env:ARCHEION_TOAST_BODY)
$xml = New-Object Windows.Data.Xml.Dom.XmlDocument
$xml.LoadXml("<toast><visual><binding template='ToastGeneric'><text>$title</text><text>$body</text></binding></visual></toast>")
$app = '{1AC14E77-02E7-4E5D-B744-2EB1AE5198B7}\WindowsPowerShell\v1.0\powershell.exe'
[Windows.UI.Notifications.ToastNotificationManager]::CreateToastNotifier($app).Show([Windows.UI.Notifications.ToastNotification]::new($xml))
"#;

#[cfg(windows)]
fn show(title: &str, body: &str) -> std::io::Result<()> {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let status = std::process::Command::new("powershell.exe")
        .args(["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-EncodedCommand"])
        .arg(encode_command(SCRIPT))
        .env("ARCHEION_TOAST_TITLE", title)
        .env("ARCHEION_TOAST_BODY", body)
        .creation_flags(CREATE_NO_WINDOW)
        .status()?;
    if status.success() { Ok(()) } else { Err(std::io::Error::other(format!("powershell: {status}"))) }
}

#[cfg(not(windows))]
fn show(title: &str, body: &str) -> std::io::Result<()> {
    tracing::info!(title, body, "notification (shown on Windows only)");
    Ok(())
}

/// `-EncodedCommand` argument: the script as UTF-16 LE, in base64.
fn encode_command(script: &str) -> String {
    let bytes: Vec<u8> = script.encode_utf16().flat_map(u16::to_le_bytes).collect();
    base64(&bytes)
}

fn base64(bytes: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let (b1, b2) = (chunk.get(1).copied(), chunk.get(2).copied());
        let n = (usize::from(chunk.first().copied().unwrap_or(0)) << 16)
            | (usize::from(b1.unwrap_or(0)) << 8)
            | usize::from(b2.unwrap_or(0));
        let sextet = |shift: u32| char::from(TABLE[(n >> shift) & 63]);
        out.push(sextet(18));
        out.push(sextet(12));
        out.push(if b1.is_some() { sextet(6) } else { '=' });
        out.push(if b2.is_some() { sextet(0) } else { '=' });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use archeion_core::{Schedule, TimeOfDay};

    #[test]
    fn base64_matches_the_standard_alphabet_and_padding() {
        assert_eq!(base64(b""), "");
        assert_eq!(base64(b"f"), "Zg==");
        assert_eq!(base64(b"fo"), "Zm8=");
        assert_eq!(base64(b"foo"), "Zm9v");
        assert_eq!(base64(b"foobar"), "Zm9vYmFy");
        assert_eq!(encode_command("a"), "YQA=");
    }

    fn automatic(stale_days: u32) -> Profile {
        let mut p = Profile::new("Docs");
        p.schedule = Schedule::Daily { at: TimeOfDay::default() };
        p.alerts.stale_days = stale_days;
        p
    }

    #[test]
    fn staleness_counts_whole_days_and_needs_an_automatic_profile() {
        let now = 100 * DAY_MS;
        assert_eq!(stale_for(&automatic(3), now - 3 * DAY_MS, now), Some(3));
        assert_eq!(stale_for(&automatic(3), now - 3 * DAY_MS + 1, now), None);
        assert_eq!(stale_for(&automatic(0), 0, now), None, "0 days = off");
        let mut manual = automatic(3);
        manual.schedule = Schedule::Manual;
        assert_eq!(stale_for(&manual, 0, now), None);
        manual.events.at_logon = true;
        assert_eq!(stale_for(&manual, 0, now), Some(100));
    }

    #[test]
    fn the_warning_state_survives_a_restart() {
        let dir = tempfile::tempdir().unwrap();
        let id = ProfileId::new_random();
        let mut s = BTreeMap::new();
        s.insert(id, 42);
        save_state(dir.path(), &s);
        assert_eq!(load_state(dir.path()).get(&id), Some(&42));
        fs::write(dir.path().join(STATE_FILE), "garbage").unwrap();
        assert!(load_state(dir.path()).is_empty());
    }
}
