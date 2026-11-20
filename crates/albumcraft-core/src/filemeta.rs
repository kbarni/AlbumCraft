//! The metadata blocks of a picture file (EXIF, XMP, IPTC, ICC), read with as little of the
//! file as they need, for `craft-metadata` to read (`craft_metadata::embedded`, `from_embedded`).
//!
//! - JPEG: the segments before the picture data (the scan is never read);
//! - PNG: every chunk but the picture data (`IDAT`, skipped), so XMP after it is found too;
//! - TIFF and the raw formats built on it: the first `HEAD_BYTES`, and the XMP (tag 700) of the
//!   first directory wherever it is;
//! - HEIF, HEIC and AVIF: their `Exif` and XMP items (`heif::embedded`: the first `HEAD_BYTES`,
//!   the whole file when the items lie past them);
//! - anything else (WebP, CR3, RAF, JPEG XL…): the first `HEAD_BYTES`.
//!
//! Problems give empty blocks, never an error: metadata is optional.

use craft_metadata::{Embedded, Metadata};
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

/// How much of a file is read when its metadata can't be located more precisely.
pub const HEAD_BYTES: u64 = 4 << 20;
/// The largest block kept (an XMP packet, a PNG text chunk).
const MAX_BLOCK: u64 = 16 << 20;

const PNG_SIG: &[u8] = b"\x89PNG\r\n\x1a\n";

/// The metadata blocks of a picture file.
pub fn blocks(path: &Path) -> Embedded {
    head(path).map(|h| if craft_heif::items::is_heif(&h) { crate::heif::embedded(&h, path) } else { embedded_of(&h, path) }).unwrap_or_default()
}

/// Everything `craft-metadata` reads from a picture file: EXIF, then IPTC filling gaps, then
/// XMP for the user's fields (rating, caption, keywords, GPS…).
pub fn metadata(path: &Path) -> Metadata {
    craft_metadata::from_embedded(&blocks(path))
}

/// The EXIF of a picture file (camera, exposure, dates, GPS…), without its XMP and IPTC.
pub fn exif(path: &Path) -> Option<Metadata> {
    blocks(path).exif.as_deref().and_then(|e| craft_metadata::try_read_exif(e).ok())
}

/// The part of the file the metadata is in, laid out as a file of the same kind (a JPEG cut
/// before its picture data, a PNG without its `IDAT` chunks, the start of anything else).
pub fn head(path: &Path) -> Option<Vec<u8>> {
    let mut f = std::fs::File::open(path).ok()?;
    let mut sig = [0u8; 8];
    let n = read_up_to(&mut f, &mut sig);
    f.seek(SeekFrom::Start(0)).ok()?;
    if sig[..n].starts_with(&[0xff, 0xd8]) {
        jpeg_head(&mut f)
    } else if sig[..n].starts_with(PNG_SIG) {
        png_head(&mut f)
    } else {
        let mut buf = vec![];
        f.take(HEAD_BYTES).read_to_end(&mut buf).ok()?;
        Some(buf)
    }
}

/// `craft_metadata::embedded` of a file's head, plus a TIFF's XMP when it lies past the head.
fn embedded_of(head: &[u8], path: &Path) -> Embedded {
    let mut e = craft_metadata::embedded(head);
    if e.xmp.is_none()
        && craft_tiff::Tiff::sniff(head).is_some()
        && let Ok(mut f) = std::fs::File::open(path)
    {
        e.xmp = tiff_xmp(&mut f);
    }
    e
}

fn read_up_to(f: &mut impl Read, buf: &mut [u8]) -> usize {
    let mut n = 0;
    while n < buf.len() {
        match f.read(&mut buf[n..]) {
            Ok(0) | Err(_) => break,
            Ok(k) => n += k,
        }
    }
    n
}

