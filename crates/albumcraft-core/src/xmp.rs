//! XMP: the notes other photo programs exchange, inside picture files (embedded) or in a
//! sidecar file next to them (`photo.jpg.xmp`, `photo.xmp`).
//!
//! - `XmpNotes`: the fields AlbumCraft reads and writes (rating, pick / reject, caption, tags
//!   with their hierarchy, people, GPS position, the edit list).
//! - `parse`: a packet's text to `XmpNotes`; `merge`: `XmpNotes` written into a packet, keeping
//!   every property AlbumCraft doesn't manage as it was.
//! - `read_embedded` / `replace_embedded`: the packet inside a file (JPEG, PNG; reading also
//!   WebP, TIFF-based raws and a packet scan for the rest).
//! - `sidecar_of`, `read_for`, `write_for`: where a picture's XMP is, and writing it there.
//!
//! Every input is hostile: sizes are capped, nothing panics, a packet that can't be read is
//! never overwritten.

use crate::library::Flag;
use albumcraft_edit::imaging::EditOp;
use serde::{Deserialize, Serialize};
use std::io::{Read, Seek, SeekFrom};
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
/// How far into a file the packet scan looks.
const SCAN_BYTES: u64 = 4 << 20;
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
    /// Latitude, longitude.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gps: Option<[f64; 2]>,
    /// When writing: remove the GPS position (the user removed the place).
    #[serde(skip)]
    pub gps_removed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub edits: Option<Vec<EditOp>>,
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

/// The notes in a packet, or None when its text isn't XML or has no RDF in it.
pub fn parse(text: &str) -> Option<XmpNotes> {
    if text.len() > MAX_PACKET {
        return None;
    }
    let doc = roxmltree::Document::parse_with_options(clean(text), parse_options()).ok()?;
    let rdf = doc.descendants().find(|n| n.has_tag_name((NS_RDF, "RDF")))?;
    let mut out = XmpNotes::default();
    let mut paths: Vec<Vec<String>> = vec![];
    let mut flat: Vec<String> = vec![];
    let mut digikam: Vec<Vec<String>> = vec![];
    let mut people: Vec<String> = vec![];
    let mut any_tags = false;
    let mut any_people = false;
    let (mut lat, mut lon) = (None, None);
    for desc in rdf.children().filter(|n| n.has_tag_name((NS_RDF, "Description"))) {
        if let Some(v) = simple(desc, NS_XMP, "Rating") {
            out.rating = v.trim().parse::<f64>().ok().filter(|r| r.is_finite()).map(|r| r.round().clamp(-1.0, 5.0) as i8);
        }
        if let Some(v) = simple(desc, NS_ALBUMCRAFT, "Flag") {
            out.flag = match v.trim() {
                "pick" => Some(Flag::Pick),
                "reject" => Some(Flag::Reject),
                _ => Some(Flag::None),
            };
        }
        if let Some(v) = alt_text(desc, NS_DC, "description") {
            out.caption = Some(v);
        }
        if let Some(list) = list(desc, NS_DC, "subject") {
            any_tags = true;
            flat.extend(list);
        }
        if let Some(list) = list(desc, NS_LR, "hierarchicalSubject") {
            any_tags = true;
            paths.extend(list.iter().map(|s| split_path(s, '|')).filter(|p| !p.is_empty()));
        }
        if let Some(list) = list(desc, NS_DIGIKAM, "TagsList") {
            any_tags = true;
            digikam.extend(list.iter().map(|s| split_path(s, '/')).filter(|p| !p.is_empty()));
        }
        if let Some(list) = list(desc, NS_IPTC_EXT, "PersonInImage") {
            any_people = true;
            people.extend(list);
        }
        let faces = face_names(desc);
        any_people |= !faces.is_empty();
        people.extend(faces);
        if let Some(v) = simple(desc, NS_EXIF, "GPSLatitude") {
            lat = gps_coord(&v);
        }
        if let Some(v) = simple(desc, NS_EXIF, "GPSLongitude") {
            lon = gps_coord(&v);
        }
        if let Some(v) = simple(desc, NS_ALBUMCRAFT, "Edits") {
            out.edits = edits_from(&v);
        }
    }
    if any_tags {
        if paths.is_empty() {
            paths = digikam;
        }
        // a plain keyword that is one of the hierarchy's levels is that level, not a tag of its own
        let known: std::collections::HashSet<&str> = paths.iter().flatten().map(String::as_str).collect();
        let mut extra: Vec<Vec<String>> = flat.iter().filter(|k| !known.contains(k.as_str())).map(|k| vec![k.clone()]).collect();
        paths.append(&mut extra);
        out.tags = Some(paths);
    }
    if any_people {
        out.people = Some(people);
    }
    if let (Some(lat), Some(lon)) = (lat, lon)
        && crate::geo::LatLon::new(lat, lon).is_some()
    {
        out.gps = Some([lat, lon]);
    }
    Some(out.normalized())
}

