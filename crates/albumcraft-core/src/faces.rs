//! Faces in the library: which face analyser the background analysis runs.
//!
//! Faces are looked for only when the user said yes (`Store.faces_enabled`, asked once, also
//! in Settings › AI features) and the AI features are on. With the *Faces* pack downloaded,
//! each face gets a vector (`ai::faces`, the table "faces.yunet-1-<recogniser>"); without it
//! they are only found ("faces.yunet-1"), which the pictures are looked at again for once the
//! pack is there. The faces are then grouped by person (`face_groups`, `group`) and the groups
//! named (`set_name`): a name is a person, like a name typed on pictures (`people_of`).
//!
//! Face vectors are biometric data: they stay in the analysis folder of this computer
//! (`ai::store::analysis_dir`), never in `library.json`, sidecars or exported copies.

use crate::ai::faces::{FaceAnalyser, detect_only, table};
use crate::ai::models::{self, Pack};
use crate::ai::store::{Spec, Table};
use crate::analysis::{Analysis, SharedTable};
use std::collections::{HashMap, HashSet};
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

/// What the user decided about one face of a picture (in `Meta.faces`). It is found again by
/// its box (`r`, as the face's in the index: left, top, width, height as fractions), so it
/// survives the picture being analysed again.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct FaceNote {
    pub r: [f32; 4],
    pub s: FaceState,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum FaceState {
    /// Put in this group by the user: it stays there.
    Person(GroupId),
    /// Never in these groups ("Not this person").
    NotPerson(Vec<GroupId>),
    /// A face, but of nobody to group (a stranger, a poster): left out of the groups.
    Ignored,
    /// Not a face at all: left out, and not counted as a person.
    NotFace,
}

/// The box overlap at which a note is a face's.
const NOTE_OVERLAP: f32 = 0.5;

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
    /// Taken for a face (`Params::is_grouped`): what the "with people" filters count.
    pub counted: bool,
    /// What the user decided about it, if anything.
    pub state: Option<FaceState>,
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

/// The faces of the library and their groups (what the People view shows).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct FaceIndex {
    /// Picture -> its faces (pictures with none are left out).
    pub pictures: HashMap<String, Vec<IndexedFace>>,
    /// Pictures looked at for faces (with or without any).
    pub analysed: HashSet<String>,
    /// Largest first.
    pub groups: Vec<GroupInfo>,
    /// Groups that may be one person: (smaller, larger, how close).
    pub proposals: Vec<(GroupId, GroupId, f32)>,
    /// The next group number the grouping gives: a new person made by hand takes it.
    pub next: GroupId,
}

impl FaceIndex {
    /// Faces found, faces in a group.
    pub fn counts(&self) -> (usize, usize) {
        let all = self.pictures.values().map(Vec::len).sum();
        let grouped = self.pictures.values().flatten().filter(|f| f.group.is_some()).count();
        (all, grouped)
    }

    /// Faces taken for faces in a picture; None when it was not looked at yet.
    pub fn people_count(&self, key: &str) -> Option<usize> {
        if !self.analysed.contains(key) {
            return None;
        }
        Some(self.pictures.get(key).map_or(0, |f| f.iter().filter(|f| f.counted).count()))
    }

    pub fn group(&self, id: GroupId) -> Option<&GroupInfo> {
        self.groups.iter().find(|g| g.id == id)
    }

    /// The face's box.
    pub fn face(&self, key: &str, index: u16) -> Option<&IndexedFace> {
        self.pictures.get(key)?.iter().find(|f| f.index == index)
    }
}

// ---------------------------------------------------------------------------
// People: the names of the groups (in `library.json`) with the names typed on pictures

/// What the user decided about faces (from `library.json`), for the grouping: merges, refused
/// merges, and the notes on single faces (matched to faces by their box when grouping).
#[derive(Clone, Debug, Default)]
pub struct Decided {
    pub merged: HashMap<GroupId, GroupId>,
    pub refused: HashSet<(GroupId, GroupId)>,
    pub notes: Vec<(String, FaceNote)>,
}

pub fn decided(store: &crate::library::Store) -> Decided {
    Decided {
        merged: store.person_merged.iter().map(|(a, b)| (*a, *b)).collect(),
        refused: store.refused_merges.iter().map(|&(a, b)| (a.min(b), a.max(b))).collect(),
        notes: store.meta.iter().flat_map(|(k, m)| m.faces.iter().map(move |n| (k.clone(), n.clone()))).collect(),
    }
}

