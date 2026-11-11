//! The background analysis of the library by the AI features' networks, and the packs of
//! networks on this computer.
//!
//! `Analysis` is given the library's pictures after each scan. For every `Analyser` it knows
//! (one per kind of result: picture vectors, faces…), it finds the pictures not analysed yet,
//! or changed since, and feeds them a few at a time to the `Pool`'s background queue, which runs
//! them only while nothing else is waiting. Each result is written at once to the analyser's
//! file (`ai::store`), so closing the program loses nothing and the next start carries on.
//! Progress arrives as `Done::Analysis`.
//!
//! Analysers always see the picture unedited and turned the right way up.

use crate::ai::store::{Spec, Stamp, Table};
use crate::ai::{self, Analyser};
use crate::geo::FileStamp;
use crate::imaging::{self, SAMPLE_PREFIX};
use crate::library::{self, Done, Job, Pool, Report};
use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, PoisonError, RwLock};

/// How far the analysis of the library is.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Progress {
    /// Pictures analysed since the work at hand started, and how many it started with.
    pub done: usize,
    pub total: usize,
    pub paused: bool,
}

impl Progress {
    /// There is work left (running or paused).
    pub fn active(&self) -> bool {
        self.done < self.total
    }
}

/// The results of one analyser, shared with whoever searches them.
pub type SharedTable = Arc<RwLock<Table>>;

/// Sends an answer to the window (and wakes it); callable from any thread.
pub type Send_ = Arc<dyn Fn(Done) + Send + Sync>;

/// An analyser that fails this many times in a row is stopped until the next start: something
/// is wrong with it, not with the pictures.
const MAX_FAILURES: usize = 20;
/// Jobs kept waiting in the pool at a time: enough to keep its background workers busy, few
/// enough that Pause acts at once.
const AHEAD: usize = 3;

struct Lane {
    analyser: Arc<dyn Analyser>,
    table: SharedTable,
    /// Failures in a row (not counting pictures that can't be read).
    failures: usize,
    stopped: bool,
}

struct Waiting {
    lane: usize,
    key: String,
    stamp: Stamp,
}

#[derive(Default)]
struct State {
    lanes: Vec<Lane>,
    /// The library's pictures, as last given.
    files: Vec<FileStamp>,
    waiting: VecDeque<Waiting>,
    /// Jobs given to the pool and not finished.
    in_pool: usize,
    done: usize,
    total: usize,
    paused: bool,
    /// Counts the times the list of work was made again: jobs of an older list still finish,
    /// but are not counted in the new one.
    round: u64,
}

impl State {
    fn progress(&self) -> Progress {
        Progress { done: self.done.min(self.total), total: self.total, paused: self.paused }
    }

    /// Make the list of work again: every picture each analyser has not seen as it is now.
    fn plan(&mut self) {
        self.round += 1;
        self.waiting.clear();
        for (i, lane) in self.lanes.iter().enumerate().filter(|(_, l)| !l.stopped) {
            let table = lane.table.read().unwrap_or_else(PoisonError::into_inner);
            for (key, modified, bytes) in &self.files {
                if !key.starts_with(SAMPLE_PREFIX) && !table.has(key, (*modified, *bytes)) {
                    self.waiting.push_back(Waiting { lane: i, key: key.clone(), stamp: (*modified, *bytes) });
                }
            }
        }
        self.done = 0;
        self.total = self.waiting.len();
    }
}

struct Inner {
    pool: Pool,
    send: Send_,
    /// Where the result files are kept (None: nowhere to keep them, nothing is analysed).
    dir: Option<PathBuf>,
    state: Mutex<State>,
}

/// The background analysis of one library.
#[derive(Clone)]
pub struct Analysis(Arc<Inner>);

impl Analysis {
    /// `dir`: where the result files are kept (`ai::store::analysis_dir()`).
    pub fn new(pool: Pool, send: Send_, dir: Option<PathBuf>) -> Analysis {
        Analysis(Arc::new(Inner { pool, send, dir, state: Mutex::new(State::default()) }))
    }

