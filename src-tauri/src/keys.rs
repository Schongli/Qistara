// Secrets in Windows Credential Manager: one JSON value per service (Last.fm, Spotify).
use serde::{de::DeserializeOwned, Serialize};

fn entry(service: &str) -> Result<keyring::Entry, String> {
    keyring::Entry::new(service, "default").map_err(|e| e.to_string())
}

pub fn load<T: DeserializeOwned>(service: &str) -> Option<T> {
    entry(service).ok()?.get_password().ok().and_then(|s| serde_json::from_str(&s).ok())
}

pub fn store<T: Serialize>(service: &str, value: &T) -> Result<(), String> {
    entry(service)?.set_password(&serde_json::to_string(value).map_err(|e| e.to_string())?).map_err(|e| e.to_string())
}

pub fn forget(service: &str) -> Result<(), String> {
    match entry(service)?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(e.to_string()),
    }
}
