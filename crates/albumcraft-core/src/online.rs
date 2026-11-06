//! Online maps, used only after the user turned them on (`Store.online_maps`): map tiles from
//! OpenStreetMap and place search from its Nominatim service. Nothing here runs otherwise.
//!
//! Both services ask their users to identify the program, show the attribution, cache tiles and
//! keep the load low. Here: a User-Agent naming AlbumCraft, tiles kept on disk for two weeks, and
//! at most one search per second, only when the user asks (no search as you type). The app's
//! tile downloader (at most two downloads, only the tiles in view) is in its `online` module.

use crate::geo::{Found, LatLon};
use image::RgbaImage;
use std::path::PathBuf;
use std::sync::{Mutex, PoisonError};
use std::time::{Duration, Instant, SystemTime};

pub const ATTRIBUTION: &str = "© OpenStreetMap contributors";
pub const COPYRIGHT_URL: &str = "https://www.openstreetmap.org/copyright";
const TILE_URL: &str = "https://tile.openstreetmap.org";
const SEARCH_URL: &str = "https://nominatim.openstreetmap.org";
/// The most detailed tiles the server has.
pub const MAX_ZOOM: u8 = 19;
/// A downloaded tile is used without asking again for this long.
const FRESH: Duration = Duration::from_secs(14 * 24 * 3600);
/// Size the tile cache is kept under.
pub const CACHE_BYTES: u64 = 500 << 20;

fn user_agent() -> String {
    format!("AlbumCraft/{} (+https://github.com/kbarni/AlbumCraft)", env!("CARGO_PKG_VERSION"))
}

/// The HTTP client for the map services (timeouts, User-Agent).
pub fn agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(20)))
        .user_agent(user_agent())
        .build()
        .into()
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct TileId {
    pub z: u8,
    pub x: u32,
    pub y: u32,
}

impl TileId {
    fn url(&self) -> String {
        format!("{TILE_URL}/{}/{}/{}.png", self.z, self.x, self.y)
    }

    fn cache_path(&self, dir: &std::path::Path) -> PathBuf {
        dir.join(self.z.to_string()).join(self.x.to_string()).join(format!("{}.png", self.y))
    }
}

/// Where downloaded tiles are kept.
pub fn tile_cache_dir() -> Option<PathBuf> {
    Some(dirs::cache_dir()?.join("AlbumCraft").join("tiles"))
}

/// A tile from the disk cache when fresh, else downloaded (falling back to an old copy offline).
pub fn load_tile(agent: &ureq::Agent, dir: Option<&std::path::Path>, id: TileId) -> Result<RgbaImage, String> {
    let path = dir.map(|d| id.cache_path(d));
    let cached = path.as_ref().and_then(|p| {
        let age = std::fs::metadata(p).and_then(|m| m.modified()).ok().and_then(|t| SystemTime::now().duration_since(t).ok())?;
        Some((std::fs::read(p).ok()?, age))
    });
    let bytes = match cached {
        Some((b, age)) if age < FRESH => b,
        old => match download(agent, &id.url()) {
            Ok(b) => {
                if let Some(p) = &path {
                    if let Some(parent) = p.parent() {
                        let _ = std::fs::create_dir_all(parent);
                    }
                    let _ = crate::library::write_library(p, &b);
                }
                b
            }
            Err(e) => old.map(|(b, _)| b).ok_or(e)?,
        },
    };
    let img = image::load_from_memory_with_format(&bytes, image::ImageFormat::Png).map_err(|e| format!("a map tile could not be read: {e}"))?;
    Ok(img.to_rgba8())
}

fn download(agent: &ureq::Agent, url: &str) -> Result<Vec<u8>, String> {
    let mut resp = agent.get(url).call().map_err(|e| format!("Online map not reachable ({e})"))?;
    resp.body_mut().with_config().limit(4 << 20).read_to_vec().map_err(|e| format!("Online map not reachable ({e})"))
}