    fn state(&self) -> std::sync::MutexGuard<'_, State> {
        self.0.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Analyse the library with this analyser too, from now on. Gives its results (what was
    /// found on earlier runs is there at once). An analyser whose file is open already is not
    /// added twice.
    pub fn add(&self, analyser: Arc<dyn Analyser>) -> Result<SharedTable, String> {
        let spec = analyser.table();
        if let Some(table) = self.table(&spec) {
            return Ok(table);
        }
        let dir = self.0.dir.as_ref().ok_or("No folder for application data was found.")?;
        let table = Arc::new(RwLock::new(Table::open(dir, &spec)?));
        let mut s = self.state();
        s.lanes.push(Lane { analyser, table: table.clone(), failures: 0, stopped: false });
        s.plan();
        drop(s);
        self.feed();
        Ok(table)
    }

    /// The results kept in the file of `spec`, when an analyser for it was added.
    pub fn table(&self, spec: &Spec) -> Option<SharedTable> {
        self.state().lanes.iter().find(|l| l.table.read().unwrap_or_else(PoisonError::into_inner).spec() == spec).map(|l| l.table.clone())
    }

    /// These are the library's pictures now (after a scan): analyse the ones that are new or
    /// changed.
    pub fn set_pictures(&self, files: Vec<FileStamp>) {
        let mut s = self.state();
        s.files = files;
        s.plan();
        let progress = s.progress();
        drop(s);
        (self.0.send)(Done::Analysis(progress));
        self.feed();
    }

    /// Stop giving out work (what is being analysed right now is finished), or carry on.
    pub fn set_paused(&self, paused: bool) {
        let mut s = self.state();
        if s.paused == paused {
            return;
        }
        s.paused = paused;
        let progress = s.progress();
        drop(s);
        (self.0.send)(Done::Analysis(progress));
        self.feed();
    }

    pub fn progress(&self) -> Progress {
        self.state().progress()
    }

    /// Give the pool its next jobs.
    fn feed(&self) {
        let mut s = self.state();
        while !s.paused && s.in_pool < AHEAD {
            let Some(w) = s.waiting.pop_front() else { break };
            let Some(lane) = s.lanes.get(w.lane).filter(|l| !l.stopped) else {
                s.total = s.total.saturating_sub(1);
                continue;
            };
            let (analyser, round, me) = (lane.analyser.clone(), s.round, self.clone());
            s.in_pool += 1;
            let work = move || {
                let found = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| look_at(&*analyser, &w.key))).unwrap_or_else(|_| Found::Failed("unexpected error".into()));
                me.finished(round, w, found)
            };
            self.0.pool.push(Job::Background(Box::new(work)), false);
        }
    }

    /// A job ended: keep what it found, count it, give out the next one.
    fn finished(&self, round: u64, w: Waiting, found: Found) -> Done {
        let mut s = self.state();
        s.in_pool = s.in_pool.saturating_sub(1);
        let mut gave_up = None;
        if let Some(lane) = s.lanes.get_mut(w.lane) {
            let kept = match found {
                Found::Records(records) => lane.table.write().unwrap_or_else(PoisonError::into_inner).put(&w.key, w.stamp, &records),
                Found::Failed(e) => Err(e),
            };
            match kept {
                Ok(()) => lane.failures = 0,
                Err(e) => {
                    lane.failures += 1;
                    if lane.failures >= MAX_FAILURES && !lane.stopped {
                        lane.stopped = true;
                        gave_up = Some(e);
                    }
                }
            }
        }
        if round == s.round {
            s.done += 1;
        }
        if gave_up.is_some() {
            // what is left for that analyser is not work any more
            let before = s.waiting.len();
            s.waiting.retain(|x| x.lane != w.lane);
            s.total = s.total.saturating_sub(before - s.waiting.len());
        }
        let progress = s.progress();
        drop(s);
        if let Some(e) = gave_up {
            (self.0.send)(Done::Report(Report::failed(format!("The analysis of the pictures was stopped: it failed {MAX_FAILURES} times in a row ({e}). It will be tried again the next time AlbumCraft starts."))));
        }
        self.feed();
        Done::Analysis(progress)
    }
}

