//! XMP: the notes other photo programs exchange, inside picture files (embedded) or in a
//! sidecar file next to them (`photo.jpg.xmp`, `photo.xmp`).
//!
//! - `XmpNotes`: the fields AlbumCraft reads and writes (rating, pick / reject, caption, tags
//!   with their hierarchy, people, GPS position, the edit list).
//! - `parse`: a packet's text to `XmpNotes`; `merge`: `XmpNotes` written into a packet, keeping
//!   every property AlbumCraft doesn't manage as it was. Both through `craft-metadata`
//!   (`parse_xmp_with`, `write_xmp_with`, `merge_xmp`).
//! - `read_embedded` / `replace_embedded`: the packet inside a file (read by `filemeta`: JPEG,
//!   PNG, WebP, TIFF-based raws, CR3, RAF, JPEG XL, HEIF / AVIF; replaced in JPEG and PNG).
//! - `sidecar_of`, `read_for`, `write_for`: where a picture's XMP is, and writing it there.
//!
//! Every input is hostile: sizes are capped, nothing panics, a packet that can't be read is
//! never overwritten.

use crate::library::Flag;
use albumcraft_edit::imaging::EditOp;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub const NS_X: &str = "adobe:ns:meta/";
pub const NS_RDF: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#";
pub const NS_XMP: &str = "http://ns.adobe.com/xap/1.0/";
pub const NS_DC: &str = "http://purl.org/dc/elements/1.1/";
pub const NS_LR: &str = "http://ns.adobe.com/lightroom/1.0/";
pub const NS_EXIF: &str = "http://ns.adobe.com/exif/1.0/";
pub const NS_IPTC_EXT: &str = "http://iptc.org/std/Iptc4xmpExt/2008-02-29/";
pub const NS_MWG_RS: &str = "http://www.metadataworkinggroup.com/schemas/regions/";
pub const NS_DIGIKAM: &str = "http://www.digikam.org/ns/1.0/";
pub const NS_XMP_NOTE: &str = "http://ns.adobe.com/xmp/note/";
/// AlbumCraft's own properties (pick / reject, the edit list).
pub const NS_ALBUMCRAFT: &str = "https://github.com/kbarni/AlbumCraft/ns/xmp/1.0/";

/// Largest packet read or written (real ones are a few KB; raw developers' can reach 1 MB).
pub const MAX_PACKET: usize = 4 << 20;
/// The edit list's format in `albumcraft:Edits`.
const EDITS_FORMAT: u32 = 1;

/// What a picture's XMP says, for the fields AlbumCraft knows. `None`: not in the XMP. When
/// writing, `None` leaves the property as it is, and an empty value removes it.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct XmpNotes {
    /// −1 (rejected), 0 (no rating) to 5.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rating: Option<i8>,
    /// AlbumCraft's pick / reject flag (`albumcraft:Flag`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flag: Option<Flag>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub caption: Option<String>,
    /// Each tag with the tags it is under, outermost first: `["Places", "France", "Bordeaux"]`.
    /// A tag with no parent is a path of one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<Vec<String>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub people: Option<Vec<String>>,
    /// Named face regions (MWG, as `craft_metadata` reads them): the name and the box (left,
    /// top, width, height as fractions of the picture as shown). Read only, and not kept with
    /// what was last seen: the window turns them into faces when it reads the file.
    #[serde(skip)]
    pub regions: Vec<(String, [f32; 4])>,
    /// Latitude, longitude.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gps: Option<[f64; 2]>,
    /// When writing: remove the GPS position (the user removed the place).
    #[serde(skip)]
    pub gps_removed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub edits: Option<Vec<EditOp>>,
    /// When the picture was taken, when the XMP says another date than the file's own EXIF
    /// (`exif:DateTimeOriginal`, else `photoshop:DateCreated`; a year or a month alone kept as
    /// such). The same date as the file's reads as None (`read_for`), so that the camera's date
    /// copied into the XMP is not taken for one the user set.
    #[serde(default, with = "crate::taken::iso", skip_serializing_if = "Option::is_none")]
    pub date: Option<craft_metadata::DateTime>,
    /// When writing: remove the date (the user went back to the file's own date).
    #[serde(skip)]
    pub date_removed: bool,
}

