//! The command layer: every action an agent can take, by name with JSON parameters, working on
//! a `Host` (the running app, or the library loaded without a window). The MCP server lists and
//! calls these; pictures are named by their key (the file path), which stays the same across
//! processes, never by the window's internal numbers.
//!
//! Commands validate their parameters and return `Err(message)` for anything wrong; they never
//! panic on input. Long work (export, gallery, movie) comes back as `Outcome::Later`, a closure
//! the caller runs off the UI thread.

use crate::develop;
use crate::imaging::{self, EditOp};
use crate::library::{self, Album, Flag, Folder, Photo, Store, MAX_RATING};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

/// What a command needs from the app.
pub trait Host {
    fn store(&self) -> &Store;
    /// The library to change (the host saves it).
    fn store_mut(&mut self) -> &mut Store;
    fn photos(&self) -> &[Photo];
    fn folders(&self) -> &[Folder];
    fn find(&self, key: &str) -> Option<usize>;
    /// A picture's notes or edits changed (the window refreshes its thumbnail).
    fn changed(&mut self, _id: usize) {}
    /// Look for pictures again after the library's folders changed.
    fn rescan(&mut self);
    /// Show a picture in the editor (only with a window).
    fn show(&mut self, _id: usize) -> Result<(), String> {
        Err("the AlbumCraft window isn't open".into())
    }
}

/// A command's answer: JSON for the agent and, optionally, a PNG picture.
#[derive(Debug, Default)]
pub struct Reply {
    pub value: Value,
    pub png: Option<Vec<u8>>,
}

impl Reply {
    fn json(value: Value) -> Reply {
        Reply { value, png: None }
    }
}

pub type Work = Box<dyn FnOnce() -> Result<Reply, String> + Send>;

pub enum Outcome {
    Now(Reply),
    /// Slow work to run away from the UI thread.
    Later(Work),
}

impl Outcome {
    /// Run the slow part here and now.
    pub fn finish(self) -> Result<Reply, String> {
        match self {
            Outcome::Now(r) => Ok(r),
            Outcome::Later(w) => w(),
        }
    }
}

pub struct Spec {
    pub name: &'static str,
    pub description: &'static str,
    /// JSON schema of the parameters.
    pub schema: fn() -> Value,
}

const KEYS: &str = "Pictures, by key (the file path, as listed by search_photos).";

fn keys_schema(extra: Value) -> Value {
    let mut props = json!({ "keys": { "type": "array", "items": { "type": "string" }, "description": KEYS } });
    if let (Some(p), Some(e)) = (props.as_object_mut(), extra.as_object()) {
        for (k, v) in e {
            p.insert(k.clone(), v.clone());
        }
    }
    let mut required = vec!["keys".to_string()];
    if let Some(e) = extra.as_object() {
        required.extend(e.iter().filter(|(_, v)| v.get("required") != Some(&json!(false))).map(|(k, _)| k.clone()));
    }
    // the "required": false markers above only steer this list
    if let Some(p) = props.as_object_mut() {
        for v in p.values_mut() {
            if let Some(o) = v.as_object_mut() {
                o.remove("required");
            }
        }
    }
    json!({ "type": "object", "properties": props, "required": required })
}

const EDIT_DOC: &str = "An edit step, as JSON. Forms: {\"Rotate\": n} (quarter turns, + = clockwise); \
{\"Crop\": [x0, y0, x1, y1]} (fractions 0-1 of the picture at this point); {\"Straighten\": degrees}; \
\"AutoContrast\"; \"AutoColor\"; \
{\"Adjust\": {\"temp\", \"tint\", \"highlights\", \"shadows\", \"blacks\", \"contrast\", \"saturation\", \"vibrance\", \"clarity\" (each -100..100), \
\"exposure\" (EV -3..3), \"sharpen\", \"noise\" (0..100)}} (omitted fields are 0); \
{\"Look\": {\"look\": name, \"amount\": 0-1}} with name one of vivid, soft_film, polaroid, cross_process, lomo, matte, teal_orange, high_key, cool, warm, black_white, dramatic_bw, sepia, soft_focus, glow, film_grain, focal_bw; \
{\"Decor\": {\"kind\": \"vignette\", \"amount\": -1..1, \"size\": 0..1}} | {\"kind\": \"border\", \"width\": 0..0.5, \"bottom\": 0..0.5, \"color\": [r,g,b]} | \
{\"kind\": \"shadow\", \"size\", \"opacity\", \"background\": [r,g,b]} | {\"kind\": \"rounded\", \"radius\", \"background\"} | \
{\"kind\": \"text\", \"text\", \"anchor\": 0-8 (top-left..bottom-right), \"size\": fraction of height, \"color\": [r,g,b], \"halo\": bool}.";

