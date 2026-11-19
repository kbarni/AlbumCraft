//! Keeping the notes in step with the pictures' XMP (`xmp.rs`).
//!
//! - Reading: after a scan, `index` reads the XMP of the pictures whose file or sidecar changed
//!   since last time (`Seen` stamps) on a thread of its own and sends what it found. The window
//!   applies it with `apply`: the first time a picture's XMP is seen it only fills what the
//!   notes don't have; afterwards each field that changed in the XMP since it was last seen
//!   replaces the notes' value, field by field.
//! - Writing: `to_write` gives the notes to put in a picture's XMP; `write` puts them there
//!   (sidecar, or the packet inside the file, see `xmp::write_for`) and returns the new `Seen`.

use crate::library::{Flag, MAX_RATING, Meta};
use crate::tags::key;
use crate::xmp::{self, XmpNotes};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

/// A file's date (seconds) and size, to tell when it changed.
pub type Stamp = (i64, u64);

/// What AlbumCraft last read from, or wrote to, a picture's XMP (`Store.xmp_seen`).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Seen {
    /// The picture file when it was read.
    pub file: Stamp,
    /// Its sidecar, when it had one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sidecar: Option<Stamp>,
    /// What its XMP said (normalized).
    pub notes: XmpNotes,
}

/// What `index` found for one picture: its stamps and its XMP's notes (None: no XMP, or none
/// that can be read).
#[derive(Clone, Debug)]
pub struct Read {
    pub key: String,
    pub file: Stamp,
    pub sidecar: Option<Stamp>,
    pub notes: Option<XmpNotes>,
}

fn stamp_of(path: &Path) -> Option<Stamp> {
    let m = std::fs::metadata(path).ok()?;
    let t = m.modified().ok()?.duration_since(std::time::UNIX_EPOCH).ok()?.as_secs() as i64;
    Some((t, m.len()))
}

/// Where `index` remembers the pictures that had no XMP (and no sidecar), so they aren't read
/// again while they don't change: only a cache.
pub fn none_cache_path() -> Option<PathBuf> {
    Some(dirs::cache_dir()?.join("AlbumCraft").join("xmp-none.json"))
}

/// Read the XMP of these pictures (key, date, size) where it may have changed since `seen`
/// (the stamps last seen, by key), handing batches to `send` (false: stop) until `stop`.
/// Pictures whose file and sidecar are as last seen are not read and not sent, nor are those
/// `cache` knows had no XMP at the same date and size.
pub fn index(files: Vec<(String, i64, u64)>, seen: HashMap<String, (Stamp, Option<Stamp>)>, cache: Option<PathBuf>, send: &dyn Fn(Vec<Read>) -> bool, stop: Arc<AtomicBool>) {
    let old_none: HashMap<String, Stamp> = cache.as_deref().and_then(|p| std::fs::read(p).ok()).and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default();
    let mut none: HashMap<String, Stamp> = HashMap::new();
    // one listing per folder to find the sidecars, rather than a file test per picture
    let mut listings: HashMap<PathBuf, HashSet<PathBuf>> = HashMap::new();
    let mut batch = vec![];
    let mut last_send = std::time::Instant::now();
    let mut read_any = false;
    for (key, modified, bytes) in files {
        if stop.load(Ordering::Relaxed) {
            return;
        }
        if key.starts_with(crate::imaging::SAMPLE_PREFIX) {
            continue;
        }
        let pic = Path::new(&key);
        let Some(dir) = pic.parent() else { continue };
        let names = listings.entry(dir.to_path_buf()).or_insert_with(|| {
            std::fs::read_dir(dir)
                .map(|rd| rd.flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|x| x.eq_ignore_ascii_case("xmp"))).collect())
                .unwrap_or_default()
        });
        let sidecar = xmp::sidecar_of(pic, &|p| names.contains(p));
        let side_stamp = sidecar.as_deref().and_then(stamp_of);
        let file = (modified, bytes);
        let known = seen.get(&key);
        if known == Some(&(file, side_stamp)) {
            continue;
        }
        if known.is_none() && side_stamp.is_none() && old_none.get(&key) == Some(&file) {
            none.insert(key, file);
            continue;
        }
        read_any = true;
        // a hostile file must not stop the index
        let found = std::panic::catch_unwind(|| xmp::read_for(pic, sidecar.as_deref())).unwrap_or_default();
        if found.notes.is_none() && side_stamp.is_none() {
            none.insert(key.clone(), file);
            if known.is_none() {
                continue;
            }
        }
        batch.push(Read { key, file, sidecar: side_stamp, notes: found.notes });
        if batch.len() >= 256 || last_send.elapsed().as_millis() > 250 {
            if !send(std::mem::take(&mut batch)) {
                return;
            }
            last_send = std::time::Instant::now();
        }
    }
    if !batch.is_empty() && !send(batch) {
        return;
    }
    if let Some(p) = cache
        && (read_any || none.len() != old_none.len())
        && let Ok(json) = serde_json::to_vec(&none)
    {
        if let Some(dir) = p.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let tmp = p.with_extension("json.tmp");
        if std::fs::write(&tmp, json).is_ok() {
            let _ = std::fs::rename(&tmp, &p);
        }
    }
}

