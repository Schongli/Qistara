// YT Music as a guest: home shelves, search, albums, artists, playlists and radio through YouTube Music's own web
// API (InnerTube, the WEB_REMIX client, no sign-in). Only what to show comes from here; the songs play through
// mpv's ytdl hook like any YouTube video (youtube.svelte.ts).
// One fixed endpoint; every id is checked before it goes into a request; no cookies, and the visitor id YouTube
// hands out isn't kept. Replies are read by walking for the renderers YT Music draws with, wherever they're
// nested, so a new wrapper around a shelf doesn't break the page.
use serde::Serialize;
use serde_json::{json, Value};

const BASE: &str = "https://music.youtube.com/youtubei/v1/";
/// The web client's version this was written against; YouTube accepts older ones for a long time.
const CLIENT_VERSION: &str = "1.20251110.01.00";

/// Search filters, as YT Music's own chips send them (fixed values).
const FILTERS: [(&str, &str); 4] = [
    ("songs", "EgWKAQIIAWoMEA4QChADEAQQCRAF"),
    ("albums", "EgWKAQIYAWoMEA4QChADEAQQCRAF"),
    ("artists", "EgWKAQIgAWoMEA4QChADEAQQCRAF"),
    ("playlists", "Eg-KAQwIABAAGAAgACgBMABqChAEEAMQCRAFEAo%3D"),
];

/// Thumbnails come only from these hosts (they're in the CSP's img-src).
const THUMB_HOSTS: [&str; 4] = ["lh3.googleusercontent.com", "yt3.ggpht.com", "yt3.googleusercontent.com", "i.ytimg.com"];

#[derive(Serialize, Debug, PartialEq, Clone)]
pub struct Ref {
    name: String,
    id: Option<String>,
}

/// One thing YT Music shows: a song (a video id to play) or an album, artist or playlist to open.
#[derive(Serialize, Debug, PartialEq, Clone)]
pub struct Item {
    kind: &'static str,
    id: String,
    title: String,
    subtitle: String,
    artists: Vec<Ref>,
    album: Option<Ref>,
    duration: Option<f64>,
    thumb: Option<String>,
}

#[derive(Serialize, Debug, PartialEq)]
pub struct Shelf {
    title: String,
    items: Vec<Item>,
}

/// An album, playlist or artist page.
#[derive(Serialize, Debug, PartialEq, Default)]
pub struct Page {
    title: String,
    subtitle: String,
    artists: Vec<Ref>,
    thumb: Option<String>,
    /// the songs (an album's or playlist's tracks, an artist's top songs)
    tracks: Vec<Item>,
    /// everything else (an artist's albums, singles, similar artists…)
    shelves: Vec<Shelf>,
}

// ---- ids: checked before they reach a request ----

fn chars(s: &str) -> bool {
    s.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}
pub fn valid_video(id: &str) -> bool {
    id.len() == 11 && chars(id)
}
pub fn valid_album(id: &str) -> bool {
    id.strip_prefix("MPREb_").is_some_and(|r| (5..=40).contains(&r.len()) && chars(r))
}
pub fn valid_artist(id: &str) -> bool {
    id.strip_prefix("UC").is_some_and(|r| r.len() == 22 && chars(r))
}
/// A playlist as YT Music browses it: VL + PL…, OLAK5uy_… (an album's), RD… (a mix).
pub fn valid_playlist(id: &str) -> bool {
    let id = id.strip_prefix("VL").unwrap_or(id);
    ["PL", "OLAK5uy_", "RD"].iter().any(|p| id.strip_prefix(p).is_some_and(|r| (8..=80).contains(&r.len()) && chars(r)))
}

// ---- reading replies ----

fn text(runs: &Value) -> String {
    runs["runs"].as_array().map(|r| r.iter().filter_map(|x| x["text"].as_str()).collect::<String>()).unwrap_or_default().trim().to_string()
}

/// "3:45" or "1:02:03" → seconds.
fn seconds(s: &str) -> Option<f64> {
    let parts: Vec<&str> = s.trim().split(':').collect();
    if !(2..=3).contains(&parts.len()) || parts.iter().any(|p| p.is_empty() || !p.bytes().all(|b| b.is_ascii_digit()) || p.len() > 3) {
        return None;
    }
    Some(parts.iter().fold(0.0, |acc, p| acc * 60.0 + p.parse::<f64>().unwrap_or(0.0)))
}

