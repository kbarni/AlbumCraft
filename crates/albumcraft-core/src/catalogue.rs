//! The catalogue cache: what the last scan found (folders, files with their date and size) and
//! what was read from each file's header since (its shape and the date it was taken), kept in
//! `dirs::cache_dir()/AlbumCraft/catalogue.json`.
//!
//! At start-up the window shows the cached catalogue at once, while the folders are scanned
//! again in the background; the scan's result then replaces it, keeping what is known about
//! files that did not change (same date and size). Like the thumbnail cache, the file can be
//! deleted at any time: it only makes the start quicker. It never holds notes (those are in
//! `library.json`).

use crate::imaging;
use crate::library::{self, Folder, Photo, ScannedFolder};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Bump when the meaning of a field changes; a cache of another version is ignored.
pub const VERSION: u32 = 1;

#[derive(Serialize, Deserialize, Default, Debug)]
#[serde(default)]
pub struct Catalogue {
    pub version: u32,
    pub folders: Vec<CachedFolder>,
}

#[derive(Serialize, Deserialize, Default, Debug)]
#[serde(default)]
pub struct CachedFolder {
    pub path: String,
    pub files: Vec<CachedFile>,
}

/// One picture. Short field names: a large library has hundreds of thousands of them.
#[derive(Serialize, Deserialize, Default, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct CachedFile {
    /// The file's path after its folder's path (usually the separator and the file name).
    #[serde(rename = "n")]
    pub rest: String,
    #[serde(rename = "m")]
    pub modified: i64,
    #[serde(rename = "b")]
    pub bytes: u64,
    /// The shape as shown (after the EXIF orientation), when it was read.
    #[serde(rename = "d", skip_serializing_if = "Option::is_none")]
    pub dims: Option<(u32, u32)>,
    /// When the picture was taken (Unix seconds; the file's date when it has no EXIF date), when
    /// it was read.
    #[serde(rename = "t", skip_serializing_if = "Option::is_none")]
    pub taken: Option<i64>,
}

/// What the header of a picture says: its shape and when it was taken. Either is None when the
/// header can't be read.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Details {
    pub dims: Option<(u32, u32)>,
    pub taken: Option<i64>,
}

/// (key, details) of a cached picture whose header was read before.
pub type KnownDetails = Vec<(String, Details)>;

pub fn path() -> Option<PathBuf> {
    Some(dirs::cache_dir()?.join("AlbumCraft").join("catalogue.json"))
}

/// Read the shape and the date taken from the picture's header (JPEG: the first segments only).
pub fn read_details(key: &str) -> Details {
    if let Some(seed) = key.strip_prefix(imaging::SAMPLE_PREFIX) {
        return Details { dims: seed.parse().ok().map(imaging::sample_size), taken: None };
    }
    if !crate::raw::is_raw(key)
        && let Some(head) = imaging::jpeg_head(key)
    {
        let taken = head.exif().and_then(|e| crate::import::exif_date_of(&e));
        return Details { dims: Some(head.oriented_dims()), taken };
    }
    Details { dims: library::source_dims(key), taken: crate::import::date_taken(key) }
}

impl Catalogue {
    /// The catalogue of what the window shows: its folders, and for each picture what `known`
    /// says was read from it.
    pub fn of(folders: &[Folder], photos: &[Photo], known: impl Fn(usize) -> Details) -> Catalogue {
        let folders = folders
            .iter()
            .map(|f| CachedFolder {
                path: f.path.clone(),
                files: f
                    .photos
                    .iter()
                    .filter_map(|&id| {
                        let p = photos.get(id)?;
                        let rest = p.key.strip_prefix(f.path.as_str())?.to_string();
                        let d = known(id);
                        Some(CachedFile { rest, modified: p.modified, bytes: p.bytes, dims: d.dims, taken: d.taken })
                    })
                    .collect(),
            })
            .collect();
        Catalogue { version: VERSION, folders }
    }

    /// The folders inside these roots, as a scan would give them, and the details known for
    /// their pictures. At most `limit` pictures.
    pub fn within(self, roots: &[String], limit: usize) -> (Vec<ScannedFolder>, KnownDetails) {
        self.within_except(roots, &[], limit)
    }

    /// `within`, leaving out the `excluded` folders and everything under them.
    pub fn within_except(self, roots: &[String], excluded: &[String], limit: usize) -> (Vec<ScannedFolder>, KnownDetails) {
        let roots = library::distinct_roots(roots);
        let mut scanned = vec![];
        let mut known = vec![];
        let mut count = 0usize;
        for f in self.folders {
            if !roots.iter().any(|r| Path::new(&f.path).starts_with(r)) || f.files.is_empty() || library::is_excluded(Path::new(&f.path), excluded) {
                continue;
            }
            if count.saturating_add(f.files.len()) > limit {
                break;
            }
            count += f.files.len();
            let mut files = Vec::with_capacity(f.files.len());
            for c in f.files {
                let key = format!("{}{}", f.path, c.rest);
                if c.dims.is_some() || c.taken.is_some() {
                    known.push((key.clone(), Details { dims: c.dims, taken: c.taken }));
                }
                files.push((key, c.modified, c.bytes));
            }
            scanned.push((f.path, files));
        }
        (scanned, known)
    }
}

