//! The `Library` interface: where the pictures and their notes live, as the window sees them.
//!
//! `App` never reads picture files, scans folders or writes `library.json` itself: it asks its
//! `Library`. Today there is one implementation, `LocalLibrary` (folders on this computer and
//! `library.json`, as always); a library on a server can implement the same interface later.
//!
//! Slow answers (scans, thumbnails, previews, exports…) come back as `Done` messages on the
//! channel given to the library, and the library wakes the window when one is ready. Quick ones
//! (a picture's size, its camera information) are answered directly.

use crate::i18n::tr;
use crate::exifinfo::CameraInfo;
use crate::geo::{self, FileStamp, LatLon};
use crate::imaging::{self, EditOp};
use crate::library::{self, Done, IniFolder, Job, Loaded, Pool, Report, Store, Wake};
use crate::slides::SlideSettings;
use image::RgbaImage;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::Arc;
use std::thread::JoinHandle;

/// Reads a picture, scaled so its longest side is at most the given size (0 = full size),
/// from any thread. None when it can't be read.
pub type Pictures = Arc<dyn Fn(&str, u32) -> Option<RgbaImage> + Send + Sync>;

/// GPS positions found by the background index, a batch at a time (see `geo::index_files`).
pub type GpsBatches = Receiver<Vec<(String, Option<LatLon>)>>;

/// A long piece of work the user watches in the status bar: its number (for `Done::Progress`
/// and `Report::task`) and the switch that asks it to stop.
pub struct Task {
    pub id: u64,
    pub stop: Arc<AtomicBool>,
}

pub trait Library {
    /// Read the saved notes (stars, captions, edits, albums, settings).
    fn load(&mut self) -> Loaded;
    /// Save the notes: in the background, or before returning when `now` (the window closes).
    /// The result is collected with `take_saved`.
    fn save(&mut self, store: &mut Store, now: bool);
    /// A save is still being written.
    fn saving(&self) -> bool;
    /// The result of the last save once it is finished (`wait`: wait for it).
    fn take_saved(&mut self, wait: bool) -> Option<Result<(), String>>;
    /// Tidy the caches (thumbnails), off the window's thread.
    fn tidy(&self);
    /// How much disk the caches use; the answer arrives as `Done::CacheSizes`.
    fn cache_sizes(&self);
    /// Empty a cache, then send `Done::CacheSizes` (a failure arrives as `Done::Toast` first).
    fn clear_cache(&self, which: library::Cache);

    /// Look for pictures in these folders; the result arrives as `Done::Scan`.
    /// `ini_done`: folders whose `.picasa.ini` was brought in already; for the others the scan
    /// reads that file (read only, never written) and sends what it holds along.
    fn scan(&self, roots: Vec<String>, excluded: Vec<String>, ini_done: Vec<String>);
    /// What the last scan found in these folders, from the catalogue cache, to show until the
    /// scan's result arrives; it arrives as `Done::Cached` (nothing when there is no cache).
    fn cached(&self, roots: Vec<String>, excluded: Vec<String>);
    /// Keep what the window knows about the pictures for the next start: in the background, or
    /// before returning when `now` (the window closes).
    fn save_catalogue(&self, cat: crate::catalogue::Catalogue, now: bool);
    /// A thumbnail, preview or sharp render; the result arrives as `Done::Thumb`/`Big`/`Zoom`.
    fn request(&self, job: Job, urgent: bool);
    /// The picture's real size in pixels, as shown (from the file header).
    fn source_dims(&self, key: &str) -> Option<(u32, u32)>;
    /// Camera information (EXIF) of a picture, None when it has none.
    fn camera_info(&self, key: &str) -> Option<CameraInfo>;
    /// The GPS position recorded in the picture file.
    fn file_gps(&self, key: &str) -> Option<LatLon>;
    /// Start reading the GPS positions of these pictures in the background, until `stop`.
    fn index_gps(&self, files: Vec<FileStamp>, stop: Arc<AtomicBool>) -> GpsBatches;
    /// Reads pictures for output made by the window (print pages, collages).
    fn pictures(&self) -> Pictures;

