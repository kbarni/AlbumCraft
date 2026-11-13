//! The fixed, private demo state behind `--screenshot` (README pictures and developer checks):
//! sample pictures with a few picks, ratings, an album and, by `ALBUMCRAFT_SHOT_VIEW`, one of
//! the views opened. Nothing here is saved.

use crate::app::*;
use eframe::egui;
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
        // a caption on two folders, a description on the first
        let paths: Vec<String> = self.folders.iter().take(2).map(|f| f.path.clone()).collect();
        for (path, caption) in paths.iter().zip(["Weekend by the sea", "Hiking with the cousins"]) {
            self.store.folder_captions.insert(path.clone(), caption.into());
        }
        if let Some(path) = paths.first() {
            self.store.folder_notes.insert(path.clone(), "Two sunny days on the coast, before the storm.".into());
            match shot_view.as_str() {
                "foldertexts" => self.edit_folder_texts(&path.clone()),
                "renamefolder" => {
                    let name = self.folders[0].name.clone();
                    self.dialog = Some(Dialog::Rename { path: path.clone(), folder: true, name, error: String::new() });
                }
                "movefolder" => {
                    self.dialog = Some(Dialog::MoveTo { what: crate::file_actions::Moving::Folder(path.clone()), filter: String::new(), error: String::new() });
                }
                _ => {}
            }
        }
        if std::env::var("ALBUMCRAFT_SHOT_BAR").is_ok_and(|b| b == "tray") {
            self.store.bar_style = crate::library::BarStyle::Tray;
        }
        if self.photos.len() > 5 {
            let k = self.photos[5].key.clone();
            self.store.meta.entry(k).or_default().people.push("Grandma".into());
            if shot_view == "tray" {
                // a clicked folder: the whole folder in the tray
                self.pick_set(TraySet::Folder(1));
            } else {
                self.selection.extend([1usize, 2, 4]);
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
            self.edit.compare = CompareMode::TwoPhotos;
            self.edit.tab = Tab::Edit;
            self.edit.advanced = true;
        }
        if shot_view == "import" {
            self.demo_import();
        }
        if shot_view == "print" && self.photos.len() > 5 {
            self.selection = [0usize, 1, 2, 3, 5].into_iter().collect();
            self.store.print.layout = crate::print::PrintLayout::Cm9x13;
            self.open_print();
        }
        if matches!(shot_view.as_str(), "collage" | "sheet") && self.photos.len() > 12 {
            self.selection = (0..12usize).collect();
            if shot_view == "collage" {
                self.store.collage.style = std::env::var("ALBUMCRAFT_SHOT_STYLE").ok().and_then(|s| crate::collage::Style::from_id(&s)).unwrap_or(crate::collage::Style::Pile);
                self.store.collage.border = self.store.collage.style.natural_border();
                self.store.collage.background = crate::collage::Background { kind: crate::collage::BgKind::Gradient, color: [250, 240, 222], color2: [196, 160, 128], ..Default::default() };
            }
            self.open_collage(shot_view == "sheet");
        }
        if shot_view == "props" || shot_view == "proposals" {
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
                    if shot_view == "proposals" {
                        // proposed tags (fixed: the demo picture is not analysed)
                        self.edit.tag_buf.clear();
                        for section in ["properties", "location"] {
                            self.store.info_sections.insert(section.into(), false);
                        }
                        self.props.demo(["Architecture", "Tower", "Town square", "Sightseeing"].iter().map(|t| crate::tag_proposals::Proposal { tag: t.to_string(), score: 0.6 }).collect());
                    }
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
        if shot_view == "viewer" && self.photos.len() > 2 {
            self.open_editor(2);
            let list = self.edit.list.clone();
            self.start_viewer(list, 2);
        }
        if shot_view == "settings" {
            use crate::settings_view::SettingsSection;
            let wanted = std::env::var("ALBUMCRAFT_SHOT_SECTION").unwrap_or_default().to_lowercase();
            let section = SettingsSection::ALL.into_iter().find(|s| format!("{s:?}").to_lowercase() == wanted).unwrap_or_default();
            self.open_settings(section);
        }
        if shot_view == "export" {
            self.store.export_creator = "Ana Pérez".into();
            self.store.export_copyright = "© 2026 Ana Pérez".into();
            self.store.export_signature = crate::export::Signature { on: true, text: "© Ana Pérez".into(), ..Default::default() };
            self.open_export(vec![0, 1, 2, 3, 4, 5]);
            if let Some(e) = self.export.as_mut() {
                e.skip.insert(4);
                e.dest = "/home/ana/Pictures/Exports".into();
            }
        }
        if shot_view == "gallery" {
            self.selection = (0..self.photos.len().min(8)).collect();
            self.store.gallery.theme = crate::gallery::Theme::Instant;
            self.open_gallery_dialog();
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
        if shot_view == "looks" || shot_view == "similar" {
            // smart search with a fixed ranking: "sea" typed (the pictures tagged Sea match it
            // word for word, those that look like it follow), or the pictures like the first
            let tagged = [0usize, 2, 4];
            for &id in &tagged {
                if let Some(p) = self.photos.get(id) {
                    self.store.meta.entry(p.key.clone()).or_default().tags = vec!["Sea".into()];
                }
            }
            let ranked = [1usize, 5, 7, 3, 8, 12, 6, 20, 15, 22];
            let hits: Vec<crate::smart_search::Hit> =
                ranked.iter().enumerate().filter_map(|(i, &id)| self.photos.get(id).map(|p| crate::smart_search::Hit { key: p.key.clone(), score: 0.31 - i as f32 * 0.012 })).collect();
            let query = if shot_view == "looks" {
                self.search = "sea".into();
                crate::smart_search::Query::Text("sea".into())
            } else {
                let key = self.photos.first().map(|p| p.key.clone()).unwrap_or_default();
                self.search_terms = vec![SearchTerm::Like(key.clone())];
                crate::smart_search::Query::Like(key)
            };
            self.smart.demo("search-light", query, hits, 6);
            self.analysis = crate::analysis::Progress { done: 1240, total: 18300, paused: false };
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
        if shot_view == "recrop" && self.photos.len() > 5 {
            // a picture adjusted, cropped and turned: the sliders keep their values and the crop
            // tool shows the whole picture with its box
            let k = self.photos[5].key.clone();
            let m = self.store.meta.entry(k).or_default();
            m.edits.push(EditOp::Adjust(crate::develop::Adjust { exposure: 0.4, contrast: 20.0, ..Default::default() }));
            m.edits.push(EditOp::Crop([0.2, 0.15, 0.8, 0.75]));
            m.edits.push(EditOp::Look { look: crate::looks::Look::Vivid, amount: 0.7 });
            self.open_editor(5);
            self.edit.advanced = true;
            let kept = crate::steps::adjust_of(&self.edits(5)).exposure == 0.4;
            self.start_crop();
            eprintln!("recrop check: {}", if kept && self.edit.sel == Some([0.2, 0.15, 0.8, 0.75]) { "kept" } else { "LOST" });
        }
        if shot_view == "sort" {
            // highest rating first in every folder: the first folder's picks lead it
            self.store.sort = library::Sort::Rating;
            let rating = |app: &App, id: usize| app.meta(id).map(|m| m.rating).unwrap_or(0);
            let groups = self.groups();
            let ok = groups.iter().all(|(_, ids)| ids.windows(2).all(|w| rating(self, w[0]) >= rating(self, w[1])))
                && groups.first().and_then(|(_, ids)| ids.first()).is_some_and(|&id| rating(self, id) == 5);
            eprintln!("sort check: {}", if ok { "ok" } else { "WRONG" });
        }
        if shot_view == "cached" {
            // the catalogue cache is shown until the first scan's result, never after it
            let folders = library::sample_folders();
            let first = folders.first().and_then(|(_, files)| files.first()).map(|f| f.0.clone()).unwrap_or_default();
            let known = vec![(first.clone(), crate::catalogue::Details { dims: Some((40, 30)), taken: Some(7) })];
            self.store.roots = vec!["/demo".into()];
            let _ = self.tx.send(library::Done::Cached(folders.clone(), known.clone()));
            self.process_results();
            let shown = self.key_index.get(&first).copied();
            let ok_before = shown.is_some_and(|id| self.taken.get(&id) == Some(&7) && self.real_dims.get(&id) == Some(&(40, 30)));
            let _ = self.tx.send(library::Done::Scan(folders[1..].to_vec(), false, vec![]));
            let _ = self.tx.send(library::Done::Cached(folders, known));
            self.process_results();
            let ok_after = !self.key_index.contains_key(&first) && self.scanned;
            self.store.roots.clear();
            eprintln!("cache check: {}", if ok_before && ok_after { "ok" } else { "WRONG" });
        }
        if shot_view == "trash" {
            self.dialog = Some(Dialog::Trash { keys: vec!["/home/me/Pictures/a.jpg".into(); 14], bytes: 61_000_000, rejects: true });
        }
        if shot_view == "open" && self.photos.len() > 12 {
            // several pictures selected, one opened: the editor steps through those, and the
            // selection is still there after stepping
            self.selection = [3usize, 7, 12].into_iter().collect();
            self.open_editor(7);
            let list = self.edit.list.clone();
            self.set_edit_photo(12);
            let ok = list == [3, 7, 12] && self.selection.len() == 3;
            eprintln!("open check: {}", if ok { "kept" } else { "LOST" });
        }
        if shot_view == "analysis" || shot_view == "packs" {
            // the background analysis under way, as the status bar and Settings show it
            self.analysis = crate::analysis::Progress { done: 1240, total: 18300, paused: std::env::var("ALBUMCRAFT_SHOT_PAUSED").is_ok() };
        }
        if shot_view == "packs" {
            // Settings › AI features with packs to download (sample entries: no pack is published yet)
            use crate::ai::models::{ModelFile, Pack};
            let pack = |id: &str, title: &str, about: &str, licence: &str, size: u64| Pack {
                id: id.into(),
                title: title.into(),
                about: about.into(),
                licence: licence.into(),
                version: 1,
                release: "models-1".into(),
                files: vec![ModelFile { name: format!("{id}.onnx"), size, sha256: "0".repeat(64) }],
                search: None,
            };
            self.packs = vec![
                crate::analysis::PackState { pack: pack("faces", "Faces", "Finds faces and groups them by person; you name the groups.", "Apache 2.0", 9 << 20), installed: true },
                crate::analysis::PackState { pack: pack("search-light", "Smart search: Light", "Find pictures by describing them, in English.", "MIT", 218 << 20), installed: false },
                crate::analysis::PackState { pack: pack("search-multilingual", "Smart search: Multilingual", "Find pictures by describing them, in your language.", "Apache 2.0", 511 << 20), installed: false },
            ];
            self.packs_at = f64::INFINITY;
            self.dialog = Some(Dialog::Settings { section: crate::settings_view::SettingsSection::Ai });
        }
        if shot_view == "maps" {
            self.dialog = Some(Dialog::OnlineMaps);
        }
    }
}

impl App {
    /// For screenshots: `ALBUMCRAFT_SHOT_RCLICK=x,y` right-clicks there (window points) after
    /// the window has settled, to show a context menu; `ALBUMCRAFT_SHOT_HOVER=x,y` then rests
    /// the pointer on a menu entry (opens a submenu).
    /// Also outside `--screenshot` (the variable is a developer's aid), counting frames itself.
    pub fn demo_input(&mut self, ctx: &egui::Context, raw: &mut egui::RawInput) {
        static FRAMES: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        if std::env::var_os("ALBUMCRAFT_SHOT_RCLICK").is_none() {
            return;
        }
        let frames = FRAMES.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        if frames < 200 {
            ctx.request_repaint();
        }
        let at = |name: &str| -> Option<egui::Pos2> {
            let v = std::env::var(name).ok()?;
            let (x, y) = v.split_once(',')?;
            Some(egui::pos2(x.trim().parse().ok()?, y.trim().parse().ok()?))
        };
        let button = |pos, pressed| egui::Event::PointerButton { pos, button: egui::PointerButton::Secondary, pressed, modifiers: Default::default() };
        if let Some(pos) = at("ALBUMCRAFT_SHOT_RCLICK") {
            match frames {
                60 => raw.events.push(egui::Event::PointerMoved(pos)),
                62 => raw.events.push(button(pos, true)),
                63 => raw.events.push(button(pos, false)),
                _ => {}
            }
        }
        if let Some(pos) = at("ALBUMCRAFT_SHOT_HOVER")
            && frames >= 70
        {
            raw.events.push(egui::Event::PointerMoved(pos));
        }
    }
}
