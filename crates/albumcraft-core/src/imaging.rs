//! Pictures read from files, on top of the edit pipeline of `albumcraft_edit::imaging` (re-exported
//! here, so `imaging::apply_all` and `imaging::load_source` are found in one place).

pub use albumcraft_edit::imaging::*;
use image::{ImageDecoder, RgbaImage};

/// Load a picture (a file path or a built-in sample) scaled so its longest side is at most `max_dim`
/// (0 = keep the original size).
pub fn load_source(key: &str, max_dim: u32) -> Option<RgbaImage> {
    if key.starts_with(SAMPLE_PREFIX) {
        return sample_source(key, max_dim);
    }
    if crate::raw::is_raw(key) {
        if max_dim > 0
            && let Some(img) = crate::raw::preview_scaled(key, max_dim)
        {
            return Some(img);
        }
        let img = crate::raw::preview(key)?;
        let img = if max_dim > 0 && (img.width() > max_dim || img.height() > max_dim) { img.thumbnail(max_dim, max_dim) } else { img };
        return Some(img.to_rgba8());
    }
    if max_dim > 0
        && let Some(img) = load_jpeg_scaled(key, max_dim)
    {
        return Some(img);
    }
    let mut decoder = open_decoder(key)?;
    // the camera's "this way up" (EXIF orientation), applied after scaling down (cheaper)
    let orientation = decoder.orientation().unwrap_or(image::metadata::Orientation::NoTransforms);
    let mut dynimg = image::DynamicImage::from_decoder(decoder).ok()?;
    if max_dim > 0 && (dynimg.width() > max_dim || dynimg.height() > max_dim) {
        dynimg = dynimg.thumbnail(max_dim, max_dim);
    }
    dynimg.apply_orientation(orientation);
    Some(dynimg.to_rgba8())
}

fn open_decoder(path: &str) -> Option<impl image::ImageDecoder> {
    image::ImageReader::open(path).ok()?.with_guessed_format().ok()?.into_decoder().ok()
}

/// The picture's size as shown, i.e. with width and height swapped when the camera recorded it
/// turned a quarter (EXIF orientation 5–8). Reads only the file header.
pub fn oriented_dims(path: &str) -> Option<(u32, u32)> {
    use image::metadata::Orientation as O;
    if crate::raw::is_raw(path) {
        return crate::raw::preview_dims(path);
    }
    if let Some(head) = jpeg_head(path) {
        return Some(head.oriented_dims());
    }
    let mut decoder = open_decoder(path)?;
    let (w, h) = decoder.dimensions();
    let turned = matches!(
        decoder.orientation().unwrap_or(O::NoTransforms),
        O::Rotate90 | O::Rotate270 | O::Rotate90FlipH | O::Rotate270FlipH
    );
    Some(if turned { (h, w) } else { (w, h) })
}

/// What the first segments of a JPEG file say: its size, the EXIF block and the orientation.
/// Reading it takes the first tens of kilobytes, where decoding takes the whole file.
pub struct JpegHead {
    /// As stored, before the EXIF orientation.
    pub width: u32,
    pub height: u32,
    /// EXIF orientation 1–8 (1 when there is none).
    pub orientation: u8,
    /// The EXIF block (TIFF data, after "Exif\0\0"), when there is one.
    pub exif: Option<Vec<u8>>,
}

impl JpegHead {
    pub fn oriented_dims(&self) -> (u32, u32) {
        if (5..=8).contains(&self.orientation) { (self.height, self.width) } else { (self.width, self.height) }
    }

    pub fn exif(&self) -> Option<exif::Exif> {
        exif::Reader::new().read_raw(self.exif.clone()?).ok()
    }
}

/// Read a JPEG file's header (None for other files or a damaged header).
pub fn jpeg_head(path: &str) -> Option<JpegHead> {
    let file = std::fs::File::open(path).ok()?;
    jpeg_head_of(std::io::BufReader::with_capacity(64 * 1024, file))
}

