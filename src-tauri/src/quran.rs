// Qur'an data for Learn mode, and the pauses in a recitation for ayah sync.
// - Text, word meanings and translations come from quran.com (Hafs) and the open fawazahmed0
//   editions on GitHub (Shu'bah text, The Clear Quran). Endpoints are fixed, the chapter is
//   checked, and requests use the HTTP/1.1 client from translate.rs.
// - quran_pauses decodes a track (symphonia), measures its loudness every 20 ms and returns the quiet
//   stretches; the TypeScript side places the ayah boundaries among them (quran.ts alignAyahs).
use crate::translate::http1_client;
use serde::{Deserialize, Serialize};
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager};

const QURAN_COM: &str = "https://api.quran.com/api/v4";
// GitHub raw, not jsDelivr: jsDelivr refuses uncached files of this repo (over its 50 MB package limit)
const EDITIONS: &str = "https://raw.githubusercontent.com/fawazahmed0/quran-api/1/editions";
use crate::UA;
/// Mishary Rashid Alafasy (murattal) on quran.com: the pacing template for ayah sync.
const ALAFASY: u32 = 7;

#[derive(Deserialize, Clone, Copy, Debug, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum WordLang {
    Id,
    En,
}

#[derive(Deserialize, Debug)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Source {
    /// Words (Uthmani text, meaning, transliteration) and the Kemenag (33) and Saheeh (20) translations.
    Verses { chapter: u16, lang: WordLang },
    Timings { chapter: u16 },
    Shubah { chapter: u16 },
    ClearQuran { chapter: u16 },
}

impl Source {
    fn chapter(&self) -> u16 {
        match *self {
            Source::Verses { chapter, .. } | Source::Timings { chapter } | Source::Shubah { chapter } | Source::ClearQuran { chapter } => chapter,
        }
    }
}

/// The request URL for a source (and page, for verses).
pub fn url(source: &Source, page: u32) -> Result<String, String> {
    let c = source.chapter();
    if !(1..=114).contains(&c) {
        return Err("There are 114 surahs.".into());
    }
    Ok(match source {
        Source::Verses { lang, .. } => {
            let l = if *lang == WordLang::Id { "id" } else { "en" };
            format!("{QURAN_COM}/verses/by_chapter/{c}?words=true&word_fields=text_uthmani&translations=33,20&language={l}&per_page=50&page={page}")
        }
        Source::Timings { .. } => format!("{QURAN_COM}/chapter_recitations/{ALAFASY}/{c}?segments=true"),
        Source::Shubah { .. } => format!("{EDITIONS}/ara-quranshouba/{c}.json"),
        Source::ClearQuran { .. } => format!("{EDITIONS}/eng-mustafakhattaba/{c}.json"),
    })
}

async fn get_json(url: &str) -> Result<serde_json::Value, String> {
    let r = http1_client()?.get(url).header("User-Agent", UA).send().await.map_err(|e| format!("network: {e}"))?;
    let status = r.status().as_u16();
    let body = r.text().await.map_err(|e| e.to_string())?;
    if status != 200 {
        let host = url.split('/').nth(2).unwrap_or("The Qur'an text service");
        log::warn!("quran: {status} from {url}");
        return Err(format!("{host} answered {status}. Try again later."));
    }
    serde_json::from_str(&body).map_err(|_| "Unexpected reply from the Qur'an text service.".to_string())
}

/// Raw JSON for one source; verses are gathered across pages into `{ "verses": [...] }`.
#[tauri::command]
pub async fn quran_fetch(source: Source) -> Result<serde_json::Value, String> {
    if !matches!(source, Source::Verses { .. }) {
        return get_json(&url(&source, 1)?).await;
    }
    let mut verses = Vec::new();
    for page in 1..=10 {
        let j = get_json(&url(&source, page)?).await?;
        verses.extend(j["verses"].as_array().cloned().unwrap_or_default());
        if j["pagination"]["next_page"].is_null() {
            break;
        }
    }
    Ok(serde_json::json!({ "verses": verses }))
}

// ---- pauses ----

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct Pauses {
    pub duration: f64,
    /// Quiet stretches as [start, end] seconds.
    pub pauses: Vec<[f64; 2]>,
}

const FRAME: f64 = 0.02;

