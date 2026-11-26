// Online lyrics. Each source is a fixed endpoint, and only the song's title, artist, album and length leave the
// machine (a server song's id goes only to your own server, a YouTube song's video id only to YT Music). Everything comes back as LRC; word timing is kept
// as enhanced LRC ("<mm:ss.mmm>word"), which lyrics.ts already reads.
use crate::UA;
use base64::Engine;
use serde::Deserialize;
use std::io::Read;
use tauri_plugin_http::reqwest;

#[derive(Deserialize, Debug, Clone, Copy, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Source {
    Subsonic,
    Lrclib,
    Netease,
    Kugou,
    /// YT Music's own lyrics, for YouTube and YT Music songs
    Ytmusic,
}

impl Source {
    pub fn name(self) -> &'static str {
        match self {
            Source::Subsonic => "subsonic",
            Source::Lrclib => "lrclib",
            Source::Netease => "netease",
            Source::Kugou => "kugou",
            Source::Ytmusic => "ytmusic",
        }
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct Song {
    pub title: String,
    pub artist: String,
    #[serde(default)]
    pub album: String,
    #[serde(default)]
    pub duration: f64, // seconds, 0 = unknown
    /// the song's id on your Subsonic server, or a YouTube song's video id (for YT Music)
    pub id: Option<String>,
}

fn client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder().user_agent(UA).timeout(std::time::Duration::from_secs(8)).build().map_err(|e| e.to_string())
}

/// Search text is capped; nothing else about the song is sent.
fn cap(s: &str) -> String {
    s.trim().chars().take(200).collect()
}

/// Ok(lyrics) or Ok("") when the source has none (worth caching); Err for trouble worth retrying later.
pub async fn fetch(source: Source, song: &Song) -> Result<String, String> {
    let (title, artist) = (cap(&song.title), cap(&song.artist));
    if title.is_empty() {
        return Ok(String::new());
    }
    let ms = (song.duration * 1000.0).round() as i64;
    match source {
        Source::Subsonic => match &song.id {
            Some(id) => crate::subsonic::lyrics(id, &artist, &title).await,
            None => Ok(String::new()),
        },
        Source::Lrclib => lrclib(&title, &artist, &cap(&song.album), song.duration).await,
        Source::Netease => netease(&title, &artist, ms).await,
        Source::Kugou => kugou(&title, &artist, ms).await,
        Source::Ytmusic => match &song.id {
            Some(id) => crate::ytmusic::lyrics(id).await,
            None => Ok(String::new()),
        },
    }
}

async fn json(req: reqwest::RequestBuilder) -> Result<serde_json::Value, String> {
    let r = req.send().await.map_err(|e| format!("network: {e}"))?;
    if !r.status().is_success() {
        return Err(format!("HTTP {}", r.status().as_u16()));
    }
    serde_json::from_str(&r.text().await.map_err(|e| e.to_string())?).map_err(|e| e.to_string())
}