pub fn specs() -> Vec<Spec> {
    vec![
        Spec {
            name: "library_info",
            description: "Overview of the library: its folders on disk, number of pictures, albums and people.",
            schema: || json!({ "type": "object", "properties": {} }),
        },
        Spec {
            name: "list_folders",
            description: "The folders that contain pictures, newest first, with their picture count and description.",
            schema: || json!({ "type": "object", "properties": {} }),
        },
        Spec {
            name: "search_photos",
            description: "Find pictures. All filters are optional and combine. Words in `query` must all appear in the file or folder name, caption, tags or people.",
            schema: || {
                json!({ "type": "object", "properties": {
                    "query": { "type": "string" },
                    "folder": { "type": "string", "description": "Folder path (as in list_folders)" },
                    "album": { "type": "string", "description": "Album name" },
                    "person": { "type": "string" },
                    "flag": { "type": "string", "enum": ["pick", "reject", "none"] },
                    "min_rating": { "type": "integer", "minimum": 0, "maximum": 5 },
                    "limit": { "type": "integer", "minimum": 1, "maximum": 1000, "default": 100 },
                    "offset": { "type": "integer", "minimum": 0, "default": 0 }
                }})
            },
        },
        Spec {
            name: "get_photo",
            description: "Everything about one picture: notes, edit steps (as JSON), size and camera information.",
            schema: || json!({ "type": "object", "properties": { "key": { "type": "string" } }, "required": ["key"] }),
        },
        Spec {
            name: "set_flag",
            description: "Mark pictures as picks or rejects, or clear the mark.",
            schema: || keys_schema(json!({ "flag": { "type": "string", "enum": ["pick", "reject", "none"] } })),
        },
        Spec {
            name: "set_rating",
            description: "Give pictures 0 (no rating) to 5 stars.",
            schema: || keys_schema(json!({ "rating": { "type": "integer", "minimum": 0, "maximum": 5 } })),
        },
        Spec {
            name: "set_caption",
            description: "Set the caption of pictures (an empty caption removes it).",
            schema: || keys_schema(json!({ "caption": { "type": "string" } })),
        },
        Spec {
            name: "tag_photos",
            description: "Add or remove tags and people on pictures.",
            schema: || {
                keys_schema(json!({
                    "add_tags": { "type": "array", "items": { "type": "string" }, "required": false },
                    "remove_tags": { "type": "array", "items": { "type": "string" }, "required": false },
                    "add_people": { "type": "array", "items": { "type": "string" }, "required": false },
                    "remove_people": { "type": "array", "items": { "type": "string" }, "required": false }
                }))
            },
        },
        Spec {
            name: "create_album",
            description: "Make a new album, optionally with pictures in it.",
            schema: || {
                json!({ "type": "object", "properties": {
                    "name": { "type": "string" },
                    "keys": { "type": "array", "items": { "type": "string" } }
                }, "required": ["name"] })
            },
        },
        Spec {
            name: "add_to_album",
            description: "Add pictures to an album (by name).",
            schema: || keys_schema(json!({ "album": { "type": "string" } })),
        },
        Spec {
            name: "remove_from_album",
            description: "Take pictures out of an album (the pictures themselves stay).",
            schema: || keys_schema(json!({ "album": { "type": "string" } })),
        },
        Spec {
            name: "delete_album",
            description: "Delete an album (its pictures stay in the library).",
            schema: || json!({ "type": "object", "properties": { "album": { "type": "string" } }, "required": ["album"] }),
        },
        Spec {
            name: "add_folder",
            description: "Add a folder on disk to the library (its pictures and subfolders are scanned).",
            schema: || json!({ "type": "object", "properties": { "path": { "type": "string" } }, "required": ["path"] }),
        },
        Spec {
            name: "add_edit",
            description: EDIT_DOC,
            schema: || keys_schema(json!({ "edit": { "description": "The edit step (see the tool description)" } })),
        },
        Spec {
            name: "auto_fix",
            description: "One-click fixes: lucky (white balance, exposure, black point, vibrance), exposure, white_balance, contrast, or straighten (levels a tilted horizon; reports when nothing to do).",
            schema: || keys_schema(json!({ "fix": { "type": "string", "enum": ["lucky", "exposure", "white_balance", "contrast", "straighten"] } })),
        },
        Spec {
            name: "undo_edits",
            description: "Remove the last `count` edit steps of each picture (all of them when `all` is true). Originals are never changed.",
            schema: || {
                keys_schema(json!({
                    "count": { "type": "integer", "minimum": 1, "default": 1, "required": false },
                    "all": { "type": "boolean", "default": false, "required": false }
                }))
            },
        },
        Spec {
            name: "copy_edits",
            description: "Add the edit steps of picture `from` (red-eye fixes excepted) after the edits of each of `keys`.",
            schema: || keys_schema(json!({ "from": { "type": "string" } })),
        },
        Spec {
            name: "render_preview",
            description: "The picture as edited (or with `edits` instead of its own, to try something), as a PNG image.",
            schema: || {
                json!({ "type": "object", "properties": {
                    "key": { "type": "string" },
                    "max_size": { "type": "integer", "minimum": 64, "maximum": 2048, "default": 1024 },
                    "edits": { "type": "array", "description": "Edit steps to use instead of the picture's own" }
                }, "required": ["key"] })
            },
        },
        Spec {
            name: "export",
            description: "Save copies of pictures with their edits as JPEG files in an existing folder (never overwrites a file).",
            schema: || {
                keys_schema(json!({
                    "folder": { "type": "string" },
                    "max_size": { "type": "integer", "minimum": 0, "description": "Longest side; 0 = full size", "default": 0, "required": false },
                    "quality": { "type": "integer", "minimum": 50, "maximum": 100, "default": 90, "required": false }
                }))
            },
        },
        Spec {
            name: "make_gallery",
            description: "Make a web gallery (index.html, pictures, thumbnails) in a new folder inside `folder`; returns the page's path.",
            schema: || {
                keys_schema(json!({
                    "folder": { "type": "string" },
                    "title": { "type": "string", "required": false },
                    "style": { "type": "string", "enum": ["light", "dark", "masonry"], "default": "light", "required": false },
                    "size": { "type": "integer", "minimum": 320, "maximum": 4096, "default": 1280, "required": false },
                    "captions": { "type": "boolean", "default": true, "required": false }
                }))
            },
        },
        Spec {
            name: "make_movie",
            description: "Make an MP4 slideshow movie with crossfades, pan and zoom and optional music (needs the FFmpeg program). Fails if `path` exists.",
            schema: || {
                keys_schema(json!({
                    "path": { "type": "string", "description": "The .mp4 file to write" },
                    "hd": { "type": "boolean", "default": false, "description": "1080p instead of 720p", "required": false },
                    "seconds": { "type": "number", "minimum": 1, "maximum": 60, "default": 4, "required": false },
                    "fade": { "type": "boolean", "default": true, "required": false },
                    "motion": { "type": "boolean", "default": true, "required": false },
                    "music": { "type": "string", "description": "A music file", "required": false }
                }))
            },
        },
        Spec {
            name: "show_photo",
            description: "Open a picture in the AlbumCraft window's editor (only while the window is open).",
            schema: || json!({ "type": "object", "properties": { "key": { "type": "string" } }, "required": ["key"] }),
        },
    ]
}

