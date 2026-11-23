//! Grouping faces by person, from their vectors (`ai::faces`). No files, no threads: `regroup`
//! takes the faces and what was decided before, and gives the groups.
//!
//! The rules (`ai-features.md`, "Automatic grouping", and `faces-plan.md`):
//! - **A group is a set of faces, not one average.** A face is compared with up to
//!   `Params::exemplars` faces of each group, chosen to cover it (different ages, poses,
//!   glasses), and its score is the mean of its `Params::top` best matches.
//! - **Groups are never reshuffled.** A face put in a group stays there; only faces in no group
//!   are placed. That keeps what the user has seen and named.
//! - **Prefer splitting to mixing.** A new group starts only from three faces that all match each
//!   other (`Params::seed`), and a face that joins becomes one of the group's exemplars only when
//!   it matches several of them (`Params::support`), so that groups grow from what they are
//!   rather than drift towards a look-alike; two groups of one person are proposed for merging
//!   instead (`proposals`).
//! - **One person is in a picture once.** Two faces of one picture never go in one group
//!   (crowds of look-alike faces at night were mixing strangers); a group made before the rule
//!   keeps the face of a picture that fits it best (the user's placements stay).
//! - **Poor faces only join.** Small, blurred or turned-away faces never start a group.
//! - **The user's corrections are constraints** (`Constraints`): a face placed by the user stays
//!   in its group, "not this person" keeps a face out of a group for good, ignored faces and
//!   things that are not faces are left out, refused merges are not proposed again.

use std::collections::{HashMap, HashSet};

/// A face: its picture (`ai::store::picture_id`) and its place among that picture's faces.
pub type FaceId = (u128, u16);

/// A group's number (a person, named or not). Never reused.
pub type GroupId = u32;

/// A face to group.
#[derive(Clone, Debug)]
pub struct Face {
    pub id: FaceId,
    /// Unit length.
    pub vector: Vec<f32>,
    /// Good enough to start a group (`Params::is_good`).
    pub good: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Params {
    /// A face joins a group scoring this much (cosine, mean of its best matches).
    pub join: f32,
    /// Three faces matching each other this much start a group.
    pub seed: f32,
    /// How many of a group's best matches make a face's score.
    pub top: usize,
    /// Faces of a group a new face is compared with.
    pub exemplars: usize,
    /// A face closer than this to a group's exemplar adds nothing as an exemplar.
    pub diverse: f32,
    /// A face that joined becomes an exemplar only when it matches (`join`) this many of the
    /// group's exemplars: groups grow from what they are and cannot drift, face by face, from
    /// one person to a look-alike.
    pub support: usize,
    /// Groups this close (but below `join`) are proposed for merging.
    pub propose: f32,
    /// Detections below this score are not grouped at all: most are not faces (hair, wheels,
    /// statues), and their vectors are alike, so they would make a "person" of their own.
    pub min_score: f32,
    /// Faces this close to the library's non-faces (`nonface_direction`) are not grouped: what
    /// the detector takes for a face that isn't (a teddy bear, a mask, a statue) scores well
    /// sometimes, and such vectors are alike. Marked faces of the tuning library stay under 0.34.
    pub max_nonface: f32,
    /// What a face needs to start a group: detection score, eye distance in pixels, sharpness,
    /// turn.
    pub good_score: f32,
    pub good_eyes: f32,
    pub good_sharpness: f32,
    pub good_yaw: f32,
}

impl Default for Params {
    fn default() -> Params {
        Params {
            join: 0.45,
            seed: 0.5,
            top: 3,
            exemplars: 50,
            diverse: 0.85,
            support: 2,
            min_score: 0.8,
            max_nonface: 0.4,
            propose: 0.36,
            good_score: 0.85,
            good_eyes: 20.0,
            good_sharpness: 40.0,
            good_yaw: 0.15,
        }
    }
}

impl Params {
    /// The face is grouped (it may join a group); `nonface`: `nonface_direction`.
    pub fn is_grouped(&self, f: &crate::ai::faces::FaceRecord, nonface: Option<&[f32]>) -> bool {
        let Some(v) = &f.vector else { return false };
        f.score >= self.min_score && nonface.is_none_or(|n| dot(v, n) < self.max_nonface)
    }