/// Apply what a picture's XMP says now (`new`) to its notes, given what it said when last seen
/// (`old`, None the first time). Returns whether the notes changed. The tags' hierarchy is
/// learnt separately (`learn_hierarchy`).
pub fn apply(meta: &mut Meta, new: &XmpNotes, old: Option<&XmpNotes>) -> bool {
    let before = meta.clone();
    match old {
        None => fill(meta, new),
        Some(old) => changes(meta, new, old),
    }
    *meta != before
}

/// The first time: only what the notes don't have yet.
fn fill(meta: &mut Meta, new: &XmpNotes) {
    if meta.flag == Flag::None {
        match (new.flag, new.rating) {
            (Some(f), _) => meta.flag = f,
            (None, Some(-1)) => meta.flag = Flag::Reject,
            _ => {}
        }
    }
    if meta.rating == 0
        && let Some(r) = new.rating.filter(|r| *r > 0)
    {
        meta.rating = (r as u8).min(MAX_RATING);
    }
    if meta.caption.trim().is_empty()
        && let Some(c) = &new.caption
    {
        meta.caption = c.clone();
    }
    if let Some(tags) = &new.tags {
        add_missing(&mut meta.tags, leaves(tags));
    }
    if let Some(people) = &new.people {
        add_missing(&mut meta.people, people.clone());
    }
    if meta.place.is_none()
        && let Some([lat, lon]) = new.gps
        && let Some(at) = crate::geo::LatLon::new(lat, lon)
    {
        meta.place = Some(crate::geo::Place::set(at, ""));
    }
    if meta.edits.is_empty()
        && let Some(e) = &new.edits
    {
        meta.edits = e.clone();
    }
}

/// Later: each field that changed in the XMP replaces the notes' value.
fn changes(meta: &mut Meta, new: &XmpNotes, old: &XmpNotes) {
    if new.rating != old.rating || new.flag != old.flag {
        let flag = match (new.flag, new.rating) {
            (Some(f), _) => f,
            (None, Some(-1)) => Flag::Reject,
            // the XMP no longer says rejected
            (None, _) if meta.flag == Flag::Reject && old.rating == Some(-1) => Flag::None,
            (None, _) => meta.flag,
        };
        meta.flag = flag;
        if new.rating != old.rating {
            meta.rating = new.rating.filter(|r| *r > 0).map_or(0, |r| (r as u8).min(MAX_RATING));
        }
    }
    if new.caption != old.caption {
        meta.caption = new.caption.clone().unwrap_or_default();
    }
    if new.tags != old.tags {
        meta.tags = new.tags.as_ref().map(|t| leaves(t)).unwrap_or_default();
    }
    if new.people != old.people {
        meta.people = new.people.clone().unwrap_or_default();
        dedup_keep_order(&mut meta.people);
    }
    if new.gps != old.gps {
        meta.place = match new.gps.and_then(|[lat, lon]| crate::geo::LatLon::new(lat, lon)) {
            Some(at) => Some(crate::geo::Place::set(at, "")),
            None => Some(crate::geo::Place::Removed),
        };
    }
    if new.edits != old.edits
        && let Some(e) = &new.edits
    {
        // an edit list removed elsewhere is kept: AlbumCraft's own edits are only undone here
        meta.edits = e.clone();
    }
}

