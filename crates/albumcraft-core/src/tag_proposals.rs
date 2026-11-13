//! Tag proposals: up to five tags proposed for a picture or a selection, from the vectors smart
//! search already made (no other network). Three sources, merged into one ranked list:
//!
//! 1. **The user's own tags** on the nearest tagged pictures, weighted by how near they are,
//!    with more weight for the same folder (usually one event) and the same place.
//! 2. **A classifier per tag** with at least `MIN_EXAMPLES` pictures: the direction from the
//!    other tagged pictures to that tag's pictures, and a logistic curve fitted along it.
//!    Trained here in a few milliseconds, kept until that tag's pictures change.
//! 3. **The built-in vocabulary** (`vocabulary::WORDS`), zero-shot: each word's sentence goes
//!    through the pack's text encoder once (in the background, a minute or so; kept in the
//!    analysis folder), then costs one product per picture.
//!
//! Nearness is judged against each picture's own scores over the library (how many standard
//! deviations above its average, from a sample of the library), so one set of limits holds
//! for every network. "Not this tag" (`Meta.not_tags`) removes a tag from that picture and
//! lowers it for the pictures near it. Nothing is ever added without the user's click.

use crate::ai::models::Pack;
use crate::ai::search::{self, Model};
use crate::geo::{FileStamp, LatLon};
use crate::smart_search::{self, Matrix};
use crate::vocabulary::WORDS;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock, Mutex, PoisonError};

/// Tags proposed at most.
pub const MAX_PROPOSALS: usize = 5;
/// Proposals below this (0–1) are not shown.
const SHOWN_FROM: f32 = 0.35;
/// A tagged picture this many standard deviations above the picture's average is near it.
const NEAR_Z: f32 = 2.0;
/// Nearest tagged pictures looked at.
const NEIGHBOURS: usize = 30;
/// Pictures a tag needs before it gets its own classifier.
const MIN_EXAMPLES: usize = 15;
/// Pictures of the library the statistics are taken from.
const SAMPLE: usize = 2000;
/// Pictures of a selection looked at (spread over it).
const MAX_TARGETS: usize = 40;
/// How far apart two places can be and still count as the same (km).
const SAME_PLACE_KM: f64 = 0.3;
/// Built-in words count for less than the user's own tags.
const VOCABULARY_WEIGHT: f32 = 0.8;

/// A picture as tag proposals see it: its tags, the tags the user said it does not show, and
/// where it was taken.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Picture {
    pub key: String,
    pub tags: Vec<String>,
    pub not_tags: Vec<String>,
    pub place: Option<LatLon>,
}

/// The library's pictures with tags or "not this tag", and a number that changes whenever
/// they change (what is learnt from them is kept until then).
#[derive(Clone, Debug, Default)]
pub struct Examples {
    pub rev: u64,
    pub pictures: Vec<Picture>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Proposal {
    pub tag: String,
    /// 0–1: how sure; only the order and the cut mean something.
    pub score: f32,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Proposals {
    pub tags: Vec<Proposal>,
    /// The built-in vocabulary is still being prepared: asking again later can give more.
    pub partial: bool,
    /// None of the pictures asked about is analysed yet.
    pub not_analysed: bool,
}

/// The pictures of `store` with tags or "not this tag", as examples. `place`: where a
/// picture was taken (its notes' place, or the file's GPS when known).
pub fn examples_of(store: &crate::library::Store, rev: u64, place: impl Fn(&str, &crate::library::Meta) -> Option<LatLon>) -> Examples {
    let pictures = store
        .meta
        .iter()
        .filter(|(k, m)| !k.starts_with(crate::imaging::SAMPLE_PREFIX) && (!m.tags.is_empty() || !m.not_tags.is_empty()))
        .map(|(k, m)| Picture { key: k.clone(), tags: m.tags.clone(), not_tags: m.not_tags.clone(), place: place(k, m) })
        .collect();
    Examples { rev, pictures }
}

/// One picture of `store` as tag proposals see it.
pub fn picture_of(store: &crate::library::Store, key: &str, place: impl Fn(&str, &crate::library::Meta) -> Option<LatLon>) -> Picture {
    match store.meta.get(key) {
        Some(m) => Picture { key: key.to_string(), tags: m.tags.clone(), not_tags: m.not_tags.clone(), place: place(key, m) },
        None => Picture { key: key.to_string(), ..Picture::default() },
    }
}

/// Propose tags for `targets` (one picture, or a selection: then the tags most of them would
/// get), with the `active` search pack's vectors of `files` kept in `dir`.
pub fn propose(active: &Pack, files: &Arc<Vec<FileStamp>>, dir: &Path, examples: &Arc<Examples>, targets: &[Picture]) -> Result<Proposals, String> {
    let (engine, matrix) = smart_search::source(active, files, dir)?;
    let model = engine.model.clone();
    let state = state_of(&model.pack_id);
    let set = state.examples(&matrix, examples);
    let classifiers = state.classifiers(&set, &matrix);
    let vocabulary = state.vocabulary(&model, dir, &matrix);
    let sample = matrix.sample(SAMPLE);
    let context = Context { set: &set, classifiers: &classifiers, vocabulary: vocabulary.as_deref(), floor: model.config.min_score, margin: model.config.text_margin, sample: &sample };
    let mut out = merge(spread(targets, MAX_TARGETS).iter().map(|t| (*t, matrix.vector(&t.key))), &context);
    out.partial = vocabulary.is_none() && state.vocabulary_pending();
    Ok(out)
}

/// At most `n` of `all`, spread over it.
fn spread<T>(all: &[T], n: usize) -> Vec<&T> {
    let step = all.len().div_ceil(n.max(1)).max(1);
    all.iter().step_by(step).collect()
}

/// What the scores of one picture are compared with.
struct Context<'a> {
    set: &'a ExampleSet,
    classifiers: &'a [(String, String, Arc<Classifier>)],
    vocabulary: Option<&'a VocabularyStats>,
    /// The pack's score floor and margin for text (`ai::search::Config`).
    floor: f32,
    margin: f32,
    sample: &'a [&'a [u8]],
}

