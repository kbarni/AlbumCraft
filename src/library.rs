//! The catalogue: scanned folders, per-photo notes (stars, captions, tags, edits) and albums.

use crate::imaging::{self, EditOp, SAMPLE_PREFIX};
use image::RgbaImage;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Condvar, Mutex};

pub const EXTENSIONS: [&str; 9] = ["jpg", "jpeg", "png", "gif", "bmp", "webp", "tif", "tiff", "jpe"];

#[derive(Serialize, Deserialize, Default, Clone, Debug)]
#[serde(default)]
pub struct Meta {
    pub star: bool,
    pub caption: String,
    pub tags: Vec<String>,
    pub people: Vec<String>,
    pub edits: Vec<EditOp>,
}

impl Meta {
    pub fn is_empty(&self) -> bool {
        !self.star && self.caption.is_empty() && self.tags.is_empty() && self.people.is_empty() && self.edits.is_empty()
    }
}

#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct Album {
    pub name: String,
    pub keys: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct Store {
    pub roots: Vec<String>,
    pub meta: HashMap<String, Meta>,
    pub albums: Vec<Album>,
    pub folder_notes: HashMap<String, String>,
    pub thumb_size: f32,
    /// Folders whose .picasa.ini was already imported once.
    pub imported_ini: Vec<String>,
    /// "Don't ask again, always use the selected image" when leaving the two-version view.
    pub dont_ask_end_2up: bool,
    /// Last used print layout, fit, border, text, copies and printer.
    pub print: crate::print::PrintSettings,
    /// Appearance: "system", "light" or "dark".
    pub theme: String,
}

impl Default for Store {
    fn default() -> Self {
        Self {
            roots: vec![],
            meta: HashMap::new(),
            albums: vec![],
            folder_notes: HashMap::new(),
            thumb_size: 120.0,
            imported_ini: vec![],
            dont_ask_end_2up: false,
            print: Default::default(),
            theme: "system".into(),
        }
    }
}

// ---------------------------------------------------------------------------
// Per-folder .picasa.ini settings files (imported once, never written):
//   [Picasa]  name= description= location= category= date=
//   [.album:<token>]  name= token= date= ...
//   [<file name>]  star=yes  caption=  keywords=a,b  rotate=rotate(n)  crop=rect64(hex)
//                  filters=name=1,args;name=1,args;  albums=token1,token2

#[derive(Default, Debug)]
pub struct IniFolder {
    pub name: Option<String>,
    pub description: Option<String>,
    /// lower-case file name -> notes for that picture
    pub photos: HashMap<String, Meta>,
    /// album token -> album name
    pub albums: Vec<(String, String)>,
    /// (lower-case file name, album token)
    pub memberships: Vec<(String, String)>,
}

/// rect64(hex): four 16-bit numbers (left, top, right, bottom) as fractions of 65535.
fn parse_rect64(hex: &str) -> Option<[f32; 4]> {
    let v = u64::from_str_radix(hex.trim(), 16).ok()?;
    let part = |shift: u32| ((v >> shift) & 0xffff) as f32 / 65535.0;
    let r = [part(48), part(32), part(16), part(0)];
    (r[2] > r[0] && r[3] > r[1]).then_some(r)
}

fn parse_filters(list: &str, edits: &mut Vec<EditOp>) {
    use crate::imaging::Effect as E;
    for entry in list.split(';') {
        let Some((name, args)) = entry.trim().split_once('=') else { continue };
        let args: Vec<&str> = args.split(',').map(|s| s.trim()).collect();
        if args.first() == Some(&"0") {
            continue; // switched off
        }
        let arg = |i: usize| args.get(i).and_then(|s| s.parse::<f32>().ok());
        let op = match name.trim() {
            "crop64" => args.get(1).and_then(|h| parse_rect64(h)).map(EditOp::Crop),
            "fill" => Some(EditOp::Tune { fill: arg(1).unwrap_or(0.0).clamp(0.0, 1.0), highlights: 0.0, shadows: 0.0, temp: 0.0 }),
            "autolight" | "autocontrast" => Some(EditOp::AutoContrast),
            "autocolor" | "colorfix" => Some(EditOp::AutoColor),
            "enhance" => Some(EditOp::Lucky),
            "sepia" => Some(EditOp::Effect(E::Sepia)),
            "bw" => Some(EditOp::Effect(E::BlackWhite)),
            "warm" => Some(EditOp::Effect(E::Warmify)),
            "grain" | "grain2" => Some(EditOp::Effect(E::FilmGrain)),
            "tint" => Some(EditOp::Effect(E::Tint)),
            "sat" => Some(EditOp::Effect(E::Saturation)),
            "radblur" => Some(EditOp::Effect(E::SoftFocus)),
            "glow" | "glow2" => Some(EditOp::Effect(E::Glow)),
            "ansel" => Some(EditOp::Effect(E::FilteredBw)),
            "radtint" => Some(EditOp::Effect(E::FocalBw)),
            "dir_tint" => Some(EditOp::Effect(E::GraduatedTint)),
            "unsharp" | "unsharp2" => Some(EditOp::Effect(E::Sharpen)),
            _ => None, // tilt, redeye, finetune, ... : not imported yet
        };
        if let Some(op) = op {
            edits.push(op);
        }
    }
}

