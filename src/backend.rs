//! The `Library` interface: where the pictures and their notes live, as the window sees them.
//!
//! `App` never reads picture files, scans folders or writes `library.json` itself: it asks its
//! `Library`. Today there is one implementation, `LocalLibrary` (folders on this computer and
//! `library.json`, as always); a library on a server can implement the same interface later.
//!
//! Slow answers (scans, thumbnails, previews, exports…) come back as `Done` messages on the
//! channel given to the library, and the library wakes the window when one is ready. Quick ones
//! (a picture's size, its camera information) are answered directly.

use crate::exifinfo::CameraInfo;
use crate::geo::{self, FileStamp, LatLon};
use crate::imaging::{self, EditOp};
use crate::library::{self, Done, IniFolder, Job, Loaded, Pool, Store, Wake};
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

    /// Look for pictures in these folders; the result arrives as `Done::Scan`.
    fn scan(&self, roots: Vec<String>);
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
    /// The notes saved in a folder's `.picasa.ini` file (read only, never written).
    fn folder_ini(&self, folder: &str) -> Option<IniFolder>;
    /// Reads pictures for output made by the window (print pages, collages).
    fn pictures(&self) -> Pictures;

    /// Export edited copies into `dir`; a report arrives as `Done::Message`.
    fn export(&self, jobs: Vec<crate::commands::ExportJob>, dir: PathBuf, max_dim: u32, quality: u8, with_location: bool);
    /// Make a web gallery in a new folder inside `parent`; a report arrives as `Done::Message`.
    fn make_gallery(&self, items: Vec<crate::gallery::Item>, parent: PathBuf, opts: crate::gallery::Options);
    /// Make a movie file; progress arrives as `Done::Toast`, the report as `Done::Message`.
    fn make_movie(&self, pictures: Vec<(String, Vec<EditOp>)>, out: PathBuf, size: (u32, u32), settings: SlideSettings);
    /// Save a collage of these pictures as a PNG file; a report arrives as `Done::Message`.
    fn make_collage(&self, pictures: Vec<(String, Vec<EditOp>)>, out: PathBuf);
}

/// The pictures in folders on this computer, notes in `library.json` (`library::store_path`).
pub struct LocalLibrary {
    pool: Pool,
    tx: Sender<Done>,
    wake: Wake,
    saving: Option<JoinHandle<Result<(), String>>>,
    /// A save that finished (or failed) before it could go to another thread.
    saved: Option<Result<(), String>>,
}

impl LocalLibrary {
    /// `tx` receives the slow answers; `wake` is called after each one.
    pub fn new(tx: Sender<Done>, wake: Wake) -> LocalLibrary {
        let threads = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4).clamp(2, 6);
        let pool = Pool::new(threads, tx.clone(), wake.clone());
        LocalLibrary { pool, tx, wake, saving: None, saved: None }
    }

    /// Run `work` on its own thread (a panic becomes `failed`) and send what it returns.
    fn in_background(&self, failed: &'static str, work: impl FnOnce(&dyn Fn(Done)) -> String + Send + 'static) {
        let (tx, wake) = (self.tx.clone(), self.wake.clone());
        std::thread::spawn(move || {
            let send = |d: Done| {
                let _ = tx.send(d);
                wake();
            };
            let msg = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| work(&send))).unwrap_or_else(|_| failed.to_string());
            send(Done::Message(msg));
        });
    }
}

impl Library for LocalLibrary {
    fn load(&mut self) -> Loaded {
        Store::load()
    }

