//! The sync stream (`POST /sync/stream`): JSON lines, each `{type, data, ack}`, giving every
//! picture, album, person, face, stack and Immich edit of the account, then only what changed
//! since the last `ack`. `Mirror` keeps the server's state as the stream describes it.
//!
//! A line is applied to the mirror, the mirror is saved, and only then is the line's `ack`
//! sent: a crash replays lines, never loses them. Lines of types this version doesn't know are
//! skipped (and acknowledged).

use std::collections::{BTreeMap, BTreeSet};
use std::io::{BufRead, BufReader, Read};

use serde::{Deserialize, Serialize};

use crate::Error;

/// The types asked for. Pictures only of the account itself for now (no partners' pictures,
/// no pictures of albums shared by others).
pub const TYPES: &[&str] = &["AssetsV2", "AssetExifsV1", "AssetEditsV1", "AssetMetadataV1", "AlbumsV2", "AlbumToAssetsV1", "PeopleV1", "AssetFacesV3", "StacksV1"];

/// One line of the stream, its data still unparsed.
#[derive(Debug, Clone)]
pub struct Line {
    pub kind: String,
    pub data: serde_json::Value,
    pub ack: String,
}

/// Read JSON lines, calling `each` for every one; lines over `limit` bytes or that aren't
/// JSON are errors (the stream is then not trusted any further).
pub fn read_lines(reader: impl Read, limit: usize, each: &mut dyn FnMut(Line) -> bool) -> Result<(), Error> {
    #[derive(Deserialize)]
    struct Raw {
        #[serde(rename = "type")]
        kind: String,
        #[serde(default)]
        data: serde_json::Value,
        #[serde(default)]
        ack: String,
    }
    let mut r = BufReader::with_capacity(1 << 20, reader);
    let mut buf = Vec::with_capacity(4096);
    loop {
        buf.clear();
        let n = (&mut r).take(limit as u64 + 1).read_until(b'\n', &mut buf).map_err(|e| Error::Network(e.to_string()))?;
        if n == 0 {
            return Ok(());
        }
        if buf.len() > limit {
            return Err(Error::Invalid(format!("a sync line is longer than {limit} bytes")));
        }
        let text = buf.trim_ascii();
        if text.is_empty() {
            continue;
        }
        let raw: Raw = serde_json::from_slice(text).map_err(|e| Error::Invalid(format!("sync line: {e}")))?;
        if !each(Line { kind: raw.kind, data: raw.data, ack: raw.ack }) {
            return Ok(());
        }
    }
}

/// A picture or video (`AssetV2`).
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct Asset {
    pub id: String,
    pub owner_id: String,
    pub original_file_name: String,
    /// When it was taken, in the place's local time, written as UTC (`2025-03-23T12:20:05.000Z`).
    pub local_date_time: Option<String>,
    pub file_created_at: Option<String>,
    pub file_modified_at: Option<String>,
    /// `IMAGE`, `VIDEO`, `AUDIO`, `OTHER`.
    #[serde(rename = "type")]
    pub kind: String,
    pub deleted_at: Option<String>,
    pub is_favorite: bool,
    /// `timeline`, `archive`, `hidden`, `locked`.
    pub visibility: String,
    pub duration: Option<String>,
    pub stack_id: Option<String>,
    pub library_id: Option<String>,
    pub live_photo_video_id: Option<String>,
    /// The size as shown: with Immich's own crop when `is_edited`.
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub is_edited: bool,
    /// SHA-1 of the file, base64.
    pub checksum: String,
    /// A ~25-byte blurred placeholder, base64.
    pub thumbhash: Option<String>,
}

impl Asset {
    /// Shown in AlbumCraft: a still picture, not in the trash, not in the locked folder, not
    /// the video half of a live photo (`hidden`).
    pub fn shown(&self) -> bool {
        self.kind == "IMAGE" && self.deleted_at.is_none() && matches!(self.visibility.as_str(), "timeline" | "archive")
    }
}