    /// Export edited copies into `dir`. Like the three below: progress arrives as
    /// `Done::Progress`, the end as `Done::Report`, both with the task's number.
    /// `with_location`: the GPS position in the copies; the rest as `opts` says.
    fn export(&self, task: Task, jobs: Vec<crate::commands::ExportJob>, dir: PathBuf, opts: crate::export::Options, with_location: bool);
    /// Move these picture files to the system's trash (`library::move_to_trash`).
    fn trash(&self, task: Task, files: Vec<String>);
    /// Small copies of these pictures, attached to a new message in the mail program
    /// (`mail::email`).
    fn email(&self, task: Task, jobs: Vec<crate::commands::ExportJob>, subject: String, with_location: bool, opts: crate::export::Options);
    /// Make a web gallery in a new folder inside `parent`.
    fn make_gallery(&self, task: Task, items: Vec<crate::gallery::Item>, parent: PathBuf, opts: crate::gallery::Options);
    /// Make a movie file.
    fn make_movie(&self, task: Task, pictures: Vec<(String, Vec<EditOp>)>, out: PathBuf, size: (u32, u32), settings: SlideSettings);
    /// Save a collage or contact sheet (a PDF, or pictures `px_long` pixels on their longest
    /// side).
    fn make_collage(&self, task: Task, job: crate::collage::Job, out: PathBuf, px_long: u32);

    /// List the pictures in an import source, marking the ones `library` (path, size) already
    /// has; the list arrives as `Done::ImportList`.
    fn list_import(&self, source: PathBuf, library: Vec<(String, u64)>);
    /// Copy pictures into `dest` (see `import::copy_into`) until `stop`; progress arrives as
    /// `Done::ImportProgress`, the end as `Done::ImportDone`.
    fn import(&self, files: Vec<String>, dest: PathBuf, stop: Arc<AtomicBool>);
    /// Memory cards, cameras and USB drives plugged in now, as (label, folder).
    fn import_sources(&self) -> Vec<(String, PathBuf)>;

    /// These are the library's pictures now: the AI features analyse the new and changed ones
    /// in the background, whenever nothing else is waiting. Progress arrives as
    /// `Done::Analysis`.
    fn analyse(&self, files: Vec<FileStamp>);
    /// Hold the background analysis, or let it carry on.
    fn pause_analysis(&self, paused: bool);
    /// The packs of networks this version knows and which are downloaded (a quick look at a
    /// few files).
    fn model_packs(&self) -> Vec<crate::analysis::PackState>;
    /// Download a pack of networks, as a task (`Done::Progress`, then `Done::Report`). The one
    /// connection the AI features make, and only when the user asks.
    fn download_pack(&self, task: Task, id: String);
    /// Remove a downloaded pack; what was computed with it is kept.
    fn remove_pack(&self, id: &str) -> Result<(), String>;
}

/// The pictures in folders on this computer, notes in `library.json` (`library::store_path`).
pub struct LocalLibrary {
    pool: Pool,
    tx: Sender<Done>,
    wake: Wake,
    analysis: crate::analysis::Analysis,
    saving: Option<JoinHandle<Result<(), String>>>,
    /// A save that finished (or failed) before it could go to another thread.
    saved: Option<Result<(), String>>,
}

impl LocalLibrary {
    /// `tx` receives the slow answers; `wake` is called after each one.
    pub fn new(tx: Sender<Done>, wake: Wake) -> LocalLibrary {
        let threads = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4).clamp(2, 6);
        let pool = Pool::new(threads, tx.clone(), wake.clone());
        let send: crate::analysis::Send_ = {
            let (tx, wake) = (tx.clone(), wake.clone());
            Arc::new(move |d: Done| {
                let _ = tx.send(d);
                wake();
            })
        };
        // No analyser yet: they come with the features that need them (picture vectors for
        // search, faces), each one when its pack of networks is downloaded.
        let analysis = crate::analysis::Analysis::new(pool.clone(), send, crate::ai::store::analysis_dir());
        LocalLibrary { pool, tx, wake, analysis, saving: None, saved: None }
    }

    /// Sends an answer to the window and wakes it; for use on any thread.
    fn sender(&self) -> impl Fn(Done) + Send + 'static {
        let (tx, wake) = (self.tx.clone(), self.wake.clone());
        move |d: Done| {
            let _ = tx.send(d);
            wake();
        }
    }

    /// Run `work` on its own thread as a task the user sees: `work` gets `step(done, total)` to
    /// say how far it is, which answers false once the user asked to stop. Its report (a panic
    /// becomes `failed`) ends the task.
    fn run_task(&self, task: Task, failed: String, work: impl FnOnce(&dyn Fn(usize, usize) -> bool) -> Report + Send + 'static) {
        let send = self.sender();
        std::thread::spawn(move || {
            let step = |done: usize, total: usize| {
                send(Done::Progress { task: task.id, done, total });
                !task.stop.load(std::sync::atomic::Ordering::Relaxed)
            };
            let mut report = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| work(&step))).unwrap_or_else(|_| Report::failed(failed));
            report.task = Some(task.id);
            send(Done::Report(report));
        });
    }
}

