//! Importing: copying pictures from a memory card, a camera or any folder into a folder of the
//! library. (Adding a folder to the library, by contrast, copies nothing: it is only scanned.)
//!
//! The source is never changed. Copies never overwrite a file: a name already taken by another
//! file gets " (2)", " (3)"… and a file that is already there with the same content is skipped.
//! Each copy is written to a temporary file first and renamed when complete, so a full disk or
//! a pulled card leaves no half-written picture behind.

use crate::library::{self, ScannedFolder};
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

/// Most pictures one import lists (a card holds far fewer; keeps memory bounded).
pub const LIST_LIMIT: usize = 50_000;

/// Pictures taken further apart than this start a new group in the import view.
pub const GROUP_GAP_SECS: i64 = 2 * 3600;

/// A picture found in the source folder.
#[derive(Clone, Debug)]
pub struct Candidate {
    pub path: String,
    pub name: String,
    /// When it was taken (EXIF), else the file's modification time; Unix seconds.
    pub taken: i64,
    pub bytes: u64,
    /// The library already has a picture with the same content.
    pub duplicate: bool,
}

/// All pictures in `source` and its subfolders, oldest first, and whether the list stopped at
/// `LIST_LIMIT`. `library` is (file path, size) for each picture of the library, to mark the
/// ones it already has.
pub fn list(source: &Path, library: &[(String, u64)]) -> (Vec<Candidate>, bool) {
    let (folders, truncated) = library::scan_roots(&[source.to_string_lossy().to_string()], LIST_LIMIT);
    let mut found = candidates(folders);
    mark_duplicates(&mut found, library);
    (found, truncated)
}

fn candidates(folders: Vec<ScannedFolder>) -> Vec<Candidate> {
    let mut out: Vec<Candidate> = folders
        .into_iter()
        .flat_map(|(_, files)| files)
        .map(|(path, modified, bytes)| {
            let taken = date_taken(&path).unwrap_or(modified);
            let name = file_name(&path);
            Candidate { path, name, taken, bytes, duplicate: false }
        })
        .collect();
    out.sort_by(|a, b| a.taken.cmp(&b.taken).then_with(|| a.name.cmp(&b.name)));
    out
}

fn file_name(path: &str) -> String {
    path.rsplit(['/', '\\']).next().unwrap_or(path).to_string()
}

/// The EXIF "taken" time as Unix seconds (it is recorded in the camera's local time).
pub(crate) fn date_taken(path: &str) -> Option<i64> {
    // JPEG: from the header only; other files (TIFF, raw): the EXIF reader
    match crate::imaging::jpeg_head(path) {
        Some(head) => exif_date_of(&head.exif()?),
        None => parse_exif_date(&crate::exifinfo::read(path)?.date_taken?),
    }
}

/// The "taken" time of an EXIF block, as Unix seconds.
pub(crate) fn exif_date_of(exif: &exif::Exif) -> Option<i64> {
    let field = exif.get_field(exif::Tag::DateTimeOriginal, exif::In::PRIMARY)?;
    match &field.value {
        exif::Value::Ascii(v) => parse_exif_date(&String::from_utf8_lossy(v.first()?)),
        _ => None,
    }
}

/// An EXIF date ("2006:08:18 10:34:25", the camera's local time) as Unix seconds.
pub fn parse_exif_date(text: &str) -> Option<i64> {
    use chrono::TimeZone;
    let naive = chrono::NaiveDateTime::parse_from_str(text.trim().trim_end_matches('\0'), "%Y:%m:%d %H:%M:%S").ok()?;
    chrono::Local.from_local_datetime(&naive).earliest().map(|d| d.timestamp())
}

/// Mark the candidates whose content the library already has: same size, then same fingerprint.
fn mark_duplicates(found: &mut [Candidate], library: &[(String, u64)]) {
    let sizes: HashSet<u64> = found.iter().map(|c| c.bytes).collect();
    let mut known: HashMap<u64, Vec<u64>> = HashMap::new();
    for (path, bytes) in library {
        if sizes.contains(bytes)
            && let Some(f) = fingerprint(Path::new(path), *bytes)
        {
            known.entry(*bytes).or_default().push(f);
        }
    }
    for c in found.iter_mut() {
        if let Some(prints) = known.get(&c.bytes) {
            c.duplicate = fingerprint(Path::new(&c.path), c.bytes).is_some_and(|f| prints.contains(&f));
        }
    }
}