    pub fn is_good(&self, f: &crate::ai::faces::FaceRecord) -> bool {
        f.score >= self.good_score && f.eyes >= self.good_eyes && f.sharpness >= self.good_sharpness && f.yaw.abs() <= self.good_yaw
    }
}

/// What the user decided (from `library.json`; the corrections come with the People view).
#[derive(Clone, Debug, Default)]
pub struct Constraints {
    /// Faces the user put in a group.
    pub placed: HashMap<FaceId, GroupId>,
    /// Groups a face must never be put in ("not this person").
    pub not_in: HashMap<FaceId, Vec<GroupId>>,
    /// Faces left out: ignored, or not a face.
    pub left_out: HashSet<FaceId>,
    /// Pairs of groups the user said are two people (smaller id first).
    pub refused: HashSet<(GroupId, GroupId)>,
    /// Groups the user merged: group -> the group it was added to (followed to the end).
    pub merged: HashMap<GroupId, GroupId>,
}

impl Constraints {
    /// The group `g` is now, after merges (a loop is cut where it closes).
    pub fn resolve(&self, mut g: GroupId) -> GroupId {
        let mut seen = HashSet::new();
        while let Some(&n) = self.merged.get(&g) {
            if !seen.insert(g) {
                break;
            }
            g = n;
        }
        g
    }
}

/// Which group each face is in.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Groups {
    pub of: HashMap<FaceId, GroupId>,
    /// The next group number to give.
    pub next: GroupId,
}

impl Groups {
    /// The faces of each group.
    pub fn members(&self) -> HashMap<GroupId, Vec<FaceId>> {
        let mut m: HashMap<GroupId, Vec<FaceId>> = HashMap::new();
        for (f, g) in &self.of {
            m.entry(*g).or_default().push(*f);
        }
        for v in m.values_mut() {
            v.sort_unstable();
        }
        m
    }
}

/// What the library's non-faces look like: the mean vector of its detections scoring under
/// 0.7 (mostly not faces), when there are enough of them to say.
pub fn nonface_direction<'a>(records: impl Iterator<Item = &'a crate::ai::faces::FaceRecord>) -> Option<Vec<f32>> {
    let mut sum: Option<Vec<f32>> = None;
    let mut n = 0;
    for v in records.filter(|r| r.score < 0.7).filter_map(|r| r.vector.as_ref()) {
        let s = sum.get_or_insert_with(|| vec![0.0; v.len()]);
        if s.len() == v.len() {
            s.iter_mut().zip(v).for_each(|(a, x)| *a += x);
            n += 1;
        }
    }
    let mut s = sum.filter(|_| n >= 50)?;
    crate::ai::search::normalise(&mut s);
    Some(s)
}

fn dot(a: &[f32], b: &[f32]) -> f32 {
    a.iter().zip(b).map(|(x, y)| x * y).sum()
}

/// One group while faces are placed: its exemplars (indexes into the faces) and their mean.
struct Group {
    id: GroupId,
    exemplars: Vec<usize>,
    sum: Vec<f32>,
    /// The pictures it has a face in: one person is in a picture once.
    pictures: HashSet<u128>,
}

impl Group {
    fn new(id: GroupId, dim: usize) -> Group {
        Group { id, exemplars: vec![], sum: vec![0.0; dim], pictures: HashSet::new() }
    }

    /// Take `i` as an exemplar when there is room and it shows something new.
    fn offer(&mut self, i: usize, faces: &[Face], p: &Params) -> bool {
        if self.exemplars.len() >= p.exemplars {
            return false;
        }
        let v = &faces[i].vector;
        if self.exemplars.iter().any(|&e| dot(&faces[e].vector, v) >= p.diverse) {
            return false;
        }
        self.exemplars.push(i);
        self.sum.iter_mut().zip(v).for_each(|(s, x)| *s += x);
        true
    }

    /// How well a face matches: the mean of its `top` best matches among the exemplars.
    fn score(&self, v: &[f32], faces: &[Face], top: usize) -> f32 {
        let mut best = [f32::NEG_INFINITY; 8];
        let top = top.clamp(1, best.len());
        for &e in &self.exemplars {
            let s = dot(&faces[e].vector, v);
            if s > best[top - 1] {
                best[top - 1] = s;
                best[..top].sort_by(|a, b| b.total_cmp(a));
            }
        }
        let found: Vec<f32> = best[..top].iter().copied().filter(|s| s.is_finite()).collect();
        if found.is_empty() { f32::NEG_INFINITY } else { found.iter().sum::<f32>() / found.len() as f32 }
    }

    /// The face matches enough of the exemplars to become one.
    fn supports(&self, v: &[f32], faces: &[Face], p: &Params) -> bool {
        let need = p.support.min(self.exemplars.len());
        self.exemplars.iter().filter(|&&e| dot(&faces[e].vector, v) >= p.join).count() >= need
    }