/// Read the cache at `path`. None when there is none, or it can't be used (damaged, another
/// version): it is only a cache, the scan finds everything again.
pub fn load_from(path: &Path) -> Option<Catalogue> {
    let bytes = std::fs::read(path).ok()?;
    let cat: Catalogue = serde_json::from_slice(&bytes).ok()?;
    (cat.version == VERSION).then_some(cat)
}

/// Write the cache at `path` (to a temporary file, then renamed over it; one write at a time,
/// as they share the temporary file).
pub fn save_to(path: &Path, cat: &Catalogue) -> Result<(), String> {
    static WRITING: std::sync::Mutex<()> = std::sync::Mutex::new(());
    let bytes = serde_json::to_vec(cat).map_err(|e| e.to_string())?;
    let _one = WRITING.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    library::write_library(path, &bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn photo(key: &str, folder: usize) -> Photo {
        Photo { key: key.into(), name: key.rsplit('/').next().unwrap_or(key).into(), folder, modified: 100, bytes: 2000 }
    }

    #[test]
    fn a_catalogue_comes_back_as_it_was_saved() {
        let dir = std::env::temp_dir().join(format!("albumcraft-catalogue-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let photos = vec![photo("/pics/a/1.jpg", 0), photo("/pics/a/2.jpg", 0), photo("/other/b/3.jpg", 1)];
        let folders = vec![
            Folder { path: "/pics/a".into(), name: "a".into(), date: 100, photos: vec![0, 1] },
            Folder { path: "/other/b".into(), name: "b".into(), date: 100, photos: vec![2] },
        ];
        let cat = Catalogue::of(&folders, &photos, |id| if id == 1 { Details { dims: Some((30, 20)), taken: Some(42) } } else { Details::default() });
        let path = dir.join("catalogue.json");
        save_to(&path, &cat).unwrap();
        let back = load_from(&path).expect("readable");
        // only the folders inside the library's folders now
        let (scanned, known) = back.within(&["/pics".into()], 1000);
        assert_eq!(scanned, vec![("/pics/a".to_string(), vec![("/pics/a/1.jpg".to_string(), 100, 2000), ("/pics/a/2.jpg".to_string(), 100, 2000)])]);
        assert_eq!(known, vec![("/pics/a/2.jpg".to_string(), Details { dims: Some((30, 20)), taken: Some(42) })]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_damaged_or_other_version_cache_is_ignored() {
        let dir = std::env::temp_dir().join(format!("albumcraft-catalogue-bad-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("catalogue.json");
        std::fs::write(&path, b"{\"version\":1,\"folders\":[{\"path\":").unwrap();
        assert!(load_from(&path).is_none());
        std::fs::write(&path, format!("{{\"version\":{},\"folders\":[]}}", VERSION + 1)).unwrap();
        assert!(load_from(&path).is_none());
        assert!(load_from(&dir.join("missing.json")).is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_limit_is_kept() {
        let photos: Vec<Photo> = (0..10).map(|i| photo(&format!("/p/{}/{i}.jpg", i / 5), i / 5)).collect();
        let folders = vec![
            Folder { path: "/p/0".into(), name: "0".into(), date: 0, photos: (0..5).collect() },
            Folder { path: "/p/1".into(), name: "1".into(), date: 0, photos: (5..10).collect() },
        ];
        let (scanned, _) = Catalogue::of(&folders, &photos, |_| Details::default()).within(&["/p".into()], 7);
        assert_eq!(scanned.len(), 1);
    }

    /// `cargo test --release -p albumcraft-core perf_catalogue -- --ignored --nocapture`: writing
    /// and reading the cache of a library at the scan limit.
    #[test]
    #[ignore]
    fn perf_catalogue() {
        let n = library::SCAN_LIMIT;
        let photos: Vec<Photo> = (0..n).map(|i| photo(&format!("/home/someone/Pictures/{}/IMG_{i:06}.JPG", i / 200), i / 200)).collect();
        let folders: Vec<Folder> = (0..n / 200)
            .map(|f| Folder { path: format!("/home/someone/Pictures/{f}"), name: f.to_string(), date: 0, photos: (f * 200..f * 200 + 200).collect() })
            .collect();
        let dir = std::env::temp_dir().join(format!("albumcraft-catalogue-perf-{}", std::process::id()));
        let path = dir.join("catalogue.json");
        let t = std::time::Instant::now();
        let cat = Catalogue::of(&folders, &photos, |_| Details { dims: Some((4032, 3024)), taken: Some(1_600_000_000) });
        let built = t.elapsed();
        save_to(&path, &cat).unwrap();
        let saved = t.elapsed() - built;
        let t = std::time::Instant::now();
        let (scanned, known) = load_from(&path).unwrap().within(&["/home/someone/Pictures".into()], n);
        let loaded = t.elapsed();
        let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
        println!("{n} pictures: built {built:?} (window thread), written {saved:?}, read {loaded:?}; {} MB", size / 1_000_000);
        assert_eq!((scanned.iter().map(|f| f.1.len()).sum::<usize>(), known.len()), (n, n));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn details_of_a_jpeg_come_from_its_header() {
        let dir = std::env::temp_dir().join(format!("albumcraft-details-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("demo.jpg");
        crate::exifinfo::write_demo_jpeg(&path).unwrap();
        let d = read_details(&path.to_string_lossy());
        assert!(d.dims.is_some_and(|(w, h)| w > 0 && h > 0));
        assert_eq!(d.taken, crate::import::parse_exif_date("2006:08:18 10:34:25"));
        assert_eq!(read_details("/no/such/file.jpg"), Details::default());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