/// The tags themselves (the last level of each path), each once.
fn leaves(paths: &[Vec<String>]) -> Vec<String> {
    let mut out: Vec<String> = paths.iter().filter_map(|p| p.last().cloned()).collect();
    dedup_keep_order(&mut out);
    out
}

fn dedup_keep_order(v: &mut Vec<String>) {
    let mut seen = HashSet::new();
    v.retain(|t| seen.insert(key(t)));
}

fn add_missing(list: &mut Vec<String>, more: Vec<String>) {
    let have: HashSet<String> = list.iter().map(|t| key(t)).collect();
    let mut seen = have;
    for t in more {
        if seen.insert(key(&t)) {
            list.push(t);
        }
    }
}

/// Put tags under the parents their XMP paths give, for tags that have no parent yet (a tag the
/// user already placed stays where it is). Never makes a loop.
pub fn learn_hierarchy<'a>(tag_parents: &mut BTreeMap<String, String>, paths: impl IntoIterator<Item = &'a Vec<String>>) {
    let mut parent_of: HashMap<String, String> = tag_parents.iter().filter(|(_, p)| !p.is_empty()).map(|(c, p)| (key(c), key(p))).collect();
    for path in paths {
        for pair in path.windows(2) {
            let (parent, child) = (&pair[0], &pair[1]);
            let (pk, ck) = (key(parent), key(child));
            if pk == ck || parent_of.contains_key(&ck) {
                continue;
            }
            // no loop: the child must not be the parent or above it
            let mut up = Some(pk.clone());
            let mut steps = 0;
            let mut looped = false;
            while let Some(u) = up {
                if u == ck {
                    looped = true;
                    break;
                }
                steps += 1;
                if steps > 64 {
                    looped = true;
                    break;
                }
                up = parent_of.get(&u).cloned();
            }
            if looped {
                continue;
            }
            tag_parents.retain(|c, _| key(c) != ck);
            tag_parents.insert(child.clone(), parent.clone());
            parent_of.insert(ck, pk);
        }
    }
}

/// The path of a tag from the top of the hierarchy: `["Places", "France", "Bordeaux"]`.
pub fn path_of(tag: &str, tag_parents: &BTreeMap<String, String>) -> Vec<String> {
    let by_key: HashMap<String, &String> = tag_parents.iter().filter(|(_, p)| !p.is_empty()).map(|(c, p)| (key(c), p)).collect();
    let mut out = vec![tag.to_string()];
    let mut cur = key(tag);
    while let Some(p) = by_key.get(&cur) {
        if out.len() > 32 || out.iter().any(|t| key(t) == key(p)) {
            break;
        }
        out.push((*p).clone());
        cur = key(p);
    }
    out.reverse();
    out
}

/// The notes to write into a picture's XMP: every field AlbumCraft has (empty ones remove the
/// property). The position only when the user set or removed one (else the file's own GPS is
/// left alone).
pub fn to_write(meta: &Meta, tag_parents: &BTreeMap<String, String>) -> XmpNotes {
    let rating = if meta.flag == Flag::Reject { -1 } else { meta.rating.min(MAX_RATING) as i8 };
    let (gps, gps_removed) = match &meta.place {
        Some(crate::geo::Place::Set { lat, lon, .. }) => (Some([*lat, *lon]), false),
        Some(crate::geo::Place::Removed) => (None, true),
        None => (None, false),
    };
    XmpNotes {
        rating: Some(rating),
        flag: Some(meta.flag),
        caption: Some(meta.caption.trim().to_string()),
        tags: Some(meta.tags.iter().map(|t| path_of(t, tag_parents)).collect()),
        people: Some(meta.people.clone()),
        gps,
        gps_removed,
        edits: Some(meta.edits.clone()),
    }
}