enum Found {
    /// What the analyser found (nothing, for a picture that can't be read: it is not tried
    /// again until its file changes).
    Records(Vec<Vec<u8>>),
    /// The analyser failed: nothing is kept, the picture is tried again at the next start.
    Failed(String),
}

fn look_at(analyser: &dyn Analyser, key: &str) -> Found {
    let dim = analyser.input_dim().max(1);
    // small inputs come from the thumbnail cache, nearly free
    let picture = if dim <= library::THUMB_DIM { library::cached_thumbnail(key, library::THUMB_DIM) } else { imaging::load_source(key, dim) };
    let Some(picture) = picture else { return Found::Records(vec![]) };
    match analyser.analyse(&picture) {
        Ok(records) => Found::Records(records),
        Err(e) => Found::Failed(e),
    }
}

// ---------------------------------------------------------------------------
// The packs of networks

/// A pack of the built-in list and whether it is on this computer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PackState {
    pub pack: ai::models::Pack,
    pub installed: bool,
}

/// Every pack this version knows, with whether it is downloaded.
pub fn packs() -> Vec<PackState> {
    let dir = ai::models::models_dir();
    ai::models::manifest().packs.iter().map(|p| PackState { pack: p.clone(), installed: dir.as_ref().is_some_and(|d| p.installed(d)) }).collect()
}

fn pack(id: &str) -> Result<&'static ai::models::Pack, String> {
    ai::models::manifest().pack(id).ok_or_else(|| {
        let known: Vec<&str> = ai::models::manifest().packs.iter().map(|p| p.id.as_str()).collect();
        if known.is_empty() { format!("no pack is named {id:?}: this version of AlbumCraft has no networks to download yet") } else { format!("no pack is named {id:?}; the packs are: {}", known.join(", ")) }
    })
}

/// Download a pack (the user asked for it). `step(done, total)` is told how far it is, in
/// 1/1000, and answers false to stop.
pub fn download_pack(id: &str, step: &dyn Fn(usize, usize) -> bool) -> Report {
    let found = pack(id).and_then(|p| Ok((p, ai::models::models_dir().ok_or("No folder for application data was found.")?)));
    let (pack, dir) = match found {
        Ok(f) => f,
        Err(e) => return Report::failed(format!("Nothing was downloaded: {e}.")),
    };
    let progress = |done: u64, total: u64| step((done.saturating_mul(1000) / total.max(1)) as usize, 1000);
    match ai::download::download_pack(pack, &dir, &progress) {
        Ok(()) => Report::done(format!("{} is downloaded ({}).", pack.title, library::fmt_bytes(pack.bytes()))),
        Err(e) if e == ai::STOPPED => Report::done(format!("The download of {} was stopped. Starting it again continues where it was.", pack.title)),
        Err(e) => Report::failed(format!("{} could not be downloaded. {e}", pack.title)),
    }
}

