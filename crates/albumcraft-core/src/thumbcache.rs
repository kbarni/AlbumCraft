//! Persistent on-disk cache of 256 px thumbnails, in `dirs::cache_dir()/AlbumCraft/thumbs`.
//!
//! The file name is a hash of (format version, path, modified time, size, THUMB_DIM), so an edited or replaced
//! picture simply misses the cache; stale files are harmless and can be deleted at any time.
//! Opaque thumbnails are stored as JPEG (small), ones with transparency as PNG.
//! A thumbnail's file time is refreshed (at most daily) when it is used, and `prune` removes the
//! least recently used ones when the cache grows past its size limit.

use image::RgbaImage;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Size the cache is kept under (about 80,000 thumbnails).
pub const MAX_BYTES: u64 = 2 << 30;

const JPEG_QUALITY: u8 = 88;
const VERSION: u32 = 2;

fn dir() -> Option<PathBuf> {
    Some(dirs::cache_dir()?.join("AlbumCraft").join("thumbs"))
}

/// FNV-1a: stable across Rust versions (unlike `DefaultHasher`).
fn fnv(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf29ce484222325u64, |h, b| (h ^ *b as u64).wrapping_mul(0x100000001b3))
}

/// Base path (no extension) for this picture's cached thumbnail, or None if it can't be cached.
fn base(cache: &Path, key: &str, dim: u32) -> Option<PathBuf> {
    if key.starts_with(crate::imaging::SAMPLE_PREFIX) {
        return None;
    }
    let meta = std::fs::metadata(key).ok()?;
    let mtime = meta.modified().ok()?.duration_since(UNIX_EPOCH).ok()?.as_nanos();
    // bump VERSION when the way thumbnails are made changes (2: EXIF orientation applied)
    let id = format!("{VERSION}\0{key}\0{mtime}\0{}\0{dim}", meta.len());
    Some(cache.join(format!("{:016x}", fnv(id.as_bytes()))))
}

pub fn get(key: &str, dim: u32) -> Option<RgbaImage> {
    get_in(&dir()?, key, dim)
}

pub fn put(key: &str, dim: u32, img: &RgbaImage) {
    if let Some(d) = dir() {
        put_in(&d, key, dim, img);
    }
}

/// Remove the least recently used thumbnails until the cache is under 80 % of `MAX_BYTES`, and
/// leftovers of interrupted writes. Meant for a background thread at start-up.
pub fn prune() {
    if let Some(d) = dir() {
        prune_in(&d, MAX_BYTES);
    }
}

fn get_in(cache: &Path, key: &str, dim: u32) -> Option<RgbaImage> {
    let base = base(cache, key, dim)?;
    for ext in ["jpg", "png"] {
        let path = base.with_extension(ext);
        if let Ok(img) = image::open(&path) {
            touch(&path);
            return Some(img.to_rgba8());
        }
    }
    None
}

/// Mark a thumbnail as recently used (file time), at most once a day to spare the disk.
fn touch(path: &Path) {
    let day = Duration::from_secs(86_400);
    let now = SystemTime::now();
    let old = std::fs::metadata(path).and_then(|m| m.modified()).is_ok_and(|t| now.duration_since(t).is_ok_and(|age| age > day));
    if old
        && let Ok(f) = std::fs::File::options().write(true).open(path) {
            let _ = f.set_modified(now);
        }
}

