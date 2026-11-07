//! Where pictures were taken: coordinates, the place kept in a picture's notes, the GPS position a
//! file carries, the built-in list of world cities (place search and "Near …" labels) and the map
//! projection the map view uses.
//!
//! The picture's own file is never changed: a place set by the user lives in `Meta.place` and
//! overrides the file's GPS; `Place::Removed` hides the file's GPS too. Exports write the
//! resulting position into the copy's EXIF (`exif_gps_fields`).

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::sync::{Arc, OnceLock};

// ---------------------------------------------------------------------------------------------
// coordinates

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LatLon {
    pub lat: f64,
    pub lon: f64,
}

impl LatLon {
    /// A position, or None when the numbers are out of range or not numbers.
    pub fn new(lat: f64, lon: f64) -> Option<LatLon> {
        let ok = lat.is_finite() && lon.is_finite() && (-90.0..=90.0).contains(&lat) && (-180.0..=180.0).contains(&lon);
        ok.then_some(LatLon { lat, lon })
    }

    /// "44.83780° N, 0.57920° W"
    pub fn text(&self) -> String {
        let ns = if self.lat < 0.0 { 'S' } else { 'N' };
        let ew = if self.lon < 0.0 { 'W' } else { 'E' };
        format!("{:.5}° {ns}, {:.5}° {ew}", self.lat.abs(), self.lon.abs())
    }

    /// Great-circle distance in kilometres.
    pub fn km_to(&self, o: LatLon) -> f64 {
        let (p1, p2) = (self.lat.to_radians(), o.lat.to_radians());
        let dp = p2 - p1;
        let dl = (o.lon - self.lon).to_radians();
        let a = (dp / 2.0).sin().powi(2) + p1.cos() * p2.cos() * (dl / 2.0).sin().powi(2);
        2.0 * 6371.0 * a.sqrt().min(1.0).asin()
    }
}

/// Read a position typed by a person: "44.84, -0.58", "44.84 N 0.58 W", "44°50'16\"N 0°34'45\"W",
/// "N 44 50.3 W 0 34.7". Returns None for anything else (a place name, say).
pub fn parse_coords(s: &str) -> Option<LatLon> {
    #[derive(Clone, Copy, PartialEq, Debug)]
    enum Tok {
        Num(f64),
        Hemi(char),
        Sep,
    }
    let mut toks = vec![];
    let mut num = String::new();
    let flush = |num: &mut String, toks: &mut Vec<Tok>| -> Option<()> {
        if !num.is_empty() {
            toks.push(Tok::Num(num.parse::<f64>().ok().filter(|v| v.is_finite())?));
            num.clear();
        }
        Some(())
    };
    for c in s.trim().chars() {
        match c {
            '0'..='9' | '.' => num.push(c),
            '-' | '+' | '−' => {
                flush(&mut num, &mut toks)?;
                if c != '+' {
                    num.push('-');
                }
            }
            'N' | 'S' | 'E' | 'W' | 'n' | 's' | 'e' | 'w' | 'O' | 'o' => {
                flush(&mut num, &mut toks)?;
                // "O" (ouest/oeste) means west in several languages
                let h = match c.to_ascii_uppercase() {
                    'O' => 'W',
                    u => u,
                };
                toks.push(Tok::Hemi(h));
            }
            ',' | ';' => {
                flush(&mut num, &mut toks)?;
                toks.push(Tok::Sep);
            }
            '°' | '\'' | '"' | '′' | '″' | '’' | '”' | 'º' => flush(&mut num, &mut toks)?,
            c if c.is_whitespace() => flush(&mut num, &mut toks)?,
            _ => return None,
        }
    }
    flush(&mut num, &mut toks)?;

    // split the tokens into two parts, each a list of numbers with an optional hemisphere
    let mut parts: Vec<(Vec<f64>, Option<char>)> = vec![];
    let has_hemi = toks.iter().any(|t| matches!(t, Tok::Hemi(_)));
    if has_hemi {
        // a letter either closes the numbers before it ("44.8 N") or opens those after it ("N 44.8")
        let leading = matches!(toks.first(), Some(Tok::Hemi(_)));
        let mut cur: (Vec<f64>, Option<char>) = (vec![], None);
        for t in toks {
            match t {
                Tok::Num(v) => cur.0.push(v),
                Tok::Hemi(h) if leading => {
                    if !cur.0.is_empty() || cur.1.is_some() {
                        parts.push(std::mem::take(&mut cur));
                    }
                    cur.1 = Some(h);
                }
                Tok::Hemi(h) => {
                    cur.1 = Some(h);
                    parts.push(std::mem::take(&mut cur));
                }
                Tok::Sep => {}
            }
        }
        if !cur.0.is_empty() || cur.1.is_some() {
            parts.push(cur);
        }
    } else {
        let seps = toks.iter().filter(|t| **t == Tok::Sep).count();
        let nums: Vec<f64> = toks.iter().filter_map(|t| if let Tok::Num(v) = t { Some(*v) } else { None }).collect();
        if seps == 1 {
            let mut cur = vec![];
            for t in toks {
                match t {
                    Tok::Num(v) => cur.push(v),
                    Tok::Sep => parts.push((std::mem::take(&mut cur), None)),
                    Tok::Hemi(_) => {}
                }
            }
            parts.push((cur, None));
        } else if seps == 0 && matches!(nums.len(), 2 | 4 | 6) {
            let half = nums.len() / 2;
            parts.push((nums[..half].to_vec(), None));
            parts.push((nums[half..].to_vec(), None));
        } else {
            return None;
        }
    }
    if parts.len() != 2 {
        return None;
    }
    let value = |(nums, hemi): &(Vec<f64>, Option<char>)| -> Option<f64> {
        let (d, rest) = nums.split_first()?;
        if rest.len() > 2 || rest.iter().any(|v| !(0.0..60.0).contains(v)) || (!rest.is_empty() && d.fract() != 0.0) {
            return None;
        }
        let mag = d.abs() + rest.first().unwrap_or(&0.0) / 60.0 + rest.get(1).unwrap_or(&0.0) / 3600.0;
        let negative = d.is_sign_negative() || matches!(hemi, Some('S' | 'W'));
        Some(if negative { -mag } else { mag })
    };
    let (a, b) = (value(&parts[0])?, value(&parts[1])?);
    let a_is_lon = matches!(parts[0].1, Some('E' | 'W'));
    let b_is_lat = matches!(parts[1].1, Some('N' | 'S'));
    let (lat, lon) = if a_is_lon || b_is_lat { (b, a) } else { (a, b) };
    LatLon::new(lat, lon)
}