/// The tags of each target, averaged over the targets (a tag a target has counts as 1),
/// without those every target has, best first.
fn merge<'a>(targets: impl Iterator<Item = (&'a Picture, Option<Vec<f32>>)>, cx: &Context) -> Proposals {
    // lower-case tag → (shown name, sum of scores, targets that have it)
    let mut sum: HashMap<String, (String, f32, usize)> = HashMap::new();
    let mut counted = 0usize;
    for (target, vector) in targets {
        let Some(v) = vector else { continue };
        counted += 1;
        for (lower, name, p) in scores_of(&v, target, cx) {
            let e = sum.entry(lower).or_insert((name, 0.0, 0));
            e.1 += p;
        }
        for tag in &target.tags {
            let e = sum.entry(tag.to_lowercase()).or_insert((tag.clone(), 0.0, 0));
            e.2 += 1;
        }
    }
    if counted == 0 {
        return Proposals { not_analysed: true, ..Proposals::default() };
    }
    let mut tags: Vec<Proposal> = sum
        .into_values()
        .filter(|(_, _, has)| *has < counted)
        .map(|(tag, s, has)| Proposal { tag, score: (s + has as f32) / counted as f32 })
        .filter(|p| p.score >= SHOWN_FROM)
        .collect();
    tags.sort_by(|a, b| b.score.total_cmp(&a.score).then_with(|| a.tag.cmp(&b.tag)));
    tags.truncate(MAX_PROPOSALS);
    Proposals { tags, ..Proposals::default() }
}