/// LRCLIB: the exact song first (title, artist, album and length all match), then a search by title and artist,
/// picked like the other sources' results. A streaming song has no album to match, and its length can be off.
async fn lrclib(title: &str, artist: &str, album: &str, duration: f64) -> Result<String, String> {
    let mut q = vec![("track_name", title.to_string()), ("artist_name", artist.to_string())];
    if !album.is_empty() {
        q.push(("album_name", album.to_string()));
    }
    if duration > 0.0 {
        q.push(("duration", (duration.round() as i64).to_string()));
    }
    let r = client()?.get("https://lrclib.net/api/get").query(&q).send().await.map_err(|e| format!("network: {e}"))?;
    if r.status().as_u16() == 404 {
        let found = json(client()?.get("https://lrclib.net/api/search").query(&[("track_name", title), ("artist_name", artist)])).await?;
        return Ok(lrclib_pick(&found, title, artist, (duration * 1000.0).round() as i64).unwrap_or_default());
    }
    if !r.status().is_success() {
        return Err(format!("HTTP {}", r.status().as_u16()));
    }
    let j: serde_json::Value = serde_json::from_str(&r.text().await.map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
    let pick = |k: &str| j[k].as_str().filter(|s| !s.trim().is_empty()).map(str::to_owned);
    // LRCLIB keeps whatever the uploader pasted, credits included
    Ok(pick("syncedLyrics").or_else(|| pick("plainLyrics")).map(|l| drop_credits(&l, title, artist)).unwrap_or_default())
}

/// The song among LRCLIB's search results, with its lyrics (synced when it has them), credits dropped.
fn lrclib_pick(found: &serde_json::Value, title: &str, artist: &str, ms: i64) -> Option<String> {
    let list = found.as_array()?;
    let lyrics = |v: &serde_json::Value| {
        let pick = |k: &str| v[k].as_str().filter(|s| !s.trim().is_empty()).map(str::to_owned);
        pick("syncedLyrics").or_else(|| pick("plainLyrics"))
    };
    let cands: Vec<Cand> = list
        .iter()
        .enumerate()
        .filter(|(_, v)| lyrics(v).is_some())
        .map(|(i, v)| {
            let s = |k: &str| v[k].as_str().unwrap_or_default().to_string();
            (i.to_string(), s("trackName"), (v["duration"].as_f64().unwrap_or(0.0) * 1000.0).round() as i64, s("artistName"))
        })
        .collect();
    let i: usize = best(&cands, title, artist, ms)?.parse().ok()?;
    lyrics(&list[i]).map(|l| drop_credits(&l, title, artist))
}

/// A search result to choose from: (id, title, length in ms, artists).
type Cand = (String, String, i64, String);

/// Lowercase letters and digits only.
fn alnum(s: &str) -> String {
    s.chars().filter(|c| c.is_alphanumeric()).flat_map(char::to_lowercase).collect()
}
/// The title without "(Remix)", "[Live]" or "- Remastered" tails.
fn norm(s: &str) -> String {
    let head = s.split(['(', '[', '（', '【']).next().unwrap_or(s);
    let n = alnum(head.split(" - ").next().unwrap_or(head));
    // "(I Can't Get No) Satisfaction", "【MV】…": nothing before the bracket, so keep it all
    if n.is_empty() { alnum(s) } else { n }
}

/// The result that is this song: within 3 s of its length when that is known, and with the same title when
/// possible. With no length to go by, only a matching title counts.
fn best(cands: &[Cand], title: &str, artist: &str, ms: i64) -> Option<String> {
    let a = alnum(artist);
    let near = |c: &Cand| ms <= 0 || c.2 <= 0 || c.2.saturating_sub(ms).saturating_abs() <= 3000;
    let exact = |c: &Cand| alnum(&c.1) == alnum(title);
    let loose = |c: &Cand| !norm(title).is_empty() && norm(&c.1) == norm(title);
    let by = |c: &Cand| a.is_empty() || alnum(&c.3).contains(&a) || a.contains(&alnum(&c.3));
    let close = |c: &Cand| ms > 0 && c.2 > 0 && c.2.saturating_sub(ms).saturating_abs() <= 1000;
    // most certain first: the same title by the same artist, then looser matches
    let tests: [&dyn Fn(&Cand) -> bool; 5] = [&|c| exact(c) && by(c), &exact, &|c| loose(c) && by(c), &loose, &|c| close(c) && by(c)];
    tests.iter().find_map(|t| cands.iter().find(|c| near(c) && t(c))).map(|c| c.0.clone())
}

async fn netease(title: &str, artist: &str, ms: i64) -> Result<String, String> {
    let c = client()?;
    let j = json(c.get("https://music.163.com/api/search/get").query(&[("s", format!("{artist} {title}")), ("type", "1".into()), ("limit", "8".into())])).await?;
    // NetEase answers 200 with its own code when it refuses (-460 …): that is trouble, not "no lyrics"
    if j["code"] != 200 {
        return Err(format!("NetEase refused the search ({})", j["code"]));
    }
    let cands: Vec<Cand> = j["result"]["songs"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|s| {
                    let artists = s["artists"].as_array().map(|x| x.iter().filter_map(|a| a["name"].as_str()).collect::<Vec<_>>().join(" ")).unwrap_or_default();
                    Some((s["id"].as_u64()?.to_string(), s["name"].as_str()?.to_string(), s["duration"].as_i64().unwrap_or(0), artists))
                })
                .collect()
        })
        .unwrap_or_default();
    let Some(id) = best(&cands, title, artist, ms) else { return Ok(String::new()) };
    let j = json(c.get("https://music.163.com/api/song/lyric").query(&[("id", id.as_str()), ("lv", "1")])).await?;
    if j["code"] != 200 {
        return Err(format!("NetEase refused the lyrics ({})", j["code"]));
    }
    Ok(drop_credits(j["lrc"]["lyric"].as_str().unwrap_or_default(), title, artist))
}

