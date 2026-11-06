//! Exporting edited copies of pictures as new files (originals are never touched).

use crate::imaging::{self, EditOp};
use std::path::{Path, PathBuf};

/// Export one picture with its edits as `<stem>.jpg` in `dir` (or `<stem>-1.jpg`, … when taken),
/// with `gps` in its EXIF. Never overwrites a file, and removes its own half-written file when
/// something fails.
pub fn export_one(key: &str, edits: &[EditOp], dir: &Path, stem: &str, max_dim: u32, quality: u8, gps: Option<crate::geo::LatLon>) -> Result<PathBuf, String> {
    let src = imaging::load_source(key, max_dim).ok_or("the picture could not be read")?;
    let img = imaging::apply_all(&src, edits);
    let rgb = image::DynamicImage::ImageRgba8(img).to_rgb8();
    // encoded before the file is made, so that a failure leaves nothing behind
    let mut jpeg = vec![];
    rgb.write_with_encoder(image::codecs::jpeg::JpegEncoder::new_with_quality(&mut jpeg, quality)).map_err(|e| e.to_string())?;
    if let Some(at) = gps {
        jpeg = crate::exifinfo::jpeg_with_exif(&jpeg, &crate::geo::exif_gps_fields(at), None).map_err(|e| e.to_string())?;
    }
    let mut n = 0;
    let (file, out) = loop {
        let out = if n == 0 { dir.join(format!("{stem}.jpg")) } else { dir.join(format!("{stem}-{n}.jpg")) };
        match std::fs::OpenOptions::new().write(true).create_new(true).open(&out) {
            Ok(f) => break (f, out),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists && n < 10_000 => n += 1,
            Err(e) => return Err(e.to_string()),
        }
    };
    let write = || -> Result<(), String> {
        use std::io::Write;
        let mut f = file;
        f.write_all(&jpeg).map_err(|e| e.to_string())?;
        f.flush().map_err(|e| e.to_string())
    };
    match write() {
        Ok(()) => Ok(out),
        Err(e) => {
            let _ = std::fs::remove_file(&out);
            Err(e)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn export_never_overwrites() {
        let dir = std::env::temp_dir().join(format!("albumcraft-export-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("a.jpg"), b"keep me").unwrap();
        let key = format!("{}1", imaging::SAMPLE_PREFIX);
        export_one(&key, &[], &dir, "a", 64, 80, None).unwrap();
        export_one(&key, &[], &dir, "a", 64, 80, None).unwrap();
        let unreadable = export_one("/no/such/picture.jpg", &[], &dir, "b", 64, 80, None);
        let kept = std::fs::read(dir.join("a.jpg")).unwrap();
        let made = [dir.join("a-1.jpg"), dir.join("a-2.jpg")].map(|p| image::image_dimensions(p).ok());
        let b_exists = dir.join("b.jpg").exists();
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(kept, b"keep me");
        assert!(made.iter().all(|d| d.is_some_and(|(w, h)| w.max(h) == 64)));
        assert!(unreadable.is_err() && !b_exists);
    }
}
