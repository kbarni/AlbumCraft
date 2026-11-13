//! What the context menus do with files and folders: open them in other programs, copy their
//! path, rename and move them on disk (one rename on the same disk; the notes follow, with Undo),
//! move pictures to the trash, make thumbnails again, take a folder out of the library. Also the
//! folder's context menu (sidebar row and grid header) and the dialogs these open.

use crate::app::*;
use crate::i18n::tr;
use crate::icons;
use crate::library_view::{open_with_system, reveal_in_file_manager};
use eframe::egui::{self, RichText};
use std::path::Path;

/// The last part of a path (file or folder name).
fn name_of(path: &str) -> String {
    Path::new(path).file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| path.to_string())
}

/// "Renamed a to b." / "Moved 3 pictures to Trip."
fn moved_text(done: &[(String, String)], renamed: bool) -> String {
    match done {
        [(from, to)] if renamed => tr!("files-renamed", from = name_of(from), to = name_of(to)),
        [(from, to), ..] => {
            let n = done.len();
            let folder = Path::new(to).parent().map(|p| name_of(&p.to_string_lossy())).unwrap_or_default();
            tr!("files-moved", n = n, count = crate::app::fmt_count(n), name = name_of(from), folder = folder)
        }
        [] => String::new(),
    }
}

/// What a Move dialog moves.
#[derive(Clone, Debug, PartialEq)]
pub enum Moving {
    /// Picture files, by key.
    Pictures(Vec<String>),
    /// A folder, by path.
    Folder(String),
}

impl App {
    /// The file part of a picture's context menu: other programs, file manager, path, and the
    /// changes on disk. `id` is the picture right-clicked, `tray` what the actions apply to.
    pub fn picture_file_menu(&mut self, ui: &mut egui::Ui, id: usize, tray: &[usize]) {
        let Some(p) = self.photos.get(id) else { return };
        if p.is_sample() {
            return;
        }
        let key = p.key.clone();
        let keys: Vec<String> = tray.iter().filter_map(|&t| self.photos.get(t)).filter(|p| !p.is_sample()).map(|p| p.key.clone()).collect();
        ui.separator();
        if ui.button(tr!("ctx-open-default")).clicked() {
            if let Err(e) = open_with_system(Path::new(&key)) {
                self.toast(tr!("notice-open-failed", error = e.to_string()));
            }
            ui.close();
        }
        self.open_with_menu(ui, &key);
        if ui.button(format!("{} {}", icons::FOLDER, tr!("ctx-show-file"))).clicked() {
            if let Err(e) = reveal_in_file_manager(Path::new(&key)) {
                self.toast(tr!("notice-no-file-manager", error = e.to_string()));
            }
            ui.close();
        }
        if ui.button(tr!("ctx-copy-path")).clicked() {
            let n = keys.len();
            ui.ctx().copy_text(keys.join("\n"));
            self.toast(tr!("notice-copied-path", n = n));
            ui.close();
        }
        ui.separator();
        if keys.len() == 1 && ui.button(format!("{} {}", icons::EDIT, tr!("ctx-rename"))).clicked() {
            let stem = Path::new(&key).file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
            self.dialog = Some(Dialog::Rename { path: key.clone(), folder: false, name: stem, error: String::new() });
            ui.close();
        }
        if ui.button(tr!("ctx-move")).clicked() {
            self.dialog = Some(Dialog::MoveTo { what: Moving::Pictures(keys.clone()), filter: String::new(), error: String::new() });
            ui.close();
        }
        if ui.button(tr!("ctx-trash")).clicked() {
            self.ask_trash(tray);
            ui.close();
        }
    }