async fn kugou(title: &str, artist: &str, ms: i64) -> Result<String, String> {
    let c = client()?;
    let mut q = vec![("ver", "1".to_string()), ("man", "yes".into()), ("client", "pc".into()), ("keyword", format!("{artist} - {title}"))];
    if ms > 0 {
        q.push(("duration", ms.to_string()));
    }
    let j = json(c.get("https://lyrics.kugou.com/search").query(&q)).await?;
    if j["status"] != 200 || !j["candidates"].is_array() {
        return Err(format!("Kugou refused the search ({})", j["status"]));
    }
    let found: Vec<(Cand, String)> = j["candidates"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|s| {
                    let cand = (s["id"].as_str()?.to_string(), s["song"].as_str().unwrap_or_default().to_string(), s["duration"].as_i64().unwrap_or(0), s["singer"].as_str().unwrap_or_default().to_string());
                    Some((cand, s["accesskey"].as_str()?.to_string()))
                })
                .collect()
        })
        .unwrap_or_default();
    let cands: Vec<Cand> = found.iter().map(|f| f.0.clone()).collect();
    let Some(id) = best(&cands, title, artist, ms) else { return Ok(String::new()) };
    let key = &found.iter().find(|f| f.0 .0 == id).ok_or("no candidate")?.1;
    let ok = |s: &str, max: usize| (1..=max).contains(&s.len()) && s.bytes().all(|b| b.is_ascii_alphanumeric());
    if !ok(&id, 20) || !ok(key, 64) {
        return Err("unexpected reply".into());
    }
    let j = json(c.get("https://lyrics.kugou.com/download").query(&[("ver", "1"), ("client", "pc"), ("id", id.as_str()), ("accesskey", key.as_str()), ("fmt", "krc"), ("charset", "utf8")])).await?;
    if j["status"] != 200 {
        return Err(format!("Kugou refused the lyrics ({})", j["status"]));
    }
    let raw = base64::engine::general_purpose::STANDARD.decode(j["content"].as_str().unwrap_or_default()).map_err(|e| e.to_string())?;
    Ok(drop_credits(&krc_to_lrc(&decrypt_krc(&raw)?), title, artist))
}

/// KRC files are "krc1", then zlib data XORed with a fixed key.
const KRC_KEY: [u8; 16] = [64, 71, 97, 119, 94, 50, 116, 71, 81, 54, 49, 45, 206, 210, 110, 105];

fn decrypt_krc(raw: &[u8]) -> Result<String, String> {
    let body = raw.strip_prefix(b"krc1").ok_or("not a KRC file")?;
    let plain: Vec<u8> = body.iter().enumerate().map(|(i, b)| b ^ KRC_KEY[i % 16]).collect();
    let mut out = String::new();
    flate2::read::ZlibDecoder::new(&plain[..]).take(4 << 20).read_to_string(&mut out).map_err(|e| e.to_string())?;
    Ok(out)
}

fn stamp(ms: i64) -> String {
    let ms = ms.max(0);
    format!("{:02}:{:02}.{:03}", ms / 60000, ms / 1000 % 60, ms % 1000)
}

/// "[16210,3460]<0,670,0>Hello<670,95,0> <765,410,0>world" (word offsets from the line start)
/// → "[00:16.210]<00:16.210>Hello <00:16.975>world<00:17.385>"
fn krc_to_lrc(krc: &str) -> String {
    let mut out = String::new();
    for line in krc.lines() {
        // the KRC header's offset applies to the LRC the same way
        if let Some(off) = line.trim().strip_prefix("[offset:").and_then(|o| o.strip_suffix(']')).filter(|o| o.trim().parse::<i64>().is_ok()) {
            out.push_str(&format!("[offset:{}]\n", off.trim()));
            continue;
        }
        let Some((head, rest)) = line.strip_prefix('[').and_then(|l| l.split_once(']')) else { continue };
        let Some((Ok(start), Ok(_))) = head.split_once(',').map(|(a, b)| (a.parse::<i64>(), b.parse::<i64>())) else { continue };
        let mut words: Vec<(i64, i64, String)> = Vec::new();
        for part in rest.split('<').skip(1) {
            let Some((t, text)) = part.split_once('>') else { continue };
            let n: Vec<i64> = t.split(',').filter_map(|x| x.parse().ok()).collect();
            let [off, dur, ..] = n[..] else { continue };
            match words.last_mut() {
                // a space is a "word" of its own in KRC: it belongs to the word before it
                Some(w) if text.trim().is_empty() => w.2.push_str(text),
                _ => words.push((start.saturating_add(off), start.saturating_add(off).saturating_add(dur), text.to_string())),
            }
        }
        if words.is_empty() {
            continue;
        }
        out.push_str(&format!("[{}]", stamp(start)));
        for (t, _, text) in &words {
            out.push_str(&format!("<{}>{text}", stamp(*t)));
        }
        out.push_str(&format!("<{}>\n", stamp(words.last().unwrap().1)));
    }
    out
}