fn parse_options<'a>() -> roxmltree::ParsingOptions<'a> {
    // no DTD: no entity expansion; a cap on the nodes for a hostile packet
    roxmltree::ParsingOptions { allow_dtd: false, nodes_limit: 1 << 20, ..Default::default() }
}

/// The text without what comes before the XML (a byte order mark, padding) and with the
/// `<?xpacket?>` instructions, which XML allows, left in.
fn clean(text: &str) -> &str {
    let t = text.trim_start_matches('\u{feff}');
    match t.find('<') {
        Some(i) => t.get(i..).unwrap_or(t).trim_end_matches(['\0', ' ', '\n', '\r', '\t']),
        None => t,
    }
}

/// A property of an rdf:Description: written as an attribute or as an element.
enum Prop<'a, 'i> {
    Attr(&'a str),
    Elem(roxmltree::Node<'a, 'i>),
}

fn prop<'a, 'i>(desc: roxmltree::Node<'a, 'i>, ns: &str, name: &str) -> Option<Prop<'a, 'i>> {
    if let Some(a) = desc.attributes().find(|a| a.namespace() == Some(ns) && a.name() == name) {
        return Some(Prop::Attr(a.value()));
    }
    desc.children().find(|n| n.has_tag_name((ns, name))).map(Prop::Elem)
}

fn text_of(n: roxmltree::Node) -> String {
    n.children().filter(|c| c.is_text()).filter_map(|c| c.text()).collect::<String>().trim().to_string()
}

fn limit(mut s: String) -> String {
    const MAX: usize = 64 * 1024;
    if s.len() > MAX {
        let mut end = MAX;
        while !s.is_char_boundary(end) {
            end -= 1;
        }
        s.truncate(end);
    }
    s
}

/// A simple value; for an element holding a container, its first item.
fn simple(desc: roxmltree::Node, ns: &str, name: &str) -> Option<String> {
    match prop(desc, ns, name)? {
        Prop::Attr(v) => Some(limit(v.to_string())),
        Prop::Elem(e) => match items(e) {
            Some(mut v) if !v.is_empty() => Some(v.swap_remove(0)),
            Some(_) => None,
            None => Some(limit(text_of(e))),
        },
    }
}

/// The `rdf:li` items of the container (Bag, Seq, Alt) inside `e`, or None when it has none.
fn items(e: roxmltree::Node) -> Option<Vec<String>> {
    let c = e.children().find(|c| c.is_element() && [(NS_RDF, "Bag"), (NS_RDF, "Seq"), (NS_RDF, "Alt")].iter().any(|t| c.has_tag_name(*t)))?;
    Some(c.children().filter(|li| li.has_tag_name((NS_RDF, "li"))).map(|li| limit(text_of(li))).filter(|s| !s.is_empty()).take(10_000).collect())
}

/// A language alternative's default text (`x-default`, else the first one).
fn alt_text(desc: roxmltree::Node, ns: &str, name: &str) -> Option<String> {
    match prop(desc, ns, name)? {
        Prop::Attr(v) => Some(limit(v.trim().to_string())),
        Prop::Elem(e) => {
            let Some(alt) = e.children().find(|c| c.has_tag_name((NS_RDF, "Alt"))) else { return Some(limit(text_of(e))) };
            let lis: Vec<roxmltree::Node> = alt.children().filter(|li| li.has_tag_name((NS_RDF, "li"))).collect();
            let pick = lis
                .iter()
                .find(|li| li.attribute(("http://www.w3.org/XML/1998/namespace", "lang")) == Some("x-default"))
                .or(lis.first())?;
            Some(limit(text_of(*pick)))
        }
    }
}

/// The items of a list; a plain value counts as a list of one.
fn list(desc: roxmltree::Node, ns: &str, name: &str) -> Option<Vec<String>> {
    match prop(desc, ns, name)? {
        Prop::Attr(v) => Some([v.trim()].into_iter().filter(|s| !s.is_empty()).map(str::to_string).collect()),
        Prop::Elem(e) => Some(items(e).unwrap_or_else(|| [text_of(e)].into_iter().filter(|s| !s.is_empty()).collect())),
    }
}

fn split_path(s: &str, sep: char) -> Vec<String> {
    s.split(sep).map(str::trim).filter(|p| !p.is_empty()).take(32).map(str::to_string).collect()
}

/// The names of the face regions (`mwg-rs:Regions`), whatever form their items are written in.
fn face_names(desc: roxmltree::Node) -> Vec<String> {
    let Some(Prop::Elem(regions)) = prop(desc, NS_MWG_RS, "Regions") else { return vec![] };
    let mut out = vec![];
    for li in regions.descendants().filter(|n| n.has_tag_name((NS_RDF, "li"))).take(1000) {
        // a region's fields are on the li itself (parseType="Resource" or attributes) or on an
        // rdf:Description inside it
        let field = |name: &str| -> Option<String> {
            simple(li, NS_MWG_RS, name).or_else(|| li.children().find(|c| c.has_tag_name((NS_RDF, "Description"))).and_then(|d| simple(d, NS_MWG_RS, name)))
        };
        let Some(name) = field("Name").map(|n| n.trim().to_string()).filter(|n| !n.is_empty()) else { continue };
        if field("Type").is_none_or(|t| t.trim() == "Face") {
            out.push(name);
        }
    }
    out
}

/// An XMP GPS coordinate: `"DDD,MM,SSk"`, `"DDD,MM.mmk"` (k = N, S, E or W), or a decimal number.
pub fn gps_coord(v: &str) -> Option<f64> {
    let v = v.trim();
    let last = v.chars().last()?;
    let (body, sign) = match last.to_ascii_uppercase() {
        'N' | 'E' => (&v[..v.len() - 1], 1.0),
        'S' | 'W' => (&v[..v.len() - 1], -1.0),
        _ => (v, 1.0),
    };
    let parts: Vec<f64> = body.split(',').map(|p| p.trim().parse::<f64>()).collect::<Result<_, _>>().ok()?;
    let value = match parts.as_slice() {
        [d] => *d,
        [d, m] => d + m / 60.0,
        [d, m, s] => d + m / 60.0 + s / 3600.0,
        _ => return None,
    };
    let value = sign * value;
    value.is_finite().then_some(value)
}

/// `"DDD,MM.mmmmmmk"`, as XMP writes GPS coordinates.
pub fn gps_text(value: f64, lat: bool) -> String {
    let hemi = match (lat, value < 0.0) {
        (true, false) => 'N',
        (true, true) => 'S',
        (false, false) => 'E',
        (false, true) => 'W',
    };
    let a = value.abs();
    let deg = a.floor();
    let min = (a - deg) * 60.0;
    format!("{},{:.6}{hemi}", deg as u32, min)
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

/// Properties AlbumCraft writes: they are removed from the old packet when the new notes set
/// them (`None` fields leave theirs).
fn managed(notes: &XmpNotes, ns: Option<&str>, name: &str) -> bool {
    match (ns, name) {
        (Some(NS_XMP), "Rating") => notes.rating.is_some(),
        (Some(NS_ALBUMCRAFT), "Flag") => notes.flag.is_some(),
        (Some(NS_ALBUMCRAFT), "Edits") => notes.edits.is_some(),
        (Some(NS_DC), "description") => notes.caption.is_some(),
        (Some(NS_DC), "subject") | (Some(NS_LR), "hierarchicalSubject") | (Some(NS_DIGIKAM), "TagsList") => notes.tags.is_some(),
        (Some(NS_IPTC_EXT), "PersonInImage") => notes.people.is_some(),
        (Some(NS_EXIF), "GPSLatitude" | "GPSLongitude") => notes.gps.is_some() || notes.gps_removed,
        _ => false,
    }
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
/// `old` that `notes` doesn't set is kept as it was, with its namespaces. `sidecar`: a sidecar
/// file (an XML declaration) rather than an embedded packet (`<?xpacket?>` around it).
/// Err when `old` can't be read: it is not replaced then.
pub fn merge(old: Option<&str>, notes: &XmpNotes, sidecar: bool) -> Result<String, String> {
    let mut kept = String::new();
    let mut has_tags_list = false;
    if let Some(old) = old.filter(|o| !o.trim().trim_matches('\0').is_empty()) {
        if old.len() > MAX_PACKET {
            return Err("the XMP is too large".into());
        }
        let src = clean(old);
        let doc = roxmltree::Document::parse_with_options(src, parse_options())
            .map_err(|e| format!("the XMP can't be read ({e})"))?;
        let rdf = doc.descendants().find(|n| n.has_tag_name((NS_RDF, "RDF"))).ok_or("the XMP has no RDF")?;
        for desc in rdf.children().filter(|n| n.is_element()) {
            if !desc.has_tag_name((NS_RDF, "Description")) {
                // another kind of RDF node: kept whole
                kept.push_str("  ");
                kept.push_str(src.get(desc.range()).unwrap_or_default());
                kept.push('\n');
                continue;
            }
            has_tags_list |= desc.attributes().any(|a| a.namespace() == Some(NS_DIGIKAM) && a.name() == "TagsList")
                || desc.children().any(|c| c.has_tag_name((NS_DIGIKAM, "TagsList")));
            let mut attrs = String::new();
            for a in desc.attributes() {
                if a.namespace() == Some(NS_RDF) || managed(notes, a.namespace(), a.name()) {
                    continue;
                }
                let Some(prefix) = a.namespace().and_then(|uri| prefix_of(desc, uri)) else { continue };
                attrs.push_str(&format!("\n    {prefix}:{}=\"{}\"", a.name(), escape(a.value())));
            }
            let mut children = String::new();
            for c in desc.children().filter(|c| c.is_element()) {
                if managed(notes, c.tag_name().namespace(), c.tag_name().name()) {
                    continue;
                }
                children.push_str("   ");
                children.push_str(src.get(c.range()).unwrap_or_default());
                children.push('\n');
            }
            if attrs.is_empty() && children.is_empty() {
                continue;
            }
            let about = desc.attribute((NS_RDF, "about")).unwrap_or_default();
            kept.push_str(&format!("  <rdf:Description rdf:about=\"{}\"", escape(about)));
            for ns in desc.namespaces() {
                if ns.uri() == "http://www.w3.org/XML/1998/namespace" {
                    continue;
                }
                match ns.name() {
                    Some("rdf") => {}
                    Some(p) => kept.push_str(&format!("\n    xmlns:{p}=\"{}\"", escape(ns.uri()))),
                    None => kept.push_str(&format!("\n    xmlns=\"{}\"", escape(ns.uri()))),
                }
            }
            kept.push_str(&attrs);
            if children.is_empty() {
                kept.push_str("/>\n");
            } else {
                kept.push_str(">\n");
                kept.push_str(&children);
                kept.push_str("  </rdf:Description>\n");
            }
        }
    }
    let ours = ours(notes, has_tags_list);
    let mut out = String::new();
    if sidecar {
        out.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    } else {
        out.push_str("<?xpacket begin=\"\u{feff}\" id=\"W5M0MpCehiHzreSzNTczkc9d\"?>\n");
    }
    out.push_str(&format!("<x:xmpmeta xmlns:x=\"{NS_X}\" x:xmptk=\"AlbumCraft {}\">\n", env!("CARGO_PKG_VERSION")));
    out.push_str(&format!(" <rdf:RDF xmlns:rdf=\"{NS_RDF}\">\n"));
    out.push_str(&kept);
    out.push_str(&ours);
    out.push_str(" </rdf:RDF>\n</x:xmpmeta>\n");
    if !sidecar {
        out.push_str("<?xpacket end=\"w\"?>");
    }
    Ok(out)
}

/// A prefix the description can use for `uri`.
fn prefix_of<'a>(desc: roxmltree::Node<'a, '_>, uri: &str) -> Option<&'a str> {
    desc.namespaces().find(|n| n.uri() == uri).and_then(|n| n.name())
}

/// The rdf:Description holding what `notes` sets (empty when it sets nothing to write).
fn ours(notes: &XmpNotes, digikam: bool) -> String {
    let mut body = String::new();
    let mut ns: Vec<(&str, &str)> = vec![];
    let use_ns = |p: &'static str, u: &'static str, ns: &mut Vec<(&str, &str)>| {
        if !ns.iter().any(|(q, _)| *q == p) {
            ns.push((p, u));
        }
    };
    let bag = |name: &str, items: &[String]| -> String {
        let mut s = format!("   <{name}>\n    <rdf:Bag>\n");
        for i in items {
            s.push_str(&format!("     <rdf:li>{}</rdf:li>\n", escape(i)));
        }
        s.push_str(&format!("    </rdf:Bag>\n   </{name}>\n"));
        s
    };
    if let Some(r) = notes.rating {
        use_ns("xmp", NS_XMP, &mut ns);
        body.push_str(&format!("   <xmp:Rating>{}</xmp:Rating>\n", r.clamp(-1, 5)));
    }
    if let Some(f) = notes.flag
        && f != Flag::None
    {
        use_ns("albumcraft", NS_ALBUMCRAFT, &mut ns);
        body.push_str(&format!("   <albumcraft:Flag>{}</albumcraft:Flag>\n", if f == Flag::Pick { "pick" } else { "reject" }));
    }
    if let Some(c) = notes.caption.as_deref().map(str::trim).filter(|c| !c.is_empty()) {
        use_ns("dc", NS_DC, &mut ns);
        body.push_str(&format!("   <dc:description>\n    <rdf:Alt>\n     <rdf:li xml:lang=\"x-default\">{}</rdf:li>\n    </rdf:Alt>\n   </dc:description>\n", escape(c)));
    }
    if let Some(tags) = notes.tags.as_ref().filter(|t| !t.is_empty()) {
        let mut leaves: Vec<String> = tags.iter().filter_map(|p| p.last().cloned()).collect();
        leaves.sort();
        leaves.dedup();
        use_ns("dc", NS_DC, &mut ns);
        body.push_str(&bag("dc:subject", &leaves));
        let hier: Vec<String> = tags.iter().filter(|p| !p.is_empty()).map(|p| p.join("|")).collect();
        use_ns("lr", NS_LR, &mut ns);
        body.push_str(&bag("lr:hierarchicalSubject", &hier));
        if digikam {
            let dk: Vec<String> = tags.iter().filter(|p| !p.is_empty()).map(|p| p.join("/")).collect();
            use_ns("digiKam", NS_DIGIKAM, &mut ns);
            body.push_str(&bag("digiKam:TagsList", &dk));
        }
    }
    if let Some(people) = notes.people.as_ref().filter(|p| !p.is_empty()) {
        use_ns("Iptc4xmpExt", NS_IPTC_EXT, &mut ns);
        body.push_str(&bag("Iptc4xmpExt:PersonInImage", people));
    }
    if let Some([lat, lon]) = notes.gps
        && !notes.gps_removed
        && crate::geo::LatLon::new(lat, lon).is_some()
    {
        use_ns("exif", NS_EXIF, &mut ns);
        body.push_str(&format!("   <exif:GPSLatitude>{}</exif:GPSLatitude>\n", gps_text(lat, true)));
        body.push_str(&format!("   <exif:GPSLongitude>{}</exif:GPSLongitude>\n", gps_text(lon, false)));
    }
    if let Some(ops) = notes.edits.as_ref().filter(|e| !e.is_empty())
        && let Ok(json) = serde_json::to_string(&EditsJson { format: EDITS_FORMAT, ops: ops.clone() })
    {
        use_ns("albumcraft", NS_ALBUMCRAFT, &mut ns);
        body.push_str(&format!("   <albumcraft:Edits>{}</albumcraft:Edits>\n", escape(&json)));
    }
    if body.is_empty() {
        return String::new();
    }
    let mut s = String::from("  <rdf:Description rdf:about=\"\"");
    for (p, u) in ns {
        s.push_str(&format!("\n    xmlns:{p}=\"{u}\""));
    }
    s.push_str(">\n");
    s.push_str(&body);
    s.push_str("  </rdf:Description>\n");
    s
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

fn packet_text(bytes: &[u8]) -> Option<String> {
    if bytes.len() > MAX_PACKET {
        return None;
    }
    let bytes = bytes.strip_prefix(b"\xef\xbb\xbf").unwrap_or(bytes);
    let s = std::str::from_utf8(bytes).ok()?;
    let s = s.trim_end_matches(['\0', ' ', '\n', '\r', '\t']);
    (!s.is_empty()).then(|| s.to_string())
}

/// The XMP inside a picture file. Reading problems give an empty answer, never an error.
pub fn read_embedded(path: &Path) -> Embedded {
    let Ok(mut f) = std::fs::File::open(path) else { return Embedded::default() };
    let mut head = [0u8; 16];
    let n = read_up_to(&mut f, &mut head);
    let head = &head[..n];
    let found = if head.starts_with(&[0xff, 0xd8]) {
        jpeg_read(&mut f)
    } else if head.starts_with(PNG_SIG) {
        png_read(&mut f).map(|p| Embedded { packet: Some(p), extended: false })
    } else if head.len() >= 12 && &head[0..4] == b"RIFF" && &head[8..12] == b"WEBP" {
        webp_read(&mut f).map(|p| Embedded { packet: Some(p), extended: false })
    } else if head.starts_with(b"II") || head.starts_with(b"MM") {
        tiff_read(&mut f).map(|p| Embedded { packet: Some(p), extended: false })
    } else {
        None
    };
    found.or_else(|| scan_read(&mut f).map(|p| Embedded { packet: Some(p), extended: false })).unwrap_or_default()
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

fn read_exact_vec(f: &mut impl Read, len: usize) -> Option<Vec<u8>> {
    if len > MAX_PACKET + 1024 {
        return None;
    }
    let mut v = vec![0u8; len];
    f.read_exact(&mut v).ok()?;
    Some(v)
}

fn jpeg_read<F: Read + Seek>(f: &mut F) -> Option<Embedded> {
    f.seek(SeekFrom::Start(2)).ok()?;
    let mut out = Embedded::default();
    for _ in 0..1000 {
        let mut m = [0u8; 2];
        f.read_exact(&mut m).ok()?;
        if m[0] != 0xff {
            break;
        }
        let marker = m[1];
        if marker == 0xff {
            // fill byte
            f.seek(SeekFrom::Current(-1)).ok()?;
            continue;
        }
        if marker == 0xda || marker == 0xd9 {
            break;
        }
        if (0xd0..=0xd7).contains(&marker) || marker == 0x01 {
            continue;
        }
        let mut l = [0u8; 2];
        f.read_exact(&mut l).ok()?;
        let len = (u16::from_be_bytes(l) as usize).checked_sub(2)?;
        if marker == 0xe1 && len >= JPEG_XMP_EXT.len() {
            let data = read_exact_vec(f, len)?;
            if out.packet.is_none()
                && let Some(p) = data.strip_prefix(JPEG_XMP)
            {
                out.packet = packet_text(p);
            } else if data.starts_with(JPEG_XMP_EXT) {
                out.extended = true;
            }
        } else {
            f.seek(SeekFrom::Current(len as i64)).ok()?;
        }
    }
    Some(out)
}

fn png_read<F: Read + Seek>(f: &mut F) -> Option<String> {
    f.seek(SeekFrom::Start(8)).ok()?;
    for _ in 0..100_000 {
        let mut h = [0u8; 8];
        f.read_exact(&mut h).ok()?;
        let len = u32::from_be_bytes([h[0], h[1], h[2], h[3]]) as u64;
        let kind = [h[4], h[5], h[6], h[7]];
        if &kind == b"IEND" {
            return None;
        }
        if &kind == b"iTXt" && (len as usize) < MAX_PACKET {
            let data = read_exact_vec(f, len as usize)?;
            f.seek(SeekFrom::Current(4)).ok()?;
            if let Some(p) = png_itxt_xmp(&data) {
                return Some(p);
            }
        } else {
            f.seek(SeekFrom::Current(len as i64 + 4)).ok()?;
        }
    }
    None
}

/// The packet of an iTXt chunk's data when it is the XMP one.
fn png_itxt_xmp(data: &[u8]) -> Option<String> {
    let rest = data.strip_prefix(PNG_KEYWORD)?.strip_prefix(b"\0")?;
    let (&compressed, rest) = rest.split_first()?;
    let (_, rest) = rest.split_first()?; // method
    // language and translated keyword, each ended by a zero
    let z = rest.iter().position(|&b| b == 0)?;
    let rest = rest.get(z + 1..)?;
    let z = rest.iter().position(|&b| b == 0)?;
    let text = rest.get(z + 1..)?;
    if compressed == 1 {
        let raw = miniz_oxide::inflate::decompress_to_vec_zlib_with_limit(text, MAX_PACKET).ok()?;
        packet_text(&raw)
    } else {
        packet_text(text)
    }
}

fn webp_read<F: Read + Seek>(f: &mut F) -> Option<String> {
    f.seek(SeekFrom::Start(12)).ok()?;
    for _ in 0..10_000 {
        let mut h = [0u8; 8];
        f.read_exact(&mut h).ok()?;
        let len = u32::from_le_bytes([h[4], h[5], h[6], h[7]]) as u64;
        if &h[0..4] == b"XMP " {
            return packet_text(&read_exact_vec(f, len as usize)?);
        }
        f.seek(SeekFrom::Current((len + (len & 1)) as i64)).ok()?;
    }
    None
}

/// TIFF and the raw formats built on it: tag 700 of the first directory.
fn tiff_read<F: Read + Seek>(f: &mut F) -> Option<String> {
    f.seek(SeekFrom::Start(0)).ok()?;
    let mut h = [0u8; 8];
    f.read_exact(&mut h).ok()?;
    let le = &h[0..2] == b"II";
    let u16_of = |b: [u8; 2]| if le { u16::from_le_bytes(b) } else { u16::from_be_bytes(b) };
    let u32_of = |b: [u8; 4]| if le { u32::from_le_bytes(b) } else { u32::from_be_bytes(b) };
    let ifd = u32_of([h[4], h[5], h[6], h[7]]);
    f.seek(SeekFrom::Start(ifd as u64)).ok()?;
    let mut c = [0u8; 2];
    f.read_exact(&mut c).ok()?;
    let count = u16_of(c).min(1000);
    for _ in 0..count {
        let mut e = [0u8; 12];
        f.read_exact(&mut e).ok()?;
        if u16_of([e[0], e[1]]) == 700 {
            let n = u32_of([e[4], e[5], e[6], e[7]]) as usize;
            let at = u32_of([e[8], e[9], e[10], e[11]]) as u64;
            if n <= 4 {
                return None;
            }
            f.seek(SeekFrom::Start(at)).ok()?;
            return packet_text(&read_exact_vec(f, n)?);
        }
    }
    None
}

/// A packet found by looking for its markers in the first bytes of the file.
fn scan_read<F: Read + Seek>(f: &mut F) -> Option<String> {
    f.seek(SeekFrom::Start(0)).ok()?;
    let mut buf = vec![];
    f.take(SCAN_BYTES).read_to_end(&mut buf).ok()?;
    let start = find(&buf, b"<x:xmpmeta").or_else(|| find(&buf, b"<x:xapmeta"))?;
    let rest = buf.get(start..)?;
    let end = find(rest, b"</x:xmpmeta>").map(|e| e + 12).or_else(|| find(rest, b"</x:xapmeta>").map(|e| e + 12))?;
    packet_text(rest.get(..end)?)
}

fn find(hay: &[u8], needle: &[u8]) -> Option<usize> {
    hay.windows(needle.len()).position(|w| w == needle)
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

fn crc32(data: &[u8]) -> u32 {
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
/// programs that write sidecars), else the packet inside the file.
pub fn read_for(picture: &Path, sidecar: Option<&Path>) -> Found {
    let sidecar = sidecar.map(Path::to_path_buf);
    let side_notes = sidecar.as_deref().and_then(|s| {
        let meta = std::fs::metadata(s).ok()?;
        if meta.len() as usize > MAX_PACKET {
            return None;
        }
        let bytes = std::fs::read(s).ok()?;
        parse(&packet_text(&bytes)?)
    });
    if side_notes.is_some() {
        return Found { sidecar, notes: side_notes, embedded: false };
    }
    let emb = read_embedded(picture);
    let embedded = emb.packet.is_some();
    Found { sidecar, notes: emb.packet.as_deref().and_then(parse), embedded }
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
    let mut cur = std::io::Cursor::new(&new);
    let back = if new.starts_with(PNG_SIG) { png_read(&mut cur) } else { jpeg_read(&mut cur).and_then(|e| e.packet) };
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