    /// A quick upper look: the face against the exemplars' mean. A face far from it is far from
    /// the group (checked with a margin, the mean of unit vectors being shorter than one).
    fn near(&self, v: &[f32], margin: f32) -> bool {
        let n = self.sum.iter().map(|x| x * x).sum::<f32>().sqrt();
        n == 0.0 || dot(&self.sum, v) / n >= margin
    }
}

/// The groups, after placing the faces in no group yet. `faces` in the order they should be
/// placed (when they were taken, so that a group grows as the person did); `previous`: what
/// was decided before (faces that are gone are dropped from it).
pub fn regroup(faces: &[Face], previous: &Groups, c: &Constraints, p: &Params) -> Groups {
    let dim = faces.first().map(|f| f.vector.len()).unwrap_or(0);
    let allowed = |i: usize, g: GroupId| !c.not_in.get(&faces[i].id).is_some_and(|n| n.contains(&g));
    // what stays: earlier decisions for faces still here, the user's placements over them
    let mut of: Vec<Option<GroupId>> = vec![None; faces.len()];
    for (i, f) in faces.iter().enumerate() {
        if c.left_out.contains(&f.id) || f.vector.len() != dim {
            continue;
        }
        let g = c.placed.get(&f.id).or_else(|| previous.of.get(&f.id)).map(|&g| c.resolve(g));
        of[i] = g.filter(|&g| c.placed.get(&f.id).map(|&p| c.resolve(p)) == Some(g) || allowed(i, g));
    }
    let mut next = previous.next.max(c.placed.values().max().map_or(0, |g| g + 1));
    // the groups as they are: the user's faces first, then good ones, in order; a second face
    // of a picture already in the group goes (one person is in a picture once), unless the user
    // put it there
    let mut groups: Vec<Group> = vec![];
    let mut at: HashMap<GroupId, usize> = HashMap::new();
    let mut order: Vec<usize> = (0..faces.len()).filter(|&i| of[i].is_some()).collect();
    order.sort_by_key(|&i| (!c.placed.contains_key(&faces[i].id), !faces[i].good, i));
    for i in order {
        let g = of[i].unwrap();
        let k = *at.entry(g).or_insert_with(|| {
            groups.push(Group::new(g, dim));
            groups.len() - 1
        });
        if !groups[k].pictures.insert(faces[i].id.0) && !c.placed.contains_key(&faces[i].id) {
            of[i] = None;
            continue;
        }
        groups[k].offer(i, faces, p);
    }
    let margin = p.join - 0.2;
    let best_group = |i: usize, groups: &[Group]| -> Option<(usize, f32)> {
        let v = &faces[i].vector;
        groups
            .iter()
            .enumerate()
            .filter(|(_, g)| allowed(i, g.id) && !g.pictures.contains(&faces[i].id.0) && g.near(v, margin))
            .map(|(k, g)| (k, g.score(v, faces, p.top)))
            .filter(|(_, s)| *s >= p.join)
            .max_by(|a, b| a.1.total_cmp(&b.1).then(b.0.cmp(&a.0)))
    };
    // place the others, in order
    let mut waiting: Vec<usize> = vec![];
    for i in 0..faces.len() {
        if of[i].is_some() || c.left_out.contains(&faces[i].id) || faces[i].vector.len() != dim || dim == 0 {
            continue;
        }
        if let Some((k, _)) = best_group(i, &groups) {
            of[i] = Some(groups[k].id);
            groups[k].pictures.insert(faces[i].id.0);
            if faces[i].good && groups[k].supports(&faces[i].vector, faces, p) {
                groups[k].offer(i, faces, p);
            }
            continue;
        }
        if !faces[i].good {
            continue;
        }
        // a new group: this face and two waiting ones that all match each other
        let v = &faces[i].vector;
        let pic = faces[i].id.0;
        let near: Vec<usize> = waiting.iter().copied().filter(|&w| faces[w].id.0 != pic && dot(&faces[w].vector, v) >= p.seed).collect();
        let mut seed: Option<Vec<usize>> = None;
        'pairs: for (x, &a) in near.iter().enumerate() {
            for &b in &near[x + 1..] {
                if faces[a].id.0 != faces[b].id.0 && dot(&faces[a].vector, &faces[b].vector) >= p.seed {
                    seed = Some(vec![i, a, b]);
                    break 'pairs;
                }
            }
        }
        let Some(mut members) = seed else {
            waiting.push(i);
            continue;
        };
        // and the other waiting ones that match at least two of them
        for &w in &near {
            let other_picture = members.iter().all(|&m| faces[m].id.0 != faces[w].id.0);
            if other_picture && members.iter().filter(|&&m| dot(&faces[m].vector, &faces[w].vector) >= p.seed).count() >= 2 {
                members.push(w);
            }
        }
        let id = next;
        next += 1;
        let mut g = Group::new(id, dim);
        for &m in &members {
            of[m] = Some(id);
            g.pictures.insert(faces[m].id.0);
            g.offer(m, faces, p);
        }
        waiting.retain(|w| !members.contains(w));
        at.insert(id, groups.len());
        groups.push(g);
    }
    // faces seen before a group of theirs was made: one more look
    for i in 0..faces.len() {
        if of[i].is_none() && !c.left_out.contains(&faces[i].id) && faces[i].vector.len() == dim && dim > 0
            && let Some((k, _)) = best_group(i, &groups)
        {
            of[i] = Some(groups[k].id);
            groups[k].pictures.insert(faces[i].id.0);
        }
    }
    Groups { of: faces.iter().zip(of).filter_map(|(f, g)| Some((f.id, g?))).collect(), next }
}