// ------------------------------------------------------------------ parameter helpers

fn str_arg<'a>(args: &'a Value, name: &str) -> Result<&'a str, String> {
    args.get(name).and_then(|v| v.as_str()).ok_or_else(|| format!("`{name}` (text) is required"))
}

fn opt_str<'a>(args: &'a Value, name: &str) -> Option<&'a str> {
    args.get(name).and_then(|v| v.as_str())
}

fn opt_u64(args: &Value, name: &str) -> Result<Option<u64>, String> {
    match args.get(name) {
        None | Some(Value::Null) => Ok(None),
        Some(v) => v.as_u64().map(Some).ok_or_else(|| format!("`{name}` must be a whole number ≥ 0")),
    }
}

fn opt_bool(args: &Value, name: &str) -> Result<Option<bool>, String> {
    match args.get(name) {
        None | Some(Value::Null) => Ok(None),
        Some(v) => v.as_bool().map(Some).ok_or_else(|| format!("`{name}` must be true or false")),
    }
}

fn strings(args: &Value, name: &str) -> Result<Vec<String>, String> {
    match args.get(name) {
        None | Some(Value::Null) => Ok(vec![]),
        Some(Value::Array(a)) => a.iter().map(|v| v.as_str().map(|s| s.trim().to_string()).ok_or_else(|| format!("`{name}` must be a list of texts"))).collect(),
        Some(_) => Err(format!("`{name}` must be a list of texts")),
    }
}

/// The pictures named by `keys`, all of which must be in the library.
fn ids(host: &dyn Host, args: &Value) -> Result<Vec<usize>, String> {
    let keys = strings(args, "keys")?;
    if keys.is_empty() {
        return Err("`keys` must name at least one picture".into());
    }
    if keys.len() > 100_000 {
        return Err("too many pictures in one call".into());
    }
    keys.iter().map(|k| host.find(k).ok_or_else(|| format!("no picture with key {k:?} in the library"))).collect()
}

fn one(host: &dyn Host, key: &str) -> Result<usize, String> {
    host.find(key).ok_or_else(|| format!("no picture with key {key:?} in the library"))
}

fn flag_name(f: Flag) -> &'static str {
    match f {
        Flag::Pick => "pick",
        Flag::Reject => "reject",
        Flag::None => "none",
    }
}

fn parse_flag(s: &str) -> Result<Flag, String> {
    match s {
        "pick" => Ok(Flag::Pick),
        "reject" => Ok(Flag::Reject),
        "none" | "" => Ok(Flag::None),
        _ => Err(format!("unknown flag {s:?} (pick, reject or none)")),
    }
}

/// An edit step from JSON, checked and kept within sensible bounds.
pub fn parse_edit(v: &Value) -> Result<EditOp, String> {
    let op: EditOp = serde_json::from_value(v.clone()).map_err(|e| format!("not a valid edit step ({e})"))?;
    let finite = |x: f32| x.is_finite();
    Ok(match op {
        EditOp::Rotate(n) => EditOp::Rotate(n.rem_euclid(4)),
        EditOp::Crop(r) if r.iter().all(|x| finite(*x)) && r[2] > r[0] && r[3] > r[1] => EditOp::Crop(r.map(|x| x.clamp(0.0, 1.0))),
        EditOp::Crop(_) => return Err("Crop needs [x0, y0, x1, y1] with x1 > x0 and y1 > y0, fractions 0-1".into()),
        EditOp::Straighten(a) if finite(a) => EditOp::Straighten(a.clamp(-45.0, 45.0)),
        EditOp::Straighten(_) => return Err("Straighten needs a number of degrees".into()),
        EditOp::Redeye(r) if r.iter().all(|x| finite(*x)) => EditOp::Redeye(r.map(|x| x.clamp(0.0, 1.0))),
        EditOp::Redeye(_) => return Err("Redeye needs four numbers".into()),
        EditOp::Adjust(a) => EditOp::Adjust(a.clamped()),
        EditOp::Look { look, amount } => EditOp::Look { look, amount: if finite(amount) { amount.clamp(0.0, 1.0) } else { 1.0 } },
        EditOp::Decor(crate::decor::Decor::Text(mut t)) => {
            t.text = t.text.chars().take(200).collect();
            t.anchor = t.anchor.min(8);
            EditOp::Decor(crate::decor::Decor::Text(t))
        }
        other => other,
    })
}

fn summary(host: &dyn Host, id: usize) -> Value {
    let Some(p) = host.photos().get(id) else { return Value::Null };
    let m = host.store().meta.get(&p.key);
    let folder = host.folders().get(p.folder).map(|f| f.path.clone()).unwrap_or_default();
    json!({
        "key": p.key,
        "name": p.name,
        "folder": folder,
        "modified": iso(p.modified),
        "flag": flag_name(m.map(|m| m.flag).unwrap_or_default()),
        "rating": m.map(|m| m.rating).unwrap_or(0),
        "caption": m.map(|m| m.caption.clone()).unwrap_or_default(),
        "tags": m.map(|m| m.tags.clone()).unwrap_or_default(),
        "people": m.map(|m| m.people.clone()).unwrap_or_default(),
        "edits": m.map(|m| m.edits.len()).unwrap_or(0),
    })
}