// ---------------------------------------------------------------------------------------------
// the place in a picture's notes

/// Where a picture was taken, as set by the user (kept in `Meta.place`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Place {
    /// Set by the user; overrides the GPS position stored in the file.
    Set {
        lat: f64,
        lon: f64,
        /// The name chosen when it was set ("Bordeaux, France"), or empty.
        #[serde(default, skip_serializing_if = "String::is_empty")]
        name: String,
    },
    /// The user removed the location, including the one stored in the file.
    Removed,
}

impl Place {
    pub fn set(at: LatLon, name: &str) -> Place {
        Place::Set { lat: at.lat, lon: at.lon, name: name.trim().chars().take(200).collect() }
    }

    /// False for a hand-edited or damaged value (dropped on load).
    pub fn is_valid(&self) -> bool {
        match self {
            Place::Set { lat, lon, .. } => LatLon::new(*lat, *lon).is_some(),
            Place::Removed => true,
        }
    }
}

/// The position to use for a picture: the user's place, else the file's GPS.
pub fn effective(place: Option<&Place>, file: Option<LatLon>) -> Option<LatLon> {
    match place {
        Some(Place::Set { lat, lon, .. }) => LatLon::new(*lat, *lon),
        Some(Place::Removed) => None,
        None => file,
    }
}

/// A short description of a position: the stored name, else the nearest city ("Near Bordeaux,
/// France"), else the coordinates.
/// Words a search can find a located picture by: the place name set by the user, and the
/// nearest city (within 60 km) with its region and country.
pub fn place_terms(place: Option<&Place>, at: LatLon) -> Vec<String> {
    let mut out: Vec<String> = vec![];
    if let Some(Place::Set { name, .. }) = place
        && !name.trim().is_empty()
    {
        out.push(name.trim().to_string());
    }
    if let Some((c, km)) = nearest(at)
        && km <= 60.0
    {
        let g = gazetteer();
        for t in [c.name.as_str(), c.admin.as_str(), g.country(c)] {
            if !t.is_empty() && !out.iter().any(|o| o.eq_ignore_ascii_case(t)) {
                out.push(t.to_string());
            }
        }
    }
    out
}

pub fn describe(place: Option<&Place>, at: LatLon) -> String {
    if let Some(Place::Set { name, .. }) = place
        && !name.is_empty()
    {
        return name.clone();
    }
    match nearest(at) {
        Some((c, km)) if km <= c.radius_km() => gazetteer().short_name(c),
        Some((c, km)) if km <= 60.0 => format!("Near {}", gazetteer().short_name(c)),
        _ => at.text(),
    }
}

// ---------------------------------------------------------------------------------------------
// GPS stored in picture files

fn rational_dms(v: &exif::Value) -> Option<f64> {
    let exif::Value::Rational(r) = v else { return None };
    let part = |i: usize| r.get(i).filter(|x| x.denom != 0).map(|x| x.to_f64());
    let d = part(0)?;
    Some(d + part(1).unwrap_or(0.0) / 60.0 + part(2).unwrap_or(0.0) / 3600.0)
}

fn ref_char(v: &exif::Value) -> Option<u8> {
    match v {
        exif::Value::Ascii(parts) => parts.first().and_then(|p| p.first()).map(|c| c.to_ascii_uppercase()),
        _ => None,
    }
}

