//! XMP in the window: the notes read from the pictures' XMP after each scan (`xmp_sync`), and
//! "Write notes to XMP" for the selected pictures.

use crate::app::App;
use crate::i18n::tr;
use crate::xmp_sync::{self, Seen};
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

impl App {
    /// Read the XMP of the library's pictures that changed since it was last seen, in the
    /// background (after each scan); the batches arrive as `Done::Xmp`.
    pub fn start_xmp_index(&mut self) {
        if let Some(stop) = self.xmp_index.take() {
            stop.store(true, Ordering::Relaxed);
        }
        let files: Vec<crate::geo::FileStamp> = self.photos.iter().filter(|p| !p.is_sample()).map(|p| (p.key.clone(), p.modified, p.bytes)).collect();
        if files.is_empty() {
            return;
        }
        // read again: what was seen with the levels above its tags taken as tags
        let seen: HashMap<_, _> = self.store.xmp_seen.iter().filter(|(_, s)| !xmp_sync::has_ancestors(&s.notes)).map(|(k, s)| (k.clone(), (s.file, s.sidecar))).collect();
        let stop = Arc::new(AtomicBool::new(false));
        self.lib.index_xmp(files, seen, stop.clone());
        self.xmp_index = Some(stop);
    }

    /// Apply what the index read: the fields changed in another program since last seen (all
    /// of them, only filling in, the first time).
    pub fn on_xmp(&mut self, batch: Vec<xmp_sync::Read>) {
        let mut changed = 0;
        let mut regions_added = false;
        for read in batch {
            let Some(&id) = self.key_index.get(&read.key) else { continue };
            let notes = read.notes.unwrap_or_default();
            let old = self.store.xmp_seen.get(&read.key).map(|s| s.notes.clone());
            let meta = self.store.meta.entry(read.key.clone()).or_default();
            let edits_before = meta.edits.clone();
            let touched = xmp_sync::apply(meta, &notes, old.as_ref());
            // named face regions: faces of those people (what the user decided about a face wins)
            if !notes.regions.is_empty() {
                let known = self.face_groups.index.clone();
                let empty = crate::faces::FaceIndex::default();
                let index = known.as_deref().unwrap_or(&empty);
                let mut added = false;
                for (name, rect) in &notes.regions {
                    added |= crate::faces::import_face(&mut self.store, index, &read.key, *rect, name);
                }
                regions_added |= added;
            }
            let meta = self.store.meta.entry(read.key.clone()).or_default();
            let edits_changed = meta.edits != edits_before;
            if meta.is_empty() {
                self.store.meta.remove(&read.key);
            }
            // the hierarchy: for every path the XMP has, the first time or when its tags changed
            if old.as_ref().is_none_or(|o| o.tags != notes.tags)
                && let Some(paths) = &notes.tags
            {
                xmp_sync::learn_hierarchy(&mut self.store.tag_parents, paths);
            }
            if touched {
                changed += 1;
                if edits_changed {
                    self.refresh_thumb(id);
                }
            }
            if notes.is_empty() && old.is_none() {
                continue;
            }
            self.store.xmp_seen.insert(read.key, Seen { file: read.file, sidecar: read.sidecar, notes });
            self.dirty = true;
        }
        if regions_added {
            self.dirty = true;
            self.people_changed();
        }
        if changed > 0 {
            self.geo.place_rev += 1;
            self.toast(tr!("xmp-read", n = changed));
        }
    }

    /// Write the notes of these pictures into their XMP, as a task (only when the user asks).
    pub fn write_xmp(&mut self, ids: &[usize]) {
        let jobs: Vec<(String, crate::xmp::XmpNotes)> = ids
            .iter()
            .filter_map(|&id| self.photos.get(id))
            .filter(|p| !p.is_sample())
            .map(|p| (p.key.clone(), crate::commands::xmp_notes_of(&self.store, &p.key)))
            .collect();
        if jobs.is_empty() {
            return;
        }
        let task = self.start_task(tr!("xmp-task", n = jobs.len()));
        self.lib.write_xmp(task, jobs, self.store.xmp_embedded);
    }
}
