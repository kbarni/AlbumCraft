//! The catalogue: scanned folders, per-photo notes (pick/reject, rating, captions, tags, edits) and albums.

use crate::imaging::{self, EditOp, SAMPLE_PREFIX};
use image::RgbaImage;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;
use std::panic::AssertUnwindSafe;
use std::sync::{Arc, Condvar, Mutex, PoisonError};

/// `ALBUMCRAFT_TRACE=1`: print how long the slow steps take (to standard error).
pub fn tracing() -> bool {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(|| std::env::var_os("ALBUMCRAFT_TRACE").is_some())
}

/// With `ALBUMCRAFT_TRACE`, print `what` and the time since `start` when it is over `min_ms`.
pub fn trace_since(what: &str, start: std::time::Instant, min_ms: u128) {
    let ms = start.elapsed().as_millis();
    if tracing() && ms >= min_ms {
        eprintln!("[trace] {what}: {ms} ms");
    }
}

pub const EXTENSIONS: [&str; 9] = ["jpg", "jpeg", "png", "gif", "bmp", "webp", "tif", "tiff", "jpe"];

/// Whether a file with this (lower-case) extension is shown in the library.
pub fn is_picture_extension(ext: &str) -> bool {
    EXTENSIONS.contains(&ext) || crate::raw::RAW_EXTENSIONS.contains(&ext)
}

/// Culling flag: keep it (pick), drop it (reject), or not decided.
#[derive(Serialize, Deserialize, Default, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "lowercase")]
pub enum Flag {
    #[default]
    None,
    Pick,
    Reject,
}

impl Flag {
    pub fn is_none(&self) -> bool {
        *self == Flag::None
    }
}

#[derive(Serialize, Deserialize, Default, Clone, Debug)]
#[serde(default)]
pub struct Meta {
    /// Earlier libraries' yes/no star, read once and turned into a pick (see `Store::sanitize`).
    #[serde(skip_serializing)]
    pub star: bool,
    #[serde(skip_serializing_if = "Flag::is_none")]
    pub flag: Flag,
    /// 0 = not rated, 1–5 stars.
    #[serde(skip_serializing_if = "is_zero")]
    pub rating: u8,
    pub caption: String,
    pub tags: Vec<String>,
    /// Proposed tags the user said this picture does not show (tag proposals leave them out
    /// here and lower them for pictures like it).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub not_tags: Vec<String>,
    pub people: Vec<String>,
    pub edits: Vec<EditOp>,
    /// Where the picture was taken, when the user set or removed it (else the file's GPS is used).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub place: Option<crate::geo::Place>,
}

fn is_zero(v: &u8) -> bool {
    *v == 0
}

/// Highest rating.
pub const MAX_RATING: u8 = 5;

impl Meta {
    pub fn is_empty(&self) -> bool {
        !self.star
            && self.flag == Flag::None
            && self.rating == 0
            && self.caption.is_empty()
            && self.tags.is_empty()
            && self.not_tags.is_empty()
            && self.people.is_empty()
            && self.edits.is_empty()
            && self.place.is_none()
    }

    /// Turn an old yes/no star into a pick, keep the rating in range and drop a damaged place.
    fn upgrade(&mut self) {
        if std::mem::take(&mut self.star) && self.flag == Flag::None {
            self.flag = Flag::Pick;
        }
        self.rating = self.rating.min(MAX_RATING);
        if self.place.as_ref().is_some_and(|p| !p.is_valid()) {
            self.place = None;
        }
    }
}

#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct Album {
    pub name: String,
    pub keys: Vec<String>,
}

/// The version of `library.json`'s format this program writes. A library saved by a newer
/// version (a higher number) is never saved over: fields this version does not know would be
/// lost.
pub const FORMAT_VERSION: u32 = 1;

/// The order of the pictures of a folder, album, person or Picks.
#[derive(Serialize, Deserialize, Default, Clone, Copy, PartialEq, Eq, Debug, Hash)]
#[serde(rename_all = "lowercase")]
pub enum Sort {
    /// By file name (folders); as arranged (albums).
    #[default]
    Name,
    /// By the date taken (EXIF), oldest first; the file's date when there is none.
    Taken,
    /// Highest rating first, then by name.
    Rating,
}

/// The bar at the bottom of the library.
#[derive(Serialize, Deserialize, Default, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "lowercase")]
pub enum BarStyle {
    /// One line, shown while something is selected; the buttons lose their names when the
    /// window is narrow.
    #[default]
    Compact,
    /// Always shown: thumbnails of the selection and large buttons.
    Tray,
}

/// A cache on disk that the settings can empty.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Cache {
    Thumbs,
    Tiles,
}

/// The bytes of the files in `dir` and its subfolders (0 when it does not exist).
pub fn dir_bytes(dir: &Path) -> u64 {
    let mut total = 0;
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&d) else { continue };
        for e in rd.flatten() {
            let Ok(m) = e.metadata() else { continue };
            if m.is_dir() {
                stack.push(e.path());
            } else {
                total += m.len();
            }
        }
    }
    total
}

/// The bytes used by the thumbnail cache and the map tile cache.
pub fn cache_sizes() -> (u64, u64) {
    let of = |d: Option<PathBuf>| d.map(|d| dir_bytes(&d)).unwrap_or(0);
    (of(crate::thumbcache::cache_dir()), of(crate::online::tile_cache_dir()))
}

/// Remove every file of a cache (they are made or downloaded again when needed).
pub fn clear_cache(which: Cache) -> Result<(), String> {
    let dir = match which {
        Cache::Thumbs => crate::thumbcache::cache_dir(),
        Cache::Tiles => crate::online::tile_cache_dir(),
    };
    match dir {
        Some(d) if d.exists() => std::fs::remove_dir_all(&d).map_err(|e| format!("{}: {e}", d.display())),
        _ => Ok(()),
    }
}

