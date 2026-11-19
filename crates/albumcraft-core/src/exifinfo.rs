//! Camera information (EXIF) read from a photo, formatted for the Properties panel
//! ("Exposure time 0.0015625 s (1/640)", "F-number f/5", "Focal length 11mm", ...), and the EXIF
//! kept in exported copies. Read with `craft-metadata` (the model, the tag list) and
//! `craft-tiff` (the few tags the model leaves out, writing).

use craft_tiff::{IfdBuilder, Tiff, Value, tags};
use image::RgbaImage;

/// Tags the model doesn't hold, read from the EXIF directories.
const SUBJECT_DISTANCE: u16 = 37382;
const LIGHT_SOURCE: u16 = 37384;
const COLOR_SPACE: u16 = 40961;

#[derive(Default)]
pub struct CameraInfo {
    pub make: Option<String>,
    pub model: Option<String>,
    pub lens: Option<String>,
    pub date_taken: Option<String>,
    pub date_digitized: Option<String>,
    pub date_modified: Option<String>,
    pub flash: Option<String>,
    pub focal_mm: Option<f64>,
    pub focal_35mm: Option<u32>,
    pub exposure_s: Option<f64>,
    pub fnumber: Option<f64>,
    pub iso: Option<u32>,
    pub exposure_bias: Option<f64>,
    pub metering: Option<String>,
    pub program: Option<String>,
    pub white_balance: Option<String>,
    /// metres ("Subject Distance", shown as "%sm")
    pub subject_distance: Option<f64>,
    pub color_space: Option<String>,
    pub software: Option<String>,
    pub orientation: Option<u32>,
    /// The small preview picture the camera stores inside the file.
    pub thumbnail: Option<RgbaImage>,
    /// Every EXIF entry: (name, value), in file order.
    pub all: Vec<(String, String)>,
}

/// Plain number without trailing zeros: 5.0 -> "5", 5.6 -> "5.6".
pub fn trim_num(x: f64, decimals: usize) -> String {
    let s = format!("{x:.decimals$}");
    crate::locale::decimal(&if s.contains('.') { s.trim_end_matches('0').trim_end_matches('.').to_string() } else { s })
}

impl CameraInfo {
    /// Properties panel: "%ss", and when the time is under 0.5 s " (1/%d)" with N = floor(1/t + 0.5).
    pub fn exposure_long(&self) -> Option<String> {
        let s = self.exposure_s?;
        let mut out = format!("{s}s");
        if s > 0.0 && s < 0.5 {
            out.push_str(&format!(" (1/{})", (1.0 / s + 0.5).floor() as u64));
        }
        Some(out)
    }
    /// Camera box under the histogram: "1/%ds" up to 0.5 s, otherwise "%2.1fs".
    pub fn exposure_short(&self) -> Option<String> {
        let s = self.exposure_s?;
        Some(if s > 0.0 && s <= 0.5 { format!("1/{}s", (1.0 / s + 0.5).floor() as u64) } else { format!("{s:2.1}s") })
    }
    pub fn camera(&self) -> Option<String> {
        match (&self.make, &self.model) {
            (Some(a), Some(b)) if b.starts_with(a.as_str()) => Some(b.clone()),
            (Some(a), Some(b)) => Some(format!("{a} {b}")),
            (a, b) => a.clone().or(b.clone()),
        }
    }
    pub fn has_camera_data(&self) -> bool {
        self.make.is_some() || self.model.is_some() || self.exposure_s.is_some() || self.fnumber.is_some() || self.iso.is_some()
    }
}

/// Flash: only whether it fired ("Used" / "Not Used").
fn flash_text(v: u32) -> String {
    if v & 1 == 1 { "Used".into() } else { "Not Used".into() }
}

/// Metering mode names.
fn metering_text(v: u32) -> String {
    match v {
        1 => "Average",
        2 => "Center Weight",
        3 => "Spot",
        4 => "Multi-spot",
        5 => "Matrix",
        6 => "Partial",
        255 => "Other",
        _ => "Unknown",
    }
    .into()
}

/// Exposure program names.
fn program_text(v: u32) -> String {
    match v {
        1 => "Manual",
        2 => "Program",
        3 => "Aperture Priority",
        4 => "Shutter Priority",
        5 => "Creative Program",
        6 => "Action Program",
        7 => "Portrait Mode",
        8 => "Landscape Mode",
        _ => "Not Defined",
    }
    .into()
}

/// Light source, shown under "White Balance" (values 0…24; 255 = "Other"; anything else is shown as
/// its number).
fn light_source_text(v: u32) -> String {
    let s = match v {
        0 => "Unknown",
        1 => "Sunny",
        2 => "Fluorescent",
        3 => "Incandescent",
        4 => "Flash",
        9 => "Fine Weather",
        10 => "Cloudy",
        11 => "Shade",
        12 => "Daylight Flourescent",
        13 => "Day White Flourescent",
        14 => "Cool White Flourescent",
        15 => "White Flourescent",
        17 => "Standard Light A",
        18 => "Standard Light B",
        19 => "Standard Light C",
        20 => "D55",
        21 => "D65",
        22 => "D75",
        23 => "D50",
        24 => "ISO Studio Tungsten",
        255 => "Other",
        _ => return v.to_string(),
    };
    s.into()
}