    /// "Open with ▸": the programs listed for this kind of picture (Linux), or the system's
    /// own chooser (Windows). Nothing where neither is known.
    fn open_with_menu(&mut self, ui: &mut egui::Ui, key: &str) {
        if crate::open_with::has_system_chooser() {
            if ui.button(tr!("ctx-open-with")).clicked() {
                if let Err(e) = crate::open_with::system_chooser(key) {
                    self.toast(tr!("notice-open-failed", error = e.to_string()));
                }
                ui.close();
            }
            return;
        }
        let Some(mime) = crate::open_with::mime_of(key) else { return };
        // read once per kind of picture, when the menu first needs it
        let programs = self.programs.entry(mime).or_insert_with(|| crate::open_with::programs_for(mime)).clone();
        if programs.is_empty() {
            return;
        }
        let mut failed = None;
        ui.menu_button(tr!("ctx-open-with-menu"), |ui| {
            for prog in &programs {
                if ui.button(&prog.name).clicked() {
                    if let Err(e) = crate::open_with::open(prog, key) {
                        failed = Some(e.to_string());
                    }
                    ui.close();
                }
            }
        });
        if let Some(error) = failed {
            self.toast(tr!("notice-open-failed", error = error));
        }
    }

    /// The context menu of a folder (its row in the sidebar or its header in the grid).
    pub fn folder_menu(&mut self, ui: &mut egui::Ui, fi: usize) {
        let Some(f) = self.folders.get(fi).cloned() else { return };
        let ids = f.photos.clone();
        if ui.button(format!("{} {}", icons::CHECK, tr!("folder-select"))).clicked() {
            self.source = Source::Library;
            self.scroll_to_folder = Some(fi);
            self.pick_set(TraySet::Folder(fi));
            ui.close();
        }
        if let Some(&first) = ids.first() {
            if ui.button(format!("{} {}", icons::PLAY, tr!("album-slideshow"))).clicked() {
                self.start_slideshow(ids.clone(), first);
                ui.close();
            }
            if ui.button(format!("{} {}", icons::MAP, tr!("album-show-map"))).clicked() {
                self.show_set_on_map(f.name.clone(), &ids);
                ui.close();
            }
            self.album_menu(ui, format!("{} {}", icons::ALBUM, tr!("ctx-add-to-album")), &ids);
            if ui.button(format!("{} {}", icons::EXPORT, tr!("ctx-export"))).clicked() {
                self.open_export(ids.clone());
                ui.close();
            }
        }
        ui.separator();
        if ui.button(format!("{} {}", icons::TEXT, tr!("folder-texts"))).clicked() {
            self.edit_folder_texts(&f.path);
            ui.close();
        }
        if self.using_samples {
            return;
        }
        ui.separator();
        if ui.button(format!("{} {}", icons::FOLDER, tr!("folder-open"))).clicked() {
            if let Err(e) = open_with_system(Path::new(&f.path)) {
                self.toast(tr!("notice-no-file-manager", error = e.to_string()));
            }
            ui.close();
        }
        if ui.button(tr!("ctx-copy-path")).clicked() {
            ui.ctx().copy_text(f.path.clone());
            self.toast(tr!("notice-copied-path", n = 1));
            ui.close();
        }
        if ui.button(format!("{} {}", icons::REFRESH, tr!("folder-refresh"))).on_hover_text(tr!("folder-refresh-hint")).clicked() {
            self.refresh_thumbs(&ids);
            self.toast(tr!("folder-refreshing", name = f.name.as_str()));
            ui.close();
        }
        ui.separator();
        if ui.button(format!("{} {}", icons::EDIT, tr!("ctx-rename"))).clicked() {
            self.dialog = Some(Dialog::Rename { path: f.path.clone(), folder: true, name: f.name.clone(), error: String::new() });
            ui.close();
        }
        if ui.button(tr!("folder-move")).clicked() {
            self.dialog = Some(Dialog::MoveTo { what: Moving::Folder(f.path.clone()), filter: String::new(), error: String::new() });
            ui.close();
        }
        if ui.button(format!("{} {}", icons::REMOVE, tr!("folder-remove"))).on_hover_text(tr!("folder-remove-hint")).clicked() {
            self.remove_folder_from_library(&f.path);
            ui.close();
        }
        if ui.button(tr!("folder-trash")).clicked() {
            self.ask_trash_folder(&f.path);
            ui.close();
        }
    }

