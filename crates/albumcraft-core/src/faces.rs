//! Faces in the library: which face analyser the background analysis runs.
//!
//! Faces are looked for only when the user said yes (`Store.faces_enabled`, asked once, also
//! in Settings › AI features) and the AI features are on. With the *Faces* pack downloaded,
//! each face gets a vector (`ai::faces`, the table "faces.yunet-1-<recogniser>"); without it
//! they are only found ("faces.yunet-1"), which the pictures are looked at again for once the
//! pack is there. Grouping the faces by person comes after (`face_groups`, not yet).
//!
//! Face vectors are biometric data: they stay in the analysis folder of this computer
//! (`ai::store::analysis_dir`), never in `library.json`, sidecars or exported copies.

use crate::ai::faces::{FaceAnalyser, detect_only, table};
use crate::ai::models::{self, Pack};
use crate::ai::store::{Spec, Table};
use crate::analysis::{Analysis, SharedTable};
use std::collections::HashMap;
use std::sync::{Arc, LazyLock, Mutex, PoisonError, RwLock};

/// The id of the Faces pack in the built-in list.
pub const PACK: &str = "faces";

/// The Faces pack of the built-in list.
pub fn pack() -> Option<&'static Pack> {
    models::manifest().pack(PACK).filter(|p| p.faces.is_some())
}

/// The Faces pack is on this computer.
pub fn pack_installed() -> bool {
    match (pack(), models::models_dir()) {
        (Some(p), Some(dir)) => p.installed(&dir),
        _ => false,
    }
}

/// Every table faces can be kept in: found only, and with the vectors of the built-in pack.
fn tables() -> Vec<Spec> {
    let mut all = vec![detect_only()];
    all.extend(pack().and_then(|p| p.faces.as_ref()).map(|c| table(Some(c))));
    all
}

type Loaded = (Arc<FaceAnalyser>, SharedTable);

/// The analysers made so far, by table (networks are loaded once per program).
fn analysers() -> std::sync::MutexGuard<'static, HashMap<String, Loaded>> {
    static ALL: LazyLock<Mutex<HashMap<String, Loaded>>> = LazyLock::new(Default::default);
    ALL.lock().unwrap_or_else(PoisonError::into_inner)
}

fn analyser(analysis: &Analysis, with_pack: bool) -> Result<Loaded, String> {
    let dir = analysis.dir().ok_or_else(crate::smart_search::no_folder)?;
    let models = models::models_dir().ok_or_else(crate::smart_search::no_folder)?;
    let a = FaceAnalyser::new(if with_pack { pack().map(|p| (p, models.as_path())) } else { None });
    let spec = crate::ai::Analyser::table(&a);
    let mut all = analysers();
    if let Some(l) = all.get(&spec.model) {
        return Ok(l.clone());
    }
    let t = Arc::new(RwLock::new(Table::open(dir, &spec)?));
    let l = (Arc::new(a), t);
    all.insert(spec.model.clone(), l.clone());
    Ok(l)
}

/// Make the background analysis look for faces (`on`), with vectors when the Faces pack is
/// downloaded, or not at all. Called by the window when either changes. What was found stays
/// in its file either way.
pub fn use_faces(analysis: &Analysis, on: bool) -> Result<(), String> {
    let with_pack = pack_installed();
    let wanted = on.then(|| if with_pack { pack().and_then(|p| p.faces.as_ref()).map(|c| table(Some(c))) } else { Some(detect_only()) }).flatten();
    for spec in tables() {
        if Some(&spec) != wanted.as_ref() {
            analysis.remove(&spec);
        }
    }
    if !with_pack {
        // the recogniser's network leaves memory once its pack is removed
        analysers().retain(|_, (a, _)| a.recogniser().is_none());
    }
    if wanted.is_none() {
        return Ok(());
    }
    let (a, t) = analyser(analysis, with_pack)?;
    analysis.add_with(a, t);
    Ok(())
}

// ---------------------------------------------------------------------------
// Grouping: the groups of the library's faces, kept between runs

use crate::ai::faces::FaceRecord;
use crate::face_groups::{Constraints, Face, FaceId, GroupId, Groups, Params};
use serde::{Deserialize, Serialize};

/// A face of a picture, as the window knows it (no vector).
#[derive(Clone, Debug, PartialEq)]
pub struct IndexedFace {
    /// Its place among the picture's faces.
    pub index: u16,
    /// Left, top, width, height as fractions of the unedited, upright picture.
    pub rect: [f32; 4],
    pub group: Option<GroupId>,
    pub score: f32,
    /// Good enough to start a group (sharp, large, facing the camera).
    pub good: bool,
}