/// After the user took faces out of the groups `from` and put them in groups of their own
/// (their placed faces), look again at the other faces of `from`: those that match the placed
/// faces of another group (`join`) and match what is left of their own group clearly less go
/// with them. This is how a group of two look-alikes (a person and a brother) is split by
/// moving a few faces. Faces the user placed or left out never move; one person is in a
/// picture once.
pub fn recheck(faces: &[Face], groups: &Groups, c: &Constraints, p: &Params, from: &HashSet<GroupId>) -> Groups {
    let mut out = groups.clone();
    let from: std::collections::BTreeSet<GroupId> = from.iter().map(|&g| c.resolve(g)).collect();
    let index: HashMap<FaceId, usize> = faces.iter().enumerate().map(|(i, f)| (f.id, i)).collect();
    let mut anchors: std::collections::BTreeMap<GroupId, Vec<usize>> = Default::default();
    for (id, g) in &c.placed {
        if let Some(&i) = index.get(id) {
            anchors.entry(c.resolve(*g)).or_default().push(i);
        }
    }
    let top = |v: &[f32], set: &[usize]| -> f32 {
        let mut s: Vec<f32> = set.iter().map(|&e| dot(&faces[e].vector, v)).collect();
        s.sort_by(|a, b| b.total_cmp(a));
        s.truncate(p.top.max(1));
        if s.is_empty() { f32::NEG_INFINITY } else { s.iter().sum::<f32>() / s.len() as f32 }
    };
    let members = groups.members();
    for a in from {
        let Some(mem) = members.get(&a) else { continue };
        let mut mine: Vec<usize> = mem.iter().filter_map(|id| index.get(id).copied()).filter(|&i| faces[i].vector.len() == faces[0].vector.len()).collect();
        for (&t, anc) in anchors.iter().filter(|(t, _)| **t != a) {
            let in_t: HashSet<u128> = out.of.iter().filter(|(_, g)| **g == t).map(|(f, _)| f.0).collect();
            let free = |i: usize| !c.placed.contains_key(&faces[i].id) && !c.left_out.contains(&faces[i].id) && !c.not_in.get(&faces[i].id).is_some_and(|n| n.contains(&t));
            let mut moved: Vec<(usize, f32)> = mine.iter().copied().filter(|&i| free(i) && !in_t.contains(&faces[i].id.0)).map(|i| (i, top(&faces[i].vector, anc))).filter(|(_, s)| *s >= p.join).collect();
            if moved.is_empty() {
                continue;
            }
            // what is left of the group once they have gone: the faces must match the placed
            // ones clearly better than that
            let gone: HashSet<usize> = moved.iter().map(|m| m.0).collect();
            let rest: Vec<usize> = mine.iter().copied().filter(|i| !gone.contains(i)).collect();
            moved.retain(|&(i, s)| s > top(&faces[i].vector, &rest) + 0.03);
            let mut pictures = in_t;
            for (i, _) in moved {
                // one person is in a picture once
                if pictures.insert(faces[i].id.0) {
                    out.of.insert(faces[i].id, t);
                    mine.retain(|&m| m != i);
                }
            }
        }
    }
    out
}

