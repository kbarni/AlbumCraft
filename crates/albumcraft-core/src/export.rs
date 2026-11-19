//! Exporting edited copies of pictures as new files (originals are never touched).

use crate::imaging::{self, EditOp};
use image::RgbaImage;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// A signature or logo laid on exported pictures (`Store.export_signature`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Signature {
    pub on: bool,
    /// A picture file (`logo`) rather than a line of text.
    pub use_logo: bool,
    pub text: String,
    pub logo: Option<String>,
    /// 0…8 = top-left … bottom-right in reading order.
    pub anchor: u8,
    /// Letter height, as a fraction of the picture's shorter side.
    pub text_size: f32,
    /// The logo's longer side, as a fraction of the picture's shorter side.
    pub logo_size: f32,
    pub opacity: f32,
    pub color: [u8; 3],
    /// A soft halo so the text reads on any picture.
    pub halo: bool,
}

impl Default for Signature {
    fn default() -> Self {
        Signature { on: false, use_logo: false, text: String::new(), logo: None, anchor: 8, text_size: 0.04, logo_size: 0.15, opacity: 0.85, color: [255, 255, 255], halo: true }
    }
}

impl Signature {
    /// Something would be drawn (switched on, with text or a logo file).
    pub fn shows(&self) -> bool {
        self.on && if self.use_logo { self.logo.is_some() } else { !self.text.trim().is_empty() }
    }
}

/// How the copies are made.
#[derive(Clone, Debug, Default)]
pub struct Options {
    /// Longest side in pixels; 0 = the picture's own size.
    pub max_dim: u32,
    pub quality: u8,
    /// Keep the original's camera information (`exifinfo::fields_to_keep`).
    pub camera_info: bool,
    /// Written into the copy's EXIF (Artist, Copyright) when not empty, replacing the original's.
    pub creator: String,
    pub copyright: String,
    pub signature: Option<Signature>,
    /// Put the picture's notes (rating, caption, tags, people) in the copy's XMP.
    pub notes: bool,
}

/// What a copy carries besides the picture: the position for its EXIF and XMP, and the notes
/// for its XMP (written when `Options.notes`).
#[derive(Clone, Debug, Default)]
pub struct Carried {
    pub gps: Option<crate::geo::LatLon>,
    pub notes: crate::xmp::XmpNotes,
}

impl Carried {
    /// The XMP packet for the copy, None when it would hold nothing.
    fn packet(&self, opts: &Options) -> Option<String> {
        if !opts.notes {
            return None;
        }
        let mut n = self.notes.clone();
        n.gps = self.gps.map(|g| [g.lat, g.lon]);
        if n.is_empty() {
            return None;
        }
        crate::xmp::merge(None, &n, false).ok()
    }
}

/// Read a logo file for `sign` (any picture format; its transparency is kept).
pub fn load_logo(path: &str) -> Result<RgbaImage, String> {
    image::open(path).map(|i| i.to_rgba8()).map_err(|e| crate::msg!("export-logo-bad", "the logo {path} can't be read ({error})", path = path, error = e).to_string())
}

/// Draw the signature on a picture: the text (with `decor`'s text drawing) or `logo`.
pub fn sign(img: RgbaImage, sig: &Signature, logo: Option<&RgbaImage>) -> RgbaImage {
    if !sig.shows() {
        return img;
    }
    if sig.use_logo {
        let mut img = img;
        if let Some(l) = logo {
            crate::decor::stamp(&mut img, l, sig.anchor, sig.logo_size, sig.opacity);
        }
        return img;
    }
    let short = img.width().min(img.height()) as f32;
    // the text step measures letters by the picture's height
    let size = sig.text_size * short / img.height().max(1) as f32;
    let t = crate::decor::TextOverlay { text: sig.text.trim().to_string(), anchor: sig.anchor, size, color: sig.color, halo: sig.halo };
    let drawn = crate::decor::Decor::Text(t).apply(img.clone());
    if sig.opacity >= 0.999 { drawn } else { crate::decor::blend(&img, &drawn, sig.opacity) }
}