fn put_in(cache: &Path, key: &str, dim: u32, img: &RgbaImage) {
    let Some(base) = base(cache, key, dim) else { return };
    if std::fs::create_dir_all(cache).is_err() {
        return;
    }
    let opaque = img.pixels().all(|p| p.0[3] == 255);
    let (ext, saved) = if opaque {
        let rgb = image::DynamicImage::ImageRgba8(img.clone()).to_rgb8();
        let mut buf = Vec::new();
        let enc = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buf, JPEG_QUALITY);
        ("jpg", rgb.write_with_encoder(enc).map(|_| buf).ok())
    } else {
        let mut buf = std::io::Cursor::new(Vec::new());
        ("png", img.write_to(&mut buf, image::ImageFormat::Png).map(|_| buf.into_inner()).ok())
    };
    let Some(bytes) = saved else { return };
    // Write then rename so a crash or a parallel worker never leaves a half-written file.
    let tmp = base.with_extension(format!("tmp{}", std::process::id()));
    let target = base.with_extension(ext);
    if std::fs::write(&tmp, bytes).is_ok() && std::fs::rename(&tmp, &target).is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
}

fn prune_in(cache: &Path, max_bytes: u64) {
    let Ok(rd) = std::fs::read_dir(cache) else { return };
    let now = SystemTime::now();
    let mut files: Vec<(SystemTime, u64, PathBuf)> = vec![];
    let mut total = 0u64;
    for e in rd.flatten() {
        let Ok(m) = e.metadata() else { continue };
        if !m.is_file() {
            continue;
        }
        let path = e.path();
        let modified = m.modified().unwrap_or(UNIX_EPOCH);
        let is_thumb = path.extension().is_some_and(|x| x == "jpg" || x == "png");
        if !is_thumb {
            // an interrupted write: remove once it is clearly not in progress
            if now.duration_since(modified).is_ok_and(|age| age > Duration::from_secs(3600)) {
                let _ = std::fs::remove_file(&path);
            }
            continue;
        }
        total = total.saturating_add(m.len());
        files.push((modified, m.len(), path));
    }
    if total <= max_bytes {
        return;
    }
    let target = max_bytes / 5 * 4;
    files.sort_by_key(|f| f.0);
    for (_, len, path) in files {
        if total <= target {
            break;
        }
        if std::fs::remove_file(&path).is_ok() {
            total = total.saturating_sub(len);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("albumcraft-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn roundtrip_and_invalidation() {
        let dir = temp_dir("thumb-test");
        let cache = dir.join("cache");
        let file = dir.join("a.png");
        RgbaImage::from_pixel(40, 30, image::Rgba([10, 200, 30, 255])).save(&file).unwrap();
        let key = file.to_string_lossy().to_string();
        assert!(get_in(&cache, &key, 256).is_none());
        let thumb = RgbaImage::from_pixel(20, 15, image::Rgba([10, 200, 30, 255]));
        put_in(&cache, &key, 256, &thumb);
        let back = get_in(&cache, &key, 256).expect("cached");
        assert_eq!((back.width(), back.height()), (20, 15));
        // a changed file (new size) misses the cache
        RgbaImage::from_pixel(50, 50, image::Rgba([0, 0, 0, 255])).save(&file).unwrap();
        assert!(get_in(&cache, &key, 256).is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn prune_removes_least_recently_used_first() {
        let cache = temp_dir("thumb-prune");
        let day = Duration::from_secs(86_400);
        let now = SystemTime::now();
        for (i, name) in ["old.jpg", "mid.jpg", "new.jpg"].iter().enumerate() {
            let p = cache.join(name);
            std::fs::write(&p, vec![0u8; 1000]).unwrap();
            let f = std::fs::File::options().write(true).open(&p).unwrap();
            f.set_modified(now - day * (3 - i as u32)).unwrap();
        }
        std::fs::write(cache.join("x.tmp1"), b"left over").unwrap();
        let f = std::fs::File::options().write(true).open(cache.join("x.tmp1")).unwrap();
        f.set_modified(now - day).unwrap();
        prune_in(&cache, 2500); // 3000 bytes > 2500: remove down to 2000
        let left: Vec<bool> = ["old.jpg", "mid.jpg", "new.jpg", "x.tmp1"].iter().map(|n| cache.join(n).exists()).collect();
        let _ = std::fs::remove_dir_all(&cache);
        assert_eq!(left, [false, true, true, false]);
    }
}