/// A JPEG's bytes up to its first scan (the start of scan marker included, none of its data).
fn jpeg_head<F: Read + Seek>(f: &mut F) -> Option<Vec<u8>> {
    let mut out = vec![0xff, 0xd8];
    f.seek(SeekFrom::Start(2)).ok()?;
    for _ in 0..4096 {
        let mut m = [0u8; 2];
        if f.read_exact(&mut m).is_err() || m[0] != 0xff {
            break;
        }
        if m[1] == 0xff {
            // fill byte
            f.seek(SeekFrom::Current(-1)).ok()?;
            continue;
        }
        out.extend_from_slice(&m);
        if m[1] == 0xda || m[1] == 0xd9 {
            break;
        }
        if (0xd0..=0xd7).contains(&m[1]) || m[1] == 0x01 {
            continue;
        }
        let mut l = [0u8; 2];
        if f.read_exact(&mut l).is_err() {
            break;
        }
        out.extend_from_slice(&l);
        let len = (u16::from_be_bytes(l) as usize).saturating_sub(2);
        let start = out.len();
        out.resize(start + len, 0);
        if f.read_exact(&mut out[start..]).is_err() {
            out.truncate(start);
            break;
        }
        if out.len() as u64 > MAX_BLOCK * 2 {
            break;
        }
    }
    Some(out)
}

/// A PNG's chunks without the picture data (`IDAT`, `fdAT`), which are skipped, not read.
fn png_head<F: Read + Seek>(f: &mut F) -> Option<Vec<u8>> {
    let mut out = PNG_SIG.to_vec();
    f.seek(SeekFrom::Start(PNG_SIG.len() as u64)).ok()?;
    for _ in 0..100_000 {
        let mut h = [0u8; 8];
        if f.read_exact(&mut h).is_err() {
            break;
        }
        let len = u32::from_be_bytes([h[0], h[1], h[2], h[3]]) as u64;
        let kind = [h[4], h[5], h[6], h[7]];
        if &kind == b"IDAT" || &kind == b"fdAT" || len > MAX_BLOCK {
            f.seek(SeekFrom::Current(len as i64 + 4)).ok()?;
            continue;
        }
        out.extend_from_slice(&h);
        let start = out.len();
        out.resize(start + len as usize + 4, 0);
        if f.read_exact(&mut out[start..]).is_err() {
            out.truncate(start - 8);
            break;
        }
        if &kind == b"IEND" {
            break;
        }
    }
    Some(out)
}

/// The XMP (tag 700) of a TIFF's first directory, read where it is.
fn tiff_xmp<F: Read + Seek>(f: &mut F) -> Option<String> {
    let mut h = [0u8; 8];
    f.read_exact(&mut h).ok()?;
    let le = &h[0..2] == b"II";
    let u16_of = |b: [u8; 2]| if le { u16::from_le_bytes(b) } else { u16::from_be_bytes(b) };
    let u32_of = |b: [u8; 4]| if le { u32::from_le_bytes(b) } else { u32::from_be_bytes(b) };
    // classic TIFF only (BigTIFF raws are rare, and their XMP is in the head when small)
    if u16_of([h[2], h[3]]) != 42 {
        return None;
    }
    f.seek(SeekFrom::Start(u32_of([h[4], h[5], h[6], h[7]]) as u64)).ok()?;
    let mut c = [0u8; 2];
    f.read_exact(&mut c).ok()?;
    for _ in 0..u16_of(c).min(1000) {
        let mut e = [0u8; 12];
        f.read_exact(&mut e).ok()?;
        if u16_of([e[0], e[1]]) == craft_tiff::tags::XMP {
            let n = u32_of([e[4], e[5], e[6], e[7]]) as u64;
            if n <= 4 || n > MAX_BLOCK {
                return None;
            }
            f.seek(SeekFrom::Start(u32_of([e[8], e[9], e[10], e[11]]) as u64)).ok()?;
            let mut v = vec![0u8; n as usize];
            f.read_exact(&mut v).ok()?;
            let v = v.strip_prefix(b"\xef\xbb\xbf").unwrap_or(&v);
            return std::str::from_utf8(v).ok().map(|s| s.trim_end_matches(['\0', ' ', '\n', '\r', '\t']).to_string());
        }
    }
    None
}

