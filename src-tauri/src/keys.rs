// Secrets in Windows Credential Manager: one JSON value per service (Last.fm, Spotify).
use serde::{de::DeserializeOwned, Serialize};

/// The app was called Sonora before; its entries are carried over on first use.
const OLD: (&str, &str) = ("Confluence Player", "Sonora");

fn entry(service: &str) -> Result<keyring::Entry, String> {
    keyring::Entry::new(service, "default").map_err(|e| e.to_string())
}

fn read<T: DeserializeOwned>(service: &str) -> Option<T> {
    entry(service).ok()?.get_password().ok().and_then(|s| serde_json::from_str(&s).ok())
}

pub fn load<T: DeserializeOwned + Serialize>(service: &str) -> Option<T> {
    read(service).or_else(|| {
        let v: T = read(&service.replace(OLD.0, OLD.1))?;
        let _ = store(service, &v);
        Some(v)
    })
}

pub fn store<T: Serialize>(service: &str, value: &T) -> Result<(), String> {
    entry(service)?.set_password(&serde_json::to_string(value).map_err(|e| e.to_string())?).map_err(|e| e.to_string())
}

/// Also forgets the old-name entry, or the next load would bring it back.
pub fn forget(service: &str) -> Result<(), String> {
    for s in [service.to_string(), service.replace(OLD.0, OLD.1)] {
        match entry(&s)?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => {}
            Err(e) => return Err(e.to_string()),
        }
    }
    Ok(())
}
