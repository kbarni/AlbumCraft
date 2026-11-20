//! HEIF photos: HEIC (HEVC pictures, as iPhones and Macs take them) and AVIF (AV1 pictures).
//!
//! - HEIC: the suite's `craft-heif` (heic-rs, pure Rust; feature `heif`). Thumbnails come from
//!   the thumbnail item the file carries when it is big enough (iPhones store one of about
//!   320 × 240), much cheaper than decoding the photo.
//! - AVIF: `craft-image-codecs`' AV1 decoder (`craft-video`, pure Rust; feature `avif`).
//! - Their EXIF and XMP: `craft_heif::items`, from the box structure alone, with or without the
//!   features (`filemeta::blocks`).
//!
//! The way up is the container's (`irot` / `imir`, and a `clap` crop), applied by the decoders;
//! the EXIF orientation only mirrors it and is never applied again. Without a feature, files of
//! that kind are left out of the library (`is_heif_extension`). Read-only: XMP goes to a sidecar.

use image::RgbaImage;
use std::path::Path;

/// HEIC (HEVC) photos.
pub const HEIC_EXTENSIONS: [&str; 3] = ["heic", "heif", "hif"];
/// AVIF (AV1) photos.
pub const AVIF_EXTENSIONS: [&str; 1] = ["avif"];

/// Largest file read (a photo is never worth more memory than that).
const MAX_FILE: u64 = 512 << 20;
/// Largest picture decoded, in pixels (a 16-bit RGBA buffer of it is 2 GiB).
#[cfg(any(feature = "heif", feature = "avif"))]
const MAX_PIXELS: u64 = 1 << 28;

/// Whether a file with this (lower-case) extension can be shown in this build.
pub fn is_heif_extension(ext: &str) -> bool {
    (cfg!(feature = "heif") && HEIC_EXTENSIONS.contains(&ext)) || (cfg!(feature = "avif") && AVIF_EXTENSIONS.contains(&ext))
}

/// Whether this file is a HEIF or AVIF photo this build shows (by its extension).
pub fn is_heif(path: &str) -> bool {
    Path::new(path).extension().and_then(|e| e.to_str()).is_some_and(|e| is_heif_extension(&e.to_ascii_lowercase()))
}

fn read_file(path: &str) -> Option<Vec<u8>> {
    let f = std::fs::File::open(path).ok()?;
    if f.metadata().ok()?.len() > MAX_FILE {
        return None;
    }
    let mut bytes = vec![];
    std::io::Read::read_to_end(&mut std::io::Read::take(f, MAX_FILE), &mut bytes).ok()?;
    Some(bytes)
}

/// Does the `ftyp` box name an AVIF brand (AV1 pictures, whatever the extension says)?
fn is_avif(bytes: &[u8]) -> bool {
    if bytes.get(4..8) != Some(b"ftyp".as_slice()) {
        return false;
    }
    let size = bytes.get(..4).map_or(0, |s| u32::from_be_bytes([s[0], s[1], s[2], s[3]]) as usize);
    let end = size.clamp(16, 256).min(bytes.len());
    // the major brand at 8, the compatible brands from 16 (12 is the minor version)
    std::iter::once(8).chain((16..end).step_by(4)).filter_map(|at| bytes.get(at..at + 4)).any(|b| b == b"avif" || b == b"avis")
}

/// The photo scaled so its longest side is at most `max_dim` (0 = keep its size), turned the
/// way up. None when it can't be read or this build has no decoder for it.
pub fn load(path: &str, max_dim: u32) -> Option<RgbaImage> {
    let bytes = read_file(path)?;
    let img = if is_avif(&bytes) { avif::decode(&bytes)? } else { heic::decode(&bytes, max_dim)? };
    Some(fit(img, max_dim))
}

/// The photo's size as shown (after the container's rotation and crop), from the boxes alone.
pub fn dims(path: &str) -> Option<(u32, u32)> {
    let bytes = read_file(path)?;
    if is_avif(&bytes) { avif::dims(&bytes) } else { heic::dims(&bytes) }
}

/// The EXIF and XMP of a HEIF or AVIF file whose first bytes are `head` (read whole from `path`
/// when the items lie past them). No picture is decoded.
pub fn embedded(head: &[u8], path: &Path) -> craft_metadata::Embedded {
    let of = |bytes: &[u8]| craft_heif::items::read(bytes).ok();
    let mut found = of(head);
    let whole = std::fs::metadata(path).is_ok_and(|m| m.len() > head.len() as u64);
    if whole && found.as_ref().is_none_or(|(exif, xmp)| exif.is_none() || xmp.is_none()) {
        // the items are usually in the `mdat` after the picture data's start: read it all
        if let Some(all) = path.to_str().and_then(read_file)
            && let Some(more) = of(&all)
        {
            found = Some(more);
        }
    }
    let (exif, xmp) = found.unwrap_or_default();
    craft_metadata::Embedded { exif, xmp, ..Default::default() }
}

