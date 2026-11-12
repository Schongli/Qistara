// Japanese lyrics for learners: splits lines into words with their dictionary form, reading and part of
// speech, using the IPADIC dictionary embedded in the app (Lindera). Offline; nothing is sent anywhere.
use lindera::{dictionary::load_dictionary, mode::Mode, segmenter::Segmenter};
use serde::Serialize;
use std::borrow::Cow;
use std::sync::OnceLock;

/// One word as IPADIC sees it. `reading` is katakana, empty when the word isn't in the dictionary
/// (English words, names). `pos`, `pos1` and `form` are IPADIC's Japanese tags; the UI turns them into English.
#[derive(Serialize, Debug, PartialEq)]
pub struct Word {
    pub text: String,
    pub pos: String,
    pub pos1: String,
    pub form: String,
    pub base: String,
    pub reading: String,
}

fn segmenter() -> Result<&'static Segmenter, String> {
    static SEG: OnceLock<Segmenter> = OnceLock::new();
    if let Some(s) = SEG.get() {
        return Ok(s);
    }
    let dict = load_dictionary("embedded://ipadic").map_err(|e| e.to_string())?;
    Ok(SEG.get_or_init(|| Segmenter::new(Mode::Normal, dict, None)))
}

fn field(d: &[&str], i: usize) -> String {
    d.get(i).filter(|v| **v != "*").map(|v| v.to_string()).unwrap_or_default()
}

/// Words of each line, in order. Spaces are dropped; everything else (punctuation too) is kept.
pub fn words(lines: &[String]) -> Result<Vec<Vec<Word>>, String> {
    let seg = segmenter()?;
    lines
        .iter()
        .map(|line| {
            let mut tokens = seg.segment(Cow::Borrowed(line.as_str())).map_err(|e| e.to_string())?;
            Ok(tokens
                .iter_mut()
                .filter(|t| !t.surface.trim().is_empty())
                .map(|t| {
                    let text = t.surface.to_string();
                    let d = t.details();
                    // IPADIC: pos, pos1, pos2, pos3, conjugation type, conjugation form, base form, reading, pronunciation
                    let base = field(&d, 6);
                    Word {
                        pos: field(&d, 0),
                        pos1: field(&d, 1),
                        form: field(&d, 5),
                        base: if base.is_empty() { text.clone() } else { base },
                        reading: field(&d, 7),
                        text,
                    }
                })
                .collect())
        })
        .collect()
}

#[tauri::command]
pub async fn japanese_words(lines: Vec<String>) -> Result<Vec<Vec<Word>>, String> {
    tauri::async_runtime::spawn_blocking(move || words(&lines)).await.map_err(|e| e.to_string())?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_a_lyric_line_into_dictionary_words() {
        let got = words(&["君の名前を呼んでいた".into()]).unwrap();
        let w = &got[0];
        let texts: Vec<_> = w.iter().map(|w| w.text.as_str()).collect();
        assert_eq!(texts, ["君", "の", "名前", "を", "呼ん", "で", "い", "た"]);
        assert_eq!((w[4].base.as_str(), w[4].reading.as_str(), w[4].pos.as_str(), w[4].form.as_str()), ("呼ぶ", "ヨン", "動詞", "連用タ接続"));
        assert_eq!((w[1].pos.as_str(), w[1].pos1.as_str()), ("助詞", "連体化"));
    }

    #[test]
    fn keeps_unknown_words_without_a_reading() {
        let got = words(&["I love you だけど".into(), "".into()]).unwrap();
        let w = &got[0];
        assert_eq!(w.iter().map(|w| w.text.as_str()).collect::<Vec<_>>(), ["I", "love", "you", "だけど"]);
        assert_eq!((w[0].reading.as_str(), w[0].base.as_str()), ("", "I"));
        assert_eq!(w[3].reading, "ダケド");
        assert!(got[1].is_empty());
    }
}