impl XmpNotes {
    pub fn is_empty(&self) -> bool {
        *self == XmpNotes::default()
    }

    /// The same notes with the lists sorted, so that two XMPs listing the same tags or people
    /// in another order compare equal.
    pub fn normalized(mut self) -> XmpNotes {
        if let Some(t) = &mut self.tags {
            t.sort();
            t.dedup();
        }
        if let Some(p) = &mut self.people {
            p.sort();
            p.dedup();
        }
        self
    }
}

// ---------------------------------------------------------------------------------------------
// reading a packet

/// The namespaces beyond the standard ones that AlbumCraft reads and writes, for
/// `craft-metadata`: its own, and digiKam's slash-separated tag list.
const NAMESPACES: &[craft_metadata::XmpNamespace<'static>] =
    &[craft_metadata::XmpNamespace { prefix: "albumcraft", uri: NS_ALBUMCRAFT }, craft_metadata::XmpNamespace { prefix: "digiKam", uri: NS_DIGIKAM }];

/// The notes in a packet, or None when its text isn't an XMP packet `craft-metadata` can read.
pub fn parse(text: &str) -> Option<XmpNotes> {
    if text.len() > MAX_PACKET {
        return None;
    }
    let data = craft_metadata::parse_xmp_with(text, NAMESPACES).ok()?;
    Some(notes_of(&data))
}

/// The notes of a parsed packet.
fn notes_of(data: &craft_metadata::XmpData) -> XmpNotes {
    let props = &data.properties;
    let first = |k: &str| props.get(k).and_then(|v| v.first()).map(|v| v.trim().to_string());
    let mut out = XmpNotes {
        // read here rather than from the model: a rating out of range is brought into it
        rating: first("xmp:Rating").and_then(|v| v.parse::<f64>().ok()).filter(|r| r.is_finite()).map(|r| r.round().clamp(-1.0, 5.0) as i8),
        flag: first("albumcraft:Flag").map(|v| match v.as_str() {
            "pick" => Flag::Pick,
            "reject" => Flag::Reject,
            _ => Flag::None,
        }),
        caption: data.metadata.caption.as_deref().map(|c| limit(c.trim())),
        edits: first("albumcraft:Edits").and_then(|v| edits_from(&v)),
        ..Default::default()
    };
    let has = |k: &str| props.contains_key(k);
    if has("dc:subject") || has("lr:hierarchicalSubject") || has("digiKam:TagsList") {
        let mut paths: Vec<Vec<String>> = data.metadata.hierarchical_keywords.iter().map(|s| split_path(s, '|')).filter(|p| !p.is_empty()).collect();
        if paths.is_empty() {
            paths = props.get("digiKam:TagsList").into_iter().flatten().map(|s| split_path(s, '/')).filter(|p| !p.is_empty()).collect();
        }
        drop_ancestors(&mut paths);
        // a plain keyword that is one of the hierarchy's levels is that level, not a tag of its own
        let known: std::collections::HashSet<String> = paths.iter().flatten().map(|t| crate::tags::key(t)).collect();
        let extra: Vec<Vec<String>> =
            data.metadata.keywords.iter().map(|k| limit(k.trim())).filter(|k| !k.is_empty() && !known.contains(&crate::tags::key(k))).map(|k| vec![k]).collect();
        paths.extend(extra);
        paths.truncate(10_000);
        out.tags = Some(paths);
    }
    let faces: Vec<String> = data
        .metadata
        .regions
        .iter()
        .filter(|r| matches!(&r.kind, craft_metadata::RegionKind::Face) || matches!(&r.kind, craft_metadata::RegionKind::Other(t) if t.trim().is_empty()))
        .filter_map(|r| r.name.as_deref().map(str::trim).filter(|n| !n.is_empty()).map(limit))
        .collect();
    out.regions = data
        .metadata
        .regions
        .iter()
        .filter(|r| matches!(&r.kind, craft_metadata::RegionKind::Face) || matches!(&r.kind, craft_metadata::RegionKind::Other(t) if t.trim().is_empty()))
        .filter_map(|r| {
            let name = r.name.as_deref().map(str::trim).filter(|n| !n.is_empty())?;
            let (x0, y0, x1, y1) = (r.rect.x0.clamp(0.0, 1.0) as f32, r.rect.y0.clamp(0.0, 1.0) as f32, r.rect.x1.clamp(0.0, 1.0) as f32, r.rect.y1.clamp(0.0, 1.0) as f32);
            (x1 > x0 && y1 > y0).then(|| (limit(name), [x0, y0, x1 - x0, y1 - y0]))
        })
        .take(1000)
        .collect();
    if has("Iptc4xmpExt:PersonInImage") || !faces.is_empty() {
        let mut people: Vec<String> = props.get("Iptc4xmpExt:PersonInImage").into_iter().flatten().map(|p| limit(p.trim())).filter(|p| !p.is_empty()).collect();
        people.extend(faces);
        people.truncate(10_000);
        out.people = Some(people);
    }
    out.gps = data.metadata.gps.filter(|g| crate::geo::LatLon::new(g.latitude, g.longitude).is_some()).map(|g| [g.latitude, g.longitude]);
    // not `xmp:CreateDate`: scanning programs put the day of the scan there
    out.date = ["exif:DateTimeOriginal", "photoshop:DateCreated"]
        .iter()
        .find_map(|k| first(k).and_then(|v| craft_metadata::DateTime::parse_iso(&v)))
        .and_then(|d| crate::taken::checked(craft_metadata::DateTime { offset_minutes: None, millis: 0, ..d }).ok());
    out.normalized()
}

/// A text cut to 64 KB at most (on a character boundary).
fn limit(s: &str) -> String {
    const MAX: usize = 64 * 1024;
    let mut end = s.len().min(MAX);
    while !s.is_char_boundary(end) {
        end -= 1;
    }
    s[..end].to_string()
}

/// Remove the paths that are only the levels above another one: Lightroom lists `Sport` and
/// `Sport|Plongee` for a picture tagged Plongee (under Sport). Letter case is ignored, as in
/// the tags.
pub fn drop_ancestors(paths: &mut Vec<Vec<String>>) {
    let keyed: Vec<Vec<String>> = paths.iter().map(|p| p.iter().map(|t| crate::tags::key(t)).collect()).collect();
    let mut i = 0;
    paths.retain(|_| {
        let p = &keyed[i];
        i += 1;
        !keyed.iter().any(|q| q.len() > p.len() && q.starts_with(p))
    });
}

/// The tags to list in `dc:subject`: each tag and, as Lightroom does, the tags above it, so a
/// program that knows only plain keywords finds the picture under Sport as well.
fn keywords(paths: &[Vec<String>]) -> Vec<String> {
    let mut out: Vec<String> = paths.iter().flatten().cloned().collect();
    out.sort_by_key(|t| crate::tags::key(t));
    out.dedup_by(|a, b| crate::tags::key(a) == crate::tags::key(b));
    out
}

fn split_path(s: &str, sep: char) -> Vec<String> {
    s.split(sep).map(str::trim).filter(|p| !p.is_empty()).take(32).map(str::to_string).collect()
}

#[derive(Serialize, Deserialize)]
struct EditsJson {
    format: u32,
    ops: Vec<EditOp>,
}

/// The edit list in `albumcraft:Edits`; None for one written by a later version this one can't
/// read (it is then left as it is).
fn edits_from(v: &str) -> Option<Vec<EditOp>> {
    let e: EditsJson = serde_json::from_str(v).ok()?;
    (e.format == EDITS_FORMAT).then_some(e.ops)
}

// ---------------------------------------------------------------------------------------------
// writing a packet

/// The properties AlbumCraft writes for these notes: replaced in the old packet (removed when
/// the notes' value is empty); `None` fields leave theirs as they are.
fn owned(notes: &XmpNotes) -> Vec<&'static str> {
    let mut out = vec![];
    let mut add = |on: bool, names: &[&'static str]| {
        if on {
            out.extend_from_slice(names);
        }
    };
    add(notes.rating.is_some(), &["xmp:Rating"]);
    add(notes.flag.is_some(), &["albumcraft:Flag"]);
    add(notes.edits.is_some(), &["albumcraft:Edits"]);
    add(notes.caption.is_some(), &["dc:description"]);
    add(notes.tags.is_some(), &["dc:subject", "lr:hierarchicalSubject", "digiKam:TagsList"]);
    add(notes.people.is_some(), &["Iptc4xmpExt:PersonInImage"]);
    add(notes.gps.is_some() || notes.gps_removed, &["exif:GPSLatitude", "exif:GPSLongitude"]);
    add(notes.date.is_some() || notes.date_removed, &["exif:DateTimeOriginal", "photoshop:DateCreated"]);
    out
}