/// Credit labels that open a "label: name" line. A label counts when it contains one of these (CJK, so
/// "混音工程师" is "混音") or starts with one (Latin, so "Mixed by" but not a lyric that mentions it later).
const CREDITS_CJK: [&str; 37] = [
    "作词", "作曲", "编曲", "制作", "监制", "混音", "母带", "录音", "和声", "配唱", "出品", "发行", "吉他", "贝斯", "鼓手", "键盘",
    "弦乐", "人声", "监唱", "企划", "统筹", "作詞", "編曲", "製作", "監製", "錄音", "和聲", "演唱", "原唱", "翻唱", "词", "曲",
    "작사", "작곡", "편곡", "프로듀서", "보컬",
];
const CREDITS_LATIN: [&str; 20] = [
    "lyrics by", "composed by", "produced by", "written by", "arranged by", "mixed by", "mastered by", "recorded by",
    "music by", "words by", "lyricist", "composer", "arranger", "producer", "lyrics", "music", "vocals", "vocal", "op", "sp",
];

fn credit_line(t: &str) -> bool {
    let Some((label, _)) = t.split_once([':', '：']) else { return false };
    let label = label.trim();
    !label.is_empty()
        && label.chars().count() <= 16
        && (CREDITS_CJK.iter().any(|c| label.contains(c)) || CREDITS_LATIN.iter().any(|c| label == *c || label.starts_with(&format!("{c} "))))
}