/// A value from `library.json`, or the default when it holds one this version does not know
/// (a newer version's setting must not make the whole library unreadable).
fn or_default<'de, D: serde::Deserializer<'de>, T: serde::de::DeserializeOwned + Default>(d: D) -> Result<T, D::Error> {
    let v = serde_json::Value::deserialize(d)?;
    Ok(serde_json::from_value(v).unwrap_or_default())
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct Store {
    /// The format version it was saved in (`FORMAT_VERSION`; libraries saved before it was
    /// recorded read as this one).
    pub version: u32,
    pub roots: Vec<String>,
    /// Folders inside the roots that are left out (with everything under them).
    pub excluded: Vec<String>,
    /// Notes by picture (file path). Notes on the built-in sample pictures are not saved.
    #[serde(serialize_with = "without_samples")]
    pub meta: HashMap<String, Meta>,
    pub albums: Vec<Album>,
    /// A folder's description (by folder path), shown under its name in the picture grid.
    pub folder_notes: HashMap<String, String>,
    /// A folder's caption (by folder path): one short line under its name in the sidebar and
    /// beside it in the picture grid.
    pub folder_captions: HashMap<String, String>,
    pub thumb_size: f32,
    /// Folders whose .picasa.ini was already imported once.
    pub imported_ini: Vec<String>,
    /// No longer used (a question of the two-version view, removed); kept so that older
    /// libraries load.
    pub dont_ask_end_2up: bool,
    /// Last used print layout, fit, border, text, copies and printer.
    pub print: crate::print::PrintSettings,
    /// Appearance: "system", "light" or "dark".
    pub theme: String,
    /// The interface's language, as a code ("en", "fr"…); empty = the system's.
    pub language: String,
    /// Slideshow (and movie) settings.
    pub slideshow: crate::slides::SlideSettings,
    /// Online maps (OpenStreetMap tiles and place search): None until the user has been asked.
    pub online_maps: Option<bool>,
    /// Exported copies get the picture's location in their EXIF.
    pub export_location: bool,
    /// Exported copies keep the original's camera information (EXIF: camera, lens, exposure,
    /// date taken), without its GPS position, which follows `export_location`.
    pub export_exif: bool,
    /// Written into exported and emailed copies (EXIF Artist and Copyright) when not empty.
    pub export_creator: String,
    pub export_copyright: String,
    /// The signature or logo laid on exported pictures.
    #[serde(deserialize_with = "or_default")]
    pub export_signature: crate::export::Signature,
    /// The folder the last export went to (offered first next time).
    pub export_to: Option<String>,
    /// The folder the last import copied into a new folder of (offered first next time).
    pub import_to: Option<String>,
    /// The editor's info panel is shown.
    pub info_open: bool,
    /// Sections of the info panel opened (true) or closed (false) by the user; others use their
    /// default.
    pub info_sections: HashMap<String, bool>,
    /// Last used collage style, page and background.
    pub collage: crate::collage::CollageSettings,
    /// Last used contact sheet settings.
    pub contact_sheet: crate::collage::SheetSettings,
    /// The order of the pictures in the library.
    #[serde(deserialize_with = "or_default")]
    pub sort: Sort,
    /// The bar at the bottom of the library: compact or the tray.
    #[serde(deserialize_with = "or_default")]
    pub bar_style: BarStyle,
    /// The editor's Edit tab opens on its sliders (Advanced) rather than the one-click fixes.
    pub edit_advanced: bool,
    /// The Export dialog's starting size (longest side in pixels, 0 = original) and JPEG quality.
    pub export_size: u32,
    pub export_quality: u8,
    /// The user paused the background analysis of the library (AI features).
    pub analysis_paused: bool,
    /// The search pack the user chose (a pack id); empty: the one for the interface's language.
    pub search_pack: String,
    /// Last used web gallery theme, layout, picture size and captions.
    #[serde(deserialize_with = "or_default")]
    pub gallery: crate::gallery::GallerySettings,
}

fn without_samples<S: serde::Serializer>(meta: &HashMap<String, Meta>, s: S) -> Result<S::Ok, S::Error> {
    s.collect_map(meta.iter().filter(|(key, _)| !key.starts_with(SAMPLE_PREFIX)))
}

impl Store {
    /// The export settings as `export::Options` at this size and quality (the signature only
    /// when it is switched on).
    pub fn export_options(&self, max_dim: u32, quality: u8) -> crate::export::Options {
        crate::export::Options {
            max_dim,
            quality,
            camera_info: self.export_exif,
            creator: self.export_creator.trim().to_string(),
            copyright: self.export_copyright.trim().to_string(),
            signature: self.export_signature.shows().then(|| self.export_signature.clone()),
        }
    }
}

impl Default for Store {
    fn default() -> Self {
        Self {
            version: FORMAT_VERSION,
            roots: vec![],
            excluded: vec![],
            meta: HashMap::new(),
            albums: vec![],
            folder_notes: HashMap::new(),
            folder_captions: HashMap::new(),
            thumb_size: 120.0,
            imported_ini: vec![],
            dont_ask_end_2up: false,
            print: Default::default(),
            theme: "system".into(),
            language: String::new(),
            slideshow: Default::default(),
            online_maps: None,
            export_location: true,
            export_exif: true,
            export_creator: String::new(),
            export_copyright: String::new(),
            export_signature: Default::default(),
            export_to: None,
            import_to: None,
            info_open: true,
            info_sections: HashMap::new(),
            collage: Default::default(),
            contact_sheet: Default::default(),
            sort: Sort::Name,
            bar_style: BarStyle::Compact,
            edit_advanced: false,
            export_size: 0,
            export_quality: 90,
            analysis_paused: false,
            search_pack: String::new(),
            gallery: Default::default(),
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
                // a star there means "I like this one": a pick
                "star" if v.eq_ignore_ascii_case("yes") => m.flag = Flag::Pick,
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

/// What was found where the library is saved.
#[derive(Debug)]
pub enum Loaded {
    /// No library yet (first start).
    Fresh,
    Store(Box<Store>),
    /// The file exists but can't be used. `backup` is where it was moved so that a new library
    /// can be saved without losing it; `None` means it is still in place and must not be overwritten.
    Unreadable { error: String, backup: Option<PathBuf> },
}

impl Store {
    pub fn load() -> Loaded {
        let Some(path) = store_path() else { return Loaded::Fresh };
        migrate_old_store(&path);
        Self::load_from(&path)
    }

    pub fn load_from(path: &Path) -> Loaded {
        let text = match std::fs::read_to_string(path) {
            Ok(t) => t,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Loaded::Fresh,
            Err(e) => return Loaded::Unreadable { error: e.to_string(), backup: None },
        };
        // a newer version's library: keep it as it is (this version would drop what it added)
        #[derive(Deserialize)]
        struct Version {
            #[serde(default)]
            version: u32,
        }
        if let Ok(Version { version }) = serde_json::from_str::<Version>(&text)
            && version > FORMAT_VERSION
        {
            return Loaded::Unreadable {
                error: format!("it was saved by a newer version of AlbumCraft (format {version}; this version knows up to {FORMAT_VERSION}). Update AlbumCraft to use it"),
                backup: None,
            };
        }
        match serde_json::from_str::<Store>(&text) {
            Ok(mut store) => {
                store.sanitize();
                Loaded::Store(Box::new(store))
            }
            Err(e) => {
                // Keep the unreadable file next to where it was, under a name that is never overwritten.
                let stamp = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0);
                let backup = path.with_file_name(format!("library.unreadable-{stamp}.json"));
                let moved = !backup.exists() && std::fs::rename(path, &backup).is_ok();
                Loaded::Unreadable { error: e.to_string(), backup: moved.then_some(backup) }
            }
        }
    }

    /// Bring values that a hand-edited or damaged file could hold back into range.
    pub fn sanitize(&mut self) {
        self.thumb_size = if self.thumb_size.is_finite() { self.thumb_size.clamp(64.0, 256.0) } else { 120.0 };
        self.print.copies = self.print.copies.clamp(1, 99);
        for m in self.meta.values_mut() {
            m.upgrade();
        }
    }

    pub fn save_to(&mut self, path: &Path) -> Result<(), String> {
        let bytes = self.snapshot()?;
        write_library(path, &bytes)
    }

    /// The library as compact JSON, dropping notes that became empty. Quick next to writing it:
    /// the window does this on the UI thread (a consistent snapshot) and writes on another thread.
    pub fn snapshot(&mut self) -> Result<Vec<u8>, String> {
        self.meta.retain(|_, m| !m.is_empty());
        self.version = FORMAT_VERSION;
        serde_json::to_vec(self).map_err(|e| e.to_string())
    }
}

/// Write to a temporary file, flush it to disk, then rename it over the library, so that a crash
/// or a full disk never leaves a half-written library behind.
pub fn write_library(path: &Path, bytes: &[u8]) -> Result<(), String> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    let tmp = path.with_extension("json.tmp");
    let write = || -> std::io::Result<()> {
        use std::io::Write;
        let mut f = std::fs::File::create(&tmp)?;
        f.write_all(bytes)?;
        f.sync_all()?;
        std::fs::rename(&tmp, path)
    };
    write().map_err(|e| {
        let _ = std::fs::remove_file(&tmp);
        format!("{}: {e}", path.display())
    })
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

/// Most pictures one library scan takes in (keeps memory and start-up time bounded).
pub const SCAN_LIMIT: usize = 200_000;

/// The roots to scan: each once, and none inside another (its pictures would show twice).
pub fn distinct_roots(roots: &[String]) -> Vec<String> {
    let mut out: Vec<String> = vec![];
    for r in roots {
        let p = Path::new(r);
        if roots.iter().any(|o| o != r && p.starts_with(o)) || out.iter().any(|o| Path::new(o) == p) {
            continue;
        }
        out.push(r.clone());
    }
    out
}

impl Store {
    /// Add a folder to the library. A folder inside one already there is refused (it is shown
    /// already); folders inside the new one are replaced by it.
    pub fn add_root(&mut self, path: &str) -> Result<(), String> {
        let p = Path::new(path);
        if let Some(outer) = self.roots.iter().find(|r| p.starts_with(r)) {
            return Err(if Path::new(outer) == p {
                crate::msg!("root-already", "That folder is already in the library.").into()
            } else {
                crate::msg!("root-inside", "That folder is already in the library, as part of {outer}.", outer = outer).into()
            });
        }
        if let Some(e) = self.excluded.iter().find(|e| p.starts_with(e)) {
            return Err(crate::msg!("root-excluded", "That folder is inside {folder}, which is left out of the library.", folder = e).into());
        }
        self.roots.retain(|r| !Path::new(r).starts_with(p));
        self.roots.push(path.to_string());
        Ok(())
    }

    /// Leave a folder out of the library, with everything under it. It must be inside a
    /// library folder (leaving out a library folder itself is removing it).
    pub fn add_excluded(&mut self, path: &str) -> Result<(), String> {
        let p = Path::new(path);
        if self.roots.iter().any(|r| Path::new(r) == p) {
            return Err(crate::msg!("exclude-is-root", "That is a library folder: remove it instead.").into());
        }
        if !self.roots.iter().any(|r| p.starts_with(r)) {
            return Err(crate::msg!("exclude-outside", "That folder is not inside a library folder.").into());
        }
        if self.excluded.iter().any(|e| p.starts_with(e)) {
            return Err(crate::msg!("exclude-already", "That folder is already left out.").into());
        }
        self.excluded.retain(|e| !Path::new(e).starts_with(p));
        self.excluded.push(path.to_string());
        Ok(())
    }

    /// A file or folder was moved (or renamed) on disk from `from` to `to`: everything noted
    /// under that path (the pictures' notes and album places, folder captions and descriptions,
    /// library and left-out folders, `.picasa.ini` already read) follows it.
    pub fn moved(&mut self, from: &str, to: &str) {
        let new_path = |k: &str| -> Option<String> {
            let rest = Path::new(k).strip_prefix(from).ok()?;
            Some(if rest.as_os_str().is_empty() { to.to_string() } else { Path::new(to).join(rest).to_string_lossy().into_owned() })
        };
        fn rekey<V>(m: &mut HashMap<String, V>, f: &dyn Fn(&str) -> Option<String>) {
            let hit: Vec<String> = m.keys().filter(|k| f(k).is_some()).cloned().collect();
            for k in hit {
                if let (Some(n), Some(v)) = (f(&k), m.remove(&k)) {
                    m.insert(n, v);
                }
            }
        }
        fn relist(v: &mut [String], f: &dyn Fn(&str) -> Option<String>) {
            for k in v.iter_mut() {
                if let Some(n) = f(k) {
                    *k = n;
                }
            }
        }
        rekey(&mut self.meta, &new_path);
        rekey(&mut self.folder_notes, &new_path);
        rekey(&mut self.folder_captions, &new_path);
        for a in &mut self.albums {
            relist(&mut a.keys, &new_path);
        }
        relist(&mut self.roots, &new_path);
        relist(&mut self.excluded, &new_path);
        relist(&mut self.imported_ini, &new_path);
    }

    /// The folder is in the library: inside a library folder and not left out.
    pub fn shows_folder(&self, dir: &str) -> bool {
        let p = Path::new(dir);
        self.roots.iter().any(|r| p.starts_with(r)) && !self.excluded.iter().any(|e| p.starts_with(e))
    }
}

/// Whether `dir` is one of the excluded folders or inside one.
pub fn is_excluded(dir: &Path, excluded: &[String]) -> bool {
    excluded.iter().any(|e| dir.starts_with(e))
}

/// Scan the folders for pictures; the flag is true when the scan stopped at `limit`.
pub fn scan_roots(roots: &[String], limit: usize) -> (Vec<ScannedFolder>, bool) {
    scan_roots_except(roots, &[], limit)
}

/// `scan_roots`, leaving out the `excluded` folders and everything under them.
pub fn scan_roots_except(roots: &[String], excluded: &[String], limit: usize) -> (Vec<ScannedFolder>, bool) {
    let t0 = std::time::Instant::now();
    let mut out = vec![];
    let mut count = 0;
    for r in &distinct_roots(roots) {
        scan_dir(Path::new(r), 0, &mut out, &mut count, limit, excluded);
    }
    trace_since(&format!("scan of {} folder(s): {count} pictures", roots.len()), t0, 0);
    (out, count >= limit)
}

fn scan_dir(dir: &Path, depth: usize, out: &mut Vec<ScannedFolder>, count: &mut usize, limit: usize, excluded: &[String]) {
    if depth > 12 || *count >= limit || is_excluded(dir, excluded) {
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
        } else if let Some(ext) = path.extension().and_then(|e| e.to_str())
            && is_picture_extension(&ext.to_ascii_lowercase()) && md.len() > 0 {
                let modified = md
                    .modified()
                    .ok()
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|d| d.as_secs() as i64)
                    .unwrap_or(0);
                files.push((path.to_string_lossy().to_string(), modified, md.len()));
            }
    }
    if !files.is_empty() {
        files.sort_by_key(|a| a.0.to_lowercase());
        *count += files.len();
        out.push((dir.to_string_lossy().to_string(), files));
    }
    subdirs.sort();
    for s in subdirs {
        scan_dir(&s, depth + 1, out, count, limit, excluded);
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
mod moved_tests {
    use super::*;

    #[test]
    fn notes_follow_a_moved_folder_or_file() {
        let j = |parts: &[&str]| parts.iter().collect::<PathBuf>().to_string_lossy().to_string();
        let (trip, tripx, other) = (j(&["/p", "trip"]), j(&["/p", "tripx"]), j(&["/p", "2024", "trip"]));
        let a = j(&["/p", "trip", "a.jpg"]);
        let mut s = Store { roots: vec![j(&["/p"])], excluded: vec![j(&["/p", "trip", "raw"])], ..Default::default() };
        s.meta.insert(a.clone(), Meta { caption: "a".into(), ..Default::default() });
        s.meta.insert(j(&["/p", "tripx", "b.jpg"]), Meta { caption: "b".into(), ..Default::default() });
        s.albums.push(Album { name: "best".into(), keys: vec![a.clone()] });
        s.folder_notes.insert(trip.clone(), "note".into());
        s.folder_captions.insert(trip.clone(), "cap".into());
        s.imported_ini.push(trip.clone());
        s.moved(&trip, &other);
        let a2 = j(&["/p", "2024", "trip", "a.jpg"]);
        assert_eq!(s.meta[&a2].caption, "a");
        assert!(!s.meta.contains_key(&a));
        // a folder whose name only starts the same stays
        assert_eq!(s.meta[&j(&["/p", "tripx", "b.jpg"])].caption, "b");
        assert_eq!(s.albums[0].keys, vec![a2.clone()]);
        assert_eq!(s.folder_notes[&other], "note");
        assert_eq!(s.folder_captions[&other], "cap");
        assert_eq!(s.imported_ini, vec![other.clone()]);
        assert_eq!(s.excluded, vec![j(&["/p", "2024", "trip", "raw"])]);
        assert_eq!(s.roots, vec![j(&["/p"])]);
        let _ = tripx;
        // one file
        let a3 = j(&["/p", "2024", "trip", "beach.jpg"]);
        s.moved(&a2, &a3);
        assert_eq!(s.meta[&a3].caption, "a");
        assert_eq!(s.albums[0].keys, vec![a3]);
        // a library folder renamed
        s.moved(&j(&["/p"]), &j(&["/q"]));
        assert_eq!(s.roots, vec![j(&["/q"])]);
        assert!(s.shows_folder(&j(&["/q", "2024"])) && !s.shows_folder(&j(&["/q", "2024", "trip", "raw"])) && !s.shows_folder(&j(&["/p"])));
    }
}

#[cfg(test)]
mod queue_tests {
    use super::*;

    fn key(j: Option<Job>) -> String {
        match j {
            Some(Job::Thumb { key, .. }) => key,
            _ => String::new(),
        }
    }

    #[test]
    fn urgent_first_then_newest_first() {
        let mut q = Queues::default();
        let job = |k: &str| Job::Thumb { id: 0, key: k.into(), dim: THUMB_DIM };
        q.normal.push_back(job("old thumb"));
        q.normal.push_back(job("new thumb"));
        q.urgent.push_back(job("preview"));
        assert_eq!(key(q.pop()), "preview");
        assert_eq!(key(q.pop()), "new thumb");
        assert_eq!(key(q.pop()), "old thumb");
        assert!(q.pop().is_none());
    }

    #[test]
    fn old_thumbnail_jobs_are_forgotten_others_stay() {
        let mut q = Queues::default();
        q.normal.push_back(Job::Dims(vec![]));
        for i in 0..10 {
            q.normal.push_back(Job::Thumb { id: i, key: format!("t{i}"), dim: THUMB_DIM });
        }
        assert!(q.forget_old_thumbs(10).is_empty());
        q.normal.push_back(Job::Thumb { id: 10, key: "t10".into(), dim: THUMB_DIM });
        // 11 waiting, 8 allowed: down to 7, the oldest first
        let dropped = q.forget_old_thumbs(8);
        assert_eq!(dropped, vec!["t0", "t1", "t2", "t3", "t4"]);
        assert_eq!(q.normal.len(), 7);
        assert!(matches!(q.normal.front(), Some(Job::Dims(_))));
        assert_eq!(key(q.pop()), "t10");
    }
}

#[cfg(test)]
mod root_tests {
    use super::*;

    #[test]
    fn nested_roots_are_scanned_once() {
        let roots: Vec<String> = ["/p/2025", "/p/2025/Trip", "/home/me", "/p/2025"].iter().map(|s| s.to_string()).collect();
        assert_eq!(distinct_roots(&roots), vec!["/p/2025".to_string(), "/home/me".to_string()]);
        // a sibling with a common prefix is not inside
        let roots: Vec<String> = ["/p/2025", "/p/2025-old"].iter().map(|s| s.to_string()).collect();
        assert_eq!(distinct_roots(&roots), roots);
    }

    #[test]
    fn excluding_folders() {
        let mut s = Store::default();
        s.add_root("/p").unwrap();
        assert!(s.add_excluded("/p").is_err());
        assert!(s.add_excluded("/elsewhere/x").is_err());
        s.add_excluded("/p/Screenshots/old").unwrap();
        s.add_excluded("/p/Screenshots").unwrap();
        assert_eq!(s.excluded, vec!["/p/Screenshots".to_string()]);
        assert!(s.add_excluded("/p/Screenshots/new").is_err());
        assert!(s.add_root("/p/Screenshots/new").is_err());
        assert!(is_excluded(Path::new("/p/Screenshots/a"), &s.excluded));
        assert!(!is_excluded(Path::new("/p/Screenshots2"), &s.excluded));
    }

    #[test]
    fn adding_roots() {
        let mut s = Store::default();
        s.add_root("/p/2025/Trip").unwrap();
        s.add_root("/p/2025/Other").unwrap();
        assert!(s.add_root("/p/2025/Trip").is_err());
        assert!(s.add_root("/p/2025/Trip/day1").is_err());
        // the parent takes the place of both
        s.add_root("/p/2025").unwrap();
        assert_eq!(s.roots, vec!["/p/2025".to_string()]);
    }
}

#[cfg(test)]
mod store_tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("albumcraft-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// Only real files are moved; nothing else is touched (the real trash is not used here:
    /// `trash_real_file` does that, by hand).
    #[test]
    fn only_real_files_go_to_the_trash() {
        let report = move_to_trash(&[format!("{SAMPLE_PREFIX}1"), "/no/such/picture.jpg".into()], &|_, _| true);
        assert!(!report.ok);
        assert!(report.text.starts_with("Moved 0 pictures") && report.text.contains("picture.jpg: not found"));
        let stopped = move_to_trash(&["/no/such/a.jpg".into()], &|_, _| false);
        assert!(stopped.text.contains("Stopped"));
    }

    /// `XDG_DATA_HOME=<empty dir> cargo test -p albumcraft-core trash_real_file -- --ignored`
    /// (Linux): a real file goes to that folder's trash.
    #[test]
    #[ignore]
    fn trash_real_file() {
        let dir = temp_dir("trash-real");
        let f = dir.join("reject.jpg");
        std::fs::write(&f, b"not a picture").unwrap();
        let report = move_to_trash(&[f.to_string_lossy().to_string()], &|_, _| true);
        assert!(report.ok, "{}", report.text);
        assert!(!f.exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The Flatpak's way into the user's trash: the file moves into `files/`, an info file says
    /// where it was and when; a second file of the same name gets its own name.
    #[cfg(all(unix, not(target_os = "macos")))]
    #[test]
    fn a_file_goes_into_the_home_trash_with_its_info() {
        let dir = temp_dir("home-trash");
        let trash = dir.join("Trash");
        for round in 0..2 {
            let f = dir.join("my picture.jpg");
            std::fs::write(&f, [round as u8]).unwrap();
            into_home_trash(&f, &trash).expect("same disk").unwrap();
            assert!(!f.exists());
        }
        assert_eq!(std::fs::read(trash.join("files/my picture.jpg")).unwrap(), [0]);
        assert_eq!(std::fs::read(trash.join("files/my picture (2).jpg")).unwrap(), [1]);
        let info = std::fs::read_to_string(trash.join("info/my picture.jpg.trashinfo")).unwrap();
        assert!(info.starts_with("[Trash Info]\nPath=/") && info.contains("/my%20picture.jpg\nDeletionDate=20"), "{info}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn missing_file_is_a_first_start() {
        let dir = temp_dir("store-missing");
        assert!(matches!(Store::load_from(&dir.join("library.json")), Loaded::Fresh));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn save_and_load_round_trip() {
        let dir = temp_dir("store-roundtrip");
        let path = dir.join("library.json");
        let mut store = Store::default();
        store.meta.insert("a.jpg".into(), Meta { flag: Flag::Pick, rating: 4, ..Default::default() });
        store.meta.insert("empty.jpg".into(), Meta::default());
        store.meta.insert(format!("{SAMPLE_PREFIX}3"), Meta { rating: 5, ..Default::default() });
        store.save_to(&path).unwrap();
        assert!(!path.with_extension("json.tmp").exists());
        let Loaded::Store(back) = Store::load_from(&path) else { panic!("not loaded") };
        assert_eq!((back.meta["a.jpg"].flag, back.meta["a.jpg"].rating), (Flag::Pick, 4));
        assert!(!back.meta.contains_key("empty.jpg"));
        // notes on the sample pictures stay in the window, not in the file
        assert!(store.meta.contains_key(&format!("{SAMPLE_PREFIX}3")) && back.meta.len() == 1);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Bug 14: a library saved by a newer version loaded without complaint, and the next save
    /// dropped every field this version does not know.
    #[test]
    fn a_newer_library_is_never_saved_over() {
        let dir = temp_dir("store-newer");
        let path = dir.join("library.json");
        let text = format!("{{\"version\": {}, \"roots\": [\"/p\"], \"faces\": {{\"x\": 1}}}}", FORMAT_VERSION + 1);
        std::fs::write(&path, &text).unwrap();
        let Loaded::Unreadable { backup: None, error } = Store::load_from(&path) else { panic!("loaded a newer library") };
        assert!(error.contains("newer version"));
        // left where it was, as it was
        assert_eq!(std::fs::read_to_string(&path).unwrap(), text);
        // even when this version could not parse it at all
        std::fs::write(&path, format!("{{\"version\": {}, \"roots\": {{\"changed\": true}}}}", FORMAT_VERSION + 1)).unwrap();
        assert!(matches!(Store::load_from(&path), Loaded::Unreadable { backup: None, .. }));
        assert!(path.exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_format_version_is_saved_and_older_libraries_load() {
        let dir = temp_dir("store-version");
        let path = dir.join("library.json");
        std::fs::write(&path, "{\"roots\": [\"/p\"], \"sort\": \"by-colour\"}").unwrap();
        let Loaded::Store(mut store) = Store::load_from(&path) else { panic!("an older library must load") };
        // a setting this version does not know: the default, not an unreadable library
        assert_eq!(store.sort, Sort::Name);
        store.sort = Sort::Taken;
        store.save_to(&path).unwrap();
        let saved: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(saved["version"], FORMAT_VERSION);
        assert_eq!(saved["sort"], "taken");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn unreadable_library_is_kept_not_overwritten() {
        let dir = temp_dir("store-broken");
        let path = dir.join("library.json");
        std::fs::write(&path, "{\"meta\": {\"a.jpg\": {\"star\": tr").unwrap();
        let Loaded::Unreadable { backup: Some(backup), .. } = Store::load_from(&path) else { panic!("not detected") };
        assert!(!path.exists());
        assert!(std::fs::read_to_string(&backup).unwrap().contains("a.jpg"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn scan_reports_when_it_stops_at_the_limit() {
        let dir = temp_dir("scan-limit");
        for n in ["a.jpg", "b.png", "c.NEF", "notes.txt"] {
            std::fs::write(dir.join(n), b"x").unwrap();
        }
        let root = [dir.to_string_lossy().to_string()];
        let (all, cut) = scan_roots(&root, 10);
        let (_, cut_small) = scan_roots(&root, 3);
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(all.iter().map(|f| f.1.len()).sum::<usize>(), 3);
        assert!(!cut && cut_small);
    }

    #[test]
    fn old_stars_become_picks() {
        let dir = temp_dir("store-stars");
        let path = dir.join("library.json");
        std::fs::write(&path, r#"{"meta": {"a.jpg": {"star": true}, "b.jpg": {"star": false, "caption": "x"}, "c.jpg": {"rating": 9}}}"#).unwrap();
        let Loaded::Store(mut s) = Store::load_from(&path) else { panic!("not loaded") };
        assert_eq!(s.meta["a.jpg"].flag, Flag::Pick);
        assert_eq!(s.meta["b.jpg"].flag, Flag::None);
        assert_eq!(s.meta["c.jpg"].rating, 5);
        s.save_to(&path).unwrap();
        let text = std::fs::read_to_string(&path).unwrap();
        let _ = std::fs::remove_dir_all(&dir);
        assert!(!text.contains("star") && text.contains("\"flag\":\"pick\""));
    }

    #[test]
    fn places_are_saved_and_damaged_ones_dropped() {
        use crate::geo::Place;
        let dir = temp_dir("store-places");
        let path = dir.join("library.json");
        std::fs::write(
            &path,
            r#"{"meta": {"a.jpg": {"place": {"set": {"lat": 44.84, "lon": -0.58, "name": "Bordeaux"}}},
                         "b.jpg": {"place": "removed"},
                         "c.jpg": {"place": {"set": {"lat": 123.0, "lon": 0.0}}, "caption": "kept"},
                         "d.jpg": {"place": {"set": {"lat": 1.0, "lon": 2.0}}}}}"#,
        )
        .unwrap();
        let Loaded::Store(mut s) = Store::load_from(&path) else { panic!("not loaded") };
        assert_eq!(s.meta["a.jpg"].place, Some(Place::Set { lat: 44.84, lon: -0.58, name: "Bordeaux".into() }));
        assert_eq!(s.meta["b.jpg"].place, Some(Place::Removed));
        assert_eq!(s.meta["c.jpg"].place, None);
        assert_eq!(s.meta["c.jpg"].caption, "kept");
        assert_eq!(s.meta["d.jpg"].place, Some(Place::Set { lat: 1.0, lon: 2.0, name: String::new() }));
        assert_eq!(s.online_maps, None);
        assert!(s.export_location);
        s.save_to(&path).unwrap();
        let Loaded::Store(back) = Store::load_from(&path) else { panic!("not loaded") };
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(back.meta["a.jpg"].place, s.meta["a.jpg"].place);
        assert_eq!(back.meta["b.jpg"].place, Some(Place::Removed));
    }

    #[test]
    fn absurd_values_are_clamped() {
        let dir = temp_dir("store-clamp");
        let path = dir.join("library.json");
        std::fs::write(&path, r#"{"thumb_size": 100000.0, "print": {"copies": 4000000000}}"#).unwrap();
        let Loaded::Store(s) = Store::load_from(&path) else { panic!("not loaded") };
        assert_eq!(s.thumb_size, 256.0);
        assert_eq!(s.print.copies, 99);
        let _ = std::fs::remove_dir_all(&dir);
    }
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
        assert_eq!(m.flag, Flag::Pick);
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

/// (year, month, day) of a timestamp, without any time zone.
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

/// The timestamp moved to the local time zone (its offset at that moment, so summer time counts).
fn local(ts: i64) -> i64 {
    use chrono::TimeZone;
    let offset = chrono::Local.timestamp_opt(ts, 0).single().map(|d| i64::from(d.offset().local_minus_utc())).unwrap_or(0);
    ts.saturating_add(offset)
}

/// The year of a timestamp, in local time.
pub fn local_year(ts: i64) -> i32 {
    civil(local(ts)).0
}

/// "May 12, 2009", in local time.
pub fn fmt_date(ts: i64) -> String {
    fmt_date_utc(local(ts))
}

/// "10:05", in local time.
pub fn fmt_time(ts: i64) -> String {
    fmt_time_utc(local(ts))
}

fn fmt_date_utc(ts: i64) -> String {
    let (y, m, d) = civil(ts);
    crate::locale::date(y, m, d)
}

fn fmt_time_utc(ts: i64) -> String {
    let s = ts.rem_euclid(86_400);
    format!("{:02}:{:02}", s / 3600, (s % 3600) / 60)
}

#[cfg(test)]
mod date_tests {
    use super::*;

    #[test]
    fn dates_and_times() {
        let t = 1_242_122_700; // 2009-05-12 10:05 UTC
        assert_eq!(fmt_date_utc(t), "May 12, 2009");
        assert_eq!(fmt_time_utc(t), "10:05");
        // local time differs from UTC by the zone's offset only
        let shift = local(t) - t;
        assert!(shift.abs() <= 14 * 3600 && shift % 900 == 0);
    }
}

pub fn fmt_bytes(b: u64) -> String {
    use crate::locale::{decimal, unit};
    if b >= 1 << 30 {
        format!("{} {}", decimal(&format!("{:.1}", b as f64 / (1u64 << 30) as f64)), unit(2))
    } else if b >= 1 << 20 {
        format!("{} {}", decimal(&format!("{:.1}", b as f64 / (1u64 << 20) as f64)), unit(1))
    } else {
        format!("{} {}", (b / 1024).max(1), unit(0))
    }
}

// ---------------------------------------------------------------------------
// Background loader

pub enum Job {
    /// A thumbnail at `dim` pixels on its longest side: THUMB_DIM, or SHARP_THUMB_DIM for large
    /// tiles.
    Thumb { id: usize, key: String, dim: u32 },
    Big { id: usize, key: String },
    /// A sharp render for zooming in: the picture at up to ZOOM_DIM pixels with its edits applied.
    Zoom { id: usize, key: String, ops: Vec<EditOp> },
    /// Any picture file, not (yet) in the library, at THUMB_DIM or BIG_DIM: the import view.
    File { key: String, dim: u32 },
    /// The shapes and dates taken of these pictures (from their headers), for the grid's layout
    /// and the order by date.
    Dims(Vec<(usize, String)>),
    /// A step of the background analysis (see `analysis`). These wait in a queue of their own
    /// and run only while nothing else is waiting.
    Background(Box<dyn FnOnce() -> Done + Send>),
}

impl Job {
    fn run(self) -> Done {
        match self {
            Job::Thumb { id, key, dim } => {
                let img = cached_thumbnail(&key, dim);
                Done::Thumb { id, key, dim, img }
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
            Job::File { key, dim } => {
                let img = if dim == THUMB_DIM { cached_thumbnail(&key, dim) } else { imaging::load_source(&key, dim) };
                Done::File { key, dim, img }
            }
            Job::Dims(batch) => Done::Dims(batch.into_iter().map(|(id, key)| {
                let d = crate::catalogue::read_details(&key);
                (id, key, d)
            }).collect()),
            Job::Background(work) => work(),
        }
    }

    /// The answer sent when the job failed outright.
    fn failed(&self) -> Done {
        match self {
            Job::Thumb { id, key, dim } => Done::Thumb { id: *id, key: key.clone(), dim: *dim, img: None },
            Job::Big { id, key } => Done::Big { id: *id, key: key.clone(), img: None },
            Job::Zoom { id, key, ops } => Done::Zoom { id: *id, key: key.clone(), ops: ops.clone(), img: None, unedited_width: 0 },
            Job::File { key, dim } => Done::File { key: key.clone(), dim: *dim, img: None },
            Job::Dims(batch) => Done::Dims(batch.iter().map(|(id, key)| (*id, key.clone(), Default::default())).collect()),
            // the analysis catches its own failures; should one get through, nothing to tell
            Job::Background(_) => Done::Dropped(vec![]),
        }
    }
}

/// The error of work the user stopped (not a failure to report as one).
pub const STOPPED: &str = "stopped";

/// How a piece of background work ended.
#[derive(Debug, Clone)]
pub struct Report {
    /// The task it ends, when it was started as one (shown with progress and Cancel).
    pub task: Option<u64>,
    pub text: String,
    /// False: something went wrong; the message stays until the user closes it.
    pub ok: bool,
    /// A file or folder that was made, to offer "Show in folder".
    pub show: Option<PathBuf>,
}

impl Report {
    pub fn done(text: impl Into<String>) -> Report {
        Report { task: None, text: text.into(), ok: true, show: None }
    }
    pub fn failed(text: impl Into<String>) -> Report {
        Report { task: None, text: text.into(), ok: false, show: None }
    }
    pub fn showing(mut self, path: impl Into<PathBuf>) -> Report {
        self.show = Some(path.into());
        self
    }
}

/// Move these picture files to the system's trash (where the user can get them back until it is
/// emptied), after the user confirmed it: how AlbumCraft removes original files.
/// Sample pictures and files that are not there are left alone. `step(done, total)` says how far
/// it is and answers false when the user stopped it.
pub fn move_to_trash(files: &[String], step: &dyn Fn(usize, usize) -> bool) -> Report {
    trash_files(files, step).report()
}

/// What `trash_files` did.
pub struct Trashed {
    pub moved: usize,
    /// "file name: why" for each file left where it was.
    pub failed: Vec<String>,
    /// The paths of those the trash refused (a disk without one, a network share…): they can
    /// still be deleted for good if the user wants (`delete_permanently`).
    pub refused: Vec<String>,
    pub stopped: bool,
}

/// Move these files to the system's trash (see `move_to_trash`).
pub fn trash_files(files: &[String], step: &dyn Fn(usize, usize) -> bool) -> Trashed {
    let mut t = Trashed { moved: 0, failed: vec![], refused: vec![], stopped: false };
    for (i, f) in files.iter().enumerate() {
        if !step(i, files.len()) {
            t.stopped = true;
            break;
        }
        let name = f.rsplit(['/', '\\']).next().unwrap_or(f);
        // a picture file or a folder
        if f.starts_with(SAMPLE_PREFIX) || std::fs::symlink_metadata(f).is_err() {
            t.failed.push(format!("{name}: not found"));
            continue;
        }
        match trash_one(Path::new(f)) {
            Ok(()) => t.moved += 1,
            Err(e) => {
                t.failed.push(format!("{name}: {e}"));
                t.refused.push(f.clone());
            }
        }
    }
    t
}

fn trash_one(file: &Path) -> Result<(), String> {
    #[cfg(all(unix, not(target_os = "macos")))]
    if std::env::var_os("FLATPAK_ID").is_some()
        && let Some(trash) = dirs::home_dir().map(|h| h.join(".local/share/Trash"))
        && let Some(done) = into_home_trash(file, &trash)
    {
        return done;
    }
    trash::delete(file).map_err(|e| e.to_string())
}

/// In a Flatpak, `XDG_DATA_HOME` is the app's own folder, so the `trash` crate would move files
/// from the home disk into a trash nobody sees. Those go to the user's own trash instead
/// (`~/.local/share/Trash`, reachable through `--filesystem=home`), as the freedesktop.org
/// trash specification describes: an info file made first (its name taken atomically), then the
/// file renamed into `files/`. None when the file is not on the same disk as that trash (the
/// crate then uses that disk's own `.Trash-<uid>` folder, which does not depend on
/// `XDG_DATA_HOME`).
#[cfg(all(unix, not(target_os = "macos")))]
fn into_home_trash(file: &Path, trash: &Path) -> Option<Result<(), String>> {
    use std::io::Write;
    use std::os::unix::ffi::OsStrExt;
    use std::os::unix::fs::MetadataExt;
    let (files, info) = (trash.join("files"), trash.join("info"));
    std::fs::create_dir_all(&files).ok()?;
    std::fs::create_dir_all(&info).ok()?;
    if std::fs::metadata(file).ok()?.dev() != std::fs::metadata(&files).ok()?.dev() {
        return None;
    }
    let absolute = std::path::absolute(file).ok()?;
    let name = file.file_name()?.to_string_lossy().to_string();
    let (stem, ext) = match name.rsplit_once('.') {
        Some((s, e)) if !s.is_empty() => (s.to_string(), format!(".{e}")),
        _ => (name.clone(), String::new()),
    };
    // the path, URL-escaped byte by byte
    let mut path = String::new();
    for &b in absolute.as_os_str().as_bytes() {
        if b.is_ascii_alphanumeric() || b"-_.~/".contains(&b) {
            path.push(b as char);
        } else {
            path.push_str(&format!("%{b:02X}"));
        }
    }
    let date = chrono::Local::now().format("%Y-%m-%dT%H:%M:%S");
    for n in 1..1000 {
        let candidate = if n == 1 { name.clone() } else { format!("{stem} ({n}){ext}") };
        let info_path = info.join(format!("{candidate}.trashinfo"));
        if files.join(&candidate).exists() {
            continue;
        }
        match std::fs::OpenOptions::new().write(true).create_new(true).open(&info_path) {
            Ok(mut f) => {
                let written = write!(f, "[Trash Info]\nPath={path}\nDeletionDate={date}\n").and_then(|_| f.sync_all());
                let moved = written.and_then(|_| std::fs::rename(file, files.join(&candidate)));
                return Some(moved.map_err(|e| {
                    let _ = std::fs::remove_file(&info_path);
                    e.to_string()
                }));
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Some(Err(e.to_string())),
        }
    }
    Some(Err("no free name in the trash".into()))
}

/// Delete these files or folders for good (no trash: they can't be brought back), after the user
/// confirmed it; for what the trash refused. `step` as for `move_to_trash`.
pub fn delete_permanently(paths: &[String], step: &dyn Fn(usize, usize) -> bool) -> Report {
    let (mut deleted, mut failed, mut stopped) = (0usize, vec![], false);
    for (i, p) in paths.iter().enumerate() {
        if !step(i, paths.len()) {
            stopped = true;
            break;
        }
        let path = Path::new(p);
        let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| p.clone());
        let Ok(m) = std::fs::symlink_metadata(path).map_err(|_| ()).and_then(|m| if p.starts_with(SAMPLE_PREFIX) { Err(()) } else { Ok(m) }) else {
            failed.push(format!("{name}: not found"));
            continue;
        };
        let done = if m.is_dir() { std::fs::remove_dir_all(path) } else { std::fs::remove_file(path) };
        match done {
            Ok(()) => deleted += 1,
            Err(e) => failed.push(format!("{name}: {e}")),
        }
    }
    let mut text: String = crate::msg!("deleted-for-good", "Deleted {n} {n:item|items} for good.", n = deleted).into();
    if stopped {
        text.push('\n');
        text.push_str(&String::from(crate::msg!("deleted-stopped", "Stopped before the end.")));
    }
    for f in failed.iter().take(10) {
        text.push_str("\n• ");
        text.push_str(f);
    }
    if failed.is_empty() { Report::done(text) } else { Report::failed(text) }
}

impl Trashed {
    pub fn report(&self) -> Report {
        let (moved, failed, stopped) = (self.moved, &self.failed, self.stopped);
        let mut text = if moved == 1 { "Moved 1 picture to the trash.".to_string() } else { format!("Moved {moved} pictures to the trash.") };
    if stopped {
        text.push_str("\nStopped before the end.");
    }
    if !failed.is_empty() {
        text.push_str("\n\nNot moved:");
        for f in failed.iter().take(10) {
            text.push_str("\n• ");
            text.push_str(f);
        }
        if failed.len() > 10 {
            text.push_str(&format!("\n… and {} more", failed.len() - 10));
        }
    }
    if failed.is_empty() { Report::done(text) } else { Report::failed(text) }
    }
}

/// A picture's thumbnail: from the disk cache, else made from the file and put in the cache.
pub(crate) fn cached_thumbnail(key: &str, dim: u32) -> Option<RgbaImage> {
    crate::thumbcache::get(key, dim).or_else(|| {
        let img = imaging::load_thumbnail(key, dim)?;
        crate::thumbcache::put(key, dim, &img);
        Some(img)
    })
}

pub enum Done {
    Thumb { id: usize, key: String, dim: u32, img: Option<RgbaImage> },
    Big { id: usize, key: String, img: Option<RgbaImage> },
    /// `unedited_width` is the width of the loaded picture before edits (to know its scale).
    Zoom { id: usize, key: String, ops: Vec<EditOp>, img: Option<RgbaImage>, unedited_width: u32 },
    /// Scanned folders, whether the scan stopped at SCAN_LIMIT, and the `.picasa.ini` notes read
    /// in folders not brought in before (by folder path).
    Scan(Vec<ScannedFolder>, bool, Vec<(String, IniFolder)>),
    /// The catalogue cache (see `catalogue`): the folders the last scan found and what was read
    /// from their pictures, shown until the scan's result arrives.
    Cached(Vec<ScannedFolder>, crate::catalogue::KnownDetails),
    /// How a piece of background work ended, to tell the user.
    Report(Report),
    /// How far a task (see `Report::task`) is: `done` of `total` steps.
    Progress { task: u64, done: usize, total: usize },
    /// Thumbnail jobs forgotten because too many were waiting (by key): not made, not failed.
    Dropped(Vec<String>),
    /// A short progress note for the toast.
    Toast(String),
    /// Files or folders moved to another disk in the background (`Library::move_across`):
    /// (old path, new path) of those moved (the task's report says why others were not).
    Moved(Vec<(String, String)>),
    /// The trash refused these files or folders (`Trashed::refused`): the window asks whether to
    /// delete them for good.
    NotTrashed(Vec<String>),
    /// The cached thumbnails of these pictures were removed (`Library::forget_thumbs`): the
    /// window asks for them again.
    ThumbsForgotten(Vec<String>),
    File { key: String, dim: u32, img: Option<RgbaImage> },
    /// (photo id, key, shown size and date taken) for each picture of a `Job::Dims`.
    Dims(Vec<FoundDims>),
    /// The pictures found in an import source (see `import::list`).
    ImportList { source: std::path::PathBuf, items: Vec<crate::import::Candidate>, truncated: bool },
    /// `done` of `total` pictures copied so far.
    ImportProgress { done: usize, total: usize },
    /// The import into `dest` finished: what to tell the user, and whether anything was copied.
    ImportDone { dest: std::path::PathBuf, summary: String, copied: usize, problems: bool },
    /// Bytes used on disk by the thumbnail cache and the map tile cache.
    CacheSizes { thumbs: u64, tiles: u64 },
    /// How far the background analysis of the library is.
    Analysis(crate::analysis::Progress),
    /// The pictures a smart search found, best first (`Library::looks_like`).
    LooksLike(crate::smart_search::Query, Result<Vec<crate::smart_search::Hit>, String>),
    /// Tags proposed for some pictures (`Library::propose_tags`; the number it was asked with).
    TagProposals(u64, Result<crate::tag_proposals::Proposals, String>),
}

/// (photo id, key, what its header says).
pub type FoundDims = (usize, String, crate::catalogue::Details);

pub const THUMB_DIM: u32 = 256;
/// Thumbnails for tiles larger than THUMB_DIM pixels (large tiles, high-density screens).
pub const SHARP_THUMB_DIM: u32 = 512;
pub const BIG_DIM: u32 = 1400;
/// Longest side of the sharp render used when zooming in (keeps memory and GPU textures in bounds).
pub const ZOOM_DIM: u32 = 5000;

/// The photo's real size in pixels as shown, i.e. after the EXIF orientation (read from the file
/// header, or the sample's size).
pub fn source_dims(key: &str) -> Option<(u32, u32)> {
    if let Some(seed) = key.strip_prefix(imaging::SAMPLE_PREFIX) {
        return seed.parse().ok().map(imaging::sample_size);
    }
    imaging::oriented_dims(key)
}

/// Called after each finished job so the window draws the result (e.g. `ctx.request_repaint`).
pub type Wake = Arc<dyn Fn() + Send + Sync>;

/// Waiting jobs: urgent ones (the picture being looked at) first, then the others; newest first
/// in both, so that what is on screen now comes before what was on screen a moment ago. Last,
/// and only when both are empty, the background analysis, oldest first.
#[derive(Default)]
struct Queues {
    urgent: VecDeque<Job>,
    normal: VecDeque<Job>,
    background: VecDeque<Job>,
    /// Background jobs being run now, and the most allowed at once: the other workers stay free
    /// for what the user is looking at.
    background_running: usize,
    background_max: usize,
}

impl Queues {
    fn pop(&mut self) -> Option<Job> {
        if let Some(job) = self.urgent.pop_back().or_else(|| self.normal.pop_back()) {
            return Some(job);
        }
        if self.background_running >= self.background_max {
            return None;
        }
        let job = self.background.pop_front()?;
        self.background_running += 1;
        Some(job)
    }

    /// Keep at most `max` thumbnail jobs waiting in the normal queue: the oldest go. Returns
    /// their pictures' keys. Other jobs (shapes, import previews) always stay.
    fn forget_old_thumbs(&mut self, max: usize) -> Vec<String> {
        let waiting = self.normal.iter().filter(|j| matches!(j, Job::Thumb { .. })).count();
        if waiting <= max {
            return vec![];
        }
        // down to three quarters, so that this is not done for every job that comes in
        let mut extra = waiting - max / 4 * 3;
        let mut dropped = vec![];
        self.normal.retain(|j| match j {
            Job::Thumb { key, .. } if extra > 0 => {
                extra -= 1;
                dropped.push(key.clone());
                false
            }
            _ => true,
        });
        dropped
    }
}

/// Most thumbnail jobs kept waiting. When the user scrolls quickly through a large library every
/// picture passed is asked for; beyond this, the ones asked for longest ago (long out of sight)
/// are forgotten and reported as `Done::Dropped`, to be asked for again if they come back.
const MAX_WAITING_THUMBS: usize = 1000;

#[derive(Clone)]
pub struct Pool {
    queue: Arc<(Mutex<Queues>, Condvar)>,
    tx: Sender<Done>,
    wake: Wake,
}

impl Pool {
    pub fn new(threads: usize, tx: Sender<Done>, wake: Wake) -> Self {
        // analysis on one or two threads, and always a worker left for the window
        let queues = Queues { background_max: if threads > 2 { 2 } else { threads.saturating_sub(1).max(1) }, ..Default::default() };
        let queue: Arc<(Mutex<Queues>, Condvar)> = Arc::new((Mutex::new(queues), Condvar::new()));
        for _ in 0..threads {
            let q = queue.clone();
            let tx = tx.clone();
            let wake = wake.clone();
            std::thread::spawn(move || loop {
                let job = {
                    let (lock, cv) = &*q;
                    let mut g = lock.lock().unwrap_or_else(PoisonError::into_inner);
                    loop {
                        if let Some(j) = g.pop() {
                            break j;
                        }
                        g = cv.wait(g).unwrap_or_else(PoisonError::into_inner);
                    }
                };
                // A picture that makes a decoder or an edit panic must not take the worker down:
                // answer "no picture" for it and carry on.
                let failed = job.failed();
                let background = matches!(job, Job::Background(_));
                let done = std::panic::catch_unwind(AssertUnwindSafe(|| job.run())).unwrap_or(failed);
                if background {
                    let mut g = q.0.lock().unwrap_or_else(PoisonError::into_inner);
                    g.background_running = g.background_running.saturating_sub(1);
                }
                if tx.send(done).is_err() {
                    return;
                }
                wake();
            });
        }
        Self { queue, tx, wake }
    }

    pub fn push(&self, job: Job, urgent: bool) {
        let (lock, cv) = &*self.queue;
        let mut g = lock.lock().unwrap_or_else(PoisonError::into_inner);
        if matches!(job, Job::Background(_)) {
            g.background.push_back(job);
        } else if urgent {
            g.urgent.push_back(job);
        } else {
            g.normal.push_back(job);
        }
        let dropped = g.forget_old_thumbs(MAX_WAITING_THUMBS);
        cv.notify_one();
        drop(g);
        if !dropped.is_empty() {
            let _ = self.tx.send(Done::Dropped(dropped));
            (self.wake)();
        }
    }
}

#[cfg(test)]
mod background_queue_tests {
    use super::*;

    fn name(job: Option<Job>) -> &'static str {
        match job {
            Some(Job::Big { .. }) => "urgent",
            Some(Job::Thumb { .. }) => "normal",
            Some(Job::Background(_)) => "background",
            _ => "none",
        }
    }

    /// Analysis jobs are handed out only when nothing else waits, oldest first, and at most
    /// `background_max` at a time.
    #[test]
    fn background_jobs_come_last_and_few_at_a_time() {
        let mut q = Queues { background_max: 2, ..Default::default() };
        let background = || Job::Background(Box::new(|| Done::Toast(String::new())));
        for _ in 0..3 {
            q.background.push_back(background());
        }
        q.normal.push_back(Job::Thumb { id: 0, key: "a".into(), dim: THUMB_DIM });
        q.urgent.push_back(Job::Big { id: 0, key: "a".into() });
        assert_eq!([name(q.pop()), name(q.pop()), name(q.pop()), name(q.pop())], ["urgent", "normal", "background", "background"]);
        // two are running: the third waits, but the window's jobs still go out
        assert_eq!(name(q.pop()), "none");
        q.normal.push_back(Job::Thumb { id: 1, key: "b".into(), dim: THUMB_DIM });
        assert_eq!([name(q.pop()), name(q.pop())], ["normal", "none"]);
        q.background_running -= 1;
        assert_eq!([name(q.pop()), name(q.pop())], ["background", "none"]);
        // too many thumbnails waiting never drops analysis jobs
        q.background.push_back(background());
        for id in 0..40 {
            q.normal.push_back(Job::Thumb { id, key: "c".into(), dim: THUMB_DIM });
        }
        assert_eq!(q.forget_old_thumbs(10).len(), 34);
        assert_eq!(q.background.len(), 1);
    }
}