fn iso(ts: i64) -> String {
    use chrono::TimeZone;
    chrono::Local.timestamp_opt(ts, 0).single().map(|d| d.format("%Y-%m-%dT%H:%M:%S%:z").to_string()).unwrap_or_default()
}

fn edits_of(host: &dyn Host, id: usize) -> Vec<EditOp> {
    host.photos().get(id).and_then(|p| host.store().meta.get(&p.key)).map(|m| m.edits.clone()).unwrap_or_default()
}

fn meta_mut<'a>(host: &'a mut dyn Host, id: usize) -> Option<&'a mut library::Meta> {
    let key = host.photos().get(id)?.key.clone();
    Some(host.store_mut().meta.entry(key).or_default())
}

fn album_index(store: &Store, name: &str) -> Result<usize, String> {
    store.albums.iter().position(|a| a.name == name).ok_or_else(|| format!("no album named {name:?}"))
}

fn png(img: &image::RgbaImage) -> Result<Vec<u8>, String> {
    let mut out = std::io::Cursor::new(Vec::new());
    img.write_to(&mut out, image::ImageFormat::Png).map_err(|e| e.to_string())?;
    Ok(out.into_inner())
}

fn existing_dir(path: &str) -> Result<PathBuf, String> {
    let p = PathBuf::from(path);
    if p.is_dir() { Ok(p) } else { Err(format!("{path:?} is not an existing folder")) }
}

// ------------------------------------------------------------------ running