/// Remove a pack's files from this computer. What was computed with it is kept.
pub fn remove_pack(id: &str) -> Result<(), String> {
    let dir = ai::models::models_dir().ok_or("No folder for application data was found.")?;
    pack(id)?.remove(&dir)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use std::sync::mpsc::{Receiver, channel};
    use std::time::Duration;

    /// Stands in for a network: the picture's size and its first pixel.
    struct Sizes {
        model: &'static str,
        seen: AtomicUsize,
        fail: AtomicBool,
    }

    impl Sizes {
        fn new(model: &'static str) -> Arc<Sizes> {
            Arc::new(Sizes { model, seen: AtomicUsize::new(0), fail: AtomicBool::new(false) })
        }
    }

    impl Analyser for Sizes {
        fn table(&self) -> Spec {
            Spec { kind: "test".into(), model: self.model.into(), record_len: 12 }
        }
        fn input_dim(&self) -> u32 {
            64
        }
        fn analyse(&self, picture: &image::RgbaImage) -> Result<Vec<Vec<u8>>, String> {
            self.seen.fetch_add(1, Ordering::SeqCst);
            if self.fail.load(Ordering::SeqCst) {
                return Err("the network is broken".into());
            }
            let mut r = picture.width().to_le_bytes().to_vec();
            r.extend_from_slice(&picture.height().to_le_bytes());
            r.extend_from_slice(&picture.get_pixel(0, 0).0);
            Ok(vec![r])
        }
    }

    struct Setup {
        dir: PathBuf,
        analysis: Analysis,
        rx: Receiver<Done>,
        pool: Pool,
    }

    fn setup(name: &str) -> Setup {
        let dir = std::env::temp_dir().join(format!("albumcraft-analysis-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("pictures")).unwrap();
        let (tx, rx) = channel();
        let pool = Pool::new(3, tx.clone(), Arc::new(|| {}));
        let send: Send_ = Arc::new(move |d| {
            let _ = tx.send(d);
        });
        let analysis = Analysis::new(pool.clone(), send, Some(dir.join("analysis")));
        Setup { dir, analysis, rx, pool }
    }

    /// `n` small pictures on disk, as the scan would list them.
    fn pictures(dir: &std::path::Path, n: u32) -> Vec<FileStamp> {
        (0..n)
            .map(|i| {
                let path = dir.join("pictures").join(format!("p{i}.png"));
                imaging::sample_image(i + 1, 40 + i, 30).save(&path).unwrap();
                let m = std::fs::metadata(&path).unwrap();
                (path.to_string_lossy().to_string(), 1000 + i as i64, m.len())
            })
            .collect()
    }

    /// Wait for the analysis to say it has done everything; returns every progress seen.
    fn wait_done(rx: &Receiver<Done>) -> Vec<Progress> {
        let mut seen = vec![];
        loop {
            match rx.recv_timeout(Duration::from_secs(30)).expect("the analysis goes on") {
                Done::Analysis(p) => {
                    seen.push(p);
                    if !p.active() {
                        return seen;
                    }
                }
                Done::Report(r) => panic!("unexpected report: {}", r.text),
                _ => {}
            }
        }
    }

    #[test]
    fn the_library_is_analysed_once_and_changed_pictures_again() {
        let s = setup("once");
        let mut files = pictures(&s.dir, 7);
        files.push((format!("{SAMPLE_PREFIX}3"), 0, 0));
        files.push((s.dir.join("pictures/unreadable.jpg").to_string_lossy().to_string(), 5, 5));
        let sizes = Sizes::new("sizes-1");
        let table = s.analysis.add(sizes.clone()).unwrap();
        s.analysis.set_pictures(files.clone());
        let seen = wait_done(&s.rx);
        assert_eq!(seen.last(), Some(&Progress { done: 8, total: 8, paused: false }), "the sample picture is not analysed");
        assert!(seen.windows(2).all(|w| w[0].done <= w[1].done));
        assert_eq!(sizes.seen.load(Ordering::SeqCst), 7);
        {
            let t = table.read().unwrap();
            let (key, modified, bytes) = &files[2];
            let found = t.get(key, (*modified, *bytes)).unwrap();
            assert_eq!(found.len(), 1);
            assert_eq!(&found[0][..8], [42u32.to_le_bytes(), 30u32.to_le_bytes()].concat(), "the picture is given whole, at the size asked for at most");
            // a file that can't be read is marked, and not tried at every start
            assert_eq!(t.get(&files[8].0, (5, 5)), Some(vec![]));
        }
        // the same library again: nothing to do
        s.analysis.set_pictures(files.clone());
        assert_eq!(wait_done(&s.rx), [Progress::default()]);
        // one picture changed, one is new
        files[0].1 += 1;
        files.extend(pictures(&s.dir, 8).split_off(7));
        s.analysis.set_pictures(files.clone());
        assert_eq!(wait_done(&s.rx).last(), Some(&Progress { done: 2, total: 2, paused: false }));
        assert_eq!(sizes.seen.load(Ordering::SeqCst), 9);
        drop(table);

        // the next start: what was found is still there, and another network starts from nothing
        let again = setup("once-again");
        let analysis = Analysis::new(again.pool.clone(), Arc::new(|_| {}), Some(s.dir.join("analysis")));
        let sizes = Sizes::new("sizes-1");
        let table = analysis.add(sizes.clone()).unwrap();
        assert!(Arc::ptr_eq(&table, &analysis.add(sizes.clone()).unwrap()), "an analyser is added once");
        analysis.set_pictures(files.clone());
        assert_eq!(analysis.progress(), Progress::default());
        assert_eq!(table.read().unwrap().pictures(), 9);
        let newer = Sizes::new("sizes-2");
        analysis.add(newer).unwrap();
        assert_eq!(analysis.progress().total, 9);
        let _ = std::fs::remove_dir_all(&s.dir);
        let _ = std::fs::remove_dir_all(&again.dir);
    }

    #[test]
    fn pause_holds_the_work_and_resume_finishes_it() {
        let s = setup("pause");
        let files = pictures(&s.dir, 12);
        let sizes = Sizes::new("sizes-1");
        s.analysis.add(sizes.clone()).unwrap();
        s.analysis.set_paused(true);
        s.analysis.set_pictures(files);
        std::thread::sleep(Duration::from_millis(150));
        assert_eq!(sizes.seen.load(Ordering::SeqCst), 0);
        assert_eq!(s.analysis.progress(), Progress { done: 0, total: 12, paused: true });
        // what was said while paused
        while s.rx.try_recv().is_ok() {}
        s.analysis.set_paused(false);
        assert_eq!(wait_done(&s.rx).last(), Some(&Progress { done: 12, total: 12, paused: false }));
        let _ = std::fs::remove_dir_all(&s.dir);
    }

    /// The analysis never takes every worker (that what the window asks for comes first is
    /// checked on the queues themselves, in `library`).
    #[test]
    fn at_most_two_workers_analyse() {
        let (tx, rx) = channel();
        let pool = Pool::new(4, tx, Arc::new(|| {}));
        let running = Arc::new(AtomicUsize::new(0));
        let most = Arc::new(AtomicUsize::new(0));
        for _ in 0..8 {
            let (running, most) = (running.clone(), most.clone());
            let work = move || {
                most.fetch_max(running.fetch_add(1, Ordering::SeqCst) + 1, Ordering::SeqCst);
                std::thread::sleep(Duration::from_millis(30));
                running.fetch_sub(1, Ordering::SeqCst);
                Done::Toast("background".into())
            };
            pool.push(Job::Background(Box::new(work)), false);
        }
        // the other workers still answer the window meanwhile
        pool.push(Job::Thumb { id: 1, key: format!("{SAMPLE_PREFIX}1"), dim: 64 }, false);
        let (mut thumbs, mut background) = (0, 0);
        while thumbs < 1 || background < 8 {
            match rx.recv_timeout(Duration::from_secs(30)).unwrap() {
                Done::Thumb { .. } => thumbs += 1,
                Done::Toast(_) => background += 1,
                _ => {}
            }
        }
        assert_eq!(most.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn a_broken_analyser_is_stopped_and_reported() {
        let s = setup("broken");
        let files = pictures(&s.dir, 30);
        let sizes = Sizes::new("sizes-1");
        sizes.fail.store(true, Ordering::SeqCst);
        let table = s.analysis.add(sizes.clone()).unwrap();
        s.analysis.set_pictures(files);
        let mut reported = false;
        loop {
            match s.rx.recv_timeout(Duration::from_secs(30)).expect("an answer") {
                Done::Report(r) => {
                    assert!(!r.ok && r.text.contains("the network is broken"));
                    reported = true;
                }
                Done::Analysis(p) if !p.active() && reported => break,
                _ => {}
            }
        }
        assert!(sizes.seen.load(Ordering::SeqCst) < 30, "it does not go through the whole library failing");
        assert_eq!(table.read().unwrap().pictures(), 0, "a failure is not kept as a result");
        let _ = std::fs::remove_dir_all(&s.dir);
    }

    #[test]
    fn unknown_packs_are_errors() {
        assert!(remove_pack("no-such-pack").is_err());
        let report = download_pack("no-such-pack", &|_, _| true);
        assert!(!report.ok && report.text.contains("no-such-pack"));
        assert_eq!(packs().len(), ai::models::manifest().packs.len());
    }
}
