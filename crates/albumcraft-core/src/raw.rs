//! Camera raw files, shown through the JPEG preview the camera embeds in them.
//!
//! Nearly every raw format (CR2, CR3, NEF, ARW, DNG, ORF, RW2, PEF, RAF, SRW…) carries a JPEG
//! preview, usually at full or near-full size. Rather than parse each container, the file is
//! searched for embedded JPEG streams and the largest one that decodes is used; the turn to apply
//! comes from the raw file's own EXIF orientation (the preview is usually stored unturned).
//! Developing the raw sensor data itself is not supported.

use image::{DynamicImage, ImageDecoder, metadata::Orientation};
use std::io::Cursor;

pub const RAW_EXTENSIONS: [&str; 19] = [
    "cr2", "cr3", "crw", "nef", "nrw", "arw", "srf", "sr2", "dng", "orf", "rw2", "raw", "pef", "raf", "srw", "3fr", "erf", "kdc", "mrw",
];

/// Largest raw file read (a preview is never worth more memory than that).
const MAX_FILE: u64 = 400 << 20;
/// Most embedded JPEG candidates looked at per file.
const MAX_CANDIDATES: usize = 64;

pub fn is_raw(path: &str) -> bool {
    std::path::Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| RAW_EXTENSIONS.contains(&e.to_ascii_lowercase().as_str()))
}

/// Embedded JPEG streams: (offset, width, height), largest first.
fn candidates(data: &[u8]) -> Vec<(usize, u32, u32)> {
    let mut found = vec![];
    let mut i = 0;
    while found.len() < MAX_CANDIDATES {
        let Some(pos) = data.get(i..).and_then(|rest| rest.windows(3).position(|w| w == [0xFF, 0xD8, 0xFF])) else { break };
        let off = i + pos;
        i = off + 3;
        let Some(stream) = data.get(off..) else { break };
        let Ok(dec) = image::codecs::jpeg::JpegDecoder::new(Cursor::new(stream)) else { continue };
        let (w, h) = dec.dimensions();
        if w >= 16 && h >= 16 {
            found.push((off, w, h));
        }
    }
    found.sort_by_key(|&(_, w, h)| std::cmp::Reverse(u64::from(w) * u64::from(h)));
    found
}

fn read_file(path: &str) -> Option<Vec<u8>> {
    let len = std::fs::metadata(path).ok()?.len();
    if len > MAX_FILE {
        return None;
    }
    std::fs::read(path).ok()
}

/// The raw file's own "this way up", if it records one.
fn raw_orientation(path: &str) -> Option<Orientation> {
    let file = std::fs::File::open(path).ok()?;
    let exif = exif::Reader::new().read_from_container(&mut std::io::BufReader::new(file)).ok()?;
    let v = exif.get_field(exif::Tag::Orientation, exif::In::PRIMARY)?.value.get_uint(0)?;
    Orientation::from_exif(u8::try_from(v).ok()?)
}

/// The largest embedded preview that decodes, turned the way the camera recorded.
pub fn preview(path: &str) -> Option<DynamicImage> {
    let data = read_file(path)?;
    let orientation = raw_orientation(path);
    for (off, _, _) in candidates(&data) {
        let Some(stream) = data.get(off..) else { continue };
        let Ok(mut dec) = image::codecs::jpeg::JpegDecoder::new(Cursor::new(stream)) else { continue };
        let own = dec.orientation().unwrap_or(Orientation::NoTransforms);
        if let Ok(mut img) = DynamicImage::from_decoder(dec) {
            img.apply_orientation(orientation.unwrap_or(own));
            return Some(img);
        }
    }
    None
}

/// Size of the preview as shown (after the turn), without decoding it.
pub fn preview_dims(path: &str) -> Option<(u32, u32)> {
    let data = read_file(path)?;
    let &(_, w, h) = candidates(&data).first()?;
    let turned = matches!(
        raw_orientation(path),
        Some(Orientation::Rotate90 | Orientation::Rotate270 | Orientation::Rotate90FlipH | Orientation::Rotate270FlipH)
    );
    Some(if turned { (h, w) } else { (w, h) })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn jpeg(w: u32, h: u32) -> Vec<u8> {
        let img = image::RgbImage::from_pixel(w, h, image::Rgb([200, 30, 30]));
        let mut out = Vec::new();
        img.write_with_encoder(image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, 80)).unwrap();
        out
    }

    #[test]
    fn largest_embedded_preview_is_used() {
        // a fake raw: header bytes, a small thumbnail, a lossless-JPEG-like stream that won't decode,
        // the large preview, then "sensor data"
        let mut file = b"II*\0fake raw header".to_vec();
        file.extend(jpeg(32, 24));
        file.extend([0x00; 100]);
        file.extend([0xFF, 0xD8, 0xFF, 0xC3, 0x00, 0x0B, 0x08, 0x10, 0x00, 0x10, 0x00, 0x01, 0x01, 0x11, 0x00]);
        file.extend([0x5A; 300]);
        file.extend(jpeg(160, 120));
        file.extend([0xAB; 5000]);
        let path = std::env::temp_dir().join(format!("albumcraft-raw-{}.NEF", std::process::id()));
        std::fs::write(&path, file).unwrap();
        let key = path.to_string_lossy().to_string();
        let img = preview(&key).map(|i| (i.width(), i.height()));
        let dims = preview_dims(&key);
        let _ = std::fs::remove_file(&path);
        assert!(is_raw(&key) && !is_raw("a.jpg"));
        assert_eq!(img, Some((160, 120)));
        assert_eq!(dims, Some((160, 120)));
    }

    /// `ALBUMCRAFT_RAW_SAMPLE=/path/to/file.NEF cargo test real_raw_file -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn real_raw_file() {
        let path = std::env::var("ALBUMCRAFT_RAW_SAMPLE").expect("set ALBUMCRAFT_RAW_SAMPLE");
        let t = std::time::Instant::now();
        let img = preview(&path).expect("a preview");
        println!("preview {}x{} in {:?}; dims {:?}", img.width(), img.height(), t.elapsed(), preview_dims(&path));
        let out = std::env::temp_dir().join("albumcraft-raw-preview.jpg");
        img.to_rgb8().save(&out).unwrap();
        println!("saved {}", out.display());
    }

    #[test]
    fn no_preview_is_no_picture() {
        let path = std::env::temp_dir().join(format!("albumcraft-raw-empty-{}.cr2", std::process::id()));
        std::fs::write(&path, [0xFFu8, 0xD8, 0xFF, 0x00, 0x01, 0x02]).unwrap();
        let key = path.to_string_lossy().to_string();
        let r = preview(&key).is_none() && preview_dims(&key).is_none();
        let _ = std::fs::remove_file(&path);
        assert!(r);
    }
}