/// Run a command. Unknown names and bad parameters are errors, never panics.
pub fn run(host: &mut dyn Host, name: &str, args: &Value) -> Result<Outcome, String> {
    let args = if args.is_null() { &json!({}) } else { args };
    if !args.is_object() {
        return Err("parameters must be a JSON object".into());
    }
    let now = |v: Value| Ok(Outcome::Now(Reply::json(v)));
    match name {
        "library_info" => {
            let s = host.store();
            let mut people: Vec<String> = s.meta.values().flat_map(|m| m.people.iter().cloned()).collect();
            people.sort();
            people.dedup();
            now(json!({
                "roots": s.roots,
                "pictures": host.photos().len(),
                "folders": host.folders().len(),
                "albums": s.albums.iter().map(|a| json!({ "name": a.name, "pictures": a.keys.len() })).collect::<Vec<_>>(),
                "people": people,
                "picks": s.meta.values().filter(|m| m.flag == Flag::Pick).count(),
            }))
        }
        "list_folders" => {
            let notes = &host.store().folder_notes;
            now(Value::Array(
                host.folders()
                    .iter()
                    .map(|f| json!({ "path": f.path, "name": f.name, "pictures": f.photos.len(), "newest": iso(f.date), "description": notes.get(&f.path) }))
                    .collect(),
            ))
        }
        "search_photos" => {
            let query = opt_str(args, "query").unwrap_or("").to_lowercase();
            let words: Vec<&str> = query.split_whitespace().collect();
            let folder = opt_str(args, "folder");
            let album: Option<std::collections::HashSet<&String>> = match opt_str(args, "album") {
                Some(a) => Some(host.store().albums.get(album_index(host.store(), a)?).map(|a| a.keys.iter().collect()).unwrap_or_default()),
                None => None,
            };
            let person = opt_str(args, "person");
            let flag = opt_str(args, "flag").map(parse_flag).transpose()?;
            let min_rating = opt_u64(args, "min_rating")?.unwrap_or(0).min(MAX_RATING as u64) as u8;
            let limit = opt_u64(args, "limit")?.unwrap_or(100).clamp(1, 1000) as usize;
            let offset = opt_u64(args, "offset")?.unwrap_or(0) as usize;
            let store = host.store();
            let matching: Vec<usize> = (0..host.photos().len())
                .filter(|&id| {
                    let p = &host.photos()[id];
                    let m = store.meta.get(&p.key);
                    let fpath = host.folders().get(p.folder).map(|f| f.path.as_str()).unwrap_or("");
                    if folder.is_some_and(|f| f != fpath) || album.as_ref().is_some_and(|a| !a.contains(&p.key)) {
                        return false;
                    }
                    if person.is_some_and(|who| !m.is_some_and(|m| m.people.iter().any(|x| x == who))) {
                        return false;
                    }
                    if flag.is_some_and(|f| m.map(|m| m.flag).unwrap_or_default() != f) || m.map(|m| m.rating).unwrap_or(0) < min_rating {
                        return false;
                    }
                    if words.is_empty() {
                        return true;
                    }
                    let mut hay = format!("{} {}", p.name, fpath).to_lowercase();
                    if let Some(m) = m {
                        hay.push(' ');
                        hay.push_str(&m.caption.to_lowercase());
                        for t in m.tags.iter().chain(&m.people) {
                            hay.push(' ');
                            hay.push_str(&t.to_lowercase());
                        }
                    }
                    words.iter().all(|w| hay.contains(w))
                })
                .collect();
            let total = matching.len();
            let page: Vec<Value> = matching.into_iter().skip(offset).take(limit).map(|id| summary(host, id)).collect();
            now(json!({ "total": total, "offset": offset, "photos": page }))
        }
        "get_photo" => {
            let id = one(host, str_arg(args, "key")?)?;
            let mut v = summary(host, id);
            let key = host.photos()[id].key.clone();
            v["edit_steps"] = serde_json::to_value(edits_of(host, id)).unwrap_or(Value::Null);
            v["size"] = library::source_dims(&key).map(|(w, h)| json!([w, h])).unwrap_or(Value::Null);
            v["raw"] = json!(crate::raw::is_raw(&key));
            if let Some(c) = (!host.photos()[id].is_sample()).then(|| crate::exifinfo::read(&key)).flatten() {
                v["camera"] = json!({
                    "camera": c.camera(),
                    "lens": c.lens,
                    "taken": c.date_taken,
                    "exposure": c.exposure_short(),
                    "aperture": c.fnumber,
                    "iso": c.iso,
                    "focal_mm": c.focal_mm,
                });
            }
            now(v)
        }
        "set_flag" => {
            let ids = ids(host, args)?;
            let flag = parse_flag(str_arg(args, "flag")?)?;
            for &id in &ids {
                if let Some(m) = meta_mut(host, id) {
                    m.flag = flag;
                }
                host.changed(id);
            }
            now(json!({ "changed": ids.len() }))
        }
        "set_rating" => {
            let ids = ids(host, args)?;
            let r = opt_u64(args, "rating")?.ok_or("`rating` (0-5) is required")?;
            if r > MAX_RATING as u64 {
                return Err("`rating` must be 0 to 5".into());
            }
            for &id in &ids {
                if let Some(m) = meta_mut(host, id) {
                    m.rating = r as u8;
                }
                host.changed(id);
            }
            now(json!({ "changed": ids.len() }))
        }
        "set_caption" => {
            let ids = ids(host, args)?;
            let caption: String = str_arg(args, "caption")?.trim().chars().take(2000).collect();
            for &id in &ids {
                if let Some(m) = meta_mut(host, id) {
                    m.caption = caption.clone();
                }
                host.changed(id);
            }
            now(json!({ "changed": ids.len() }))
        }
        "tag_photos" => {
            let ids = ids(host, args)?;
            let (add_t, rm_t) = (strings(args, "add_tags")?, strings(args, "remove_tags")?);
            let (add_p, rm_p) = (strings(args, "add_people")?, strings(args, "remove_people")?);
            for &id in &ids {
                if let Some(m) = meta_mut(host, id) {
                    m.tags.retain(|t| !rm_t.contains(t));
                    m.people.retain(|t| !rm_p.contains(t));
                    for t in add_t.iter().filter(|t| !t.is_empty()) {
                        if !m.tags.contains(t) {
                            m.tags.push(t.clone());
                        }
                    }
                    for t in add_p.iter().filter(|t| !t.is_empty()) {
                        if !m.people.contains(t) {
                            m.people.push(t.clone());
                        }
                    }
                }
                host.changed(id);
            }
            now(json!({ "changed": ids.len() }))
        }
        "create_album" => {
            let name = str_arg(args, "name")?.trim().to_string();
            if name.is_empty() {
                return Err("the album needs a name".into());
            }
            if host.store().albums.iter().any(|a| a.name == name) {
                return Err(format!("there is already an album named {name:?}"));
            }
            let keys: Vec<String> = if args.get("keys").is_some() {
                ids(host, args)?.into_iter().map(|id| host.photos()[id].key.clone()).collect()
            } else {
                vec![]
            };
            let n = keys.len();
            host.store_mut().albums.push(Album { name, keys });
            now(json!({ "pictures": n }))
        }
        "add_to_album" | "remove_from_album" => {
            let ids = ids(host, args)?;
            let keys: Vec<String> = ids.iter().map(|&id| host.photos()[id].key.clone()).collect();
            let i = album_index(host.store(), str_arg(args, "album")?)?;
            let album = &mut host.store_mut().albums[i];
            if name == "add_to_album" {
                for k in keys {
                    if !album.keys.contains(&k) {
                        album.keys.push(k);
                    }
                }
            } else {
                album.keys.retain(|k| !keys.contains(k));
            }
            let n = album.keys.len();
            now(json!({ "album_pictures": n }))
        }
        "delete_album" => {
            let i = album_index(host.store(), str_arg(args, "album")?)?;
            host.store_mut().albums.remove(i);
            now(json!({ "deleted": true }))
        }
        "add_folder" => {
            let dir = existing_dir(str_arg(args, "path")?)?;
            let dir = dir.canonicalize().map_err(|e| e.to_string())?;
            let s = dir.to_string_lossy().to_string();
            if host.store().roots.contains(&s) {
                return Err("that folder is already in the library".into());
            }
            host.store_mut().roots.push(s);
            host.rescan();
            now(json!({ "pictures": host.photos().len() }))
        }
        "add_edit" => {
            let ids = ids(host, args)?;
            let op = parse_edit(args.get("edit").ok_or("`edit` is required")?)?;
            for &id in &ids {
                if let Some(m) = meta_mut(host, id) {
                    m.edits.push(op.clone());
                }
                host.changed(id);
            }
            now(json!({ "changed": ids.len(), "edit": op.label() }))
        }
        "auto_fix" => {
            let ids = ids(host, args)?;
            let fix = str_arg(args, "fix")?.to_string();
            if !["lucky", "exposure", "white_balance", "contrast", "straighten"].contains(&fix.as_str()) {
                return Err(format!("unknown fix {fix:?}"));
            }
            let mut results = vec![];
            for &id in &ids {
                let key = host.photos()[id].key.clone();
                let mut edits = edits_of(host, id);
                let op = if fix == "contrast" {
                    Some(EditOp::AutoContrast)
                } else {
                    let base = imaging::load_source(&key, library::BIG_DIM).ok_or_else(|| format!("{key}: the picture can't be read"))?;
                    // look at the picture before its last adjustment step, which the fix replaces
                    let last_adjust = match edits.last() {
                        Some(EditOp::Adjust(a)) => Some(*a),
                        _ => None,
                    };
                    let before: Vec<EditOp> = if last_adjust.is_some() { edits[..edits.len() - 1].to_vec() } else { edits.clone() };
                    let img = imaging::apply_all(&base, &before);
                    let mut a = last_adjust.unwrap_or_default();
                    match fix.as_str() {
                        "lucky" => a = develop::Adjust { clarity: a.clarity, sharpen: a.sharpen, noise: a.noise, ..develop::lucky(&img) },
                        "exposure" => a.exposure = develop::auto_exposure(&img),
                        "white_balance" => (a.temp, a.tint) = develop::auto_white_balance(&img),
                        _ => {
                            let full = imaging::apply_all(&base, &edits);
                            let angle = develop::auto_straighten(&full).filter(|a| a.abs() >= 0.2);
                            results.push(json!({ "key": key, "straighten": angle }));
                            if let (Some(angle), Some(m)) = (angle, meta_mut(host, id)) {
                                m.edits.push(EditOp::Straighten((angle * 10.0).round() / 10.0));
                            }
                            host.changed(id);
                            continue;
                        }
                    }
                    if last_adjust.is_some() {
                        edits.pop();
                    }
                    edits.push(EditOp::Adjust(a.clamped()));
                    if let Some(m) = meta_mut(host, id) {
                        m.edits = edits;
                    }
                    host.changed(id);
                    results.push(json!({ "key": key, "adjust": a.label() }));
                    None
                };
                if let Some(op) = op {
                    if let Some(m) = meta_mut(host, id) {
                        m.edits.push(op);
                    }
                    host.changed(id);
                    results.push(json!({ "key": key, "edit": "Auto Contrast" }));
                }
            }
            now(Value::Array(results))
        }
        "undo_edits" => {
            let ids = ids(host, args)?;
            let all = opt_bool(args, "all")?.unwrap_or(false);
            let count = opt_u64(args, "count")?.unwrap_or(1) as usize;
            let mut removed = 0;
            for &id in &ids {
                if let Some(m) = meta_mut(host, id) {
                    let n = if all { m.edits.len() } else { count.min(m.edits.len()) };
                    m.edits.truncate(m.edits.len() - n);
                    removed += n;
                }
                host.changed(id);
            }
            now(json!({ "removed": removed }))
        }
        "copy_edits" => {
            let from = one(host, str_arg(args, "from")?)?;
            let ops = crate::app::copyable_edits(&edits_of(host, from));
            if ops.is_empty() {
                return Err("that picture has no edits to copy".into());
            }
            let ids = ids(host, args)?;
            for &id in &ids {
                if let Some(m) = meta_mut(host, id) {
                    m.edits.extend(ops.iter().cloned());
                }
                host.changed(id);
            }
            now(json!({ "changed": ids.len(), "steps": ops.len() }))
        }
        "render_preview" => {
            let id = one(host, str_arg(args, "key")?)?;
            let key = host.photos()[id].key.clone();
            let max = opt_u64(args, "max_size")?.unwrap_or(1024).clamp(64, 2048) as u32;
            let edits = match args.get("edits") {
                Some(Value::Array(a)) => a.iter().map(parse_edit).collect::<Result<Vec<_>, _>>()?,
                None | Some(Value::Null) => edits_of(host, id),
                Some(_) => return Err("`edits` must be a list of edit steps".into()),
            };
            Ok(Outcome::Later(Box::new(move || {
                let src = imaging::load_source(&key, max).ok_or("the picture can't be read")?;
                let mut img = imaging::apply_all(&src, &edits);
                if img.width().max(img.height()) > max {
                    let (w, h) = imaging::fit(img.width(), img.height(), max);
                    img = image::imageops::resize(&img, w, h, image::imageops::FilterType::Triangle);
                }
                let value = json!({ "key": key, "width": img.width(), "height": img.height() });
                Ok(Reply { value, png: Some(png(&img)?) })
            })))
        }
        "export" => {
            let ids = ids(host, args)?;
            let dir = existing_dir(str_arg(args, "folder")?)?;
            let max = opt_u64(args, "max_size")?.unwrap_or(0).min(20_000) as u32;
            let quality = opt_u64(args, "quality")?.unwrap_or(90).clamp(50, 100) as u8;
            let jobs: Vec<(String, String, Vec<EditOp>)> = ids.iter().map(|&id| (host.photos()[id].key.clone(), host.photos()[id].name.clone(), edits_of(host, id))).collect();
            Ok(Outcome::Later(Box::new(move || {
                let mut done = vec![];
                let mut failed = vec![];
                for (key, name, edits) in &jobs {
                    let stem = name.rsplit_once('.').map(|(s, _)| s).unwrap_or(name);
                    match crate::app::export_one(key, edits, &dir, stem, max, quality) {
                        Ok(p) => done.push(p.to_string_lossy().to_string()),
                        Err(e) => failed.push(json!({ "key": key, "error": e })),
                    }
                }
                Ok(Reply::json(json!({ "written": done, "failed": failed })))
            })))
        }
        "make_gallery" => {
            let ids = ids(host, args)?;
            let parent = existing_dir(str_arg(args, "folder")?)?;
            let style = match opt_str(args, "style").unwrap_or("light") {
                "light" => crate::gallery::Style::Light,
                "dark" => crate::gallery::Style::Dark,
                "masonry" => crate::gallery::Style::Masonry,
                s => return Err(format!("unknown style {s:?}")),
            };
            let opts = crate::gallery::Options {
                title: opt_str(args, "title").unwrap_or("Gallery").to_string(),
                style,
                size: opt_u64(args, "size")?.unwrap_or(1280).clamp(320, 4096) as u32,
                captions: opt_bool(args, "captions")?.unwrap_or(true),
            };
            let items: Vec<crate::gallery::Item> = ids
                .iter()
                .map(|&id| {
                    let p = &host.photos()[id];
                    let caption = host.store().meta.get(&p.key).map(|m| m.caption.clone()).unwrap_or_default();
                    (p.key.clone(), p.name.clone(), edits_of(host, id), caption)
                })
                .collect();
            Ok(Outcome::Later(Box::new(move || {
                let (dir, failed) = crate::gallery::write(&parent, &items, &opts)?;
                Ok(Reply::json(json!({ "page": dir.join("index.html").to_string_lossy(), "left_out": failed })))
            })))
        }
        "make_movie" => {
            let ids = ids(host, args)?;
            let path = PathBuf::from(str_arg(args, "path")?);
            if path.exists() {
                return Err(format!("{} already exists", path.display()));
            }
            if !path.parent().is_some_and(|p| p.as_os_str().is_empty() || p.is_dir()) {
                return Err("the movie's folder doesn't exist".into());
            }
            let mut s = host.store().slideshow.clone();
            if let Some(v) = args.get("seconds").and_then(|v| v.as_f64()) {
                s.interval = (v as f32).clamp(1.0, 60.0);
            }
            if let Some(b) = opt_bool(args, "fade")? {
                s.fade = b;
            }
            if let Some(b) = opt_bool(args, "motion")? {
                s.motion = b;
            }
            match opt_str(args, "music") {
                Some(m) if Path::new(m).is_file() => {
                    s.music = Some(m.to_string());
                    s.music_on = true;
                }
                Some(m) => return Err(format!("music file {m:?} not found")),
                None => s.music_on = false,
            }
            let size = if opt_bool(args, "hd")?.unwrap_or(false) { (1920, 1080) } else { (1280, 720) };
            let pictures: Vec<(String, Vec<EditOp>)> = ids.iter().map(|&id| (host.photos()[id].key.clone(), edits_of(host, id))).collect();
            Ok(Outcome::Later(Box::new(move || {
                if !crate::movie::ffmpeg_available() {
                    return Err("making movies needs the FFmpeg program, which wasn't found".into());
                }
                crate::movie::make(&path, &pictures, size, &s, &|_| {})?;
                Ok(Reply::json(json!({ "movie": path.to_string_lossy(), "seconds": crate::movie::duration(pictures.len(), &s) })))
            })))
        }
        "show_photo" => {
            let id = one(host, str_arg(args, "key")?)?;
            host.show(id)?;
            now(json!({ "shown": true }))
        }
        _ => Err(format!("unknown command {name:?}")),
    }
}