pub fn escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            // characters XML 1.0 can't hold
            c if (c as u32) < 0x20 && !matches!(c, '\t' | '\n' | '\r') => {}
            c => out.push(c),
        }
    }
    out
}

/// A packet with `notes` written into `old` (None or empty: a new packet). Every property of
/// `old` that `notes` doesn't set is kept byte for byte (`craft_metadata::merge_xmp`).
/// `sidecar`: a new packet is written for a sidecar file (without the `<?xpacket?>` around it).
/// Err when `old` can't be read: it is not replaced then.
pub fn merge(old: Option<&str>, notes: &XmpNotes, sidecar: bool) -> Result<String, String> {
    let old = old.filter(|o| !o.trim().trim_matches('\0').is_empty());
    let Some(old) = old else {
        let fresh = fresh_packet(notes, false);
        return Ok(if sidecar { without_wrapper(&fresh) } else { fresh });
    };
    if old.len() > MAX_PACKET {
        return Err("the XMP is too large".into());
    }
    let parsed = craft_metadata::parse_xmp_with(old.trim_end_matches(['\0', ' ', '\n', '\r', '\t']), NAMESPACES).map_err(|e| format!("the XMP can't be read ({e})"))?;
    // digiKam's own list is kept up to date when the packet has one
    let fresh = fresh_packet(notes, parsed.properties.contains_key("digiKam:TagsList"));
    let owned = owned(notes);
    let rules = craft_metadata::MergeRules { owned: &owned, owned_if_present: &[], namespaces: NAMESPACES };
    craft_metadata::merge_xmp(old, &fresh, rules).map_err(|e| format!("the XMP can't be read ({e})"))
}