/// A quick fingerprint of a file's content: its size and its first and last 64 KB. Two
/// different photos of the same size practically never share both ends.
fn fingerprint(path: &Path, bytes: u64) -> Option<u64> {
    const PART: u64 = 64 * 1024;
    let mut f = std::fs::File::open(path).ok()?;
    let mut h = std::collections::hash_map::DefaultHasher::new();
    bytes.hash(&mut h);
    let mut buf = vec![0u8; PART.min(bytes) as usize];
    f.read_exact(&mut buf).ok()?;
    buf.hash(&mut h);
    if bytes > PART {
        f.seek(SeekFrom::Start(bytes.saturating_sub(PART))).ok()?;
        f.read_exact(&mut buf).ok()?;
        buf.hash(&mut h);
    }
    Some(h.finish())
}

/// Index ranges of `items` (sorted by time) taken within `GROUP_GAP_SECS` of each other.
pub fn groups(items: &[Candidate]) -> Vec<std::ops::Range<usize>> {
    let mut out = vec![];
    let mut start = 0;
    for i in 1..=items.len() {
        let split = match (items.get(i - 1), items.get(i)) {
            (Some(a), Some(b)) => b.taken.saturating_sub(a.taken) > GROUP_GAP_SECS,
            _ => true,
        };
        if split {
            if start < i {
                out.push(start..i);
            }
            start = i;
        }
    }
    out
}

/// The folder name to suggest for pictures from `source`: its own name, unless it is one of the
/// names cameras give their folders (DCIM, 100CANON…), which say nothing about the pictures.
pub fn suggested_name(source: &Path) -> String {
    let name = source.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    if is_camera_folder(&name) { String::new() } else { name }
}

fn is_camera_folder(name: &str) -> bool {
    let n = name.to_ascii_uppercase();
    let digits = n.chars().take(3).filter(|c| c.is_ascii_digit()).count();
    n.is_empty() || n == "DCIM" || n == "CAMERA" || n == "PRIVATE" || (digits == 3 && n.len() == 8) || n.ends_with(":\\") || n == "/"
}

/// Checks a new folder's name typed by the user; returns it trimmed.
pub fn check_folder_name(name: &str) -> Result<String, String> {
    let name = name.trim();
    if name.is_empty() {
        return Err(crate::msg!("import-name-empty", "Type a name for the new folder.").into());
    }
    if name == "." || name == ".." || name.chars().any(|c| matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|') || c.is_control()) {
        return Err(crate::msg!("import-name-chars", "A folder name can't contain / \\ : * ? \" < > |").into());
    }
    if name.len() > 200 {
        return Err(crate::msg!("import-name-long", "That folder name is too long.").into());
    }
    Ok(name.to_string())
}

/// Memory cards, cameras and USB drives that are plugged in, as (label, folder). On Windows, the
/// drives other than C: that have a DCIM folder; elsewhere, the usual mount folders.
pub fn removable_sources() -> Vec<(String, PathBuf)> {
    let mut out = vec![];
    #[cfg(windows)]
    for letter in b'D'..=b'Z' {
        let root = PathBuf::from(format!("{}:\\", letter as char));
        if root.join("DCIM").is_dir() {
            out.push((crate::msg!("import-drive", "Drive {letter}:", letter = letter as char).to_string(), root));
        }
    }
    #[cfg(not(windows))]
    {
        let user = std::env::var("USER").unwrap_or_default();
        let mut parents = vec![PathBuf::from("/Volumes")];
        if !user.is_empty() {
            parents.push(PathBuf::from("/media").join(&user));
            parents.push(PathBuf::from("/run/media").join(&user));
        }
        for parent in parents {
            let Ok(rd) = std::fs::read_dir(&parent) else { continue };
            let mut found: Vec<PathBuf> = rd.flatten().map(|e| e.path()).filter(|p| p.is_dir()).collect();
            found.sort();
            for p in found {
                // macOS lists the system disk too
                if parent == Path::new("/Volumes") && std::fs::read_link(&p).is_ok_and(|t| t == Path::new("/")) {
                    continue;
                }
                let label = p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
                // straight to the camera's pictures when there are any
                let dcim = p.join("DCIM");
                out.push((label, if dcim.is_dir() { dcim } else { p }));
            }
        }
    }
    out
}

/// What a copy did.
#[derive(Debug, Default)]
pub struct Report {
    /// The new files.
    pub copied: Vec<PathBuf>,
    /// Pictures already in the folder with the same content (not copied again).
    pub already_there: usize,
    /// (picture, why) for each picture that could not be copied.
    pub failed: Vec<(String, String)>,
    /// Stopped by the user before the end.
    pub stopped: bool,
}