/// Merges only (enough to follow a group to the one it was added to).
pub fn constraints(store: &crate::library::Store) -> Constraints {
    Constraints { merged: store.person_merged.iter().map(|(a, b)| (*a, *b)).collect(), ..Default::default() }
}

/// The face of `faces` (boxes) a note is about: the one its box overlaps most, enough.
pub fn note_target(note: &[f32; 4], faces: impl Iterator<Item = (u16, [f32; 4])>) -> Option<u16> {
    faces.map(|(i, r)| (i, overlap(note, &r))).filter(|(_, o)| *o >= NOTE_OVERLAP).max_by(|a, b| a.1.total_cmp(&b.1)).map(|(i, _)| i)
}

/// Write what the user decided about a face (`state` None: forget it) on its picture's notes.
pub fn note_face(meta: &mut crate::library::Meta, rect: [f32; 4], state: Option<FaceState>) {
    meta.faces.retain(|n| overlap(&n.r, &rect) < NOTE_OVERLAP);
    if let Some(s) = state {
        meta.faces.push(FaceNote { r: rect.map(|v| (v * 1e4).round() / 1e4), s });
    }
}

/// The group `g` is now, after the user's merges.
pub fn resolve(store: &crate::library::Store, g: GroupId) -> GroupId {
    constraints(store).resolve(g)
}

/// The name of a group (after merges).
pub fn name_of(store: &crate::library::Store, g: GroupId) -> Option<&str> {
    store.person_names.get(&resolve(store, g)).map(String::as_str).filter(|n| !n.is_empty())
}

/// The group that has this name already (names compare as tags do), other than `but`.
pub fn group_named(store: &crate::library::Store, name: &str, but: Option<GroupId>) -> Option<GroupId> {
    let key = crate::tags::key(name);
    store.person_names.iter().filter(|(g, _)| !store.person_merged.contains_key(g) && Some(**g) != but).find(|(_, n)| crate::tags::key(n) == key).map(|(g, _)| *g)
}

/// Name group `g` (an empty name takes the name away). Refused when another group has that
/// name: `merge` adds `g` to it instead, as the user asked.
pub fn set_name(store: &mut crate::library::Store, g: GroupId, name: &str, merge: bool) -> Result<GroupId, GroupId> {
    let g = resolve(store, g);
    let name = name.trim();
    if name.is_empty() {
        store.person_names.remove(&g);
        return Ok(g);
    }
    if let Some(other) = group_named(store, name, Some(g)) {
        if !merge {
            return Err(other);
        }
        merge_groups(store, g, other);
        return Ok(other);
    }
    store.person_names.insert(g, name.to_string());
    Ok(g)
}

/// Add group `from` to group `into` for good (its name goes, `into` keeps its own, or takes
/// `from`'s when it has none).
pub fn merge_groups(store: &mut crate::library::Store, from: GroupId, into: GroupId) {
    let (from, into) = (resolve(store, from), resolve(store, into));
    if from == into {
        return;
    }
    if let Some(n) = store.person_names.remove(&from)
        && !store.person_names.contains_key(&into)
    {
        store.person_names.insert(into, n);
    }
    store.person_merged.insert(from, into);
    // a refused name of `from` is refused for the person now
    for r in store.refused_names.iter_mut().filter(|r| r.0 == from) {
        r.0 = into;
    }
}

/// A change to faces asked by the user.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Correction {
    /// Into that group (a person).
    MoveTo(GroupId),
    /// Into a group of their own.
    NewPerson,
    /// Never in that group.
    NotThisPerson(GroupId),
    /// A stranger, a poster: out of the groups.
    Ignore,
    NotFace,
    /// Forget what was decided about these faces.
    Reset,
}

/// A group number nobody has: for a new person made by hand.
pub fn new_person_id(store: &crate::library::Store, index: &FaceIndex) -> GroupId {
    let mut n = index.next;
    n = n.max(store.person_names.keys().max().map_or(0, |g| g + 1));
    n = n.max(store.person_merged.iter().map(|(a, b)| a.max(b) + 1).max().unwrap_or(0));
    n = n.max(index.groups.iter().map(|g| g.id + 1).max().unwrap_or(0));
    for m in store.meta.values() {
        for f in &m.faces {
            if let FaceState::Person(g) = f.s {
                n = n.max(g + 1);
            }
        }
    }
    n
}