/// A whole packet with what `notes` sets (`craft_metadata::write_xmp_with`, plus the lists it
/// doesn't write: people, and digiKam's tag list when `digikam`).
fn fresh_packet(notes: &XmpNotes, digikam: bool) -> String {
    let mut m = craft_metadata::Metadata { rating: notes.rating.map(|r| r.clamp(-1, 5)), ..Default::default() };
    m.caption = notes.caption.as_deref().map(str::trim).filter(|c| !c.is_empty()).map(str::to_string);
    if let Some(tags) = &notes.tags {
        m.keywords = keywords(tags);
        m.hierarchical_keywords = tags.iter().filter(|p| !p.is_empty()).map(|p| p.join("|")).collect();
    }
    if let Some([lat, lon]) = notes.gps
        && !notes.gps_removed
        && crate::geo::LatLon::new(lat, lon).is_some()
    {
        m.gps = Some(craft_metadata::Gps { latitude: lat, longitude: lon, altitude: None });
    }
    // a year or a month alone goes in `photoshop:DateCreated` only (`craft-metadata`)
    if !notes.date_removed {
        m.capture_time = notes.date;
    }
    let flag = match notes.flag {
        Some(Flag::Pick) => Some("pick"),
        Some(Flag::Reject) => Some("reject"),
        _ => None,
    };
    let edits = notes.edits.as_ref().filter(|e| !e.is_empty()).and_then(|ops| serde_json::to_string(&EditsJson { format: EDITS_FORMAT, ops: ops.clone() }).ok());
    let mut props: Vec<(&str, &str)> = vec![];
    if let Some(f) = flag {
        props.push(("Flag", f));
    }
    if let Some(e) = &edits {
        props.push(("Edits", e));
    }
    let toolkit = format!("AlbumCraft {}", env!("CARGO_PKG_VERSION"));
    let with = craft_metadata::XmpWrite { toolkit: &toolkit, namespace: NAMESPACES.first().copied(), properties: &props };
    let mut packet = craft_metadata::write_xmp_with(&m, &with);
    let bag = |name: &str, items: &[String]| -> String {
        let mut s = format!("   <{name}>\n    <rdf:Bag>\n");
        for i in items {
            s.push_str(&format!("     <rdf:li>{}</rdf:li>\n", escape(i)));
        }
        s.push_str(&format!("    </rdf:Bag>\n   </{name}>\n"));
        s
    };
    let mut extra = String::new();
    if let Some(people) = notes.people.as_ref().filter(|p| !p.is_empty()) {
        extra.push_str(&bag("Iptc4xmpExt:PersonInImage", people));
    }
    if digikam && let Some(tags) = notes.tags.as_ref().filter(|t| !t.is_empty()) {
        let dk: Vec<String> = tags.iter().filter(|p| !p.is_empty()).map(|p| p.join("/")).collect();
        extra.push_str(&bag("digiKam:TagsList", &dk));
        packet = packet.replacen("<rdf:Description rdf:about=\"\"", &format!("<rdf:Description rdf:about=\"\"\n    xmlns:digiKam=\"{NS_DIGIKAM}\""), 1);
    }
    if let Some(at) = packet.rfind("  </rdf:Description>") {
        packet.insert_str(at, &extra);
    }
    packet
}

