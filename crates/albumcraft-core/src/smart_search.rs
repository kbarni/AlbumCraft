//! Smart search: finding pictures by describing them ("Looks like": "red car in the snow"), and
//! pictures like a given one ("More like this").
//!
//! The vectors of the library's pictures are made by the background analysis (`analysis`)
//! with the active search pack's image encoder; a search turns the typed text into a vector
//! with its text encoder and ranks every picture by one dot product. No index is needed: 50 000
//! pictures are ranked in a few milliseconds.
//!
//! One search pack is active at a time (the user's choice, `Store.search_pack`, else Light, or
//! Multilingual when the interface is not in English). Vectors of two networks can't be
//! compared: after a switch the library is analysed again in the background, and searches keep
//! using the other pack's vectors until the new ones cover as many pictures.
//!
//! Each pack's networks and vectors are loaded once per program (`Engine`), shared by the
//! window's background analysis and its searches, and by the commands.

use crate::ai::models::{self, Pack};
use crate::ai::search::{self, Model};
use crate::ai::store::Table;
use crate::analysis::{Analysis, SharedTable};
use crate::geo::FileStamp;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock, Mutex, PoisonError, RwLock};

/// The most pictures a search gives back (ranked); what is shown is cut much earlier (`cut`).
pub const MAX_HITS: usize = 2000;

/// A picture found, by key, and how well it matches (higher is better; not a probability).
#[derive(Clone, Debug, PartialEq)]
pub struct Hit {
    pub key: String,
    pub score: f32,
}

/// What is searched for.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Query {
    /// Pictures that look like this description.
    Text(String),
    /// Pictures like this one (its key).
    Like(String),
}

/// Every search pack this version knows.
pub fn search_packs() -> Vec<&'static Pack> {
    models::manifest().packs.iter().filter(|p| p.search.is_some()).collect()
}

fn installed(pack: &Pack) -> bool {
    models::models_dir().is_some_and(|d| pack.installed(&d))
}

/// The search pack searches use: `choice` (a pack id) if it is on this computer; else the
/// multilingual one when the interface's `language` is not English, else the English one,
/// else any. None: no search pack is downloaded.
pub fn active(choice: &str, language: &str) -> Option<&'static Pack> {
    let packs: Vec<&'static Pack> = search_packs().into_iter().filter(|p| installed(p)).collect();
    if let Some(p) = packs.iter().find(|p| p.id == choice) {
        return Some(p);
    }
    let english = language.is_empty() || language.starts_with("en");
    let multilingual = |p: &&&Pack| p.search.as_ref().is_some_and(|c| c.multilingual);
    if english {
        packs.iter().find(|p| !multilingual(p)).or(packs.first()).copied()
    } else {
        packs.iter().find(multilingual).or(packs.first()).copied()
    }
}

/// The vectors of the library's pictures in one block, in the order of the picture list they
/// were made from.
struct Matrix {
    files: Arc<Vec<FileStamp>>,
    changes: u64,
    /// For each vector, its picture's place in `files`.
    rows: Vec<u32>,
    /// The kept records, one after the other.
    data: Vec<u8>,
    record_len: usize,
}

impl Matrix {
    fn of(files: &Arc<Vec<FileStamp>>, table: &Table) -> Matrix {
        let record_len = table.spec().record_len;
        let (mut rows, mut data) = (vec![], vec![]);
        for (i, (key, modified, bytes)) in files.iter().enumerate() {
            if let Some(first) = table.get(key, (*modified, *bytes)).and_then(|r| r.first().copied())
                && let Ok(i) = u32::try_from(i)
            {
                rows.push(i);
                data.extend_from_slice(first);
            }
        }
        Matrix { files: files.clone(), changes: table.changes(), rows, data, record_len }
    }

    fn covered(&self) -> usize {
        self.rows.len()
    }

    fn record_of(&self, key: &str) -> Option<&[u8]> {
        let at = self.rows.iter().position(|&r| self.files.get(r as usize).is_some_and(|f| f.0 == key))?;
        self.data.get(at * self.record_len..(at + 1) * self.record_len)
    }

    /// Every picture, best match first, at most `MAX_HITS` (leaving out `skip`).
    fn rank(&self, query: &[f32], skip: Option<&str>) -> Vec<Hit> {
        let mut scored: Vec<(f32, u32)> = self
            .data
            .chunks_exact(self.record_len.max(1))
            .zip(&self.rows)
            .map(|(record, &row)| (search::score(query, record), row))
            .filter(|(s, _)| s.is_finite())
            .collect();
        scored.sort_unstable_by(|a, b| b.0.total_cmp(&a.0));
        scored
            .into_iter()
            .filter_map(|(score, row)| Some(Hit { key: self.files.get(row as usize)?.0.clone(), score }))
            .filter(|h| skip != Some(h.key.as_str()))
            .take(MAX_HITS)
            .collect()
    }
}