/// The GPS position in an EXIF block, if any. (0, 0) counts as none: cameras without a fix write it.
pub fn gps_of(exif: &exif::Exif) -> Option<LatLon> {
    use exif::{In, Tag};
    let get = |t: Tag| exif.get_field(t, In::PRIMARY).map(|f| &f.value);
    let mut lat = get(Tag::GPSLatitude).and_then(rational_dms)?;
    let mut lon = get(Tag::GPSLongitude).and_then(rational_dms)?;
    if get(Tag::GPSLatitudeRef).and_then(ref_char) == Some(b'S') {
        lat = -lat;
    }
    if get(Tag::GPSLongitudeRef).and_then(ref_char) == Some(b'W') {
        lon = -lon;
    }
    if lat == 0.0 && lon == 0.0 {
        return None;
    }
    LatLon::new(lat, lon)
}

/// The GPS position stored in a picture file (JPEG, TIFF-based raw files, HEIF, PNG, WebP).
pub fn file_gps(path: &str) -> Option<LatLon> {
    if path.starts_with(crate::imaging::SAMPLE_PREFIX) {
        return None;
    }
    let file = std::fs::File::open(path).ok()?;
    let mut reader = std::io::BufReader::new(file);
    let exif = exif::Reader::new().read_from_container(&mut reader).ok()?;
    gps_of(&exif)
}

/// EXIF fields that store `at` (GPS version 2.3, degrees/minutes/seconds), for exported copies.
pub fn exif_gps_fields(at: LatLon) -> Vec<exif::Field> {
    use exif::{Field, In, Rational, Tag, Value};
    let dms = |v: f64| {
        let v = v.abs();
        let d = v.floor();
        let m = ((v - d) * 60.0).floor();
        // seconds to 1/10000 s of arc (a few millimetres)
        let s = (((v - d) * 60.0 - m) * 60.0 * 10_000.0).round();
        Value::Rational(vec![
            Rational { num: d as u32, denom: 1 },
            Rational { num: m as u32, denom: 1 },
            Rational { num: s as u32, denom: 10_000 },
        ])
    };
    let field = |tag, value| Field { tag, ifd_num: In::PRIMARY, value };
    vec![
        field(Tag::GPSVersionID, Value::Byte(vec![2, 3, 0, 0])),
        field(Tag::GPSLatitudeRef, Value::Ascii(vec![if at.lat < 0.0 { b"S".to_vec() } else { b"N".to_vec() }])),
        field(Tag::GPSLatitude, dms(at.lat)),
        field(Tag::GPSLongitudeRef, Value::Ascii(vec![if at.lon < 0.0 { b"W".to_vec() } else { b"E".to_vec() }])),
        field(Tag::GPSLongitude, dms(at.lon)),
    ]
}

// ---------------------------------------------------------------------------------------------
// background index of the files' GPS positions, cached on disk

/// One picture to look at: key (path), modified time, size in bytes.
pub type FileStamp = (String, i64, u64);

#[derive(Serialize, Deserialize, Default)]
struct GpsCache {
    version: u32,
    /// key -> (modified, size, [lat, lon] or nothing)
    files: HashMap<String, (i64, u64, Option<[f64; 2]>)>,
}

const GPS_CACHE_VERSION: u32 = 1;

pub fn gps_cache_path() -> Option<std::path::PathBuf> {
    Some(dirs::cache_dir()?.join("AlbumCraft").join("gps.json"))
}

fn load_gps_cache(path: &std::path::Path) -> GpsCache {
    std::fs::read(path)
        .ok()
        .and_then(|b| serde_json::from_slice::<GpsCache>(&b).ok())
        .filter(|c| c.version == GPS_CACHE_VERSION)
        .unwrap_or_default()
}

fn save_gps_cache(path: &std::path::Path, cache: &GpsCache) {
    let Ok(bytes) = serde_json::to_vec(cache) else { return };
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    // the cache can always be rebuilt: a failed write is not worth a message
    let _ = crate::library::write_library(path, &bytes);
}