fn fit(img: RgbaImage, max_dim: u32) -> RgbaImage {
    if max_dim > 0 && (img.width() > max_dim || img.height() > max_dim) {
        image::DynamicImage::ImageRgba8(img).thumbnail(max_dim, max_dim).to_rgba8()
    } else {
        img
    }
}

/// 16-bit samples (0–65535) to 8 bits, rounded.
#[cfg_attr(not(any(feature = "heif", test)), allow(dead_code))]
fn to_u8(v: u16) -> u8 {
    ((u32::from(v) * 255 + 32767) / 65535) as u8
}

#[cfg(feature = "heif")]
mod heic {
    use super::*;

    fn options() -> craft_heif::Options {
        craft_heif::Options { max_pixels: MAX_PIXELS, ..Default::default() }
    }

    pub(super) fn decode(bytes: &[u8], max_dim: u32) -> Option<RgbaImage> {
        let info = craft_heif::probe(bytes).ok()?;
        // the file's own thumbnail when it is big enough and has the photo's shape
        if max_dim > 0
            && let Ok(Some(t)) = craft_heif::decode_thumbnail(bytes, max_dim, &options())
            && same_shape((t.width, t.height), (info.width, info.height))
            && let Some(img) = rgba(&t)
        {
            return Some(img);
        }
        rgba(&craft_heif::decode(bytes, &options()).ok()?)
    }

    pub(super) fn dims(bytes: &[u8]) -> Option<(u32, u32)> {
        let info = craft_heif::probe(bytes).ok()?;
        Some((info.width, info.height))
    }

    fn same_shape(a: (u32, u32), b: (u32, u32)) -> bool {
        let r = |(w, h): (u32, u32)| w as f32 / h.max(1) as f32;
        (r(a) / r(b) - 1.0).abs() <= 0.02
    }

    fn rgba(d: &craft_heif::Decoded) -> Option<RgbaImage> {
        let channels = if d.has_alpha { 4 } else { 3 };
        let mut out = Vec::with_capacity(d.width as usize * d.height as usize * 4);
        for p in d.data.chunks_exact(channels) {
            out.extend_from_slice(&[to_u8(p[0]), to_u8(p[1]), to_u8(p[2]), if d.has_alpha { to_u8(p[3]) } else { 255 }]);
        }
        RgbaImage::from_raw(d.width, d.height, out)
    }
}

#[cfg(not(feature = "heif"))]
mod heic {
    use super::*;

    pub(super) fn decode(_: &[u8], _: u32) -> Option<RgbaImage> {
        None
    }

    pub(super) fn dims(_: &[u8]) -> Option<(u32, u32)> {
        None
    }
}

#[cfg(feature = "avif")]
mod avif {
    use super::*;
    use craft_image_codecs::{DecodeOptions, Format, Limits, SampleType};

    pub(super) fn decode(bytes: &[u8]) -> Option<RgbaImage> {
        let limits = Limits { max_pixels: MAX_PIXELS, max_alloc: 4 << 30, ..Limits::default() };
        let img = craft_image_codecs::decode_as_with(Format::Avif, bytes, &DecodeOptions { limits, keep_orientation: false }).ok()?;
        let (w, h) = img.dimensions();
        let data = if img.sample_type() == SampleType::F32 {
            // PQ and HLG photos open as linear light: shown as sRGB, highlights clipped
            let lin = img.to_rgba_f32(&Limits::default()).ok()?;
            lin.as_chunks::<4>().0.iter().flat_map(|p| [srgb(p[0]), srgb(p[1]), srgb(p[2]), (p[3].clamp(0.0, 1.0) * 255.0 + 0.5) as u8]).collect()
        } else {
            img.to_rgba8()
        };
        RgbaImage::from_raw(w, h, data)
    }

    pub(super) fn dims(bytes: &[u8]) -> Option<(u32, u32)> {
        craft_image_codecs::probe_size(bytes).ok()
    }

    fn srgb(v: f32) -> u8 {
        let v = v.clamp(0.0, 1.0);
        let e = if v <= 0.003_130_8 { v * 12.92 } else { 1.055 * v.powf(1.0 / 2.4) - 0.055 };
        (e * 255.0 + 0.5) as u8
    }
}