    /// Open the dialog for a folder's caption and description.
    pub fn edit_folder_texts(&mut self, path: &str) {
        let caption = self.store.folder_captions.get(path).cloned().unwrap_or_default();
        let text = self.store.folder_notes.get(path).cloned().unwrap_or_default();
        self.dialog = Some(Dialog::FolderNote { path: path.to_string(), caption, text });
    }

    /// Take a folder out of the library (its files and notes stay), with Undo.
    pub fn remove_folder_from_library(&mut self, path: &str) {
        match crate::commands::remove_folder(self, path) {
            Ok(at) => {
                let undo = match at {
                    Some(at) => Undo::Root { at, path: path.to_string() },
                    None => Undo::Excluded(path.to_string()),
                };
                self.selection.clear();
                self.tray_set = None;
                self.list_pick = None;
                self.undoable(tr!("removed-folder", path = path), undo);
            }
            Err(e) => self.toast(e),
        }
    }

    /// Ask before moving these pictures to the trash.
    pub fn ask_trash(&mut self, ids: &[usize]) {
        let pics: Vec<&crate::library::Photo> = ids.iter().filter_map(|&id| self.photos.get(id)).filter(|p| !p.is_sample()).collect();
        if pics.is_empty() {
            self.toast(tr!("trash-samples"));
            return;
        }
        let bytes = pics.iter().map(|p| p.bytes).sum();
        let keys = pics.iter().map(|p| p.key.clone()).collect();
        self.dialog = Some(Dialog::Trash { keys, bytes, rejects: false });
    }

    /// Make the thumbnails of these pictures again from their files.
    pub fn refresh_thumbs(&mut self, ids: &[usize]) {
        let keys: Vec<String> = ids.iter().filter_map(|&id| self.photos.get(id)).filter(|p| !p.is_sample()).map(|p| p.key.clone()).collect();
        if !keys.is_empty() {
            self.lib.forget_thumbs(keys);
        }
    }

    /// The cached thumbnails of these pictures are gone: forget the ones shown, so they are
    /// asked for again when drawn.
    pub fn thumbs_forgotten(&mut self, keys: Vec<String>) {
        for key in keys {
            let Some(&id) = self.key_index.get(&key) else { continue };
            self.thumb_base.remove(&id);
            self.thumbs.remove(&id);
            self.sharp_thumbs.remove(&id);
            self.sharp_req.remove(&id);
            self.thumb_req.remove(&id);
            self.thumb_failed.remove(&id);
            self.stale_thumbs.remove(&id);
            self.exif_thumbs.remove(&id);
        }
        self.ctx.request_repaint();
    }

    /// Files or folders moved on disk (old, new): the window's pictures and folders take their
    /// new paths before the rescan, so it keeps their thumbnails, the selection and the open
    /// picture.
    pub fn paths_moved(&mut self, pairs: &[(String, String)]) {
        let new_path = |k: &str| -> Option<String> {
            pairs.iter().find_map(|(from, to)| {
                let rest = Path::new(k).strip_prefix(from).ok()?;
                Some(if rest.as_os_str().is_empty() { to.clone() } else { Path::new(to).join(rest).to_string_lossy().into_owned() })
            })
        };
        for id in 0..self.photos.len() {
            let Some(n) = new_path(&self.photos[id].key) else { continue };
            self.key_index.remove(&self.photos[id].key);
            self.key_index.insert(n.clone(), id);
            self.photos[id].name = name_of(&n);
            self.photos[id].key = n;
        }
        for f in &mut self.folders {
            if let Some(n) = new_path(&f.path) {
                f.name = name_of(&n);
                f.path = n;
            }
        }
        self.list_rev += 1;
    }