/// Read the GPS position of each file (from the cache file `cache` when the file hasn't changed),
/// sending results in batches. Stops early when `stop` is set. Meant for its own thread.
pub fn index_files(files: Vec<FileStamp>, cache: Option<std::path::PathBuf>, tx: Sender<Vec<(String, Option<LatLon>)>>, stop: Arc<AtomicBool>) {
    let path = cache;
    let old = path.as_deref().map(load_gps_cache).unwrap_or_default();
    let mut fresh = GpsCache { version: GPS_CACHE_VERSION, files: HashMap::with_capacity(files.len()) };
    let mut batch = vec![];
    let mut last_send = std::time::Instant::now();
    let mut read_any = false;
    for (key, modified, bytes) in files {
        if stop.load(Ordering::Relaxed) {
            return;
        }
        if key.starts_with(crate::imaging::SAMPLE_PREFIX) {
            continue;
        }
        let pos = match old.files.get(&key) {
            Some(&(m, b, pos)) if m == modified && b == bytes => pos.and_then(|[lat, lon]| LatLon::new(lat, lon)),
            _ => {
                read_any = true;
                // a hostile file must not stop the index
                std::panic::catch_unwind(|| file_gps(&key)).ok().flatten()
            }
        };
        fresh.files.insert(key.clone(), (modified, bytes, pos.map(|p| [p.lat, p.lon])));
        batch.push((key, pos));
        if batch.len() >= 512 || last_send.elapsed().as_millis() > 250 {
            if tx.send(std::mem::take(&mut batch)).is_err() {
                return;
            }
            last_send = std::time::Instant::now();
        }
    }
    if !batch.is_empty() && tx.send(batch).is_err() {
        return;
    }
    if let Some(p) = path
        && (read_any || fresh.files.len() != old.files.len())
    {
        save_gps_cache(&p, &fresh);
    }
}

// ---------------------------------------------------------------------------------------------
// built-in list of world cities (GeoNames, places of 15,000 people or more)

static CITIES_DATA: &[u8] = include_bytes!("../../../assets/geo/cities.bin");

pub struct City {
    pub name: String,
    pub admin: String,
    country: u16,
    pub lat: f32,
    pub lon: f32,
    pub pop: u32,
    /// Folded (lowercase, no accents) name and other names, for search.
    keys: Vec<String>,
}

impl City {
    pub fn at(&self) -> LatLon {
        LatLon { lat: self.lat as f64, lon: self.lon as f64 }
    }

    /// How far from the centre a point still counts as "in" the city.
    fn radius_km(&self) -> f64 {
        2.0 + (self.pop as f64).sqrt() / 150.0
    }

    /// A map zoom that shows the whole city.
    pub fn zoom(&self) -> f64 {
        (13.0 - (self.pop as f64).max(1.0).log10() * 0.5).clamp(9.0, 12.0)
    }
}

pub struct Gazetteer {
    pub cities: Vec<City>,
    /// (code, name, folded name)
    countries: Vec<(String, String, String)>,
    /// City indexes sorted by latitude (for `nearest`).
    by_lat: Vec<u32>,
}

impl Gazetteer {
    fn parse(text: &str) -> Gazetteer {
        let mut countries: Vec<(String, String, String)> = vec![];
        let mut index: HashMap<String, u16> = HashMap::new();
        let mut cities = vec![];
        for line in text.lines() {
            if let Some(rest) = line.strip_prefix('=') {
                let mut f = rest.split('\t');
                if let (Some(code), Some(name)) = (f.next(), f.next())
                    && let Ok(i) = u16::try_from(countries.len())
                {
                    index.insert(code.to_string(), i);
                    countries.push((code.to_string(), name.to_string(), fold(name)));
                }
                continue;
            }
            // name, other names, region, country code, latitude, longitude, population
            let f: Vec<&str> = line.split('\t').collect();
            let [name, alts, admin, cc, lat, lon, pop] = f[..] else { continue };
            let (Ok(lat), Ok(lon)) = (lat.parse::<f32>(), lon.parse::<f32>()) else { continue };
            if LatLon::new(lat as f64, lon as f64).is_none() {
                continue;
            }
            let mut keys = vec![fold(name)];
            for a in alts.split(',').filter(|a| !a.is_empty()) {
                let k = fold(a);
                if !keys.contains(&k) {
                    keys.push(k);
                }
            }
            cities.push(City {
                name: name.to_string(),
                admin: admin.to_string(),
                country: index.get(cc).copied().unwrap_or(u16::MAX),
                lat,
                lon,
                pop: pop.parse().unwrap_or(0),
                keys,
            });
        }
        let mut by_lat: Vec<u32> = (0..cities.len() as u32).collect();
        by_lat.sort_by(|&a, &b| cities[a as usize].lat.total_cmp(&cities[b as usize].lat));
        Gazetteer { cities, countries, by_lat }
    }

    pub fn country(&self, c: &City) -> &str {
        self.countries.get(c.country as usize).map(|c| c.1.as_str()).unwrap_or("")
    }

    /// "Bordeaux, France"
    pub fn short_name(&self, c: &City) -> String {
        let country = self.country(c);
        if country.is_empty() { c.name.clone() } else { format!("{}, {country}", c.name) }
    }

    /// "Bordeaux, Nouvelle-Aquitaine, France"
    pub fn long_name(&self, c: &City) -> String {
        let mut parts = vec![c.name.as_str()];
        if !c.admin.is_empty() && c.admin != c.name {
            parts.push(&c.admin);
        }
        let country = self.country(c);
        if !country.is_empty() {
            parts.push(country);
        }
        parts.join(", ")
    }