/// A search pack's networks and the vectors made with them.
struct Engine {
    model: Arc<Model>,
    table: SharedTable,
    matrix: Mutex<Option<Arc<Matrix>>>,
}

impl Engine {
    /// The vectors for this list of pictures, made again only when the list or the vectors
    /// changed.
    fn matrix(&self, files: &Arc<Vec<FileStamp>>) -> Arc<Matrix> {
        let table = self.table.read().unwrap_or_else(PoisonError::into_inner);
        let mut cached = self.matrix.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(m) = cached.as_ref()
            && Arc::ptr_eq(&m.files, files)
            && m.changes == table.changes()
        {
            return m.clone();
        }
        let m = Arc::new(Matrix::of(files, &table));
        *cached = Some(m.clone());
        m
    }
}

static ENGINES: LazyLock<Mutex<HashMap<String, Arc<Engine>>>> = LazyLock::new(Default::default);

fn engines() -> std::sync::MutexGuard<'static, HashMap<String, Arc<Engine>>> {
    ENGINES.lock().unwrap_or_else(PoisonError::into_inner)
}

fn no_folder() -> String {
    crate::msg!("analysis-no-data-dir", "No folder for application data was found.").to_string()
}

/// The engine of a pack (loaded once per program). `dir`: where the vectors are kept.
fn engine(pack: &Pack, dir: &Path) -> Result<Arc<Engine>, String> {
    let mut all = engines();
    if let Some(e) = all.get(&pack.id) {
        return Ok(e.clone());
    }
    let models = models::models_dir().ok_or_else(no_folder)?;
    let model = Model::new(pack, &models).ok_or_else(|| format!("{} is not a search pack", pack.id))?;
    let table = Table::open(dir, &model.config.table())?;
    let e = Arc::new(Engine { model: Arc::new(model), table: Arc::new(RwLock::new(table)), matrix: Mutex::new(None) });
    all.insert(pack.id.clone(), e.clone());
    Ok(e)
}

/// Make the background analysis use `active` (and no other search pack): called at the start,
/// after a pack was downloaded or removed, and when the user picks another one. Packs that are
/// no longer on this computer are forgotten (their networks leave memory; their vectors stay
/// on disk).
pub fn use_pack(analysis: &Analysis, active: Option<&Pack>) -> Result<(), String> {
    for pack in search_packs() {
        let Some(cfg) = &pack.search else { continue };
        if Some(pack.id.as_str()) != active.map(|a| a.id.as_str()) {
            analysis.remove(&cfg.table());
            if !installed(pack) {
                engines().remove(&pack.id);
            }
        }
    }
    let Some(pack) = active else { return Ok(()) };
    let dir = analysis.dir().ok_or_else(no_folder)?;
    let e = engine(pack, dir)?;
    analysis.add_with(e.model.clone(), e.table.clone());
    Ok(())
}

/// The engine and vectors to search with: the active pack's, unless another pack on this
/// computer has vectors for more of these pictures (the library is being analysed again after
/// a switch).
fn source(active: &Pack, files: &Arc<Vec<FileStamp>>, dir: &Path) -> Result<(Arc<Engine>, Arc<Matrix>), String> {
    let e = engine(active, dir)?;
    let mut best = (e.clone(), e.matrix(files));
    if best.1.covered() < files.len() {
        for other in search_packs().into_iter().filter(|p| p.id != active.id && installed(p)) {
            let Ok(o) = engine(other, dir) else { continue };
            let m = o.matrix(files);
            if m.covered() > best.1.covered() {
                best = (o, m);
            }
        }
    }
    Ok(best)
}

/// Rank `files` (the library's pictures) for a query with the `active` pack. `dir`: where the
/// vectors are kept (`ai::store::analysis_dir()`). The text encoder is loaded on the first
/// text search (a second or two), then each search takes some tens of milliseconds.
pub fn search(active: &Pack, files: &Arc<Vec<FileStamp>>, dir: &Path, query: &Query) -> Result<Vec<Hit>, String> {
    let (e, m) = source(active, files, dir)?;
    match query {
        Query::Text(text) => {
            let v = e.model.encode_text(text)?;
            Ok(m.rank(&v, None))
        }
        Query::Like(key) => {
            let record = m.record_of(key).ok_or_else(|| crate::msg!("search-not-analysed", "This picture is not analysed yet.").to_string())?;
            let v = search::dequantise(record, e.model.config.dim).ok_or("a damaged vector")?;
            Ok(m.rank(&v, Some(key)))
        }
    }
}

