//! Camera information (EXIF) read from a photo, formatted for the Properties panel
//! ("Exposure time 0.0015625 s (1/640)", "F-number f/5", "Focal length 11mm", ...).

use exif::{In, Tag, Value};
use image::RgbaImage;

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

fn ascii(v: &Value) -> Option<String> {
    match v {
        Value::Ascii(parts) => {
            let s = parts
                .iter()
                .map(|p| String::from_utf8_lossy(p).trim_end_matches('\0').trim().to_string())
                .filter(|s| !s.is_empty())
                .collect::<Vec<_>>()
                .join(" ");
            (!s.is_empty()).then_some(s)
        }
        _ => None,
    }
}

fn real(v: &Value) -> Option<f64> {
    match v {
        Value::Rational(r) => r.first().filter(|r| r.denom != 0).map(|r| r.to_f64()),
        Value::SRational(r) => r.first().filter(|r| r.denom != 0).map(|r| r.to_f64()),
        _ => v.get_uint(0).map(|u| u as f64),
    }
}

/// Plain number without trailing zeros: 5.0 -> "5", 5.6 -> "5.6".
pub fn trim_num(x: f64, decimals: usize) -> String {
    let s = format!("{x:.decimals$}");
    if s.contains('.') { s.trim_end_matches('0').trim_end_matches('.').to_string() } else { s }
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
    let file = std::fs::File::open(path).ok()?;
    let mut reader = std::io::BufReader::new(file);
    let exif = exif::Reader::new().read_from_container(&mut reader).ok()?;
    let get = |t: Tag| exif.get_field(t, In::PRIMARY).map(|f| &f.value);
    let mut info = CameraInfo {
        make: get(Tag::Make).and_then(ascii),
        model: get(Tag::Model).and_then(ascii),
        lens: get(Tag::LensModel).and_then(ascii),
        date_taken: get(Tag::DateTimeOriginal).and_then(ascii),
        date_digitized: get(Tag::DateTimeDigitized).and_then(ascii),
        date_modified: get(Tag::DateTime).and_then(ascii),
        flash: get(Tag::Flash).and_then(|v| v.get_uint(0)).map(flash_text),
        focal_mm: get(Tag::FocalLength).and_then(real),
        focal_35mm: get(Tag::FocalLengthIn35mmFilm).and_then(|v| v.get_uint(0)).filter(|&v| v > 0),
        exposure_s: get(Tag::ExposureTime).and_then(real),
        fnumber: get(Tag::FNumber).and_then(real),
        iso: get(Tag::PhotographicSensitivity).and_then(|v| v.get_uint(0)),
        exposure_bias: get(Tag::ExposureBiasValue).and_then(real),
        metering: get(Tag::MeteringMode).and_then(|v| v.get_uint(0)).map(metering_text),
        program: get(Tag::ExposureProgram).and_then(|v| v.get_uint(0)).map(program_text),
        white_balance: get(Tag::LightSource)
            .and_then(|v| v.get_uint(0))
            .map(light_source_text)
            .or_else(|| get(Tag::WhiteBalance).and_then(|v| v.get_uint(0)).map(|v| if v == 1 { "Manual".into() } else { "Auto".into() })),
        subject_distance: get(Tag::SubjectDistance).and_then(real).filter(|d| *d > 0.0),
        color_space: get(Tag::ColorSpace).and_then(|v| v.get_uint(0)).map(|v| if v == 1 { "sRGB".into() } else { "Uncalibrated".into() }),
        software: get(Tag::Software).and_then(ascii),
        orientation: get(Tag::Orientation).and_then(|v| v.get_uint(0)),
        ..Default::default()
    };
    // the camera's own small preview (IFD1: offset and length inside the EXIF block)
    let off = exif.get_field(Tag::JPEGInterchangeFormat, In::THUMBNAIL).and_then(|f| f.value.get_uint(0));
    let len = exif.get_field(Tag::JPEGInterchangeFormatLength, In::THUMBNAIL).and_then(|f| f.value.get_uint(0));
    if let (Some(off), Some(len)) = (off, len) {
        let (o, l) = (off as usize, len as usize);
        if let Some(bytes) = exif.buf().get(o..o.saturating_add(l)) {
            let orientation = info
                .orientation
                .and_then(|o| u8::try_from(o).ok())
                .and_then(image::metadata::Orientation::from_exif);
            info.thumbnail = image::load_from_memory(bytes).ok().map(|mut i| {
                if let Some(o) = orientation {
                    i.apply_orientation(o);
                }
                i.to_rgba8()
            });
        }
    }
    for f in exif.fields() {
        if f.tag == Tag::MakerNote {
            continue;
        }
        let value = shorten(f.display_value().with_unit(&exif).to_string(), 120);
        let name = if f.ifd_num == In::THUMBNAIL { format!("{} (thumbnail)", f.tag) } else { f.tag.to_string() };
        info.all.push((name, value));
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
    use exif::{Field, Rational};
    let to_io = |e: image::ImageError| std::io::Error::other(e.to_string());
    let big = crate::imaging::sample_image(3, 900, 1200);
    let small = image::imageops::resize(&big, 120, 160, image::imageops::FilterType::Triangle);
    let jpeg = |img: &RgbaImage| -> std::io::Result<Vec<u8>> {
        let mut out = Vec::new();
        let rgb = image::DynamicImage::ImageRgba8(img.clone()).to_rgb8();
        rgb.write_with_encoder(image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, 90)).map_err(to_io)?;
        Ok(out)
    };
    let (main, thumb) = (jpeg(&big)?, jpeg(&small)?);
    let ascii = |t: Tag, s: &str| Field { tag: t, ifd_num: In::PRIMARY, value: Value::Ascii(vec![s.as_bytes().to_vec()]) };
    let rat = |t: Tag, n: u32, d: u32| Field { tag: t, ifd_num: In::PRIMARY, value: Value::Rational(vec![Rational { num: n, denom: d }]) };
    let short = |t: Tag, v: u16| Field { tag: t, ifd_num: In::PRIMARY, value: Value::Short(vec![v]) };
    let fields = vec![
        ascii(Tag::Make, "Example Camera Co."),
        ascii(Tag::Model, "EC-10"),
        ascii(Tag::DateTime, "2006:08:18 10:34:25"),
        ascii(Tag::DateTimeOriginal, "2006:08:18 10:34:25"),
        ascii(Tag::DateTimeDigitized, "2006:08:18 10:34:25"),
        rat(Tag::ExposureTime, 1, 640),
        rat(Tag::FNumber, 5, 1),
        short(Tag::PhotographicSensitivity, 160),
        rat(Tag::FocalLength, 11, 1),
        short(Tag::Flash, 16),
        short(Tag::MeteringMode, 5),
        short(Tag::ExposureProgram, 2),
        short(Tag::ColorSpace, 1),
    ];
    std::fs::write(path, jpeg_with_exif(&main, &fields, Some(&thumb))?)
}