/// What Immich read from a picture's file, and the caption and rating it keeps there
/// (`AssetExifV1`).
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct Exif {
    pub asset_id: String,
    pub description: Option<String>,
    pub exif_image_width: Option<u32>,
    pub exif_image_height: Option<u32>,
    pub file_size_in_byte: Option<u64>,
    /// EXIF orientation as text (`"1"`, `"6"`…).
    pub orientation: Option<String>,
    pub date_time_original: Option<String>,
    pub time_zone: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub city: Option<String>,
    pub state: Option<String>,
    pub country: Option<String>,
    pub make: Option<String>,
    pub model: Option<String>,
    pub lens_model: Option<String>,
    pub f_number: Option<f64>,
    pub focal_length: Option<f64>,
    pub iso: Option<f64>,
    pub exposure_time: Option<String>,
    /// 0–5, or None when not rated (some servers allow -1).
    pub rating: Option<i32>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct Album {
    pub id: String,
    pub name: String,
    pub description: String,
    pub thumbnail_asset_id: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct Person {
    pub id: String,
    pub name: String,
    pub is_hidden: bool,
    pub face_asset_id: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct Face {
    pub id: String,
    pub asset_id: String,
    pub person_id: Option<String>,
    pub is_visible: bool,
    pub deleted_at: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct Stack {
    pub id: String,
    pub primary_asset_id: String,
}

/// One of Immich's own edits of a picture: `crop`, `rotate` or `mirror`.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct Edit {
    pub id: String,
    pub asset_id: String,
    pub sequence: i64,
    pub action: String,
    pub parameters: serde_json::Value,
}

/// A tag (`GET /tags`; not in the sync stream). `value` is the full path, `a/b/c`.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct Tag {
    pub id: String,
    pub name: String,
    pub value: String,
    pub parent_id: Option<String>,
}

/// The account's library as the server described it.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Mirror {
    pub assets: BTreeMap<String, Asset>,
    pub exif: BTreeMap<String, Exif>,
    pub albums: BTreeMap<String, Album>,
    /// (album id, asset id)
    pub album_assets: BTreeSet<(String, String)>,
    pub people: BTreeMap<String, Person>,
    pub faces: BTreeMap<String, Face>,
    pub stacks: BTreeMap<String, Stack>,
    pub edits: BTreeMap<String, Edit>,
    /// (asset id, key) → value: AlbumCraft keeps its edit recipe under `albumcraft.edits`.
    pub metadata: BTreeMap<String, BTreeMap<String, serde_json::Value>>,
    /// Tag id → tag, and tag id → its pictures (from the tag calls, not the stream).
    pub tags: BTreeMap<String, Tag>,
    pub tagged: BTreeMap<String, BTreeSet<String>>,
    /// The last `ack` of each type applied, to send when the mirror is saved.
    pub acks: BTreeMap<String, String>,
    /// The stream said to start again from nothing (`SyncResetV1`).
    pub reset_asked: bool,
    /// The first full stream has been read to its end (`SyncCompleteV1`).
    pub complete: bool,
    /// Lines that could not be read (kept as a count, for the status).
    pub skipped: u64,
}

fn str_field(v: &serde_json::Value, name: &str) -> Option<String> {
    v.get(name).and_then(|x| x.as_str()).map(str::to_string)
}

impl Mirror {
    /// Apply one line. Returns false when the line could not be read (it is still
    /// acknowledged: reading it again would fail again).
    pub fn apply(&mut self, line: Line) -> bool {
        fn parse<T: serde::de::DeserializeOwned>(v: serde_json::Value) -> Option<T> {
            serde_json::from_value(v).ok()
        }
        let d = line.data;
        let ok = match line.kind.as_str() {
            "AssetV1" | "AssetV2" => parse::<Asset>(d).filter(|a| !a.id.is_empty()).map(|a| {
                self.assets.insert(a.id.clone(), a);
            }),
            "AssetDeleteV1" => str_field(&d, "assetId").map(|id| {
                self.assets.remove(&id);
                self.exif.remove(&id);
                self.metadata.remove(&id);
            }),
            "AssetExifV1" => parse::<Exif>(d).filter(|e| !e.asset_id.is_empty()).map(|e| {
                self.exif.insert(e.asset_id.clone(), e);
            }),
            "AlbumV1" | "AlbumV2" => parse::<Album>(d).filter(|a| !a.id.is_empty()).map(|a| {
                self.albums.insert(a.id.clone(), a);
            }),
            "AlbumDeleteV1" => str_field(&d, "albumId").map(|id| {
                self.albums.remove(&id);
                self.album_assets.retain(|(a, _)| *a != id);
            }),
            "AlbumToAssetV1" => str_field(&d, "albumId").zip(str_field(&d, "assetId")).map(|p| {
                self.album_assets.insert(p);
            }),
            "AlbumToAssetDeleteV1" => str_field(&d, "albumId").zip(str_field(&d, "assetId")).map(|p| {
                self.album_assets.remove(&p);
            }),
            "PersonV1" => parse::<Person>(d).filter(|p| !p.id.is_empty()).map(|p| {
                self.people.insert(p.id.clone(), p);
            }),
            "PersonDeleteV1" => str_field(&d, "personId").map(|id| {
                self.people.remove(&id);
            }),
            "AssetFaceV1" | "AssetFaceV2" | "AssetFaceV3" => parse::<Face>(d).filter(|f| !f.id.is_empty()).map(|f| {
                self.faces.insert(f.id.clone(), f);
            }),
            "AssetFaceDeleteV1" => str_field(&d, "assetFaceId").map(|id| {
                self.faces.remove(&id);
            }),
            "StackV1" => parse::<Stack>(d).filter(|s| !s.id.is_empty()).map(|s| {
                self.stacks.insert(s.id.clone(), s);
            }),
            "StackDeleteV1" => str_field(&d, "stackId").map(|id| {
                self.stacks.remove(&id);
            }),
            "AssetEditV1" => parse::<Edit>(d).filter(|e| !e.id.is_empty()).map(|e| {
                self.edits.insert(e.id.clone(), e);
            }),
            "AssetEditDeleteV1" => str_field(&d, "editId").map(|id| {
                self.edits.remove(&id);
            }),
            "AssetMetadataV1" => str_field(&d, "assetId").zip(str_field(&d, "key")).map(|(a, k)| {
                let v = d.get("value").cloned().unwrap_or(serde_json::Value::Null);
                self.metadata.entry(a).or_default().insert(k, v);
            }),
            "AssetMetadataDeleteV1" => str_field(&d, "assetId").zip(str_field(&d, "key")).map(|(a, k)| {
                if let Some(m) = self.metadata.get_mut(&a) {
                    m.remove(&k);
                    if m.is_empty() {
                        self.metadata.remove(&a);
                    }
                }
            }),
            "SyncCompleteV1" => {
                self.complete = true;
                Some(())
            }
            "SyncResetV1" => {
                self.reset_asked = true;
                Some(())
            }
            // a type this version doesn't know: nothing to do with it
            _ => Some(()),
        };
        if let Some(kind) = line.ack.split('|').next()
            && !kind.is_empty()
            && !line.ack.is_empty()
        {
            self.acks.insert(kind.to_string(), line.ack.clone());
        }
        if ok.is_none() {
            self.skipped = self.skipped.saturating_add(1);
        }
        ok.is_some()
    }

    /// Forget everything (after `SyncResetV1`, or signing in as someone else).
    pub fn clear(&mut self) {
        *self = Mirror::default();
    }

    /// The acks to send now that the mirror is saved.
    pub fn pending_acks(&self) -> Vec<String> {
        self.acks.values().cloned().collect()
    }

    /// Replace the tags and their pictures with a fresh reading.
    pub fn set_tags(&mut self, tags: Vec<Tag>, tagged: BTreeMap<String, BTreeSet<String>>) {
        self.tags = tags.into_iter().filter(|t| !t.id.is_empty()).map(|t| (t.id.clone(), t)).collect();
        self.tagged = tagged;
    }

    /// The full names (`a/b/c`) of a picture's tags.
    pub fn tags_of(&self, asset: &str) -> Vec<String> {
        self.tagged.iter().filter(|(_, ids)| ids.contains(asset)).filter_map(|(t, _)| self.tags.get(t)).map(|t| t.value.clone()).collect()
    }

    /// The names of the people on a picture (named ones, visible faces).
    pub fn people_of(&self, asset: &str) -> Vec<String> {
        let mut out: Vec<String> = self
            .faces
            .values()
            .filter(|f| f.asset_id == asset && f.deleted_at.is_none() && f.is_visible)
            .filter_map(|f| f.person_id.as_ref())
            .filter_map(|p| self.people.get(p))
            .filter(|p| !p.name.is_empty() && !p.is_hidden)
            .map(|p| p.name.clone())
            .collect();
        out.sort();
        out.dedup();
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const STREAM: &str = r#"{"type":"AssetV2","data":{"id":"a1","ownerId":"u","originalFileName":"x.jpg","fileCreatedAt":"2006-06-15T23:32:26.000Z","localDateTime":"2006-06-15T23:32:26.000Z","type":"IMAGE","deletedAt":null,"isFavorite":true,"visibility":"timeline","width":500,"height":400,"isEdited":false,"checksum":"V2As/8RReMweVAInki9tWiFnFmU=","thumbhash":"iwcK","futureField":1},"ack":"AssetV2|1"}
{"type":"AssetV2","data":{"id":"a2","type":"VIDEO","visibility":"timeline"},"ack":"AssetV2|2"}
{"type":"AssetV2","data":{"id":"a3","type":"IMAGE","visibility":"locked"},"ack":"AssetV2|3"}

{"type":"AssetExifV1","data":{"assetId":"a1","description":"Beach","exifImageWidth":500,"exifImageHeight":400,"orientation":"1","latitude":45.03,"longitude":7.67,"city":"Moncalieri","rating":4,"iso":138},"ack":"AssetExifV1|1"}
{"type":"AlbumV2","data":{"id":"al","name":"March","description":"","thumbnailAssetId":"a1"},"ack":"AlbumV2|1"}
{"type":"AlbumToAssetV1","data":{"assetId":"a1","albumId":"al"},"ack":"AlbumToAssetV1|1"}
{"type":"PersonV1","data":{"id":"p1","name":"Ann","isHidden":false},"ack":"PersonV1|1"}
{"type":"AssetFaceV3","data":{"id":"f1","assetId":"a1","personId":"p1","isVisible":true,"deletedAt":null},"ack":"AssetFaceV3|1"}
{"type":"AssetEditV1","data":{"id":"e1","assetId":"a1","sequence":0,"action":"crop","parameters":{"x":0,"y":0,"width":10,"height":10}},"ack":"AssetEditV1|1"}
{"type":"SomethingNewV9","data":{"x":1},"ack":"SomethingNewV9|1"}
{"type":"AssetExifV1","data":{"assetId":7},"ack":"AssetExifV1|2"}
{"type":"SyncCompleteV1","data":{},"ack":"SyncCompleteV1|1"}
"#;

    fn mirror_of(text: &str) -> Mirror {
        let mut m = Mirror::default();
        let r = read_lines(text.as_bytes(), 1 << 20, &mut |l| {
            m.apply(l);
            true
        });
        assert!(r.is_ok(), "{r:?}");
        m
    }

    #[test]
    fn a_stream_fills_the_mirror() {
        let m = mirror_of(STREAM);
        assert_eq!(m.assets.len(), 3);
        let a = &m.assets["a1"];
        assert!(a.shown() && a.is_favorite && a.width == Some(500));
        assert!(!m.assets["a2"].shown(), "videos are left out for now");
        assert!(!m.assets["a3"].shown(), "the locked folder is never shown");
        assert_eq!(m.exif["a1"].rating, Some(4));
        assert_eq!(m.exif["a1"].description.as_deref(), Some("Beach"));
        assert!(m.album_assets.contains(&("al".to_string(), "a1".to_string())));
        assert_eq!(m.people_of("a1"), vec!["Ann".to_string()]);
        assert_eq!(m.edits["e1"].action, "crop");
        assert!(m.complete);
        assert_eq!(m.skipped, 1, "the damaged exif line");
        assert_eq!(m.acks["AssetV2"], "AssetV2|3");
        assert_eq!(m.acks["AssetExifV1"], "AssetExifV1|2", "a damaged line is acknowledged too");
        assert!(m.acks.contains_key("SomethingNewV9"));
    }

    #[test]
    fn deletes_remove() {
        let mut text = STREAM.to_string();
        text.push_str(
            r#"{"type":"AssetDeleteV1","data":{"assetId":"a1"},"ack":"AssetDeleteV1|1"}
{"type":"AlbumToAssetDeleteV1","data":{"albumId":"al","assetId":"a1"},"ack":"x|1"}
{"type":"AssetFaceDeleteV1","data":{"assetFaceId":"f1"},"ack":"x|2"}
{"type":"AssetEditDeleteV1","data":{"editId":"e1"},"ack":"x|3"}
{"type":"AlbumDeleteV1","data":{"albumId":"al"},"ack":"x|4"}
"#,
        );
        let m = mirror_of(&text);
        assert!(!m.assets.contains_key("a1") && !m.exif.contains_key("a1"));
        assert!(m.album_assets.is_empty() && m.faces.is_empty() && m.edits.is_empty() && m.albums.is_empty());
    }

    #[test]
    fn broken_streams_are_errors_not_crashes() {
        let mut m = Mirror::default();
        assert!(read_lines("not json\n".as_bytes(), 1 << 20, &mut |l| m.apply(l)).is_err());
        let long = format!("{{\"type\":\"AssetV2\",\"data\":{{\"id\":\"{}\"}}}}\n", "x".repeat(2000));
        assert!(read_lines(long.as_bytes(), 1000, &mut |l| m.apply(l)).is_err());
        // a cut last line
        assert!(read_lines(r#"{"type":"AssetV2","data":{"id":"#.as_bytes(), 1000, &mut |l| m.apply(l)).is_err());
        // a wrong shape for a known type is skipped, not fatal
        let mut n = Mirror::default();
        assert!(read_lines(r#"{"type":"AssetV2","data":[1,2]}"#.as_bytes(), 1000, &mut |l| n.apply(l)).is_ok());
        assert_eq!((n.assets.len(), n.skipped), (0, 1));
        // stopping early
        let mut count = 0;
        assert!(read_lines(STREAM.as_bytes(), 1 << 20, &mut |_| {
            count += 1;
            count < 2
        })
        .is_ok());
        assert_eq!(count, 2);
    }

    #[test]
    fn tags_and_metadata() {
        let mut m = mirror_of(STREAM);
        let tags = vec![Tag { id: "t1".into(), name: "b".into(), value: "a/b".into(), parent_id: None }];
        m.set_tags(tags, [("t1".to_string(), ["a1".to_string()].into())].into());
        assert_eq!(m.tags_of("a1"), vec!["a/b".to_string()]);
        assert!(m.tags_of("a2").is_empty());
        let line = |kind: &str, data: serde_json::Value| Line { kind: kind.into(), data, ack: format!("{kind}|9") };
        m.apply(line("AssetMetadataV1", serde_json::json!({"assetId":"a1","key":"albumcraft.edits","value":{"format":1}})));
        assert_eq!(m.metadata["a1"]["albumcraft.edits"]["format"], 1);
        m.apply(line("AssetMetadataDeleteV1", serde_json::json!({"assetId":"a1","key":"albumcraft.edits"})));
        assert!(!m.metadata.contains_key("a1"));
        m.apply(line("SyncResetV1", serde_json::json!({})));
        assert!(m.reset_asked);
        let saved = serde_json::to_string(&m).unwrap_or_default();
        let back: Mirror = serde_json::from_str(&saved).unwrap_or_default();
        assert_eq!(back.assets.len(), m.assets.len());
    }
}
