//! Passwords of encrypted profiles, kept in Windows Credential Manager
//! under "Archeion" with the profile id as user name (passwords saved under
//! "Archeion", the program's first name, move there when first read). Only the Windows
//! account that saved a password can read it back, which is what lets
//! scheduled backups run unattended.

use archeion_core::ProfileId;
use archeion_engine::Password;

const SERVICE: &str = "Archeion";
const LEGACY_SERVICE: &str = "Archeion";

fn entry_in(service: &str, id: ProfileId) -> Result<keyring::Entry, String> {
    keyring::Entry::new(service, &id.to_string()).map_err(|e| e.to_string())
}

fn entry(id: ProfileId) -> Result<keyring::Entry, String> {
    entry_in(SERVICE, id)
}

fn read(entry: &keyring::Entry) -> Result<Option<Password>, String> {
    match entry.get_password() {
        Ok(text) => Ok(Password::new(text)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(e.to_string()),
    }
}

/// The saved password of `id`, if any.
pub fn get(id: ProfileId) -> Result<Option<Password>, String> {
    if let Some(p) = read(&entry(id)?)? {
        return Ok(Some(p));
    }
    let legacy = entry_in(LEGACY_SERVICE, id)?;
    let Some(p) = read(&legacy)? else { return Ok(None) };
    // Move it under the new name; if that fails the old entry still works.
    if set(id, &p).is_ok() {
        let _ = legacy.delete_credential();
    }
    Ok(Some(p))
}

/// Saves (or replaces) the password of `id`.
pub fn set(id: ProfileId, password: &Password) -> Result<(), String> {
    entry(id)?.set_password(password.as_str()).map_err(|e| e.to_string())
}

/// Forgets the password of `id`; nothing to do when none is saved.
pub fn delete(id: ProfileId) -> Result<(), String> {
    for service in [SERVICE, LEGACY_SERVICE] {
        match entry_in(service, id)?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => {}
            Err(e) => return Err(e.to_string()),
        }
    }
    Ok(())
}

/// The saved password, or `None` when there is none or it cannot be read
/// (the engine then reports that a password is needed).
pub fn get_or_none(id: ProfileId) -> Option<Password> {
    get(id).unwrap_or_else(|e| {
        tracing::warn!(error = %e, "reading the saved password");
        None
    })
}