/// The library without a window: loaded from library.json, saved after every change.
pub struct Headless {
    pub path: PathBuf,
    pub store: Store,
    pub photos: Vec<Photo>,
    pub folders: Vec<Folder>,
    index: std::collections::HashMap<String, usize>,
}

impl Headless {
    /// Load the library at `path` (a missing file is an empty library; an unreadable one is an error,
    /// so that it is never overwritten).
    pub fn open(path: PathBuf) -> Result<Headless, String> {
        let store = match Store::load_from(&path) {
            library::Loaded::Store(s) => *s,
            library::Loaded::Fresh => Store::default(),
            library::Loaded::Unreadable { error, backup } => {
                return Err(format!(
                    "the library file can't be read ({error}){}",
                    backup.map(|b| format!("; it was kept as {}", b.display())).unwrap_or_default()
                ))
            }
        };
        let mut h = Headless { path, store, photos: vec![], folders: vec![], index: Default::default() };
        h.rescan();
        Ok(h)
    }

    pub fn save(&mut self) -> Result<(), String> {
        let path = self.path.clone();
        self.store.save_to(&path)
    }

    fn set_library(&mut self, scanned: Vec<library::ScannedFolder>) {
        self.photos.clear();
        self.folders.clear();
        self.index.clear();
        let mut scanned = scanned;
        scanned.sort_by_key(|(_, files)| -files.iter().map(|f| f.1).max().unwrap_or(0));
        for (path, files) in scanned {
            let fi = self.folders.len();
            let name = path.rsplit(['/', '\\']).find(|s| !s.is_empty()).unwrap_or(&path).to_string();
            let date = files.iter().map(|f| f.1).max().unwrap_or(0);
            let mut ids = vec![];
            for (key, modified, bytes) in files {
                let id = self.photos.len();
                let pname = key.rsplit(['/', '\\']).next().unwrap_or(&key).to_string();
                self.index.insert(key.clone(), id);
                self.photos.push(Photo { key, name: pname, folder: fi, modified, bytes });
                ids.push(id);
            }
            self.folders.push(Folder { path, name, date, photos: ids });
        }
    }
}