/// Every tag worth proposing for the picture `v` of `target`: (lower case, name, 0–1), not
/// the tags it has or was said not to show.
fn scores_of(v: &[f32], target: &Picture, cx: &Context) -> Vec<(String, String, f32)> {
    let (mean, sd) = stats(cx.sample.iter().map(|r| search::score(v, r)));
    let z = |s: f32| (s - mean) / sd;
    let folder = folder_of(&target.key);
    let mut best: HashMap<String, (String, f32)> = HashMap::new();
    let mut put = |lower: String, name: &str, p: f32| {
        let e = best.entry(lower).or_insert_with(|| (name.to_string(), 0.0));
        if p > e.1 {
            e.1 = p;
        }
    };

    // 1. the tags of the nearest tagged pictures; in the same folder or at the same place (one
    //    event, usually), also those that only look somewhat alike
    let same_place = |e: &Example| matches!((e.place, target.place), (Some(a), Some(b)) if a.km_to(b) <= SAME_PLACE_KM);
    let mut near: Vec<(f32, &Example)> = cx
        .set
        .items
        .iter()
        .filter(|e| e.key != target.key)
        .map(|e| (z(search::score(v, &e.record)), e))
        .filter(|(z, e)| *z >= NEAR_Z || (*z >= 0.0 && (e.folder == folder || same_place(e))))
        .collect();
    near.sort_by(|a, b| b.0.total_cmp(&a.0));
    near.truncate(NEIGHBOURS);
    let (mut total, mut has, mut against) = (0f32, HashMap::<&str, (&str, f32)>::new(), HashMap::<&str, f32>::new());
    for (zi, e) in &near {
        let mut w = if *zi >= NEAR_Z { zi - NEAR_Z + 0.25 } else { 0.25 * zi / NEAR_Z };
        if e.folder == folder {
            w *= 2.0;
        }
        if same_place(e) {
            w *= 1.5;
        }
        total += w;
        for (lower, name) in &e.tags {
            has.entry(lower).or_insert((name, 0.0)).1 += w;
        }
        for lower in &e.not_tags {
            *against.entry(lower).or_default() += w;
        }
    }
    for (lower, (name, w)) in &has {
        put(lower.to_string(), name, w / (total + 1.0));
    }

    // 2. the classifiers of the tags with enough pictures
    for (lower, name, c) in cx.classifiers {
        put(lower.clone(), name, 0.9 * c.p(v));
    }

    // 3. the built-in words: those close to the picture's best word, above the pack's floor,
    //    surer as the score is higher, a little less for words most of the library is near
    if let Some(vocabulary) = cx.vocabulary {
        let scores: Vec<f32> = vocabulary.vectors.iter().map(|w| search::dot(v, w)).collect();
        let top = scores.iter().copied().fold(f32::NEG_INFINITY, f32::max);
        let (floor, margin) = (cx.floor, cx.margin.max(1e-3));
        for (i, &s) in scores.iter().enumerate() {
            if s < floor || top - s > 2.0 * margin {
                continue;
            }
            let (Some(&(mean, sd)), Some(name)) = (vocabulary.stats.get(i), vocabulary.names.get(i)) else { continue };
            let sure = ((s - floor) / (2.0 * margin)).clamp(0.0, 1.0);
            let close = 1.0 - (top - s) / (2.0 * margin);
            let rare = (((s - mean) / sd + 1.0) / 3.0).clamp(0.0, 1.0);
            let lower = name.to_lowercase();
            // the user's spelling of the same tag, if they use it
            let shown = cx.set.names.get(&lower).cloned().unwrap_or_else(|| name.clone());
            put(lower, &shown, VOCABULARY_WEIGHT * sure * close * rare);
        }
    }

    let own: Vec<String> = target.tags.iter().chain(&target.not_tags).map(|t| t.to_lowercase()).collect();
    best.into_iter()
        .filter(|(lower, _)| !own.contains(lower))
        .map(|(lower, (name, p))| {
            let against = against.get(lower.as_str()).copied().unwrap_or(0.0);
            (lower, name, p / (1.0 + against))
        })
        .collect()
}

fn sigmoid(x: f32) -> f32 {
    1.0 / (1.0 + (-x).exp())
}

/// Mean and standard deviation (never 0).
fn stats(values: impl Iterator<Item = f32>) -> (f32, f32) {
    let (mut n, mut sum, mut sq) = (0f64, 0f64, 0f64);
    for x in values.filter(|x| x.is_finite()) {
        n += 1.0;
        sum += x as f64;
        sq += (x as f64) * (x as f64);
    }
    if n < 2.0 {
        return (0.0, 1.0);
    }
    let mean = sum / n;
    let var = (sq / n - mean * mean).max(0.0);
    (mean as f32, (var.sqrt() as f32).max(1e-4))
}

fn folder_of(key: &str) -> PathBuf {
    Path::new(key).parent().map(Path::to_path_buf).unwrap_or_default()
}

/// A tagged picture with its vector.
struct Example {
    key: String,
    folder: PathBuf,
    place: Option<LatLon>,
    /// (lower case, as written)
    tags: Vec<(String, String)>,
    not_tags: Vec<String>,
    record: Vec<u8>,
}

/// The examples that have vectors, for one matrix and one `Examples::rev`.
struct ExampleSet {
    made_from: ((usize, u64), u64),
    items: Vec<Example>,
    /// Every tag the user wrote, by its lower case.
    names: HashMap<String, String>,
}

impl ExampleSet {
    fn of(matrix: &Matrix, examples: &Examples) -> ExampleSet {
        let mut names = HashMap::new();
        let items = examples
            .pictures
            .iter()
            .filter_map(|p| {
                for t in &p.tags {
                    names.entry(t.to_lowercase()).or_insert_with(|| t.clone());
                }
                let v = matrix.vector(&p.key)?;
                Some(Example {
                    key: p.key.clone(),
                    folder: folder_of(&p.key),
                    place: p.place,
                    tags: p.tags.iter().map(|t| (t.to_lowercase(), t.clone())).collect(),
                    not_tags: p.not_tags.iter().map(|t| t.to_lowercase()).collect(),
                    record: search::quantise(&v),
                })
            })
            .collect();
        ExampleSet { made_from: (matrix.identity(), examples.rev), items, names }
    }
}

/// A linear classifier: `p = sigmoid(scale * (w · v) + bias)`.
#[derive(Debug)]
struct Classifier {
    w: Vec<f32>,
    scale: f32,
    bias: f32,
}

impl Classifier {
    fn p(&self, v: &[f32]) -> f32 {
        sigmoid(self.scale * search::dot(&self.w, v) + self.bias)
    }