/// Read a JPEG header from the start of a stream (a file, or a JPEG embedded in a raw file).
pub fn jpeg_head_of(mut r: impl std::io::Read) -> Option<JpegHead> {
    let mut two = [0u8; 2];
    r.read_exact(&mut two).ok()?;
    if two != [0xFF, 0xD8] {
        return None;
    }
    let mut exif_block: Option<Vec<u8>> = None;
    // a few hundred segments at most before the frame header
    for _ in 0..512 {
        let mut b = [0u8; 1];
        r.read_exact(&mut b).ok()?;
        if b[0] != 0xFF {
            return None;
        }
        let mut marker = 0xFF;
        while marker == 0xFF {
            r.read_exact(&mut b).ok()?;
            marker = b[0];
        }
        // markers without a length
        if marker == 0x01 || (0xD0..=0xD7).contains(&marker) {
            continue;
        }
        if marker == 0xD9 || marker == 0xDA {
            return None; // end of image or scan data before any frame header
        }
        r.read_exact(&mut two).ok()?;
        let len = u16::from_be_bytes(two) as usize;
        let body_len = len.checked_sub(2)?;
        // other frame types (lossless, as raw sensor data uses; arithmetic; hierarchical) are
        // not pictures the decoders here can show
        if matches!(marker, 0xC3 | 0xC5..=0xC7 | 0xC9..=0xCB | 0xCD..=0xCF) {
            return None;
        }
        if matches!(marker, 0xC0..=0xC2) {
            let mut sof = [0u8; 5];
            r.read_exact(&mut sof).ok()?;
            let height = u16::from_be_bytes([sof[1], sof[2]]) as u32;
            let width = u16::from_be_bytes([sof[3], sof[4]]) as u32;
            if width == 0 || height == 0 {
                return None;
            }
            let exif = exif_block.and_then(|b| b.get(6..).map(|t| t.to_vec()));
            let orientation = exif
                .as_ref()
                .and_then(|t| exif::Reader::new().read_raw(t.clone()).ok())
                .and_then(|e| e.get_field(exif::Tag::Orientation, exif::In::PRIMARY).and_then(|f| f.value.get_uint(0)))
                .and_then(|o| u8::try_from(o).ok())
                .filter(|o| (1..=8).contains(o))
                .unwrap_or(1);
            return Some(JpegHead { width, height, orientation, exif });
        }
        let mut body = vec![0u8; body_len];
        r.read_exact(&mut body).ok()?;
        if marker == 0xE1 && exif_block.is_none() && body.starts_with(b"Exif\0\0") {
            exif_block = Some(body);
        }
    }
    None
}

/// The camera's own preview embedded in a JPEG file, turned the way up, when its longest side is
/// at least `min_side` and it has the main picture's shape (so it shows the same picture, not a
/// letterboxed or out-of-date one). Much cheaper than decoding the picture.
pub fn embedded_preview(path: &str, min_side: u32) -> Option<RgbaImage> {
    let head = jpeg_head(path)?;
    let exif = head.exif()?;
    use exif::{In, Tag};
    let off = exif.get_field(Tag::JPEGInterchangeFormat, In::THUMBNAIL)?.value.get_uint(0)? as usize;
    let len = exif.get_field(Tag::JPEGInterchangeFormatLength, In::THUMBNAIL)?.value.get_uint(0)? as usize;
    let bytes = exif.buf().get(off..off.checked_add(len)?)?;
    let thumb = image::load_from_memory_with_format(bytes, image::ImageFormat::Jpeg).ok()?;
    if thumb.width().max(thumb.height()) < min_side {
        return None;
    }
    let main = head.width as f32 / head.height as f32;
    let small = thumb.width() as f32 / thumb.height().max(1) as f32;
    if (main / small - 1.0).abs() > 0.02 {
        return None;
    }
    let mut thumb = if thumb.width().max(thumb.height()) > min_side { thumb.thumbnail(min_side, min_side) } else { thumb };
    if let Some(o) = image::metadata::Orientation::from_exif(head.orientation) {
        thumb.apply_orientation(o);
    }
    Some(thumb.to_rgba8())
}

/// A JPEG at most `max_dim` pixels on its longest side, decoded at 1/2, 1/4 or 1/8 of its size
/// (much cheaper than decoding it whole and scaling down). None when it is no more than twice
/// `max_dim` (nothing to gain), not a plain RGB or gray JPEG, or can't be read.
fn load_jpeg_scaled(path: &str, max_dim: u32) -> Option<RgbaImage> {
    let head = jpeg_head(path)?;
    let file = std::fs::File::open(path).ok()?;
    let orientation = head.orientation;
    decode_jpeg_scaled(std::io::BufReader::with_capacity(256 * 1024, file), &head, max_dim, orientation)
}