    fn save(&mut self, store: &mut Store, now: bool) {
        let job = library::store_path()
            .ok_or_else(|| "No folder for application data was found.".to_string())
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

    fn scan(&self, roots: Vec<String>) {
        let (tx, wake) = (self.tx.clone(), self.wake.clone());
        std::thread::spawn(move || {
            let (found, truncated) = std::panic::catch_unwind(|| library::scan_roots(&roots, library::SCAN_LIMIT)).unwrap_or_default();
            let _ = tx.send(Done::Scan(found, truncated));
            wake();
        });
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

    fn folder_ini(&self, folder: &str) -> Option<IniFolder> {
        library::read_picasa_ini(Path::new(folder))
    }

    fn pictures(&self) -> Pictures {
        Arc::new(imaging::load_source)
    }

    fn export(&self, jobs: Vec<crate::commands::ExportJob>, dir: PathBuf, max_dim: u32, quality: u8, with_location: bool) {
        self.in_background("Could not export: unexpected error.", move |_| {
            let mut ok = 0;
            let mut failed: Vec<String> = vec![];
            for (key, name, edits, place) in &jobs {
                let stem = name.rsplit_once('.').map(|(s, _)| s).unwrap_or(name);
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    let gps = if with_location { geo::effective(place.as_ref(), geo::file_gps(key)) } else { None };
                    crate::export::export_one(key, edits, &dir, stem, max_dim, quality, gps)
                }))
                .unwrap_or_else(|_| Err("unexpected error".into()));
                match result {
                    Ok(_) => ok += 1,
                    Err(e) => failed.push(format!("{name}: {e}")),
                }
            }
            let mut msg = format!("Exported {ok} of {} picture(s) to {}", jobs.len(), dir.display());
            if !failed.is_empty() {
                msg.push_str("\n\nNot exported:");
                for f in failed.iter().take(10) {
                    msg.push_str("\n• ");
                    msg.push_str(f);
                }
                if failed.len() > 10 {
                    msg.push_str(&format!("\n… and {} more", failed.len() - 10));
                }
            }
            msg
        });
    }

    fn make_gallery(&self, items: Vec<crate::gallery::Item>, parent: PathBuf, opts: crate::gallery::Options) {
        self.in_background("Could not make the web gallery: unexpected error.", move |_| match crate::gallery::write(&parent, &items, &opts) {
            Ok((dir, failed)) => {
                let index = dir.join("index.html");
                let _ = crate::library_view::open_with_system(&index);
                let mut m = format!("The web gallery is ready:\n{}", index.display());
                if failed > 0 {
                    m.push_str(&format!("\n\n{failed} picture(s) could not be read and were left out."));
                }
                m
            }
            Err(e) => format!("Could not make the web gallery: {e}"),
        });
    }

    fn make_movie(&self, pictures: Vec<(String, Vec<EditOp>)>, out: PathBuf, size: (u32, u32), settings: SlideSettings) {
        self.in_background("Could not make the movie: unexpected error", move |send| {
            let progress = |f: f32| send(Done::Toast(format!("Making the movie… {:.0} %", f * 100.0)));
            match crate::movie::make(&out, &pictures, size, &settings, &progress) {
                Ok(()) => format!("The movie is ready:\n{}", out.display()),
                Err(e) => format!("Could not make the movie: {e}"),
            }
        });
    }

    fn make_collage(&self, pictures: Vec<(String, Vec<EditOp>)>, out: PathBuf) {
        self.in_background("Could not make the collage: unexpected error.", move |_| {
            let pics: Vec<RgbaImage> =
                pictures.iter().filter_map(|(k, e)| imaging::load_source(k, 900).map(|i| imaging::apply_all(&i, e))).collect();
            match imaging::make_collage(&pics, 600).save(&out) {
                Ok(()) => format!("Collage saved to {}", out.display()),
                Err(e) => format!("Could not save the collage: {e}"),
            }
        });
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
        lib.request(Job::Thumb { id: 7, key: key.clone() }, false);
        match next(&rx) {
            Done::Thumb { id, key: k, img } => assert!(id == 7 && k == key && img.is_some()),
            _ => panic!("expected a thumbnail"),
        }
        // a missing picture still gets an answer, so the window never waits forever
        lib.request(Job::Big { id: 1, key: "/no/such/picture.jpg".into() }, true);
        assert!(matches!(next(&rx), Done::Big { id: 1, img: None, .. }));
        lib.scan(vec!["/no/such/folder".into()]);
        assert!(matches!(next(&rx), Done::Scan(found, false) if found.is_empty()));
        assert!(lib.pictures()(&key, 50).is_some_and(|i| i.width().max(i.height()) == 50));
        assert!(lib.camera_info(&key).is_none() && lib.source_dims("/no/such/picture.jpg").is_none());
    }

    #[test]
    fn a_failed_collage_is_reported() {
        let (lib, rx) = local();
        // a folder that does not exist: the PNG can't be written
        lib.make_collage(vec![(format!("{}1", imaging::SAMPLE_PREFIX), vec![])], PathBuf::from("/no/such/folder/c.png"));
        assert!(matches!(next(&rx), Done::Message(m) if m.starts_with("Could not save the collage")));
    }
}