    /// From the vectors of a tag's pictures and of others (with a weight each: a "not this
    /// tag" counts more). None when there is too little to learn from.
    fn train(pos: &[Vec<f32>], neg: &[(Vec<f32>, f32)]) -> Option<Classifier> {
        let dim = pos.first()?.len();
        if pos.len() < MIN_EXAMPLES || neg.is_empty() || dim == 0 {
            return None;
        }
        let mut w = mean(pos.iter().map(|v| (v.as_slice(), 1.0)), dim);
        let other = mean(neg.iter().map(|(v, k)| (v.as_slice(), *k)), dim);
        for (a, b) in w.iter_mut().zip(&other) {
            *a -= b;
        }
        search::normalise(&mut w);
        if w.iter().all(|x| *x == 0.0) {
            return None;
        }
        // one dimension: fit p = sigmoid(a t + c) by Newton's method, both classes weighted
        // equally, a little ridge so separable data stays finite
        let pw = 1.0 / pos.len() as f64;
        let nw = 1.0 / neg.iter().map(|(_, k)| *k as f64).sum::<f64>().max(1e-9);
        let points: Vec<(f64, f64, f64)> = pos
            .iter()
            .map(|v| (search::dot(&w, v) as f64, 1.0, pw))
            .chain(neg.iter().map(|(v, k)| (search::dot(&w, v) as f64, 0.0, nw * *k as f64)))
            .collect();
        let (mut a, mut c) = (1.0f64, 0.0f64);
        for _ in 0..30 {
            let (mut ga, mut gc, mut haa, mut hac, mut hcc) = (1e-3 * a, 0.0, 1e-3, 0.0, 1e-9);
            for &(t, y, k) in &points {
                let p = 1.0 / (1.0 + (-(a * t + c)).exp());
                let (g, h) = (k * (p - y), k * p * (1.0 - p));
                ga += g * t;
                gc += g;
                haa += h * t * t;
                hac += h * t;
                hcc += h;
            }
            let det = haa * hcc - hac * hac;
            if !det.is_finite() || det.abs() < 1e-18 {
                break;
            }
            let (da, dc) = ((hcc * ga - hac * gc) / det, (haa * gc - hac * ga) / det);
            a -= da;
            c -= dc;
            if !a.is_finite() || !c.is_finite() {
                return None;
            }
            if da.abs() + dc.abs() < 1e-6 {
                break;
            }
        }
        // a direction that does not separate the tag's pictures is no classifier
        (a > 0.0).then_some(Classifier { w, scale: a as f32, bias: c as f32 })
    }
}

fn mean<'a>(vs: impl Iterator<Item = (&'a [f32], f32)>, dim: usize) -> Vec<f32> {
    let mut m = vec![0f32; dim];
    let mut total = 0f32;
    for (v, k) in vs {
        for (a, b) in m.iter_mut().zip(v) {
            *a += k * b;
        }
        total += k;
    }
    if total > 0.0 {
        for a in &mut m {
            *a /= total;
        }
    }
    m
}

/// The built-in words' vectors with each word's mean and spread over the library.
struct VocabularyStats {
    made_from: (usize, u64),
    names: Vec<String>,
    vectors: Arc<Vec<Vec<f32>>>,
    stats: Vec<(f32, f32)>,
}

enum Vocabulary {
    Missing,
    Computing,
    Ready(Arc<Vec<Vec<f32>>>),
    /// The text encoder could not be used: no built-in words for this pack until the next start.
    Failed,
}

/// A tag's classifier (None: its pictures could not be told apart) and what it was trained
/// from (`signature`), by the tag's lower case.
type Trained = (u64, Option<Arc<Classifier>>);

/// What is learnt for one search pack.
struct PackState {
    examples: Mutex<Option<Arc<ExampleSet>>>,
    classifiers: Mutex<HashMap<String, Trained>>,
    vocabulary: Mutex<Vocabulary>,
    vocabulary_stats: Mutex<Option<Arc<VocabularyStats>>>,
}

static STATES: LazyLock<Mutex<HashMap<String, Arc<PackState>>>> = LazyLock::new(Default::default);

fn state_of(pack: &str) -> Arc<PackState> {
    let mut all = STATES.lock().unwrap_or_else(PoisonError::into_inner);
    all.entry(pack.to_string())
        .or_insert_with(|| {
            Arc::new(PackState {
                examples: Mutex::new(None),
                classifiers: Mutex::new(HashMap::new()),
                vocabulary: Mutex::new(Vocabulary::Missing),
                vocabulary_stats: Mutex::new(None),
            })
        })
        .clone()
}

