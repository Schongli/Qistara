// Google Translate's free endpoint (translate_a/single, client=gtx), for lyrics translation and Learn-mode
// word lookups. It answers HTTP/2 clients like reqwest with 429 Too Many Requests and serves HTTP/1.1 fine,
// so requests go out here on an HTTP/1.1-only client rather than through the webview's HTTP plugin.
// Only the text to translate is sent; the endpoint and parameters are fixed.
use std::time::Duration;
use tauri_plugin_http::reqwest;

const ENDPOINT: &str = "https://translate.googleapis.com/translate_a/single";

/// Google language codes: "id", "en", "fil", "zh-CN", "pt-PT" (and "auto" as a source).
pub fn valid_lang(code: &str) -> bool {
    let mut parts = code.split('-');
    let base = parts.next().unwrap_or("");
    let region = parts.next();
    (2..=3).contains(&base.len())
        && base.bytes().all(|b| b.is_ascii_alphabetic())
        && region.is_none_or(|r| (2..=4).contains(&r.len()) && r.bytes().all(|b| b.is_ascii_alphanumeric()))
        && parts.next().is_none()
}

/// The query string: translation (`dt=t`), plus romanisation and dictionary entries for a word lookup.
pub fn query(from: &str, to: &str, dictionary: bool) -> Vec<(&'static str, String)> {
    let mut q = vec![("client", "gtx".to_string()), ("sl", from.to_string()), ("tl", to.to_string()), ("dt", "t".to_string())];
    if dictionary {
        q.push(("dt", "rm".into()));
        q.push(("dt", "bd".into()));
    }
    q
}

/// A client that speaks HTTP/1.1 only, with a 20 s timeout; Google and others turn reqwest's HTTP/2 away.
pub fn http1_client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder().http1_only().timeout(Duration::from_secs(20)).build().map_err(|e| e.to_string())
}

/// Google's raw JSON reply; the TypeScript side parses it (translate.ts, learn).
#[tauri::command]
pub async fn google_translate(text: String, from: String, to: String, dictionary: bool) -> Result<String, String> {
    if !(from == "auto" || valid_lang(&from)) || !valid_lang(&to) {
        return Err("Not a language code.".into());
    }
    if text.len() > 20_000 {
        return Err("Too much text to translate at once.".into());
    }
    let r = http1_client()?
        .post(ENDPOINT)
        .query(&query(&from, &to, dictionary))
        .header("User-Agent", crate::UA)
        .form(&[("q", text.as_str())])
        .send()
        .await
        .map_err(|e| format!("network: {e}"))?;
    let status = r.status().as_u16();
    let body = r.text().await.map_err(|e| e.to_string())?;
    match status {
        200 => Ok(body),
        429 => Err("Google Translate is busy. Try again later.".into()),
        s => Err(format!("Google Translate answered {s}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_only_language_codes() {
        for ok in ["id", "en", "fil", "zh-CN", "zh-TW", "pt-PT", "sr-Latn"] {
            assert!(valid_lang(ok), "{ok}");
        }
        for bad in ["", "e", "english", "en-", "en-US-x", "en&tl=x", "../x", "zh_CN"] {
            assert!(!valid_lang(bad), "{bad}");
        }
    }

    #[test]
    fn asks_for_dictionary_entries_only_for_lookups() {
        let dts = |d| query("auto", "en", d).into_iter().filter(|(k, _)| *k == "dt").map(|(_, v)| v).collect::<Vec<_>>();
        assert_eq!(dts(false), ["t"]);
        assert_eq!(dts(true), ["t", "rm", "bd"]);
        assert!(query("ja", "id", false).contains(&("sl", "ja".to_string())));
    }
}