impl Report {
    /// One sentence or two for the user.
    pub fn summary(&self, dest: &Path) -> String {
        let mut s = format!("Imported {} picture(s) into {}.", self.copied.len(), dest.display());
        if self.already_there > 0 {
            s += &format!(" {} were already there.", self.already_there);
        }
        if self.stopped {
            s += " The import was stopped before the end.";
        }
        if !self.failed.is_empty() {
            s += &format!("\n\n{} could not be copied:", self.failed.len());
            for (f, why) in self.failed.iter().take(8) {
                s += &format!("\n{}: {why}", file_name(f));
            }
            if self.failed.len() > 8 {
                s += "\n…";
            }
        }
        s
    }
}

/// Copy `files` into `dest` (created if needed), all in that one folder. `progress(done, total)`
/// is called after each file; setting `stop` ends the copy after the current file.
pub fn copy_into(files: &[String], dest: &Path, progress: &dyn Fn(usize, usize), stop: &AtomicBool) -> Result<Report, String> {
    std::fs::create_dir_all(dest).map_err(|e| crate::msg!("import-mkdir-fail", "The folder {path} could not be made: {error}", path = dest.display(), error = e))?;
    let mut report = Report::default();
    for (i, src) in files.iter().enumerate() {
        if stop.load(Ordering::Relaxed) {
            report.stopped = true;
            break;
        }
        match copy_one(Path::new(src), dest) {
            Ok(Some(path)) => report.copied.push(path),
            Ok(None) => report.already_there += 1,
            Err(e) => report.failed.push((src.clone(), e)),
        }
        progress(i + 1, files.len());
    }
    Ok(report)
}

