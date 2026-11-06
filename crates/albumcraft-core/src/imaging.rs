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
        let img = crate::raw::preview(key)?;
        let img = if max_dim > 0 && (img.width() > max_dim || img.height() > max_dim) { img.thumbnail(max_dim, max_dim) } else { img };
        return Some(img.to_rgba8());
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
    let mut decoder = open_decoder(path)?;
    let (w, h) = decoder.dimensions();
    let turned = matches!(
        decoder.orientation().unwrap_or(O::NoTransforms),
        O::Rotate90 | O::Rotate270 | O::Rotate90FlipH | O::Rotate270FlipH
    );
    Some(if turned { (h, w) } else { (w, h) })
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