/// A packet without the `<?xpacket?>` instructions around it, as sidecar files are written.
fn without_wrapper(packet: &str) -> String {
    let body = packet.trim();
    let body = match (body.find("?>"), body.starts_with("<?xpacket")) {
        (Some(i), true) => body[i + 2..].trim_start(),
        _ => body,
    };
    let body = match body.rfind("<?xpacket") {
        Some(i) => body[..i].trim_end(),
        None => body,
    };
    format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n{body}\n")
}

// ---------------------------------------------------------------------------------------------
// the packet inside a file

/// The XMP header of a JPEG APP1 segment.
const JPEG_XMP: &[u8] = b"http://ns.adobe.com/xap/1.0/\0";
const JPEG_XMP_EXT: &[u8] = b"http://ns.adobe.com/xmp/extension/\0";
const PNG_SIG: &[u8] = b"\x89PNG\r\n\x1a\n";
const PNG_KEYWORD: &[u8] = b"XML:com.adobe.xmp";

/// What is known of a file's embedded XMP.
#[derive(Debug, Default, PartialEq)]
pub struct Embedded {
    pub packet: Option<String>,
    /// The file has extended XMP (JPEG packets over 64 KB), which isn't rewritten.
    pub extended: bool,
}

/// The XMP inside a picture file (`filemeta::blocks`). Reading problems give an empty answer,
/// never an error.
pub fn read_embedded(path: &Path) -> Embedded {
    embedded_of(&crate::filemeta::blocks(path))
}

fn embedded_of(b: &craft_metadata::Embedded) -> Embedded {
    Embedded {
        packet: b.xmp.as_deref().map(|p| p.trim_end_matches(['\0', ' ', '\n', '\r', '\t'])).filter(|p| !p.is_empty() && p.len() <= MAX_PACKET).map(str::to_string),
        extended: b.xmp_extended.is_some() || b.xmp_extended_error.is_some(),
    }
}

/// The file's bytes with its embedded packet replaced by `packet` (JPEG and PNG that already
/// have one). Everything else in the file is copied unchanged. Err when the file isn't one of
/// those, has no packet, has extended XMP, or the packet doesn't fit.
pub fn replace_embedded(file: &[u8], packet: &str) -> Result<Vec<u8>, String> {
    if file.starts_with(&[0xff, 0xd8]) {
        jpeg_replace(file, packet)
    } else if file.starts_with(PNG_SIG) {
        png_replace(file, packet)
    } else {
        Err("XMP can be changed inside JPEG and PNG files only".into())
    }
}