/// A JPEG file: `jpeg` (an encoded picture) with an EXIF block holding `fields` and an optional
/// embedded thumbnail. For tests and demo pictures.
pub fn jpeg_with_exif(jpeg: &[u8], fields: &[exif::Field], thumb: Option<&[u8]>) -> std::io::Result<Vec<u8>> {
    let mut w = exif::experimental::Writer::new();
    for f in fields {
        w.push_field(f);
    }
    if let Some(t) = thumb {
        w.set_jpeg(t, In::THUMBNAIL);
    }
    let mut tiff = std::io::Cursor::new(Vec::new());
    w.write(&mut tiff, true).map_err(|e| std::io::Error::other(e.to_string()))?;
    let tiff = tiff.into_inner();
    let len = u16::try_from(2 + 6 + tiff.len()).map_err(|_| std::io::Error::other("EXIF block too large"))?;
    let body = jpeg.get(2..).ok_or_else(|| std::io::Error::other("not a JPEG"))?;
    // JPEG = start marker, APP1 "Exif" block, then the encoded picture without its own start marker
    let mut file = vec![0xFF, 0xD8, 0xFF, 0xE1];
    file.extend_from_slice(&len.to_be_bytes());
    file.extend_from_slice(b"Exif\0\0");
    file.extend_from_slice(&tiff);
    file.extend_from_slice(body);
    Ok(file)
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