impl PackState {
    fn examples(&self, matrix: &Matrix, examples: &Examples) -> Arc<ExampleSet> {
        let mut cached = self.examples.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(set) = cached.as_ref()
            && set.made_from == (matrix.identity(), examples.rev)
        {
            return set.clone();
        }
        let set = Arc::new(ExampleSet::of(matrix, examples));
        *cached = Some(set.clone());
        set
    }

    /// The classifier of every tag with enough pictures; a tag's is trained again only when
    /// its pictures (or those said not to show it) changed, or the examples grew by a lot.
    fn classifiers(&self, set: &ExampleSet, matrix: &Matrix) -> Vec<(String, String, Arc<Classifier>)> {
        let mut by_tag: HashMap<&str, (Vec<usize>, Vec<usize>)> = HashMap::new();
        for (i, e) in set.items.iter().enumerate() {
            for (lower, _) in &e.tags {
                by_tag.entry(lower).or_default().0.push(i);
            }
            for lower in &e.not_tags {
                by_tag.entry(lower).or_default().1.push(i);
            }
        }
        let mut cache = self.classifiers.lock().unwrap_or_else(PoisonError::into_inner);
        let mut out = vec![];
        for (lower, (pos, not)) in by_tag {
            if pos.len() < MIN_EXAMPLES {
                continue;
            }
            let signature = signature(&pos, &not, set);
            let fresh = cache.get(lower).filter(|(s, _)| *s == signature).map(|(_, c)| c.clone());
            let c = match fresh {
                Some(c) => c,
                None => {
                    let c = train_tag(&pos, &not, set, matrix).map(Arc::new);
                    cache.insert(lower.to_string(), (signature, c.clone()));
                    c
                }
            };
            if let Some(c) = c {
                let name = set.names.get(lower).cloned().unwrap_or_else(|| lower.to_string());
                out.push((lower.to_string(), name, c));
            }
        }
        out
    }

    fn vocabulary_pending(&self) -> bool {
        matches!(*self.vocabulary.lock().unwrap_or_else(PoisonError::into_inner), Vocabulary::Missing | Vocabulary::Computing)
    }

    /// The built-in words with their statistics for this matrix; None while their vectors are
    /// being made (started here, once, on a thread of its own).
    fn vocabulary(&self, model: &Arc<Model>, dir: &Path, matrix: &Matrix) -> Option<Arc<VocabularyStats>> {
        let vectors = {
            let mut v = self.vocabulary.lock().unwrap_or_else(PoisonError::into_inner);
            match &*v {
                Vocabulary::Ready(vectors) => vectors.clone(),
                Vocabulary::Computing | Vocabulary::Failed => return None,
                Vocabulary::Missing => {
                    if let Some(vectors) = read_vocabulary(dir, model) {
                        let vectors = Arc::new(vectors);
                        *v = Vocabulary::Ready(vectors.clone());
                        vectors
                    } else {
                        *v = Vocabulary::Computing;
                        start_vocabulary(state_of(&model.pack_id), model.clone(), dir.to_path_buf());
                        return None;
                    }
                }
            }
        };
        let mut cached = self.vocabulary_stats.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(s) = cached.as_ref()
            && s.made_from == matrix.identity()
            && Arc::ptr_eq(&s.vectors, &vectors)
        {
            return Some(s.clone());
        }
        let sample = matrix.sample(SAMPLE);
        let stats = vectors.iter().map(|w| self::stats(sample.iter().map(|r| search::score(w, r)))).collect();
        let s = Arc::new(VocabularyStats { made_from: matrix.identity(), names: WORDS.iter().map(|w| w.name()).collect(), vectors, stats });
        *cached = Some(s.clone());
        Some(s)
    }
}

/// What a tag's classifier is trained from, as one number.
fn signature(pos: &[usize], not: &[usize], set: &ExampleSet) -> u64 {
    let mut h = Fnv::new();
    for &i in pos {
        h.add(set.items.get(i).map_or("", |e| e.key.as_str()).as_bytes());
    }
    h.add(b"/");
    for &i in not {
        h.add(set.items.get(i).map_or("", |e| e.key.as_str()).as_bytes());
    }
    // every hundred new examples, the others it is told apart from have changed enough
    h.add(&((set.items.len() / 100) as u64).to_le_bytes());
    h.0
}

fn train_tag(pos: &[usize], not: &[usize], set: &ExampleSet, matrix: &Matrix) -> Option<Classifier> {
    let dim = matrix.dim();
    let vector = |i: usize| set.items.get(i).and_then(|e| search::dequantise(&e.record, dim));
    let positives: Vec<Vec<f32>> = pos.iter().filter_map(|&i| vector(i)).collect();
    let mut negatives: Vec<(Vec<f32>, f32)> = not.iter().filter_map(|&i| vector(i).map(|v| (v, 2.0))).collect();
    // the other tagged pictures, at most 600
    let others: Vec<usize> = (0..set.items.len()).filter(|i| !pos.contains(i) && !not.contains(i)).collect();
    negatives.extend(spread(&others, 600).into_iter().filter_map(|&i| vector(i).map(|v| (v, 1.0))));
    // few other tags yet: pictures of the library stand for "not this", counting less
    if negatives.len() < 30 {
        negatives.extend(matrix.sample(200).into_iter().filter_map(|r| search::dequantise(r, dim)).map(|v| (v, 0.5)));
    }
    Classifier::train(&positives, &negatives)
}