/// Decode a JPEG stream (whose header is `head`) at a reduced scale, then turn it by the EXIF
/// `orientation` (1–8). See `load_jpeg_scaled`.
pub fn decode_jpeg_scaled(stream: impl std::io::Read, head: &JpegHead, max_dim: u32, orientation: u8) -> Option<RgbaImage> {
    if max_dim == 0 || head.width.max(head.height) < max_dim.saturating_mul(2) {
        return None;
    }
    let mut d = jpeg_decoder::Decoder::new(stream);
    // the smallest scale that still has at least max_dim on the long side
    let (w, h) = (head.width as f32, head.height as f32);
    let k = max_dim as f32 / w.max(h);
    let (sw, sh) = d.scale(((w * k).ceil() as u16).max(1), ((h * k).ceil() as u16).max(1)).ok()?;
    let pixels = d.decode().ok()?;
    let info = d.info()?;
    let (sw, sh) = (sw as u32, sh as u32);
    let rgba = match info.pixel_format {
        jpeg_decoder::PixelFormat::RGB24 => {
            let mut out = Vec::with_capacity(pixels.len() / 3 * 4);
            for p in pixels.as_chunks::<3>().0 {
                out.extend_from_slice(&[p[0], p[1], p[2], 255]);
            }
            RgbaImage::from_raw(sw, sh, out)?
        }
        jpeg_decoder::PixelFormat::L8 => {
            RgbaImage::from_raw(sw, sh, pixels.iter().flat_map(|&g| [g, g, g, 255]).collect())?
        }
        _ => return None,
    };
    let mut img = image::DynamicImage::ImageRgba8(rgba);
    if img.width() > max_dim || img.height() > max_dim {
        img = img.thumbnail(max_dim, max_dim);
    }
    if let Some(o) = image::metadata::Orientation::from_exif(orientation) {
        img.apply_orientation(o);
    }
    Some(img.to_rgba8())
}

/// A thumbnail of at most `dim` pixels: from the camera's embedded preview when it is big enough,
/// else from the picture itself.
pub fn load_thumbnail(key: &str, dim: u32) -> Option<RgbaImage> {
    if !key.starts_with(SAMPLE_PREFIX)
        && !crate::raw::is_raw(key)
        && let Some(t) = embedded_preview(key, dim)
    {
        return Some(t);
    }
    load_source(key, dim)
}

#[cfg(test)]
mod orientation_tests {
    use super::*;
    use image::Rgba;

    #[test]
    fn camera_orientation_is_applied() {
        // 40 x 20, left half red, right half blue; EXIF orientation 6 = turn 90° clockwise to view
        let img = RgbaImage::from_fn(40, 20, |x, _| if x < 20 { Rgba([255, 0, 0, 255]) } else { Rgba([0, 0, 255, 255]) });
        let rgb = image::DynamicImage::ImageRgba8(img).to_rgb8();
        let mut jpeg = Vec::new();
        rgb.write_with_encoder(image::codecs::jpeg::JpegEncoder::new_with_quality(&mut jpeg, 95)).unwrap();
        let field = exif::Field { tag: exif::Tag::Orientation, ifd_num: exif::In::PRIMARY, value: exif::Value::Short(vec![6]) };
        let file = crate::exifinfo::jpeg_with_exif(&jpeg, &[field], None).unwrap();
        let path = std::env::temp_dir().join(format!("albumcraft-orient-{}.jpg", std::process::id()));
        std::fs::write(&path, file).unwrap();
        let key = path.to_string_lossy().to_string();
        let loaded = load_source(&key, 0).unwrap();
        let dims = oriented_dims(&key);
        let small = load_source(&key, 10).unwrap();
        let _ = std::fs::remove_file(&path);
        assert_eq!((loaded.width(), loaded.height()), (20, 40));
        assert_eq!(dims, Some((20, 40)));
        assert_eq!((small.width(), small.height()), (5, 10));
        // turned clockwise: the left (red) half is now on top
        assert!(loaded.get_pixel(10, 5)[0] > 200 && loaded.get_pixel(10, 35)[2] > 200);
    }
}