/// Quiet stretches of at least `min` seconds in a loudness curve (dB per 20 ms frame): where the level,
/// smoothed over 100 ms, falls 12 dB below the recording's median. Speech fills most of a recitation,
/// so the median is its speaking level; the smoothing keeps the reverb tail's flicker inside a pause
/// from breaking it up. Tuned on Alafasy's Al-Fatihah (8 pauses: isti'adha + 7 ayahs).
pub fn find_pauses(db: &[f32], min: f64) -> Vec<[f64; 2]> {
    if db.is_empty() {
        return vec![];
    }
    let smooth: Vec<f32> = (0..db.len())
        .map(|i| {
            let (a, b) = (i.saturating_sub(2), (i + 3).min(db.len()));
            db[a..b].iter().sum::<f32>() / (b - a) as f32
        })
        .collect();
    let mut sorted = db.to_vec();
    sorted.sort_by(|a, b| a.total_cmp(b));
    let threshold = sorted[sorted.len() / 2] - 12.0;
    let mut out = Vec::new();
    let mut start: Option<usize> = None;
    for (i, &v) in smooth.iter().chain(std::iter::once(&f32::INFINITY)).enumerate() {
        match (v < threshold, start) {
            (true, None) => start = Some(i),
            (false, Some(s)) => {
                if (i - s) as f64 * FRAME >= min {
                    out.push([round2(s as f64 * FRAME), round2(i as f64 * FRAME)]);
                }
                start = None;
            }
            _ => {}
        }
    }
    out
}

fn round2(x: f64) -> f64 {
    (x * 100.0).round() / 100.0
}

