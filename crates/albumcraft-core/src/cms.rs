//! Colour management, with the suite's `craft-cms` (pure-Rust ICC v2/v4).
//!
//! AlbumCraft works in sRGB: every picture is turned into sRGB as it is decoded, from the ICC
//! profile it carries (a Display P3 photo from a phone, an Adobe RGB one from a camera) or the
//! colour description of a HEIF / AVIF file (`nclx`). Thumbnails, the viewer, edits, the AI
//! features and every copy made from the picture then show the right colours. Exported JPEGs
//! say so with an sRGB profile (`srgb_icc`).
//!
//! Left as they are: pictures without a profile (taken as sRGB), sRGB ones, and profiles that
//! can't apply to the decoded RGB samples (CMYK, Lab, broken ones). Gray profiles are applied to
//! the gray samples, spread over R, G and B.

use craft_cms::{Builtin, ColorSpace, Profile, Transform, TransformOptions};
use image::RgbaImage;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock, PoisonError};

/// Where a picture's colours are described.
#[derive(Debug, Clone, Copy)]
pub enum Source<'a> {
    /// An embedded ICC profile.
    Icc(&'a [u8]),
    /// An ITU-T H.273 colour description (HEIF / AVIF `nclx`): colour primaries and transfer
    /// characteristics. `linear`: the samples were already decoded to linear light (PQ, HLG).
    Cicp { primaries: u16, transfer: u16, linear: bool },
}

/// The transform from a source to sRGB, None when there is nothing to do (sRGB already) or
/// nothing that can be done. Built once per profile (a few milliseconds) and kept.
pub fn to_srgb(source: Source) -> Option<Arc<Transform>> {
    type Cache = HashMap<Vec<u8>, Option<Arc<Transform>>>;
    static CACHE: OnceLock<Mutex<Cache>> = OnceLock::new();
    let key = match source {
        Source::Icc(bytes) => bytes.to_vec(),
        Source::Cicp { primaries, transfer, linear } => [b"cicp".as_slice(), &primaries.to_be_bytes(), &transfer.to_be_bytes(), &[linear as u8]].concat(),
    };
    let cache = CACHE.get_or_init(Default::default);
    if let Some(t) = cache.lock().unwrap_or_else(PoisonError::into_inner).get(&key) {
        return t.clone();
    }
    let t = build(source).map(Arc::new);
    let mut c = cache.lock().unwrap_or_else(PoisonError::into_inner);
    if c.len() >= 32 {
        c.clear();
    }
    c.insert(key, t.clone());
    t
}

fn build(source: Source) -> Option<Transform> {
    let profile = match source {
        Source::Icc(bytes) => {
            let p = Profile::parse(bytes).ok()?;
            match p.color_space {
                ColorSpace::Rgb => p,
                ColorSpace::Gray => p.gray_as_rgb()?,
                _ => return None,
            }
        }
        Source::Cicp { primaries, transfer, linear } => craft_cms::builtin::cicp_rgb(primaries, transfer, linear)?,
    };
    let srgb = Builtin::Srgb.profile();
    if profile.same_colors(srgb) {
        return None;
    }
    Transform::with_options(&profile, srgb, TransformOptions::default()).ok()
}

/// Turn an RGBA picture's colours into sRGB in place (alpha kept).
pub fn convert_rgba8(img: &mut RgbaImage, t: &Transform) {
    let src = img.as_raw().clone();
    // the buffer is whole RGBA pixels, so the transform can't refuse it
    let _ = t.convert_u8(&src, 4, img, 4, true);
}

/// `convert_rgba8` when the source asks for it.
pub fn rgba8_to_srgb(img: &mut RgbaImage, source: Option<Source>) {
    if let Some(t) = source.and_then(to_srgb) {
        convert_rgba8(img, &t);
    }
}

/// Turn interleaved 16-bit RGB or RGBA samples (`channels` 3 or 4) into sRGB in place.
pub fn rgb16_to_srgb(data: &mut [u16], channels: usize, source: Option<Source>) {
    if let Some(t) = source.and_then(to_srgb) {
        let src = data.to_vec();
        let _ = t.convert_u16(&src, channels, data, channels, true);
    }
}

/// Turn interleaved linear-light float RGBA samples (as AVIF decodes PQ and HLG photos) into
/// linear sRGB primaries in place. Values above 1 are kept, for the caller to clip.
pub fn linear_rgba_f32_to_srgb(data: &mut [f32], primaries: u16) {
    if primaries == 1 {
        return; // BT.709 primaries are sRGB's
    }
    let Some(src) = craft_cms::builtin::cicp_rgb(primaries, 8, true) else { return };
    let dst = Builtin::LinearSrgb.profile();
    if let Ok(t) = Transform::with_options(&src, dst, TransformOptions { bpc: false, ..Default::default() }) {
        let _ = t.apply(data, 4);
    }
}

