//! The command layer: every action an agent can take, by name with JSON parameters, working on
//! a `Host` (the running app, or the library loaded without a window). The MCP server lists and
//! calls these; pictures are named by their key (the file path), which stays the same across
//! processes, never by the window's internal numbers.
//!
//! Commands validate their parameters and return `Err(message)` for anything wrong; they never
//! panic on input. Long work (export, gallery, movie) comes back as `Outcome::Later`, a closure
//! the caller runs off the UI thread.

use crate::develop;
use crate::geo::{self, LatLon, Place};
use crate::imaging::{self, EditOp};
use crate::library::{self, Album, Flag, Folder, Photo, Store, MAX_RATING};
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
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
    /// Files or folders were moved on disk, (old path, new path), and the notes followed them;
    /// a rescan comes next (the window keeps its thumbnails and selection across it).
    fn moved(&mut self, _pairs: &[(String, String)]) {}
    /// Move or rename a file or folder on disk (see `files::move_path`).
    fn move_path(&mut self, from: &str, to: &str) -> Result<(), String> {
        crate::files::move_path(from, to)
    }
    /// Show a picture in the editor (only with a window).
    fn show(&mut self, _id: usize) -> Result<(), String> {
        Err("the AlbumCraft window isn't open".into())
    }
    /// Open the Places map with only these pictures on it, titled `title` (only with a window).
    fn show_map(&mut self, _ids: &[usize], _title: String) -> Result<(), String> {
        Err("the AlbumCraft window isn't open".into())
    }
    /// The GPS positions stored in the picture files, by key (None: the file has none). Pictures
    /// not looked at yet may be missing.
    fn file_gps(&mut self) -> &HashMap<String, Option<LatLon>>;
    /// How far the background analysis of the library is (only the window runs it).
    /// The library's faces and their groups (`faces::FaceIndex`), when faces are found.
    fn faces(&self) -> Option<std::sync::Arc<crate::faces::FaceIndex>> {
        None
    }
    fn analysis(&self) -> Option<crate::analysis::Progress> {
        None
    }
    /// Remove every face found, the groups, the reference faces and the face crops from this
    /// computer (the window also clears what it shows).
    fn delete_face_data(&mut self) -> Result<(), String> {
        crate::faces::delete_face_files()
    }
}

/// The position to use for a picture: the place set by the user, else the file's GPS.
pub fn location_of(store: &Store, gps: &HashMap<String, Option<LatLon>>, key: &str) -> Option<LatLon> {
    let file = match gps.get(key) {
        Some(g) => *g,
        None => geo::file_gps(key),
    };
    geo::effective(store.meta.get(key).and_then(|m| m.place.as_ref()), file)
}

/// Set (Some) or clear (None: back to the file's own GPS) the place of these pictures.
pub fn set_place(host: &mut dyn Host, ids: &[usize], place: Option<Place>) {
    for &id in ids {
        if let Some(m) = meta_mut(host, id) {
            m.place = place.clone();
        }
        host.changed(id);
    }
}

/// Remove the location of these pictures: hide the file's GPS too (unless `back_to_file`).
pub fn remove_place(host: &mut dyn Host, ids: &[usize], back_to_file: bool) {
    for &id in ids {
        let Some(key) = host.photos().get(id).map(|p| p.key.clone()) else { continue };
        let has_file_gps = !back_to_file && host.file_gps().get(&key).copied().unwrap_or_else(|| geo::file_gps(&key)).is_some();
        if let Some(m) = meta_mut(host, id) {
            m.place = if has_file_gps { Some(Place::Removed) } else { None };
        }
        host.changed(id);
    }
}

/// The date a picture was taken as AlbumCraft uses it: the user's (`Meta.taken`), else the
/// file's (`file`, Unix seconds in this computer's time zone).
pub fn taken_of(store: &Store, key: &str, file: Option<i64>) -> Option<craft_metadata::DateTime> {
    store.meta.get(key).and_then(|m| m.taken).or_else(|| file.and_then(crate::taken::from_seconds))
}

/// Set the date these pictures were taken (Some) or go back to the files' own dates (None).
pub fn set_taken(host: &mut dyn Host, ids: &[usize], date: Option<craft_metadata::DateTime>) {
    for &id in ids {
        if let Some(m) = meta_mut(host, id) {
            m.taken = date;
        }
        host.changed(id);
    }
}

/// Move the date of these pictures by `secs` (a camera clock that was off): from the date set
/// in AlbumCraft, else the file's (`file_taken`). Returns how many moved; a picture with no
/// date, or only a year or a month, is left as it is.
pub fn shift_taken(host: &mut dyn Host, ids: &[usize], secs: i64, file_taken: &dyn Fn(&str) -> Option<i64>) -> usize {
    let mut moved = 0;
    for &id in ids {
        let Some(key) = host.photos().get(id).map(|p| p.key.clone()) else { continue };
        let Some(to) = taken_of(host.store(), &key, file_taken(&key)).and_then(|d| crate::taken::shifted(&d, secs)) else { continue };
        if let Some(m) = meta_mut(host, id) {
            m.taken = Some(to);
            moved += 1;
        }
        host.changed(id);
    }
    moved
}

/// What `move_on_disk` did: the (old, new) paths moved and why the others were not.
#[derive(Debug, Default)]
pub struct Moves {
    pub done: Vec<(String, String)>,
    pub failed: Vec<String>,
}

/// Move (or rename) files or folders on disk, (from, to) each: their notes follow them, and
/// the library is scanned again when anything moved. The window and the commands use this.
pub fn move_on_disk(host: &mut dyn Host, pairs: &[(String, String)]) -> Moves {
    let mut m = Moves::default();
    for (from, to) in pairs {
        if from == to {
            continue;
        }
        match host.move_path(from, to) {
            Ok(()) => {
                host.store_mut().moved(from, to);
                m.done.push((from.clone(), to.clone()));
            }
            Err(e) => m.failed.push(e),
        }
    }
    if !m.done.is_empty() {
        host.moved(&m.done);
        host.rescan();
    }
    m
}

/// A folder pictures or folders can be moved into: there, and part of the library.
pub fn library_folder(store: &Store, dir: &str) -> Result<(), String> {
    if !Path::new(dir).is_dir() {
        return Err(crate::msg!("move-no-folder", "The folder {path} doesn't exist.", path = dir).into());
    }
    if !store.shows_folder(dir) {
        return Err(crate::msg!("move-outside", "{path} is not in the library: choose a folder inside a library folder.", path = dir).into());
    }
    Ok(())
}

/// Take a folder out of the library without touching its files: a library folder is removed
/// from the list, a folder inside one is left out (`Store.excluded`). Returns whether it was a
/// library folder (`roots`), and its place there.
pub fn remove_folder(host: &mut dyn Host, path: &str) -> Result<Option<usize>, String> {
    let s = host.store_mut();
    let at = s.roots.iter().position(|r| Path::new(r) == Path::new(path));
    match at {
        Some(i) => {
            s.roots.remove(i);
        }
        None => s.add_excluded(path)?,
    }
    host.rescan();
    Ok(at)
}

/// Before a folder goes to the trash: a library folder is taken off the list (it would be
/// missing at every scan after).
pub fn forget_root(store: &mut Store, path: &str) {
    store.roots.retain(|r| Path::new(r) != Path::new(path));
}