fn jpeg_replace(file: &[u8], packet: &str) -> Result<Vec<u8>, String> {
    let seg_len = JPEG_XMP.len() + packet.len() + 2;
    if seg_len > 0xffff {
        return Err("the XMP is too large to go inside the picture".into());
    }
    let mut out = Vec::with_capacity(file.len() + packet.len());
    out.extend_from_slice(&file[..2]);
    let mut i = 2usize;
    let mut replaced = false;
    loop {
        let (Some(&ff), Some(&marker)) = (file.get(i), file.get(i + 1)) else { return Err("the picture file is cut short".into()) };
        if ff != 0xff {
            return Err("the picture file is damaged".into());
        }
        if marker == 0xff {
            out.push(0xff);
            i += 1;
            continue;
        }
        if marker == 0xda || marker == 0xd9 {
            break;
        }
        if (0xd0..=0xd7).contains(&marker) || marker == 0x01 {
            out.extend_from_slice(&file[i..i + 2]);
            i += 2;
            continue;
        }
        let len = file.get(i + 2..i + 4).map(|l| u16::from_be_bytes([l[0], l[1]]) as usize).ok_or("the picture file is cut short")?;
        let end = i.checked_add(2).and_then(|v| v.checked_add(len)).filter(|&e| e <= file.len() && len >= 2).ok_or("the picture file is damaged")?;
        let data = &file[i + 4..end];
        if marker == 0xe1 && data.starts_with(JPEG_XMP_EXT) {
            return Err("the picture has extended XMP".into());
        }
        if marker == 0xe1 && data.starts_with(JPEG_XMP) {
            if !replaced {
                out.extend_from_slice(&[0xff, 0xe1]);
                out.extend_from_slice(&(seg_len as u16).to_be_bytes());
                out.extend_from_slice(JPEG_XMP);
                out.extend_from_slice(packet.as_bytes());
                replaced = true;
            }
        } else {
            out.extend_from_slice(&file[i..end]);
        }
        i = end;
    }
    if !replaced {
        return Err("the picture has no XMP inside".into());
    }
    out.extend_from_slice(&file[i..]);
    Ok(out)
}

fn png_replace(file: &[u8], packet: &str) -> Result<Vec<u8>, String> {
    let mut out = Vec::with_capacity(file.len() + packet.len());
    out.extend_from_slice(PNG_SIG);
    let mut i = PNG_SIG.len();
    let mut replaced = false;
    while i < file.len() {
        let len = file.get(i..i + 4).map(|l| u32::from_be_bytes([l[0], l[1], l[2], l[3]]) as usize).ok_or("the picture file is cut short")?;
        let end = i.checked_add(12).and_then(|v| v.checked_add(len)).filter(|&e| e <= file.len()).ok_or("the picture file is damaged")?;
        let kind = &file[i + 4..i + 8];
        let data = &file[i + 8..i + 8 + len];
        if kind == b"iTXt" && data.strip_prefix(PNG_KEYWORD).is_some_and(|r| r.first() == Some(&0)) {
            if !replaced {
                let mut body = Vec::with_capacity(packet.len() + 32);
                body.extend_from_slice(PNG_KEYWORD);
                body.extend_from_slice(&[0, 0, 0, 0, 0]);
                body.extend_from_slice(packet.as_bytes());
                let blen = u32::try_from(body.len()).map_err(|_| "the XMP is too large")?;
                out.extend_from_slice(&blen.to_be_bytes());
                let mut crc_data = b"iTXt".to_vec();
                crc_data.extend_from_slice(&body);
                out.extend_from_slice(&crc_data);
                out.extend_from_slice(&crc32(&crc_data).to_be_bytes());
                replaced = true;
            }
        } else {
            out.extend_from_slice(&file[i..end]);
        }
        i = end;
        if kind == b"IEND" {
            out.extend_from_slice(&file[i..]);
            break;
        }
    }
    if !replaced {
        return Err("the picture has no XMP inside".into());
    }
    Ok(out)
}

pub(crate) fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xffff_ffffu32;
    for &b in data {
        crc ^= b as u32;
        for _ in 0..8 {
            crc = if crc & 1 != 0 { (crc >> 1) ^ 0xedb8_8320 } else { crc >> 1 };
        }
    }
    !crc
}