impl Library for LocalLibrary {
    fn load(&mut self) -> Loaded {
        Store::load()
    }

    fn save(&mut self, store: &mut Store, now: bool) {
        let job = library::store_path()
            .ok_or_else(|| tr!("backend-no-data-dir"))
            .and_then(|path| Ok((path, store.snapshot()?)));
        match job {
            Ok((path, bytes)) if now => self.saved = Some(library::write_library(&path, &bytes)),
            Ok((path, bytes)) => self.saving = Some(std::thread::spawn(move || library::write_library(&path, &bytes))),
            Err(e) => self.saved = Some(Err(e)),
        }
    }

    fn saving(&self) -> bool {
        self.saving.is_some()
    }

    fn take_saved(&mut self, wait: bool) -> Option<Result<(), String>> {
        if let Some(r) = self.saved.take() {
            return Some(r);
        }
        if self.saving.as_ref().is_some_and(|h| wait || h.is_finished()) {
            let h = self.saving.take()?;
            return Some(h.join().unwrap_or_else(|_| Err("the save stopped unexpectedly".into())));
        }
        None
    }

    fn tidy(&self) {
        std::thread::spawn(crate::thumbcache::prune);
    }

    fn cache_sizes(&self) {
        let send = self.sender();
        std::thread::spawn(move || {
            let (thumbs, tiles) = library::cache_sizes();
            send(Done::CacheSizes { thumbs, tiles });
        });
    }

    fn clear_cache(&self, which: library::Cache) {
        let send = self.sender();
        std::thread::spawn(move || {
            if let Err(e) = library::clear_cache(which) {
                send(Done::Toast(tr!("backend-cache-fail", error = e.to_string())));
            }
            let (thumbs, tiles) = library::cache_sizes();
            send(Done::CacheSizes { thumbs, tiles });
        });
    }

    fn scan(&self, roots: Vec<String>, excluded: Vec<String>, ini_done: Vec<String>) {
        let send = self.sender();
        std::thread::spawn(move || {
            let (found, truncated, inis) = std::panic::catch_unwind(|| {
                let (found, truncated) = library::scan_roots_except(&roots, &excluded, library::SCAN_LIMIT);
                let done: std::collections::HashSet<&str> = ini_done.iter().map(String::as_str).collect();
                let inis: Vec<(String, IniFolder)> = found
                    .iter()
                    .filter(|(path, _)| !done.contains(path.as_str()))
                    .filter_map(|(path, _)| Some((path.clone(), library::read_picasa_ini(Path::new(path))?)))
                    .collect();
                (found, truncated, inis)
            })
            .unwrap_or_default();
            send(Done::Scan(found, truncated, inis));
        });
    }

    fn cached(&self, roots: Vec<String>, excluded: Vec<String>) {
        let send = self.sender();
        std::thread::spawn(move || {
            let t0 = std::time::Instant::now();
            let found = std::panic::catch_unwind(|| {
                let cat = crate::catalogue::load_from(&crate::catalogue::path()?)?;
                Some(cat.within_except(&roots, &excluded, library::SCAN_LIMIT))
            });
            if let Ok(Some((folders, known))) = found
                && !folders.is_empty()
            {
                library::trace_since(&format!("catalogue cache: {} folders", folders.len()), t0, 0);
                send(Done::Cached(folders, known));
            }
        });
    }