/// Write the user's correction on these faces (picture, place among its faces) into their
/// pictures' notes; how many were found. The groups follow at the next grouping.
pub fn correct(store: &mut crate::library::Store, index: &FaceIndex, faces: &[(String, u16)], c: Correction) -> usize {
    let new = (c == Correction::NewPerson).then(|| new_person_id(store, index));
    let mut done = 0;
    for (key, i) in faces {
        let Some(face) = index.face(key, *i) else { continue };
        let state = match c {
            Correction::MoveTo(g) => Some(FaceState::Person(resolve(store, g))),
            Correction::NewPerson => new.map(FaceState::Person),
            Correction::NotThisPerson(g) => {
                let g = resolve(store, g);
                let mut not = match &face.state {
                    Some(FaceState::NotPerson(gs)) => gs.clone(),
                    _ => vec![],
                };
                if !not.contains(&g) {
                    not.push(g);
                }
                Some(FaceState::NotPerson(not))
            }
            Correction::Ignore => Some(FaceState::Ignored),
            Correction::NotFace => Some(FaceState::NotFace),
            Correction::Reset => None,
        };
        note_face(store.meta.entry(key.clone()).or_default(), face.rect, state);
        done += 1;
    }
    done
}

/// For each picture, the names of the people recognised in it (named groups only).
pub fn names_by_picture(index: &FaceIndex, store: &crate::library::Store) -> HashMap<String, Vec<String>> {
    let mut out: HashMap<String, Vec<String>> = HashMap::new();
    for (key, faces) in &index.pictures {
        let mut names: Vec<String> = faces.iter().filter_map(|f| name_of(store, f.group?)).map(str::to_string).collect();
        names.sort();
        names.dedup();
        if !names.is_empty() {
            out.insert(key.clone(), names);
        }
    }
    out
}

/// The names of the people recognised in one picture.
pub fn names_in(index: &FaceIndex, store: &crate::library::Store, key: &str) -> Vec<String> {
    let mut names: Vec<String> = index.pictures.get(key).into_iter().flatten().filter_map(|f| name_of(store, f.group?)).map(str::to_string).collect();
    names.sort();
    names.dedup();
    names
}

/// The people of a picture: the names typed on it and the people recognised in it, each once
/// (as written on the picture first).
pub fn people_of(meta: Option<&crate::library::Meta>, recognised: Option<&Vec<String>>) -> Vec<String> {
    let mut out: Vec<String> = meta.map(|m| m.people.clone()).unwrap_or_default();
    let mut seen: HashSet<String> = out.iter().map(|n| crate::tags::key(n)).collect();
    for n in recognised.into_iter().flatten() {
        if seen.insert(crate::tags::key(n)) {
            out.push(n.clone());
        }
    }
    out
}

/// Names proposed for unnamed groups, from the names typed on their pictures, without those
/// the user refused.
pub fn suggestions(index: &FaceIndex, store: &crate::library::Store) -> Vec<(GroupId, String)> {
    let mut groups = Groups::default();
    let mut names: HashMap<u128, Vec<String>> = HashMap::new();
    for g in index.groups.iter().filter(|g| g.faces.len() >= MIN_GROUP) {
        for (key, i) in &g.faces {
            let pic = crate::ai::store::picture_id(key);
            groups.of.insert((pic, *i), g.id);
            if let Some(m) = store.meta.get(key).filter(|m| !m.people.is_empty()) {
                names.insert(pic, m.people.clone());
            }
        }
    }
    let named: HashMap<GroupId, String> = store.person_names.iter().filter(|(g, _)| !store.person_merged.contains_key(g)).map(|(g, n)| (*g, n.clone())).collect();
    crate::face_groups::name_suggestions(&groups, &names, &named, 0.6, 3)
        .into_iter()
        .filter(|(g, n)| !store.refused_names.iter().any(|(rg, rn)| rg == g && *rn == crate::tags::key(n)))
        .collect()
}

// ---------------------------------------------------------------------------
// Reading the faces and keeping the groups

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
/// Reference faces kept per named person (`face-refs`).
const REFS: usize = 20;

fn saved_path(dir: &std::path::Path, model: &str) -> std::path::PathBuf {
    dir.join(format!("face-groups.{model}.json"))
}