// ---------------------------------------------------------------------------------------------
// EXIF blocks

/// The TIFF data of an EXIF block (without the `Exif\0\0` of a JPEG segment).
pub fn tiff_of(exif: &[u8]) -> &[u8] {
    craft_metadata::strip_exif_header(exif)
}

/// The small preview a camera stores in the EXIF block (IFD1), as JPEG bytes.
pub fn exif_thumbnail(exif: &[u8]) -> Option<&[u8]> {
    let data = tiff_of(exif);
    let tiff = craft_tiff::Tiff::parse(data).ok()?;
    let ifd1 = tiff.ifds.get(1)?;
    let off = ifd1.u64(craft_tiff::tags::JPEG_INTERCHANGE_FORMAT)? as usize;
    let len = ifd1.u64(craft_tiff::tags::JPEG_INTERCHANGE_FORMAT_LENGTH)? as usize;
    data.get(off..off.checked_add(len)?)
}

/// A camera's local date and time (EXIF dates have no time zone) as Unix seconds in this
/// computer's time zone, as the library shows them.
pub fn local_seconds(d: &craft_metadata::DateTime) -> Option<i64> {
    use chrono::TimeZone;
    let date = chrono::NaiveDate::from_ymd_opt(d.year as i32, d.month as u32, d.day as u32)?;
    let naive = date.and_hms_opt(d.hour as u32, d.minute as u32, d.second as u32)?;
    chrono::Local.from_local_datetime(&naive).earliest().map(|t| t.timestamp())
}

/// An EXIF block for a picture file: `ifd0` (with its EXIF and GPS directories as children) and
/// an optional thumbnail (IFD1), little-endian TIFF data without the `Exif\0\0` header.
pub fn write_exif_block(ifd0: &craft_tiff::IfdBuilder, thumb: Option<&[u8]>) -> Result<Vec<u8>, String> {
    use craft_tiff::{IfdBuilder, Value, tags};
    let w = craft_tiff::TiffWriter::new(craft_tiff::ByteOrder::Little, false);
    let Some(thumb) = thumb else { return w.write(std::slice::from_ref(ifd0)).map_err(|e| e.to_string()) };
    // the writer knows nothing of JPEG thumbnails: written with a dummy offset, the bytes added
    // at the end, the offset set where the writer put it
    let len = u32::try_from(thumb.len()).map_err(|_| "the thumbnail is too large")?;
    let ifd1 = IfdBuilder::new()
        .with(tags::COMPRESSION, Value::Short(vec![6]))
        .with(tags::JPEG_INTERCHANGE_FORMAT, Value::Long(vec![0]))
        .with(tags::JPEG_INTERCHANGE_FORMAT_LENGTH, Value::Long(vec![len]));
    let mut out = w.write(&[ifd0.clone(), ifd1]).map_err(|e| e.to_string())?;
    let parsed = craft_tiff::Tiff::parse(&out).map_err(|e| e.to_string())?;
    let at = parsed.ifds.get(1).and_then(|i| i.get(tags::JPEG_INTERCHANGE_FORMAT)).ok_or("no thumbnail entry")?.offset as usize;
    let off = u32::try_from(out.len()).map_err(|_| "the EXIF is too large")?;
    out.get_mut(at..at + 4).ok_or("no thumbnail entry")?.copy_from_slice(&off.to_le_bytes());
    out.extend_from_slice(thumb);
    Ok(out)
}

/// A JPEG file: `jpeg` (an encoded picture) with `exif` (TIFF data) as its first segment.
pub fn jpeg_with_exif(jpeg: &[u8], exif: &[u8]) -> Result<Vec<u8>, String> {
    let len = u16::try_from(2 + 6 + exif.len()).map_err(|_| "the EXIF is too large")?;
    let body = jpeg.strip_prefix(&[0xff, 0xd8]).ok_or("not a JPEG")?;
    let mut file = vec![0xff, 0xd8, 0xff, 0xe1];
    file.extend_from_slice(&len.to_be_bytes());
    file.extend_from_slice(b"Exif\0\0");
    file.extend_from_slice(exif);
    file.extend_from_slice(body);
    Ok(file)
}

