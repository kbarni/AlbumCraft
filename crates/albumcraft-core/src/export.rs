//! Exporting edited copies of pictures as new files (originals are never touched).

use crate::imaging::{self, EditOp};
use std::path::{Path, PathBuf};

/// Export one picture with its edits as `<stem>.jpg` in `dir` (or `<stem>-1.jpg`, … when taken),
/// with `gps` in its EXIF, and the original's camera information when `camera_info`
/// (`exifinfo::fields_to_keep`). Never overwrites a file, and removes its own half-written file
/// when something fails.
#[allow(clippy::too_many_arguments)]
pub fn export_one(key: &str, edits: &[EditOp], dir: &Path, stem: &str, max_dim: u32, quality: u8, gps: Option<crate::geo::LatLon>, camera_info: bool) -> Result<PathBuf, String> {
    let src = imaging::load_source(key, max_dim).ok_or("the picture could not be read")?;
    let img = imaging::apply_all(&src, edits);
    let rgb = image::DynamicImage::ImageRgba8(img).to_rgb8();
    // encoded before the file is made, so that a failure leaves nothing behind
    let mut jpeg = vec![];
    rgb.write_with_encoder(image::codecs::jpeg::JpegEncoder::new_with_quality(&mut jpeg, quality)).map_err(|e| e.to_string())?;
    let gps_fields = gps.map(crate::geo::exif_gps_fields).unwrap_or_default();
    let mut fields = if camera_info { crate::exifinfo::fields_to_keep(key) } else { vec![] };
    fields.extend(gps_fields.iter().cloned());
    if !fields.is_empty() {
        // an original field the writer refuses must not cost the location
        jpeg = match crate::exifinfo::jpeg_with_exif(&jpeg, &fields, None) {
            Ok(j) => j,
            Err(_) if gps_fields.is_empty() => jpeg,
            Err(_) => crate::exifinfo::jpeg_with_exif(&jpeg, &gps_fields, None).map_err(|e| e.to_string())?,
        };
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
        export_one(&key, &[], &dir, "a", 64, 80, None, true).unwrap();
        export_one(&key, &[], &dir, "a", 64, 80, None, true).unwrap();
        let unreadable = export_one("/no/such/picture.jpg", &[], &dir, "b", 64, 80, None, true);
        let kept = std::fs::read(dir.join("a.jpg")).unwrap();
        let made = [dir.join("a-1.jpg"), dir.join("a-2.jpg")].map(|p| image::image_dimensions(p).ok());
        let b_exists = dir.join("b.jpg").exists();
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(kept, b"keep me");
        assert!(made.iter().all(|d| d.is_some_and(|(w, h)| w.max(h) == 64)));
        assert!(unreadable.is_err() && !b_exists);
    }

    #[test]
    fn camera_information_is_copied_only_when_asked() {
        let dir = std::env::temp_dir().join(format!("albumcraft-export-exif-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let original = dir.join("original.jpg");
        crate::exifinfo::write_demo_jpeg(&original).unwrap();
        let key = original.to_string_lossy().to_string();
        let at = crate::geo::LatLon { lat: 44.84, lon: -0.58 };
        let with = export_one(&key, &[], &dir, "with", 200, 80, Some(at), true).unwrap();
        let without = export_one(&key, &[], &dir, "without", 200, 80, Some(at), false).unwrap();
        let read = |p: &Path| crate::exifinfo::read(&p.to_string_lossy());
        let (with_info, without_info) = (read(&with), read(&without));
        let gps = (crate::geo::file_gps(&with.to_string_lossy()), crate::geo::file_gps(&without.to_string_lossy()));
        let _ = std::fs::remove_dir_all(&dir);
        let with_info = with_info.expect("EXIF in the copy");
        assert_eq!(with_info.camera().as_deref(), Some("Example Camera Co. EC-10"));
        assert_eq!(with_info.iso, Some(160));
        // no embedded thumbnail: it would show the picture before its edits
        assert!(with_info.thumbnail.is_none());
        assert!(without_info.is_none_or(|i| i.camera().is_none()));
        // the location follows its own setting
        assert!(gps.0.is_some() && gps.1.is_some());
    }
}