fn refs_path(dir: &std::path::Path, model: &str) -> std::path::PathBuf {
    dir.join(format!("face-refs.{model}.bin"))
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

const REFS_MAGIC: &[u8; 8] = b"ACFREFS1";

/// Reference faces of named people: group -> vectors (`face-refs.<model>.bin`: magic, then per
/// group its number, how many vectors, and their values; little-endian).
fn encode_refs(refs: &std::collections::BTreeMap<GroupId, Vec<Vec<f32>>>) -> Vec<u8> {
    let mut out = REFS_MAGIC.to_vec();
    for (g, vs) in refs {
        out.extend_from_slice(&g.to_le_bytes());
        out.extend_from_slice(&(vs.len() as u32).to_le_bytes());
        out.extend_from_slice(&(vs.first().map_or(0, Vec::len) as u32).to_le_bytes());
        for v in vs {
            v.iter().for_each(|x| out.extend_from_slice(&x.to_le_bytes()));
        }
    }
    out
}

fn decode_refs(b: &[u8]) -> std::collections::BTreeMap<GroupId, Vec<Vec<f32>>> {
    let mut out = std::collections::BTreeMap::new();
    let Some(mut rest) = b.strip_prefix(REFS_MAGIC.as_slice()) else { return out };
    let u32_at = |b: &[u8], i: usize| b.get(i..i + 4).map(|x| u32::from_le_bytes(x.try_into().unwrap()));
    while let (Some(g), Some(n), Some(dim)) = (u32_at(rest, 0), u32_at(rest, 4), u32_at(rest, 8)) {
        let (n, dim) = (n as usize, dim as usize);
        let need = 12 + n.saturating_mul(dim).saturating_mul(4);
        if dim == 0 || dim > 4096 || n > 1000 || rest.len() < need {
            break;
        }
        let vs = (0..n).map(|k| (0..dim).map(|j| f32::from_le_bytes(rest[12 + (k * dim + j) * 4..][..4].try_into().unwrap())).collect()).collect();
        out.insert(g, vs);
        rest = &rest[need..];
    }
    out
}

/// The faces of the library as read from the table, in the pictures' order.
struct Read {
    faces: Vec<Face>,
    records: HashMap<FaceId, FaceRecord>,
    keys: HashMap<u128, String>,
    analysed: HashSet<String>,
    counted: HashSet<FaceId>,
}

fn read(t: &Table, files: &[crate::geo::FileStamp], p: &Params) -> Read {
    let mut records: HashMap<FaceId, FaceRecord> = HashMap::new();
    let mut keys = HashMap::new();
    let mut analysed = HashSet::new();
    for (key, modified, size) in files {
        let Some(found) = t.get(key, (*modified, *size)) else { continue };
        analysed.insert(key.clone());
        let pic = crate::ai::store::picture_id(key);
        keys.insert(pic, key.clone());
        for (i, r) in found.into_iter().enumerate() {
            if let Some(r) = FaceRecord::from_bytes(r) {
                records.insert((pic, i as u16), r);
            }
        }
    }
    let nonface = crate::face_groups::nonface_direction(records.values());
    let mut faces = vec![];
    let mut counted = HashSet::new();
    for (key, ..) in files {
        let pic = crate::ai::store::picture_id(key);
        for i in 0u16.. {
            let Some(r) = records.get(&(pic, i)) else { break };
            if p.is_grouped(r, nonface.as_deref()) {
                counted.insert((pic, i));
                faces.push(Face { id: (pic, i), vector: r.vector.clone().unwrap_or_default(), good: p.is_good(r) });
            }
        }
    }
    Read { faces, records, keys, analysed, counted }
}

/// The user's decisions as constraints on these faces, and each face's state.
fn apply(d: &Decided, r: &Read) -> (Constraints, HashMap<FaceId, FaceState>) {
    let mut c = Constraints { merged: d.merged.clone(), refused: d.refused.clone(), ..Default::default() };
    let mut by_pic: HashMap<u128, Vec<(u16, [f32; 4])>> = HashMap::new();
    for (id, rec) in &r.records {
        by_pic.entry(id.0).or_default().push((id.1, rec.rect));
    }
    let mut states = HashMap::new();
    for (key, note) in &d.notes {
        let pic = crate::ai::store::picture_id(key);
        let Some(i) = by_pic.get(&pic).and_then(|f| note_target(&note.r, f.iter().copied())) else { continue };
        let id = (pic, i);
        match &note.s {
            FaceState::Person(g) => {
                let g = c.resolve(*g);
                c.placed.insert(id, g);
            }
            FaceState::NotPerson(gs) => {
                let gs: Vec<GroupId> = gs.iter().map(|g| c.resolve(*g)).collect();
                c.not_in.insert(id, gs);
            }
            FaceState::Ignored | FaceState::NotFace => {
                c.left_out.insert(id);
            }
        }
        states.insert(id, note.s.clone());
    }
    (c, states)
}

/// Faces of named people that lost their group (the analysis was made again from nothing):
/// those matching the person's reference faces go back to it.
fn restore_named(r: &Read, before: &mut Groups, refs: &std::collections::BTreeMap<GroupId, Vec<Vec<f32>>>, named: &HashSet<GroupId>, p: &Params) {
    let present: HashSet<GroupId> = before.of.values().copied().collect();
    for (g, vs) in refs.iter().filter(|(g, _)| named.contains(g) && !present.contains(g)) {
        let free: Vec<&Face> = r.faces.iter().filter(|f| !before.of.contains_key(&f.id)).collect();
        for f in free {
            let mut sims: Vec<f32> = vs.iter().filter(|v| v.len() == f.vector.len()).map(|v| v.iter().zip(&f.vector).map(|(a, b)| a * b).sum()).collect();
            sims.sort_by(|a, b| b.total_cmp(a));
            let top = &sims[..sims.len().min(p.top)];
            if !top.is_empty() && top.iter().sum::<f32>() / top.len() as f32 >= p.join {
                before.of.insert(f.id, *g);
            }
        }
        before.next = before.next.max(g + 1);
    }
}

/// Reference faces of the named groups: their good faces, spread over the group.
fn make_refs(r: &Read, groups: &Groups, named: &HashSet<GroupId>) -> std::collections::BTreeMap<GroupId, Vec<Vec<f32>>> {
    let by_id: HashMap<FaceId, &Face> = r.faces.iter().map(|f| (f.id, f)).collect();
    let mut out = std::collections::BTreeMap::new();
    for (g, members) in groups.members().into_iter().filter(|(g, _)| named.contains(g)) {
        let mut good: Vec<&Face> = members.iter().filter_map(|f| by_id.get(f).copied()).filter(|f| f.good).collect();
        if good.is_empty() {
            good = members.iter().filter_map(|f| by_id.get(f).copied()).collect();
        }
        let step = (good.len() / REFS).max(1);
        out.insert(g, good.iter().step_by(step).take(REFS).map(|f| f.vector.clone()).collect());
    }
    out
}

fn index_of(r: &Read, groups: &Groups, proposals: Vec<(GroupId, GroupId, f32)>, files: &[crate::geo::FileStamp], p: &Params, states: &HashMap<FaceId, FaceState>) -> FaceIndex {
    let mut pictures: HashMap<String, Vec<IndexedFace>> = HashMap::new();
    for (id, rec) in &r.records {
        let Some(key) = r.keys.get(&id.0) else { continue };
        pictures.entry(key.clone()).or_default().push(IndexedFace {
            index: id.1,
            rect: rec.rect,
            group: groups.of.get(id).copied(),
            score: rec.score,
            good: p.is_good(rec),
            counted: r.counted.contains(id) && states.get(id) != Some(&FaceState::NotFace) || matches!(states.get(id), Some(FaceState::Person(_))),
            state: states.get(id).cloned(),
        });
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
                    let (ra, rb) = (&r.records[a], &r.records[b]);
                    (p.is_good(ra), ra.eyes, ra.score).partial_cmp(&(p.is_good(rb), rb.eyes, rb.score)).unwrap_or(std::cmp::Ordering::Equal)
                })
                .copied()
                .unwrap_or(members[0]);
            let pics: HashSet<u128> = members.iter().map(|f| f.0).collect();
            let name = |f: &FaceId| (r.keys.get(&f.0).cloned().unwrap_or_default(), f.1);
            GroupInfo { id: g, pictures: pics.len(), cover: name(&cover), faces: members.iter().map(name).collect() }
        })
        .collect();
    list.sort_by(|a, b| b.faces.len().cmp(&a.faces.len()).then(a.id.cmp(&b.id)));
    FaceIndex { pictures, analysed: r.analysed.clone(), groups: list, proposals, next: groups.next }
}