    fn qualifier_matches(&self, c: &City, q: &str) -> bool {
        let Some((code, _, folded)) = self.countries.get(c.country as usize) else { return false };
        code.eq_ignore_ascii_case(q) || folded.starts_with(q) || fold(&c.admin).starts_with(q)
    }
}

/// The built-in city list, read on first use (a few tens of milliseconds).
pub fn gazetteer() -> &'static Gazetteer {
    static G: OnceLock<Gazetteer> = OnceLock::new();
    G.get_or_init(|| {
        let text = miniz_oxide::inflate::decompress_to_vec_with_limit(CITIES_DATA, 64 << 20)
            .ok()
            .and_then(|b| String::from_utf8(b).ok())
            .unwrap_or_default();
        Gazetteer::parse(&text)
    })
}

/// Lowercase, without accents, so that "Munchen", "münchen" and "MÜNCHEN" all match.
pub fn fold(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars().flat_map(char::to_lowercase) {
        let r = match c {
            'à' | 'á' | 'â' | 'ã' | 'ä' | 'å' | 'ā' | 'ă' | 'ą' => "a",
            'æ' => "ae",
            'ç' | 'ć' | 'č' | 'ĉ' | 'ċ' => "c",
            'ď' | 'đ' | 'ð' => "d",
            'è' | 'é' | 'ê' | 'ë' | 'ē' | 'ĕ' | 'ė' | 'ę' | 'ě' => "e",
            'ğ' | 'ĝ' | 'ġ' | 'ģ' => "g",
            'ĥ' | 'ħ' => "h",
            'ì' | 'í' | 'î' | 'ï' | 'ĩ' | 'ī' | 'ĭ' | 'į' | 'ı' => "i",
            'ĵ' => "j",
            'ķ' => "k",
            'ĺ' | 'ļ' | 'ľ' | 'ŀ' | 'ł' => "l",
            'ñ' | 'ń' | 'ņ' | 'ň' => "n",
            'ò' | 'ó' | 'ô' | 'õ' | 'ö' | 'ø' | 'ō' | 'ŏ' | 'ő' => "o",
            'œ' => "oe",
            'ŕ' | 'ŗ' | 'ř' => "r",
            'ś' | 'ŝ' | 'ş' | 'š' | 'ș' => "s",
            'ß' => "ss",
            'ţ' | 'ť' | 'ŧ' | 'ț' => "t",
            'þ' => "th",
            'ù' | 'ú' | 'û' | 'ü' | 'ũ' | 'ū' | 'ŭ' | 'ů' | 'ű' | 'ų' => "u",
            'ŵ' => "w",
            'ý' | 'ÿ' | 'ŷ' => "y",
            'ź' | 'ż' | 'ž' => "z",
            '\'' | '’' | '`' | 'ʻ' | 'ʼ' => "",
            '-' | '_' | '.' => " ",
            c => {
                out.push(c);
                continue;
            }
        };
        out.push_str(r);
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// A place found by a search.
#[derive(Clone, Debug, PartialEq)]
pub struct Found {
    /// "Bordeaux, Nouvelle-Aquitaine, France"
    pub name: String,
    /// The name stored with the pictures: "Bordeaux, France"
    pub short: String,
    pub at: LatLon,
    /// A map zoom that shows the place.
    pub zoom: f64,
}

/// Search the built-in city list. "paris" finds the French capital first (most people); "paris,
/// texas" or "paris us" narrows by region or country. Typed coordinates come back as one result.
pub fn search(query: &str, limit: usize) -> Vec<Found> {
    if let Some(at) = parse_coords(query) {
        return vec![Found { name: at.text(), short: String::new(), at, zoom: 14.0 }];
    }
    let mut parts = query.split(',').map(fold).filter(|p| !p.is_empty());
    let Some(mut q) = parts.next() else { return vec![] };
    let mut quals: Vec<String> = parts.collect();
    let g = gazetteer();
    // "paris us": a trailing word that names a country or region also narrows the search
    let run = |q: &str, quals: &[String]| -> Vec<(u8, u32, usize)> {
        let mut hits: Vec<(u8, u32, usize)> = vec![];
        for (i, c) in g.cities.iter().enumerate() {
            let score = c.keys.iter().enumerate().map(|(k, key)| match_score(key, q).saturating_sub(u8::from(k > 0))).max().unwrap_or(0);
            if score == 0 || !quals.iter().all(|w| g.qualifier_matches(c, w)) {
                continue;
            }
            hits.push((score, c.pop, i));
        }
        hits
    };
    let mut hits = run(&q, &quals);
    if hits.is_empty() && quals.is_empty()
        && let Some((head, last)) = q.rsplit_once(' ')
    {
        quals = vec![last.to_string()];
        q = head.to_string();
        hits = run(&q, &quals);
    }
    hits.sort_by(|a, b| b.0.cmp(&a.0).then(b.1.cmp(&a.1)));
    hits.into_iter()
        .take(limit)
        .filter_map(|(_, _, i)| g.cities.get(i))
        .map(|c| Found { name: g.long_name(c), short: g.short_name(c), at: c.at(), zoom: c.zoom() })
        .collect()
}

/// 4 = same name, 3 = name starts with the query, 2 = a word of it does, 1 = contains it (3+ letters).
fn match_score(key: &str, q: &str) -> u8 {
    if key == q {
        4
    } else if key.starts_with(q) {
        3
    } else if key.split(' ').any(|w| w.starts_with(q)) {
        2
    } else if q.len() >= 3 && key.contains(q) {
        1
    } else {
        0
    }
}

/// The city nearest to a position, with its distance in kilometres.
pub fn nearest(at: LatLon) -> Option<(&'static City, f64)> {
    let g = gazetteer();
    let coslat = at.lat.to_radians().cos().max(0.01);
    // a quick flat distance (in degrees) to pick the candidate, then the real one
    let flat = |c: &City| {
        let dx = ((c.lon as f64 - at.lon + 540.0) % 360.0 - 180.0) * coslat;
        let dy = c.lat as f64 - at.lat;
        (dx * dx + dy * dy).sqrt()
    };
    // look in a band of latitudes first: a city at flat distance d is at most d degrees of
    // latitude away, so the best one in a band at most `band` away is the best overall
    for band in [1.0, 4.0, 15.0] {
        let lo = g.by_lat.partition_point(|&i| (g.cities[i as usize].lat as f64) < at.lat - band);
        let hi = g.by_lat.partition_point(|&i| (g.cities[i as usize].lat as f64) <= at.lat + band);
        let best = g.by_lat.get(lo..hi)?.iter().map(|&i| &g.cities[i as usize]).min_by(|a, b| flat(a).total_cmp(&flat(b)));
        if let Some(c) = best
            && flat(c) <= band
        {
            return Some((c, at.km_to(c.at())));
        }
    }
    let best = g.cities.iter().min_by(|a, b| flat(a).total_cmp(&flat(b)))?;
    Some((best, at.km_to(best.at())))
}

// ---------------------------------------------------------------------------------------------
// map projection (Web Mercator, the one map tiles use): the world is the unit square, x to the
// east and y to the south

/// The projection can't show the poles; tiles stop at this latitude.
pub const MAX_LAT: f64 = 85.051_128_78;

pub fn to_world(at: LatLon) -> [f64; 2] {
    let lat = at.lat.clamp(-MAX_LAT, MAX_LAT).to_radians();
    let x = (at.lon + 180.0) / 360.0;
    let y = (1.0 - (lat.tan() + 1.0 / lat.cos()).ln() / std::f64::consts::PI) / 2.0;
    [x, y]
}

pub fn from_world(p: [f64; 2]) -> LatLon {
    let lon = p[0].clamp(0.0, 1.0) * 360.0 - 180.0;
    let n = std::f64::consts::PI * (1.0 - 2.0 * p[1].clamp(0.0, 1.0));
    let lat = n.sinh().atan().to_degrees();
    LatLon { lat, lon }
}

// ---------------------------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: LatLon, lat: f64, lon: f64) -> bool {
        (a.lat - lat).abs() < 1e-4 && (a.lon - lon).abs() < 1e-4
    }

    #[test]
    fn typed_coordinates() {
        let p = |s| parse_coords(s);
        assert!(close(p("44.8378, -0.5792").unwrap(), 44.8378, -0.5792));
        assert!(close(p("44.8378 -0.5792").unwrap(), 44.8378, -0.5792));
        assert!(close(p("44.8378 N 0.5792 W").unwrap(), 44.8378, -0.5792));
        assert!(close(p("0.5792W 44.8378N").unwrap(), 44.8378, -0.5792));
        assert!(close(p("N 44.8378, W 0.5792").unwrap(), 44.8378, -0.5792));
        assert!(close(p("44°50'16\"N 0°34'45\"W").unwrap(), 44.837_777, -0.579_166));
        assert!(close(p("33 52 S 151 12 E").unwrap(), -33.866_666, 151.2));
        assert!(close(p("48 51.4 2 21.1").unwrap(), 48.856_666, 2.351_666));
        for bad in ["Paris", "Paris 15", "", "91, 10", "10, 181", "1 2 3", "44,84, -0,58", "44 70 0 0", "NaN, 3", "--1, 2"] {
            assert_eq!(parse_coords(bad), None, "{bad}");
        }
    }

    #[test]
    fn projection_round_trip() {
        for (lat, lon) in [(0.0, 0.0), (44.84, -0.58), (-33.87, 151.21), (84.0, 179.9), (-84.0, -179.9)] {
            let back = from_world(to_world(LatLon { lat, lon }));
            assert!(close(back, lat, lon), "{lat} {lon} -> {back:?}");
        }
        let [x, y] = to_world(LatLon { lat: 0.0, lon: 0.0 });
        assert!((x - 0.5).abs() < 1e-12 && (y - 0.5).abs() < 1e-12);
    }

    #[test]
    fn distances() {
        let bordeaux = LatLon { lat: 44.8378, lon: -0.5792 };
        let paris = LatLon { lat: 48.8566, lon: 2.3522 };
        let km = bordeaux.km_to(paris);
        assert!((495.0..505.0).contains(&km), "{km}");
        assert_eq!(bordeaux.km_to(bordeaux), 0.0);
    }

    #[test]
    fn city_search() {
        assert!(gazetteer().cities.len() > 20_000);
        let first = |q: &str| search(q, 5).into_iter().next().map(|f| f.short).unwrap_or_default();
        assert_eq!(first("bordeaux"), "Bordeaux, France");
        assert_eq!(first("Paris"), "Paris, France");
        assert_eq!(first("münchen"), "Munich, Germany");
        assert_eq!(first("wien"), "Vienna, Austria");
        assert_eq!(first("paris, texas"), "Paris, United States");
        assert_eq!(first("paris us"), "Paris, United States");
        assert!(search("zzzzqqq", 5).is_empty());
        assert!(search("   ", 5).is_empty());
        let coords = search("44.84, -0.58", 5);
        assert_eq!(coords.len(), 1);
    }

    #[test]
    fn nearest_city_labels() {
        let (c, km) = nearest(LatLon { lat: 44.84, lon: -0.58 }).unwrap();
        assert_eq!(c.name, "Bordeaux");
        assert!(km < 5.0);
        assert_eq!(describe(None, LatLon { lat: 44.84, lon: -0.58 }), "Bordeaux, France");
        let named = Place::set(LatLon { lat: 1.0, lon: 2.0 }, " Grandma's garden ");
        assert_eq!(describe(Some(&named), LatLon { lat: 1.0, lon: 2.0 }), "Grandma's garden");
        // the middle of the Pacific: coordinates
        assert!(describe(None, LatLon { lat: -30.0, lon: -140.0 }).contains('°'));
    }

    #[test]
    fn effective_position() {
        let file = LatLon::new(10.0, 20.0);
        assert_eq!(effective(None, file), file);
        assert_eq!(effective(Some(&Place::Removed), file), None);
        let set = Place::set(LatLon { lat: 1.0, lon: 2.0 }, "");
        assert_eq!(effective(Some(&set), file), LatLon::new(1.0, 2.0));
        assert!(!Place::Set { lat: f64::NAN, lon: 0.0, name: String::new() }.is_valid());
        assert!(!Place::Set { lat: 0.0, lon: 200.0, name: String::new() }.is_valid());
    }

    #[test]
    fn gps_written_to_exif_reads_back() {
        let at = LatLon { lat: -33.856_784, lon: 151.215_297 };
        let img = image::RgbImage::from_pixel(8, 8, image::Rgb([90, 120, 200]));
        let mut jpeg = vec![];
        image::DynamicImage::ImageRgb8(img).write_to(&mut std::io::Cursor::new(&mut jpeg), image::ImageFormat::Jpeg).unwrap();
        let file = crate::exifinfo::jpeg_with_exif(&jpeg, &exif_gps_fields(at), None).unwrap();
        let exif = exif::Reader::new().read_from_container(&mut std::io::Cursor::new(&file)).unwrap();
        let back = gps_of(&exif).unwrap();
        assert!(back.km_to(at) < 0.001, "{back:?}");
        assert!(image::load_from_memory(&file).is_ok());
    }

    #[test]
    fn hostile_gps_is_ignored() {
        use exif::{Field, In, Rational, Tag, Value};
        let field = |tag, value| Field { tag, ifd_num: In::PRIMARY, value };
        let img = image::RgbImage::new(4, 4);
        let mut jpeg = vec![];
        image::DynamicImage::ImageRgb8(img).write_to(&mut std::io::Cursor::new(&mut jpeg), image::ImageFormat::Jpeg).unwrap();
        for lat in [
            Value::Rational(vec![Rational { num: 1, denom: 0 }]),
            Value::Rational(vec![]),
            Value::Rational(vec![Rational { num: 95, denom: 1 }]),
            Value::Ascii(vec![b"12".to_vec()]),
        ] {
            let fields = [field(Tag::GPSLatitude, lat), field(Tag::GPSLongitude, Value::Rational(vec![Rational { num: 3, denom: 1 }]))];
            let file = crate::exifinfo::jpeg_with_exif(&jpeg, &fields, None).unwrap();
            let exif = exif::Reader::new().read_from_container(&mut std::io::Cursor::new(&file)).unwrap();
            assert_eq!(gps_of(&exif), None);
        }
    }

    /// Builds `assets/geo/cities.bin` from GeoNames files (cities15000.txt, countryInfo.txt,
    /// admin1CodesASCII.txt) in the folder named by ALBUMCRAFT_GEO_SRC.
    #[test]
    #[ignore]
    fn build_city_data() {
        let src = std::path::PathBuf::from(std::env::var("ALBUMCRAFT_GEO_SRC").expect("set ALBUMCRAFT_GEO_SRC"));
        let read = |n: &str| std::fs::read_to_string(src.join(n)).unwrap();
        let mut out = String::new();
        let mut countries = HashMap::new();
        for line in read("countryInfo.txt").lines().filter(|l| !l.starts_with('#')) {
            let f: Vec<&str> = line.split('\t').collect();
            if f.len() > 4 {
                countries.insert(f[0].to_string(), f[4].to_string());
                out.push_str(&format!("={}\t{}\n", f[0], f[4]));
            }
        }
        let mut admin1 = HashMap::new();
        for line in read("admin1CodesASCII.txt").lines() {
            let f: Vec<&str> = line.split('\t').collect();
            if f.len() > 1 {
                admin1.insert(f[0].to_string(), f[1].to_string());
            }
        }
        let latin = |s: &str| s.chars().all(|c| (c.is_alphabetic() && (c as u32) < 0x250) || " -'.".contains(c));
        let mut n = 0;
        for line in read("cities15000.txt").lines() {
            let f: Vec<&str> = line.split('\t').collect();
            if f.len() < 15 {
                continue;
            }
            let (name, ascii, alts, lat, lon, cc, a1, pop) = (f[1], f[2], f[3], f[4], f[5], f[8], f[10], f[14]);
            let pop: u32 = pop.parse().unwrap_or(0);
            let mut keep: Vec<String> = vec![];
            let mut seen = vec![fold(name)];
            let mut consider = |a: &str, keep: &mut Vec<String>| {
                let k = fold(a);
                if a.len() >= 2 && a.len() <= 40 && latin(a) && !seen.contains(&k) {
                    seen.push(k);
                    keep.push(a.to_string());
                }
            };
            consider(ascii, &mut keep);
            // other names (Wien, München…) for the larger places only, to keep the list small
            // (GeoNames lists them alphabetically, so a cap per city has to be generous)
            let cap = match pop {
                1_000_000.. => 40,
                250_000.. => 16,
                100_000.. => 8,
                _ => 0,
            };
            for a in alts.split(',').filter(|a| a.len() <= 24) {
                if keep.len() >= cap {
                    break;
                }
                consider(a, &mut keep);
            }
            let admin = admin1.get(&format!("{cc}.{a1}")).cloned().unwrap_or_default();
            let (lat, lon): (f64, f64) = (lat.parse().unwrap(), lon.parse().unwrap());
            out.push_str(&format!("{name}\t{}\t{admin}\t{cc}\t{lat:.4}\t{lon:.4}\t{pop}\n", keep.join(",")));
            n += 1;
        }
        let packed = miniz_oxide::deflate::compress_to_vec(out.as_bytes(), 10);
        let dest = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/geo/cities.bin");
        std::fs::create_dir_all(dest.parent().unwrap()).unwrap();
        std::fs::write(&dest, &packed).unwrap();
        eprintln!("{n} cities, {} countries: {} bytes -> {} bytes", countries.len(), out.len(), packed.len());
    }
}