/// Copy one file into `dest`. None: the same picture is already there.
fn copy_one(src: &Path, dest: &Path) -> Result<Option<PathBuf>, String> {
    let meta = std::fs::metadata(src).map_err(|e| e.to_string())?;
    let name = src.file_name().ok_or("no file name")?.to_string_lossy().to_string();
    let (stem, ext) = match name.rfind('.') {
        Some(i) if i > 0 => (&name[..i], &name[i..]),
        _ => (name.as_str(), ""),
    };
    let mut target = dest.join(&name);
    let mut n = 1;
    while let Ok(existing) = std::fs::metadata(&target) {
        if existing.len() == meta.len() && fingerprint(&target, existing.len()).is_some_and(|f| Some(f) == fingerprint(src, meta.len())) {
            return Ok(None);
        }
        n += 1;
        if n > 9999 {
            return Err(crate::msg!("import-too-many", "too many files with this name").into());
        }
        target = dest.join(format!("{stem} ({n}){ext}"));
    }
    let temp = dest.join(format!(".{name}.albumcraft-part"));
    let result = (|| -> std::io::Result<()> {
        std::fs::copy(src, &temp)?;
        // keep the original's date: the library sorts and groups by it
        if let Ok(modified) = meta.modified() {
            let f = std::fs::OpenOptions::new().write(true).open(&temp)?;
            let _ = f.set_modified(modified);
        }
        // the name was free a moment ago; nothing else writes into this new folder meanwhile
        if target.exists() {
            return Err(std::io::Error::new(std::io::ErrorKind::AlreadyExists, crate::msg!("import-appeared", "a file with this name appeared during the copy").to_string()));
        }
        std::fs::rename(&temp, &target)
    })();
    match result {
        Ok(()) => Ok(Some(target)),
        Err(e) => {
            let _ = std::fs::remove_file(&temp);
            Err(e.to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("albumcraft-import-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn cand(taken: i64) -> Candidate {
        Candidate { path: String::new(), name: String::new(), taken, bytes: 1, duplicate: false }
    }

    #[test]
    fn groups_split_on_long_gaps() {
        let items = [cand(0), cand(600), cand(600 + GROUP_GAP_SECS + 1), cand(600 + GROUP_GAP_SECS + 2)];
        assert_eq!(groups(&items), vec![0..2, 2..4]);
        assert!(groups(&[]).is_empty());
        assert_eq!(groups(&[cand(5)]), vec![0..1]);
    }

    #[test]
    fn exif_dates() {
        assert!(parse_exif_date("2006:08:18 10:34:25").is_some());
        assert!(parse_exif_date("2006:08:18 10:34:25\0").is_some());
        assert_eq!(parse_exif_date("0000:00:00 00:00:00"), None);
        assert_eq!(parse_exif_date(""), None);
    }

    #[test]
    fn folder_names() {
        assert_eq!(check_folder_name("  Spain by bike "), Ok("Spain by bike".into()));
        assert!(check_folder_name("").is_err());
        assert!(check_folder_name("..").is_err());
        assert!(check_folder_name("a/b").is_err());
        assert!(check_folder_name("a\\b").is_err());
        assert_eq!(suggested_name(Path::new("/media/card/DCIM")), "");
        assert_eq!(suggested_name(Path::new("/media/card/DCIM/100CANON")), "");
        assert_eq!(suggested_name(Path::new("/home/me/Arcachon")), "Arcachon");
    }

    /// Copies never overwrite: a taken name gets " (2)", the same picture is skipped, and the
    /// source and the existing files are unchanged.
    #[test]
    fn copy_never_overwrites() {
        let src = temp_dir("src");
        let dest = temp_dir("dest");
        std::fs::create_dir_all(src.join("100CAM")).unwrap();
        std::fs::create_dir_all(src.join("101CAM")).unwrap();
        std::fs::write(src.join("100CAM/IMG_1.JPG"), b"first picture").unwrap();
        std::fs::write(src.join("101CAM/IMG_1.JPG"), b"second picture").unwrap();
        std::fs::write(src.join("100CAM/IMG_2.JPG"), b"same as in dest").unwrap();
        std::fs::write(dest.join("IMG_2.JPG"), b"same as in dest").unwrap();
        std::fs::write(dest.join("IMG_1.JPG"), b"an older, other picture").unwrap();

        let (found, truncated) = list(&src, &[]);
        assert!(!truncated);
        assert_eq!(found.len(), 3);
        let files: Vec<String> = found.iter().map(|c| c.path.clone()).collect();
        let report = copy_into(&files, &dest, &|_, _| {}, &AtomicBool::new(false)).unwrap();
        assert_eq!(report.copied.len(), 2);
        assert_eq!(report.already_there, 1);
        assert!(report.failed.is_empty());
        assert_eq!(std::fs::read(dest.join("IMG_1.JPG")).unwrap(), b"an older, other picture");
        let mut copies: Vec<Vec<u8>> = ["IMG_1 (2).JPG", "IMG_1 (3).JPG"].iter().map(|n| std::fs::read(dest.join(n)).unwrap()).collect();
        copies.sort();
        assert_eq!(copies, vec![b"first picture".to_vec(), b"second picture".to_vec()]);
        assert_eq!(std::fs::read(src.join("100CAM/IMG_1.JPG")).unwrap(), b"first picture");
        // no temporary files left
        assert!(std::fs::read_dir(&dest).unwrap().flatten().all(|e| !e.file_name().to_string_lossy().ends_with("albumcraft-part")));

        // importing the same card again copies nothing
        let again = copy_into(&files, &dest, &|_, _| {}, &AtomicBool::new(false)).unwrap();
        assert_eq!(again.copied.len(), 0);
        assert_eq!(again.already_there, 3);
        let _ = std::fs::remove_dir_all(&src);
        let _ = std::fs::remove_dir_all(&dest);
    }

    #[test]
    fn duplicates_of_the_library_are_marked() {
        let src = temp_dir("dup-src");
        let lib = temp_dir("dup-lib");
        std::fs::write(src.join("a.jpg"), b"picture A").unwrap();
        std::fs::write(src.join("b.jpg"), b"picture B").unwrap();
        std::fs::write(lib.join("renamed.jpg"), b"picture A").unwrap();
        std::fs::write(lib.join("same-size.jpg"), b"picture C").unwrap();
        let library: Vec<(String, u64)> =
            ["renamed.jpg", "same-size.jpg"].iter().map(|n| (lib.join(n).to_string_lossy().to_string(), 9)).collect();
        let (found, _) = list(&src, &library);
        let dup: Vec<(String, bool)> = found.iter().map(|c| (c.name.clone(), c.duplicate)).collect();
        assert!(dup.contains(&("a.jpg".into(), true)));
        assert!(dup.contains(&("b.jpg".into(), false)));
        let _ = std::fs::remove_dir_all(&src);
        let _ = std::fs::remove_dir_all(&lib);
    }

    #[test]
    fn stop_ends_the_copy() {
        let src = temp_dir("stop-src");
        let dest = temp_dir("stop-dest");
        std::fs::write(src.join("a.jpg"), b"a").unwrap();
        let report = copy_into(&[src.join("a.jpg").to_string_lossy().to_string()], &dest, &|_, _| {}, &AtomicBool::new(true)).unwrap();
        assert!(report.stopped);
        assert!(report.copied.is_empty());
        // a missing source file is reported, not fatal
        let report = copy_into(&["/no/such/file.jpg".into()], &dest, &|_, _| {}, &AtomicBool::new(false)).unwrap();
        assert_eq!(report.failed.len(), 1);
        let _ = std::fs::remove_dir_all(&src);
        let _ = std::fs::remove_dir_all(&dest);
    }
}