/// Group the faces of `files` (the library's pictures, in the order to place them) found with
/// the Faces pack, keeping what was decided before (`c`: the user's decisions; `named`: the
/// groups that have a name, whose reference faces are kept so that the name survives a new
/// analysis); the groups are saved beside the table. None when faces have no vectors yet.
pub fn group(analysis: &Analysis, files: &[crate::geo::FileStamp], d: &Decided, named: &HashSet<GroupId>) -> Result<Option<FaceIndex>, String> {
    let Some(cfg) = pack().filter(|_| pack_installed()).and_then(|p| p.faces.as_ref()) else { return Ok(None) };
    let spec = table(Some(cfg));
    let dir = analysis.dir().ok_or_else(crate::smart_search::no_folder)?;
    // the table as the analysis keeps it, or read from its file when faces are off right now
    let shared = match analysis.table(&spec) {
        Some(t) => t,
        None => Arc::new(RwLock::new(Table::open(dir, &spec)?)),
    };
    let p = Params::default();
    let r = read(&shared.read().unwrap_or_else(PoisonError::into_inner), files, &p);
    let (c, states) = apply(d, &r);
    let c = &c;
    let path = saved_path(dir, &spec.model);
    let mut before = load_groups(&path, &r.records);
    let refs_file = refs_path(dir, &spec.model);
    let old_refs = std::fs::read(&refs_file).map(|b| decode_refs(&b)).unwrap_or_default();
    restore_named(&r, &mut before, &old_refs, named, &p);
    let mut groups = crate::face_groups::regroup(&r.faces, &before, c, &p);
    // faces the user placed that are not grouped by themselves (too small, an unsure detection)
    for (id, g) in &c.placed {
        if r.records.contains_key(id) {
            groups.of.insert(*id, *g);
            groups.next = groups.next.max(g + 1);
        }
    }
    if groups != before || !path.exists() {
        save_groups(&path, &groups, &r.records)?;
    }
    // the reference faces follow the named groups; a group that lost every face keeps its old ones
    let mut refs = make_refs(&r, &groups, named);
    for (g, v) in old_refs {
        if named.contains(&g) {
            refs.entry(g).or_insert(v);
        }
    }
    let bytes = encode_refs(&refs);
    if std::fs::read(&refs_file).ok().as_deref() != Some(bytes.as_slice()) {
        crate::library::write_library(&refs_file, &bytes)?;
    }
    let proposals = crate::face_groups::proposals(&r.faces, &groups, c, &p, MIN_GROUP);
    Ok(Some(index_of(&r, &groups, proposals, files, &p, &states)))
}