/// A JPEG with `packet` as a new XMP segment after its first ones (APP0 / EXIF): for exported
/// copies, which have none.
pub fn jpeg_with_xmp(jpeg: &[u8], packet: &str) -> Result<Vec<u8>, String> {
    let seg_len = JPEG_XMP.len() + packet.len() + 2;
    if seg_len > 0xffff || !jpeg.starts_with(&[0xff, 0xd8]) {
        return Err("the XMP can't be added".into());
    }
    // after the APP0 / APP1 segments at the start
    let mut i = 2usize;
    while let (Some(&0xff), Some(&m)) = (jpeg.get(i), jpeg.get(i + 1)) {
        if m != 0xe0 && m != 0xe1 {
            break;
        }
        let Some(len) = jpeg.get(i + 2..i + 4).map(|l| u16::from_be_bytes([l[0], l[1]]) as usize) else { break };
        let Some(end) = i.checked_add(2 + len).filter(|&e| e <= jpeg.len()) else { break };
        i = end;
    }
    let mut out = Vec::with_capacity(jpeg.len() + seg_len + 2);
    out.extend_from_slice(&jpeg[..i]);
    out.extend_from_slice(&[0xff, 0xe1]);
    out.extend_from_slice(&(seg_len as u16).to_be_bytes());
    out.extend_from_slice(JPEG_XMP);
    out.extend_from_slice(packet.as_bytes());
    out.extend_from_slice(&jpeg[i..]);
    Ok(out)
}

// ---------------------------------------------------------------------------------------------
// sidecars, and a picture's XMP wherever it is

/// The sidecar names a picture can have, the preferred one first: `photo.jpg.xmp`, `photo.xmp`.
pub fn sidecar_names(picture: &Path) -> Vec<PathBuf> {
    let Some(name) = picture.file_name() else { return vec![] };
    let mut long = name.to_os_string();
    long.push(".xmp");
    let mut out = vec![picture.with_file_name(long)];
    if picture.extension().is_some() {
        out.push(picture.with_extension("xmp"));
    }
    out
}

/// The picture's sidecar on disk: one of `sidecar_names`, in any letter case, found with
/// `exists` (a file test, or a lookup in a folder listing).
pub fn sidecar_of(picture: &Path, exists: &dyn Fn(&Path) -> bool) -> Option<PathBuf> {
    for p in sidecar_names(picture) {
        if exists(&p) {
            return Some(p);
        }
        let upper = p.with_extension("XMP");
        if exists(&upper) {
            return Some(upper);
        }
    }
    None
}

/// A picture's XMP as read from disk: where it was found and what it says.
#[derive(Debug, Default)]
pub struct Found {
    pub sidecar: Option<PathBuf>,
    /// The notes: the sidecar's when there is one, else the embedded packet's.
    pub notes: Option<XmpNotes>,
    /// The picture has an embedded packet.
    pub embedded: bool,
}

/// Read a picture's XMP: the sidecar when there is one (it holds the newer notes in the
/// programs that write sidecars), else the packet inside the file, with what the file's IPTC
/// says (caption, keywords) where the packet says nothing.
pub fn read_for(picture: &Path, sidecar: Option<&Path>) -> Found {
    let sidecar = sidecar.map(Path::to_path_buf);
    let side_notes = sidecar.as_deref().and_then(|s| {
        let meta = std::fs::metadata(s).ok()?;
        if meta.len() as usize > MAX_PACKET {
            return None;
        }
        let bytes = std::fs::read(s).ok()?;
        let bytes = bytes.strip_prefix(b"\xef\xbb\xbf").unwrap_or(&bytes);
        parse(std::str::from_utf8(bytes).ok()?.trim_end_matches(['\0', ' ', '\n', '\r', '\t']))
    });
    if let Some(mut n) = side_notes {
        if n.date.is_some() {
            let file = crate::filemeta::exif(picture).and_then(|m| m.capture_time);
            drop_file_date(&mut n, file);
        }
        return Found { sidecar, notes: Some(n), embedded: false };
    }
    let blocks = crate::filemeta::blocks(picture);
    let emb = embedded_of(&blocks);
    let embedded = emb.packet.is_some();
    let mut notes = emb.packet.as_deref().and_then(parse);
    if let Some(iptc) = blocks.iptc.as_deref() {
        let m = craft_metadata::parse_iptc(iptc);
        let n = notes.get_or_insert_with(XmpNotes::default);
        if n.caption.is_none() {
            n.caption = m.caption.as_deref().map(str::trim).filter(|c| !c.is_empty()).map(limit);
        }
        if n.tags.is_none() && !m.keywords.is_empty() {
            n.tags = Some(m.keywords.iter().map(|k| limit(k.trim())).filter(|k| !k.is_empty()).map(|k| vec![k]).collect());
        }
        let n = std::mem::take(n).normalized();
        notes = (!n.is_empty()).then_some(n);
    }
    if let Some(n) = notes.as_mut().filter(|n| n.date.is_some()) {
        let file = blocks.exif.as_deref().and_then(|e| craft_metadata::try_read_exif(e).ok()).and_then(|m| m.capture_time);
        drop_file_date(n, file);
        if n.is_empty() {
            notes = None;
        }
    }
    Found { sidecar, notes, embedded }
}