/// Export one picture with its edits as `<stem>.jpg` in `dir` (or `<stem>-1.jpg`, … when taken),
/// made as `opts` says, with `gps` in its EXIF and `logo` for a logo signature (`load_logo`).
/// Never overwrites a file, and removes its own half-written file when something fails.
pub fn export_one(key: &str, edits: &[EditOp], dir: &Path, stem: &str, opts: &Options, carried: &Carried, logo: Option<&RgbaImage>) -> Result<PathBuf, String> {
    let gps = carried.gps;
    let src = imaging::load_source(key, opts.max_dim).ok_or_else(|| crate::msg!("export-unreadable", "the picture could not be read"))?;
    let mut img = imaging::apply_all(&src, edits);
    if let Some(sig) = &opts.signature {
        img = sign(img, sig, logo);
    }
    let rgb = image::DynamicImage::ImageRgba8(img).to_rgb8();
    // encoded before the file is made, so that a failure leaves nothing behind
    let mut jpeg = vec![];
    rgb.write_with_encoder(image::codecs::jpeg::JpegEncoder::new_with_quality(&mut jpeg, opts.quality.clamp(1, 100))).map_err(|e| e.to_string())?;
    let gps_fields = gps.map(crate::geo::exif_gps_fields).unwrap_or_default();
    let mut fields = if opts.camera_info { crate::exifinfo::fields_to_keep(key) } else { vec![] };
    for (tag, text) in [(exif::Tag::Artist, &opts.creator), (exif::Tag::Copyright, &opts.copyright)] {
        let text = text.trim();
        if !text.is_empty() {
            fields.retain(|f| f.tag != tag);
            fields.push(exif::Field { tag, ifd_num: exif::In::PRIMARY, value: exif::Value::Ascii(vec![text.as_bytes().to_vec()]) });
        }
    }
    fields.extend(gps_fields.iter().cloned());
    if !fields.is_empty() {
        // an original field the writer refuses must not cost the location
        jpeg = match crate::exifinfo::jpeg_with_exif(&jpeg, &fields, None) {
            Ok(j) => j,
            Err(_) if gps_fields.is_empty() => jpeg,
            Err(_) => crate::exifinfo::jpeg_with_exif(&jpeg, &gps_fields, None).map_err(|e| e.to_string())?,
        };
    }
    if let Some(packet) = carried.packet(opts) {
        // a packet too large to go in (thousands of tags) leaves the copy without it
        if let Ok(j) = crate::xmp::jpeg_with_xmp(&jpeg, &packet) {
            jpeg = j;
        }
    }
    // a free name, never one already there; written whole or not at all (`craft-atomic`)
    let mut n = 0;
    loop {
        let out = if n == 0 { dir.join(format!("{stem}.jpg")) } else { dir.join(format!("{stem}-{n}.jpg")) };
        match craft_atomic::write_new(&out, &jpeg) {
            Ok(()) => return Ok(out),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists && n < 10_000 => n += 1,
            Err(e) => return Err(e.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn small() -> Options {
        Options { max_dim: 64, quality: 80, camera_info: true, ..Default::default() }
    }

    #[test]
    fn creator_and_copyright_replace_the_originals() {
        let dir = std::env::temp_dir().join(format!("albumcraft-export-credit-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let original = dir.join("original.jpg");
        crate::exifinfo::write_demo_jpeg(&original).unwrap();
        let key = original.to_string_lossy().to_string();
        let opts = Options { max_dim: 200, quality: 80, camera_info: true, creator: "Ana Pérez".into(), copyright: "© 2026 Ana Pérez".into(), signature: None, notes: false };
        let out = export_one(&key, &[], &dir, "credited", &opts, &Carried::default(), None).unwrap();
        let file = std::fs::File::open(&out).unwrap();
        let exif = exif::Reader::new().read_from_container(&mut std::io::BufReader::new(file)).unwrap();
        let text = |t| exif.get_field(t, exif::In::PRIMARY).map(|f| f.display_value().to_string());
        let (artist, copyright, model) = (text(exif::Tag::Artist), text(exif::Tag::Copyright), text(exif::Tag::Model));
        let _ = std::fs::remove_dir_all(&dir);
        assert!(artist.is_some_and(|a| a.contains("Ana P")), "artist");
        assert!(copyright.is_some_and(|c| c.contains("2026")), "copyright");
        assert!(model.is_some(), "the camera information is still there");
    }

    #[test]
    fn a_signature_changes_its_corner_only() {
        let pic = RgbaImage::from_pixel(300, 200, image::Rgba([40, 60, 80, 255]));
        let sig = Signature { on: true, text: "Ana".into(), anchor: 8, text_size: 0.15, halo: false, opacity: 1.0, ..Default::default() };
        let signed = sign(pic.clone(), &sig, None);
        let changed = |x0: u32, y0: u32, x1: u32, y1: u32| (x0..x1).any(|x| (y0..y1).any(|y| signed.get_pixel(x, y) != pic.get_pixel(x, y)));
        assert!(changed(150, 100, 300, 200), "bottom right is signed");
        assert!(!changed(0, 0, 150, 100), "top left is untouched");
        // off, or nothing to show: the picture as it was
        assert_eq!(sign(pic.clone(), &Signature { on: false, ..sig.clone() }, None), pic);
        assert_eq!(sign(pic.clone(), &Signature { use_logo: true, logo: None, ..sig }, None), pic);
    }

    #[test]
    fn export_never_overwrites() {
        let dir = std::env::temp_dir().join(format!("albumcraft-export-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("a.jpg"), b"keep me").unwrap();
        let key = format!("{}1", imaging::SAMPLE_PREFIX);
        export_one(&key, &[], &dir, "a", &small(), &Carried::default(), None).unwrap();
        export_one(&key, &[], &dir, "a", &small(), &Carried::default(), None).unwrap();
        let unreadable = export_one("/no/such/picture.jpg", &[], &dir, "b", &small(), &Carried::default(), None);
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
        let with = export_one(&key, &[], &dir, "with", &Options { max_dim: 200, quality: 80, camera_info: true, ..Default::default() }, &Carried { gps: Some(at), ..Default::default() }, None).unwrap();
        let without = export_one(&key, &[], &dir, "without", &Options { max_dim: 200, quality: 80, ..Default::default() }, &Carried { gps: Some(at), ..Default::default() }, None).unwrap();
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

    #[test]
    fn notes_go_into_the_copys_xmp_only_when_asked() {
        let dir = std::env::temp_dir().join(format!("albumcraft-export-xmp-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let original = dir.join("original.jpg");
        crate::exifinfo::write_demo_jpeg(&original).unwrap();
        let key = original.to_string_lossy().to_string();
        let notes = crate::xmp::XmpNotes {
            rating: Some(4),
            caption: Some("Low tide".into()),
            tags: Some(vec![vec!["Places".into(), "Brittany".into()]]),
            people: Some(vec!["Ann".into()]),
            ..Default::default()
        };
        let at = crate::geo::LatLon { lat: 47.5, lon: -3.1 };
        let carried = Carried { gps: Some(at), notes };
        let opts = |notes| Options { max_dim: 100, quality: 80, camera_info: true, notes, ..Default::default() };
        let with = export_one(&key, &[], &dir, "with", &opts(true), &carried, None).unwrap();
        let without = export_one(&key, &[], &dir, "without", &opts(false), &carried, None).unwrap();
        let read = |p: &Path| crate::xmp::read_embedded(p).packet.and_then(|t| crate::xmp::parse(&t));
        let (got, none) = (read(&with), read(&without));
        let still_jpeg = image::open(&with).is_ok();
        let exif = crate::exifinfo::read(&with.to_string_lossy());
        let _ = std::fs::remove_dir_all(&dir);
        let got = got.expect("XMP in the copy");
        assert_eq!((got.rating, got.caption.as_deref()), (Some(4), Some("Low tide")));
        assert_eq!(got.tags, Some(vec![vec!["Places".to_string(), "Brittany".to_string()]]));
        assert_eq!(got.people, Some(vec!["Ann".to_string()]));
        assert!(got.gps.is_some_and(|[la, lo]| (la - 47.5).abs() < 1e-6 && (lo + 3.1).abs() < 1e-6));
        assert!(none.is_none());
        assert!(still_jpeg);
        // the EXIF segment is still read
        assert!(exif.is_some_and(|i| i.camera().is_some()));
    }
}