/// The faces and groups as last saved, without grouping again (for the commands when the
/// window is not open). None without the Faces pack or before any grouping.
pub fn saved_index(files: &[crate::geo::FileStamp], d: &Decided) -> Option<FaceIndex> {
    let cfg = pack().filter(|_| pack_installed()).and_then(|p| p.faces.as_ref())?;
    let spec = table(Some(cfg));
    let dir = crate::ai::store::analysis_dir()?;
    let path = saved_path(&dir, &spec.model);
    if !path.exists() {
        return None;
    }
    let p = Params::default();
    let r = read(&Table::open(&dir, &spec).ok()?, files, &p);
    let mut groups = load_groups(&path, &r.records);
    let (c, states) = apply(d, &r);
    for g in groups.of.values_mut() {
        *g = c.resolve(*g);
    }
    groups.of.retain(|f, _| !c.left_out.contains(f));
    groups.of.extend(c.placed.iter().map(|(f, g)| (*f, *g)));
    Some(index_of(&r, &groups, vec![], files, &p, &states))
}

/// Remove every face found, the groups and the reference faces from this computer (the names
/// and the user's decisions stay in `library.json`).
pub fn delete_face_data(analysis: &Analysis) -> Result<(), String> {
    for spec in tables() {
        analysis.remove(&spec);
    }
    analysers().clear();
    let dir = analysis.dir().ok_or_else(crate::smart_search::no_folder)?;
    crate::ai::store::remove_kind(dir, "faces")?;
    for e in std::fs::read_dir(dir).into_iter().flatten().flatten() {
        let name = e.file_name().to_string_lossy().into_owned();
        if name.starts_with("face-groups.") || name.starts_with("face-refs.") {
            std::fs::remove_file(e.path()).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
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

    #[test]
    fn names_merges_and_the_people_of_a_picture() {
        let mut store = crate::library::Store::default();
        assert_eq!(set_name(&mut store, 4, " Anna ", false), Ok(4));
        assert_eq!(name_of(&store, 4), Some("Anna"));
        // another group named anna: refused, unless merging
        assert_eq!(set_name(&mut store, 7, "anna", false), Err(4));
        assert_eq!(name_of(&store, 7), None);
        assert_eq!(set_name(&mut store, 7, "anna", true), Ok(4));
        assert_eq!(resolve(&store, 7), 4);
        assert_eq!(name_of(&store, 7), Some("Anna"), "a merged group is its person");
        // merging an unnamed group into another keeps the name of the one named
        set_name(&mut store, 9, "Marc", false).unwrap();
        merge_groups(&mut store, 9, 11);
        assert_eq!(name_of(&store, 11), Some("Marc"));
        assert_eq!(group_named(&store, "MARC", None), Some(11));
        // naming with nothing takes the name away
        set_name(&mut store, 11, "", false).unwrap();
        assert_eq!(name_of(&store, 9), None);
        // a picture's people: typed names and recognised ones, each once
        let meta = crate::library::Meta { people: vec!["Anna".into(), "Zoé".into()], ..Default::default() };
        assert_eq!(people_of(Some(&meta), Some(&vec!["anna".into(), "Paul".into()])), vec!["Anna", "Zoé", "Paul"]);
        assert_eq!(people_of(None, None), Vec::<String>::new());
        // the index gives each picture the names of its named faces
        let face = |g: Option<GroupId>| IndexedFace { index: 0, rect: [0.0; 4], group: g, score: 0.9, good: true, counted: true, state: None };
        let index = FaceIndex {
            pictures: HashMap::from([("a.jpg".to_string(), vec![face(Some(7)), IndexedFace { index: 1, ..face(None) }]), ("b.jpg".to_string(), vec![face(Some(12))])]),
            analysed: HashSet::from(["a.jpg".to_string(), "b.jpg".to_string(), "c.jpg".to_string()]),
            ..Default::default()
        };
        let names = names_by_picture(&index, &store);
        assert_eq!(names.get("a.jpg"), Some(&vec!["Anna".to_string()]));
        assert!(!names.contains_key("b.jpg"), "an unnamed group adds no name");
        assert_eq!((index.people_count("a.jpg"), index.people_count("c.jpg"), index.people_count("d.jpg")), (Some(2), Some(0), None));
    }

    #[test]
    fn names_come_back_after_a_new_analysis() {
        let p = Params::default();
        let v = |x: f32| {
            let mut v = vec![0.0; 8];
            v[0] = 1.0;
            v[1] = x;
            crate::ai::search::normalise(&mut v);
            v
        };
        let faces: Vec<Face> = (0..5u128).map(|k| Face { id: (k, 0), vector: v(k as f32 * 0.05), good: true }).collect();
        let r = Read { faces: faces.clone(), records: HashMap::new(), keys: HashMap::new(), analysed: HashSet::new(), counted: HashSet::new() };
        let groups = Groups { of: faces.iter().map(|f| (f.id, 3)).collect(), next: 4 };
        let named = HashSet::from([3]);
        let refs = make_refs(&r, &groups, &named);
        assert_eq!(refs[&3].len(), 5);
        assert_eq!(decode_refs(&encode_refs(&refs)), refs);
        assert!(decode_refs(b"ACFREFS1\x03\0\0\0\xff\xff").is_empty(), "a damaged file is nothing, not a crash");
        // everything was analysed again: no groups, but person 3's faces go back to it
        let mut before = Groups::default();
        restore_named(&r, &mut before, &refs, &named, &p);
        assert_eq!(before.of.len(), 5);
        assert!(before.of.values().all(|&g| g == 3) && before.next == 4);
        // an unnamed group's faces are not given back
        let mut before = Groups::default();
        restore_named(&r, &mut before, &refs, &HashSet::new(), &p);
        assert!(before.of.is_empty());
    }

    #[test]
    fn corrections_find_their_faces_again() {
        let rec = |x: f32| FaceRecord { rect: [x, 0.2, 0.1, 0.12], landmarks: [[0.0; 2]; 5], score: 0.95, eyes: 30.0, sharpness: 100.0, yaw: 0.0, from_full: false, vector: None };
        let pic = crate::ai::store::picture_id("p.jpg");
        let records: HashMap<FaceId, FaceRecord> = (0..4u16).map(|i| ((pic, i), rec(0.1 + 0.2 * i as f32))).collect();
        let r = Read { faces: vec![], records, keys: HashMap::from([(pic, "p.jpg".to_string())]), analysed: HashSet::new(), counted: HashSet::new() };
        // notes written on an earlier analysis: the boxes a little off now
        let mut meta = crate::library::Meta::default();
        note_face(&mut meta, [0.105, 0.21, 0.1, 0.12], Some(FaceState::Person(7)));
        note_face(&mut meta, [0.3, 0.2, 0.1, 0.12], Some(FaceState::NotPerson(vec![2, 3])));
        note_face(&mut meta, [0.5, 0.2, 0.1, 0.12], Some(FaceState::Ignored));
        note_face(&mut meta, [0.7, 0.2, 0.1, 0.12], Some(FaceState::NotFace));
        // a second note on the same face replaces the first
        note_face(&mut meta, [0.5, 0.2, 0.1, 0.12], Some(FaceState::NotFace));
        assert_eq!(meta.faces.len(), 4);
        let mut store = crate::library::Store::default();
        store.meta.insert("p.jpg".into(), meta.clone());
        store.person_merged.insert(7, 9);
        let (c, states) = apply(&decided(&store), &r);
        assert_eq!(c.placed.get(&(pic, 0)), Some(&9), "placed in the group 7 became");
        assert_eq!(c.not_in.get(&(pic, 1)), Some(&vec![2, 3]));
        assert!(c.left_out.contains(&(pic, 2)) && c.left_out.contains(&(pic, 3)));
        assert_eq!(states.get(&(pic, 2)), Some(&FaceState::NotFace));
        // a note whose face is not found any more does nothing
        let mut far = meta.clone();
        far.faces = vec![FaceNote { r: [0.9, 0.9, 0.05, 0.05], s: FaceState::Ignored }];
        store.meta.insert("p.jpg".into(), far);
        assert!(apply(&decided(&store), &r).0.left_out.is_empty());
        // forgetting
        note_face(&mut meta, [0.7, 0.2, 0.1, 0.12], None);
        assert_eq!(meta.faces.len(), 3);
        // as kept in library.json
        let json = serde_json::to_string(&meta.faces[0]).unwrap();
        assert!(json.contains("\"person\":7"), "{json}");
        let back: Vec<FaceNote> = serde_json::from_str(&serde_json::to_string(&meta.faces).unwrap()).unwrap();
        assert_eq!(back, meta.faces);
    }

    #[test]
    fn corrections_are_written_on_the_pictures() {
        let face = |i: u16, x: f32, g: Option<GroupId>| IndexedFace { index: i, rect: [x, 0.1, 0.1, 0.1], group: g, score: 0.9, good: true, counted: true, state: None };
        let index = FaceIndex {
            pictures: HashMap::from([("a.jpg".to_string(), vec![face(0, 0.1, Some(3)), face(1, 0.5, Some(3))])]),
            groups: vec![GroupInfo { id: 3, faces: vec![("a.jpg".into(), 0), ("a.jpg".into(), 1)], pictures: 1, cover: ("a.jpg".into(), 0) }],
            next: 5,
            ..Default::default()
        };
        let mut store = crate::library::Store::default();
        store.person_names.insert(8, "Anna".into());
        // a new person: a number nobody has
        assert_eq!(new_person_id(&store, &index), 9);
        assert_eq!(correct(&mut store, &index, &[("a.jpg".into(), 0)], Correction::NewPerson), 1);
        assert_eq!(store.meta["a.jpg"].faces[0].s, FaceState::Person(9));
        assert_eq!(new_person_id(&store, &index), 10, "taken now");
        // not this person: the group, and others said before
        let mut index2 = index.clone();
        index2.pictures.get_mut("a.jpg").unwrap()[1].state = Some(FaceState::NotPerson(vec![1]));
        correct(&mut store, &index2, &[("a.jpg".into(), 1)], Correction::NotThisPerson(3));
        assert!(store.meta["a.jpg"].faces.iter().any(|f| f.s == FaceState::NotPerson(vec![1, 3])));
        // unknown faces are skipped; reset forgets
        assert_eq!(correct(&mut store, &index, &[("b.jpg".into(), 0), ("a.jpg".into(), 7)], Correction::Ignore), 0);
        correct(&mut store, &index, &[("a.jpg".into(), 0), ("a.jpg".into(), 1)], Correction::Reset);
        assert!(store.meta["a.jpg"].faces.is_empty());
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
