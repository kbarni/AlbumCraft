//! `ImmichLibrary`: the pictures of an Immich server as the window's library (see
//! `immich-integration.md` in the project notes; core's `immich.rs` does the server side).
//!
//! It wraps the window's `LocalLibrary` and hands it most of the work: since the core's
//! picture readers understand server keys (`remote.rs`), thumbnails, previews, export, print
//! and collages work unchanged. What differs:
//! - the notes: the window keeps one `Store`; while the server is shown, its library part
//!   (notes, albums, folders) is the server's, saved in the connection's own `library.json`,
//!   and this computer's part waits here (`stash`) and is saved with the settings as usual;
//! - the pictures come from the sync stream (`Done::Scan` / `Done::ServerNotes`), not a scan;
//! - files can't be moved, renamed or deleted, and nothing is analysed on this computer
//!   (Immich has its own faces and search).

use crate::backend::{GpsBatches, Library, Pictures, Task};
use crate::exifinfo::CameraInfo;
use crate::geo::{FileStamp, LatLon};
use crate::i18n::tr;
use crate::imaging::EditOp;
use crate::immich::{Account, Engine, Status};
use crate::library::{self, Done, Job, LibraryPart, Loaded, Report, Store};
use crate::slides::SlideSettings;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::sync::Arc;
use std::thread::JoinHandle;

/// The library of this computer's folders, which can show an Immich server's pictures instead.
pub struct ImmichLibrary {
    /// This computer's library, which does the work that isn't the server's.
    local: Box<dyn Library>,
    /// The server, while its pictures are shown.
    server: Option<Server>,
    send: Arc<dyn Fn(Done) + Send + Sync>,
    saving: Option<JoinHandle<Result<(), String>>>,
    saved: Option<Result<(), String>>,
}

struct Server {
    engine: Arc<Engine>,
    /// This computer's notes, albums and folders while the server's are in the window's `Store`.
    stash: LibraryPart,
    stop: Arc<AtomicBool>,
}

impl ImmichLibrary {
    pub fn new(local: Box<dyn Library>, tx: Sender<Done>, wake: library::Wake) -> ImmichLibrary {
        let send: Arc<dyn Fn(Done) + Send + Sync> = Arc::new(move |d| {
            let _ = tx.send(d);
            wake();
        });
        ImmichLibrary { local, server: None, send, saving: None, saved: None }
    }

    /// Show the server's pictures: its notes go into `store` (this computer's are kept aside)
    /// and the sync starts.
    fn enter(&mut self, store: &mut Store, account: Account) -> Result<(), String> {
        let engine = Engine::open(account)?;
        let mut part = match engine.notes_path().map(|p| LibraryPart::load_from(&p)) {
            Some(Ok(p)) => p,
            Some(Err(e)) => return Err(tr!("immich-notes-unreadable", error = e)),
            None => LibraryPart::default(),
        };
        // the server is the library's one folder
        part.roots = vec![engine.root()];
        part.excluded.clear();
        store.swap_library(&mut part);
        crate::remote::set(Some(engine.clone()));
        let stop = Arc::new(AtomicBool::new(false));
        engine.clone().run(self.send.clone(), stop.clone());
        self.server = Some(Server { engine, stash: part, stop });
        Ok(())
    }

    /// Back to this computer's folders: the server's notes are saved, this computer's put back.
    fn leave(&mut self, store: &mut Store) {
        if self.server.is_none() {
            return;
        }
        self.save(store, true);
        let _ = self.take_saved(true);
        if let Some(mut s) = self.server.take() {
            s.stop.store(true, Ordering::Relaxed);
            crate::remote::set(None);
            store.swap_library(&mut s.stash);
        }
    }

    fn not_here(&self, task: Task) {
        let mut r = Report::failed(tr!("immich-not-here"));
        r.task = Some(task.id);
        (self.send)(Done::Report(r));
    }

    /// The server's part of `store` as bytes for its `library.json` (the store keeps it).
    fn server_snapshot(store: &mut Store) -> Result<Vec<u8>, String> {
        let mut part = LibraryPart::default();
        store.swap_library(&mut part);
        let bytes = part.snapshot();
        store.swap_library(&mut part);
        bytes
    }
}