#[cfg(test)]
mod nearest_tests {
    use super::*;

    /// The quick lookup finds the same city as looking at every one, all over the world.
    #[test]
    fn nearest_matches_a_full_scan() {
        let g = gazetteer();
        assert!(!g.cities.is_empty());
        let mut seed = 12345u64;
        let mut rnd = || {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            (seed >> 33) as f64 / (1u64 << 31) as f64
        };
        for _ in 0..400 {
            let at = LatLon { lat: rnd() * 160.0 - 80.0, lon: rnd() * 360.0 - 180.0 };
            let coslat = at.lat.to_radians().cos().max(0.01);
            let flat = |c: &City| {
                let dx = ((c.lon as f64 - at.lon + 540.0) % 360.0 - 180.0) * coslat;
                let dy = c.lat as f64 - at.lat;
                dx * dx + dy * dy
            };
            let full = g.cities.iter().min_by(|a, b| flat(a).total_cmp(&flat(b))).unwrap();
            let (quick, _) = nearest(at).unwrap();
            assert!((flat(quick) - flat(full)).abs() < 1e-9, "{at:?}: {} vs {}", quick.name, full.name);
        }
    }

    #[test]
    fn place_terms_name_city_region_country() {
        let bordeaux = LatLon { lat: 44.8378, lon: -0.5792 };
        let terms = place_terms(None, bordeaux);
        assert!(terms.iter().any(|t| t == "Bordeaux"), "{terms:?}");
        assert!(terms.iter().any(|t| t == "France"), "{terms:?}");
        let set = Place::Set { lat: bordeaux.lat, lon: bordeaux.lon, name: "Grandma's house".into() };
        assert_eq!(place_terms(Some(&set), bordeaux).first().map(String::as_str), Some("Grandma's house"));
        // the middle of the Pacific: no city near
        assert!(place_terms(None, LatLon { lat: -30.0, lon: -140.0 }).is_empty());
    }
}