    /// Move or rename on disk, tell the user how it went (with Undo). Returns the first
    /// problem when nothing moved.
    pub fn move_files(&mut self, pairs: Vec<(String, String)>, renamed: bool) -> Result<(), String> {
        // to another disk: copied in the background, as a task
        let (near, far): (Vec<_>, Vec<_>) = pairs.into_iter().partition(|(a, b)| self.lib.same_disk(a, b));
        if !far.is_empty() {
            let n = far.len();
            let task = self.start_task(tr!("task-move-across", n = n));
            self.lib.move_across(task, far);
            if near.is_empty() {
                return Ok(());
            }
        }
        let m = crate::commands::move_on_disk(self, &near);
        if m.done.is_empty() {
            return Err(m.failed.first().cloned().unwrap_or_default());
        }
        if !m.failed.is_empty() {
            let mut lines: Vec<String> = m.failed.iter().take(5).cloned().collect();
            if m.failed.len() > 5 {
                let n = m.failed.len() - 5;
                lines.push(tr!("files-more-problems", n = n));
            }
            self.error(lines.join("\n"));
        }
        let text = moved_text(&m.done, renamed);
        self.undoable(text, Undo::Moved(m.done));
        Ok(())
    }

    /// Files or folders were moved to another disk in the background: the notes follow them.
    pub fn moved_across(&mut self, done: Vec<(String, String)>) {
        if done.is_empty() {
            return;
        }
        for (from, to) in &done {
            self.store.moved(from, to);
        }
        crate::commands::Host::moved(self, &done);
        self.rescan();
        let text = moved_text(&done, false);
        self.undoable(text, Undo::Moved(done));
    }

    /// Put moved files back (Undo).
    pub fn move_back(&mut self, done: Vec<(String, String)>) {
        let back: Vec<(String, String)> = done.into_iter().rev().map(|(a, b)| (b, a)).collect();
        if let Err(e) = self.move_files(back, false) {
            self.error(e);
        }
    }

    /// Ask before moving a folder and everything in it to the trash.
    pub fn ask_trash_folder(&mut self, path: &str) {
        let inside = |p: &str| Path::new(p).starts_with(path);
        let ids: Vec<usize> = self.folders.iter().filter(|f| inside(&f.path)).flat_map(|f| f.photos.iter().copied()).collect();
        let bytes = ids.iter().filter_map(|&id| self.photos.get(id)).map(|p| p.bytes).sum();
        self.dialog = Some(Dialog::TrashFolder { path: path.to_string(), pictures: ids.len(), bytes });
    }

    /// Move a folder to the trash (confirmed); a library folder leaves the list when it worked.
    pub fn trash_folder(&mut self, path: String) {
        let task = self.start_task(tr!("task-trash-folder", name = name_of(&path)));
        self.rescan_after = Some(task.id);
        if self.store.roots.iter().any(|r| Path::new(r) == Path::new(&path)) {
            self.roots_after.push((task.id, path.clone()));
        }
        self.selection.clear();
        self.tray_set = None;
        self.list_pick = None;
        self.lib.trash(task, vec![path]);
    }

    /// Delete for good what the trash refused (confirmed).
    pub fn delete_forever(&mut self, paths: Vec<String>) {
        let n = paths.len();
        let task = self.start_task(tr!("task-delete", n = n));
        self.rescan_after = Some(task.id);
        for p in &paths {
            if self.store.roots.iter().any(|r| Path::new(r) == Path::new(p)) {
                self.roots_after.push((task.id, p.clone()));
            }
        }
        self.selection.clear();
        self.tray_set = None;
        self.lib.delete_permanently(task, paths);
    }

    /// The Rename dialog's content; true when it is done.
    pub fn rename_dialog(&mut self, ui: &mut egui::Ui, path: &str, folder: bool, name: &mut String, error: &mut String, enter: bool) -> bool {
        ui.label(tr!("rename-label"));
        let r = ui.add(egui::TextEdit::singleline(name).desired_width(f32::INFINITY));
        r.request_focus();
        let ext = if folder { None } else { Path::new(path).extension().map(|e| e.to_string_lossy().into_owned()) };
        if let Some(ext) = &ext {
            ui.label(RichText::new(tr!("rename-keeps-ext", ext = ext.as_str())).weak());
        }
        if !error.is_empty() {
            ui.colored_label(crate::theme::t().reject, error.as_str());
        }
        ui.add_space(6.0);
        let mut close = false;
        ui.horizontal(|ui| {
            if ui.button(tr!("rename-button")).clicked() || enter {
                let to = if folder { crate::files::renamed_folder(path, name) } else { crate::files::renamed_file(path, name) };
                match to.and_then(|to| self.move_files(vec![(path.to_string(), to)], true)) {
                    Ok(()) => close = true,
                    Err(e) => *error = e,
                }
            }
            if ui.button(tr!("button-cancel")).clicked() {
                close = true;
            }
        });
        close
    }

