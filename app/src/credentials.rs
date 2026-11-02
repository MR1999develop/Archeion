//! Passwords of encrypted profiles, kept in Windows Credential Manager
//! under "Archeion" with the profile id as user name. Only the Windows
//! account that saved a password can read it back, which is what lets
//! scheduled backups run unattended.

use archeion_core::ProfileId;
use archeion_engine::Password;

const SERVICE: &str = "Archeion";

fn entry(id: ProfileId) -> Result<keyring::Entry, String> {
    keyring::Entry::new(SERVICE, &id.to_string()).map_err(|e| e.to_string())
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
    read(&entry(id)?)
}

/// Saves (or replaces) the password of `id`.
pub fn set(id: ProfileId, password: &Password) -> Result<(), String> {
    entry(id)?.set_password(password.as_str()).map_err(|e| e.to_string())
}

/// Forgets the password of `id`; nothing to do when none is saved.
pub fn delete(id: ProfileId) -> Result<(), String> {
    match entry(id)?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(e.to_string()),
    }
}

/// The saved password, or `None` when there is none or it cannot be read
/// (the engine then reports that a password is needed).
pub fn get_or_none(id: ProfileId) -> Option<Password> {
    get(id).unwrap_or_else(|e| {
        tracing::warn!(error = %e, "reading the saved password");
        None
    })
}