/// Pairs of groups that may be one person (smaller group first), closest first: the exemplars
/// of one score at least `Params::propose` in the other, typically (the median). Pairs the user refused are left out.
pub fn proposals(faces: &[Face], groups: &Groups, c: &Constraints, p: &Params, min_faces: usize) -> Vec<(GroupId, GroupId, f32)> {
    let index: HashMap<FaceId, usize> = faces.iter().enumerate().map(|(i, f)| (f.id, i)).collect();
    let dim = faces.first().map(|f| f.vector.len()).unwrap_or(0);
    let mut built: Vec<(Group, usize)> = vec![];
    let mut members: Vec<(GroupId, Vec<FaceId>)> = groups.members().into_iter().filter(|(_, m)| m.len() >= min_faces).collect();
    members.sort_unstable_by_key(|(g, _)| *g);
    for (g, list) in members {
        let mut grp = Group::new(g, dim);
        let mut idx: Vec<usize> = list.iter().filter_map(|f| index.get(f).copied()).collect();
        idx.sort_by_key(|&i| (!faces[i].good, i));
        for i in idx {
            grp.offer(i, faces, p);
        }
        built.push((grp, list.len()));
    }
    let mut out = vec![];
    for (x, (a, na)) in built.iter().enumerate() {
        for (b, nb) in &built[x + 1..] {
            let key = (a.id.min(b.id), a.id.max(b.id));
            if c.refused.contains(&key) {
                continue;
            }
            // how a group's exemplars score in the other, typically (the median: the best of 50
            // tries finds a look-alike between any two groups), the closer of the two ways
            let median = |from: &Group, to: &Group| {
                let mut v: Vec<f32> = from.exemplars.iter().map(|&i| to.score(&faces[i].vector, faces, p.top)).collect();
                v.sort_by(f32::total_cmp);
                v.get(v.len() / 2).copied().unwrap_or(f32::NEG_INFINITY)
            };
            let s = median(a, b).max(median(b, a));
            if s >= p.propose {
                let (small, large) = if na <= nb { (a.id, b.id) } else { (b.id, a.id) };
                out.push((small, large, s));
            }
        }
    }
    out.sort_by(|x, y| y.2.total_cmp(&x.2).then(x.0.cmp(&y.0)));
    out
}