/// The notes an exported copy carries in its XMP: rating, caption, tags (with their
/// hierarchy) and people. Not the pick / reject flag (a choice about the original) nor the
/// edits (already applied); the position follows the export's location setting.
pub fn for_copy(meta: &Meta, tag_parents: &BTreeMap<String, String>) -> XmpNotes {
    let some = |v: Vec<String>| (!v.is_empty()).then_some(v);
    XmpNotes {
        rating: (meta.rating > 0).then(|| meta.rating.min(MAX_RATING) as i8),
        caption: Some(meta.caption.trim().to_string()).filter(|c| !c.is_empty()),
        tags: some(meta.tags.clone()).map(|t| t.iter().map(|t| path_of(t, tag_parents)).collect()),
        people: some(meta.people.clone()),
        ..Default::default()
    }
}

/// Write the notes into a picture's XMP (`xmp::write_for`) and say what is there now, to be
/// kept as `Store.xmp_seen` so that the next scan doesn't take AlbumCraft's own writing for a
/// change made elsewhere.
pub fn write(key: &str, notes: &XmpNotes, embedded_ok: bool) -> Result<(xmp::Written, Seen), String> {
    let pic = Path::new(key);
    if key.starts_with(crate::imaging::SAMPLE_PREFIX) {
        return Err("a sample picture has no file".into());
    }
    let sidecar = xmp::sidecar_of(pic, &|p| p.is_file());
    let written = xmp::write_for(pic, sidecar.as_deref(), notes, embedded_ok)?;
    let file = stamp_of(pic).ok_or("the picture can't be read")?;
    let side = match &written {
        xmp::Written::Sidecar(s) => Some(s.clone()),
        xmp::Written::Embedded => sidecar,
    };
    let found = xmp::read_for(pic, side.as_deref());
    let seen = Seen { file, sidecar: side.as_deref().and_then(stamp_of), notes: found.notes.unwrap_or_default() };
    Ok((written, seen))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(v: &[&str]) -> Vec<String> {
        v.iter().map(|x| x.to_string()).collect()
    }

    #[test]
    fn first_sight_only_fills() {
        let mut m = Meta { rating: 2, caption: "mine".into(), tags: s(&["sea"]), ..Default::default() };
        let x = XmpNotes { rating: Some(5), caption: Some("theirs".into()), tags: Some(vec![s(&["Sea"]), s(&["Places", "Spain"])]), people: Some(s(&["Ann"])), ..Default::default() };
        assert!(apply(&mut m, &x, None));
        assert_eq!(m.rating, 2);
        assert_eq!(m.caption, "mine");
        assert_eq!(m.tags, s(&["sea", "Spain"]));
        assert_eq!(m.people, s(&["Ann"]));
        // nothing new: no change
        assert!(!apply(&mut m, &x, None));
    }

    #[test]
    fn later_only_what_changed() {
        let old = XmpNotes { rating: Some(3), caption: Some("a".into()), ..Default::default() };
        let mut m = Meta { rating: 4, caption: "changed here".into(), ..Default::default() };
        // the caption changed elsewhere, the rating didn't: AlbumCraft's rating stays
        let new = XmpNotes { rating: Some(3), caption: Some("b".into()), ..Default::default() };
        assert!(apply(&mut m, &new, Some(&old)));
        assert_eq!((m.rating, m.caption.as_str()), (4, "b"));
        // rejected elsewhere, then not any more
        let rej = XmpNotes { rating: Some(-1), ..new.clone() };
        apply(&mut m, &rej, Some(&new));
        assert_eq!((m.flag, m.rating), (Flag::Reject, 0));
        let back = XmpNotes { rating: Some(2), ..new.clone() };
        apply(&mut m, &back, Some(&rej));
        assert_eq!((m.flag, m.rating), (Flag::None, 2));
        // the caption removed elsewhere
        let gone = XmpNotes { caption: None, ..back.clone() };
        apply(&mut m, &gone, Some(&back));
        assert_eq!(m.caption, "");
    }

    #[test]
    fn hierarchy_learnt_without_loops_or_moving_placed_tags() {
        let mut tp = BTreeMap::new();
        tp.insert("Bordeaux".to_string(), "Gironde".to_string());
        let paths = [s(&["Places", "France", "Bordeaux"]), s(&["France", "Places"]), s(&["a", "a"])];
        learn_hierarchy(&mut tp, &paths);
        assert_eq!(tp.get("Bordeaux").map(String::as_str), Some("Gironde"));
        assert_eq!(tp.get("France").map(String::as_str), Some("Places"));
        assert!(tp.get("Places").is_none_or(|p| p.is_empty()));
        assert_eq!(path_of("bordeaux", &tp), s(&["Gironde", "bordeaux"]));
        assert_eq!(path_of("France", &tp), s(&["Places", "France"]));
    }

    #[test]
    fn written_notes_read_back_the_same() {
        let mut tp = BTreeMap::new();
        tp.insert("Spain".to_string(), "Places".to_string());
        let m = Meta { flag: Flag::Pick, rating: 3, caption: "c".into(), tags: s(&["Spain", "sea"]), people: s(&["Ann"]), ..Default::default() };
        let w = to_write(&m, &tp);
        let packet = xmp::merge(None, &w, true).unwrap();
        let back = xmp::parse(&packet).unwrap();
        let mut m2 = Meta::default();
        apply(&mut m2, &back, None);
        let mut tp2 = BTreeMap::new();
        learn_hierarchy(&mut tp2, back.tags.iter().flatten());
        assert_eq!((m2.flag, m2.rating, m2.caption.as_str()), (Flag::Pick, 3, "c"));
        let mut tags = m2.tags.clone();
        tags.sort();
        assert_eq!(tags, s(&["Spain", "sea"]));
        assert_eq!(tp2.get("Spain").map(String::as_str), Some("Places"));
    }

    #[test]
    fn index_reads_only_what_changed() {
        let dir = std::env::temp_dir().join(format!("albumcraft-xmp-index-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let pic = dir.join("p.jpg");
        let img = image::RgbImage::new(4, 4);
        img.save(&pic).unwrap();
        std::fs::write(dir.join("p.jpg.xmp"), xmp::merge(None, &XmpNotes { rating: Some(4), ..Default::default() }, true).unwrap()).unwrap();
        let key = pic.to_string_lossy().into_owned();
        let (m, b) = stamp_of(&pic).unwrap();
        let cache = dir.join("none.json");
        let run = |seen: HashMap<String, (Stamp, Option<Stamp>)>| {
            let got = std::sync::Mutex::new(vec![]);
            index(vec![(key.clone(), m, b)], seen, Some(cache.clone()), &|b| { got.lock().unwrap().extend(b); true }, Arc::new(AtomicBool::new(false)));
            got.into_inner().unwrap()
        };
        let got = run(HashMap::new());
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].notes.as_ref().and_then(|n| n.rating), Some(4));
        let again = run(HashMap::from([(key.clone(), (got[0].file, got[0].sidecar))]));
        assert!(again.is_empty());
        // without its sidecar: sent once with no notes (it had some), then cached as having none
        std::fs::remove_file(dir.join("p.jpg.xmp")).unwrap();
        let gone = run(HashMap::from([(key.clone(), (got[0].file, got[0].sidecar))]));
        assert_eq!(gone.len(), 1);
        assert!(gone[0].notes.is_none());
        assert!(run(HashMap::new()).is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