    fn save_catalogue(&self, cat: crate::catalogue::Catalogue, now: bool) {
        // only a cache: a failure means a slower next start, nothing to tell the user
        let write = move || {
            if let Some(path) = crate::catalogue::path() {
                let _ = std::panic::catch_unwind(|| crate::catalogue::save_to(&path, &cat));
            }
        };
        if now {
            write();
        } else {
            std::thread::spawn(write);
        }
    }

    fn request(&self, job: Job, urgent: bool) {
        self.pool.push(job, urgent);
    }

    fn source_dims(&self, key: &str) -> Option<(u32, u32)> {
        library::source_dims(key)
    }

    fn camera_info(&self, key: &str) -> Option<CameraInfo> {
        if key.starts_with(imaging::SAMPLE_PREFIX) { None } else { crate::exifinfo::read(key) }
    }

    fn file_gps(&self, key: &str) -> Option<LatLon> {
        geo::file_gps(key)
    }

    fn index_gps(&self, files: Vec<FileStamp>, stop: Arc<AtomicBool>) -> GpsBatches {
        let (tx, rx) = std::sync::mpsc::channel();
        let cache = geo::gps_cache_path();
        std::thread::spawn(move || geo::index_files(files, cache, tx, stop));
        rx
    }

    fn pictures(&self) -> Pictures {
        Arc::new(imaging::load_source)
    }