/// FNV-1a, for the signatures and the vocabulary file's check.
struct Fnv(u64);

impl Fnv {
    fn new() -> Fnv {
        Fnv(0xcbf2_9ce4_8422_2325)
    }

    fn add(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.0 ^= b as u64;
            self.0 = self.0.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }
}

const VOCABULARY_MAGIC: &[u8; 8] = b"ACVOCAB1";

/// The file the vocabulary's vectors are kept in, and the check of what it was made from (the
/// network and every sentence).
fn vocabulary_file(dir: &Path, model: &Model) -> (PathBuf, u64) {
    let mut h = Fnv::new();
    h.add(model.config.model.as_bytes());
    for w in WORDS {
        h.add(b"\n");
        h.add(w.prompt.as_bytes());
    }
    (dir.join(format!("vocabulary-{}.bin", model.config.model)), h.0)
}

fn read_vocabulary(dir: &Path, model: &Model) -> Option<Vec<Vec<f32>>> {
    let (path, check) = vocabulary_file(dir, model);
    let bytes = std::fs::read(path).ok()?;
    let dim = model.config.dim;
    let head = bytes.get(..24)?;
    let n = u32::from_le_bytes(head.get(8..12)?.try_into().ok()?) as usize;
    let d = u32::from_le_bytes(head.get(12..16)?.try_into().ok()?) as usize;
    let h = u64::from_le_bytes(head.get(16..24)?.try_into().ok()?);
    if head.get(..8)? != VOCABULARY_MAGIC || n != WORDS.len() || d != dim || h != check || bytes.len() != 24 + n * d * 4 {
        return None;
    }
    let floats: Vec<f32> = bytes.get(24..)?.as_chunks::<4>().0.iter().map(|c| f32::from_le_bytes(*c)).collect();
    Some(floats.chunks_exact(d).map(<[f32]>::to_vec).collect())
}

fn write_vocabulary(dir: &Path, model: &Model, vectors: &[Vec<f32>]) -> Result<(), String> {
    let (path, check) = vocabulary_file(dir, model);
    let mut bytes = VOCABULARY_MAGIC.to_vec();
    bytes.extend_from_slice(&(vectors.len() as u32).to_le_bytes());
    bytes.extend_from_slice(&(model.config.dim as u32).to_le_bytes());
    bytes.extend_from_slice(&check.to_le_bytes());
    for v in vectors {
        for x in v {
            bytes.extend_from_slice(&x.to_le_bytes());
        }
    }
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let part = path.with_extension("part");
    std::fs::write(&part, bytes).map_err(|e| e.to_string())?;
    std::fs::rename(&part, &path).map_err(|e| e.to_string())
}