/// The best thumbnail (for square art, asked for at 544 px), from a host the page may load.
fn thumb(v: &Value) -> Option<String> {
    let list = find(v, "thumbnails")?.as_array()?;
    let best = list.iter().max_by_key(|t| t["width"].as_u64().unwrap_or(0))?;
    let url = best["url"].as_str()?;
    let u = url::Url::parse(url).ok()?;
    if u.scheme() != "https" || !u.host_str().is_some_and(|h| THUMB_HOSTS.contains(&h)) {
        return None;
    }
    // lh3/yt3 art takes its size from the URL's tail: ask for one sharp enough for a big header
    Some(match url.rfind("=w") {
        Some(i) if !url.contains("ytimg") => format!("{}=w544-h544-l90-rj", &url[..i]),
        _ => url.to_string(),
    })
}

/// The first value under `key` anywhere inside `v` (depth-first).
fn find<'a>(v: &'a Value, key: &str) -> Option<&'a Value> {
    match v {
        Value::Object(m) => m.get(key).or_else(|| m.values().find_map(|x| find(x, key))),
        Value::Array(a) => a.iter().find_map(|x| find(x, key)),
        _ => None,
    }
}

/// What a navigation endpoint points at: ("song", video id) or (kind, browse id).
fn target(nav: &Value) -> Option<(&'static str, String)> {
    if let Some(v) = nav["watchEndpoint"]["videoId"].as_str().filter(|v| valid_video(v)) {
        return Some(("song", v.to_string()));
    }
    let b = &nav["browseEndpoint"];
    let id = b["browseId"].as_str()?;
    let page = b["browseEndpointContextSupportedConfigs"]["browseEndpointContextMusicConfig"]["pageType"].as_str().unwrap_or("");
    let kind = match page {
        "MUSIC_PAGE_TYPE_ALBUM" | "MUSIC_PAGE_TYPE_AUDIOBOOK" => "album",
        "MUSIC_PAGE_TYPE_ARTIST" | "MUSIC_PAGE_TYPE_USER_CHANNEL" => "artist",
        "MUSIC_PAGE_TYPE_PLAYLIST" => "playlist",
        _ if valid_album(id) => "album",
        _ if valid_artist(id) => "artist",
        _ if valid_playlist(id) => "playlist",
        _ => return None,
    };
    let ok = match kind {
        "album" => valid_album(id),
        "artist" => valid_artist(id),
        _ => valid_playlist(id),
    };
    ok.then(|| (kind, id.to_string()))
}

/// Artists, album and duration from a subtitle's runs: "Song • Artist & Artist • Album • 3:45".
fn credits(runs: &[&Value]) -> (Vec<Ref>, Option<Ref>, Option<f64>) {
    let (mut artists, mut album, mut duration) = (Vec::new(), None, None);
    for r in runs {
        let name = r["text"].as_str().unwrap_or_default().trim();
        match target(&r["navigationEndpoint"]) {
            Some(("artist", id)) => artists.push(Ref { name: name.into(), id: Some(id) }),
            Some(("album", id)) => album = Some(Ref { name: name.into(), id: Some(id) }),
            _ => duration = duration.or_else(|| seconds(name)),
        }
    }
    (artists, album, duration)
}

const LABELS: [&str; 10] = ["Song", "Video", "Album", "Single", "EP", "Playlist", "Artist", "Episode", "Lagu", "Video musik"];

/// With no linked artist (often in search), the first plain part of the subtitle that isn't a label or a number.
fn unlinked_artist(subtitle: &str) -> Vec<Ref> {
    subtitle
        .split(" • ")
        .map(str::trim)
        .find(|p| !p.is_empty() && !LABELS.contains(p) && seconds(p).is_none() && !p.ends_with("views") && !p.ends_with("plays"))
        .map(|n| vec![Ref { name: n.into(), id: None }])
        .unwrap_or_default()
}