impl Library for ImmichLibrary {
    fn load(&mut self) -> Loaded {
        self.local.load()
    }

    fn save(&mut self, store: &mut Store, now: bool) {
        let Some(server) = &mut self.server else { return self.local.save(store, now) };
        // the server's notes, in the connection's folder
        if let Some(h) = self.saving.take() {
            let r = h.join().unwrap_or_else(|_| Err("the save stopped unexpectedly".into()));
            if r.is_err() {
                self.saved = Some(r);
            }
        }
        match (server.engine.notes_path(), Self::server_snapshot(store)) {
            (Some(path), Ok(bytes)) if now => {
                if let Err(e) = library::write_library(&path, &bytes) {
                    self.saved = Some(Err(e));
                }
            }
            (Some(path), Ok(bytes)) => self.saving = Some(std::thread::spawn(move || library::write_library(&path, &bytes))),
            (None, _) => self.saved = Some(Err(tr!("backend-no-data-dir"))),
            (_, Err(e)) => self.saved = Some(Err(e)),
        }
        // the settings with this computer's notes, in library.json as always
        store.swap_library(&mut server.stash);
        self.local.save(store, now);
        store.swap_library(&mut server.stash);
    }

    fn relocate(&mut self, store: &mut Store, folder: Option<PathBuf>) -> Result<library::Relocated, String> {
        if self.server.is_some() {
            return Err(tr!("immich-switch-first"));
        }
        self.local.relocate(store, folder)
    }

    fn open_library_in(&mut self, folder: Option<PathBuf>) -> Result<Store, String> {
        if self.server.is_some() {
            return Err(tr!("immich-switch-first"));
        }
        self.local.open_library_in(folder)
    }

    fn saving(&self) -> bool {
        self.saving.is_some() || self.local.saving()
    }

    fn take_saved(&mut self, wait: bool) -> Option<Result<(), String>> {
        if let Some(r) = self.saved.take() {
            return Some(r);
        }
        if self.saving.as_ref().is_some_and(|h| wait || h.is_finished())
            && let Some(h) = self.saving.take()
        {
            let r = h.join().unwrap_or_else(|_| Err("the save stopped unexpectedly".into()));
            if r.is_err() {
                return Some(r);
            }
        }
        self.local.take_saved(wait)
    }

    fn tidy(&self) {
        self.local.tidy();
    }

    fn cache_sizes(&self) {
        self.local.cache_sizes();
    }

    fn clear_cache(&self, which: library::Cache) {
        self.local.clear_cache(which);
    }

    fn scan(&self, roots: Vec<String>, excluded: Vec<String>, ini_done: Vec<String>) {
        match &self.server {
            // the engine answers with Done::Scan after its next sync (or at once when offline)
            Some(s) => s.engine.poke(),
            None => self.local.scan(roots, excluded, ini_done),
        }
    }

    fn cached(&self, roots: Vec<String>, excluded: Vec<String>) {
        let Some(s) = &self.server else { return self.local.cached(roots, excluded) };
        let (folders, known) = s.engine.folders();
        if !folders.is_empty() {
            (self.send)(Done::Cached(folders, known));
        }
    }

    fn save_catalogue(&self, cat: crate::catalogue::Catalogue, now: bool) {
        // for a server, the mirror is the catalogue; the cache file is this computer's folders'
        if self.server.is_none() {
            self.local.save_catalogue(cat, now);
        }
    }

    fn request(&self, job: Job, urgent: bool) {
        self.local.request(job, urgent);
    }

    fn source_dims(&self, key: &str) -> Option<(u32, u32)> {
        self.local.source_dims(key)
    }

    fn camera_info(&self, key: &str) -> Option<CameraInfo> {
        self.local.camera_info(key)
    }

    fn file_gps(&self, key: &str) -> Option<LatLon> {
        self.local.file_gps(key)
    }

    fn index_gps(&self, files: Vec<FileStamp>, stop: Arc<AtomicBool>) -> GpsBatches {
        self.local.index_gps(files, stop)
    }

    fn pictures(&self) -> Pictures {
        self.local.pictures()
    }