    /// The Move dialog's content: the library's folders to choose from; true when it is done.
    pub fn move_dialog(&mut self, ui: &mut egui::Ui, what: &Moving, filter: &mut String, error: &mut String) -> bool {
        let (sources, intro): (Vec<String>, String) = match what {
            Moving::Pictures(keys) => {
                let n = keys.len();
                let first = keys.first().map(|k| name_of(k)).unwrap_or_default();
                (keys.clone(), tr!("move-pictures-into", n = n, count = crate::app::fmt_count(n), name = first))
            }
            Moving::Folder(path) => (vec![path.clone()], tr!("move-folder-into", name = name_of(path))),
        };
        ui.label(intro);
        ui.add(egui::TextEdit::singleline(filter).hint_text(tr!("move-filter")).desired_width(f32::INFINITY));
        let wanted = filter.trim().to_lowercase();
        // where they are now (no use moving there), and for a folder: itself and what is inside
        let here: Vec<&Path> = sources.iter().filter_map(|s| Path::new(s).parent()).collect();
        let mut dirs: Vec<String> = self.store.roots.iter().cloned().chain(self.folders.iter().map(|f| f.path.clone())).collect();
        dirs.sort();
        dirs.dedup();
        dirs.retain(|d| {
            let p = Path::new(d);
            let inside = matches!(what, Moving::Folder(f) if p.starts_with(f));
            let same = here.iter().all(|h| *h == p);
            !inside && !same && (wanted.is_empty() || d.to_lowercase().contains(&wanted))
        });
        let mut chosen = None;
        egui::ScrollArea::vertical().max_height(260.0).auto_shrink([false, true]).show(ui, |ui| {
            if dirs.is_empty() {
                ui.weak(tr!("move-none"));
            }
            for d in dirs.iter().take(300) {
                let root = self.store.roots.iter().filter(|r| Path::new(d).starts_with(r)).max_by_key(|r| r.len());
                // shown from the library folder down, e.g. "Pictures / 2024 / Trip"
                let shown = match root.and_then(|r| Some((r, Path::new(d).strip_prefix(r).ok()?))) {
                    Some((r, rest)) => std::iter::once(name_of(r)).chain(rest.iter().map(|c| c.to_string_lossy().into_owned())).collect::<Vec<_>>().join(" / "),
                    None => d.clone(),
                };
                if ui.selectable_label(false, format!("{} {shown}", icons::FOLDER)).on_hover_text(d).clicked() {
                    chosen = Some(d.clone());
                }
            }
        });
        if !error.is_empty() {
            ui.colored_label(crate::theme::t().reject, error.as_str());
        }
        ui.add_space(6.0);
        let mut close = false;
        ui.horizontal(|ui| {
            if ui.button(tr!("move-other")).clicked() {
                let start = self.store.roots.first().cloned().unwrap_or_default();
                if let Some(dir) = rfd::FileDialog::new().set_title(tr!("choose-folder-dialog")).set_directory(start).pick_folder() {
                    chosen = Some(dir.to_string_lossy().into_owned());
                }
            }
            if ui.button(tr!("button-cancel")).clicked() {
                close = true;
            }
        });
        if let Some(dir) = chosen {
            let pairs = crate::commands::library_folder(&self.store, &dir)
                .and_then(|()| sources.iter().map(|s| crate::files::moved_into(s, &dir).map(|to| (s.clone(), to))).collect::<Result<Vec<_>, _>>());
            match pairs.and_then(|pairs| self.move_files(pairs, false)) {
                Ok(()) => close = true,
                Err(e) => *error = e,
            }
        }
        close
    }
}