/// A row in a list (search results, an album's tracks, top songs).
fn list_item(r: &Value) -> Option<Item> {
    let cols: Vec<&Value> = r["flexColumns"].as_array()?.iter().map(|c| &c["musicResponsiveListItemFlexColumnRenderer"]["text"]).collect();
    let first = cols.first()?;
    let title = text(first);
    let video = r["playlistItemData"]["videoId"].as_str().filter(|v| valid_video(v)).map(str::to_owned).or_else(|| {
        first["runs"].as_array().and_then(|a| a.first()).and_then(|x| target(&x["navigationEndpoint"])).filter(|t| t.0 == "song").map(|t| t.1)
    });
    let (kind, id) = match video {
        Some(v) => ("song", v),
        None => target(&r["navigationEndpoint"])?,
    };
    let runs: Vec<&Value> = cols.iter().skip(1).filter_map(|c| c["runs"].as_array()).flatten().collect();
    let subtitle = runs.iter().filter_map(|x| x["text"].as_str()).collect::<String>().trim().to_string();
    let (mut artists, album, mut duration) = credits(&runs);
    if let Some(fixed) = r["fixedColumns"].as_array().and_then(|f| f.first()) {
        duration = duration.or_else(|| seconds(&text(&fixed["musicResponsiveListItemFixedColumnRenderer"]["text"])));
    }
    if artists.is_empty() && kind == "song" {
        artists = unlinked_artist(&subtitle);
    }
    Some(Item { kind, id, title, subtitle, artists, album, duration, thumb: r.get("thumbnail").and_then(thumb) })
}

/// A card in a shelf (home, an artist's albums).
fn card(r: &Value) -> Option<Item> {
    let (kind, id) = target(&r["navigationEndpoint"]).or_else(|| target(&r["title"]["runs"][0]["navigationEndpoint"]))?;
    let runs: Vec<&Value> = r["subtitle"]["runs"].as_array().map(|a| a.iter().collect()).unwrap_or_default();
    let subtitle = text(&r["subtitle"]);
    let (mut artists, album, duration) = credits(&runs);
    if artists.is_empty() && kind != "artist" {
        artists = unlinked_artist(&subtitle);
    }
    Some(Item { kind, id, title: text(&r["title"]), subtitle, artists, album, duration, thumb: r.get("thumbnailRenderer").and_then(thumb) })
}

/// The "Top result" card in search.
fn top_card(r: &Value) -> Option<Item> {
    let (kind, id) = target(&r["title"]["runs"][0]["navigationEndpoint"]).or_else(|| target(&r["onTap"]))?;
    let runs: Vec<&Value> = r["subtitle"]["runs"].as_array().map(|a| a.iter().collect()).unwrap_or_default();
    let (artists, album, duration) = credits(&runs);
    Some(Item { kind, id, title: text(&r["title"]), subtitle: text(&r["subtitle"]), artists, album, duration, thumb: r.get("thumbnail").and_then(thumb) })
}

/// A song in a radio / up-next queue.
fn panel_item(r: &Value) -> Option<Item> {
    let id = r["videoId"].as_str().filter(|v| valid_video(v))?.to_string();
    let runs: Vec<&Value> = r["longBylineText"]["runs"].as_array().map(|a| a.iter().collect()).unwrap_or_default();
    let subtitle = text(&r["longBylineText"]);
    let (mut artists, album, _) = credits(&runs);
    if artists.is_empty() {
        artists = unlinked_artist(&subtitle);
    }
    Some(Item { kind: "song", id, title: text(&r["title"]), subtitle, artists, album, duration: seconds(&text(&r["lengthText"])), thumb: thumb(&r["thumbnail"]) })
}

fn items_of(list: &Value) -> Vec<Item> {
    list.as_array()
        .into_iter()
        .flatten()
        .filter_map(|x| {
            x.get("musicResponsiveListItemRenderer").and_then(list_item).or_else(|| x.get("musicTwoRowItemRenderer").and_then(card)).or_else(|| {
                let p = x.get("playlistPanelVideoRenderer").or_else(|| x["playlistPanelVideoWrapperRenderer"]["primaryRenderer"].get("playlistPanelVideoRenderer"));
                p.and_then(panel_item)
            })
        })
        .collect()
}

const SHELVES: [&str; 5] = ["musicShelfRenderer", "musicCarouselShelfRenderer", "musicPlaylistShelfRenderer", "gridRenderer", "musicCardShelfRenderer"];

/// Every shelf in a reply, in order, with its title.
fn shelves(v: &Value, out: &mut Vec<Shelf>) {
    match v {
        Value::Object(m) => {
            for (k, x) in m {
                if !SHELVES.contains(&k.as_str()) {
                    shelves(x, out);
                    continue;
                }
                let title = [&x["title"], &x["header"]["musicCarouselShelfBasicHeaderRenderer"]["title"], &x["header"]["gridHeaderRenderer"]["title"]]
                    .into_iter()
                    .map(text)
                    .find(|t| !t.is_empty())
                    .unwrap_or_default();
                let items = if k == "musicCardShelfRenderer" {
                    let mut it: Vec<Item> = top_card(x).into_iter().collect();
                    it.extend(items_of(&x["contents"]));
                    it
                } else {
                    items_of(if x["contents"].is_array() { &x["contents"] } else { &x["items"] })
                };
                if !items.is_empty() {
                    out.push(Shelf { title: if k == "musicCardShelfRenderer" { "Top result".into() } else { title }, items });
                }
            }
        }
        Value::Array(a) => a.iter().for_each(|x| shelves(x, out)),
        _ => {}
    }
}