/// The sRGB profile, to tag the JPEGs AlbumCraft writes.
pub fn srgb_icc() -> Vec<u8> {
    Builtin::Srgb.profile().to_bytes().to_vec()
}

/// Reassemble an ICC profile from a JPEG's APP2 `ICC_PROFILE` segment bodies (each: the
/// signature, its 1-based sequence number, the count, then a slice of the profile). None when
/// chunks are missing, repeated or disagree on the count.
pub fn icc_from_jpeg_chunks(chunks: &[Vec<u8>]) -> Option<Vec<u8>> {
    const SIG: &[u8] = b"ICC_PROFILE\0";
    let mut parts: Vec<(u8, &[u8])> = vec![];
    let mut count = None;
    for c in chunks {
        let rest = c.strip_prefix(SIG)?;
        let (&[seq, n], data) = rest.split_first_chunk::<2>()?;
        if seq == 0 || n == 0 || *count.get_or_insert(n) != n {
            return None;
        }
        parts.push((seq, data));
    }
    let n = count? as usize;
    parts.sort_by_key(|p| p.0);
    if parts.len() != n || parts.iter().enumerate().any(|(i, p)| p.0 as usize != i + 1) {
        return None;
    }
    Some(parts.into_iter().flat_map(|p| p.1.iter().copied()).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p3() -> Vec<u8> {
        Builtin::DisplayP3.profile().to_bytes().to_vec()
    }

    #[test]
    fn srgb_and_unusable_profiles_are_left_alone() {
        assert!(to_srgb(Source::Icc(&srgb_icc())).is_none());
        assert!(to_srgb(Source::Icc(b"not a profile")).is_none());
        let cmyk = Builtin::CoatedCmyk.profile().to_bytes();
        assert!(to_srgb(Source::Icc(&cmyk)).is_none());
        assert!(to_srgb(Source::Cicp { primaries: 1, transfer: 13, linear: false }).is_none());
    }

    #[test]
    fn display_p3_red_is_turned_into_srgb() {
        // P3's pure red lies outside sRGB: clipped to sRGB red; a P3 mid gray stays gray
        let mut img = RgbaImage::from_raw(2, 1, vec![255, 0, 0, 255, 128, 128, 128, 7]).unwrap();
        rgba8_to_srgb(&mut img, Some(Source::Icc(&p3())));
        assert_eq!(&img.as_raw()[..4], &[255, 0, 0, 255]);
        let g = &img.as_raw()[4..];
        assert!(g[0].abs_diff(128) <= 1 && g[0] == g[1] && g[1] == g[2] && g[3] == 7, "{g:?}");
        // a P3 orange less saturated than sRGB's edge keeps its hue but gains red
        let mut o = RgbaImage::from_raw(1, 1, vec![200, 120, 60, 255]).unwrap();
        rgba8_to_srgb(&mut o, Some(Source::Cicp { primaries: 12, transfer: 13, linear: false }));
        let o = o.as_raw();
        assert!(o[0] > 210 && o[1] < 125 && o[2] < 60, "{o:?}");
    }

    #[test]
    fn sixteen_bit_samples_too() {
        let mut d = vec![0u16, 65535, 0];
        rgb16_to_srgb(&mut d, 3, Some(Source::Icc(&p3())));
        assert!(d[0] == 0 && d[1] == 65535 && d[2] == 0, "{d:?}");
        let mut d = vec![40000u16, 20000, 10000, 65535];
        rgb16_to_srgb(&mut d, 4, Some(Source::Icc(&p3())));
        assert!(d[0] > 42000 && d[3] == 65535, "{d:?}");
    }

    #[test]
    fn jpeg_icc_chunks_are_put_back_in_order() {
        let chunk = |seq: u8, n: u8, data: &[u8]| [b"ICC_PROFILE\0".as_slice(), &[seq, n], data].concat();
        let got = icc_from_jpeg_chunks(&[chunk(2, 2, b"world"), chunk(1, 2, b"hello ")]);
        assert_eq!(got.as_deref(), Some(b"hello world".as_slice()));
        assert!(icc_from_jpeg_chunks(&[chunk(1, 2, b"a")]).is_none());
        assert!(icc_from_jpeg_chunks(&[chunk(1, 1, b"a"), chunk(1, 1, b"a")]).is_none());
        assert!(icc_from_jpeg_chunks(&[]).is_none());
    }
}