pub fn read_picasa_ini(dir: &Path) -> Option<IniFolder> {
    let text = [".picasa.ini", "Picasa.ini"].iter().find_map(|n| std::fs::read(dir.join(n)).ok())?;
    let text = String::from_utf8_lossy(&text);
    let mut out = IniFolder::default();
    let mut section = String::new();
    let mut rotate: HashMap<String, i32> = HashMap::new();
    for line in text.lines() {
        let line = line.trim();
        if let Some(s) = line.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
            section = s.to_string();
            continue;
        }
        let Some((k, v)) = line.split_once('=') else { continue };
        let (k, v) = (k.trim(), v.trim());
        if section.eq_ignore_ascii_case("Picasa") {
            match k {
                "name" if !v.is_empty() => out.name = Some(v.to_string()),
                "description" if !v.is_empty() => out.description = Some(v.to_string()),
                _ => {}
            }
        } else if let Some(token) = section.strip_prefix(".album:") {
            if k == "name" {
                out.albums.push((token.to_string(), v.to_string()));
            }
        } else if !section.is_empty() && section != "encoding" {
            let file = section.to_lowercase();
            let m = out.photos.entry(file.clone()).or_default();
            match k {
                "star" => m.star = v.eq_ignore_ascii_case("yes"),
                "caption" => m.caption = v.to_string(),
                "keywords" => m.tags = v.split(',').map(|t| t.trim().to_string()).filter(|t| !t.is_empty()).collect(),
                "rotate" => {
                    if let Some(n) = v.strip_prefix("rotate(").and_then(|s| s.strip_suffix(')')).and_then(|s| s.parse::<i32>().ok()) {
                        rotate.insert(file, n);
                    }
                }
                "crop" => {
                    if let Some(r) = v.strip_prefix("rect64(").and_then(|s| s.strip_suffix(')')).and_then(parse_rect64) {
                        m.edits.insert(0, EditOp::Crop(r));
                    }
                }
                "filters" => parse_filters(v, &mut m.edits),
                "albums" => {
                    for t in v.split(',').map(|t| t.trim()).filter(|t| !t.is_empty()) {
                        out.memberships.push((file.clone(), t.to_string()));
                    }
                }
                _ => {}
            }
        }
    }
    // rotate(n) is a quarter-turn count, applied after the filters.
    for (file, n) in rotate {
        if n.rem_euclid(4) != 0 {
            out.photos.entry(file).or_default().edits.push(EditOp::Rotate(n));
        }
    }
    Some(out)
}

pub fn store_path() -> Option<PathBuf> {
    Some(dirs::data_dir()?.join("AlbumCraft").join("library.json"))
}

/// The library saved under the app's earlier working name, PhotoShelf.
fn old_store_path() -> Option<PathBuf> {
    Some(dirs::data_dir()?.join("PhotoShelf").join("library.json"))
}

/// Copy (not move: the old file stays as a backup) a library saved under the earlier name.
fn migrate_old_store(path: &Path) {
    let Some(old) = old_store_path() else { return };
    if path.exists() || !old.is_file() {
        return;
    }
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let _ = std::fs::copy(&old, path);
}

impl Store {
    pub fn load() -> Option<Store> {
        let path = store_path()?;
        migrate_old_store(&path);
        let text = std::fs::read_to_string(&path).ok()?;
        serde_json::from_str(&text).ok()
    }