pub fn read(path: &str) -> Option<CameraInfo> {
    let exif = crate::filemeta::blocks(std::path::Path::new(path)).exif?;
    let m = craft_metadata::try_read_exif(&exif).ok()?;
    let tiff = Tiff::parse(crate::filemeta::tiff_of(&exif)).ok();
    // IFD0, then the EXIF directory
    let text = |tag: u16| tiff.as_ref()?.find(tag)?.value.as_str().map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
    let number = |tag: u16| tiff.as_ref()?.find(tag)?.value.get_f64(0).filter(|v| v.is_finite());
    let mut info = CameraInfo {
        make: m.make.clone(),
        model: m.model.clone(),
        lens: m.lens_model.clone(),
        date_taken: text(tags::DATE_TIME_ORIGINAL),
        date_digitized: text(tags::DATE_TIME_DIGITIZED),
        date_modified: text(tags::DATE_TIME),
        flash: m.flash.map(|f| flash_text(f.raw as u32)),
        focal_mm: m.focal_length,
        focal_35mm: m.focal_length_35mm.map(|f| f.round() as u32).filter(|&v| v > 0),
        exposure_s: m.exposure_time,
        fnumber: m.f_number,
        iso: m.iso,
        exposure_bias: m.exposure_bias,
        metering: m.metering_mode.map(|v| metering_text(v as u32)),
        program: m.exposure_program.map(|v| program_text(v as u32)),
        white_balance: number(LIGHT_SOURCE)
            .map(|v| light_source_text(v as u32))
            .or_else(|| m.white_balance.map(|v| if v == 1 { "Manual".into() } else { "Auto".into() })),
        subject_distance: number(SUBJECT_DISTANCE).filter(|d| *d > 0.0),
        color_space: number(COLOR_SPACE).map(|v| if v == 1.0 { "sRGB".into() } else { "Uncalibrated".into() }),
        software: m.software.clone(),
        orientation: m.orientation.map(|o| o.to_exif() as u32),
        ..Default::default()
    };
    // the camera's own small preview (IFD1)
    if let Some(bytes) = crate::filemeta::exif_thumbnail(&exif) {
        let orientation = info.orientation.and_then(|o| u8::try_from(o).ok()).and_then(image::metadata::Orientation::from_exif);
        info.thumbnail = image::load_from_memory(bytes).ok().map(|mut i| {
            if let Some(o) = orientation {
                i.apply_orientation(o);
            }
            i.to_rgba8()
        });
    }
    if let Some(t) = &tiff {
        info.all = craft_metadata::tag_rows(t).into_iter().map(|r| (r.name, shorten(r.value, 120))).collect();
    }
    Some(info)
}

/// Cut a value to at most `max` characters (not bytes, so text in any language is safe), ending in "…".
fn shorten(value: String, max: usize) -> String {
    if value.chars().count() <= max {
        return value;
    }
    let mut out: String = value.chars().take(max.saturating_sub(1)).collect();
    out.push('…');
    out
}

/// Write a small JPEG with typical camera information and an embedded thumbnail (for the test and
/// the developer screenshot of the Properties panel).
pub fn write_demo_jpeg(path: &std::path::Path) -> std::io::Result<()> {
    let to_io = |e: String| std::io::Error::other(e);
    let big = crate::imaging::sample_image(3, 900, 1200);
    let small = image::imageops::resize(&big, 120, 160, image::imageops::FilterType::Triangle);
    let jpeg = |img: &RgbaImage| -> std::io::Result<Vec<u8>> {
        let mut out = Vec::new();
        let rgb = image::DynamicImage::ImageRgba8(img.clone()).to_rgb8();
        rgb.write_with_encoder(image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, 90)).map_err(|e| std::io::Error::other(e.to_string()))?;
        Ok(out)
    };
    let (main, thumb) = (jpeg(&big)?, jpeg(&small)?);
    let ascii = |s: &str| Value::Ascii(s.into());
    let date = "2006:08:18 10:34:25";
    let mut ifd0 = IfdBuilder::new();
    ifd0.set(tags::MAKE, ascii("Example Camera Co.")).set(tags::MODEL, ascii("EC-10")).set(tags::DATE_TIME, ascii(date));
    let mut ex = IfdBuilder::new();
    ex.set(tags::DATE_TIME_ORIGINAL, ascii(date))
        .set(tags::DATE_TIME_DIGITIZED, ascii(date))
        .set(tags::EXPOSURE_TIME, Value::Rational(vec![(1, 640)]))
        .set(tags::F_NUMBER, Value::Rational(vec![(5, 1)]))
        .set(tags::ISO_SPEED, Value::Short(vec![160]))
        .set(tags::FOCAL_LENGTH, Value::Rational(vec![(11, 1)]))
        .set(tags::FLASH, Value::Short(vec![16]))
        .set(tags::METERING_MODE, Value::Short(vec![5]))
        .set(tags::EXPOSURE_PROGRAM, Value::Short(vec![2]))
        .set(COLOR_SPACE, Value::Short(vec![1]));
    ifd0.set_child(tags::EXIF_IFD, ex);
    let block = crate::filemeta::write_exif_block(&ifd0, Some(&thumb)).map_err(to_io)?;
    std::fs::write(path, crate::filemeta::jpeg_with_exif(&main, &block).map_err(to_io)?)
}