/// How many of the ranked `hits` are good enough to show at first: those close to the best
/// one, and for typed text above the pack's `min_score` (none when nothing shows it).
pub fn cut(pack: &Pack, hits: &[Hit], query: &Query) -> usize {
    let (floor, margin) = pack.search.as_ref().map_or((0.0, 0.05), |c| (c.min_score, c.text_margin));
    cut_at(hits, query, floor, margin)
}

fn cut_at(hits: &[Hit], query: &Query, floor: f32, margin: f32) -> usize {
    let Some(best) = hits.first().map(|h| h.score) else { return 0 };
    match query {
        Query::Text(_) => hits.iter().take_while(|h| h.score >= best - margin && h.score >= floor).count(),
        // the most alike picture is always shown
        Query::Like(_) => hits.iter().take_while(|h| h.score >= best - 0.2).count().max(1),
    }
}

/// How many of `files` have vectors of the active pack (for the agents' status).
pub fn analysed(active: &Pack, files: &Arc<Vec<FileStamp>>, dir: &Path) -> Result<usize, String> {
    Ok(engine(active, dir)?.matrix(files).covered())
}

/// Where the vectors are kept when there is no window (the commands): the usual place.
pub fn vectors_dir() -> Result<PathBuf, String> {
    crate::ai::store::analysis_dir().ok_or_else(no_folder)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::search::quantise;
    use crate::ai::store::Spec;

    fn unit(v: &[f32]) -> Vec<f32> {
        let mut v = v.to_vec();
        search::normalise(&mut v);
        v
    }

    fn table(name: &str, vectors: &[(&str, Vec<f32>)]) -> Table {
        let dir = std::env::temp_dir().join(format!("albumcraft-smart-search-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let mut t = Table::open(&dir, &Spec { kind: "embeddings".into(), model: "test-1".into(), record_len: search::record_len(3) }).unwrap();
        for (key, v) in vectors {
            t.put(key, (1, 2), &[quantise(&unit(v))]).unwrap();
        }
        t
    }

    #[test]
    fn pictures_are_ranked_by_how_well_they_match() {
        let t = table("rank", &[("/a.jpg", vec![1.0, 0.0, 0.0]), ("/b.jpg", vec![0.7, 0.7, 0.0]), ("/c.jpg", vec![0.0, 0.0, 1.0]), ("/changed.jpg", vec![1.0, 0.0, 0.0])]);
        // changed.jpg's file is not what was analysed; d.jpg was never analysed
        let files = Arc::new(vec![("/a.jpg".to_string(), 1, 2), ("/b.jpg".into(), 1, 2), ("/c.jpg".into(), 1, 2), ("/changed.jpg".into(), 9, 9), ("/d.jpg".into(), 1, 2)]);
        let m = Matrix::of(&files, &t);
        assert_eq!(m.covered(), 3);
        let hits = m.rank(&unit(&[1.0, 0.1, 0.0]), None);
        let keys: Vec<&str> = hits.iter().map(|h| h.key.as_str()).collect();
        assert_eq!(keys, ["/a.jpg", "/b.jpg", "/c.jpg"]);
        assert!(hits[0].score > 0.98 && hits[2].score.abs() < 0.05);
        // like a.jpg: itself left out
        let like = m.rank(&search::dequantise(m.record_of("/a.jpg").unwrap(), 3).unwrap(), Some("/a.jpg"));
        assert_eq!(like.first().map(|h| h.key.as_str()), Some("/b.jpg"));
        assert_eq!(like.len(), 2);
        assert!(m.record_of("/d.jpg").is_none());
    }

    #[test]
    fn only_matches_near_the_best_are_shown_at_first() {
        let hit = |s: f32| Hit { key: String::new(), score: s };
        let text = Query::Text("snow".into());
        assert_eq!(cut_at(&[hit(0.31), hit(0.29), hit(0.27), hit(0.2), hit(0.1)], &text, 0.0, 0.05), 3);
        assert_eq!(cut_at(&[], &text, 0.0, 0.05), 0);
        assert_eq!(cut_at(&[hit(0.1)], &text, 0.0, 0.05), 1);
        // nothing shows what was typed: nothing is shown
        assert_eq!(cut_at(&[hit(0.16), hit(0.15)], &text, 0.2, 0.05), 0);
        assert_eq!(cut_at(&[hit(0.24), hit(0.21), hit(0.19)], &text, 0.2, 0.05), 2);
        assert_eq!(cut_at(&[hit(0.9), hit(0.75), hit(0.5)], &Query::Like("/a.jpg".into()), 0.5, 0.05), 2);
    }

    /// The whole path with a real pack and real pictures: analysed in the background, then
    /// searched. `XDG_DATA_HOME=<dir>` with the pack's files in `<dir>/AlbumCraft/models`,
    /// `ALBUMCRAFT_SEARCH_PHOTOS=<folder>`, `ALBUMCRAFT_SEARCH_QUERIES="a beach|snow|…"`,
    /// `ALBUMCRAFT_SEARCH_PACK_ID=search-multilingual` (default: the Light pack);
    /// `cargo test --release -p albumcraft-core real_search -- --ignored --nocapture` prints the
    /// best matches.
    #[test]
    #[ignore]
    fn real_search() {
        use crate::analysis::Analysis;
        use crate::library::{Done, Pool};
        let photos = std::env::var("ALBUMCRAFT_SEARCH_PHOTOS").expect("ALBUMCRAFT_SEARCH_PHOTOS");
        let queries = std::env::var("ALBUMCRAFT_SEARCH_QUERIES").unwrap_or_else(|_| "a church|the sea|snow and ice|a street at night|food".into());
        let id = std::env::var("ALBUMCRAFT_SEARCH_PACK_ID").unwrap_or_else(|_| "search-light".into());
        let pack = active(&id, "en").filter(|p| p.id == id).expect("the pack's files in XDG_DATA_HOME/AlbumCraft/models");
        let files: Vec<FileStamp> = walk(Path::new(&photos));
        let dir = std::env::temp_dir().join(format!("albumcraft-real-search-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let (tx, rx) = std::sync::mpsc::channel();
        let pool = Pool::new(4, tx.clone(), Arc::new(|| {}));
        let analysis = Analysis::new(pool, Arc::new(move |d| drop(tx.send(d))), Some(dir.clone()));
        use_pack(&analysis, Some(pack)).unwrap();
        analysis.set_pictures(files.clone());
        let started = std::time::Instant::now();
        loop {
            match rx.recv_timeout(std::time::Duration::from_secs(120)).expect("the analysis goes on") {
                Done::Analysis(p) if !p.active() => break,
                Done::Report(r) => panic!("{}", r.text),
                _ => {}
            }
        }
        println!("{} pictures analysed in {:.1} s", files.len(), started.elapsed().as_secs_f64());
        let files = Arc::new(files);
        assert_eq!(analysed(pack, &files, &dir).unwrap(), files.len());
        let name = |k: &str| k.strip_prefix(&photos).unwrap_or(k).to_string();
        for q in queries.split('|') {
            let query = Query::Text(q.into());
            let t = std::time::Instant::now();
            let hits = search(pack, &files, &dir, &query).unwrap();
            println!("\n“{q}” ({} ms, {} shown first):", t.elapsed().as_millis(), cut(pack, &hits, &query));
            for h in hits.iter().take(4) {
                println!("  {:.3} {}", h.score, name(&h.key));
            }
        }
        let first = files[0].0.clone();
        let like = Query::Like(first.clone());
        let hits = search(pack, &files, &dir, &like).unwrap();
        println!("\nlike {} ({} shown first):", name(&first), cut(pack, &hits, &like));
        for h in hits.iter().take(4) {
            println!("  {:.3} {}", h.score, name(&h.key));
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    fn walk(dir: &Path) -> Vec<FileStamp> {
        let mut out = vec![];
        let Ok(entries) = std::fs::read_dir(dir) else { return out };
        let mut entries: Vec<_> = entries.flatten().map(|e| e.path()).collect();
        entries.sort();
        for p in entries {
            if p.is_dir() {
                out.extend(walk(&p));
            } else if p.extension().is_some_and(|e| e.eq_ignore_ascii_case("jpg") || e.eq_ignore_ascii_case("jpeg")) {
                if let Ok(m) = std::fs::metadata(&p) {
                    let modified = m.modified().ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map_or(0, |d| d.as_secs() as i64);
                    out.push((p.to_string_lossy().to_string(), modified, m.len()));
                }
            }
        }
        out
    }

    #[test]
    fn the_english_pack_unless_the_interface_is_not_english() {
        // with no search pack on this computer, none is active, whatever the choice
        if search_packs().iter().all(|p| !installed(p)) {
            assert!(active("", "fr").is_none());
            assert!(active("search-light", "en").is_none());
        }
    }
}