pub fn parse_shelves(v: &Value) -> Vec<Shelf> {
    let mut out = Vec::new();
    shelves(v, &mut out);
    out
}

const HEADERS: [&str; 5] = ["musicResponsiveHeaderRenderer", "musicDetailHeaderRenderer", "musicImmersiveHeaderRenderer", "musicVisualHeaderRenderer", "musicEditablePlaylistDetailHeaderRenderer"];

/// An album, playlist or artist page: its header, its songs, its other shelves.
pub fn parse_page(v: &Value) -> Page {
    let header = HEADERS.iter().find_map(|h| find(v, h)).cloned().unwrap_or(Value::Null);
    // an editable playlist wraps the real header
    let header = find(&header, "musicResponsiveHeaderRenderer").cloned().unwrap_or(header);
    let strap: Vec<&Value> = header["straplineTextOne"]["runs"].as_array().map(|a| a.iter().collect()).unwrap_or_default();
    let sub: Vec<&Value> = header["subtitle"]["runs"].as_array().map(|a| a.iter().collect()).unwrap_or_default();
    let mut artists = credits(&strap).0;
    if artists.is_empty() {
        artists = credits(&sub).0;
    }
    if artists.is_empty() {
        artists = strap.iter().filter_map(|r| r["text"].as_str()).map(str::trim).filter(|t| !t.is_empty() && *t != "•").take(1).map(|n| Ref { name: n.into(), id: None }).collect();
    }
    let thumb_of = header.get("thumbnail").or_else(|| header.get("foregroundThumbnail")).and_then(thumb);
    let mut page = Page { title: text(&header["title"]), subtitle: text(&header["subtitle"]), artists, thumb: thumb_of, ..Default::default() };
    for shelf in parse_shelves(v) {
        let songs = shelf.items.iter().all(|i| i.kind == "song");
        if songs && page.tracks.is_empty() {
            page.tracks = shelf.items;
        } else {
            page.shelves.push(shelf);
        }
    }
    // album tracks carry neither the artist nor the art: they're the album's
    for t in &mut page.tracks {
        if t.artists.is_empty() {
            t.artists = page.artists.clone();
        }
        if t.thumb.is_none() {
            t.thumb = page.thumb.clone();
        }
    }
    page
}

pub fn parse_radio(v: &Value) -> Vec<Item> {
    find(v, "playlistPanelRenderer").map(|p| items_of(&p["contents"])).unwrap_or_default()
}

// ---- requests ----

/// "en" or "id": the language YT Music answers in (titles of shelves, "Song", "Album").
fn language(hl: &str) -> &'static str {
    if hl == "id" { "id" } else { "en" }
}

async fn call(endpoint: &str, hl: &str, extra: Value) -> Result<Value, String> {
    use tauri_plugin_http::reqwest::Client;
    let mut body = json!({ "context": { "client": { "clientName": "WEB_REMIX", "clientVersion": CLIENT_VERSION, "hl": language(hl) } } });
    if let (Some(b), Some(e)) = (body.as_object_mut(), extra.as_object()) {
        b.extend(e.clone());
    }
    let client = Client::builder().user_agent(crate::UA).timeout(std::time::Duration::from_secs(15)).build().map_err(|e| e.to_string())?;
    let r = client
        .post(format!("{BASE}{endpoint}?prettyPrint=false"))
        .header("Content-Type", "application/json")
        .header("Origin", "https://music.youtube.com")
        .body(body.to_string())
        .send()
        .await
        .map_err(|_| "YT Music didn't answer. Check your connection.".to_string())?;
    if !r.status().is_success() {
        return Err(format!("YT Music refused the request ({}).", r.status().as_u16()));
    }
    let text = r.text().await.map_err(|_| "YT Music's reply was cut off.".to_string())?;
    serde_json::from_str(&text).map_err(|_| "Unexpected reply from YT Music.".to_string())
}