/// The original's EXIF worth keeping in an exported copy, as an IFD0 to write (with its EXIF
/// directory): camera, lens, exposure, dates, author and copyright. Left out: what the copy
/// changes (orientation, pixel sizes, thumbnails), the camera maker's private notes, very long
/// values, and the GPS position (which exports add only when asked, from `Meta.place`). Empty
/// when the file has no EXIF.
pub fn exif_to_keep(path: &str) -> IfdBuilder {
    const TIFF_KEPT: [u16; 7] = [tags::MAKE, tags::MODEL, tags::DATE_TIME, tags::ARTIST, tags::COPYRIGHT, tags::IMAGE_DESCRIPTION, tags::SOFTWARE];
    const EXIF_LEFT_OUT: [u16; 5] = [tags::MAKER_NOTE, tags::PIXEL_X_DIMENSION, tags::PIXEL_Y_DIMENSION, tags::USER_COMMENT, tags::INTEROP_IFD];
    let mut out = IfdBuilder::new();
    let Some(exif) = crate::filemeta::blocks(std::path::Path::new(path)).exif else { return out };
    let Ok(tiff) = Tiff::parse(crate::filemeta::tiff_of(&exif)) else { return out };
    let Some(ifd0) = tiff.ifds.first() else { return out };
    let small = |v: &Value| !matches!(v, Value::Ifd(_) | Value::Ifd8(_)) && v.count().saturating_mul(v.field_type().size()) <= 4096;
    for e in ifd0.entries.iter().filter(|e| TIFF_KEPT.contains(&e.tag) && small(&e.value)) {
        out.set(e.tag, e.value.clone());
    }
    if let Some(ex) = &ifd0.exif {
        let mut child = IfdBuilder::new();
        for e in ex.entries.iter().filter(|e| !EXIF_LEFT_OUT.contains(&e.tag) && small(&e.value)) {
            child.set(e.tag, e.value.clone());
        }
        if !child.entries.is_empty() {
            out.set_child(tags::EXIF_IFD, child);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn long_values_are_cut_on_character_boundaries() {
        // 'é' is two bytes: cutting at a byte count would land inside one
        let v = shorten("é".repeat(200), 120);
        assert_eq!(v.chars().count(), 120);
        assert!(v.ends_with('…'));
        assert_eq!(shorten("short".into(), 120), "short");
    }

    #[test]
    fn reads_camera_information_from_a_jpeg() {
        let path = std::env::temp_dir().join(format!("albumcraft-exif-test-{}.jpg", std::process::id()));
        write_demo_jpeg(&path).unwrap();
        let info = read(path.to_str().unwrap()).expect("exif");
        assert_eq!(image::image_dimensions(&path).unwrap(), (900, 1200));
        let _ = std::fs::remove_file(&path);
        assert_eq!(info.camera().unwrap(), "Example Camera Co. EC-10");
        assert_eq!(info.exposure_long().unwrap(), "0.0015625s (1/640)");
        assert_eq!(info.iso, Some(160));
        assert_eq!(info.metering.as_deref(), Some("Matrix"));
        assert_eq!(info.flash.as_deref(), Some("Not Used"));
        assert_eq!(info.program.as_deref(), Some("Program"));
        assert_eq!(info.thumbnail.as_ref().map(|t| t.dimensions()), Some((120, 160)));
        assert!(info.all.len() >= 13);
    }
    #[test]
    fn formats_exposure_times() {
        let c = CameraInfo { exposure_s: Some(1.0 / 640.0), fnumber: Some(5.0), ..Default::default() };
        assert_eq!(c.exposure_long().unwrap(), "0.0015625s (1/640)");
        assert_eq!(c.exposure_short().unwrap(), "1/640s");
        let long = CameraInfo { exposure_s: Some(2.0), ..Default::default() };
        assert_eq!(long.exposure_long().unwrap(), "2s");
        assert_eq!(long.exposure_short().unwrap(), "2.0s");
        assert_eq!(trim_num(5.0, 1), "5");
        assert_eq!(flash_text(0x10), "Not Used");
        assert_eq!(flash_text(0x19), "Used");
        assert_eq!(light_source_text(1), "Sunny");
    }
}