/// Remove the least recently written tiles when the cache is bigger than `max_bytes`.
pub fn prune(dir: &std::path::Path, max_bytes: u64) {
    let mut files = vec![];
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&d) else { continue };
        for e in rd.flatten() {
            let Ok(m) = e.metadata() else { continue };
            if m.is_dir() {
                stack.push(e.path());
            } else {
                files.push((m.modified().unwrap_or(SystemTime::UNIX_EPOCH), m.len(), e.path()));
            }
        }
    }
    let mut total: u64 = files.iter().map(|f| f.1).sum();
    if total <= max_bytes {
        return;
    }
    files.sort_unstable_by_key(|f| f.0);
    for (_, len, path) in files {
        if total <= max_bytes * 9 / 10 {
            break;
        }
        if std::fs::remove_file(&path).is_ok() {
            total = total.saturating_sub(len);
        }
    }
}

// ---------------------------------------------------------------------------------------------
// place search (Nominatim)

/// Keeps searches at least a second apart, as the service asks.
fn wait_turn() {
    static LAST: Mutex<Option<Instant>> = Mutex::new(None);
    let mut last = LAST.lock().unwrap_or_else(PoisonError::into_inner);
    if let Some(t) = *last {
        let gap = Duration::from_millis(1100);
        if t.elapsed() < gap {
            std::thread::sleep(gap - t.elapsed());
        }
    }
    *last = Some(Instant::now());
}