impl Host for Headless {
    fn store(&self) -> &Store {
        &self.store
    }
    fn store_mut(&mut self) -> &mut Store {
        &mut self.store
    }
    fn photos(&self) -> &[Photo] {
        &self.photos
    }
    fn folders(&self) -> &[Folder] {
        &self.folders
    }
    fn find(&self, key: &str) -> Option<usize> {
        self.index.get(key).copied()
    }
    fn rescan(&mut self) {
        let (found, _) = library::scan_roots(&self.store.roots, library::SCAN_LIMIT);
        self.set_library(found);
    }
}

/// Whether a command only reads (no need to save after it).
pub fn read_only(name: &str) -> bool {
    matches!(name, "library_info" | "list_folders" | "search_photos" | "get_photo" | "render_preview" | "export" | "make_gallery" | "make_movie" | "show_photo")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A library in a temporary folder with three small pictures.
    fn fixture(name: &str) -> (PathBuf, Headless, Vec<String>) {
        let dir = std::env::temp_dir().join(format!("albumcraft-cmd-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let pics = dir.join("pics");
        std::fs::create_dir_all(&pics).unwrap();
        let mut keys = vec![];
        for (i, n) in ["beach.jpg", "hills.jpg", "sunset.png"].iter().enumerate() {
            let p = pics.join(n);
            image::DynamicImage::ImageRgba8(crate::imaging::sample_image(i as u32 + 1, 120, 80)).to_rgb8().save(&p).unwrap();
            keys.push(p.to_string_lossy().to_string());
        }
        let mut store = Store::default();
        store.roots.push(pics.to_string_lossy().to_string());
        store.save_to(&dir.join("library.json")).unwrap();
        let h = Headless::open(dir.join("library.json")).unwrap();
        (dir, h, keys)
    }

    fn call(h: &mut Headless, name: &str, args: Value) -> Result<Value, String> {
        run(h, name, &args)?.finish().map(|r| r.value)
    }

    #[test]
    fn organize_and_search() {
        let (dir, mut h, keys) = fixture("organize");
        assert_eq!(call(&mut h, "library_info", json!({})).unwrap()["pictures"], 3);
        call(&mut h, "set_flag", json!({ "keys": [keys[0]], "flag": "pick" })).unwrap();
        call(&mut h, "set_rating", json!({ "keys": [keys[0], keys[1]], "rating": 4 })).unwrap();
        call(&mut h, "set_caption", json!({ "keys": [keys[1]], "caption": "Green hills" })).unwrap();
        call(&mut h, "tag_photos", json!({ "keys": [keys[2]], "add_tags": ["evening"], "add_people": ["Ann"] })).unwrap();
        call(&mut h, "create_album", json!({ "name": "Best", "keys": [keys[0]] })).unwrap();
        let picks = call(&mut h, "search_photos", json!({ "flag": "pick" })).unwrap();
        assert_eq!(picks["total"], 1);
        assert_eq!(call(&mut h, "search_photos", json!({ "min_rating": 4 })).unwrap()["total"], 2);
        assert_eq!(call(&mut h, "search_photos", json!({ "query": "green" })).unwrap()["photos"][0]["key"], keys[1]);
        assert_eq!(call(&mut h, "search_photos", json!({ "person": "Ann" })).unwrap()["total"], 1);
        assert_eq!(call(&mut h, "search_photos", json!({ "album": "Best" })).unwrap()["total"], 1);
        // saved and read back
        h.save().unwrap();
        let mut again = Headless::open(dir.join("library.json")).unwrap();
        assert_eq!(call(&mut again, "get_photo", json!({ "key": keys[0] })).unwrap()["flag"], "pick");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn edits_preview_and_export() {
        let (dir, mut h, keys) = fixture("edit");
        call(&mut h, "add_edit", json!({ "keys": [keys[0]], "edit": { "Rotate": 1 } })).unwrap();
        call(&mut h, "add_edit", json!({ "keys": [keys[0]], "edit": { "Adjust": { "exposure": 0.5, "vibrance": 30 } } })).unwrap();
        call(&mut h, "add_edit", json!({ "keys": [keys[0]], "edit": { "Look": { "look": "vivid", "amount": 0.5 } } })).unwrap();
        call(&mut h, "auto_fix", json!({ "keys": [keys[1]], "fix": "lucky" })).unwrap();
        call(&mut h, "copy_edits", json!({ "from": keys[0], "keys": [keys[2]] })).unwrap();
        let p = call(&mut h, "get_photo", json!({ "key": keys[2] })).unwrap();
        assert_eq!(p["edit_steps"].as_array().map(|a| a.len()), Some(3));
        let r = run(&mut h, "render_preview", &json!({ "key": keys[0], "max_size": 64 })).unwrap().finish().unwrap();
        assert_eq!((r.value["width"].as_u64(), r.value["height"].as_u64()), (Some(43), Some(64)), "rotated a quarter turn");
        assert!(r.png.is_some_and(|b| b.starts_with(b"\x89PNG")));
        let out = dir.join("out");
        std::fs::create_dir_all(&out).unwrap();
        let e = call(&mut h, "export", json!({ "keys": keys, "folder": out.to_string_lossy(), "max_size": 100 })).unwrap();
        assert_eq!(e["written"].as_array().map(|a| a.len()), Some(3));
        call(&mut h, "undo_edits", json!({ "keys": [keys[0]], "all": true })).unwrap();
        assert_eq!(call(&mut h, "get_photo", json!({ "key": keys[0] })).unwrap()["edits"], 0);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn bad_input_is_an_error_not_a_crash() {
        let (dir, mut h, keys) = fixture("bad");
        let k = &keys[0];
        let bad = [
            ("nope", json!({})),
            ("set_flag", json!({ "keys": [k], "flag": "maybe" })),
            ("set_rating", json!({ "keys": [k], "rating": 9 })),
            ("set_rating", json!({ "keys": [k], "rating": -1 })),
            ("set_flag", json!({ "keys": ["/not/in/library.jpg"], "flag": "pick" })),
            ("set_flag", json!({ "keys": [], "flag": "pick" })),
            ("set_flag", json!({ "keys": "x", "flag": "pick" })),
            ("add_edit", json!({ "keys": [k], "edit": { "Crop": [0.5, 0.5, 0.2, 0.2] } })),
            ("add_edit", json!({ "keys": [k], "edit": { "Explode": 1 } })),
            ("add_edit", json!({ "keys": [k] })),
            ("auto_fix", json!({ "keys": [k], "fix": "magic" })),
            ("add_to_album", json!({ "keys": [k], "album": "missing" })),
            ("add_folder", json!({ "path": "/no/such/folder" })),
            ("export", json!({ "keys": [k], "folder": "/no/such/folder" })),
            ("render_preview", json!({ "key": k, "edits": "x" })),
            ("make_movie", json!({ "keys": [k], "path": dir.join("library.json").to_string_lossy() })),
            ("get_photo", json!([1, 2])),
            ("show_photo", json!({ "key": k })),
        ];
        for (name, args) in bad {
            assert!(run(&mut h, name, &args).and_then(|o| o.finish()).is_err(), "{name} {args} should fail");
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn every_command_has_a_valid_schema() {
        for s in specs() {
            let schema = (s.schema)();
            assert_eq!(schema["type"], "object", "{}", s.name);
            // "required: false" markers used to build schemas must not leak into them
            assert!(!schema.to_string().contains("\"required\":false"), "{}", s.name);
        }
    }
}