/// Leave out the XMP's date when it is the file's own (`file`, from its EXIF): programs copy
/// the camera's date into the XMP; only another date is one the user set.
fn drop_file_date(notes: &mut XmpNotes, file: Option<craft_metadata::DateTime>) {
    if let (Some(d), Some(f)) = (notes.date, file)
        && crate::taken::same(&d, &f)
    {
        notes.date = None;
    }
}

/// Where `write_for` put the notes.
#[derive(Debug, PartialEq)]
pub enum Written {
    Sidecar(PathBuf),
    Embedded,
}

/// Write `notes` into the picture's XMP, keeping everything else in it:
/// - into `sidecar` when the picture has one;
/// - else into the packet inside the file when it has one and `embedded_ok`;
/// - else into a new sidecar (`photo.jpg.xmp`).
///
/// Files are written to a temporary file and renamed; a packet that can't be read is never
/// replaced (Err).
pub fn write_for(picture: &Path, sidecar: Option<&Path>, notes: &XmpNotes, embedded_ok: bool) -> Result<Written, String> {
    if let Some(side) = sidecar {
        let old = std::fs::read(side).map_err(|e| e.to_string())?;
        if old.len() > MAX_PACKET {
            return Err("the XMP file is too large".into());
        }
        let old = String::from_utf8(old).map_err(|_| "the XMP file isn't text")?;
        let text = merge(Some(&old), notes, true)?;
        write_atomic(side, text.as_bytes())?;
        return Ok(Written::Sidecar(side.to_path_buf()));
    }
    if embedded_ok {
        let emb = read_embedded(picture);
        if let Some(old) = emb.packet.as_deref().filter(|_| !emb.extended) {
            match embed(picture, old, notes) {
                Ok(()) => return Ok(Written::Embedded),
                // a format or a packet that can't be changed inside the file: a sidecar instead
                Err(Embed::Unsupported) => {}
                Err(Embed::Failed(e)) => return Err(e),
            }
        }
    }
    let side = sidecar_names(picture).into_iter().next().ok_or("the picture has no file name")?;
    if side.exists() {
        return Err("a sidecar file is in the way".into());
    }
    let text = merge(None, notes, true)?;
    write_atomic(&side, text.as_bytes())?;
    Ok(Written::Sidecar(side))
}

enum Embed {
    Unsupported,
    Failed(String),
}

fn embed(picture: &Path, old: &str, notes: &XmpNotes) -> Result<(), Embed> {
    let packet = merge(Some(old), notes, false).map_err(Embed::Failed)?;
    let file = std::fs::read(picture).map_err(|e| Embed::Failed(e.to_string()))?;
    let new = replace_embedded(&file, &packet).map_err(|_| Embed::Unsupported)?;
    // check before writing: the new file holds the new packet and the same picture data
    let back = embedded_of(&craft_metadata::embedded(&new)).packet;
    if back.as_deref() != Some(packet.trim_end_matches(['\0', ' ', '\n', '\r', '\t'])) {
        return Err(Embed::Failed("the XMP could not be checked after writing; the file was not changed".into()));
    }
    write_atomic(picture, &new).map_err(Embed::Failed)
}

/// Write to a flushed temporary file next to `path`, then rename it over `path` (`craft-atomic`).
/// A replaced file keeps its permissions; through a symbolic link, the file it names is written.
fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), String> {
    craft_atomic::atomic_write(path, bytes).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests;