#[cfg(not(feature = "avif"))]
mod avif {
    use super::*;

    pub(super) fn decode(_: &[u8]) -> Option<RgbaImage> {
        None
    }

    pub(super) fn dims(_: &[u8]) -> Option<(u32, u32)> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(any(feature = "heif", feature = "avif"))]
    fn temp(name: &str, bytes: &[u8]) -> std::path::PathBuf {
        let path = std::env::temp_dir().join(format!("albumcraft-heif-{}-{name}", std::process::id()));
        std::fs::write(&path, bytes).unwrap();
        path
    }

    #[test]
    fn extensions_and_brands() {
        assert_eq!(is_heif("/p/IMG_0001.HEIC"), cfg!(feature = "heif"));
        assert_eq!(is_heif("/p/a.avif"), cfg!(feature = "avif"));
        assert!(!is_heif("/p/a.jpg"));
        assert!(is_avif(b"\0\0\0\x1cftypavif\0\0\0\0avifmif1miaf"));
        assert!(is_avif(b"\0\0\0\x18ftypmif1\0\0\0\0mif1avif"));
        assert!(!is_avif(b"\0\0\0\x18ftypheic\0\0\0\0mif1heic"));
        assert_eq!(to_u8(0), 0);
        assert_eq!(to_u8(65535), 255);
        assert_eq!(to_u8(32896), 128);
    }

    /// A 64 × 32 HEIC, left half red and right half blue, turned a quarter (`irot` 1: counter-
    /// clockwise), with EXIF and a thumbnail item.
    #[cfg(feature = "heif")]
    fn heic_file() -> Vec<u8> {
        use craft_heif::testdata::{Prop, Spec, build};
        // BT.601 full range: red and blue
        let sample = |x: u32, _: u32| if x < 32 { [76, 85, 255] } else { [29, 255, 107] };
        let small = |x: u32, _: u32| if x < 16 { [76, 85, 255] } else { [29, 255, 107] };
        let exif = crate::filemeta::write_exif_block(
            &craft_tiff::IfdBuilder::new().with(craft_tiff::tags::MAKE, craft_tiff::Value::Ascii("Phone".into())),
            None,
        )
        .unwrap();
        build(&Spec {
            width: 64,
            height: 32,
            bit_depth: 8,
            sample: &sample,
            props: vec![Prop::Nclx(1, 13, 6, true), Prop::Irot(1)],
            exif: Some(exif),
            alpha: None,
            thumbnail: Some(&small),
            ignored_aux: vec![],
            stereo: false,
            alpha_tile_repeats: 0,
            thumbnail_tile_repeats: 0,
        })
    }

    #[cfg(feature = "heif")]
    #[test]
    fn heic_decodes_turned_with_metadata() {
        let path = temp("a.heic", &heic_file());
        let key = path.to_string_lossy().to_string();
        let full = load(&key, 0).unwrap();
        let thumb = load(&key, 20).unwrap();
        let shown = dims(&key);
        let meta = crate::filemeta::metadata(&path);
        let _ = std::fs::remove_file(&path);
        assert_eq!((full.width(), full.height()), (32, 64));
        assert_eq!(shown, Some((32, 64)));
        assert_eq!((thumb.width(), thumb.height()), (10, 20));
        // turned counter-clockwise: the right (blue) half is now on top
        let (top, bottom) = (full.get_pixel(16, 8), full.get_pixel(16, 56));
        assert!(top[2] > 200 && top[0] < 60, "{top:?}");
        assert!(bottom[0] > 200 && bottom[2] < 60, "{bottom:?}");
        assert_eq!(meta.make.as_deref(), Some("Phone"));
    }

    #[cfg(feature = "avif")]
    #[test]
    fn avif_decodes() {
        // 33 × 17, four quadrants (red, green, blue, near-white), lossless (craft-image-codecs' fixture)
        let path = temp("a.avif", include_bytes!("../testdata/quad-444-gbr-lossless.avif"));
        let key = path.to_string_lossy().to_string();
        let img = load(&key, 0).unwrap();
        let shown = dims(&key);
        let small = load(&key, 16).unwrap();
        let _ = std::fs::remove_file(&path);
        assert_eq!((img.width(), img.height()), (33, 17));
        assert_eq!(shown, Some((33, 17)));
        assert_eq!(small.width(), 16);
        let p = img.get_pixel(2, 2);
        assert!(p[0] > 200 && p[1] < 60 && p[2] < 60, "{p:?}");
    }
}