/// A group of faces: one person, named or not.
#[derive(Clone, Debug, PartialEq)]
pub struct GroupInfo {
    pub id: GroupId,
    /// Its faces: (picture, place among the picture's faces), in the library's order.
    pub faces: Vec<(String, u16)>,
    /// Pictures it is in.
    pub pictures: usize,
    /// The face that shows it best.
    pub cover: (String, u16),
}

/// The faces of the library and their groups (what the People view will show).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct FaceIndex {
    /// Picture -> its faces (pictures with none are left out).
    pub pictures: HashMap<String, Vec<IndexedFace>>,
    /// Largest first.
    pub groups: Vec<GroupInfo>,
    /// Groups that may be one person: (smaller, larger, how close).
    pub proposals: Vec<(GroupId, GroupId, f32)>,
}

impl FaceIndex {
    /// Faces found, faces in a group.
    pub fn counts(&self) -> (usize, usize) {
        let all = self.pictures.values().map(Vec::len).sum();
        let grouped = self.pictures.values().flatten().filter(|f| f.group.is_some()).count();
        (all, grouped)
    }
}

/// The groups as kept in `face-groups.<model>.json` beside the faces' table.
#[derive(Serialize, Deserialize, Default)]
struct Saved {
    version: u32,
    next: GroupId,
    faces: Vec<SavedFace>,
}

#[derive(Serialize, Deserialize)]
struct SavedFace {
    /// The picture (`ai::store::picture_id`, hexadecimal), the face's place and box.
    p: String,
    i: u16,
    r: [f32; 4],
    g: GroupId,
}

const SAVED_VERSION: u32 = 1;
/// Groups shown as people from this many faces; smaller ones are "Others".
pub const MIN_GROUP: usize = 3;

fn saved_path(dir: &std::path::Path, model: &str) -> std::path::PathBuf {
    dir.join(format!("face-groups.{model}.json"))
}

fn overlap(a: &[f32; 4], b: &[f32; 4]) -> f32 {
    let w = ((a[0] + a[2]).min(b[0] + b[2]) - a[0].max(b[0])).max(0.0);
    let h = ((a[1] + a[3]).min(b[1] + b[3]) - a[1].max(b[1])).max(0.0);
    let i = w * h;
    let u = a[2] * a[3] + b[2] * b[3] - i;
    if u > 0.0 { i / u } else { 0.0 }
}

/// What was decided before, for the faces still there: a face whose picture changed keeps its
/// group when a face of the new analysis is in the same place.
fn load_groups(path: &std::path::Path, records: &HashMap<FaceId, FaceRecord>) -> Groups {
    let Some(saved) = std::fs::read(path).ok().and_then(|b| serde_json::from_slice::<Saved>(&b).ok()).filter(|s| s.version == SAVED_VERSION) else {
        return Groups::default();
    };
    let mut of = HashMap::new();
    for f in saved.faces {
        let Ok(pic) = u128::from_str_radix(&f.p, 16) else { continue };
        if records.get(&(pic, f.i)).is_some_and(|r| overlap(&r.rect, &f.r) >= 0.5) {
            of.insert((pic, f.i), f.g);
        }
    }
    Groups { of, next: saved.next }
}

fn save_groups(path: &std::path::Path, groups: &Groups, records: &HashMap<FaceId, FaceRecord>) -> Result<(), String> {
    let mut faces: Vec<SavedFace> = groups
        .of
        .iter()
        .filter_map(|(id, g)| {
            let r = records.get(id)?;
            Some(SavedFace { p: format!("{:032x}", id.0), i: id.1, r: r.rect.map(|v| (v * 1e4).round() / 1e4), g: *g })
        })
        .collect();
    faces.sort_by(|a, b| (&a.p, a.i).cmp(&(&b.p, b.i)));
    let bytes = serde_json::to_vec(&Saved { version: SAVED_VERSION, next: groups.next, faces }).map_err(|e| e.to_string())?;
    crate::library::write_library(path, &bytes)
}