#[cfg(test)]
mod tests {
    use super::*;
    use craft_tiff::{IfdBuilder, Value, tags};

    fn tmp(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("albumcraft-filemeta-{}-{name}", std::process::id()))
    }

    fn small_jpeg() -> Vec<u8> {
        let mut out = vec![];
        image::DynamicImage::ImageRgb8(image::RgbImage::from_pixel(16, 8, image::Rgb([200, 10, 10])))
            .write_to(&mut std::io::Cursor::new(&mut out), image::ImageFormat::Jpeg)
            .unwrap();
        out
    }

    #[test]
    fn exif_with_a_thumbnail_reads_back() {
        let thumb = small_jpeg();
        let mut ifd0 = IfdBuilder::new();
        ifd0.set(tags::MODEL, Value::Ascii("EC-10".into()));
        ifd0.set_child(tags::EXIF_IFD, IfdBuilder::new().with(tags::DATE_TIME_ORIGINAL, Value::Ascii("2006:08:18 10:34:25".into())));
        let block = write_exif_block(&ifd0, Some(&thumb)).unwrap();
        assert_eq!(exif_thumbnail(&block), Some(&thumb[..]));
        let file = jpeg_with_exif(&small_jpeg(), &block).unwrap();
        let path = tmp("thumb.jpg");
        std::fs::write(&path, &file).unwrap();
        let m = exif(&path).unwrap();
        let head = head(&path).unwrap();
        let _ = std::fs::remove_file(&path);
        assert_eq!(m.model.as_deref(), Some("EC-10"));
        assert_eq!(m.capture_time.map(|d| (d.year, d.hour)), Some((2006, 10)));
        // the picture data isn't read
        assert!(head.len() < file.len());
        assert!(image::load_from_memory(&file).is_ok());
    }

    #[test]
    fn png_xmp_after_the_picture_data_is_found() {
        let mut png = vec![];
        image::DynamicImage::ImageRgb8(image::RgbImage::new(4, 4)).write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png).unwrap();
        // an iTXt XMP chunk just before IEND
        let packet = r#"<x:xmpmeta xmlns:x="adobe:ns:meta/"><rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"><rdf:Description xmlns:xmp="http://ns.adobe.com/xap/1.0/" xmp:Rating="3"/></rdf:RDF></x:xmpmeta>"#;
        let mut body = b"XML:com.adobe.xmp\0\0\0\0\0".to_vec();
        body.extend_from_slice(packet.as_bytes());
        let mut chunk = (body.len() as u32).to_be_bytes().to_vec();
        let mut crc_data = b"iTXt".to_vec();
        crc_data.extend_from_slice(&body);
        chunk.extend_from_slice(&crc_data);
        chunk.extend_from_slice(&crate::xmp::crc32(&crc_data).to_be_bytes());
        let iend = png.len() - 12;
        png.splice(iend..iend, chunk);
        let path = tmp("after.png");
        std::fs::write(&path, &png).unwrap();
        let b = blocks(&path);
        let _ = std::fs::remove_file(&path);
        assert!(b.xmp.is_some_and(|x| x.contains("xmp:Rating")));
    }

    #[test]
    fn hostile_files_give_nothing() {
        for bytes in [&b""[..], b"\xff\xd8", b"\xff\xd8\xff\xe1\xff\xff", b"\x89PNG\r\n\x1a\n\xff\xff\xff\xffiTXt", b"II*\0\xff\xff\xff\xff", b"MM\0*\0\0\0\x08\x00\x01\x02\xbc"] {
            let path = tmp("hostile");
            std::fs::write(&path, bytes).unwrap();
            let m = metadata(&path);
            let _ = std::fs::remove_file(&path);
            assert_eq!(m.gps, None);
        }
    }
}