/// Names proposed for unnamed groups, from the names typed on their pictures (`Meta.people`):
/// a group whose pictures mostly (`share`, at least `min_pictures`) carry one name gets it,
/// unless another group has more of that name's pictures or a group is that person already
/// (`named`). Names compare as tags do (`tags::key`).
pub fn name_suggestions(groups: &Groups, names: &HashMap<u128, Vec<String>>, named: &HashMap<GroupId, String>, share: f32, min_pictures: usize) -> Vec<(GroupId, String)> {
    let taken: HashSet<String> = named.values().map(|n| crate::tags::key(n)).collect();
    let mut pictures: HashMap<GroupId, HashSet<u128>> = HashMap::new();
    for ((pic, _), g) in &groups.of {
        if !named.contains_key(g) {
            pictures.entry(*g).or_default().insert(*pic);
        }
    }
    // the best name of each group, with its count
    let mut best: Vec<(GroupId, String, usize)> = vec![];
    for (g, pics) in &pictures {
        let mut count: HashMap<String, (usize, &String)> = HashMap::new();
        for pic in pics {
            for n in names.get(pic).into_iter().flatten() {
                let e = count.entry(crate::tags::key(n)).or_insert((0, n));
                e.0 += 1;
            }
        }
        if let Some((key, (n, name))) = count.into_iter().max_by(|a, b| a.1.0.cmp(&b.1.0).then(b.0.cmp(&a.0)))
            && n >= min_pictures
            && n as f32 >= share * pics.len() as f32
            && !taken.contains(&key)
        {
            best.push((*g, name.clone(), n));
        }
    }
    // one group per name: the one with most pictures of it
    best.sort_by(|a, b| b.2.cmp(&a.2).then(a.0.cmp(&b.0)));
    let mut used = HashSet::new();
    let mut out: Vec<(GroupId, String)> = best.into_iter().filter(|(_, n, _)| used.insert(crate::tags::key(n))).map(|(g, n, _)| (g, n)).collect();
    out.sort_by_key(|(g, _)| *g);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A unit vector near the axis `person` (people are far apart), moved a little by `wobble`.
    fn v(person: usize, wobble: f32, seed: u32) -> Vec<f32> {
        let mut x: Vec<f32> = (0..16).map(|i| if i == person { 1.0 } else { 0.0 }).collect();
        let mut s = seed.wrapping_mul(2654435761).wrapping_add(person as u32 * 97);
        for e in x.iter_mut() {
            s = s.wrapping_mul(1103515245).wrapping_add(12345);
            *e += wobble * (((s >> 8) & 0xffff) as f32 / 65535.0 - 0.5);
        }
        crate::ai::search::normalise(&mut x);
        x
    }

    fn face(pic: u128, n: u16, vector: Vec<f32>, good: bool) -> Face {
        Face { id: (pic, n), vector, good }
    }

    /// `n` faces of each person, one picture each, in turn.
    fn people(persons: &[usize], n: usize, wobble: f32) -> Vec<Face> {
        let mut out = vec![];
        for k in 0..n {
            for &p in persons {
                out.push(face((p * 1000 + k) as u128, 0, v(p, wobble, k as u32), true));
            }
        }
        out
    }

    fn sizes(g: &Groups) -> Vec<usize> {
        let mut s: Vec<usize> = g.members().values().map(Vec::len).collect();
        s.sort_unstable();
        s
    }

    #[test]
    fn people_are_grouped_and_kept_apart() {
        let faces = people(&[0, 1, 2], 10, 0.6);
        let g = regroup(&faces, &Groups::default(), &Constraints::default(), &Params::default());
        assert_eq!(sizes(&g), vec![10, 10, 10]);
        // each group is one person
        for m in g.members().values() {
            let first = m[0].0 / 1000;
            assert!(m.iter().all(|f| f.0 / 1000 == first));
        }
        assert_eq!(g.next, 3);
        // the same again: nothing changes
        assert_eq!(regroup(&faces, &g, &Constraints::default(), &Params::default()), g);
    }

    #[test]
    fn two_faces_alone_make_no_group_and_poor_faces_only_join() {
        // two faces of person 0 and two of person 1: not enough for a group
        let faces = people(&[0, 1], 2, 0.3);
        let g = regroup(&faces, &Groups::default(), &Constraints::default(), &Params::default());
        assert!(g.of.is_empty());
        // poor faces never start one, even many of them
        let poor: Vec<Face> = people(&[0], 10, 0.3).into_iter().map(|f| Face { good: false, ..f }).collect();
        assert!(regroup(&poor, &Groups::default(), &Constraints::default(), &Params::default()).of.is_empty());
        // but they join a group that exists, also one made after them
        let mut faces = poor.clone();
        faces.extend(people(&[0], 3, 0.3).into_iter().map(|f| Face { id: (f.id.0 + 500, 0), ..f }));
        let g = regroup(&faces, &Groups::default(), &Constraints::default(), &Params::default());
        assert_eq!(sizes(&g), vec![13]);
    }

    #[test]
    fn look_alikes_and_faces_like_both_do_not_mix_two_people() {
        // two people who look somewhat alike (cosine 0.3), and a few faces that look like both
        // (0.5 to each): a blurred face, a profile
        let mut a = vec![0.0; 16];
        a[0] = 1.0;
        let mut b = vec![0.0; 16];
        b[0] = 0.3;
        b[1] = (1.0f32 - 0.09).sqrt();
        // 0.385 (a + b) has a cosine of 0.5 with each; the rest of its length elsewhere
        let mut both: Vec<f32> = a.iter().zip(&b).map(|(x, y)| 0.385 * (x + y)).collect();
        both[2] = (1.0f32 - 0.385 * 0.385 * 2.6).sqrt();
        let around = |base: &Vec<f32>, k: u32| {
            let mut x = base.clone();
            x[4 + (k as usize % 10)] += 0.25;
            crate::ai::search::normalise(&mut x);
            x
        };
        let mut faces = vec![];
        for k in 0..8u32 {
            faces.push(face(k as u128, 0, around(&a, k), true));
        }
        for k in 0..3u32 {
            faces.push(face(500 + k as u128, 0, around(&both, k), true));
        }
        for k in 0..8u32 {
            faces.push(face(1000 + k as u128, 0, around(&b, k), true));
        }
        let g = regroup(&faces, &Groups::default(), &Constraints::default(), &Params::default());
        for m in g.members().values() {
            let has_a = m.iter().any(|f| f.0 < 500);
            let has_b = m.iter().any(|f| f.0 >= 1000);
            assert!(!(has_a && has_b), "a group with both people: {m:?}");
        }
        assert!(g.members().values().any(|m| m.iter().filter(|f| f.0 < 500).count() == 8));
        assert!(g.members().values().any(|m| m.iter().filter(|f| f.0 >= 1000).count() == 8));
    }

    #[test]
    fn a_group_of_two_look_alikes_is_split_by_moving_a_few_faces() {
        // person A and a brother B who looks a little like A (cosine 0.35), all in one group
        let mut a = vec![0.0; 16];
        a[0] = 1.0;
        let mut b = vec![0.0; 16];
        b[0] = 0.35;
        b[1] = (1.0f32 - 0.35 * 0.35).sqrt();
        let around = |base: &Vec<f32>, k: u32| {
            let mut x = base.clone();
            x[4 + (k as usize % 10)] += 0.25;
            crate::ai::search::normalise(&mut x);
            x
        };
        let mut faces = vec![];
        for k in 0..8u32 {
            faces.push(Face { id: (k as u128, 0), vector: around(&a, k), good: true });
            faces.push(Face { id: (100 + k as u128, 0), vector: around(&b, k), good: true });
        }
        let all_in_one = Groups { of: faces.iter().map(|f| (f.id, 0)).collect(), next: 1 };
        // the user moved two of B's faces to a group of their own
        let c = Constraints { placed: HashMap::from([((100, 0), 5), ((101, 0), 5)]), ..Default::default() };
        let mut start = all_in_one.clone();
        start.of.insert((100, 0), 5);
        start.of.insert((101, 0), 5);
        let after = recheck(&faces, &start, &c, &Params::default(), &HashSet::from([0]));
        for k in 0..8u128 {
            assert_eq!(after.of[&(k, 0)], 0, "A's face {k} stays");
            assert_eq!(after.of[&(100 + k, 0)], 5, "B's face {k} follows");
        }
        // nothing happens for a group not asked about
        assert_eq!(recheck(&faces, &start, &c, &Params::default(), &HashSet::new()), start);
    }

    #[test]
    fn groups_are_not_reshuffled_and_new_people_appear() {
        let p = Params::default();
        let first = people(&[0, 1], 5, 0.5);
        let g1 = regroup(&first, &Groups::default(), &Constraints::default(), &p);
        // later pictures: more of both, and a new person
        let mut all = first.clone();
        all.extend(people(&[0, 1, 2], 5, 0.5).into_iter().map(|f| Face { id: (f.id.0 + 100, 0), ..f }));
        let g2 = regroup(&all, &g1, &Constraints::default(), &p);
        for (f, g) in &g1.of {
            assert_eq!(g2.of.get(f), Some(g), "a face kept its group");
        }
        assert_eq!(sizes(&g2), vec![5, 10, 10]);
        assert_eq!(g2.next, 3);
        // a group the old faces no longer reach, e.g. forced into an odd group by hand earlier,
        // stays as it is: only faces in no group are placed
        let mut odd = g2.clone();
        let moved = first[0].id;
        odd.of.insert(moved, 99);
        odd.next = 100;
        assert_eq!(regroup(&all, &odd, &Constraints::default(), &p).of[&moved], 99);
        // faces that are gone are dropped
        let fewer: Vec<Face> = all.iter().skip(2).cloned().collect();
        let g3 = regroup(&fewer, &g2, &Constraints::default(), &p);
        assert!(!g3.of.contains_key(&all[0].id) && !g3.of.contains_key(&all[1].id));
    }

    #[test]
    fn corrections_are_kept() {
        let p = Params::default();
        let faces = people(&[0, 1], 6, 0.5);
        let g = regroup(&faces, &Groups::default(), &Constraints::default(), &p);
        let group_of = |person: usize, g: &Groups| g.of[&faces.iter().find(|f| f.id.0 as usize / 1000 == person).unwrap().id];
        let (g0, g1) = (group_of(0, &g), group_of(1, &g));
        // a face of person 0 moved to person 1's group by the user; another said "not this
        // person"; a third ignored
        let (moved, not, ignored) = (faces[0].id, faces[2].id, faces[4].id);
        let c = Constraints {
            placed: HashMap::from([(moved, g1)]),
            not_in: HashMap::from([(not, vec![g0])]),
            left_out: HashSet::from([ignored]),
            refused: HashSet::new(),
            merged: HashMap::new(),
        };
        let mut fresh = g.clone();
        fresh.of.remove(&not);
        let after = regroup(&faces, &fresh, &c, &p);
        assert_eq!(after.of[&moved], g1);
        assert_ne!(after.of.get(&not), Some(&g0), "never put back where the user said it isn't");
        assert!(!after.of.contains_key(&ignored));
        // even from scratch
        let scratch = regroup(&faces, &Groups::default(), &c, &p);
        assert_eq!(scratch.of[&moved], g1);
        assert!(!scratch.of.contains_key(&ignored));
        assert!(scratch.next > g1);
    }

    #[test]
    fn two_faces_of_one_picture_are_two_people() {
        let p = Params::default();
        // a picture of twins (alike faces), and more pictures of one of them
        let mut faces = vec![face(1, 0, v(0, 0.2, 1), true), face(1, 1, v(0, 0.2, 2), true)];
        faces.extend((2..8u128).map(|k| face(k, 0, v(0, 0.2, k as u32 + 10), true)));
        let g = regroup(&faces, &Groups::default(), &Constraints::default(), &p);
        let a = g.of.get(&(1, 0));
        let b = g.of.get(&(1, 1));
        assert!(a.is_some() || b.is_some());
        assert_ne!(a, b, "the two faces of picture 1 are not in one group");
        // a group made before the rule loses one of them
        let before = Groups { of: faces.iter().map(|f| (f.id, 4)).collect(), next: 5 };
        let after = regroup(&faces, &before, &Constraints::default(), &p);
        assert_eq!(after.of.values().filter(|&&x| x == 4).count(), 7);
        // but not one the user put there
        let c = Constraints { placed: HashMap::from([((1, 0), 4), ((1, 1), 4)]), ..Default::default() };
        let placed = regroup(&faces, &before, &c, &p);
        assert_eq!((placed.of[&(1, 0)], placed.of[&(1, 1)]), (4, 4));
    }

    #[test]
    fn merged_groups_stay_one() {
        let p = Params::default();
        let faces = people(&[0, 1], 5, 0.4);
        let g = regroup(&faces, &Groups::default(), &Constraints::default(), &p);
        let ids: Vec<GroupId> = {
            let mut v: Vec<GroupId> = g.members().keys().copied().collect();
            v.sort_unstable();
            v
        };
        // the user says both are one (a chain of merges, and one that loops back)
        let c = Constraints { merged: HashMap::from([(ids[1], ids[0]), (99, ids[1])]), ..Default::default() };
        let after = regroup(&faces, &g, &c, &p);
        assert_eq!(sizes(&after), vec![10]);
        assert_eq!(c.resolve(99), ids[0]);
        let looped = Constraints { merged: HashMap::from([(1, 2), (2, 1)]), ..Default::default() };
        assert!([1, 2].contains(&looped.resolve(1)));
    }

    #[test]
    fn close_groups_are_proposed_unless_refused() {
        let p = Params::default();
        // person 0 seen two ways: the two groups are close but below `join`
        let mut faces = vec![];
        let a: Vec<f32> = {
            let mut x = vec![0.0; 16];
            x[0] = 1.0;
            x
        };
        let b: Vec<f32> = {
            let mut x = vec![0.0; 16];
            x[0] = 0.42;
            x[1] = (1.0f32 - 0.42 * 0.42).sqrt();
            x
        };
        for k in 0..4u32 {
            for (n, base) in [(0u128, &a), (1, &b)] {
                let mut x = base.clone();
                x[2 + (k as usize % 3)] = 0.05;
                crate::ai::search::normalise(&mut x);
                faces.push(face(n * 100 + k as u128, 0, x, true));
            }
        }
        faces.extend(people(&[9], 4, 0.3));
        let g = regroup(&faces, &Groups::default(), &Constraints::default(), &p);
        assert_eq!(sizes(&g), vec![4, 4, 4]);
        let props = proposals(&faces, &g, &Constraints::default(), &p, 3);
        assert_eq!(props.len(), 1, "{props:?}");
        let (x, y, s) = props[0];
        assert!(s >= p.propose && s < p.join);
        let refused = Constraints { refused: HashSet::from([(x.min(y), x.max(y))]), ..Default::default() };
        assert!(proposals(&faces, &g, &refused, &p, 3).is_empty());
    }

    #[test]
    fn typed_names_are_proposed_for_groups() {
        let p = Params::default();
        let faces = people(&[0, 1], 5, 0.4);
        let g = regroup(&faces, &Groups::default(), &Constraints::default(), &p);
        let group_of = |person: usize| g.of[&faces.iter().find(|f| f.id.0 as usize / 1000 == person).unwrap().id];
        let mut names: HashMap<u128, Vec<String>> = HashMap::new();
        // four of person 0's five pictures say Anna; person 1's say Marc only twice
        for k in 0..4 {
            names.insert(k as u128, vec!["Anna".into()]);
        }
        names.insert(1000, vec!["Marc".into()]);
        names.insert(1001, vec!["marc".into(), "Anna".into()]);
        let s = name_suggestions(&g, &names, &HashMap::new(), 0.6, 3);
        assert_eq!(s, vec![(group_of(0), "Anna".to_string())]);
        // a group that is Anna already: nobody else is proposed as Anna
        let named = HashMap::from([(group_of(1), "anna".to_string())]);
        assert!(name_suggestions(&g, &names, &named, 0.6, 3).is_empty());
    }
}