fn encode(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => out.push(b as char),
            b' ' => out.push('+'),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

fn get_json(url: &str) -> Result<serde_json::Value, String> {
    wait_turn();
    let mut resp = agent().get(url).call().map_err(|e| format!("The online search is not reachable ({e})."))?;
    let text = resp.body_mut().with_config().limit(2 << 20).read_to_string().map_err(|e| e.to_string())?;
    serde_json::from_str(&text).map_err(|e| format!("Unexpected answer from the online search: {e}"))
}

/// "Place de la Bourse, Bordeaux, France" from an answer's address parts.
fn short_name(v: &serde_json::Value) -> String {
    let a = &v["address"];
    let s = |k: &str| a[k].as_str().unwrap_or("").to_string();
    let town = ["city", "town", "village", "hamlet", "municipality", "county", "state"].iter().map(|k| s(k)).find(|t| !t.is_empty()).unwrap_or_default();
    let name = v["name"].as_str().unwrap_or("").to_string();
    let first = if name.is_empty() { s("suburb") } else { name };
    let mut parts: Vec<String> = vec![];
    for p in [first, town, s("country")] {
        if !p.is_empty() && !parts.contains(&p) {
            parts.push(p);
        }
    }
    if parts.is_empty() {
        return v["display_name"].as_str().unwrap_or("").chars().take(120).collect();
    }
    parts.join(", ")
}

fn found_of(v: &serde_json::Value) -> Option<Found> {
    let num = |k: &str| v[k].as_str().and_then(|s| s.parse::<f64>().ok());
    let at = LatLon::new(num("lat")?, num("lon")?)?;
    // a zoom that shows the place's extent: [south, north, west, east]
    let bb: Vec<f64> = v["boundingbox"].as_array().map(|a| a.iter().filter_map(|x| x.as_str()?.parse().ok()).collect()).unwrap_or_default();
    let zoom = match bb[..] {
        [s, n, w, e] => {
            let span = ((e - w).abs()).max((n - s).abs() * 1.5).max(1e-4);
            (360.0 / span).log2().clamp(3.0, 17.0)
        }
        _ => 14.0,
    };
    let name: String = v["display_name"].as_str().unwrap_or("").chars().take(200).collect();
    Some(Found { name, short: short_name(v), at, zoom })
}

/// Search for addresses and places. Blocking: run it off the UI thread.
pub fn search(query: &str) -> Result<Vec<Found>, String> {
    let q: String = query.trim().chars().take(200).collect();
    if q.is_empty() {
        return Ok(vec![]);
    }
    let v = get_json(&format!("{SEARCH_URL}/search?format=jsonv2&addressdetails=1&limit=10&q={}", encode(&q)))?;
    let all: Vec<Found> = v.as_array().map(|a| a.iter().filter_map(found_of).collect()).unwrap_or_default();
    Ok(dedup(all))
}

/// One result per place: the map data often has several objects (a square, its outline, a stop…)
/// with the same name at the same spot.
fn dedup(all: Vec<Found>) -> Vec<Found> {
    let mut out: Vec<Found> = vec![];
    for f in all {
        if !out.iter().any(|o| o.short == f.short && o.at.km_to(f.at) < 0.3) {
            out.push(f);
        }
    }
    out
}

/// The name of the place at a position ("Saint-Pierre, Bordeaux, France"). Blocking.
pub fn name_of(at: LatLon) -> Result<String, String> {
    let v = get_json(&format!("{SEARCH_URL}/reverse?format=jsonv2&addressdetails=1&zoom=16&lat={:.6}&lon={:.6}", at.lat, at.lon))?;
    if v.get("error").is_some() {
        return Ok(String::new());
    }
    Ok(short_name(&v))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn queries_are_encoded() {
        assert_eq!(encode("Place de la Bourse"), "Place+de+la+Bourse");
        assert_eq!(encode("München&x=1"), "M%C3%BCnchen%26x%3D1");
    }

    #[test]
    fn answers_are_read_defensively() {
        let good = serde_json::json!({
            "lat": "44.8412", "lon": "-0.5695", "name": "Place de la Bourse",
            "display_name": "Place de la Bourse, Bordeaux, Gironde, France",
            "boundingbox": ["44.8405", "44.8420", "-0.5705", "-0.5685"],
            "address": { "road": "Place de la Bourse", "city": "Bordeaux", "country": "France" }
        });
        let f = found_of(&good).unwrap();
        assert_eq!(f.short, "Place de la Bourse, Bordeaux, France");
        assert!(f.zoom > 15.0);
        for bad in [
            serde_json::json!({}),
            serde_json::json!({ "lat": "x", "lon": "1" }),
            serde_json::json!({ "lat": "95", "lon": "1" }),
            serde_json::json!({ "lat": 4, "lon": 1 }),
        ] {
            assert!(found_of(&bad).is_none());
        }
        // no bounding box: still a result
        assert!(found_of(&serde_json::json!({ "lat": "1", "lon": "2" })).is_some());
        // the same place twice (two map objects) is shown once
        let twice = dedup(vec![f.clone(), Found { at: LatLon { lat: 44.8413, lon: -0.5696 }, ..f.clone() }]);
        assert_eq!(twice.len(), 1);
    }

    /// Talks to the real servers (one tile, one search, one reverse lookup):
    /// `cargo test online_services -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn online_services() {
        let dir = std::env::temp_dir().join(format!("albumcraft-tiles-{}", std::process::id()));
        let img = load_tile(&agent(), Some(&dir), TileId { z: 3, x: 3, y: 2 }).unwrap();
        assert_eq!(img.dimensions(), (256, 256));
        assert!(TileId { z: 3, x: 3, y: 2 }.cache_path(&dir).is_file(), "kept on disk");
        let found = search("Place de la Bourse, Bordeaux").unwrap();
        eprintln!("{found:#?}");
        assert!(found.first().is_some_and(|f| f.at.km_to(LatLon { lat: 44.841, lon: -0.570 }) < 2.0));
        let name = name_of(LatLon { lat: 44.8412, lon: -0.5695 }).unwrap();
        eprintln!("reverse: {name}");
        assert!(name.contains("Bordeaux"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn tile_paths() {
        let id = TileId { z: 5, x: 15, y: 11 };
        assert_eq!(id.url(), "https://tile.openstreetmap.org/5/15/11.png");
        assert!(id.cache_path(std::path::Path::new("c")).ends_with("5/15/11.png"));
    }
}
