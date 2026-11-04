//! Persistent on-disk cache of 256 px thumbnails, in `dirs::cache_dir()/AlbumCraft/thumbs`.
//!
//! The file name is a hash of (path, modified time, size, THUMB_DIM), so an edited or replaced
//! picture simply misses the cache; stale files are harmless and can be deleted at any time.
//! Opaque thumbnails are stored as JPEG (small), ones with transparency as PNG.

use image::RgbaImage;
use std::path::PathBuf;
use std::time::UNIX_EPOCH;

const JPEG_QUALITY: u8 = 88;

fn dir() -> Option<PathBuf> {
    Some(dirs::cache_dir()?.join("AlbumCraft").join("thumbs"))
}

/// FNV-1a: stable across Rust versions (unlike `DefaultHasher`).
fn fnv(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf29ce484222325u64, |h, b| (h ^ *b as u64).wrapping_mul(0x100000001b3))
}

/// Base path (no extension) for this picture's cached thumbnail, or None if it can't be cached.
fn base(key: &str, dim: u32) -> Option<PathBuf> {
    if key.starts_with(crate::imaging::SAMPLE_PREFIX) {
        return None;
    }
    let meta = std::fs::metadata(key).ok()?;
    let mtime = meta.modified().ok()?.duration_since(UNIX_EPOCH).ok()?.as_nanos();
    let id = format!("{key}\0{mtime}\0{}\0{dim}", meta.len());
    Some(dir()?.join(format!("{:016x}", fnv(id.as_bytes()))))
}

pub fn get(key: &str, dim: u32) -> Option<RgbaImage> {
    let base = base(key, dim)?;
    for ext in ["jpg", "png"] {
        if let Ok(img) = image::open(base.with_extension(ext)) {
            return Some(img.to_rgba8());
        }
    }
    None
}

pub fn put(key: &str, dim: u32, img: &RgbaImage) {
    let Some(base) = base(key, dim) else { return };
    let Some(parent) = base.parent() else { return };
    if std::fs::create_dir_all(parent).is_err() {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_and_invalidation() {
        let dir = std::env::temp_dir().join(format!("albumcraft-thumb-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("a.png");
        RgbaImage::from_pixel(40, 30, image::Rgba([10, 200, 30, 255])).save(&file).unwrap();
        let key = file.to_string_lossy().to_string();
        assert!(get(&key, 7777).is_none());
        let thumb = RgbaImage::from_pixel(20, 15, image::Rgba([10, 200, 30, 255]));
        put(&key, 7777, &thumb);
        let back = get(&key, 7777).expect("cached");
        assert_eq!((back.width(), back.height()), (20, 15));
        // a changed file (new size) misses the cache
        RgbaImage::from_pixel(50, 50, image::Rgba([0, 0, 0, 255])).save(&file).unwrap();
        assert!(get(&key, 7777).is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