/// Make the vocabulary's vectors on a thread of their own, then keep them.
fn start_vocabulary(state: Arc<PackState>, model: Arc<Model>, dir: PathBuf) {
    let for_thread = state.clone();
    let spawned = std::thread::Builder::new().name("tag vocabulary".into()).spawn(move || {
        let state = for_thread;
        let made: Result<Vec<Vec<f32>>, String> = WORDS.iter().map(|w| model.encode_sentence(w.prompt)).collect();
        let next = match made {
            Ok(vectors) => {
                // not kept: made again at the next start
                let _ = write_vocabulary(&dir, &model, &vectors);
                Vocabulary::Ready(Arc::new(vectors))
            }
            Err(_) => Vocabulary::Failed,
        };
        *state.vocabulary.lock().unwrap_or_else(PoisonError::into_inner) = next;
    });
    if spawned.is_err() {
        *state.vocabulary.lock().unwrap_or_else(PoisonError::into_inner) = Vocabulary::Failed;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A unit vector near the axis `axis` of 8, nudged by `k`.
    fn near(axis: usize, k: f32) -> Vec<f32> {
        let mut v: Vec<f32> = (0..8).map(|i| if i == axis { 1.0 } else { 0.05 * ((i as f32 + k).sin()) }).collect();
        search::normalise(&mut v);
        v
    }

    fn picture(key: &str, tags: &[&str]) -> Picture {
        Picture { key: key.into(), tags: tags.iter().map(|t| t.to_string()).collect(), ..Picture::default() }
    }

    fn example(key: &str, tags: &[&str], not: &[&str], v: &[f32]) -> Example {
        Example {
            key: key.into(),
            folder: folder_of(key),
            place: None,
            tags: tags.iter().map(|t| (t.to_lowercase(), t.to_string())).collect(),
            not_tags: not.iter().map(|t| t.to_lowercase()).collect(),
            record: search::quantise(v),
        }
    }

    /// Records of a library: `n` pictures around each of the 8 axes.
    fn library(n: usize) -> Vec<Vec<u8>> {
        (0..8).flat_map(|a| (0..n).map(move |k| search::quantise(&near(a, k as f32)))).collect()
    }

    #[test]
    fn the_tags_of_the_nearest_pictures_are_proposed() {
        let set = ExampleSet {
            made_from: ((0, 0), 0),
            items: vec![
                example("/trip/a.jpg", &["Regatta 2024", "Boat"], &[], &near(0, 1.0)),
                example("/trip/b.jpg", &["Regatta 2024"], &[], &near(0, 2.0)),
                example("/home/c.jpg", &["Garden"], &[], &near(5, 3.0)),
            ],
            names: HashMap::new(),
        };
        let lib = library(10);
        let sample: Vec<&[u8]> = lib.iter().map(Vec::as_slice).collect();
        let cx = Context { set: &set, classifiers: &[], vocabulary: None, floor: 0.0, margin: 0.05, sample: &sample };
        let target = picture("/trip/d.jpg", &[]);
        let got = merge(std::iter::once((&target, Some(near(0, 4.0)))), &cx);
        let names: Vec<&str> = got.tags.iter().map(|p| p.tag.as_str()).collect();
        assert_eq!(names.first(), Some(&"Regatta 2024"));
        assert!(names.contains(&"Boat") && !names.contains(&"Garden"), "{got:?}");

        // a tag the picture has is not proposed again; one it was said not to show neither
        let mut t2 = picture("/trip/d.jpg", &["regatta 2024"]);
        t2.not_tags = vec!["Boat".into()];
        let got = merge(std::iter::once((&t2, Some(near(0, 4.0)))), &cx);
        assert!(got.tags.is_empty(), "{got:?}");

        // nothing analysed: said so
        assert!(merge(std::iter::once((&target, None)), &cx).not_analysed);
    }

    #[test]
    fn a_tag_said_not_to_show_near_by_counts_less() {
        let items = |not: &[&str]| {
            vec![example("/x/a.jpg", &["Sea"], &[], &near(2, 1.0)), example("/x/b.jpg", &[], not, &near(2, 1.5)), example("/x/c.jpg", &["Sea"], &[], &near(2, 2.0))]
        };
        let lib = library(10);
        let sample: Vec<&[u8]> = lib.iter().map(Vec::as_slice).collect();
        let score = |not: &[&str]| {
            let set = ExampleSet { made_from: ((0, 0), 0), items: items(not), names: HashMap::new() };
            let cx = Context { set: &set, classifiers: &[], vocabulary: None, floor: 0.0, margin: 0.05, sample: &sample };
            scores_of(&near(2, 3.0), &picture("/x/d.jpg", &[]), &cx).into_iter().find(|(l, _, _)| l == "sea").map_or(0.0, |s| s.2)
        };
        assert!(score(&["sea"]) < score(&[]) * 0.7, "{} {}", score(&["sea"]), score(&[]));
    }

    #[test]
    fn a_selection_gets_the_tags_most_of_it_would_get() {
        let set = ExampleSet {
            made_from: ((0, 0), 0),
            items: vec![example("/a/1.jpg", &["Snow"], &[], &near(3, 1.0)), example("/a/2.jpg", &["Snow"], &[], &near(3, 2.0)), example("/b/3.jpg", &["Food"], &[], &near(6, 1.0))],
            names: HashMap::new(),
        };
        let lib = library(10);
        let sample: Vec<&[u8]> = lib.iter().map(Vec::as_slice).collect();
        let cx = Context { set: &set, classifiers: &[], vocabulary: None, floor: 0.0, margin: 0.05, sample: &sample };
        let (p1, p2, p3) = (picture("/a/4.jpg", &[]), picture("/a/5.jpg", &["Snow"]), picture("/a/6.jpg", &[]));
        let got = merge([(&p1, Some(near(3, 4.0))), (&p2, Some(near(3, 5.0))), (&p3, Some(near(3, 6.0)))].into_iter(), &cx);
        assert_eq!(got.tags.first().map(|p| p.tag.as_str()), Some("Snow"), "{got:?}");
        // every one has it: nothing to propose
        let all = [picture("/a/4.jpg", &["Snow"]), picture("/a/5.jpg", &["snow"])];
        let got = merge(all.iter().map(|p| (p, Some(near(3, 4.0)))), &cx);
        assert!(got.tags.iter().all(|p| p.tag.to_lowercase() != "snow"), "{got:?}");
    }

    #[test]
    fn a_classifier_tells_a_tags_pictures_apart() {
        let pos: Vec<Vec<f32>> = (0..20).map(|k| near(1, k as f32)).collect();
        let neg: Vec<(Vec<f32>, f32)> = (0..40).map(|k| (near(2 + k % 5, k as f32), 1.0)).collect();
        let c = Classifier::train(&pos, &neg).unwrap();
        assert!(c.p(&near(1, 99.0)) > 0.8, "{}", c.p(&near(1, 99.0)));
        assert!(c.p(&near(4, 99.0)) < 0.2, "{}", c.p(&near(4, 99.0)));
        // too few pictures, or nothing to tell them from: none
        assert!(Classifier::train(&pos[..5], &neg).is_none());
        assert!(Classifier::train(&pos, &[]).is_none());
        // the same vectors on both sides: never a crash
        let same: Vec<(Vec<f32>, f32)> = pos.iter().map(|v| (v.clone(), 1.0)).collect();
        let _ = Classifier::train(&pos, &same);
    }

    /// Real pictures and a real pack (see `smart_search::tests::analysed_library`): what the
    /// built-in words propose with no tag in the library, then what is proposed for the
    /// untagged pictures once a few in each folder have a tag of the user's.
    /// `cargo test --release -p albumcraft-core real_tag_proposals -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn real_tag_proposals() {
        let (pack, files, dir) = crate::smart_search::tests::analysed_library("real-tags");
        let photos = std::env::var("ALBUMCRAFT_SEARCH_PHOTOS").unwrap_or_default();
        let name = |k: &str| k.strip_prefix(photos.as_str()).unwrap_or(k).to_string();
        let files = Arc::new(files);
        let ask = |examples: &Arc<Examples>, target: &Picture| {
            let started = std::time::Instant::now();
            loop {
                let got = propose(pack, &files, &dir, examples, std::slice::from_ref(target)).unwrap();
                if !got.partial {
                    return got;
                }
                assert!(started.elapsed().as_secs() < 600, "the vocabulary is never ready");
                std::thread::sleep(std::time::Duration::from_millis(500));
            }
        };
        let t = std::time::Instant::now();
        let none = Arc::new(Examples::default());
        let _ = ask(&none, &picture(&files[0].0, &[]));
        println!("vocabulary ready in {:.1} s", t.elapsed().as_secs_f64());
        println!("\nno tags in the library:");
        for f in files.iter().step_by(9) {
            let t = std::time::Instant::now();
            let got = ask(&none, &picture(&f.0, &[]));
            let tags: Vec<String> = got.tags.iter().map(|p| format!("{} {:.2}", p.tag, p.score)).collect();
            println!("  {:60} {} ms: {}", name(&f.0).chars().take(60).collect::<String>(), t.elapsed().as_millis(), tags.join(", "));
        }
        // every third picture of each folder tagged with the folder's own tag
        let folder_tag = |k: &str| format!("{} trip", folder_of(k).file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default());
        let tagged: Vec<Picture> = files.iter().enumerate().filter(|(i, _)| i % 3 == 0).map(|(_, f)| picture(&f.0, &[&folder_tag(&f.0)])).collect();
        let examples = Arc::new(Examples { rev: 1, pictures: tagged });
        let (mut right, mut wrong, mut asked) = (0, 0, 0);
        println!("\nevery third picture tagged with its folder:");
        for (i, f) in files.iter().enumerate().filter(|(i, _)| i % 3 != 0) {
            let got = ask(&examples, &picture(&f.0, &[]));
            asked += 1;
            if got.tags.iter().any(|p| p.tag == folder_tag(&f.0)) {
                right += 1;
            }
            if got.tags.iter().any(|p| p.tag.ends_with(" trip") && p.tag != folder_tag(&f.0)) {
                wrong += 1;
            }
            if i % 7 == 1 {
                let tags: Vec<String> = got.tags.iter().map(|p| format!("{} {:.2}", p.tag, p.score)).collect();
                println!("  {:60} {}", name(&f.0).chars().take(60).collect::<String>(), tags.join(", "));
            }
        }
        println!("the folder's tag proposed for {right} of {asked}, another folder's for {wrong}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn stats_never_divide_by_zero() {
        assert_eq!(stats([].into_iter()), (0.0, 1.0));
        let (m, sd) = stats([0.5, 0.5, 0.5].into_iter());
        assert!((m - 0.5).abs() < 1e-6 && sd > 0.0);
        assert_eq!(spread(&[1, 2, 3, 4, 5], 2), [&1, &4]);
        assert_eq!(spread::<u8>(&[], 3).len(), 0);
    }
}
