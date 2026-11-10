//! The fixed, private demo state behind `--screenshot` (README pictures and developer checks):
//! sample pictures with a few picks, ratings, an album and, by `ALBUMCRAFT_SHOT_VIEW`, one of
//! the views opened. Nothing here is saved.

use crate::app::*;
use crate::imaging::{self, EditOp};
use crate::library::{self, Album, Flag};

impl App {
    pub(crate) fn demo_setup(&mut self) {
        // A fixed, private demo state for the automatic picture.
        self.set_library(library::sample_folders(), true);
        self.store.albums.push(Album { name: "Best of the year".into(), keys: vec![] });
        for (i, p) in [0usize, 3, 10, 17].iter().enumerate() {
            if *p < self.photos.len() {
                let key = self.photos[*p].key.clone();
                let m = self.store.meta.entry(key.clone()).or_default();
                m.flag = Flag::Pick;
                m.rating = [5, 4, 3, 4][i];
                if i < 3 {
                    self.store.albums[0].keys.push(key);
                }
            }
        }
        // one rejected picture, to show the mark
        if let Some(p) = self.photos.get(6) {
            self.store.meta.entry(p.key.clone()).or_default().flag = Flag::Reject;
        }
        let shot_view = std::env::var("ALBUMCRAFT_SHOT_VIEW").unwrap_or_default();
        if self.photos.len() > 5 {
            let k = self.photos[5].key.clone();
            self.store.meta.entry(k).or_default().people.push("Grandma".into());
            if shot_view == "tray" {
                // a clicked folder with nothing held: the whole folder in the tray
                self.pick_set(TraySet::Folder(1));
            } else {
                self.selection.extend([1usize, 2, 4]);
                self.held.push(9);
            }
        }
        // Developer aid: ALBUMCRAFT_SHOT_VIEW=edit pictures the editor instead of the library.
        if shot_view == "compare" && self.photos.len() > 2 {
            let k = self.photos[0].key.clone();
            let m = self.store.meta.entry(k).or_default();
            m.edits.push(EditOp::Adjust(crate::develop::Adjust {
                temp: 18.0,
                exposure: 0.3,
                shadows: 30.0,
                contrast: 15.0,
                vibrance: 25.0,
                clarity: 20.0,
                ..Default::default()
            }));
            self.open_editor(0);
            self.edit.compare = CompareMode::BeforeAfter;
            self.edit.tab = Tab::Edit;
            self.edit.advanced = true;
        }
        if shot_view == "import" {
            self.demo_import();
        }
        if shot_view == "print" && self.photos.len() > 5 {
            self.selection = [0usize, 1, 2, 3, 5].into_iter().collect();
            self.held.clear();
            self.store.print.layout = crate::print::PrintLayout::Cm9x13;
            self.open_print();
        }
        if matches!(shot_view.as_str(), "collage" | "sheet") && self.photos.len() > 12 {
            self.selection = (0..12usize).collect();
            self.held.clear();
            if shot_view == "collage" {
                self.store.collage.style = std::env::var("ALBUMCRAFT_SHOT_STYLE").ok().and_then(|s| crate::collage::Style::from_id(&s)).unwrap_or(crate::collage::Style::Pile);
                self.store.collage.border = self.store.collage.style.natural_border();
                self.store.collage.background = crate::collage::Background { kind: crate::collage::BgKind::Gradient, color: [250, 240, 222], color2: [196, 160, 128], ..Default::default() };
            }
            self.open_collage(shot_view == "sheet");
        }
        if shot_view == "props" {
            // a temporary folder with one photo carrying camera information
            let dir = std::env::temp_dir().join("albumcraft-props-demo");
            let _ = std::fs::create_dir_all(&dir);
            if crate::exifinfo::write_demo_jpeg(&dir.join("P8184048.JPG")).is_ok() {
                let (found, _) = library::scan_roots(&[dir.to_string_lossy().to_string()], 10);
                self.set_library(found, false);
                if !self.photos.is_empty() {
                    // a place and tags, and every section open, to show the whole panel
                    let key = self.photos[0].key.clone();
                    let m = self.store.meta.entry(key).or_default();
                    m.place = Some(crate::geo::Place::Set { lat: 46.0717, lon: 24.7928, name: "Sighișoara".into() });
                    m.tags = vec!["Travel".into(), "Old town".into()];
                    self.open_editor(0);
                    self.store.info_open = true;
                    self.edit.tag_buf = "o".into();
                }
            }
        }
        if shot_view == "zoom" && self.photos.len() > 25 {
            self.open_editor(25);
            self.edit.zoom = Some(2.0);
        }
        if shot_view == "edit" && self.photos.len() > 2 {
            let k = self.photos[2].key.clone();
            let m = self.store.meta.entry(k).or_default();
            m.caption = "Evening by the water".into();
            m.edits.push(EditOp::Effect(imaging::Effect::Warmify));
            m.edits.push(EditOp::AutoContrast);
            m.edits.push(EditOp::Look { look: crate::looks::Look::Vivid, amount: 0.6 });
            self.open_editor(2);
            self.store.info_open = true;
        }
        if shot_view == "effects" && self.photos.len() > 4 {
            let k = self.photos[4].key.clone();
            let m = self.store.meta.entry(k).or_default();
            m.edits.push(EditOp::Decor(crate::decor::Decor::Border { width: 0.05, bottom: 0.15, color: [250, 248, 242] }));
            m.edits.push(EditOp::Decor(crate::decor::Decor::Text(crate::decor::TextOverlay { text: "Summer 2009".into(), anchor: 7, size: 0.07, color: [40, 40, 40], halo: false })));
            self.open_editor(4);
            self.edit.tab = Tab::Effects;
        }
        if shot_view == "slideshow" {
            let list: Vec<usize> = (0..self.photos.len()).collect();
            self.start_slideshow(list, 0);
        }
        if shot_view == "movie" {
            self.selection = [0usize, 1, 2, 3].into_iter().collect();
            self.dialog = Some(Dialog::Movie { hd: true });
        }
        if shot_view == "presets" && self.photos.len() > 3 {
            let k = self.photos[3].key.clone();
            self.store.meta.entry(k).or_default().edits.push(EditOp::Look { look: crate::looks::Look::Polaroid, amount: 0.8 });
            self.open_editor(3);
            self.edit.tab = Tab::Presets;
        }
        if matches!(shot_view.as_str(), "map" | "place" | "pin" | "mapscope") {
            // ALBUMCRAFT_SHOT_ONLINE=1: show the online map (the screenshot saves nothing)
            if std::env::var_os("ALBUMCRAFT_SHOT_ONLINE").is_some() {
                self.store.online_maps = Some(true);
            }
            // sample pictures spread over a few places, several at each
            let spots = [(44.8378, -0.5792), (48.8566, 2.3522), (43.2965, 5.3698), (41.9028, 12.4964), (51.5072, -0.1276), (40.4168, -3.7038), (52.52, 13.405), (45.764, 4.8357)];
            for id in 0..self.photos.len().min(28) {
                let (lat, lon) = spots[id % spots.len()];
                let jitter = (id / spots.len()) as f64 * 0.01;
                let k = self.photos[id].key.clone();
                self.store.meta.entry(k).or_default().place = crate::geo::LatLon::new(lat + jitter, lon + jitter).map(|at| crate::geo::Place::set(at, ""));
            }
            if shot_view == "map" {
                self.open_places();
            } else if shot_view == "mapscope" {
                // one folder on the map
                if let Some(f) = self.folders.first().cloned() {
                    self.show_set_on_map(f.name.clone(), &f.photos);
                }
            } else {
                let n = self.photos.len();
                self.start_placing((28..n.min(31)).collect());
                if shot_view == "pin" {
                    if let Some(at) = crate::geo::LatLon::new(44.8412, -0.5695) {
                        self.set_pin(at, None);
                    }
                } else {
                    self.type_map_search("bord");
                }
            }
        }
        if shot_view == "search" {
            // places in Scotland and Spain, a few tags, "sc" typed, one tag already chosen
            let spots = [(56.4907, -4.2026), (55.9533, -3.1883), (57.4778, -4.2247), (43.3614, -5.8593), (43.4623, -3.8100)];
            let tags = ["Sea", "Castle", "Scenery", "Beach", "Hiking", "Old town"];
            for id in 0..self.photos.len().min(25) {
                let (lat, lon) = spots[id % spots.len()];
                let k = self.photos[id].key.clone();
                let m = self.store.meta.entry(k).or_default();
                m.place = crate::geo::LatLon::new(lat, lon).map(|at| crate::geo::Place::set(at, ""));
                m.tags = vec![tags[id % tags.len()].into(), tags[(id / 2) % tags.len()].into()];
            }
            self.search_terms = vec![SearchTerm::Tag("Sea".into())];
            self.search = "sc".into();
            self.search_popup = true;
        }
        if shot_view == "geometry" && self.photos.len() > 5 {
            let k = self.photos[5].key.clone();
            let m = self.store.meta.entry(k).or_default();
            m.edits.push(EditOp::Straighten(2.5));
            m.edits.push(EditOp::Perspective { vertical: 0.6, horizontal: 0.0 });
            self.open_editor(5);
            self.edit.advanced = true;
            // the grid shown while the geometry sliders move
            self.edit.grid_until = f64::MAX;
        }
        if shot_view == "crop" && self.photos.len() > 5 {
            self.open_editor(5);
            self.edit.aspect = CropAspect::ThreeTwo;
            self.start_crop();
        }
        if shot_view == "rescan" && self.photos.len() > 12 {
            // a scan that finds one folder less while a picture is open and others are selected:
            // the editor stays on the same picture, the selection on the same pictures
            self.open_editor(12);
            self.selection.extend([10usize, 11, 2]);
            let key = |app: &App, id: usize| app.photos.get(id).map(|p| p.key.clone());
            let (open, chosen): (_, Vec<_>) = (key(self, 12), [10usize, 11].iter().map(|&id| key(self, id)).collect());
            let gone = self.folders.first().map(|f| f.path.clone()).unwrap_or_default();
            let rest: Vec<_> = library::sample_folders().into_iter().filter(|(path, _)| *path != gone).collect();
            self.set_library(rest, true);
            let kept: Vec<_> = self.selection.iter().map(|&id| key(self, id)).collect();
            let ok = self.view == View::Edit && key(self, self.edit.id) == open && kept.len() == 3 && chosen.iter().all(|k| kept.contains(k));
            eprintln!("rescan check: {}", if ok { "kept" } else { "LOST" });
        }
        if shot_view == "scroll" {
            // large pictures, scrolled to the last one: rows above the window are skipped
            self.store.thumb_size = 256.0;
            self.scroll_to_photo = self.photos.len().checked_sub(1);
        }
        if shot_view == "notices" {
            // a running task, a result, an offer to undo and an error
            let _ = self.start_task("Exporting 24 picture(s)");
            if let Some(t) = self.tasks.last_mut() {
                (t.done, t.total) = (9, 24);
            }
            self.notices.push(Notice { text: "The web gallery is ready:\n/home/me/Pictures/Summer by the sea/index.html".into(), error: false, born: 0.0, life: None, action: Some(NoticeAction::Show("/home/me/Pictures".into())) });
            self.notices.push(Notice { text: "Deleted the album \"Best of the year\". Its pictures are untouched.".into(), error: false, born: 0.0, life: None, action: Some(NoticeAction::Undo(Undo::Edits(vec![]))) });
            self.error("Your library could not be saved: no space left on the disk.\nYour latest changes are only in this window for now; AlbumCraft keeps trying.");
        }
        if shot_view == "dialog" {
            self.dialog = Some(Dialog::NewAlbum { name: "Best of 2009".into() });
        }
        if shot_view == "maps" {
            self.dialog = Some(Dialog::OnlineMaps);
        }
    }
}