#[tauri::command]
pub async fn ytm_home(hl: String) -> Result<Vec<Shelf>, String> {
    Ok(parse_shelves(&call("browse", &hl, json!({ "browseId": "FEmusic_home" })).await?))
}

#[tauri::command]
pub async fn ytm_search(query: String, filter: Option<String>, hl: String) -> Result<Vec<Shelf>, String> {
    let q: String = query.chars().filter(|c| !c.is_control()).collect();
    let q = q.trim();
    if q.is_empty() || q.chars().count() > 200 {
        return Err("Type what to search for.".into());
    }
    let mut extra = json!({ "query": q });
    if let Some(f) = filter.as_deref() {
        let params = FILTERS.iter().find(|(n, _)| *n == f).ok_or("Unknown search filter.")?.1;
        extra["params"] = json!(params);
    }
    Ok(parse_shelves(&call("search", &hl, extra).await?))
}

#[tauri::command]
pub async fn ytm_album(id: String, hl: String) -> Result<Page, String> {
    if !valid_album(&id) {
        return Err("bad album id".into());
    }
    Ok(parse_page(&call("browse", &hl, json!({ "browseId": id })).await?))
}

#[tauri::command]
pub async fn ytm_artist(id: String, hl: String) -> Result<Page, String> {
    if !valid_artist(&id) {
        return Err("bad artist id".into());
    }
    Ok(parse_page(&call("browse", &hl, json!({ "browseId": id })).await?))
}

#[tauri::command]
pub async fn ytm_playlist(id: String, hl: String) -> Result<Page, String> {
    if !valid_playlist(&id) {
        return Err("bad playlist id".into());
    }
    let id = if id.starts_with("VL") { id } else { format!("VL{id}") };
    Ok(parse_page(&call("browse", &hl, json!({ "browseId": id })).await?))
}