    fn export(&self, task: Task, jobs: Vec<crate::commands::ExportJob>, dir: PathBuf, opts: crate::export::Options, with_location: bool) {
        self.run_task(task, tr!("fail-export"), move |step| {
            // the logo is read once for all the pictures
            let logo = match opts.signature.as_ref().filter(|s| s.use_logo).and_then(|s| s.logo.as_deref()) {
                Some(path) => match crate::export::load_logo(path) {
                    Ok(l) => Some(l),
                    Err(e) => return Report::failed(tr!("export-nothing", error = e.to_string())),
                },
                None => None,
            };
            let mut ok = 0;
            let mut failed: Vec<String> = vec![];
            let mut stopped = false;
            for (i, (key, name, edits, place)) in jobs.iter().enumerate() {
                if !step(i, jobs.len()) {
                    stopped = true;
                    break;
                }
                let stem = name.rsplit_once('.').map(|(s, _)| s).unwrap_or(name);
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    let gps = if with_location { geo::effective(place.as_ref(), geo::file_gps(key)) } else { None };
                    crate::export::export_one(key, edits, &dir, stem, &opts, gps, logo.as_ref())
                }))
                .unwrap_or_else(|_| Err("unexpected error".into()));
                match result {
                    Ok(_) => ok += 1,
                    Err(e) => failed.push(format!("{name}: {e}")),
                }
            }
            let mut msg = tr!("export-result", ok = ok, total = jobs.len(), path = dir.display().to_string());
            if stopped {
                msg.push_str(&format!("\n{}", tr!("export-stopped")));
            }
            if !failed.is_empty() {
                msg.push_str(&format!("\n\n{}", tr!("export-failed-list")));
                for f in failed.iter().take(10) {
                    msg.push_str("\n• ");
                    msg.push_str(f);
                }
                if failed.len() > 10 {
                    let more = failed.len() - 10;
                    msg.push_str(&format!("\n{}", tr!("export-more", n = more)));
                }
            }
            let report = if failed.is_empty() { Report::done(msg) } else { Report::failed(msg) };
            if ok > 0 { report.showing(dir) } else { report }
        });
    }

    fn trash(&self, task: Task, files: Vec<String>) {
        self.run_task(task, tr!("fail-trash"), move |step| library::move_to_trash(&files, step));
    }

    fn email(&self, task: Task, jobs: Vec<crate::commands::ExportJob>, subject: String, with_location: bool, opts: crate::export::Options) {
        self.run_task(task, tr!("fail-email"), move |step| crate::mail::email(&jobs, &subject, with_location, &opts, step));
    }

    fn make_gallery(&self, task: Task, items: Vec<crate::gallery::Item>, parent: PathBuf, opts: crate::gallery::Options) {
        self.run_task(task, tr!("fail-gallery"), move |step| match crate::gallery::write(&parent, &items, &opts, step) {
            Ok((dir, failed)) => {
                let index = dir.join("index.html");
                let _ = crate::library_view::open_with_system(&index);
                let mut m = tr!("gallery-ready", path = index.display().to_string());
                if failed > 0 {
                    m.push_str(&format!("\n\n{}", tr!("gallery-left-out", n = failed)));
                }
                Report::done(m).showing(index)
            }
            Err(e) if e == library::STOPPED => Report::done(tr!("gallery-stopped")),
            Err(e) => Report::failed(tr!("gallery-failed", error = e.to_string())),
        });
    }

    fn make_movie(&self, task: Task, pictures: Vec<(String, Vec<EditOp>)>, out: PathBuf, size: (u32, u32), settings: SlideSettings) {
        self.run_task(task, tr!("fail-movie"), move |step| {
            let progress = |f: f32| step((f * 1000.0) as usize, 1000);
            match crate::movie::make(&out, &pictures, size, &settings, &progress) {
                Ok(()) => Report::done(tr!("movie-ready", path = out.display().to_string())).showing(out),
                Err(e) if e == library::STOPPED => Report::done(tr!("movie-stopped")),
                Err(e) => Report::failed(tr!("movie-failed", error = e.to_string())),
            }
        });
    }

    fn make_collage(&self, task: Task, job: crate::collage::Job, out: PathBuf, px_long: u32) {
        self.run_task(task, tr!("fail-collage"), move |_| {
            match crate::collage::save(&job, &out, px_long, &imaging::load_source) {
                Ok(files) if files.len() > 1 => Report::done(tr!("collage-saved-many", n = files.len(), path = out.display().to_string())).showing(out),
                Ok(_) => Report::done(tr!("collage-saved", path = out.display().to_string())).showing(out),
                Err(e) => Report::failed(tr!("collage-save-failed", error = e.to_string())),
            }
        });
    }

    fn import_sources(&self) -> Vec<(String, PathBuf)> {
        crate::import::removable_sources()
    }

    fn list_import(&self, source: PathBuf, library: Vec<(String, u64)>) {
        let send = self.sender();
        std::thread::spawn(move || {
            let (items, truncated) = std::panic::catch_unwind(|| crate::import::list(&source, &library)).unwrap_or_default();
            send(Done::ImportList { source, items, truncated });
        });
    }

    fn import(&self, files: Vec<String>, dest: PathBuf, stop: Arc<AtomicBool>) {
        let send = self.sender();
        std::thread::spawn(move || {
            let progress = |done, total| send(Done::ImportProgress { done, total });
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| crate::import::copy_into(&files, &dest, &progress, &stop)));
            let done = match result {
                Ok(Ok(r)) => Done::ImportDone {
                    summary: crate::labels::import_summary(&r, &dest),
                    copied: r.copied.len(),
                    problems: !r.failed.is_empty() || r.stopped,
                    dest,
                },
                Ok(Err(e)) => Done::ImportDone { dest, summary: e, copied: 0, problems: true },
                Err(_) => Done::ImportDone { dest, summary: tr!("import-unexpected"), copied: 0, problems: true },
            };
            send(done);
        });
    }

    fn analyse(&self, files: Vec<FileStamp>) {
        self.analysis.set_pictures(files);
    }

    fn pause_analysis(&self, paused: bool) {
        self.analysis.set_paused(paused);
    }

    fn model_packs(&self) -> Vec<crate::analysis::PackState> {
        crate::analysis::packs()
    }

    fn download_pack(&self, task: Task, id: String) {
        self.run_task(task, tr!("fail-download"), move |step| crate::analysis::download_pack(&id, step));
    }

    fn remove_pack(&self, id: &str) -> Result<(), String> {
        crate::analysis::remove_pack(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn local() -> (LocalLibrary, Receiver<Done>) {
        let (tx, rx) = std::sync::mpsc::channel();
        (LocalLibrary::new(tx, Arc::new(|| {})), rx)
    }

    fn next(rx: &Receiver<Done>) -> Done {
        rx.recv_timeout(Duration::from_secs(20)).expect("an answer")
    }

    #[test]
    fn local_answers_jobs_and_scans_on_the_channel() {
        let (lib, rx) = local();
        let key = format!("{}3", imaging::SAMPLE_PREFIX);
        lib.request(Job::Thumb { id: 7, key: key.clone(), dim: library::THUMB_DIM }, false);
        match next(&rx) {
            Done::Thumb { id, key: k, img, .. } => assert!(id == 7 && k == key && img.is_some()),
            _ => panic!("expected a thumbnail"),
        }
        // a missing picture still gets an answer, so the window never waits forever
        lib.request(Job::Big { id: 1, key: "/no/such/picture.jpg".into() }, true);
        assert!(matches!(next(&rx), Done::Big { id: 1, img: None, .. }));
        lib.scan(vec!["/no/such/folder".into()], vec![], vec![]);
        assert!(matches!(next(&rx), Done::Scan(found, false, inis) if found.is_empty() && inis.is_empty()));
        assert!(lib.pictures()(&key, 50).is_some_and(|i| i.width().max(i.height()) == 50));
        assert!(lib.camera_info(&key).is_none() && lib.source_dims("/no/such/picture.jpg").is_none());
    }

    /// The AI features' part answers too, with nothing to do while no network is installed.
    #[test]
    fn analysis_and_packs_answer() {
        let (lib, rx) = local();
        lib.analyse(vec![("/no/such/picture.jpg".into(), 1, 2)]);
        assert!(matches!(next(&rx), Done::Analysis(p) if !p.active() && !p.paused));
        lib.pause_analysis(true);
        assert!(matches!(next(&rx), Done::Analysis(p) if p.paused));
        assert!(lib.remove_pack("no-such-pack").is_err());
        lib.download_pack(Task { id: 3, stop: Arc::new(AtomicBool::new(false)) }, "no-such-pack".into());
        assert!(matches!(next(&rx), Done::Report(r) if r.task == Some(3) && !r.ok));
        assert_eq!(lib.model_packs().len(), crate::ai::models::manifest().packs.len());
    }

    #[test]
    fn a_failed_collage_is_reported() {
        let (lib, rx) = local();
        // a folder that does not exist: the PNG can't be written
        let job = crate::collage::Job {
            pages: vec![crate::collage::Page { size: [100.0, 80.0], shapes: vec![] }],
            background: Default::default(),
            bg_picture: None,
            pictures: vec![(format!("{}1", imaging::SAMPLE_PREFIX), vec![])],
        };
        lib.make_collage(Task { id: 7, stop: Arc::new(AtomicBool::new(false)) }, job, PathBuf::from("/no/such/folder/c.png"), 400);
        assert!(matches!(next(&rx), Done::Report(r) if r.task == Some(7) && !r.ok && r.text.starts_with("Could not save the collage")));
    }

    /// The window's import path: list a card, copy what is new, report progress and the end.
    #[test]
    fn import_lists_and_copies_in_the_background() {
        let dir = std::env::temp_dir().join(format!("albumcraft-backend-import-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let card = dir.join("card/DCIM/100CAM");
        std::fs::create_dir_all(&card).unwrap();
        for i in 0..3 {
            image::DynamicImage::ImageRgba8(imaging::sample_image(i + 1, 60, 40)).to_rgb8().save(card.join(format!("IMG_{i}.JPG"))).unwrap();
        }
        let (lib, rx) = local();
        lib.list_import(dir.join("card"), vec![]);
        let Done::ImportList { items, truncated, .. } = next(&rx) else { panic!("expected the list") };
        assert_eq!(items.len(), 3);
        assert!(!truncated);
        let dest = dir.join("Trip");
        lib.import(items.iter().map(|c| c.path.clone()).collect(), dest.clone(), Arc::new(AtomicBool::new(false)));
        let mut progress = 0;
        loop {
            match next(&rx) {
                Done::ImportProgress { done, total } => {
                    assert_eq!(total, 3);
                    progress = done;
                }
                Done::ImportDone { dest: d, copied, problems, .. } => {
                    assert_eq!((d, copied, problems), (dest.clone(), 3, false));
                    break;
                }
                _ => panic!("unexpected message"),
            }
        }
        assert_eq!(progress, 3);
        assert_eq!(std::fs::read_dir(&dest).unwrap().count(), 3);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