/// Set a folder's caption and / or description (empty clears it).
pub fn set_folder_text(store: &mut Store, path: &str, caption: Option<&str>, description: Option<&str>) {
    for (map, text) in [(&mut store.folder_captions, caption), (&mut store.folder_notes, description)] {
        match text.map(str::trim) {
            Some("") => {
                map.remove(path);
            }
            Some(t) => {
                map.insert(path.to_string(), t.to_string());
            }
            None => {}
        }
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
{\"Perspective\": {\"vertical\": -1..1 (> 0 widens the top: straightens buildings shot from below), \"horizontal\": -1..1 (> 0 makes the right side taller)}}; \
\"AutoContrast\"; \"AutoColor\"; \
{\"Adjust\": {\"temp\", \"tint\", \"highlights\", \"shadows\", \"blacks\", \"contrast\", \"saturation\", \"vibrance\", \"clarity\" (each -100..100), \
\"exposure\" (EV -3..3), \"sharpen\", \"noise\" (0..100)}} (omitted fields are 0); \
{\"Look\": {\"look\": name, \"amount\": 0-1}} with name one of vivid, soft_film, polaroid, cross_process, lomo, matte, teal_orange, high_key, cool, warm, black_white, dramatic_bw, sepia, soft_focus, glow, film_grain, focal_bw; \
{\"Decor\": {\"kind\": \"vignette\", \"amount\": -1..1, \"size\": 0..1}} | {\"kind\": \"border\", \"width\": 0..0.5, \"bottom\": 0..0.5, \"color\": [r,g,b]} | \
{\"kind\": \"shadow\", \"size\", \"opacity\", \"background\": [r,g,b]} | {\"kind\": \"rounded\", \"radius\", \"background\"} | \
{\"kind\": \"text\", \"text\", \"anchor\": 0-8 (top-left..bottom-right), \"size\": fraction of height, \"color\": [r,g,b], \"halo\": bool}. \
A picture's steps are kept in a fixed order whatever the order they are added in: turn, straighten, perspective, crop, adjustment, look, \
then vignette, rounded corners, border, text, shadow. A picture has one of each (texts excepted): adding a step of a kind it has replaces that one \
(an Adjust replaces the whole adjustment, a Rotate is added to the turn).";

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
            description: "Find pictures. All filters are optional and combine. Words in `query` must all appear in the file or folder name, caption, tags, people or place (the name set for it, or the nearest city, its region and country).",
            schema: || {
                json!({ "type": "object", "properties": {
                    "query": { "type": "string" },
                    "folder": { "type": "string", "description": "Folder path (as in list_folders)" },
                    "album": { "type": "string", "description": "Album name" },
                    "person": { "type": "string" },
                    "tag": { "type": "string", "description": "Only pictures with this tag or a tag under it in the tag hierarchy (any letter case)" },
                    "place": { "type": "string", "description": "Only pictures whose place, nearest city, region or country contains this, e.g. \"Scotland\"" },
                    "flag": { "type": "string", "enum": ["pick", "reject", "none"] },
                    "min_rating": { "type": "integer", "minimum": 0, "maximum": 5 },
                    "has_location": { "type": "boolean", "description": "Only pictures with (true) or without (false) a location" },
                    "near": { "type": "object", "description": "Only pictures taken within radius_km of a point",
                        "properties": { "latitude": { "type": "number" }, "longitude": { "type": "number" }, "radius_km": { "type": "number", "minimum": 0, "default": 10 } },
                        "required": ["latitude", "longitude"] },
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
            name: "import_pictures",
            description: "Copy pictures from a memory card, camera or other folder (and its subfolders) into one folder, which is added to \
the library. Originals are not changed; nothing is overwritten (a taken name gets \" (2)\"; a picture already in that folder is skipped). \
Call `rescan` afterwards to see the new pictures in the library.",
            schema: || {
                json!({ "type": "object", "properties": {
                    "source": { "type": "string", "description": "The folder to import from, e.g. a card's DCIM folder" },
                    "destination": { "type": "string", "description": "The folder to copy into (made if missing; its parent must exist)" },
                    "files": { "type": "array", "items": { "type": "string" }, "description": "Only these files of the source (paths); default: all" },
                    "skip_duplicates": { "type": "boolean", "description": "Leave out pictures the library already has (same content); default true" }
                }, "required": ["source", "destination"] })
            },
        },
        Spec {
            name: "set_sort_order",
            description: "The order of the pictures in the window's library: by file name (albums as arranged), by date taken (oldest first), or by rating (highest first).",
            schema: || json!({ "type": "object", "properties": { "order": { "type": "string", "enum": ["name", "taken", "rating"] } }, "required": ["order"] }),
        },
        Spec {
            name: "rescan",
            description: "Look again for added or removed pictures in the library's folders.",
            schema: || json!({ "type": "object", "properties": {} }),
        },
        Spec {
            name: "add_edit",
            description: EDIT_DOC,
            schema: || keys_schema(json!({ "edit": { "description": "The edit step (see the tool description)" } })),
        },
        Spec {
            name: "auto_fix",
            description: "One-click fixes, each setting slider values of the picture's one adjustment (returned) rather than adding a step: lucky (white balance, exposure, contrast, vibrance), exposure (exposure, shadows), white_balance (temperature, tint), contrast (black point, white point, contrast), or straighten (sets the straighten angle and the vertical perspective from the picture's upright lines, or its horizon; null when no clear lines are found).",
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
            description: "Put the edit steps of picture `from` (red-eye fixes excepted) on each of `keys`: a step of a kind the picture has (adjustment, look, crop, border…) replaces that one.",
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
                    "quality": { "type": "integer", "minimum": 50, "maximum": 100, "default": 90, "required": false },
                    "include_location": { "type": "boolean", "description": "Write the picture's location (GPS) into the copy; default: the setting of the Export window", "required": false },
                    "include_notes": { "type": "boolean", "description": "Put the picture's rating, caption, tags and people in the copy's XMP; default: the setting of the Export window", "required": false },
                    "include_camera_info": { "type": "boolean", "description": "Copy the original's camera information (EXIF: camera, lens, exposure, date taken; not the location) into the copy; default: the setting of the Export window", "required": false },
                    "creator": { "type": "string", "description": "The creator's name written into the copy (EXIF Artist); default: the one set in Settings", "required": false },
                    "copyright": { "type": "string", "description": "The copyright notice written into the copy (EXIF Copyright); default: the one set in Settings", "required": false }
                }))
            },
        },
        Spec {
            name: "trash_rejected",
            description: "Move the files of rejected pictures (flag \"reject\") to the system's trash: they are removed from their folders, and the user can get them back from the trash until it is emptied. Only rejected pictures are moved. Ask the user first, then call with confirm: true. Without `keys`, every rejected picture of the library.",
            schema: || json!({ "type": "object", "properties": {
                "keys": { "type": "array", "items": { "type": "string" }, "description": "Rejected pictures to move (default: all rejected pictures)" },
                "confirm": { "type": "boolean", "description": "Must be true: the user agreed to move these files to the trash" }
            }, "required": ["confirm"] }),
        },
        Spec {
            name: "trash_pictures",
            description: "Move picture files to the system's trash: they are removed from their folders, and the user can get them back from the trash until it is emptied. Ask the user first, then call with confirm: true.",
            schema: || keys_schema(json!({ "confirm": { "type": "boolean", "description": "Must be true: the user agreed to move these files to the trash" } })),
        },
        Spec {
            name: "trash_folder",
            description: "Move a folder of the library, with everything in it, to the system's trash. Ask the user first, then call with confirm: true. A library folder is also taken off the library's list.",
            schema: || json!({ "type": "object", "properties": {
                "path": { "type": "string", "description": "The folder, as listed by list_folders" },
                "confirm": { "type": "boolean", "description": "Must be true: the user agreed to move this folder to the trash" }
            }, "required": ["path", "confirm"] }),
        },
        Spec {
            name: "delete_permanently",
            description: "Delete pictures or library folders for good, without the trash: they can't be brought back. Only for what the trash refused (`not_moved` of trash_pictures / trash_folder, e.g. on a network drive). Ask the user first, saying it can't be undone, then call with confirm: true.",
            schema: || json!({ "type": "object", "properties": {
                "paths": { "type": "array", "items": { "type": "string" }, "description": "Picture keys or library folders" },
                "confirm": { "type": "boolean", "description": "Must be true: the user agreed to delete these for good" }
            }, "required": ["paths", "confirm"] }),
        },
        Spec {
            name: "rename_picture",
            description: "Rename a picture's file on disk (in its folder; never over another file). Its notes, edits and albums follow it. The extension is kept when the new name leaves it out.",
            schema: || json!({ "type": "object", "properties": {
                "key": { "type": "string", "description": "The picture (its file path)" },
                "name": { "type": "string", "description": "The new file name, e.g. \"Beach at dawn\" or \"beach.jpg\"" }
            }, "required": ["key", "name"] }),
        },
        Spec {
            name: "write_xmp",
            description: "Write the pictures' notes (rating, pick / reject, caption, tags with their hierarchy, people, a location the user set, edits) into their XMP, for other photo programs. Goes into the picture's sidecar file when it has one; else into the XMP block already inside the file (JPEG, PNG) unless the user turned that off; else into a new sidecar (photo.jpg.xmp). Everything else in the XMP is kept. Only when the user asked for it.",
            schema: || keys_schema(json!({})),
        },
        Spec {
            name: "move_pictures",
            description: "Move picture files into another folder of the library, on the same disk (never over another file). Their notes, edits and albums follow them.",
            schema: || keys_schema(json!({ "folder": { "type": "string", "description": "The folder to move them into (must exist and be in the library)" } })),
        },
        Spec {
            name: "rename_folder",
            description: "Rename a folder of the library on disk. The notes of its pictures, its caption and description follow it.",
            schema: || json!({ "type": "object", "properties": {
                "path": { "type": "string", "description": "The folder, as listed by list_folders" },
                "name": { "type": "string", "description": "Its new name (not a path)" }
            }, "required": ["path", "name"] }),
        },
        Spec {
            name: "move_folder",
            description: "Move a folder of the library (with everything in it) into another library folder, on the same disk. The notes follow it.",
            schema: || json!({ "type": "object", "properties": {
                "path": { "type": "string", "description": "The folder to move" },
                "into": { "type": "string", "description": "The folder to move it into (must exist and be in the library)" }
            }, "required": ["path", "into"] }),
        },
        Spec {
            name: "set_folder_text",
            description: "Set a folder's caption (one short line, shown under its name) and / or description (shown above its pictures). An empty text removes it; a missing one is left as it is.",
            schema: || json!({ "type": "object", "properties": {
                "path": { "type": "string", "description": "The folder, as listed by list_folders" },
                "caption": { "type": "string" },
                "description": { "type": "string" }
            }, "required": ["path"] }),
        },
        Spec {
            name: "remove_folder",
            description: "Take a folder out of the library; its files and the notes on its pictures are not touched. A library folder is removed from the list; a folder inside one is left out of the scans (Settings › Folders lists it, to bring it back).",
            schema: || json!({ "type": "object", "properties": { "path": { "type": "string" } }, "required": ["path"] }),
        },
        Spec {
            name: "email_pictures",
            description: "Open a new message in the user's mail program with small copies (1200 px, JPEG) of the pictures attached, for the user to address and send. Nothing is sent by this command. Where no mail program can take attachments, the reply says where the copies are.",
            schema: || keys_schema(json!({ "subject": { "type": "string", "required": false } })),
        },
        Spec {
            name: "make_gallery",
            description: "Make a web gallery (index.html, pictures, thumbnails) in a new folder inside `folder`; returns the page's path.",
            schema: || {
                keys_schema(json!({
                    "folder": { "type": "string" },
                    "title": { "type": "string", "required": false },
                    "theme": { "type": "string", "enum": ["paper", "night", "museum", "darkroom", "instant", "ocean", "vintage", "blossom"], "default": "paper", "required": false },
                    "layout": { "type": "string", "enum": ["grid", "masonry", "rows"], "default": "grid", "required": false, "description": "grid: square thumbnails; masonry: whole pictures in columns; rows: whole pictures in rows of even height" },
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
            name: "make_collage",
            description: "Make a collage of at least two pictures and save it as a PDF, JPEG or PNG file (fails if `path` exists). Styles: pile (scattered prints), mosaic (different sizes filling the page), centre (a mosaic around the first picture), grid, film (film strips), clothesline (instant pictures on lines), fan, shape (heart, circle or star of pictures), blend (pictures through each other). Settings left out are those last used in the window.",
            schema: || {
                keys_schema(json!({
                    "path": { "type": "string", "description": "The .pdf, .jpg or .png file to write" },
                    "style": { "type": "string", "enum": ["pile", "mosaic", "centre", "grid", "film", "clothesline", "fan", "shape", "blend"], "required": false },
                    "page": { "type": "string", "enum": ["A4", "A3", "Letter", "10x15cm", "13x18cm", "20x30cm", "Square", "Wide"], "required": false },
                    "landscape": { "type": "boolean", "required": false },
                    "border": { "type": "string", "enum": ["none", "white", "instant"], "description": "For pile, clothesline, fan and grid", "required": false },
                    "shadow": { "type": "boolean", "required": false },
                    "shape": { "type": "string", "enum": ["heart", "circle", "star"], "required": false },
                    "background": { "type": "string", "description": "Colour as #rrggbb", "required": false },
                    "background2": { "type": "string", "description": "Second colour (#rrggbb): a gradient from top to bottom", "required": false },
                    "background_picture": { "type": "string", "description": "A picture file for the background (blurred)", "required": false },
                    "size": { "type": "integer", "minimum": 256, "maximum": 12000, "description": "Longest side in pixels of a JPEG or PNG", "required": false },
                    "seed": { "type": "integer", "minimum": 0, "description": "Another number gives other positions and sizes", "required": false }
                }))
            },
        },
        Spec {
            name: "make_contact_sheet",
            description: "Make a contact sheet: pages of small pictures with a title and their file names, saved as a PDF (or one JPEG / PNG per page). Fails if `path` exists.",
            schema: || {
                keys_schema(json!({
                    "path": { "type": "string", "description": "The .pdf, .jpg or .png file to write" },
                    "title": { "type": "string", "required": false },
                    "subtitle": { "type": "string", "description": "Default: the number of pictures and their dates", "required": false },
                    "picture_size": { "type": "number", "minimum": 15, "maximum": 90, "description": "Millimetres", "required": false },
                    "names": { "type": "boolean", "required": false },
                    "captions": { "type": "boolean", "required": false },
                    "dates": { "type": "boolean", "required": false },
                    "page": { "type": "string", "enum": ["A4", "A3", "Letter"], "required": false },
                    "landscape": { "type": "boolean", "required": false }
                }))
            },
        },
        Spec {
            name: "show_photo",
            description: "Open a picture in the AlbumCraft window's editor (only while the window is open).",
            schema: || json!({ "type": "object", "properties": { "key": { "type": "string" } }, "required": ["key"] }),
        },
        Spec {
            name: "show_on_map",
            description: "Open the Places map in the AlbumCraft window with only these pictures on it (only while the window is open). Pictures without a location are left out; at least one needs one.",
            schema: || keys_schema(json!({ "title": { "type": "string", "description": "Shown above the map, e.g. the folder's name", "required": false } })),
        },
        Spec {
            name: "set_location",
            description: "Set where pictures were taken (kept in the library, overriding the GPS position in the file; the file is not changed). Exported copies get it in their EXIF. Use find_place to get coordinates from a place name.",
            schema: || {
                keys_schema(json!({
                    "latitude": { "type": "number", "minimum": -90, "maximum": 90 },
                    "longitude": { "type": "number", "minimum": -180, "maximum": 180 },
                    "name": { "type": "string", "description": "Place name to show, e.g. \"Bordeaux, France\"", "required": false }
                }))
            },
        },
        Spec {
            name: "geotag_from_gpx",
            description: "Set where pictures were taken from GPS track logs (GPX files, e.g. from a phone app or a hiking GPS): each picture gets the position the track had when it was taken, interpolated between the two logged points around that time. GPX times are UTC; give `time_zone` when the pictures' times are local time of another zone than this computer's (the camera's own offset is used when it wrote one), and `clock_shift_seconds` when the camera clock was off (+120 for a clock 2 minutes slow). Pictures that already have a location are kept unless `replace_existing`. Kept in the library like set_location; the files are not changed. With `dry_run`, nothing is changed and the answer says what would be. When few are placed, `best_time_zone` in the answer is the zone that would place the most.",
            schema: || {
                keys_schema(json!({
                    "gpx_files": { "type": "array", "items": { "type": "string" }, "description": "Paths of the GPX files" },
                    "time_zone": { "type": "string", "description": "\"pictures\" (default: the zone each picture says, else this computer's), or a UTC offset such as \"+02:00\", \"-03:30\", \"UTC\"", "required": false },
                    "clock_shift_seconds": { "type": "integer", "description": "Added to every picture's time", "required": false },
                    "max_gap_seconds": { "type": "number", "description": "A picture further than this from a logged point gets no position (default 600)", "minimum": 1, "required": false },
                    "replace_existing": { "type": "boolean", "default": false, "required": false },
                    "dry_run": { "type": "boolean", "default": false, "required": false }
                }))
            },
        },
        Spec {
            name: "set_date_taken",
            description: "Set when pictures were taken, for scanned prints or a camera whose clock was wrong (kept in the library, overriding the date in the file; the file is not changed). Give one of: `date` (the same date for all: \"1978\", \"1978-07\", \"1978-07-14\" or \"1978-07-14T10:30\", local time; a year or a month alone when that is all that is known), `shift_seconds` (move each picture's date by this many seconds, e.g. 3600 for a clock an hour behind; pictures with no exact date are left alone), or `reset: true` (back to the files' own dates). Sorting, folder dates, the XMP written and exported copies' EXIF use it.",
            schema: || {
                keys_schema(json!({
                    "date": { "type": "string", "required": false },
                    "shift_seconds": { "type": "integer", "required": false },
                    "reset": { "type": "boolean", "required": false }
                }))
            },
        },
        Spec {
            name: "remove_location",
            description: "Remove the location of pictures, including the GPS position stored in the file (hidden, the file is not changed). With back_to_file, forget the location set in AlbumCraft and use the file's own GPS position again.",
            schema: || keys_schema(json!({ "back_to_file": { "type": "boolean", "default": false, "required": false } })),
        },
        Spec {
            name: "list_tags",
            description: "Every tag of the library as a tree (the tag hierarchy: a tag under another, e.g. Church under Buildings), with how many pictures have each tag itself and with the tags under it. Searching for a tag also finds the pictures of the tags under it.",
            schema: || json!({ "type": "object", "properties": {} }),
        },
        Spec {
            name: "edit_tag",
            description: "Change a tag for the whole library, as the tag manager does: rename it (renaming to an existing tag merges the two: every picture gets that tag instead), put it under another tag (`parent`; \"\" for the top level; a parent that is no tag yet is created), or delete it from every picture (`delete: true`; the tags under it move up). One change per call; the user can't undo it from the window, so say what you did.",
            schema: || json!({ "type": "object", "properties": {
                "tag": { "type": "string", "description": "The tag to change (any letter case)" },
                "rename_to": { "type": "string", "description": "New name, or an existing tag to merge into" },
                "parent": { "type": "string", "description": "The tag to put it under; empty for the top level" },
                "delete": { "type": "boolean", "description": "Take the tag off every picture" }
            }, "required": ["tag"] }),
        },
        Spec {
            name: "list_people",
            description: "The people of the library: every name (typed on pictures, or given to a group of faces found by face grouping) with how many pictures show that person, then the groups of faces nobody has named yet (a number each, largest first). Faces are found only when the user turned face grouping on (Settings > AI features); the groups are as the window last made them.",
            schema: || json!({ "type": "object", "properties": {} }),
        },
        Spec {
            name: "name_person",
            description: "Give a name to a group of faces (a number from list_people), or change it; an empty name takes it away. The name then finds the group's pictures like a name typed on them. When another group already has that name, nothing changes unless merge is true: then the two groups become one person (for good).",
            schema: || json!({ "type": "object", "properties": {
                "group": { "type": "integer", "description": "The group of faces (list_people)" },
                "name": { "type": "string" },
                "merge": { "type": "boolean", "default": false, "description": "When the name is another group's: make them one person" }
            }, "required": ["group", "name"] }),
        },
        Spec {
            name: "merge_people",
            description: "Make two groups of faces one person, for good (the same person seen in two groups: as a child and grown up, with and without glasses). Each is a group number or the name of a group. The kept group keeps its name, or takes the other's when it has none.",
            schema: || json!({ "type": "object", "properties": {
                "from": { "description": "Group number or name: the group that is added", "type": ["integer", "string"] },
                "into": { "description": "Group number or name: the group it is added to", "type": ["integer", "string"] }
            }, "required": ["from", "into"] }),
        },
        Spec {
            name: "correct_faces",
            description: "Correct faces found by face grouping, as the user does on a person's faces page: move them to a person (`group`: number or name), to a new person, say they are not that person (they are never put in that group again), ignore them (strangers, posters: left out of the groups) or say they are not faces; `reset` forgets what was decided. A face is a picture key and its place among the picture's faces (`faces` of get_photo / search_photos). The groups follow within a second while the window is open.",
            schema: || json!({ "type": "object", "properties": {
                "faces": { "type": "array", "items": { "type": "object", "properties": {
                    "picture": { "type": "string" }, "face": { "type": "integer" }
                }, "required": ["picture", "face"] } },
                "action": { "type": "string", "enum": ["move", "new_person", "not_this_person", "ignore", "not_a_face", "reset"] },
                "group": { "description": "For move and not_this_person: the group number or name", "type": ["integer", "string"] }
            }, "required": ["faces", "action"] }),
        },
        Spec {
            name: "same_person_proposals",
            description: "Groups of faces that may be one person (seen as two: childhood, glasses, profile), closest first, without those the user already answered. Answer with merge_people (yes) or not_same_person (no).",
            schema: || json!({ "type": "object", "properties": { "limit": { "type": "integer", "default": 20 } } }),
        },
        Spec {
            name: "not_same_person",
            description: "Say that two groups of faces are two people: they are not proposed as one again.",
            schema: || json!({ "type": "object", "properties": {
                "a": { "type": ["integer", "string"] }, "b": { "type": ["integer", "string"] }
            }, "required": ["a", "b"] }),
        },
        Spec {
            name: "hide_person",
            description: "Keep a group of faces (strangers, passers-by) out of the list of people, or list it again (`hidden: false`). Its faces stay grouped.",
            schema: || json!({ "type": "object", "properties": {
                "group": { "type": ["integer", "string"] }, "hidden": { "type": "boolean", "default": true }
            }, "required": ["group"] }),
        },
        Spec {
            name: "exclude_from_face_detection",
            description: "Keep pictures or whole folders (with everything under them) out of face detection: paintings in a museum, screenshots of games, posters. Their faces are not looked for, and those found before are left out of the people. `excluded: false` looks at them again (a picture inside a folder that is kept out stays out with it). Without pictures or folders, only lists what is kept out.",
            schema: || json!({ "type": "object", "properties": {
                "keys": { "type": "array", "items": { "type": "string" }, "description": "Pictures, by key" },
                "folders": { "type": "array", "items": { "type": "string" }, "description": "Folders, as listed by list_folders" },
                "excluded": { "type": "boolean", "default": true }
            } }),
        },
        Spec {
            name: "ai_status",
            description: "The AI features (faces, smart search, tag proposals; everything is computed on the user's computer): whether this build can run networks, the packs of networks that can be downloaded with their size, licence and whether they are installed, and how far the background analysis of the library is (only while the AlbumCraft window is open).",
            schema: || json!({ "type": "object", "properties": {} }),
        },
        Spec {
            name: "download_model_pack",
            description: "Download a pack of networks for the AI features (see ai_status for the packs and their sizes) from AlbumCraft's own releases; continues a download that was cut. Nothing about the user or the pictures is sent. Ask the user first: packs are 10 to 400 MB.",
            schema: || json!({ "type": "object", "properties": { "pack": { "type": "string", "description": "The pack's id, from ai_status" } }, "required": ["pack"] }),
        },
        Spec {
            name: "delete_face_data",
            description: "Delete everything computed about faces on this computer: the faces found, their vectors, the groups and the face crops. Names, corrections and typed people stay in the library; face search is turned off (the user can turn it on again in Settings). Ask the user first, then call with confirm: true.",
            schema: || json!({ "type": "object", "properties": {
                "confirm": { "type": "boolean", "description": "Must be true: the user agreed to delete the face data" }
            }, "required": ["confirm"] }),
        },
        Spec {
            name: "remove_model_pack",
            description: "Remove a downloaded pack of networks from the computer. What was computed with it is kept.",
            schema: || json!({ "type": "object", "properties": { "pack": { "type": "string" } }, "required": ["pack"] }),
        },
        Spec {
            name: "semantic_search",
            description: "Find pictures by describing what is in them (\"group photo sailing\", \"red car in the snow\", \"autumn forest\"), ranked best first; computed on the user's computer with the smart search pack (see ai_status; it must be downloaded, and only pictures already analysed are found). Without `limit`, only the matches close to the best one are given. Scores rank pictures of one search; they are not probabilities. Use search_photos for names, tags, captions, people and places.",
            schema: || {
                json!({ "type": "object", "properties": {
                    "text": { "type": "string", "description": "What the pictures show, in a few words" },
                    "limit": { "type": "integer", "minimum": 1, "maximum": 2000, "description": "How many pictures to give, best first (default: the good matches)" }
                }, "required": ["text"] })
            },
        },
        Spec {
            name: "similar_pictures",
            description: "Pictures that look like a given one (same scene, subject or event), best first, from the smart search pack's analysis (see ai_status). Without `limit`, only the close ones are given.",
            schema: || {
                json!({ "type": "object", "properties": {
                    "key": { "type": "string" },
                    "limit": { "type": "integer", "minimum": 1, "maximum": 2000 }
                }, "required": ["key"] })
            },
        },
        Spec {
            name: "propose_tags",
            description: "Tags proposed for pictures (for several, the tags most of them would get), best first, at most five: the user's own tags of the pictures that look alike or are in the same folder or place, and a built-in vocabulary (beach, wedding, dog, sunset…) in the user's language. From the smart search pack's analysis (see ai_status). Nothing is added: tag_photos adds the ones to keep. `partial`: the built-in vocabulary is still being prepared (a minute after the first call); ask again later for more.",
            schema: || json!({ "type": "object", "properties": { "keys": { "type": "array", "items": { "type": "string" } } }, "required": ["keys"] }),
        },
        Spec {
            name: "pause_analysis",
            description: "Pause (or resume) the background analysis of the library by the AI features. What is analysed is kept.",
            schema: || json!({ "type": "object", "properties": { "paused": { "type": "boolean" } }, "required": ["paused"] }),
        },
        Spec {
            name: "find_place",
            description: "Look up a place by name (\"Bordeaux\", \"Paris, Texas\") or typed coordinates, and get its coordinates. Uses the built-in list of world cities; with online, addresses too (only if online maps are turned on in AlbumCraft).",
            schema: || {
                json!({ "type": "object", "properties": {
                    "query": { "type": "string" },
                    "limit": { "type": "integer", "minimum": 1, "maximum": 20, "default": 5 },
                    "online": { "type": "boolean", "default": false }
                }, "required": ["query"] })
            },
        },
    ]
}

// ------------------------------------------------------------------ parameter helpers

fn str_arg<'a>(args: &'a Value, name: &str) -> Result<&'a str, String> {
    args.get(name).and_then(|v| v.as_str()).ok_or_else(|| format!("`{name}` (text) is required"))
}

/// A time zone as an agent writes it: "pictures", "UTC", "+02:00", "UTC-03:30", "+0530", "+2".
fn parse_zone(z: &str) -> Option<crate::geotag::Zone> {
    let z = z.trim();
    if z.eq_ignore_ascii_case("pictures") {
        return Some(crate::geotag::Zone::Pictures);
    }
    let rest = z.strip_prefix("UTC").or_else(|| z.strip_prefix("utc")).or_else(|| z.strip_prefix("GMT")).unwrap_or(z).trim();
    if rest.is_empty() || rest == "Z" {
        return Some(crate::geotag::Zone::Fixed(0));
    }
    let (sign, digits) = match rest.chars().next()? {
        '+' => (1, &rest[1..]),
        '-' | '−' => (-1, &rest[rest.chars().next()?.len_utf8()..]),
        _ => return None,
    };
    let (h, m) = match digits.split_once(':') {
        Some((h, m)) => (h.parse::<i32>().ok()?, m.parse::<i32>().ok()?),
        None if digits.len() == 4 => (digits[..2].parse().ok()?, digits[2..].parse().ok()?),
        None => (digits.parse().ok()?, 0),
    };
    (h <= 14 && (0..60).contains(&m) && h * 60 + m <= 14 * 60).then_some(crate::geotag::Zone::Fixed(sign * (h * 60 + m)))
}

/// Unix seconds as "2024-06-01T10:00:00Z".
fn utc_text(secs: f64) -> String {
    chrono::DateTime::from_timestamp(secs as i64, 0).map(|t| t.format("%Y-%m-%dT%H:%M:%SZ").to_string()).unwrap_or_default()
}

/// A group of faces given by number or by name.
fn group_arg(host: &dyn Host, v: Option<&Value>, what: &str) -> Result<u32, String> {
    match v {
        Some(Value::Number(n)) => n.as_u64().filter(|&g| g <= u32::MAX as u64).map(|g| g as u32).ok_or(format!("`{what}` is not a group number")),
        Some(Value::String(n)) => crate::faces::group_named(host.store(), n, None).ok_or(format!("no group of faces is named {n:?} (see list_people)")),
        _ => Err(format!("`{what}` must be a group number or a name")),
    }
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

/// The AI features are on, or the error an agent gets.
fn ai_on(host: &dyn Host) -> Result<(), String> {
    if host.store().ai_enabled { Ok(()) } else { Err("the AI features are turned off (Settings › AI features); the user can turn them on".into()) }
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
        EditOp::Perspective { vertical, horizontal } => {
            let v = |x: f32| if finite(x) { x.clamp(-1.0, 1.0) } else { 0.0 };
            EditOp::Perspective { vertical: v(vertical), horizontal: v(horizontal) }
        }
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
    let faces = host.faces();
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
        "people": crate::faces::people_of(m, faces.as_ref().map(|i| crate::faces::names_in(i, host.store(), &p.key)).as_ref()),
        "faces": faces.as_ref().and_then(|i| i.pictures.get(&p.key)).map(|f| f.iter().filter(|f| f.counted).map(|f| json!({
            "box": f.rect.map(|v| (v * 1000.0).round() / 1000.0),
            "group": f.group.map(|g| crate::faces::resolve(host.store(), g)),
            "name": f.group.and_then(|g| crate::faces::name_of(host.store(), g)),
        })).collect::<Vec<_>>()),
        "edits": m.map(|m| m.edits.len()).unwrap_or(0),
        "date_taken_set": m.and_then(|m| m.taken).map(|d| d.to_iso()),
    })
}

fn iso(ts: i64) -> String {
    use chrono::TimeZone;
    chrono::Local.timestamp_opt(ts, 0).single().map(|d| d.format("%Y-%m-%dT%H:%M:%S%:z").to_string()).unwrap_or_default()
}

/// What exporting one picture needs.
#[derive(Clone, Debug, Default)]
pub struct ExportJob {
    pub key: String,
    /// The file name.
    pub name: String,
    pub edits: Vec<EditOp>,
    pub place: Option<Place>,
    /// The notes for the copy's XMP (`xmp_sync::for_copy`).
    pub notes: crate::xmp::XmpNotes,
}

impl ExportJob {
    /// The file name without its extension.
    pub fn stem(&self) -> &str {
        self.name.rsplit_once('.').map(|(s, _)| s).unwrap_or(&self.name)
    }

    /// What the copy carries: the notes, and the position when `with_location` (the user's
    /// place, else the file's GPS; reads the file).
    pub fn carried(&self, with_location: bool) -> crate::export::Carried {
        let gps = if with_location { geo::effective(self.place.as_ref(), geo::file_gps(&self.key)) } else { None };
        crate::export::Carried { gps, notes: self.notes.clone() }
    }
}

pub fn export_job(host: &dyn Host, id: usize) -> ExportJob {
    let Some(p) = host.photos().get(id) else { return Default::default() };
    let store = host.store();
    let meta = store.meta.get(&p.key);
    let mut notes = meta.map(|m| crate::xmp_sync::for_copy(m, &store.tag_parents)).unwrap_or_default();
    // the people: the names typed on the picture and the people recognised in it (names only)
    let faces = host.faces();
    let people = people_union(store, faces.as_deref(), &p.key);
    notes.people = (!people.is_empty()).then_some(people);
    ExportJob { key: p.key.clone(), name: p.name.clone(), edits: edits_of(host, id), place: meta.and_then(|m| m.place.clone()), notes }
}

/// The people of a picture for its XMP (`Iptc4xmpExt:PersonInImage`): the names typed on it and
/// the names of the people recognised in it (faces found, drawn or imported), each once. Names
/// only: nothing about the faces themselves.
pub fn people_union(store: &Store, faces: Option<&crate::faces::FaceIndex>, key: &str) -> Vec<String> {
    let empty = crate::faces::FaceIndex::default();
    let recognised = crate::faces::names_in(faces.unwrap_or(&empty), store, key);
    crate::faces::people_of(store.meta.get(key), Some(&recognised))
}

/// The notes to write into a picture's XMP (`xmp_sync::to_write`).
pub fn xmp_notes_of(store: &Store, faces: Option<&crate::faces::FaceIndex>, key: &str) -> crate::xmp::XmpNotes {
    let meta = store.meta.get(key).cloned().unwrap_or_default();
    let mut notes = crate::xmp_sync::to_write(&meta, &store.tag_parents);
    notes.people = Some(people_union(store, faces, key));
    // back to the file's date: a date the XMP says (another than the file's) goes
    notes.date_removed = meta.taken.is_none() && store.xmp_seen.get(key).is_some_and(|s| s.notes.date.is_some());
    notes
}

fn edits_of(host: &dyn Host, id: usize) -> Vec<EditOp> {
    host.photos().get(id).and_then(|p| host.store().meta.get(&p.key)).map(|m| m.edits.clone()).unwrap_or_default()
}

fn meta_mut(host: &mut dyn Host, id: usize) -> Option<&mut library::Meta> {
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

/// A file to write that doesn't exist yet, in an existing folder, saved as PDF, JPEG or PNG.
fn new_output(path: &str) -> Result<PathBuf, String> {
    let p = PathBuf::from(path);
    if p.exists() {
        return Err(format!("{} already exists", p.display()));
    }
    if !p.parent().is_some_and(|d| d.as_os_str().is_empty() || d.is_dir()) {
        return Err("the file's folder doesn't exist".into());
    }
    match p.extension().map(|e| e.to_string_lossy().to_ascii_lowercase()).as_deref() {
        Some("pdf" | "png" | "jpg" | "jpeg") => Ok(p),
        _ => Err("the file must end in .pdf, .jpg or .png".into()),
    }
}

/// "#rrggbb" (or "rrggbb") as a colour.
fn hex_color(s: &str) -> Result<[u8; 3], String> {
    let h = s.trim().trim_start_matches('#');
    let bad = || format!("colour {s:?} is not #rrggbb");
    if h.len() != 6 || !h.is_ascii() {
        return Err(bad());
    }
    let byte = |i: usize| u8::from_str_radix(&h[i..i + 2], 16).map_err(|_| bad());
    Ok([byte(0)?, byte(2)?, byte(4)?])
}

/// A folder in the library: one with pictures, a library folder, or one inside it.
fn library_dir(host: &dyn Host, path: &str) -> Result<String, String> {
    let path = path.trim_end_matches(['/', '\\']);
    let known = host.folders().iter().any(|f| f.path == path) || host.store().roots.iter().any(|r| r == path);
    if known || (Path::new(path).is_dir() && host.store().shows_folder(path)) {
        Ok(path.to_string())
    } else {
        Err(format!("{path:?} is not a folder of the library"))
    }
}

fn moved_reply(m: Moves) -> Result<Outcome, String> {
    if m.done.is_empty()
        && let Some(e) = m.failed.first()
    {
        return Err(e.clone());
    }
    let moved: Vec<Value> = m.done.iter().map(|(a, b)| json!({ "from": a, "to": b })).collect();
    Ok(Outcome::Now(Reply::json(json!({ "moved": moved, "not_moved": m.failed }))))
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
            if host.faces().is_some() {
                people.extend(s.person_names.iter().filter(|(g, _)| !s.person_merged.contains_key(g)).map(|(_, n)| n.clone()));
            }
            people.sort_by_key(|n| crate::tags::key(n));
            people.dedup_by_key(|n| crate::tags::key(n));
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
            let (notes, captions) = (&host.store().folder_notes, &host.store().folder_captions);
            now(Value::Array(
                host.folders()
                    .iter()
                    .map(|f| json!({ "path": f.path, "name": f.name, "pictures": f.photos.len(), "newest": iso(f.date), "caption": captions.get(&f.path), "description": notes.get(&f.path) }))
                    .collect(),
            ))
        }
        "search_photos" => {
            let has_location = opt_bool(args, "has_location")?;
            let near = match args.get("near") {
                None | Some(Value::Null) => None,
                Some(n) => {
                    let num = |k: &str| n.get(k).and_then(|v| v.as_f64());
                    let at = LatLon::new(num("latitude").ok_or("`near` needs latitude and longitude")?, num("longitude").ok_or("`near` needs latitude and longitude")?)
                        .ok_or("`near`: latitude must be -90 to 90 and longitude -180 to 180")?;
                    let r = num("radius_km").unwrap_or(10.0);
                    if !(r.is_finite() && r >= 0.0) {
                        return Err("`near.radius_km` must be a number ≥ 0".into());
                    }
                    Some((at, r))
                }
            };
            let tag = opt_str(args, "tag").map(str::trim).filter(|t| !t.is_empty());
            // the tag and the tags under it (`tags::family`)
            let tag_family = tag.map(|t| crate::tags::family(host.store(), t));
            let place_q = opt_str(args, "place").map(geo::fold).filter(|t| !t.trim().is_empty());
            let query = opt_str(args, "query").unwrap_or("").to_lowercase();
            // positions only when asked for (looking at every file can take a while)
            let located: Option<Vec<Option<LatLon>>> = if has_location.is_some() || near.is_some() || place_q.is_some() || !query.trim().is_empty() {
                let gps = host.file_gps().clone();
                Some(host.photos().iter().map(|p| location_of(host.store(), &gps, &p.key)).collect())
            } else {
                None
            };
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
            let faces_in = if person.is_some() { host.faces() } else { None };
            let matching: Vec<usize> = (0..host.photos().len())
                .filter(|&id| {
                    let p = &host.photos()[id];
                    let m = store.meta.get(&p.key);
                    let fpath = host.folders().get(p.folder).map(|f| f.path.as_str()).unwrap_or("");
                    if folder.is_some_and(|f| f != fpath) || album.as_ref().is_some_and(|a| !a.contains(&p.key)) {
                        return false;
                    }
                    if person.is_some_and(|who| !crate::faces::people_of(m, faces_in.as_ref().map(|i| crate::faces::names_in(i, store, &p.key)).as_ref()).iter().any(|x| crate::tags::key(x) == crate::tags::key(who))) {
                        return false;
                    }
                    if flag.is_some_and(|f| m.map(|m| m.flag).unwrap_or_default() != f) || m.map(|m| m.rating).unwrap_or(0) < min_rating {
                        return false;
                    }
                    if tag_family.as_ref().is_some_and(|f| !m.is_some_and(|m| m.tags.iter().any(|x| f.contains(&crate::tags::key(x))))) {
                        return false;
                    }
                    let at = located.as_ref().and_then(|l| l.get(id).copied().flatten());
                    let place_words = || at.map(|a| geo::place_terms(m.and_then(|m| m.place.as_ref()), a)).unwrap_or_default();
                    if let Some(q) = &place_q
                        && !place_words().iter().any(|w| geo::fold(w).contains(q.as_str()))
                    {
                        return false;
                    }
                    if located.is_some() {
                        if has_location.is_some_and(|want| want != at.is_some()) {
                            return false;
                        }
                        if let Some((c, r)) = near
                            && !at.is_some_and(|a| a.km_to(c) <= r)
                        {
                            return false;
                        }
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
                    for w in place_words() {
                        hay.push(' ');
                        hay.push_str(&w.to_lowercase());
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
            let place = host.store().meta.get(&key).and_then(|m| m.place.clone());
            let file = geo::file_gps(&key);
            v["location"] = match geo::effective(place.as_ref(), file) {
                Some(at) => json!({
                    "latitude": at.lat,
                    "longitude": at.lon,
                    "place": geo::describe(place.as_ref(), at),
                    "source": if place.is_some() { "set in AlbumCraft" } else { "the file's GPS" },
                }),
                None => Value::Null,
            };
            now(v)
        }
        "set_location" => {
            let ids = ids(host, args)?;
            let num = |k: &str| args.get(k).and_then(|v| v.as_f64()).ok_or_else(|| format!("`{k}` (a number) is required"));
            let at = LatLon::new(num("latitude")?, num("longitude")?).ok_or("latitude must be -90 to 90 and longitude -180 to 180")?;
            let name = opt_str(args, "name").unwrap_or("");
            set_place(host, &ids, Some(Place::set(at, name)));
            now(json!({ "changed": ids.len(), "place": geo::describe(Some(&Place::set(at, name)), at) }))
        }
        "geotag_from_gpx" => {
            let ids = ids(host, args)?;
            let files = strings(args, "gpx_files")?;
            if files.is_empty() {
                return Err("`gpx_files` must name at least one GPX file".into());
            }
            let zone = match opt_str(args, "time_zone") {
                None => crate::geotag::Zone::Pictures,
                Some(z) => parse_zone(z).ok_or("`time_zone` must be \"pictures\" or a UTC offset such as \"+02:00\"")?,
            };
            let clock_shift = match args.get("clock_shift_seconds") {
                None | Some(Value::Null) => 0,
                Some(v) => v.as_i64().filter(|s| s.unsigned_abs() <= 100 * 366 * 86_400).ok_or("`clock_shift_seconds` must be a whole number of seconds")?,
            };
            let max_gap = match args.get("max_gap_seconds") {
                None | Some(Value::Null) => crate::geotag::DEFAULT_MAX_GAP,
                Some(v) => v.as_f64().filter(|g| g.is_finite() && *g >= 1.0).ok_or("`max_gap_seconds` must be a number of at least 1")?,
            };
            let replace = opt_bool(args, "replace_existing")?.unwrap_or(false);
            let dry_run = opt_bool(args, "dry_run")?.unwrap_or(false);
            let log = crate::geotag::read_gpx(&files)?;
            let keys: Vec<String> = ids.iter().filter_map(|&id| host.photos().get(id).map(|p| p.key.clone())).collect();
            let photos = crate::geotag::photo_times(host.store(), &keys);
            let opts = crate::geotag::Options { zone, clock_shift, max_gap, replace };
            let (found, c) = crate::geotag::matches(&photos, &log, &opts);
            let mut placed = vec![];
            for (p, o) in photos.iter().zip(&found) {
                if let crate::geotag::Outcome::Placed { at, interpolated } = *o {
                    if !dry_run && let Some(id) = host.find(&p.key) {
                        set_place(host, &[id], Some(Place::set(at, "")));
                    }
                    placed.push(json!({ "key": p.key, "latitude": at.lat, "longitude": at.lon, "between_points": interpolated }));
                }
            }
            let best = crate::geotag::best_zone(&photos, &log, &opts).map(|(m, n)| json!({ "time_zone": crate::geotag::Zone::offset_text(m), "would_place": n }));
            now(json!({
                "dry_run": dry_run,
                "placed": c.placed,
                "kept_their_location": c.kept,
                "without_date": c.no_date,
                "outside_track": c.outside,
                "track_points": log.points.len(),
                "track_span_utc": log.span().map(|(a, b)| [utc_text(a), utc_text(b)]),
                "best_time_zone": best,
                "pictures": placed,
            }))
        }
        "set_date_taken" => {
            let ids = ids(host, args)?;
            let date = opt_str(args, "date");
            let shift = match args.get("shift_seconds") {
                None | Some(Value::Null) => None,
                Some(v) => Some(v.as_i64().ok_or("`shift_seconds` must be a whole number")?),
            };
            let reset = opt_bool(args, "reset")?.unwrap_or(false);
            if [date.is_some(), shift.is_some(), reset].iter().filter(|b| **b).count() != 1 {
                return Err("give one of `date`, `shift_seconds` or `reset: true`".into());
            }
            if let Some(secs) = shift {
                // a century either way is plenty for a wrong clock
                if secs.unsigned_abs() > 100 * 366 * 86_400 {
                    return Err("`shift_seconds` is too large".into());
                }
                let moved = shift_taken(host, &ids, secs, &|k: &str| crate::import::date_taken(k));
                return now(json!({ "changed": moved, "left_alone": ids.len() - moved }));
            }
            let d = date.map(crate::taken::parse).transpose()?;
            set_taken(host, &ids, d);
            now(json!({ "changed": ids.len(), "date": d.map(|d| d.to_iso()) }))
        }
        "list_people" => {
            let index = host.faces();
            let empty = crate::faces::FaceIndex::default();
            let s = host.store();
            // each name once, with the pictures it is typed on or recognised in
            let mut by_name: std::collections::BTreeMap<String, (String, HashSet<usize>, HashSet<usize>)> = Default::default();
            for (id, p) in host.photos().iter().enumerate() {
                for n in s.meta.get(&p.key).map(|m| m.people.as_slice()).unwrap_or_default() {
                    by_name.entry(crate::tags::key(n)).or_insert_with(|| (n.clone(), HashSet::new(), HashSet::new())).1.insert(id);
                }
                // faces drawn or imported by hand name people with no analysis too
                for n in crate::faces::names_in(index.as_deref().unwrap_or(&empty), s, &p.key) {
                    by_name.entry(crate::tags::key(&n)).or_insert_with(|| (n.clone(), HashSet::new(), HashSet::new())).2.insert(id);
                }
            }
            let people: Vec<Value> = by_name
                .values()
                .map(|(name, typed, seen)| {
                    let group = crate::faces::group_named(s, name, None);
                    json!({ "name": name, "pictures": typed.union(seen).count(), "named_on": typed.len(), "recognised_in": seen.len(), "group": group })
                })
                .collect();
            let unnamed: Vec<Value> = index
                .iter()
                .flat_map(|i| i.groups.iter())
                .filter(|g| g.faces.len() >= crate::faces::MIN_GROUP && crate::faces::name_of(s, g.id).is_none() && !s.hidden_groups.contains(&crate::faces::resolve(s, g.id)))
                .take(200)
                .map(|g| json!({ "group": g.id, "faces": g.faces.len(), "pictures": g.pictures, "cover": { "picture": g.cover.0, "face": g.cover.1 } }))
                .collect();
            now(json!({ "people": people, "unnamed_groups": unnamed, "faces_found": index.is_some() }))
        }
        "name_person" => {
            let group = args.get("group").and_then(Value::as_u64).filter(|&g| g <= u32::MAX as u64).ok_or("`group` must be a group number from list_people")? as u32;
            let name = str_arg(args, "name")?.trim().to_string();
            let merge = opt_bool(args, "merge")?.unwrap_or(false);
            let known = host.faces().is_some_and(|i| i.group(crate::faces::resolve(host.store(), group)).is_some()) || host.store().person_names.contains_key(&group);
            if !known {
                return Err(format!("there is no group of faces {group} (see list_people)"));
            }
            match crate::faces::set_name(host.store_mut(), group, &name, merge) {
                Ok(g) => now(json!({ "group": g, "name": crate::faces::name_of(host.store(), g) })),
                Err(other) => Err(format!("group {other} is {name:?} already; pass merge: true to make the two groups one person")),
            }
        }
        "correct_faces" => {
            let index = host.faces().ok_or("no faces were found: face grouping is off, or the library is not analysed yet")?;
            let list = args.get("faces").and_then(Value::as_array).ok_or("`faces` must be a list of { picture, face }")?;
            let mut faces = vec![];
            for f in list {
                let key = f.get("picture").and_then(Value::as_str).ok_or("each face needs `picture` (a key)")?;
                let i = f.get("face").and_then(Value::as_u64).filter(|&i| i <= u16::MAX as u64).ok_or("each face needs `face` (its place in the picture)")? as u16;
                if index.face(key, i).is_none() {
                    return Err(format!("{key} has no face {i} (see get_photo)"));
                }
                faces.push((key.to_string(), i));
            }
            let c = match str_arg(args, "action")? {
                "move" => crate::faces::Correction::MoveTo(group_arg(&*host, args.get("group"), "group")?),
                "new_person" => crate::faces::Correction::NewPerson,
                "not_this_person" => crate::faces::Correction::NotThisPerson(group_arg(&*host, args.get("group"), "group")?),
                "ignore" => crate::faces::Correction::Ignore,
                "not_a_face" => crate::faces::Correction::NotFace,
                "reset" => crate::faces::Correction::Reset,
                other => return Err(format!("unknown action {other:?}")),
            };
            let n = crate::faces::correct(host.store_mut(), &index, &faces, c);
            now(json!({ "corrected": n }))
        }
        "same_person_proposals" => {
            let limit = args.get("limit").and_then(Value::as_u64).unwrap_or(20).min(500) as usize;
            let index = host.faces().ok_or("no faces were found: face grouping is off, or the library is not analysed yet")?;
            let s = host.store();
            let r = |g| crate::faces::resolve(s, g);
            let size = |g: u32| index.groups.iter().filter(|x| r(x.id) == g).map(|x| x.faces.len()).sum::<usize>();
            let mut seen = HashSet::new();
            let list: Vec<Value> = index
                .proposals
                .iter()
                .map(|&(a, b, score)| (r(a), r(b), score))
                .filter(|&(a, b, _)| a != b && !s.refused_merges.contains(&(a.min(b), a.max(b))) && !s.hidden_groups.contains(&a) && !s.hidden_groups.contains(&b))
                .filter(|&(a, b, _)| seen.insert((a.min(b), a.max(b))))
                .take(limit)
                .map(|(a, b, score)| json!({
                    "a": { "group": a, "name": crate::faces::name_of(s, a), "faces": size(a) },
                    "b": { "group": b, "name": crate::faces::name_of(s, b), "faces": size(b) },
                    "likeness": (score * 1000.0).round() / 1000.0,
                }))
                .collect();
            now(json!({ "proposals": list }))
        }
        "not_same_person" => {
            let (a, b) = (group_arg(&*host, args.get("a"), "a")?, group_arg(&*host, args.get("b"), "b")?);
            let (a, b) = (crate::faces::resolve(host.store(), a), crate::faces::resolve(host.store(), b));
            if a == b {
                return Err("these are one person".into());
            }
            let pair = (a.min(b), a.max(b));
            if !host.store().refused_merges.contains(&pair) {
                host.store_mut().refused_merges.push(pair);
            }
            now(json!({ "two_people": [a, b] }))
        }
        "exclude_from_face_detection" => {
            let out = opt_bool(args, "excluded")?.unwrap_or(true);
            let mut paths: Vec<String> = match args.get("keys") {
                None | Some(Value::Null) => vec![],
                _ => ids(&*host, args)?.into_iter().filter_map(|id| host.photos().get(id).map(|p| p.key.clone())).collect(),
            };
            for f in strings(args, "folders")? {
                paths.push(library_dir(&*host, &f)?);
            }
            let changed = !paths.is_empty() && host.store_mut().set_no_faces(&paths, out);
            now(json!({ "changed": changed, "excluded": host.store().faces_excluded }))
        }
        "hide_person" => {
            let g = crate::faces::resolve(host.store(), group_arg(&*host, args.get("group"), "group")?);
            let hidden = opt_bool(args, "hidden")?.unwrap_or(true);
            if hidden {
                host.store_mut().hidden_groups.insert(g);
            } else {
                host.store_mut().hidden_groups.remove(&g);
            }
            now(json!({ "group": g, "hidden": hidden }))
        }
        "merge_people" => {
            let group_of = group_arg;
            let (from, into) = (group_of(&*host, args.get("from"), "from")?, group_of(&*host, args.get("into"), "into")?);
            if crate::faces::resolve(host.store(), from) == crate::faces::resolve(host.store(), into) {
                return Err("these are one person already".into());
            }
            crate::faces::merge_groups(host.store_mut(), from, into);
            let g = crate::faces::resolve(host.store(), into);
            now(json!({ "group": g, "name": crate::faces::name_of(host.store(), g) }))
        }
        "list_tags" => {
            fn node(n: &crate::tags::Node) -> Value {
                json!({ "tag": n.name, "pictures": n.own, "with_tags_under": n.total, "under": n.children.iter().map(node).collect::<Vec<_>>() })
            }
            now(json!({ "tags": crate::tags::tree(host.store()).iter().map(node).collect::<Vec<_>>() }))
        }
        "edit_tag" => {
            let tag = str_arg(args, "tag")?.trim().to_string();
            let rename_to = opt_str(args, "rename_to").map(str::trim).filter(|t| !t.is_empty()).map(str::to_string);
            let parent = opt_str(args, "parent").map(|p| p.trim().to_string());
            let delete = opt_bool(args, "delete")?.unwrap_or(false);
            if [rename_to.is_some(), parent.is_some(), delete].iter().filter(|b| **b).count() != 1 {
                return Err("give one of `rename_to`, `parent` or `delete: true`".into());
            }
            if !crate::tags::counts(host.store()).contains_key(&crate::tags::key(&tag)) {
                return Err(format!("there is no tag {tag:?} (see list_tags)"));
            }
            if delete {
                let changed = crate::tags::delete(host.store_mut(), &tag);
                return now(json!({ "deleted": tag, "pictures_changed": changed.len() }));
            }
            if let Some(to) = rename_to {
                let changed = crate::tags::rename(host.store_mut(), &tag, &to)?;
                return now(json!({ "renamed": tag, "to": to, "pictures_changed": changed.len() }));
            }
            let parent = parent.filter(|p| !p.is_empty());
            crate::tags::set_parent(host.store_mut(), &tag, parent.as_deref())?;
            now(json!({ "tag": tag, "under": parent }))
        }
        "remove_location" => {
            let ids = ids(host, args)?;
            remove_place(host, &ids, opt_bool(args, "back_to_file")?.unwrap_or(false));
            now(json!({ "changed": ids.len() }))
        }
        "find_place" => {
            let query = str_arg(args, "query")?.to_string();
            let limit = opt_u64(args, "limit")?.unwrap_or(5).clamp(1, 20) as usize;
            let as_json = |f: &geo::Found| json!({ "name": f.name, "short_name": f.short, "latitude": f.at.lat, "longitude": f.at.lon });
            if opt_bool(args, "online")?.unwrap_or(false) {
                if host.store().online_maps != Some(true) {
                    return Err("online maps are turned off in AlbumCraft (main menu › Online maps); search without `online`".into());
                }
                return Ok(Outcome::Later(Box::new(move || {
                    let found = crate::online::search(&query)?;
                    Ok(Reply::json(json!({ "places": found.iter().take(limit).map(as_json).collect::<Vec<_>>() })))
                })));
            }
            now(json!({ "places": geo::search(&query, limit).iter().map(as_json).collect::<Vec<_>>() }))
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
            host.store_mut().add_root(&s)?;
            host.rescan();
            now(json!({ "pictures": host.photos().len() }))
        }
        "import_pictures" => {
            let source = existing_dir(str_arg(args, "source")?)?;
            let dest = PathBuf::from(str_arg(args, "destination")?);
            let parent = dest.parent().filter(|p| !p.as_os_str().is_empty()).ok_or("`destination` must be a full folder path")?;
            if !parent.is_dir() {
                return Err(format!("{:?} is not an existing folder", parent));
            }
            if dest.file_name().is_none() || dest.is_file() {
                return Err("`destination` must be a folder".into());
            }
            let only: Option<std::collections::HashSet<String>> = match args.get("files") {
                None | Some(Value::Null) => None,
                Some(v) => Some(
                    v.as_array()
                        .ok_or("`files` must be a list of paths")?
                        .iter()
                        .map(|f| f.as_str().map(str::to_string).ok_or("`files` must be a list of paths"))
                        .collect::<Result<_, _>>()?,
                ),
            };
            let skip_dups = match args.get("skip_duplicates") {
                None | Some(Value::Null) => true,
                Some(v) => v.as_bool().ok_or("`skip_duplicates` must be true or false")?,
            };
            let library: Vec<(String, u64)> = host.photos().iter().map(|p| (p.key.clone(), p.bytes)).collect();
            // already shown when inside a library folder (that is not an error here)
            let _ = host.store_mut().add_root(&dest.to_string_lossy());
            Ok(Outcome::Later(Box::new(move || {
                let (found, _) = crate::import::list(&source, &library);
                let duplicates = found.iter().filter(|c| c.duplicate).count();
                let files: Vec<String> = found
                    .into_iter()
                    .filter(|c| !(skip_dups && c.duplicate))
                    .filter(|c| only.as_ref().is_none_or(|o| o.contains(&c.path)))
                    .map(|c| c.path)
                    .collect();
                let stop = std::sync::atomic::AtomicBool::new(false);
                let r = crate::import::copy_into(&files, &dest, &|_, _| {}, &stop)?;
                Ok(Reply::json(json!({
                    "folder": dest.to_string_lossy(),
                    "copied": r.copied.len(),
                    "already_there": r.already_there,
                    "left_out_duplicates": if skip_dups { duplicates } else { 0 },
                    "failed": r.failed.iter().map(|(f, why)| json!({ "file": f, "error": why })).collect::<Vec<_>>(),
                })))
            })))
        }
        "ai_status" => {
            let packs: Vec<Value> = crate::analysis::packs()
                .iter()
                .map(|p| json!({ "id": p.pack.id, "title": p.pack.title, "about": p.pack.about, "licence": p.pack.licence, "version": p.pack.version, "bytes": p.pack.bytes(), "installed": p.installed }))
                .collect();
            let analysis = host.analysis().map(|a| json!({ "analysed": a.done, "to_analyse": a.total, "running": a.active() && !a.paused }));
            let search = crate::smart_search::active(&host.store().search_pack).map(|p| p.id.clone());
            now(json!({ "enabled": host.store().ai_enabled, "can_run_networks": crate::ai::runtime::available(), "packs": packs, "analysis_paused": host.store().analysis_paused, "analysis": analysis, "search_pack": search }))
        }
        "download_model_pack" => {
            let id = str_arg(args, "pack")?.to_string();
            if !crate::analysis::packs().iter().any(|p| p.pack.id == id) {
                return Err(format!("no pack is named {id:?} (see ai_status)"));
            }
            Ok(Outcome::Later(Box::new(move || {
                let report = crate::analysis::download_pack(&id, &|_, _| true);
                if report.ok { Ok(Reply::json(json!({ "downloaded": id, "message": report.text }))) } else { Err(report.text) }
            })))
        }
        "delete_face_data" => {
            if opt_bool(args, "confirm")? != Some(true) {
                return Err("`confirm` must be true, once the user agreed: the faces found and the groups are deleted (names and corrections stay)".into());
            }
            host.delete_face_data()?;
            host.store_mut().faces_enabled = Some(false);
            now(json!({ "deleted": true, "faces_enabled": false }))
        }
        "remove_model_pack" => {
            let id = str_arg(args, "pack")?;
            crate::analysis::remove_pack(id)?;
            now(json!({ "removed": id }))
        }
        "propose_tags" => {
            ai_on(host)?;
            let ids = ids(host, args)?;
            let store = host.store();
            let pack = crate::smart_search::active(&store.search_pack)
                .ok_or("tag proposals need a search pack: none is downloaded (see ai_status and download_model_pack)")?;
            let set = |_: &str, m: &library::Meta| crate::geo::effective(m.place.as_ref(), None);
            let photos = host.photos();
            let targets: Vec<crate::tag_proposals::Picture> = ids.iter().filter_map(|&i| photos.get(i)).map(|p| crate::tag_proposals::picture_of(store, &p.key, set)).collect();
            let examples = std::sync::Arc::new(crate::tag_proposals::examples_of(store, 0, set));
            let files: Vec<crate::geo::FileStamp> = photos.iter().filter(|p| !p.is_sample()).map(|p| (p.key.clone(), p.modified, p.bytes)).collect();
            let files = std::sync::Arc::new(files);
            Ok(Outcome::Later(Box::new(move || {
                let dir = crate::smart_search::vectors_dir()?;
                let got = crate::tag_proposals::propose(pack, &files, &dir, &examples, &targets)?;
                let tags: Vec<Value> = got.tags.iter().map(|p| json!({ "tag": p.tag, "score": (f64::from(p.score) * 100.0).round() / 100.0 })).collect();
                Ok(Reply::json(json!({ "tags": tags, "partial": got.partial, "analysed": !got.not_analysed, "pack": pack.id })))
            })))
        }
        "semantic_search" | "similar_pictures" => {
            ai_on(host)?;
            let query = if name == "semantic_search" {
                let text = str_arg(args, "text")?.trim();
                if text.is_empty() {
                    return Err("`text` is empty".into());
                }
                crate::smart_search::Query::Text(text.chars().take(500).collect())
            } else {
                let key = str_arg(args, "key")?;
                one(host, key)?;
                crate::smart_search::Query::Like(key.to_string())
            };
            let limit = opt_u64(args, "limit")?.map(|n| n.clamp(1, crate::smart_search::MAX_HITS as u64) as usize);
            let store = host.store();
            let pack = crate::smart_search::active(&store.search_pack)
                .ok_or("smart search needs a search pack: none is downloaded (see ai_status and download_model_pack)")?;
            let files: Vec<crate::geo::FileStamp> = host.photos().iter().filter(|p| !p.is_sample()).map(|p| (p.key.clone(), p.modified, p.bytes)).collect();
            let files = std::sync::Arc::new(files);
            Ok(Outcome::Later(Box::new(move || {
                let dir = crate::smart_search::vectors_dir()?;
                let hits = crate::smart_search::search(pack, &files, &dir, &query)?;
                let n = limit.unwrap_or_else(|| crate::smart_search::cut(pack, &hits, &query));
                let analysed = crate::smart_search::analysed(pack, &files, &dir)?;
                let pictures: Vec<Value> = hits
                    .iter()
                    .take(n)
                    .map(|h| {
                        let name = std::path::Path::new(&h.key).file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
                        json!({ "key": h.key, "name": name, "score": (f64::from(h.score) * 1000.0).round() / 1000.0 })
                    })
                    .collect();
                Ok(Reply::json(json!({ "pictures": pictures, "more": hits.len().saturating_sub(n), "pack": pack.id, "analysed": analysed, "library": files.len() })))
            })))
        }
        "pause_analysis" => {
            let paused = opt_bool(args, "paused")?.ok_or("`paused` (true or false) is required")?;
            host.store_mut().analysis_paused = paused;
            now(json!({ "paused": paused }))
        }
        "set_sort_order" => {
            let order = match str_arg(args, "order")? {
                "name" => library::Sort::Name,
                "taken" => library::Sort::Taken,
                "rating" => library::Sort::Rating,
                other => return Err(format!("`order` must be name, taken or rating, not {other:?}")),
            };
            host.store_mut().sort = order;
            now(json!({ "order": str_arg(args, "order")? }))
        }
        "rescan" => {
            host.rescan();
            now(json!({ "rescanning": true }))
        }
        "add_edit" => {
            let ids = ids(host, args)?;
            let op = parse_edit(args.get("edit").ok_or("`edit` is required")?)?;
            for &id in &ids {
                if let Some(m) = meta_mut(host, id) {
                    crate::steps::put(&mut m.edits, op.clone());
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
                let base = imaging::load_source(&key, library::BIG_DIM).ok_or_else(|| format!("{key}: the picture can't be read"))?;
                if fix == "straighten" {
                    // looks at the picture with only its quarter turns, and sets the straighten
                    // angle and the vertical perspective as a whole
                    let turns: Vec<EditOp> = edits.iter().filter(|e| matches!(e, EditOp::Rotate(_))).cloned().collect();
                    let found = crate::auto::geometry(&imaging::apply_all(&base, &turns));
                    results.push(json!({ "key": key, "straighten": found.map(|g| g.angle), "vertical_perspective": found.map(|g| g.vertical) }));
                    if let (Some(g), Some(m)) = (found, meta_mut(host, id)) {
                        let horizontal = crate::steps::geometry_of(&m.edits).1.1;
                        crate::steps::set_geometry(&mut m.edits, EditOp::Straighten(g.angle));
                        crate::steps::set_geometry(&mut m.edits, EditOp::Perspective { vertical: g.vertical, horizontal });
                    }
                    host.changed(id);
                    continue;
                }
                // look at the picture without its adjustment, whose sliders the fix sets
                let mut before = edits.clone();
                crate::steps::set_adjust(&mut before, develop::Adjust::default());
                let img = imaging::apply_all(&base, &before);
                let mut a = crate::steps::adjust_of(&edits);
                match fix.as_str() {
                    "lucky" => a = crate::auto::lucky(&img, a),
                    "exposure" => a = crate::auto::exposure(&img, a),
                    "contrast" => a = crate::auto::contrast(&img, a),
                    _ => (a.temp, a.tint) = crate::auto::white_balance(&img),
                }
                crate::steps::set_adjust(&mut edits, a);
                if let Some(m) = meta_mut(host, id) {
                    m.edits = edits;
                }
                host.changed(id);
                results.push(json!({ "key": key, "adjust": a.clamped() }));
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
            let ops = crate::imaging::copyable_edits(&edits_of(host, from));
            if ops.is_empty() {
                return Err("that picture has no edits to copy".into());
            }
            let ids = ids(host, args)?;
            for &id in &ids {
                if let Some(m) = meta_mut(host, id) {
                    crate::steps::merge(&mut m.edits, &ops);
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
            let with_location = opt_bool(args, "include_location")?.unwrap_or(host.store().export_location);
            let store = host.store();
            let opts = crate::export::Options {
                max_dim: max,
                quality,
                camera_info: opt_bool(args, "include_camera_info")?.unwrap_or(store.export_exif),
                creator: opt_str(args, "creator").unwrap_or(&store.export_creator).to_string(),
                copyright: opt_str(args, "copyright").unwrap_or(&store.export_copyright).to_string(),
                signature: None,
                notes: opt_bool(args, "include_notes")?.unwrap_or(store.export_notes),
            };
            let jobs: Vec<ExportJob> = ids.iter().map(|&id| export_job(host, id)).collect();
            Ok(Outcome::Later(Box::new(move || {
                let mut done = vec![];
                let mut failed = vec![];
                for job in &jobs {
                    match crate::export::export_one(&job.key, &job.edits, &dir, job.stem(), &opts, &job.carried(with_location), None) {
                        Ok(p) => done.push(p.to_string_lossy().to_string()),
                        Err(e) => failed.push(json!({ "key": job.key, "error": e })),
                    }
                }
                Ok(Reply::json(json!({ "written": done, "failed": failed })))
            })))
        }
        "trash_rejected" => {
            if opt_bool(args, "confirm")? != Some(true) {
                return Err("`confirm` must be true, once the user agreed: the files are removed from their folders (to the system's trash)".into());
            }
            let rejected = |host: &dyn Host, id: usize| {
                host.photos().get(id).is_some_and(|p| !p.is_sample()) && host.store().meta.get(&host.photos()[id].key).is_some_and(|m| m.flag == Flag::Reject)
            };
            let chosen: Vec<usize> = if args.get("keys").is_some() {
                let chosen = ids(host, args)?;
                if let Some(&id) = chosen.iter().find(|&&id| !rejected(host, id)) {
                    return Err(format!("{} is not rejected: only rejected pictures can be moved to the trash", host.photos()[id].key));
                }
                chosen
            } else {
                (0..host.photos().len()).filter(|&id| rejected(host, id)).collect()
            };
            if chosen.is_empty() {
                return Err("no picture is rejected".into());
            }
            let files: Vec<String> = chosen.iter().map(|&id| host.photos()[id].key.clone()).collect();
            let done = library::trash_files(&files, &|_, _| true);
            host.rescan();
            now(json!({ "moved": done.moved, "not_moved": done.failed, "message": done.report().text }))
        }
        "trash_pictures" => {
            let chosen = ids(host, args)?;
            if opt_bool(args, "confirm")? != Some(true) {
                return Err("`confirm` must be true, once the user agreed: the files are removed from their folders (to the system's trash)".into());
            }
            let files: Vec<String> = chosen.iter().map(|&id| host.photos()[id].key.clone()).collect();
            let done = library::trash_files(&files, &|_, _| true);
            host.rescan();
            now(json!({ "moved": done.moved, "not_moved": done.failed, "refused_by_trash": done.refused, "message": done.report().text }))
        }
        "trash_folder" => {
            let path = library_dir(host, str_arg(args, "path")?)?;
            if opt_bool(args, "confirm")? != Some(true) {
                return Err("`confirm` must be true, once the user agreed: the folder and everything in it go to the system's trash".into());
            }
            let done = library::trash_files(std::slice::from_ref(&path), &|_, _| true);
            if done.moved > 0 {
                forget_root(host.store_mut(), &path);
            }
            host.rescan();
            now(json!({ "moved": done.moved, "not_moved": done.failed, "refused_by_trash": done.refused, "message": done.report().text }))
        }
        "delete_permanently" => {
            let paths = strings(args, "paths")?;
            if paths.is_empty() {
                return Err("`paths` is empty".into());
            }
            for p in &paths {
                if host.find(p).is_none() {
                    library_dir(host, p)?;
                }
            }
            if opt_bool(args, "confirm")? != Some(true) {
                return Err("`confirm` must be true, once the user agreed: these are deleted for good and can't be brought back".into());
            }
            for p in &paths {
                forget_root(host.store_mut(), p);
            }
            let r = library::delete_permanently(&paths, &|_, _| true);
            host.rescan();
            if r.ok { now(json!({ "message": r.text })) } else { Err(r.text) }
        }
        "write_xmp" => {
            let ids = ids(host, args)?;
            if ids.len() > 2000 {
                return Err("at most 2000 pictures in one call".into());
            }
            let embedded_ok = host.store().xmp_embedded;
            let mut written = vec![];
            let mut failed = vec![];
            for id in ids {
                let Some(key) = host.photos().get(id).map(|p| p.key.clone()) else { continue };
                let faces = host.faces();
                let notes = xmp_notes_of(host.store(), faces.as_deref(), &key);
                match crate::xmp_sync::write(&key, &notes, embedded_ok) {
                    Ok((w, seen)) => {
                        host.store_mut().xmp_seen.insert(key.clone(), seen);
                        let into = match w {
                            crate::xmp::Written::Embedded => "inside the file".to_string(),
                            crate::xmp::Written::Sidecar(p) => p.to_string_lossy().into_owned(),
                        };
                        written.push(json!({ "key": key, "written_to": into }));
                    }
                    Err(e) => failed.push(json!({ "key": key, "error": e })),
                }
            }
            Ok(Outcome::Now(Reply::json(json!({ "written": written, "failed": failed }))))
        }
        "rename_picture" => {
            let id = one(host, str_arg(args, "key")?)?;
            let key = host.photos()[id].key.clone();
            let to = crate::files::renamed_file(&key, str_arg(args, "name")?)?;
            moved_reply(move_on_disk(host, &[(key, to)]))
        }
        "move_pictures" => {
            let chosen = ids(host, args)?;
            let folder = str_arg(args, "folder")?;
            library_folder(host.store(), folder)?;
            let pairs = chosen
                .iter()
                .map(|&id| {
                    let key = host.photos()[id].key.clone();
                    crate::files::moved_into(&key, folder).map(|to| (key, to))
                })
                .collect::<Result<Vec<_>, _>>()?;
            moved_reply(move_on_disk(host, &pairs))
        }
        "rename_folder" => {
            let path = library_dir(host, str_arg(args, "path")?)?;
            let to = crate::files::renamed_folder(&path, str_arg(args, "name")?)?;
            moved_reply(move_on_disk(host, &[(path, to)]))
        }
        "move_folder" => {
            let path = library_dir(host, str_arg(args, "path")?)?;
            let into = str_arg(args, "into")?;
            library_folder(host.store(), into)?;
            let to = crate::files::moved_into(&path, into)?;
            moved_reply(move_on_disk(host, &[(path, to)]))
        }
        "set_folder_text" => {
            let path = library_dir(host, str_arg(args, "path")?)?;
            let text = |k: &str| match args.get(k) {
                None | Some(Value::Null) => Ok(None),
                Some(v) => v.as_str().map(Some).ok_or(format!("`{k}` must be text")),
            };
            let (caption, description) = (text("caption")?, text("description")?);
            set_folder_text(host.store_mut(), &path, caption, description);
            let s = host.store();
            now(json!({ "path": path, "caption": s.folder_captions.get(&path), "description": s.folder_notes.get(&path) }))
        }
        "remove_folder" => {
            let path = library_dir(host, str_arg(args, "path")?)?;
            let was_root = remove_folder(host, &path)?.is_some();
            now(json!({ "removed": path, "was_library_folder": was_root, "pictures": host.photos().len() }))
        }
        "email_pictures" => {
            let ids = ids(host, args)?;
            let subject = opt_str(args, "subject").unwrap_or("Pictures").to_string();
            let with_location = host.store().export_location;
            let opts = host.store().export_options(0, 0);
            let jobs: Vec<ExportJob> = ids.iter().map(|&id| export_job(host, id)).collect();
            Ok(Outcome::Later(Box::new(move || {
                let r = crate::mail::email(&jobs, &subject, with_location, &opts, &|_, _| true);
                let folder = r.show.as_ref().map(|p| p.to_string_lossy().to_string());
                if r.ok { Ok(Reply::json(json!({ "message": r.text, "folder": folder }))) } else { Err(r.text) }
            })))
        }
        "make_gallery" => {
            let ids = ids(host, args)?;
            let parent = existing_dir(str_arg(args, "folder")?)?;
            let theme = opt_str(args, "theme").unwrap_or("paper");
            let theme = crate::gallery::Theme::from_id(theme).ok_or_else(|| format!("unknown theme {theme:?}"))?;
            let layout = opt_str(args, "layout").unwrap_or("grid");
            let layout = crate::gallery::Layout::from_id(layout).ok_or_else(|| format!("unknown layout {layout:?}"))?;
            let opts = crate::gallery::Options {
                title: opt_str(args, "title").unwrap_or("Gallery").to_string(),
                theme,
                layout,
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
                let (dir, failed) = crate::gallery::write(&parent, &items, &opts, &|_, _| true)?;
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
                if !crate::movie::available() {
                    return Err("this build has no video support, so it can't make movies".into());
                }
                crate::movie::make(&path, &pictures, size, &s, &|_| true)?;
                Ok(Reply::json(json!({ "movie": path.to_string_lossy(), "seconds": crate::movie::duration(pictures.len(), &s) })))
            })))
        }
        "make_collage" | "make_contact_sheet" => {
            use crate::collage::{self, BgKind, Border, Format, ShapeKind, Style};
            let ids = ids(host, args)?;
            let sheet = name == "make_contact_sheet";
            if !sheet && ids.len() < 2 {
                return Err("a collage needs at least two pictures".into());
            }
            let path = new_output(str_arg(args, "path")?)?;
            let page = match opt_str(args, "page") {
                Some(p) => Some(Format::from_id(p).ok_or_else(|| format!("unknown page size {p:?}"))?),
                None => None,
            };
            let landscape = opt_bool(args, "landscape")?;
            let pictures: Vec<(String, Vec<EditOp>)> = ids.iter().map(|&id| (host.photos()[id].key.clone(), edits_of(host, id))).collect();
            let notes: Vec<(String, String, i64)> = ids
                .iter()
                .map(|&id| {
                    let p = &host.photos()[id];
                    (p.name.clone(), host.store().meta.get(&p.key).map(|m| m.caption.clone()).unwrap_or_default(), p.modified)
                })
                .collect();
            let pics_of = move |pictures: &[(String, Vec<EditOp>)]| -> Vec<collage::Pic> {
                pictures
                    .iter()
                    .zip(&notes)
                    .map(|((key, edits), (name, caption, t))| {
                        let aspect = imaging::load_thumbnail(key, 256).map(|i| imaging::apply_all(&i, edits)).map(|i| i.width() as f32 / i.height().max(1) as f32).unwrap_or(1.5);
                        collage::Pic { aspect, name: name.clone(), caption: caption.clone(), date: library::fmt_date(*t) }
                    })
                    .collect()
            };
            if sheet {
                let mut s = host.store().contact_sheet.clone();
                if let Some(f) = page {
                    s.format = f;
                }
                if let Some(l) = landscape {
                    s.landscape = l;
                }
                if let Some(v) = args.get("picture_size") {
                    let mm = v.as_f64().ok_or("picture_size must be a number")? as f32;
                    s.thumb_mm = mm.clamp(collage::THUMB_MM_MIN, collage::THUMB_MM_MAX);
                }
                for (field, value) in [("names", &mut s.names), ("captions", &mut s.captions), ("dates", &mut s.dates)] {
                    if let Some(b) = opt_bool(args, field)? {
                        *value = b;
                    }
                }
                let title = opt_str(args, "title").unwrap_or("").to_string();
                let times: Vec<i64> = ids.iter().map(|&id| host.photos()[id].modified).collect();
                let subtitle = opt_str(args, "subtitle").map(str::to_string).unwrap_or_else(|| collage::sheet_subtitle(&times));
                return Ok(Outcome::Later(Box::new(move || {
                    let pics = pics_of(&pictures);
                    let pages = collage::contact_sheet(&s, &pics, &title, &subtitle);
                    let n = pages.len();
                    let job = collage::Job { pages, background: collage::Background::white(), bg_picture: None, pictures };
                    let files = collage::save(&job, &path, 2400, &imaging::load_source)?;
                    Ok(Reply::json(json!({ "written": files.iter().map(|f| f.to_string_lossy()).collect::<Vec<_>>(), "pages": n })))
                })));
            }
            let mut s = host.store().collage.clone();
            if let Some(st) = opt_str(args, "style") {
                s.style = Style::from_id(st).ok_or_else(|| format!("unknown style {st:?}"))?;
                s.border = s.style.natural_border();
            }
            if let Some(f) = page {
                s.format = f;
            }
            if let Some(l) = landscape {
                s.landscape = l;
            }
            if let Some(b) = opt_str(args, "border") {
                s.border = match b {
                    "none" => Border::None,
                    "white" => Border::White,
                    "instant" => Border::Instant,
                    _ => return Err(format!("unknown border {b:?}")),
                };
            }
            if let Some(b) = opt_bool(args, "shadow")? {
                s.shadow = b;
            }
            if let Some(k) = opt_str(args, "shape") {
                s.shape = match k {
                    "heart" => ShapeKind::Heart,
                    "circle" => ShapeKind::Circle,
                    "star" => ShapeKind::Star,
                    _ => return Err(format!("unknown shape {k:?}")),
                };
            }
            if let Some(c) = opt_str(args, "background") {
                s.background.color = hex_color(c)?;
                s.background.kind = BgKind::Color;
            }
            if let Some(c) = opt_str(args, "background2") {
                s.background.color2 = hex_color(c)?;
                s.background.kind = BgKind::Gradient;
                s.background.gradient = collage::Gradient::Vertical;
            }
            let bg_picture = match opt_str(args, "background_picture") {
                Some(f) if Path::new(f).is_file() => {
                    s.background.kind = BgKind::Picture;
                    s.background.blur = true;
                    let edits = host.find(f).map(|id| edits_of(host, id)).unwrap_or_default();
                    Some((f.to_string(), edits))
                }
                Some(f) => return Err(format!("background picture {f:?} not found")),
                None => {
                    if s.background.kind == BgKind::Picture {
                        s.background.kind = BgKind::Color;
                    }
                    None
                }
            };
            if let Some(px) = opt_u64(args, "size")? {
                s.out_px = px.clamp(256, collage::MAX_PAGE_PX as u64) as u32;
            }
            if let Some(seed) = opt_u64(args, "seed")? {
                s.seed = seed;
            }
            Ok(Outcome::Later(Box::new(move || {
                let pics = pics_of(&pictures);
                let page = collage::collage(&s, &pics, &[]);
                let job = collage::Job { pages: vec![page], background: s.background, bg_picture, pictures };
                let files = collage::save(&job, &path, s.out_px, &imaging::load_source)?;
                Ok(Reply::json(json!({ "written": files.iter().map(|f| f.to_string_lossy()).collect::<Vec<_>>() })))
            })))
        }
        "show_photo" => {
            let id = one(host, str_arg(args, "key")?)?;
            host.show(id)?;
            now(json!({ "shown": true }))
        }
        "show_on_map" => {
            let ids = ids(host, args)?;
            let title = opt_str(args, "title").map(str::trim).filter(|t| !t.is_empty()).map(str::to_string);
            let title = title.unwrap_or_else(|| format!("{} picture{}", ids.len(), if ids.len() == 1 { "" } else { "s" }));
            host.show_map(&ids, title)?;
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
    /// The files' GPS positions, read on first need (and cached on disk in `gps_cache`).
    gps: Option<HashMap<String, Option<LatLon>>>,
    pub gps_cache: Option<PathBuf>,
    /// The faces as last grouped by the window, read on first need.
    faces: std::sync::OnceLock<Option<std::sync::Arc<crate::faces::FaceIndex>>>,
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
        let mut h = Headless { path, store, photos: vec![], folders: vec![], index: Default::default(), gps: None, gps_cache: geo::gps_cache_path(), faces: Default::default() };
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
        self.gps = None;
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
    fn faces(&self) -> Option<std::sync::Arc<crate::faces::FaceIndex>> {
        self.faces
            .get_or_init(|| {
                if !(self.store.ai_enabled && self.store.faces_enabled == Some(true)) {
                    return None;
                }
                let files: Vec<crate::geo::FileStamp> = self.photos.iter().map(|p| (p.key.clone(), p.modified, p.bytes)).collect();
                crate::faces::saved_index(&files, &crate::faces::decided(&self.store)).map(std::sync::Arc::new)
            })
            .clone()
    }

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
        let (found, _) = library::scan_roots_except(&self.store.roots, &self.store.excluded, library::SCAN_LIMIT);
        self.set_library(found);
    }
    fn file_gps(&mut self) -> &HashMap<String, Option<LatLon>> {
        let (photos, cache) = (&self.photos, self.gps_cache.clone());
        self.gps.get_or_insert_with(|| {
            let files = photos.iter().map(|p| (p.key.clone(), p.modified, p.bytes)).collect();
            let (tx, rx) = std::sync::mpsc::channel();
            geo::index_files(files, cache, tx, Default::default());
            rx.try_iter().flatten().collect()
        })
    }
}

/// Whether a command only reads (no need to save after it).
pub fn read_only(name: &str) -> bool {
    matches!(
        name,
        "library_info" | "list_folders" | "search_photos" | "get_photo" | "render_preview" | "export" | "make_gallery" | "make_movie" | "show_photo" | "show_on_map" | "find_place" | "semantic_search" | "similar_pictures" | "propose_tags" | "ai_status" | "list_tags" | "list_people" | "same_person_proposals"
    )
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
        let mut h = Headless::open(dir.join("library.json")).unwrap();
        h.gps_cache = Some(dir.join("gps.json"));
        (dir, h, keys)
    }

    #[test]
    fn pictures_and_folders_are_kept_out_of_face_detection() {
        let (dir, mut h, keys) = fixture("nofaces");
        h.rescan();
        let folder = dir.join("pics").to_string_lossy().to_string();
        let r = call(&mut h, "exclude_from_face_detection", json!({ "keys": [keys[0]] })).unwrap();
        assert_eq!((r["changed"].clone(), r["excluded"].clone()), (json!(true), json!([keys[0]])));
        let r = call(&mut h, "exclude_from_face_detection", json!({ "folders": [folder] })).unwrap();
        assert_eq!(r["excluded"], json!([folder]), "the folder takes the place of its picture");
        // nothing named: the list only
        assert_eq!(call(&mut h, "exclude_from_face_detection", json!({})).unwrap()["changed"], false);
        let r = call(&mut h, "exclude_from_face_detection", json!({ "folders": [folder], "excluded": false })).unwrap();
        assert_eq!(r["excluded"], json!([]));
        assert!(call(&mut h, "exclude_from_face_detection", json!({ "keys": ["/no/such.jpg"] })).is_err());
        assert!(call(&mut h, "exclude_from_face_detection", json!({ "folders": ["/no/such/folder"] })).is_err());
        assert!(call(&mut h, "exclude_from_face_detection", json!({ "keys": [keys[1]], "excluded": "yes" })).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn geotagging_from_a_track_log() {
        let (dir, mut h, keys) = fixture("geotag");
        h.rescan();
        // logged every minute from 10:00 to 10:30 UTC, going north
        let mut gpx = String::from(r#"<gpx version="1.1" xmlns="http://www.topografix.com/GPX/1/1"><trk><trkseg>"#);
        for i in 0..=30 {
            gpx += &format!(r#"<trkpt lat="{}" lon="-0.58"><time>2024-06-01T10:{i:02}:00Z</time></trkpt>"#, 44.0 + f64::from(i) * 0.01);
        }
        gpx += "</trkseg></trk></gpx>";
        let track = dir.join("walk.gpx");
        std::fs::write(&track, gpx).unwrap();
        let track = track.to_string_lossy().to_string();
        // 12:10:30 and 13:00 local time in UTC+02:00: in the track, and half an hour after it
        call(&mut h, "set_date_taken", json!({ "keys": [keys[1]], "date": "2024-06-01T12:10:30" })).unwrap();
        call(&mut h, "set_date_taken", json!({ "keys": [keys[2]], "date": "2024-06-01T13:00:00" })).unwrap();
        let args = json!({ "keys": keys, "gpx_files": [track], "time_zone": "+02:00", "dry_run": true });
        let dry = call(&mut h, "geotag_from_gpx", args.clone()).unwrap();
        assert_eq!((dry["placed"].as_u64(), dry["without_date"].as_u64(), dry["outside_track"].as_u64()), (Some(1), Some(1), Some(1)), "{dry}");
        assert!(h.store.meta.get(&keys[1]).and_then(|m| m.place.as_ref()).is_none(), "a dry run changes nothing");
        let mut real = args;
        real["dry_run"] = json!(false);
        let done = call(&mut h, "geotag_from_gpx", real).unwrap();
        let lat = done["pictures"][0]["latitude"].as_f64().unwrap();
        assert!((lat - 44.105).abs() < 1e-9, "{done}");
        assert!(matches!(h.store.meta.get(&keys[1]).and_then(|m| m.place.as_ref()), Some(Place::Set { .. })));
        // read as UTC the picture is two hours after the track: only UTC+02:00 places it
        let utc = call(&mut h, "geotag_from_gpx", json!({ "keys": [keys[1]], "gpx_files": [track], "time_zone": "UTC", "dry_run": true, "replace_existing": true })).unwrap();
        assert_eq!(utc["placed"], 0);
        assert_eq!(utc["best_time_zone"]["time_zone"], "UTC+02:00");
        // bad input is an error
        for bad in [json!({ "keys": [keys[1]], "gpx_files": [] }), json!({ "keys": [keys[1]], "gpx_files": [track], "time_zone": "Mars" }), json!({ "keys": [keys[1]], "gpx_files": ["/no/such.gpx"] })] {
            assert!(call(&mut h, "geotag_from_gpx", bad).is_err());
        }
        let _ = std::fs::remove_dir_all(&dir);
        use crate::geotag::Zone::{Fixed, Pictures};
        assert_eq!(["pictures", "UTC", "+02:00", "UTC-03:30", "+0530", "+2", "−05:00"].map(parse_zone), [Some(Pictures), Some(Fixed(0)), Some(Fixed(120)), Some(Fixed(-210)), Some(Fixed(330)), Some(Fixed(120)), Some(Fixed(-300))]);
        assert_eq!(["+15:00", "02:00", "+1:75"].map(parse_zone), [None, None, None]);
    }

    #[test]
    fn dates_taken_set_shifted_and_reset() {
        let (dir, mut h, keys) = fixture("dates");
        // the demo picture has a camera date; the samples have none
        crate::exifinfo::write_demo_jpeg(Path::new(&keys[0])).unwrap();
        h.rescan();
        let set = |h: &Headless, k: &str| h.store.meta.get(k).and_then(|m| m.taken).map(|d| d.to_iso());
        let r = call(&mut h, "set_date_taken", json!({ "keys": [keys[1], keys[2]], "date": "1978-07" })).unwrap();
        assert_eq!((r["changed"].clone(), r["date"].clone()), (json!(2), json!("1978-07")));
        assert_eq!(set(&h, &keys[2]).as_deref(), Some("1978-07"));
        let p = call(&mut h, "get_photo", json!({ "key": keys[1] })).unwrap();
        assert_eq!(p["date_taken_set"], "1978-07");
        // shifting: the camera's date moves, a month alone and a picture without a date don't
        let camera = crate::import::date_taken(&keys[0]).unwrap();
        let r = call(&mut h, "set_date_taken", json!({ "keys": [keys[0], keys[1]], "shift_seconds": 3600 })).unwrap();
        assert_eq!((r["changed"].clone(), r["left_alone"].clone()), (json!(1), json!(1)));
        let moved = h.store.meta.get(&keys[0]).and_then(|m| m.taken).and_then(|d| crate::taken::seconds(&d));
        assert_eq!(moved, Some(camera + 3600));
        // and again from the date set
        call(&mut h, "set_date_taken", json!({ "keys": [keys[0]], "shift_seconds": -7200 })).unwrap();
        assert_eq!(h.store.meta.get(&keys[0]).and_then(|m| m.taken).and_then(|d| crate::taken::seconds(&d)), Some(camera - 3600));
        call(&mut h, "set_date_taken", json!({ "keys": [keys[0], keys[1]], "reset": true })).unwrap();
        assert_eq!((set(&h, &keys[0]), set(&h, &keys[1])), (None, None));
        // the XMP gets the date; going back to the file's date removes it there
        call(&mut h, "set_date_taken", json!({ "keys": [keys[2]], "date": "1950" })).unwrap();
        run(&mut h, "write_xmp", &json!({ "keys": [keys[2]] })).and_then(|o| o.finish()).unwrap();
        let side = format!("{}.xmp", keys[2]);
        let read = |p: &str| crate::xmp::parse(&std::fs::read_to_string(p).unwrap()).and_then(|n| n.date).map(|d| d.to_iso());
        assert_eq!(read(&side).as_deref(), Some("1950"));
        call(&mut h, "set_date_taken", json!({ "keys": [keys[2]], "reset": true })).unwrap();
        run(&mut h, "write_xmp", &json!({ "keys": [keys[2]] })).and_then(|o| o.finish()).unwrap();
        assert_eq!(read(&side), None);
        for bad in [
            json!({ "keys": [keys[0]] }),
            json!({ "keys": [keys[0]], "date": "1978", "reset": true }),
            json!({ "keys": [keys[0]], "date": "July" }),
            json!({ "keys": [keys[0]], "date": "1600" }),
            json!({ "keys": [keys[0]], "shift_seconds": 1.5 }),
            json!({ "keys": [keys[0]], "shift_seconds": 1_000_000_000_000i64 }),
        ] {
            assert!(call(&mut h, "set_date_taken", bad.clone()).is_err(), "{bad}");
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn notes_written_to_xmp_sidecars() {
        let (dir, mut h, keys) = fixture("xmp");
        run(&mut h, "set_rating", &json!({ "keys": [keys[0]], "rating": 4 })).and_then(|o| o.finish()).unwrap();
        run(&mut h, "set_caption", &json!({ "keys": [keys[0]], "caption": "Low tide" })).and_then(|o| o.finish()).unwrap();
        let before = std::fs::read(&keys[0]).unwrap();
        let reply = run(&mut h, "write_xmp", &json!({ "keys": [keys[0]] })).and_then(|o| o.finish()).unwrap();
        assert_eq!(reply.value["failed"].as_array().map(Vec::len), Some(0), "{:?}", reply.value);
        // the JPEG had no XMP inside: a sidecar, and the picture unchanged
        let side = format!("{}.xmp", keys[0]);
        let notes = crate::xmp::parse(&std::fs::read_to_string(&side).unwrap()).unwrap();
        assert_eq!((notes.rating, notes.caption.as_deref()), (Some(4), Some("Low tide")));
        assert_eq!(std::fs::read(&keys[0]).unwrap(), before);
        assert!(h.store().xmp_seen.contains_key(&keys[0]));
        assert!(run(&mut h, "write_xmp", &json!({ "keys": [] })).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn sort_order_is_set_and_saved() {
        let (dir, mut h, _) = fixture("sort");
        let reply = run(&mut h, "set_sort_order", &json!({ "order": "taken" })).and_then(|o| o.finish()).unwrap();
        assert_eq!(reply.value["order"], "taken");
        assert_eq!(h.store().sort, library::Sort::Taken);
        h.save().unwrap();
        let back = Headless::open(dir.join("library.json")).unwrap();
        assert_eq!(back.store().sort, library::Sort::Taken);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn import_copies_new_pictures_only() {
        let (dir, mut h, keys) = fixture("import");
        let card = dir.join("card/DCIM/100CAM");
        std::fs::create_dir_all(&card).unwrap();
        // one picture the library already has (renamed), one new
        std::fs::copy(&keys[0], card.join("IMG_0001.JPG")).unwrap();
        image::DynamicImage::ImageRgba8(crate::imaging::sample_image(9, 100, 70)).to_rgb8().save(card.join("IMG_0002.JPG")).unwrap();
        let dest = dir.join("Trip");
        let args = json!({ "source": dir.join("card").to_string_lossy(), "destination": dest.to_string_lossy() });
        let reply = run(&mut h, "import_pictures", &args).and_then(|o| o.finish()).unwrap();
        assert_eq!(reply.value["copied"], 1);
        assert_eq!(reply.value["left_out_duplicates"], 1);
        assert!(dest.join("IMG_0002.JPG").is_file());
        assert!(!dest.join("IMG_0001.JPG").exists());
        assert!(h.store().roots.contains(&dest.to_string_lossy().to_string()));
        run(&mut h, "rescan", &json!({})).unwrap();
        assert_eq!(h.photos().len(), 4);
        // the card is untouched
        assert!(card.join("IMG_0001.JPG").is_file() && card.join("IMG_0002.JPG").is_file());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn pictures_and_folders_move_with_their_notes() {
        let (dir, mut h, keys) = fixture("moves");
        h.rescan();
        let pics = dir.join("pics");
        call(&mut h, "set_caption", json!({ "keys": [keys[0]], "caption": "Dawn" })).unwrap();
        call(&mut h, "create_album", json!({ "name": "Best", "keys": [keys[0]] })).unwrap();
        // rename: the extension is kept, the notes follow
        let r = call(&mut h, "rename_picture", json!({ "key": keys[0], "name": "dawn" })).unwrap();
        let dawn = pics.join("dawn.jpg").to_string_lossy().to_string();
        assert_eq!(r["moved"][0]["to"], dawn);
        assert!(Path::new(&dawn).is_file() && !Path::new(&keys[0]).exists());
        assert_eq!(h.store().meta[&dawn].caption, "Dawn");
        assert_eq!(h.store().albums[0].keys, vec![dawn.clone()]);
        assert!(h.find(&dawn).is_some());
        // into a subfolder
        let sub = pics.join("2024");
        std::fs::create_dir_all(&sub).unwrap();
        call(&mut h, "move_pictures", json!({ "keys": [dawn, keys[1]], "folder": sub.to_string_lossy() })).unwrap();
        let moved = sub.join("dawn.jpg").to_string_lossy().to_string();
        assert_eq!(h.store().meta[&moved].caption, "Dawn");
        assert_eq!(h.photos().len(), 3);
        // the folder: caption, then renamed, then removed from the library
        let s = sub.to_string_lossy().to_string();
        call(&mut h, "set_folder_text", json!({ "path": s, "caption": "Summer", "description": "By the sea" })).unwrap();
        call(&mut h, "rename_folder", json!({ "path": s, "name": "Summer 2024" })).unwrap();
        let summer = pics.join("Summer 2024").to_string_lossy().to_string();
        assert_eq!(h.store().folder_captions[&summer], "Summer");
        assert_eq!(h.store().folder_notes[&summer], "By the sea");
        assert_eq!(h.store().meta[&Path::new(&summer).join("dawn.jpg").to_string_lossy().to_string()].caption, "Dawn");
        call(&mut h, "set_folder_text", json!({ "path": summer, "caption": "" })).unwrap();
        assert!(!h.store().folder_captions.contains_key(&summer));
        call(&mut h, "remove_folder", json!({ "path": summer })).unwrap();
        assert_eq!(h.photos().len(), 1);
        assert!(Path::new(&summer).join("dawn.jpg").is_file());
        // deleting for good (the real trash is not used in tests)
        let last = h.photos()[0].key.clone();
        call(&mut h, "delete_permanently", json!({ "paths": [last], "confirm": true })).unwrap();
        assert!(!Path::new(&last).exists() && h.photos().is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn locations() {
        let (dir, mut h, keys) = fixture("places");
        // beach.jpg carries a GPS position (Bordeaux) in its EXIF
        let bordeaux = LatLon { lat: 44.8378, lon: -0.5792 };
        let mut jpeg = vec![];
        image::DynamicImage::ImageRgba8(crate::imaging::sample_image(1, 120, 80))
            .to_rgb8()
            .write_to(&mut std::io::Cursor::new(&mut jpeg), image::ImageFormat::Jpeg)
            .unwrap();
        std::fs::write(&keys[0], crate::filemeta::jpeg_with_exif(&jpeg, &geo::exif_with_gps(bordeaux)).unwrap()).unwrap();
        h.rescan();
        assert_eq!(call(&mut h, "search_photos", json!({ "has_location": true })).unwrap()["total"], 1);
        assert_eq!(call(&mut h, "search_photos", json!({ "has_location": false })).unwrap()["total"], 2);
        let p = call(&mut h, "get_photo", json!({ "key": keys[0] })).unwrap();
        assert_eq!(p["location"]["source"], "the file's GPS");
        assert_eq!(p["location"]["place"], "Bordeaux, France");

        // set one by hand
        call(&mut h, "set_location", json!({ "keys": [keys[1]], "latitude": 48.8566, "longitude": 2.3522, "name": "Paris" })).unwrap();
        let near = call(&mut h, "search_photos", json!({ "near": { "latitude": 48.86, "longitude": 2.35, "radius_km": 5 } })).unwrap();
        assert_eq!(near["total"], 1);
        assert_eq!(near["photos"][0]["key"], keys[1]);
        assert_eq!(call(&mut h, "search_photos", json!({ "query": "paris" })).unwrap()["total"], 1);
        assert_eq!(call(&mut h, "get_photo", json!({ "key": keys[1] })).unwrap()["location"]["place"], "Paris");
        // places found by city, region or country, also from the file's GPS (no name typed)
        assert_eq!(call(&mut h, "search_photos", json!({ "query": "bordeaux" })).unwrap()["total"], 1);
        assert_eq!(call(&mut h, "search_photos", json!({ "place": "france" })).unwrap()["total"], 2);
        assert_eq!(call(&mut h, "search_photos", json!({ "place": "Île-de-France" })).unwrap()["total"], 1);
        call(&mut h, "tag_photos", json!({ "keys": [keys[0]], "add_tags": ["Beach"] })).unwrap();
        assert_eq!(call(&mut h, "search_photos", json!({ "tag": "beach", "place": "France" })).unwrap()["total"], 1);
        assert_eq!(call(&mut h, "search_photos", json!({ "tag": "nothing" })).unwrap()["total"], 0);

        // export writes the location into the copy, unless asked not to
        let out = dir.join("out");
        std::fs::create_dir_all(&out).unwrap();
        let gps_of_copy = |v: &Value| {
            let path = v["written"][0].as_str().unwrap().to_string();
            crate::filemeta::exif(Path::new(&path)).and_then(|m| geo::gps_of(&m))
        };
        let e = call(&mut h, "export", json!({ "keys": [keys[1]], "folder": out.to_string_lossy(), "max_size": 64 })).unwrap();
        assert!(gps_of_copy(&e).is_some_and(|g| g.km_to(LatLon { lat: 48.8566, lon: 2.3522 }) < 0.01));
        let e = call(&mut h, "export", json!({ "keys": [keys[1]], "folder": out.to_string_lossy(), "max_size": 64, "include_location": false })).unwrap();
        assert_eq!(gps_of_copy(&e), None);

        // removing hides the file's GPS too; back_to_file brings it back
        call(&mut h, "remove_location", json!({ "keys": [keys[0]] })).unwrap();
        assert_eq!(call(&mut h, "get_photo", json!({ "key": keys[0] })).unwrap()["location"], Value::Null);
        assert_eq!(call(&mut h, "search_photos", json!({ "has_location": true })).unwrap()["total"], 1);
        call(&mut h, "remove_location", json!({ "keys": [keys[0]], "back_to_file": true })).unwrap();
        assert_eq!(call(&mut h, "search_photos", json!({ "has_location": true })).unwrap()["total"], 2);

        // place search, offline only unless online maps are on
        let found = call(&mut h, "find_place", json!({ "query": "Bordeaux" })).unwrap();
        assert!((found["places"][0]["latitude"].as_f64().unwrap() - 44.84).abs() < 0.05);
        assert!(call(&mut h, "find_place", json!({ "query": "Bordeaux", "online": true })).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }

    fn call(h: &mut Headless, name: &str, args: Value) -> Result<Value, String> {
        run(h, name, &args)?.finish().map(|r| r.value)
    }

    #[test]
    fn people_names_and_merges() {
        let (dir, mut h, keys) = fixture("people");
        h.rescan();
        // a name typed on a picture, and two named groups of faces (faces were grouped earlier)
        h.store.meta.entry(keys[0].clone()).or_default().people.push("Anna".into());
        h.store.person_names.insert(3, "Anna".into());
        h.store.person_names.insert(5, "Marc".into());
        let people = call(&mut h, "list_people", json!({})).unwrap();
        assert_eq!(people["people"][0]["name"], "Anna");
        assert_eq!(people["people"][0]["named_on"], 1);
        // another group named like one that exists: refused unless merging
        assert!(call(&mut h, "name_person", json!({ "group": 5, "name": "anna" })).unwrap_err().contains("merge"));
        assert!(call(&mut h, "name_person", json!({ "group": 77, "name": "Zoé" })).is_err(), "no such group");
        assert_eq!(call(&mut h, "name_person", json!({ "group": 5, "name": "Marc D." })).unwrap()["name"], "Marc D.");
        let merged = call(&mut h, "merge_people", json!({ "from": "Marc D.", "into": 3 })).unwrap();
        assert_eq!((merged["group"].clone(), merged["name"].clone()), (json!(3), json!("Anna")));
        assert_eq!(crate::faces::resolve(&h.store, 5), 3);
        assert!(call(&mut h, "merge_people", json!({ "from": 5, "into": 3 })).is_err(), "one person already");
        // searching a name finds the typed one (no faces without the Faces pack)
        let found = call(&mut h, "search_photos", json!({ "person": "anna" })).unwrap();
        assert_eq!(found["total"], 1);
        let _ = std::fs::remove_dir_all(&dir);
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
    fn ai_status_and_pause() {
        let (dir, mut h, _) = fixture("ai");
        let Ok(Outcome::Now(reply)) = run(&mut h, "ai_status", &json!({})) else { panic!("ai_status answers at once") };
        let status = reply.value;
        assert_eq!(status["can_run_networks"], crate::ai::runtime::available());
        assert_eq!(status["packs"].as_array().map(Vec::len), Some(crate::ai::models::manifest().packs.len()));
        // without a window nothing is analysed; the pause is kept for the window
        assert!(status["analysis"].is_null() && status["analysis_paused"] == false);
        assert!(run(&mut h, "pause_analysis", &json!({ "paused": true })).is_ok());
        assert!(h.store.analysis_paused);
        // turned off in Settings: searches are refused with a reason, not run
        assert_eq!(status["enabled"], true);
        h.store.ai_enabled = false;
        let Err(e) = run(&mut h, "semantic_search", &json!({ "text": "a dog" })) else { panic!("refused while off") };
        assert!(e.contains("turned off"), "{e}");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn the_people_of_a_picture_for_its_xmp_are_typed_and_recognised_names_once_each() {
        let mut store = Store::default();
        store.person_names.insert(5, "Anna".into());
        let mut meta = library::Meta { people: vec!["marc".into(), "anna".into()], ..Default::default() };
        crate::faces::add_face(&mut meta, [0.1, 0.1, 0.1, 0.1], 5);
        store.meta.insert("p.jpg".into(), meta);
        assert_eq!(people_union(&store, None, "p.jpg"), vec!["marc".to_string(), "anna".to_string()]);
        let notes = xmp_notes_of(&store, None, "p.jpg");
        assert_eq!(notes.people, Some(vec!["marc".to_string(), "anna".to_string()]));
        // a picture with only a drawn face
        let mut only = library::Meta::default();
        crate::faces::add_face(&mut only, [0.1, 0.1, 0.1, 0.1], 5);
        store.meta.insert("q.jpg".into(), only);
        assert_eq!(people_union(&store, None, "q.jpg"), vec!["Anna".to_string()]);
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
            ("import_pictures", json!({ "source": "/no/such/folder", "destination": dir.join("x").to_string_lossy() })),
            ("import_pictures", json!({ "source": dir.to_string_lossy(), "destination": "/no/such/parent/x" })),
            ("import_pictures", json!({ "source": dir.to_string_lossy(), "destination": "relative" })),
            ("import_pictures", json!({ "source": dir.to_string_lossy(), "destination": dir.join("x").to_string_lossy(), "files": "a" })),
            ("import_pictures", json!({ "source": dir.to_string_lossy(), "destination": dir.join("x").to_string_lossy(), "skip_duplicates": 1 })),
            ("export", json!({ "keys": [k], "folder": "/no/such/folder" })),
            ("render_preview", json!({ "key": k, "edits": "x" })),
            ("make_movie", json!({ "keys": [k], "path": dir.join("library.json").to_string_lossy() })),
            ("get_photo", json!([1, 2])),
            ("show_photo", json!({ "key": k })),
            ("show_on_map", json!({ "keys": [k] })),
            ("show_on_map", json!({ "keys": [] })),
            ("email_pictures", json!({ "keys": [] })),
            ("trash_rejected", json!({})),
            ("download_model_pack", json!({})),
            ("download_model_pack", json!({ "pack": "../../no-such-pack" })),
            ("remove_model_pack", json!({ "pack": "no-such-pack" })),
            ("delete_face_data", json!({})),
            ("delete_face_data", json!({ "confirm": "yes" })),
            ("delete_face_data", json!({ "confirm": false })),
            ("pause_analysis", json!({})),
            ("pause_analysis", json!({ "paused": "yes" })),
            ("semantic_search", json!({})),
            ("propose_tags", json!({ "keys": [] })),
            ("propose_tags", json!({ "keys": ["/no/such.jpg"] })),
            ("semantic_search", json!({ "text": "   " })),
            ("semantic_search", json!({ "text": "snow", "limit": "ten" })),
            ("similar_pictures", json!({ "key": "/no/such/picture.jpg" })),
            ("trash_rejected", json!({ "confirm": "yes" })),
            ("trash_rejected", json!({ "confirm": false })),
            ("trash_rejected", json!({ "keys": [k], "confirm": true })),
            ("set_sort_order", json!({ "order": "colour" })),
            ("trash_pictures", json!({ "keys": [k] })),
            ("trash_folder", json!({ "path": dir.join("pics").to_string_lossy() })),
            ("trash_folder", json!({ "path": "/no/such/folder", "confirm": true })),
            ("delete_permanently", json!({ "paths": [k] })),
            ("delete_permanently", json!({ "paths": [], "confirm": true })),
            ("delete_permanently", json!({ "paths": ["/etc/passwd"], "confirm": true })),
            ("trash_pictures", json!({ "keys": [k], "confirm": "yes" })),
            ("rename_picture", json!({ "key": k, "name": "a/b" })),
            ("rename_picture", json!({ "key": k, "name": "" })),
            ("rename_picture", json!({ "key": k, "name": "hills.jpg" })),
            ("move_pictures", json!({ "keys": [k], "folder": "/no/such/folder" })),
            ("move_pictures", json!({ "keys": [k], "folder": dir.to_string_lossy() })),
            ("rename_folder", json!({ "path": "/no/such/folder", "name": "x" })),
            ("move_folder", json!({ "path": dir.join("pics").to_string_lossy(), "into": dir.join("pics").to_string_lossy() })),
            ("set_folder_text", json!({ "path": dir.join("pics").to_string_lossy(), "caption": 3 })),
            ("remove_folder", json!({ "path": dir.to_string_lossy() })),
            ("set_sort_order", json!({})),
            ("set_location", json!({ "keys": [k], "latitude": 100, "longitude": 0 })),
            ("set_location", json!({ "keys": [k], "latitude": 10 })),
            ("set_location", json!({ "keys": [k], "latitude": "10", "longitude": 0 })),
            ("search_photos", json!({ "near": { "latitude": 10 } })),
            ("search_photos", json!({ "near": { "latitude": 10, "longitude": 0, "radius_km": -1 } })),
            ("search_photos", json!({ "has_location": "yes" })),
            ("find_place", json!({})),
            ("edit_tag", json!({ "tag": "nothing", "delete": true })),
            ("edit_tag", json!({ "tag": "x" })),
            ("edit_tag", json!({ "tag": "x", "rename_to": "y", "delete": true })),
            ("remove_location", json!({ "keys": [k], "back_to_file": 1 })),
            ("make_collage", json!({ "keys": [k], "path": dir.join("c.jpg").to_string_lossy() })),
            ("make_collage", json!({ "keys": keys, "path": dir.join("c.gif").to_string_lossy() })),
            ("make_collage", json!({ "keys": keys, "path": dir.join("library.json").to_string_lossy() })),
            ("make_collage", json!({ "keys": keys, "path": dir.join("c.jpg").to_string_lossy(), "style": "spiral" })),
            ("make_collage", json!({ "keys": keys, "path": dir.join("c.jpg").to_string_lossy(), "background": "#12345" })),
            ("make_collage", json!({ "keys": keys, "path": dir.join("c.jpg").to_string_lossy(), "background": "#ééé" })),
            ("make_contact_sheet", json!({ "keys": keys, "path": "/no/such/folder/s.pdf" })),
            ("make_contact_sheet", json!({ "keys": keys, "path": dir.join("s.pdf").to_string_lossy(), "page": "B7" })),
        ];
        for (name, args) in bad {
            assert!(run(&mut h, name, &args).and_then(|o| o.finish()).is_err(), "{name} {args} should fail");
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn tags_as_a_tree() {
        let (dir, mut h, keys) = fixture("tags");
        for (k, t) in keys.iter().zip(["church", "Cathedral", "sea"]) {
            h.store.meta.entry(k.clone()).or_default().tags = vec![t.to_string()];
        }
        call(&mut h, "edit_tag", json!({ "tag": "cathedral", "rename_to": "Church" })).unwrap();
        call(&mut h, "edit_tag", json!({ "tag": "church", "parent": "Buildings" })).unwrap();
        let t = call(&mut h, "list_tags", json!({})).unwrap();
        let b = t["tags"].as_array().unwrap().iter().find(|n| n["tag"] == "Buildings").unwrap().clone();
        assert_eq!((b["pictures"].as_u64(), b["with_tags_under"].as_u64()), (Some(0), Some(2)));
        assert_eq!(call(&mut h, "search_photos", json!({ "tag": "buildings" })).unwrap()["total"], 2);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn collages_and_contact_sheets() {
        let (dir, mut h, keys) = fixture("collage");
        let out = dir.join("c.jpg");
        let r = call(&mut h, "make_collage", json!({ "keys": keys, "path": out.to_string_lossy(), "style": "clothesline", "background": "#f0e8d8", "background2": "#a08060", "size": 600 })).unwrap();
        assert_eq!(r["written"][0], out.to_string_lossy().as_ref());
        let img = image::open(&out).unwrap();
        assert_eq!(img.width().max(img.height()), 600);
        // never overwrites
        assert!(call(&mut h, "make_collage", json!({ "keys": keys, "path": out.to_string_lossy() })).is_err());
        let pdf = dir.join("s.pdf");
        let r = call(&mut h, "make_contact_sheet", json!({ "keys": keys, "path": pdf.to_string_lossy(), "title": "Test", "dates": true })).unwrap();
        assert_eq!(r["pages"], 1);
        assert!(std::fs::read(&pdf).unwrap().starts_with(b"%PDF"));
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