/// Group the faces of `files` (the library's pictures, in the order to place them) found with
/// the Faces pack, keeping what was decided before; the groups are saved beside the table.
/// None when faces have no vectors yet (no pack).
pub fn group(analysis: &Analysis, files: &[crate::geo::FileStamp], c: &Constraints) -> Result<Option<FaceIndex>, String> {
    let Some(cfg) = pack().filter(|_| pack_installed()).and_then(|p| p.faces.as_ref()) else { return Ok(None) };
    let spec = table(Some(cfg));
    let dir = analysis.dir().ok_or_else(crate::smart_search::no_folder)?;
    // the table as the analysis keeps it, or read from its file when faces are off right now
    let shared = match analysis.table(&spec) {
        Some(t) => t,
        None => Arc::new(RwLock::new(Table::open(dir, &spec)?)),
    };
    let p = Params::default();
    let mut faces: Vec<Face> = vec![];
    let mut records: HashMap<FaceId, FaceRecord> = HashMap::new();
    let mut keys: HashMap<u128, &str> = HashMap::new();
    {
        let t = shared.read().unwrap_or_else(PoisonError::into_inner);
        for (key, modified, size) in files {
            let Some(found) = t.get(key, (*modified, *size)) else { continue };
            let pic = crate::ai::store::picture_id(key);
            keys.insert(pic, key);
            for (i, r) in found.into_iter().enumerate() {
                let Some(r) = FaceRecord::from_bytes(r) else { continue };
                let id = (pic, i as u16);
                records.insert(id, r);
            }
        }
    }
    // in the pictures' order
    let nonface = crate::face_groups::nonface_direction(records.values());
    for (key, ..) in files {
        let pic = crate::ai::store::picture_id(key);
        for i in 0u16.. {
            let Some(r) = records.get(&(pic, i)) else { break };
            if p.is_grouped(r, nonface.as_deref()) {
                faces.push(Face { id: (pic, i), vector: r.vector.clone().unwrap_or_default(), good: p.is_good(r) });
            }
        }
    }
    let path = saved_path(dir, &spec.model);
    let before = load_groups(&path, &records);
    let groups = crate::face_groups::regroup(&faces, &before, c, &p);
    if groups != before || !path.exists() {
        save_groups(&path, &groups, &records)?;
    }
    let proposals = crate::face_groups::proposals(&faces, &groups, c, &p, MIN_GROUP);
    // the index: per picture, per group
    let mut pictures: HashMap<String, Vec<IndexedFace>> = HashMap::new();
    for (id, r) in &records {
        let Some(key) = keys.get(&id.0) else { continue };
        pictures.entry(key.to_string()).or_default().push(IndexedFace { index: id.1, rect: r.rect, group: groups.of.get(id).copied(), score: r.score, good: p.is_good(r) });
    }
    for v in pictures.values_mut() {
        v.sort_by_key(|f| f.index);
    }
    let order: HashMap<u128, usize> = files.iter().enumerate().map(|(n, f)| (crate::ai::store::picture_id(&f.0), n)).collect();
    let mut list: Vec<GroupInfo> = groups
        .members()
        .into_iter()
        .map(|(g, mut members)| {
            members.sort_by_key(|f| (order.get(&f.0).copied().unwrap_or(usize::MAX), f.1));
            // the cover: a good face, the largest, then the surest
            let cover = members
                .iter()
                .max_by(|a, b| {
                    let (ra, rb) = (&records[a], &records[b]);
                    (p.is_good(ra), ra.eyes, ra.score).partial_cmp(&(p.is_good(rb), rb.eyes, rb.score)).unwrap_or(std::cmp::Ordering::Equal)
                })
                .copied()
                .unwrap_or(members[0]);
            let pics: std::collections::HashSet<u128> = members.iter().map(|f| f.0).collect();
            let name = |f: &FaceId| (keys.get(&f.0).copied().unwrap_or_default().to_string(), f.1);
            GroupInfo { id: g, pictures: pics.len(), cover: name(&cover), faces: members.iter().map(name).collect() }
        })
        .collect();
    list.sort_by(|a, b| b.faces.len().cmp(&a.faces.len()).then(a.id.cmp(&b.id)));
    Ok(Some(FaceIndex { pictures, groups: list, proposals }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_faces_pack_is_built_in_with_its_recogniser() {
        let p = pack().expect("a Faces pack in models.json");
        let cfg = p.faces.as_ref().unwrap();
        assert!(cfg.valid(p));
        assert!(p.files.iter().any(|f| f.name == cfg.recogniser));
        let t = tables();
        assert_eq!(t.len(), 2);
        assert_ne!(t[0], t[1], "found-only faces and faces with vectors are kept apart");
    }

    #[test]
    fn groups_are_kept_and_follow_their_faces() {
        let dir = std::env::temp_dir().join(format!("albumcraft-face-groups-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let rec = |x: f32| FaceRecord { rect: [x, 0.2, 0.1, 0.1], landmarks: [[0.0; 2]; 5], score: 0.95, eyes: 30.0, sharpness: 100.0, yaw: 0.0, from_full: false, vector: None };
        let records: HashMap<FaceId, FaceRecord> = HashMap::from([((7, 0), rec(0.1)), ((7, 1), rec(0.5)), ((9, 0), rec(0.3))]);
        let groups = Groups { of: HashMap::from([((7, 0), 4), ((7, 1), 5), ((9, 0), 4)]), next: 6 };
        let path = saved_path(&dir, "test-1");
        save_groups(&path, &groups, &records).unwrap();
        assert_eq!(load_groups(&path, &records), groups);
        // picture 7 changed: its first face moved away, the second stayed where it was; picture
        // 9 is gone
        let after: HashMap<FaceId, FaceRecord> = HashMap::from([((7, 0), rec(0.7)), ((7, 1), rec(0.51))]);
        let kept = load_groups(&path, &after);
        assert_eq!(kept.of, HashMap::from([((7, 1), 5)]));
        assert_eq!(kept.next, 6, "group numbers are never given twice");
        // a damaged file is no groups, not an error
        std::fs::write(&path, b"{").unwrap();
        assert_eq!(load_groups(&path, &records), Groups::default());
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The face analysis on real pictures, as the background analysis runs it (1400 px, the
    /// full-size picture for small faces): speed and what is found.
    /// `ALBUMCRAFT_FACES_PHOTOS=<folder> ALBUMCRAFT_FACES_PACK=<folder with sface-1.onnx>
    /// [ALBUMCRAFT_FACES_COUNT=200] cargo test --release -p albumcraft-core real_faces_analysis
    /// -- --ignored --nocapture`. Two threads, like the background analysis.
    #[test]
    #[ignore]
    fn real_faces_analysis() {
        use crate::ai::Analyser;
        let photos = std::env::var("ALBUMCRAFT_FACES_PHOTOS").expect("ALBUMCRAFT_FACES_PHOTOS");
        let dir = std::path::PathBuf::from(std::env::var("ALBUMCRAFT_FACES_PACK").expect("ALBUMCRAFT_FACES_PACK"));
        let count: usize = std::env::var("ALBUMCRAFT_FACES_COUNT").ok().and_then(|n| n.parse().ok()).unwrap_or(200);
        let mut files: Vec<String> = walk(std::path::Path::new(&photos));
        files.sort();
        // spread over the folders rather than the first ones
        let step = (files.len() / count.max(1)).max(1);
        let files: Vec<String> = files.into_iter().step_by(step).take(count).collect();
        let a = Arc::new(FaceAnalyser::new(Some((pack().unwrap(), dir.as_path()))));
        let started = std::time::Instant::now();
        let results: Vec<(u128, Vec<crate::ai::faces::FaceRecord>)> = std::thread::scope(|s| {
            let chunks: Vec<&[String]> = files.chunks(files.len().div_ceil(2).max(1)).collect();
            let handles: Vec<_> = chunks
                .into_iter()
                .map(|chunk| {
                    let a = a.clone();
                    s.spawn(move || {
                        chunk
                            .iter()
                            .filter_map(|key| {
                                let t = std::time::Instant::now();
                                let picture = crate::imaging::load_source(key, a.input_dim())?;
                                let full = || crate::imaging::load_source(key, 4096);
                                let found = a.faces(&picture, &full).unwrap();
                                Some((t.elapsed().as_millis(), found))
                            })
                            .collect::<Vec<_>>()
                    })
                })
                .collect();
            handles.into_iter().flat_map(|h| h.join().unwrap()).collect()
        });
        let wall = started.elapsed().as_secs_f32();
        let faces: Vec<&crate::ai::faces::FaceRecord> = results.iter().flat_map(|(_, f)| f).collect();
        let with = |p: &dyn Fn(&crate::ai::faces::FaceRecord) -> bool| faces.iter().filter(|f| p(f)).count();
        let mut ms: Vec<u128> = results.iter().map(|(t, _)| *t).collect();
        ms.sort_unstable();
        println!(
            "{} pictures in {wall:.1} s on 2 threads ({:.0} ms each; one picture: median {} ms, slowest {} ms); {} with faces; {} faces: {} with a vector, {} cut from the full-size picture, {} scored 0.9 or more",
            results.len(),
            wall * 1000.0 / results.len().max(1) as f32,
            ms.get(ms.len() / 2).copied().unwrap_or(0),
            ms.last().copied().unwrap_or(0),
            results.iter().filter(|(_, f)| !f.is_empty()).count(),
            faces.len(),
            with(&|f| f.vector.is_some()),
            with(&|f| f.from_full),
            with(&|f| f.score >= 0.9),
        );
        println!("50 000 pictures would take about {:.1} h", wall / results.len().max(1) as f32 * 50_000.0 / 3600.0);
    }

    fn walk(dir: &std::path::Path) -> Vec<String> {
        let mut out = vec![];
        for e in std::fs::read_dir(dir).into_iter().flatten().flatten() {
            let p = e.path();
            if p.is_dir() {
                out.extend(walk(&p));
            } else if p.extension().and_then(|x| x.to_str()).is_some_and(|x| matches!(x.to_ascii_lowercase().as_str(), "jpg" | "jpeg")) {
                out.push(p.to_string_lossy().into_owned());
            }
        }
        out
    }
}