    pub fn save(&mut self) {
        self.meta.retain(|_, m| !m.is_empty());
        if let Some(p) = store_path() {
            if let Some(dir) = p.parent() {
                let _ = std::fs::create_dir_all(dir);
            }
            if let Ok(text) = serde_json::to_string_pretty(self) {
                let _ = std::fs::write(p, text);
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct Photo {
    pub key: String,
    pub name: String,
    pub folder: usize,
    pub modified: i64,
    pub bytes: u64,
}

impl Photo {
    pub fn is_sample(&self) -> bool {
        self.key.starts_with(SAMPLE_PREFIX)
    }
}

#[derive(Clone, Debug)]
pub struct Folder {
    pub path: String,
    pub name: String,
    pub date: i64,
    pub photos: Vec<usize>,
}

/// One scanned folder: its path and (file path, modified, size) for each picture.
pub type ScannedFolder = (String, Vec<(String, i64, u64)>);

pub fn scan_roots(roots: &[String], limit: usize) -> Vec<ScannedFolder> {
    let mut out = vec![];
    let mut count = 0;
    for r in roots {
        scan_dir(Path::new(r), 0, &mut out, &mut count, limit);
    }
    out
}

fn scan_dir(dir: &Path, depth: usize, out: &mut Vec<ScannedFolder>, count: &mut usize, limit: usize) {
    if depth > 12 || *count >= limit {
        return;
    }
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    let mut files = vec![];
    let mut subdirs = vec![];
    for entry in rd.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') || name.starts_with('$') {
            continue;
        }
        let Ok(md) = entry.metadata() else { continue };
        if md.is_dir() {
            subdirs.push(path);
        } else if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            if EXTENSIONS.contains(&ext.to_ascii_lowercase().as_str()) && md.len() > 0 {
                let modified = md
                    .modified()
                    .ok()
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|d| d.as_secs() as i64)
                    .unwrap_or(0);
                files.push((path.to_string_lossy().to_string(), modified, md.len()));
            }
        }
    }
    if !files.is_empty() {
        files.sort_by(|a, b| a.0.to_lowercase().cmp(&b.0.to_lowercase()));
        *count += files.len();
        out.push((dir.to_string_lossy().to_string(), files));
    }
    subdirs.sort();
    for s in subdirs {
        scan_dir(&s, depth + 1, out, count, limit);
    }
}

/// Built-in sample folders shown when the library has no pictures of its own.
pub fn sample_folders() -> Vec<ScannedFolder> {
    let groups: [(&str, i64, u32); 4] = [
        ("Summer by the sea", 1_247_580_000, 9),
        ("Mountain walk", 1_224_800_000, 7),
        ("Garden in spring", 1_210_500_000, 8),
        ("Winter lights", 1_197_100_000, 6),
    ];
    let mut seed = 1;
    groups
        .iter()
        .map(|(name, date, n)| {
            let files = (0..*n)
                .map(|i| {
                    let s = seed;
                    seed += 1;
                    (format!("{SAMPLE_PREFIX}{s}"), date + i as i64 * 1800, 180_000 + (s as u64 * 7919) % 90_000)
                })
                .collect();
            (format!("{SAMPLE_PREFIX}folder/{name}"), files)
        })
        .collect()
}

pub fn sample_file_name(key: &str) -> String {
    let n = key.trim_start_matches(SAMPLE_PREFIX);
    format!("IMG_{:04}.JPG", 2000 + n.parse::<u32>().unwrap_or(0) * 7)
}

#[cfg(test)]
mod ini_tests {
    use super::*;