/// Loudness in dB for every 20 ms of the track (mono mix).
pub fn loudness(path: &Path) -> Result<Vec<f32>, String> {
    use symphonia::core::audio::SampleBuffer;
    use symphonia::core::codecs::{DecoderOptions, CODEC_TYPE_NULL};
    use symphonia::core::errors::Error;
    use symphonia::core::formats::FormatOptions;
    use symphonia::core::io::MediaSourceStream;
    use symphonia::core::meta::MetadataOptions;
    use symphonia::core::probe::Hint;

    let file = std::fs::File::open(path).map_err(|e| e.to_string())?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());
    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }
    let probed = symphonia::default::get_probe()
        .format(&hint, mss, &FormatOptions::default(), &MetadataOptions::default())
        .map_err(|e| format!("can't read this file's audio: {e}"))?;
    let mut format = probed.format;
    let track = format.tracks().iter().find(|t| t.codec_params.codec != CODEC_TYPE_NULL).ok_or("no audio track")?;
    let (id, rate) = (track.id, track.codec_params.sample_rate.ok_or("unknown sample rate")?);
    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &DecoderOptions::default())
        .map_err(|e| format!("can't decode this file's audio: {e}"))?;

    let per_frame = ((rate as f64) * FRAME).round() as usize;
    let (mut sum, mut n) = (0f64, 0usize);
    let mut out = Vec::new();
    let mut buf: Option<(SampleBuffer<f32>, usize)> = None;
    loop {
        let packet = match format.next_packet() {
            Ok(p) => p,
            Err(Error::IoError(e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
            Err(Error::ResetRequired) => break,
            Err(e) => return Err(e.to_string()),
        };
        if packet.track_id() != id {
            continue;
        }
        let decoded = match decoder.decode(&packet) {
            Ok(d) => d,
            Err(Error::DecodeError(_)) => continue, // one damaged packet: skip it
            Err(e) => return Err(e.to_string()),
        };
        let spec = *decoded.spec();
        let channels = spec.channels.count().max(1);
        if buf.as_ref().is_none_or(|(_, cap)| *cap < decoded.capacity()) {
            buf = Some((SampleBuffer::new(decoded.capacity() as u64, spec), decoded.capacity()));
        }
        let (b, _) = buf.as_mut().unwrap();
        b.copy_interleaved_ref(decoded);
        for frame in b.samples().chunks(channels) {
            let m = frame.iter().sum::<f32>() / channels as f32;
            sum += (m * m) as f64;
            n += 1;
            if n == per_frame {
                out.push((10.0 * (sum / n as f64 + 1e-12).log10()) as f32);
                (sum, n) = (0.0, 0);
            }
        }
    }
    Ok(out)
}

fn cache_file(dir: &Path, path: &Path) -> Option<PathBuf> {
    let meta = std::fs::metadata(path).ok()?;
    let mut h = std::hash::DefaultHasher::new();
    path.to_string_lossy().to_lowercase().hash(&mut h);
    meta.len().hash(&mut h);
    meta.modified().ok()?.hash(&mut h);
    Some(dir.join(format!("{:016x}.json", h.finish())))
}

/// Pauses of one track; computed once and cached (keyed by path, size and modification time).
pub fn pauses_cached(dir: &Path, path: &Path) -> Result<Pauses, String> {
    let cache = cache_file(dir, path);
    if let Some(p) = cache.as_ref().and_then(|c| std::fs::read_to_string(c).ok()).and_then(|s| serde_json::from_str(&s).ok()) {
        return Ok(p);
    }
    let db = loudness(path)?;
    let result = Pauses { duration: round2(db.len() as f64 * FRAME), pauses: find_pauses(&db, 0.3) };
    if let Some(c) = cache {
        let _ = std::fs::create_dir_all(dir);
        let _ = std::fs::write(c, serde_json::to_string(&result).unwrap_or_default());
    }
    Ok(result)
}

#[tauri::command]
pub async fn quran_pauses(app: AppHandle, path: String) -> Result<Pauses, String> {
    let dir = app.path().app_local_data_dir().map_err(|e| e.to_string())?.join("quran-sync");
    tauri::async_runtime::spawn_blocking(move || pauses_cached(&dir, Path::new(&path))).await.map_err(|e| e.to_string())?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_fixed_urls_for_valid_surahs_only() {
        let v = url(&Source::Verses { chapter: 2, lang: WordLang::Id }, 3).unwrap();
        assert!(v.starts_with("https://api.quran.com/api/v4/verses/by_chapter/2?"));
        assert!(v.contains("translations=33,20") && v.contains("language=id") && v.ends_with("page=3"));
        assert_eq!(url(&Source::Timings { chapter: 112 }, 1).unwrap(), "https://api.quran.com/api/v4/chapter_recitations/7/112?segments=true");
        assert!(url(&Source::Shubah { chapter: 78 }, 1).unwrap().ends_with("/ara-quranshouba/78.json"));
        assert!(url(&Source::ClearQuran { chapter: 0 }, 1).is_err());
        assert!(url(&Source::Shubah { chapter: 115 }, 1).is_err());
    }

    #[test]
    fn parses_sources_from_the_frontend() {
        let s: Source = serde_json::from_str(r#"{"kind":"verses","chapter":1,"lang":"en"}"#).unwrap();
        assert!(matches!(s, Source::Verses { chapter: 1, lang: WordLang::En }));
        let s: Source = serde_json::from_str(r#"{"kind":"clearQuran","chapter":5}"#).unwrap();
        assert!(matches!(s, Source::ClearQuran { chapter: 5 }));
    }

    #[test]
    fn finds_pauses_long_enough_only() {
        let mut db = vec![-20.0f32; 50]; // 1 s of speech
        db.extend(vec![-70.0; 20]); // 0.4 s pause
        db.extend(vec![-20.0; 30]);
        db.extend(vec![-70.0; 10]); // 0.2 s: a breath, too short
        db.extend(vec![-20.0; 40]);
        let p = find_pauses(&db, 0.3);
        assert_eq!(p.len(), 1, "{p:?}");
        assert!((p[0][0] - 1.0).abs() <= 0.04 && (p[0][1] - 1.4).abs() <= 0.04, "{p:?}");
    }

    /// 16-bit mono WAV: 0.5 s tone, 0.6 s silence, 0.5 s tone.
    fn wav(path: &Path) {
        let rate = 8000u32;
        let mut samples: Vec<i16> = Vec::new();
        let tone = |n: usize, s: &mut Vec<i16>| s.extend((0..n).map(|i| ((i as f32 * 0.3).sin() * 12000.0) as i16));
        tone(4000, &mut samples);
        samples.extend(std::iter::repeat_n(0i16, 4800));
        tone(4000, &mut samples);
        let data: Vec<u8> = samples.iter().flat_map(|s| s.to_le_bytes()).collect();
        let mut f = Vec::new();
        f.extend(b"RIFF");
        f.extend((36 + data.len() as u32).to_le_bytes());
        f.extend(b"WAVEfmt ");
        f.extend(16u32.to_le_bytes());
        f.extend(1u16.to_le_bytes()); // PCM
        f.extend(1u16.to_le_bytes()); // mono
        f.extend(rate.to_le_bytes());
        f.extend((rate * 2).to_le_bytes());
        f.extend(2u16.to_le_bytes());
        f.extend(16u16.to_le_bytes());
        f.extend(b"data");
        f.extend((data.len() as u32).to_le_bytes());
        f.extend(data);
        std::fs::write(path, f).unwrap();
    }

    #[test]
    fn decodes_a_file_and_caches_its_pauses() {
        let d = tempfile::tempdir().unwrap();
        let song = d.path().join("ayat.wav");
        wav(&song);
        let cache = d.path().join("cache");
        let p = pauses_cached(&cache, &song).unwrap();
        assert!((p.duration - 1.6).abs() < 0.05, "{p:?}");
        assert_eq!(p.pauses.len(), 1, "{p:?}");
        let [s, e] = p.pauses[0];
        assert!((s - 0.5).abs() < 0.05 && (e - 1.1).abs() < 0.05, "{p:?}");
        // the second call reads the cache
        assert_eq!(std::fs::read_dir(&cache).unwrap().count(), 1);
        assert_eq!(pauses_cached(&cache, &song).unwrap(), p);
    }
}
