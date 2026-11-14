// Secrets in Windows Credential Manager: one JSON value per service (Last.fm, Spotify).
use serde::{de::DeserializeOwned, Serialize};

/// The app's earlier names, newest first: their entries are carried over on first use.
const NAME: &str = "Qistara";
const OLD: [&str; 2] = ["Confluence Player", "Sonora"];

/// "Qistara Spotify" → ["Confluence Player Spotify", "Sonora Spotify"]
fn old_names(service: &str) -> Vec<String> {
    match service.strip_prefix(NAME) {
        Some(rest) => OLD.iter().map(|o| format!("{o}{rest}")).collect(),
        None => vec![],
    }
}

fn entry(service: &str) -> Result<keyring::Entry, String> {
    keyring::Entry::new(service, "default").map_err(|e| e.to_string())
}
fn read<T: DeserializeOwned>(service: &str) -> Option<T> {
    entry(service).ok()?.get_password().ok().and_then(|s| serde_json::from_str(&s).ok())
}
pub fn load<T: DeserializeOwned + Serialize>(service: &str) -> Option<T> {
    read(service).or_else(|| {
        let v: T = old_names(service).iter().find_map(|o| read(o))?;
        let _ = store(service, &v);
        Some(v)
    })
}
pub fn store<T: Serialize>(service: &str, value: &T) -> Result<(), String> {
    entry(service)?.set_password(&serde_json::to_string(value).map_err(|e| e.to_string())?).map_err(|e| e.to_string())
}
/// Also forgets the old-name entries, or the next load would bring them back.
pub fn forget(service: &str) -> Result<(), String> {
    for s in std::iter::once(service.to_string()).chain(old_names(service)) {
        match entry(&s)?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => {}
            Err(e) => return Err(e.to_string()),
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_a_service_to_its_old_names() {
        assert_eq!(old_names("Qistara Spotify"), ["Confluence Player Spotify", "Sonora Spotify"]);
        assert!(old_names("Something else").is_empty());
    }
}