    #[test]
    fn reads_picasa_ini() {
        let dir = std::env::temp_dir().join(format!("albumcraft-ini-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join(".picasa.ini"),
            "[encoding]\r\nutf8=1\r\n[Picasa]\r\nname=Trip\r\ndescription=Beach week\r\n\
             [.album:abc123]\r\nname=Best\r\ntoken=abc123\r\n\
             [IMG_1.JPG]\r\nstar=yes\r\ncaption=Sunset\r\nkeywords=sea,sky\r\nrotate=rotate(1)\r\n\
             filters=crop64=1,1000200030004000;sepia=1;fill=1,0.500000;\r\nalbums=abc123\r\n",
        )
        .unwrap();
        let ini = read_picasa_ini(&dir).unwrap();
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(ini.description.as_deref(), Some("Beach week"));
        let m = &ini.photos["img_1.jpg"];
        assert!(m.star);
        assert_eq!(m.caption, "Sunset");
        assert_eq!(m.tags, vec!["sea", "sky"]);
        assert!(matches!(m.edits[0], EditOp::Crop(_)));
        assert!(matches!(m.edits[1], EditOp::Effect(crate::imaging::Effect::Sepia)));
        assert!(matches!(m.edits[2], EditOp::Tune { fill, .. } if (fill - 0.5).abs() < 1e-6));
        assert!(matches!(m.edits[3], EditOp::Rotate(1)));
        assert_eq!(ini.albums, vec![("abc123".to_string(), "Best".to_string())]);
        assert_eq!(ini.memberships, vec![("img_1.jpg".to_string(), "abc123".to_string())]);
    }
}

// ---------------------------------------------------------------------------
// Dates

pub fn civil(ts: i64) -> (i32, u32, u32) {
    let z = ts.div_euclid(86_400) + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    ((if m <= 2 { y + 1 } else { y }) as i32, m, d)
}

pub fn fmt_date(ts: i64) -> String {
    const M: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
    let (y, m, d) = civil(ts);
    format!("{} {}, {}", M[(m as usize).saturating_sub(1).min(11)], d, y)
}

pub fn fmt_time(ts: i64) -> String {
    let s = ts.rem_euclid(86_400);
    format!("{:02}:{:02}", s / 3600, (s % 3600) / 60)
}

pub fn fmt_bytes(b: u64) -> String {
    if b >= 1 << 30 {
        format!("{:.1} GB", b as f64 / (1u64 << 30) as f64)
    } else if b >= 1 << 20 {
        format!("{:.1} MB", b as f64 / (1u64 << 20) as f64)
    } else {
        format!("{} KB", (b / 1024).max(1))
    }
}

// ---------------------------------------------------------------------------
// Background loader

pub enum Job {
    Thumb { id: usize, key: String },
    Big { id: usize, key: String },
    /// A sharp render for zooming in: the picture at up to ZOOM_DIM pixels with its edits applied.
    Zoom { id: usize, key: String, ops: Vec<EditOp> },
}

pub enum Done {
    Thumb { id: usize, key: String, img: Option<RgbaImage> },
    Big { id: usize, key: String, img: Option<RgbaImage> },
    /// `unedited_width` is the width of the loaded picture before edits (to know its scale).
    Zoom { id: usize, key: String, ops: Vec<EditOp>, img: Option<RgbaImage>, unedited_width: u32 },
    Scan(Vec<ScannedFolder>),
    Message(String),
}

pub const THUMB_DIM: u32 = 256;
pub const BIG_DIM: u32 = 1400;
/// Longest side of the sharp render used when zooming in (keeps memory and GPU textures in bounds).
pub const ZOOM_DIM: u32 = 5000;

/// The photo's real size in pixels (read from the file header, or the sample's size).
pub fn source_dims(key: &str) -> Option<(u32, u32)> {
    if let Some(seed) = key.strip_prefix(imaging::SAMPLE_PREFIX) {
        return seed.parse().ok().map(imaging::sample_size);
    }
    image::image_dimensions(key).ok()
}

#[derive(Clone)]
pub struct Pool {
    queue: Arc<(Mutex<VecDeque<Job>>, Condvar)>,
}

impl Pool {
    pub fn new(threads: usize, tx: Sender<Done>, ctx: eframe::egui::Context) -> Self {
        let queue: Arc<(Mutex<VecDeque<Job>>, Condvar)> = Arc::new((Mutex::new(VecDeque::new()), Condvar::new()));
        for _ in 0..threads {
            let q = queue.clone();
            let tx = tx.clone();
            let ctx = ctx.clone();
            std::thread::spawn(move || loop {
                let job = {
                    let (lock, cv) = &*q;
                    let mut g = lock.lock().unwrap();
                    loop {
                        if let Some(j) = g.pop_front() {
                            break j;
                        }
                        g = cv.wait(g).unwrap();
                    }
                };
                let done = match job {
                    Job::Thumb { id, key } => {
                        let img = crate::thumbcache::get(&key, THUMB_DIM).or_else(|| {
                            let img = imaging::load_source(&key, THUMB_DIM)?;
                            crate::thumbcache::put(&key, THUMB_DIM, &img);
                            Some(img)
                        });
                        Done::Thumb { id, key, img }
                    }
                    Job::Big { id, key } => {
                        let img = imaging::load_source(&key, BIG_DIM);
                        Done::Big { id, key, img }
                    }
                    Job::Zoom { id, key, ops } => {
                        let src = imaging::load_source(&key, ZOOM_DIM);
                        let unedited_width = src.as_ref().map(|s| s.width()).unwrap_or(0);
                        let img = src.map(|s| imaging::apply_all(&s, &ops));
                        Done::Zoom { id, key, ops, img, unedited_width }
                    }
                };
                if tx.send(done).is_err() {
                    return;
                }
                ctx.request_repaint();
            });
        }
        Self { queue }
    }

    pub fn push(&self, job: Job, urgent: bool) {
        let (lock, cv) = &*self.queue;
        let mut g = lock.lock().unwrap();
        if urgent {
            g.push_front(job);
        } else {
            g.push_back(job);
        }
        cv.notify_one();
    }
}