/// Songs that follow on from one: YT Music's own radio for it.
#[tauri::command]
pub async fn ytm_radio(id: String, hl: String) -> Result<Vec<Item>, String> {
    if !valid_video(&id) {
        return Err("bad video id".into());
    }
    let r = call("next", &hl, json!({ "videoId": id, "playlistId": format!("RDAMVM{id}"), "isAudioOnly": true })).await?;
    Ok(parse_radio(&r).into_iter().filter(|i| i.id != id).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(name: &str) -> Value {
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/ytm").join(name);
        serde_json::from_str(&std::fs::read_to_string(p).unwrap()).unwrap()
    }

    #[test]
    fn checks_every_id_before_it_reaches_a_request() {
        assert!(valid_video("dQw4w9WgXcQ") && !valid_video("dQw4w9WgXc") && !valid_video("dQw4w9WgX/Q"));
        assert!(valid_album("MPREb_4pL8gzRtw1p") && !valid_album("MPREb_../../x") && !valid_album("PL123"));
        assert!(valid_artist("UCuAXFkgsw1L7xaCfnd5JJOw") && !valid_artist("UCshort") && !valid_artist("UCuAXFkgsw1L7xaCfnd5JJO?"));
        for ok in ["VLPLx0sYbCqOb8TBPRdmBHs5Iftvv9TPboYG", "OLAK5uy_kIIM2ZZxUw0PyXpG9N11HNeH2nVRYTRWk", "RDCLAK5uy_kmPRjHDECIcuVwnKsx2Ng7fyNgFKWNJFs"] {
            assert!(valid_playlist(ok), "{ok}");
        }
        for bad in ["VLLM", "PL", "XX123456789", "PLabc def123", "PL/../../browse"] {
            assert!(!valid_playlist(bad), "{bad}");
        }
    }

    #[test]
    fn reads_home_shelves() {
        let s = parse_shelves(&fixture("home.json"));
        assert_eq!(s.iter().map(|s| s.title.as_str()).collect::<Vec<_>>(), ["Quick picks", "Albums for you"]);
        let song = &s[0].items[0];
        assert_eq!((song.kind, song.id.as_str(), song.title.as_str()), ("song", "dQw4w9WgXcQ", "Never Gonna Give You Up"));
        assert_eq!(song.artists, [Ref { name: "Rick Astley".into(), id: Some("UCuAXFkgsw1L7xaCfnd5JJOw".into()) }]);
        assert_eq!(song.album.as_ref().map(|a| a.name.as_str()), Some("Whenever You Need Somebody"));
        // square art is asked for at a size fit for a header
        assert_eq!(song.thumb.as_deref(), Some("https://lh3.googleusercontent.com/abc=w544-h544-l90-rj"));
        let album = &s[1].items[0];
        assert_eq!((album.kind, album.id.as_str(), album.subtitle.as_str()), ("album", "MPREb_4pL8gzRtw1p", "Album • Rick Astley"));
        // a card pointing somewhere odd, and a thumbnail from another host, are dropped
        assert_eq!(s[1].items.len(), 1);
        assert_eq!(s[1].items[0].thumb, None);
    }

    #[test]
    fn reads_search_results_with_a_top_result() {
        let s = parse_shelves(&fixture("search.json"));
        assert_eq!(s[0].title, "Top result");
        assert_eq!((s[0].items[0].kind, s[0].items[0].id.as_str()), ("artist", "UCuAXFkgsw1L7xaCfnd5JJOw"));
        let songs = &s[1];
        assert_eq!(songs.title, "Songs");
        let t = &songs.items[0];
        assert_eq!((t.kind, t.duration, t.artists[0].name.as_str()), ("song", Some(213.0), "Rick Astley"));
        // no linked artist: the subtitle's first plain part, not the "Song" label
        assert_eq!(songs.items[1].artists, [Ref { name: "Various".into(), id: None }]);
    }

    #[test]
    fn reads_an_album_page_and_gives_tracks_its_artist_and_art() {
        let p = parse_page(&fixture("album.json"));
        assert_eq!((p.title.as_str(), p.subtitle.as_str()), ("Whenever You Need Somebody", "Album • 1987"));
        assert_eq!(p.artists[0].id.as_deref(), Some("UCuAXFkgsw1L7xaCfnd5JJOw"));
        assert_eq!(p.tracks.len(), 2);
        assert_eq!((p.tracks[1].title.as_str(), p.tracks[1].duration), ("Whenever You Need Somebody", Some(234.0)));
        assert_eq!(p.tracks[1].artists, p.artists);
        assert_eq!(p.tracks[1].thumb, p.thumb);
    }

    #[test]
    fn reads_an_artist_page() {
        let p = parse_page(&fixture("artist.json"));
        assert_eq!(p.title, "Rick Astley");
        assert_eq!(p.tracks[0].id, "dQw4w9WgXcQ");
        assert_eq!(p.shelves.iter().map(|s| s.title.as_str()).collect::<Vec<_>>(), ["Albums"]);
    }

    #[test]
    fn reads_a_radio_queue() {
        let r = parse_radio(&fixture("radio.json"));
        assert_eq!(r.iter().map(|i| i.id.as_str()).collect::<Vec<_>>(), ["dQw4w9WgXcQ", "yPYZpwSpKmA"]);
        assert_eq!((r[1].duration, r[1].artists[0].name.as_str()), (Some(201.0), "Rick Astley"));
    }

    #[test]
    fn fuzz_odd_replies_never_panic() {
        let all = ["home.json", "search.json", "album.json", "artist.json", "radio.json"].map(fixture);
        let mut x = 0x1234_5678u32;
        for _ in 0..400 {
            for f in &all {
                let mut v = f.clone();
                // knock out or replace random parts
                fn walk(v: &mut Value, x: &mut u32) {
                    *x ^= *x << 13;
                    *x ^= *x >> 17;
                    *x ^= *x << 5;
                    match v {
                        Value::Object(m) => {
                            if (*x).is_multiple_of(7)
                                && let Some(k) = m.keys().next().cloned() {
                                    m.insert(k, json!(null));
                                }
                            m.values_mut().for_each(|c| walk(c, x));
                        }
                        Value::Array(a) => a.iter_mut().for_each(|c| walk(c, x)),
                        Value::String(s) if (*x).is_multiple_of(11) => *s = "1:2:3:4".into(),
                        _ => {}
                    }
                }
                walk(&mut v, &mut x);
                let _ = (parse_shelves(&v), parse_page(&v), parse_radio(&v));
            }
        }
    }

    #[test]
    #[ignore = "network: run on purpose to see YT Music still answers in the shapes this reads"]
    fn live_guest_api_answers() {
        let home = tauri::async_runtime::block_on(ytm_home("en".into())).unwrap();
        assert!(!home.is_empty());
        let found = tauri::async_runtime::block_on(ytm_search("Rick Astley".into(), Some("songs".into()), "en".into())).unwrap();
        let song = found.iter().flat_map(|s| &s.items).find(|i| i.kind == "song").unwrap().clone();
        assert!(!tauri::async_runtime::block_on(ytm_radio(song.id, "en".into())).unwrap().is_empty());
    }
}