    fn export(&self, task: Task, jobs: Vec<crate::commands::ExportJob>, dir: PathBuf, opts: crate::export::Options, with_location: bool) {
        self.local.export(task, jobs, dir, opts, with_location);
    }

    fn trash(&self, task: Task, files: Vec<String>) {
        if self.server.is_some() {
            return self.not_here(task);
        }
        self.local.trash(task, files);
    }

    fn move_path(&self, from: &str, to: &str) -> Result<(), String> {
        if self.server.is_some() {
            return Err(tr!("immich-not-here"));
        }
        self.local.move_path(from, to)
    }

    fn same_disk(&self, from: &str, to: &str) -> bool {
        self.server.is_none() && self.local.same_disk(from, to)
    }

    fn move_across(&self, task: Task, pairs: Vec<(String, String)>) {
        if self.server.is_some() {
            return self.not_here(task);
        }
        self.local.move_across(task, pairs);
    }

    fn delete_permanently(&self, task: Task, paths: Vec<String>) {
        if self.server.is_some() {
            return self.not_here(task);
        }
        self.local.delete_permanently(task, paths);
    }

    fn forget_thumbs(&self, keys: Vec<String>) {
        self.local.forget_thumbs(keys);
    }

    fn open_video(&self, key: &str, at: f64, max_dim: u32) -> crate::video::Playback {
        self.local.open_video(key, at, max_dim)
    }

    fn filmstrip(&self, key: &str, duration: f64, count: usize, max_dim: u32) -> Arc<AtomicBool> {
        self.local.filmstrip(key, duration, count, max_dim)
    }

    fn email(&self, task: Task, jobs: Vec<crate::commands::ExportJob>, subject: String, with_location: bool, opts: crate::export::Options) {
        self.local.email(task, jobs, subject, with_location, opts);
    }

    fn make_gallery(&self, task: Task, items: Vec<crate::gallery::Item>, parent: PathBuf, opts: crate::gallery::Options) {
        self.local.make_gallery(task, items, parent, opts);
    }

    fn make_movie(&self, task: Task, pictures: Vec<(String, Vec<EditOp>)>, out: PathBuf, size: (u32, u32), settings: SlideSettings) {
        self.local.make_movie(task, pictures, out, size, settings);
    }

    fn make_collage(&self, task: Task, job: crate::collage::Job, out: PathBuf, px_long: u32) {
        self.local.make_collage(task, job, out, px_long);
    }

    fn list_import(&self, source: PathBuf, library: Vec<(String, u64)>) {
        self.local.list_import(source, library);
    }

    fn import(&self, files: Vec<String>, dest: PathBuf, stop: Arc<AtomicBool>) {
        self.local.import(files, dest, stop);
    }

    fn import_sources(&self) -> Vec<(String, PathBuf)> {
        self.local.import_sources()
    }

    fn analyse(&self, files: Vec<FileStamp>) {
        // Immich analyses its own pictures (faces, smart search)
        self.local.analyse(if self.server.is_some() { vec![] } else { files });
    }

    fn pause_analysis(&self, paused: bool) {
        self.local.pause_analysis(paused);
    }

    fn model_packs(&self) -> Vec<crate::analysis::PackState> {
        self.local.model_packs()
    }

    fn download_pack(&self, task: Task, id: String) {
        self.local.download_pack(task, id);
    }

    fn remove_pack(&self, id: &str) -> Result<(), String> {
        self.local.remove_pack(id)
    }

    fn use_search_pack(&self, pack: Option<String>) {
        self.local.use_search_pack(pack);
    }

    fn looks_like(&self, pack: String, query: crate::smart_search::Query) {
        self.local.looks_like(pack, query);
    }

    fn propose_tags(&self, pack: String, asked: u64, examples: Arc<crate::tag_proposals::Examples>, targets: Vec<crate::tag_proposals::Picture>) {
        self.local.propose_tags(pack, asked, examples, targets);
    }

    fn remote_status(&self) -> Option<Status> {
        self.server.as_ref().map(|s| s.engine.status())
    }

    fn show_server(&mut self, store: &mut Store, account: Option<Account>) -> Result<(), String> {
        self.leave(store);
        match account {
            Some(a) => self.enter(store, a),
            None => Ok(()),
        }
    }
}