/// Lyrics services open with credit lines ("作词 : …", "Lyrics by: …") and a "Title - Artist" line.
fn drop_credits(lrc: &str, title: &str, artist: &str) -> String {
    let heading = [alnum(&format!("{title}{artist}")), alnum(&format!("{artist}{title}"))];
    let mut out = String::new();
    let mut sung = false;
    for line in lrc.lines() {
        let text: String = line.split(']').next_back().unwrap_or_default().split('>').filter_map(|p| p.split('<').next()).collect();
        let t = text.trim().to_lowercase();
        if credit_line(&t) || (!t.is_empty() && heading.contains(&alnum(&t))) || line.trim_start().starts_with('{') {
            continue;
        }
        // a sung line, timed or plain; "[ti:…]"-style tags alone are not lyrics
        let l = line.trim();
        sung |= !t.is_empty() && !(l.starts_with('[') && l.ends_with(']') && !l.contains("]["));
        out.push_str(line);
        out.push('\n');
    }
    if sung { out } else { String::new() }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn picks_the_song_from_lrclib_search_results() {
        let found = serde_json::json!([
            { "trackName": "Like Flames (Instrumental)", "artistName": "MindaRyn", "duration": 271.0, "plainLyrics": "" },
            { "trackName": "Like Flames", "artistName": "Someone Else", "duration": 200.0, "plainLyrics": "wrong song" },
            { "trackName": "Like Flames", "artistName": "MindaRyn", "duration": 272.0, "syncedLyrics": "[00:10.00]burning", "plainLyrics": "burning" },
        ]);
        assert_eq!(lrclib_pick(&found, "Like Flames", "MindaRyn", 271_000).as_deref().map(str::trim), Some("[00:10.00]burning"));
        // nothing near its length: no guess
        assert_eq!(lrclib_pick(&found, "Like Flames", "MindaRyn", 100_000), None);
        assert_eq!(lrclib_pick(&serde_json::json!({}), "x", "y", 0), None);
    }

    #[test]
    fn fuzz_krc_and_credits_never_panic() {
        let mut x = 0xdead_beefu32;
        let mut next = || {
            x ^= x << 13;
            x ^= x >> 17;
            x ^= x << 5;
            x
        };
        let pieces = ["[", "]", "<", ">", ",", "0", "999999999999999999999", "-1", "作词", "：", ":", "\n", "offset:", "a", " "];
        for _ in 0..20_000 {
            let mut raw = b"krc1".to_vec();
            raw.extend((0..next() % 64).map(|_| next() as u8));
            let _ = decrypt_krc(&raw);
            let text: String = (0..next() % 24).map(|_| pieces[next() as usize % pieces.len()]).collect();
            let _ = drop_credits(&krc_to_lrc(&text), "t", "a");
        }
    }

    #[test]
    fn decrypts_krc_and_keeps_word_timing() {
        let text = "[ti:Song]\n[0,2380]<0,476,0>Song<476,95,0> <571,190,0>-<761,95,0> <856,286,0>Artist\n[9551,2435]<0,236,0>The <236,487,0>club<723,95,0> <818,271,0>isn't\n";
        let mut z = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
        z.write_all(text.as_bytes()).unwrap();
        let mut raw = b"krc1".to_vec();
        raw.extend(z.finish().unwrap().iter().enumerate().map(|(i, b)| b ^ KRC_KEY[i % 16]));
        let lrc = drop_credits(&krc_to_lrc(&decrypt_krc(&raw).unwrap()), "Song", "Artist");
        assert_eq!(lrc, "[00:09.551]<00:09.551>The <00:09.787>club <00:10.369>isn't<00:10.640>\n");
        assert!(decrypt_krc(b"nope").is_err());
    }

    #[test]
    fn drops_credit_lines() {
        let lrc = "[00:00.000] 作词 : Someone\n[00:01.000] 作曲 : Someone\n[00:09.480]The club isn't the best place\n[00:12.000]\n";
        assert_eq!(drop_credits(lrc, "Shape of You", "Ed Sheeran"), "[00:09.480]The club isn't the best place\n[00:12.000]\n");
        assert_eq!(drop_credits("[00:00.000] 作词 : x\n", "a", "b"), "", "credits alone are no lyrics");
        assert_eq!(drop_credits("[ti:x]\nPlain line\n", "a", "b"), "[ti:x]\nPlain line\n", "unsynced lyrics stay");
        let more = "[00:00.10]混音工程师 : A\n[00:00.20]和声编写：B\n[00:00.30]作詞：C\n[00:00.40]작사 : D\n[00:00.50]Mixed by: E\n[00:00.60]Lyricist : F\n[00:05.00]Music is all I need\n[00:06.00]Producer of dreams, that's me\n";
        assert_eq!(drop_credits(more, "t", "a"), "[00:05.00]Music is all I need\n[00:06.00]Producer of dreams, that's me\n");
    }

    #[test]
    fn keeps_the_krc_offset() {
        assert_eq!(krc_to_lrc("[offset:-250]\n[1000,500]<0,500,0>Hi\n"), "[offset:-250]\n[00:01.000]<00:01.000>Hi<00:01.500>\n");
    }

    #[test]
    fn picks_the_same_song_by_title_and_length() {
        let c = |id: &str, t: &str, ms: i64, a: &str| (id.to_string(), t.to_string(), ms, a.to_string());
        let cands = [c("1", "Shape Of You (Galantis Remix)", 235415, "Ed Sheeran"), c("2", "Shape of You", 233712, "Ed Sheeran")];
        assert_eq!(best(&cands, "Shape of You", "Ed Sheeran", 233743), Some("2".into()));
        assert_eq!(best(&cands, "Shape of You", "Ed Sheeran", 0), Some("2".into()));
        assert_eq!(best(&cands, "Something Else", "Nobody", 100000), None);
        // a different title only counts when the length matches closely
        assert_eq!(best(&[c("3", "夜に駆ける", 261000, "YOASOBI")], "Yoru ni Kakeru", "YOASOBI", 261200), Some("3".into()));
        // a title that starts with a bracket still has to match itself, not anything
        assert_eq!(best(&[c("4", "Paint It Black", 0, "x")], "(I Can't Get No) Satisfaction", "x", 0), None);
    }

    #[test]
    #[ignore = "network: run on purpose to see the services still answer"]
    fn live_sources_answer() {
        let song = Song { title: "Shape of You".into(), artist: "Ed Sheeran".into(), album: "÷".into(), duration: 233.7, id: None };
        for s in [Source::Lrclib, Source::Netease, Source::Kugou] {
            let text = tauri::async_runtime::block_on(fetch(s, &song)).unwrap();
            assert!(text.contains("club"), "{s:?}: {text:.200}");
        }
    }
}
