//! `View::Import`: copy pictures from a memory card, a camera or any folder into a new folder of
//! the library. Pick the source, uncheck what you don't want (pictures the library already has
//! start hidden), name the folder, import. The copy runs in the background (`Library::import`);
//! the status bar shows its progress and the new folder opens when it is done.
//!
//! Adding a folder to the library (the folder manager) is the other way in: it copies nothing.
#![allow(deprecated)] // egui::Id::new, as used elsewhere in this crate

use crate::i18n::tr;
use crate::app::{to_color_image, App, Dialog, TraySet, View};
use crate::icons;
use crate::import::{self, Candidate};
use crate::library::{fmt_bytes, fmt_date, fmt_time, Job, BIG_DIM, THUMB_DIM};
use crate::theme::t;
use eframe::egui::{self, Color32, Pos2, Rect, RichText, Sense, Stroke, StrokeKind, TextureHandle, TextureOptions, Vec2};
use std::collections::{HashMap, HashSet};
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

const CELL: f32 = 116.0;
const GAP: f32 = 8.0;
const HEADER: f32 = 38.0;
/// Thumbnails kept as textures; the ones not seen for a while go first.
const MAX_THUMBS: usize = 600;

pub struct ImportState {
    back: View,
    sources: Vec<(String, PathBuf)>,
    source: Option<PathBuf>,
    listing: bool,
    items: Vec<Candidate>,
    groups: Vec<Range<usize>>,
    checked: Vec<bool>,
    truncated: bool,
    hide_dups: bool,
    current: Option<usize>,
    scroll_to_current: bool,
    thumbs: HashMap<String, (TextureHandle, f64)>,
    thumb_req: HashSet<String>,
    thumb_failed: HashSet<String>,
    preview: Option<(String, TextureHandle)>,
    dest_root: String,
    /// Folders offered as the place of the new folder.
    dest_roots: Vec<String>,
    name: String,
    /// The user typed the name: a new source doesn't replace it.
    name_edited: bool,
}

/// A copy running in the background.
pub struct Importing {
    pub done: usize,
    pub total: usize,
    pub stop: Arc<AtomicBool>,
}

impl ImportState {
    fn visible(&self, i: usize) -> bool {
        !(self.hide_dups && self.items.get(i).is_some_and(|c| c.duplicate))
    }

    fn visible_in(&self, r: &Range<usize>) -> Vec<usize> {
        r.clone().filter(|&i| self.visible(i)).collect()
    }

    /// The pictures to copy: checked and shown, oldest first.
    fn chosen(&self) -> Vec<usize> {
        (0..self.items.len()).filter(|&i| self.visible(i) && self.checked.get(i).copied().unwrap_or(false)).collect()
    }

    fn duplicates(&self) -> usize {
        self.items.iter().filter(|c| c.duplicate).count()
    }

    fn step(&mut self, by: isize) {
        let shown: Vec<usize> = (0..self.items.len()).filter(|&i| self.visible(i)).collect();
        if shown.is_empty() {
            return;
        }
        let pos = self.current.and_then(|c| shown.iter().position(|&i| i == c));
        let next = match pos {
            Some(p) => (p as isize + by).clamp(0, shown.len() as isize - 1) as usize,
            None => 0,
        };
        self.current = shown.get(next).copied();
        self.scroll_to_current = true;
    }

    fn toggle(&mut self, i: usize) {
        if let Some(c) = self.checked.get_mut(i) {
            *c = !*c;
        }
    }

    fn dest(&self) -> Result<PathBuf, String> {
        let name = import::check_folder_name(&self.name)?;
        if self.dest_root.is_empty() {
            return Err(tr!("import-pick-dest"));
        }
        Ok(Path::new(&self.dest_root).join(name))
    }
}

impl App {
    pub fn open_import(&mut self) {
        if self.importing.is_some() {
            self.toast(tr!("import-running"));
            return;
        }
        let mut dest_roots: Vec<String> = if self.using_samples { vec![] } else { self.store.roots.clone() };
        if let Some(last) = &self.store.import_to
            && !dest_roots.contains(last)
        {
            dest_roots.insert(0, last.clone());
        }
        if dest_roots.is_empty()
            && let Some(p) = dirs::picture_dir()
        {
            dest_roots.push(p.to_string_lossy().to_string());
        }
        let dest_root = self.store.import_to.clone().or_else(|| dest_roots.first().cloned()).unwrap_or_default();
        let sources = self.lib.import_sources();
        let only = if sources.len() == 1 { sources.first().map(|s| s.1.clone()) } else { None };
        self.import = Some(Box::new(ImportState {
            back: if self.view == View::Edit { View::Edit } else { View::Library },
            sources,
            source: None,
            listing: false,
            items: vec![],
            groups: vec![],
            checked: vec![],
            truncated: false,
            hide_dups: true,
            current: None,
            scroll_to_current: false,
            thumbs: HashMap::new(),
            thumb_req: HashSet::new(),
            thumb_failed: HashSet::new(),
            preview: None,
            dest_root,
            dest_roots,
            name: String::new(),
            name_edited: false,
        }));
        self.view = View::Import;
        if let Some(s) = only {
            self.set_import_source(s);
        }
    }

    fn leave_import(&mut self) {
        let back = self.import.as_ref().map(|s| s.back).unwrap_or(View::Library);
        self.import = None;
        self.view = back;
    }

    fn set_import_source(&mut self, source: PathBuf) {
        let library: Vec<(String, u64)> =
            if self.using_samples { vec![] } else { self.photos.iter().map(|p| (p.key.clone(), p.bytes)).collect() };
        let Some(st) = self.import.as_mut() else { return };
        if !st.name_edited {
            st.name = import::suggested_name(&source);
        }
        st.source = Some(source.clone());
        st.listing = true;
        st.items.clear();
        st.groups.clear();
        st.checked.clear();
        st.current = None;
        st.thumbs.clear();
        st.thumb_req.clear();
        st.thumb_failed.clear();
        st.preview = None;
        self.lib.list_import(source, library);
    }

    fn choose_import_source(&mut self) {
        if let Some(dir) = rfd::FileDialog::new().set_title(tr!("import-from-dialog")).pick_folder() {
            self.set_import_source(dir);
        }
    }

    pub fn on_import_list(&mut self, source: PathBuf, items: Vec<Candidate>, truncated: bool) {
        let Some(st) = self.import.as_mut() else { return };
        if st.source.as_ref() != Some(&source) {
            return; // an older source
        }
        st.listing = false;
        st.groups = import::groups(&items);
        st.checked = items.iter().map(|c| !c.duplicate).collect();
        st.items = items;
        st.truncated = truncated;
        st.current = None;
        st.step(0);
    }

    pub fn on_import_file(&mut self, key: String, dim: u32, img: Option<image::RgbaImage>) {
        let Some(st) = self.import.as_mut() else { return };
        let now = self.ctx.input(|i| i.time);
        match (dim, img) {
            (BIG_DIM, Some(img)) => {
                let current = st.current.and_then(|c| st.items.get(c)).map(|c| c.path.as_str());
                if current == Some(key.as_str()) {
                    let tex = self.ctx.load_texture("import-preview", to_color_image(&img), TextureOptions::LINEAR);
                    st.preview = Some((key, tex));
                }
            }
            (BIG_DIM, None) => {}
            (_, Some(img)) => {
                let tex = self.ctx.load_texture(format!("import:{key}"), to_color_image(&img), TextureOptions::LINEAR);
                st.thumbs.insert(key, (tex, now));
            }
            (_, None) => {
                st.thumb_failed.insert(key);
            }
        }
    }

    pub fn on_import_done(&mut self, dest: PathBuf, summary: String, copied: usize, problems: bool) {
        self.importing = None;
        if copied > 0 {
            // already shown when inside a library folder (that is not an error here)
            let _ = self.store.add_root(&dest.to_string_lossy());
            self.dirty = true;
            self.show_after_scan = Some(dest);
            self.rescan();
        }
        if problems || copied == 0 {
            if self.dialog.is_none() {
                self.dialog = Some(Dialog::Info { title: tr!("bar-import"), text: summary });
            } else {
                self.toast(summary);
            }
        } else {
            self.toast(summary);
        }
    }

    /// After the scan that follows an import: show the new folder.
    pub fn show_imported_folder(&mut self) {
        let Some(dest) = self.show_after_scan.take() else { return };
        if let Some(fi) = self.folders.iter().position(|f| Path::new(&f.path) == dest) {
            self.source = crate::app::Source::Library;
            self.scroll_to_folder = Some(fi);
            self.pick_set(TraySet::Folder(fi));
        }
    }

    fn start_import(&mut self) {
        let Some(st) = self.import.as_ref() else { return };
        let dest = match st.dest() {
            Ok(d) => d,
            Err(e) => {
                self.toast(e);
                return;
            }
        };
        let files: Vec<String> = st.chosen().into_iter().filter_map(|i| st.items.get(i)).map(|c| c.path.clone()).collect();
        if files.is_empty() {
            return;
        }
        let n = files.len();
        self.store.import_to = Some(st.dest_root.clone());
        self.dirty = true;
        let stop = Arc::new(AtomicBool::new(false));
        self.lib.import(files, dest.clone(), stop.clone());
        self.importing = Some(Importing { done: 0, total: n, stop });
        self.leave_import();
        self.toast(tr!("import-started", n = n, path = dest.display().to_string()));
    }

    pub fn stop_import(&mut self) {
        if let Some(i) = &self.importing {
            i.stop.store(true, Ordering::Relaxed);
        }
    }

    pub fn import_ui(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        if self.import.is_none() {
            self.view = View::Library;
            return;
        }
        let typing = ctx.memory(|m| m.focused().is_some());
        if !typing && self.dialog.is_none() {
            let (esc, left, right, space) = ctx.input(|i| {
                (
                    i.key_pressed(egui::Key::Escape),
                    i.key_pressed(egui::Key::ArrowLeft),
                    i.key_pressed(egui::Key::ArrowRight),
                    i.key_pressed(egui::Key::Space),
                )
            });
            if esc {
                self.leave_import();
                return;
            }
            if let Some(st) = self.import.as_mut() {
                if left {
                    st.step(-1);
                }
                if right {
                    st.step(1);
                }
                if space && let Some(c) = st.current {
                    st.toggle(c);
                }
            }
        }
        self.import_top_bar(ui);
        self.import_bottom_bar(ui);
        let has_items = self.import.as_ref().is_some_and(|s| !s.items.is_empty());
        if has_items {
            self.import_preview(ui);
        }
        egui::CentralPanel::default().frame(egui::Frame::new().fill(t().lightbox_bg).inner_margin(12)).show(ui, |ui| {
            if has_items {
                self.import_toolbar(ui);
                self.import_grid(ui);
            } else {
                self.import_empty(ui);
            }
        });
    }

    fn import_top_bar(&mut self, ui: &mut egui::Ui) {
        let mut leave = false;
        let mut pick: Option<PathBuf> = None;
        let mut choose = false;
        let mut redetect = false;
        let Some(st) = self.import.as_mut() else { return };
        egui::Panel::top("import_top")
            .exact_size(60.0)
            .frame(egui::Frame::new().fill(t().toolbar_bg).inner_margin(egui::Margin::symmetric(16, 8)).stroke(Stroke::new(1.0, t().border)))
            .show(ui, |ui| {
                ui.horizontal_centered(|ui| {
                    let back = format!("{}  {}", icons::ARROW_BACK, if st.back == View::Edit { tr!("view-back-editor") } else { tr!("view-back-library") });
                    if ui.add(egui::Button::new(RichText::new(back).size(13.5)).min_size(egui::vec2(0.0, 32.0))).on_hover_text(tr!("view-back-hint")).clicked() {
                        leave = true;
                    }
                    ui.add_space(10.0);
                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing.y = 0.0;
                        ui.label(RichText::new(tr!("bar-import")).size(20.0).strong().color(t().text_strong));
                        let sub = if st.listing {
                            tr!("scanning")
                        } else if st.source.is_none() {
                            tr!("import-subtitle")
                        } else {
                            let shown = (0..st.items.len()).filter(|&i| st.visible(i)).count();
                            tr!("count-pictures", n = shown)
                        };
                        ui.label(RichText::new(sub).size(12.0).color(t().text_weak));
                    });
                    ui.add_space(18.0);
                    ui.label(RichText::new(tr!("import-from")).color(t().text_weak));
                    let current = match &st.source {
                        Some(p) => st.sources.iter().find(|s| &s.1 == p).map(|s| s.0.clone()).unwrap_or_else(|| p.display().to_string()),
                        None => tr!("import-choose"),
                    };
                    egui::ComboBox::from_id_salt("import_from").selected_text(current).width(260.0).show_ui(ui, |ui| {
                        for (label, path) in &st.sources {
                            if ui.selectable_label(st.source.as_ref() == Some(path), format!("{}  {label}", icons::DRIVE)).on_hover_text(path.display().to_string()).clicked() {
                                pick = Some(path.clone());
                            }
                        }
                        if !st.sources.is_empty() {
                            ui.separator();
                        }
                        if ui.button(format!("{}  {}", icons::FOLDER, tr!("export-choose-folder"))).clicked() {
                            choose = true;
                        }
                    });
                    if ui.button(icons::REFRESH).on_hover_text(tr!("import-refresh-hint")).clicked() {
                        redetect = true;
                    }
                });
            });
        if redetect {
            st.sources = self.lib.import_sources();
            if let Some(s) = st.source.clone() {
                pick = Some(s);
            }
        }
        if leave {
            self.leave_import();
        } else if let Some(p) = pick {
            self.set_import_source(p);
        } else if choose {
            self.choose_import_source();
        }
    }

    /// Above the grid: hide duplicates, check / uncheck all.
    fn import_toolbar(&mut self, ui: &mut egui::Ui) {
        let Some(st) = self.import.as_mut() else { return };
        ui.horizontal(|ui| {
            let dups = st.duplicates();
            if dups > 0 {
                ui.checkbox(&mut st.hide_dups, tr!("import-hide-dups", n = dups))
                    .on_hover_text(tr!("import-dups-hint"));
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button(tr!("import-uncheck-all")).clicked() {
                    st.checked.iter_mut().for_each(|c| *c = false);
                }
                if ui.button(tr!("import-check-all")).clicked() {
                    for i in 0..st.items.len() {
                        if st.visible(i)
                            && let Some(c) = st.checked.get_mut(i)
                        {
                            *c = true;
                        }
                    }
                }
            });
        });
        ui.add_space(4.0);
    }

    fn import_bottom_bar(&mut self, ui: &mut egui::Ui) {
        let mut go = false;
        let mut cancel = false;
        let mut choose_dest = false;
        let Some(st) = self.import.as_mut() else { return };
        let chosen = st.chosen().len();
        let dest = st.dest();
        egui::Panel::bottom("import_bottom")
            .frame(egui::Frame::new().fill(t().toolbar_bg).inner_margin(egui::Margin::symmetric(16, 10)).stroke(Stroke::new(1.0, t().border)))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(tr!("import-copy-into")).color(t().text_weak));
                    let shown = if st.dest_root.is_empty() { tr!("import-choose") } else { st.dest_root.clone() };
                    egui::ComboBox::from_id_salt("import_to").selected_text(shown).width(240.0).show_ui(ui, |ui| {
                        for r in &st.dest_roots {
                            if ui.selectable_label(&st.dest_root == r, r).clicked() {
                                st.dest_root = r.clone();
                            }
                        }
                        if !st.dest_roots.is_empty() {
                            ui.separator();
                        }
                        if ui.button(format!("{}  {}", icons::FOLDER, tr!("import-another-folder"))).clicked() {
                            choose_dest = true;
                        }
                    });
                    ui.label(RichText::new("/").size(16.0).color(t().text_weak));
                    let r = ui.add(
                        egui::TextEdit::singleline(&mut st.name)
                            .hint_text(tr!("import-name-hint"))
                            .desired_width(ui.available_width().clamp(120.0, 320.0)),
                    );
                    if r.changed() {
                        st.name_edited = true;
                    }
                });
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    let shown = (0..st.items.len()).filter(|&i| st.visible(i)).count();
                    match &dest {
                        _ if st.items.is_empty() => {}
                        Ok(_) => {
                            ui.label(RichText::new(tr!("import-checked", chosen = chosen, shown = shown)).size(12.5).color(t().text_weak));
                        }
                        Err(e) => {
                            ui.label(RichText::new(e).size(12.5).color(t().bad));
                        }
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let label = tr!("import-go", n = chosen);
                        let ok = chosen > 0 && dest.is_ok();
                        let b = egui::Button::new(RichText::new(label).strong().color(if ok { Color32::WHITE } else { t().text_weak }))
                            .fill(if ok { t().accent } else { t().surface })
                            .min_size(egui::vec2(150.0, 32.0));
                        if ui.add_enabled(ok, b).clicked() {
                            go = true;
                        }
                        if ui.add(egui::Button::new(tr!("button-cancel")).min_size(egui::vec2(0.0, 32.0))).clicked() {
                            cancel = true;
                        }
                    });
                });
            });
        if choose_dest
            && let Some(dir) = rfd::FileDialog::new().set_title(tr!("import-dest-dialog")).pick_folder()
        {
            let d = dir.to_string_lossy().to_string();
            if !st.dest_roots.contains(&d) {
                st.dest_roots.push(d.clone());
            }
            st.dest_root = d;
        }
        if go {
            self.start_import();
        } else if cancel {
            self.leave_import();
        }
    }

    fn import_empty(&mut self, ui: &mut egui::Ui) {
        let mut pick = None;
        let mut choose = false;
        let mut folders = false;
        let Some(st) = self.import.as_ref() else { return };
        ui.vertical_centered(|ui| {
            ui.add_space((ui.available_height() / 2.0 - 140.0).max(20.0));
            if st.listing {
                ui.spinner();
                ui.add_space(8.0);
                let from = st.source.as_ref().map(|p| p.display().to_string()).unwrap_or_default();
                ui.label(RichText::new(tr!("import-listing-in", from = from)).size(15.0).color(t().text_weak));
                return;
            }
            if let Some(src) = &st.source {
                ui.label(RichText::new(icons::SEARCH).size(42.0).color(t().text_weak));
                ui.add_space(6.0);
                ui.label(RichText::new(tr!("import-none-in", path = src.display().to_string())).size(15.0).color(t().text_weak));
                ui.add_space(12.0);
                if ui.button(format!("{}  {}", icons::FOLDER, tr!("import-choose-another"))).clicked() {
                    choose = true;
                }
                return;
            }
            ui.label(RichText::new(icons::DRIVE).size(48.0).color(t().text_weak));
            ui.add_space(6.0);
            ui.label(RichText::new(tr!("import-heading")).size(18.0).color(t().text_strong));
            ui.add_space(4.0);
            ui.label(RichText::new(tr!("import-copy-note")).color(t().text_weak));
            ui.add_space(16.0);
            for (label, path) in &st.sources {
                let b = egui::Button::new(RichText::new(format!("{}  {label}", icons::DRIVE)).size(14.0)).min_size(egui::vec2(260.0, 34.0));
                if ui.add(b).on_hover_text(path.display().to_string()).clicked() {
                    pick = Some(path.clone());
                }
            }
            if st.sources.is_empty() {
                ui.label(RichText::new(tr!("import-no-cards", icon = icons::REFRESH)).color(t().text_weak).size(12.5));
                ui.add_space(4.0);
            }
            let b = egui::Button::new(RichText::new(format!("{}  {}", icons::FOLDER, tr!("export-choose-folder"))).size(14.0)).min_size(egui::vec2(260.0, 34.0));
            if ui.add(b).clicked() {
                choose = true;
            }
            ui.add_space(28.0);
            ui.label(RichText::new(tr!("import-instead")).size(12.5).color(t().text_weak));
            if ui.link(RichText::new(tr!("import-add-instead")).size(12.5)).clicked() {
                folders = true;
            }
        });
        if let Some(p) = pick {
            self.set_import_source(p);
        } else if choose {
            self.choose_import_source();
        } else if folders {
            self.leave_import();
            self.open_settings(crate::settings_view::SettingsSection::Folders);
        }
    }

    fn import_grid(&mut self, ui: &mut egui::Ui) {
        let now = ui.ctx().input(|i| i.time);
        let mut wanted: Vec<String> = vec![];
        let Some(st) = self.import.as_mut() else { return };
        let width = ui.available_width();
        let cols = (((width + GAP) / (CELL + GAP)).floor() as usize).max(1);
        // the layout: one header row per group, then its rows of tiles
        enum Row {
            Header(usize),
            Tiles(Vec<usize>),
        }
        let mut rows: Vec<(f32, Row)> = vec![];
        let mut y = 0.0;
        for (g, r) in st.groups.iter().enumerate() {
            let shown = st.visible_in(r);
            if shown.is_empty() {
                continue;
            }
            rows.push((y, Row::Header(g)));
            y += HEADER;
            for chunk in shown.chunks(cols) {
                rows.push((y, Row::Tiles(chunk.to_vec())));
                y += CELL + GAP;
            }
            y += GAP;
        }
        if rows.is_empty() {
            ui.vertical_centered(|ui| {
                ui.add_space(60.0);
                ui.label(RichText::new(tr!("import-all-dups", n = st.items.len())).size(15.0).color(t().text_weak));
                if ui.button(tr!("import-show-anyway")).clicked() {
                    st.hide_dups = false;
                }
            });
            return;
        }
        if st.truncated {
            ui.label(RichText::new(tr!("import-truncated", n = import::LIST_LIMIT)).color(t().bad));
        }
        let total = y;
        let mut toggle_group: Option<usize> = None;
        let mut clicked: Option<(usize, bool)> = None;
        egui::ScrollArea::vertical().auto_shrink([false, false]).show_viewport(ui, |ui, vp| {
            ui.set_height(total);
            let origin = ui.max_rect().min;
            for (ry, row) in &rows {
                let h = match row {
                    Row::Header(_) => HEADER,
                    Row::Tiles(_) => CELL,
                };
                let scroll_here = matches!(row, Row::Tiles(ids) if st.scroll_to_current && st.current.is_some_and(|c| ids.contains(&c)));
                if !scroll_here && (*ry + h < vp.min.y - 200.0 || *ry > vp.max.y + 200.0) {
                    continue;
                }
                let top = origin + Vec2::new(0.0, *ry);
                match row {
                    Row::Header(g) => {
                        let Some(r) = st.groups.get(*g) else { continue };
                        let shown = st.visible_in(r);
                        let on = shown.iter().filter(|&&i| st.checked.get(i).copied().unwrap_or(false)).count();
                        let rect = Rect::from_min_size(top, egui::vec2(width, HEADER - 6.0));
                        let first = shown.first().and_then(|&i| st.items.get(i));
                        let title = first.map(|c| format!("{}, {}", fmt_date(c.taken), fmt_time(c.taken))).unwrap_or_default();
                        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(rect).layout(egui::Layout::left_to_right(egui::Align::Center)));
                        let mut all = on == shown.len();
                        let cb = egui::Checkbox::new(&mut all, RichText::new(title).size(15.0).color(t().text_strong)).indeterminate(on > 0 && on < shown.len());
                        if child.add(cb).on_hover_text(tr!("import-group-hint")).clicked() {
                            toggle_group = Some(*g);
                        }
                        child.label(RichText::new(format!("· {}", tr!("count-pictures", n = shown.len()))).color(t().text_weak));
                    }
                    Row::Tiles(ids) => {
                        for (col, &i) in ids.iter().enumerate() {
                            let Some(c) = st.items.get(i) else { continue };
                            let rect = Rect::from_min_size(top + Vec2::new(col as f32 * (CELL + GAP), 0.0), Vec2::splat(CELL));
                            let resp = ui.interact(rect, egui::Id::new(("import_tile", i)), Sense::click());
                            if st.scroll_to_current && st.current == Some(i) {
                                ui.scroll_to_rect(rect, None);
                                st.scroll_to_current = false;
                            }
                            let p = ui.painter();
                            let checked = st.checked.get(i).copied().unwrap_or(false);
                            p.rect_filled(rect, 6, if resp.hovered() { t().surface_hover } else { t().surface });
                            let inner = rect.shrink(6.0);
                            match st.thumbs.get_mut(&c.path) {
                                Some((tex, seen)) => {
                                    *seen = now;
                                    let size = tex.size_vec2();
                                    let scale = (inner.width() / size.x.max(1.0)).min(inner.height() / size.y.max(1.0));
                                    let img = Rect::from_center_size(inner.center(), size * scale);
                                    p.image(tex.id(), img, Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)), Color32::WHITE);
                                    if !checked {
                                        p.rect_filled(img, 0, Color32::from_black_alpha(110));
                                    }
                                }
                                None => {
                                    p.rect_filled(inner, 3, t().placeholder);
                                    if !st.thumb_failed.contains(&c.path) && !st.thumb_req.contains(&c.path) {
                                        wanted.push(c.path.clone());
                                    }
                                }
                            }
                            if c.duplicate {
                                let tag = Rect::from_min_size(inner.left_bottom() - Vec2::new(0.0, 18.0), egui::vec2(inner.width(), 18.0));
                                p.rect_filled(tag, 0, Color32::from_black_alpha(160));
                                p.text(tag.center(), egui::Align2::CENTER_CENTER, tr!("import-in-library"), egui::FontId::proportional(11.0), Color32::WHITE);
                            }
                            // the check circle, top left: click it to check / uncheck
                            let dot = rect.left_top() + Vec2::new(17.0, 17.0);
                            if checked {
                                p.circle_filled(dot, 10.0, t().accent);
                                p.text(dot, egui::Align2::CENTER_CENTER, icons::CHECK, egui::FontId::proportional(13.0), Color32::WHITE);
                            } else {
                                p.circle(dot, 10.0, Color32::from_black_alpha(90), Stroke::new(1.5, Color32::WHITE));
                            }
                            if st.current == Some(i) {
                                p.rect_stroke(rect, 6, Stroke::new(2.5, t().accent), StrokeKind::Inside);
                            }
                            if resp.clicked() {
                                let on_dot = resp.interact_pointer_pos().is_some_and(|q| q.distance(dot) < 14.0);
                                clicked = Some((i, on_dot));
                            }
                            if resp.double_clicked() {
                                clicked = Some((i, true));
                            }
                            resp.on_hover_text(format!("{}\n{} · {}", c.name, fmt_date(c.taken), fmt_bytes(c.bytes)));
                        }
                    }
                }
            }
        });
        if let Some(g) = toggle_group
            && let Some(r) = st.groups.get(g).cloned()
        {
            let shown = st.visible_in(&r);
            let all = shown.iter().all(|&i| st.checked.get(i).copied().unwrap_or(false));
            for i in shown {
                if let Some(c) = st.checked.get_mut(i) {
                    *c = !all;
                }
            }
        }
        if let Some((i, on_dot)) = clicked {
            if on_dot {
                st.toggle(i);
            }
            st.current = Some(i);
        }
        // forget thumbnails scrolled far away
        if st.thumbs.len() > MAX_THUMBS {
            let old: Vec<String> = st.thumbs.iter().filter(|(_, (_, seen))| now - *seen > 2.0).map(|(k, _)| k.clone()).collect();
            for k in old {
                st.thumbs.remove(&k);
                st.thumb_req.remove(&k);
            }
        }
        for key in wanted {
            st.thumb_req.insert(key.clone());
            self.lib.request(Job::File { key, dim: THUMB_DIM }, false);
        }
    }

    fn import_preview(&mut self, ui: &mut egui::Ui) {
        let Some(st) = self.import.as_mut() else { return };
        let current = st.current.and_then(|c| st.items.get(c).cloned());
        if let Some(c) = &current
            && st.preview.as_ref().is_none_or(|(k, _)| k != &c.path)
            && !st.thumb_req.contains(&format!("big:{}", c.path))
        {
            st.thumb_req.insert(format!("big:{}", c.path));
            self.lib.request(Job::File { key: c.path.clone(), dim: BIG_DIM }, true);
        }
        let mut toggle = false;
        let mut step = 0;
        let full = ui.available_width();
        egui::Panel::right("import_preview")
            .resizable(true)
            .default_size(full * 0.42)
            .min_size(260.0)
            .frame(egui::Frame::new().fill(t().edit_bg).inner_margin(12))
            .show(ui, |ui| {
                let Some(c) = &current else {
                    ui.centered_and_justified(|ui| ui.label(RichText::new(tr!("import-click-preview")).color(t().text_weak)));
                    return;
                };
                let info_h = 64.0;
                let area = Rect::from_min_size(ui.cursor().min, egui::vec2(ui.available_width(), (ui.available_height() - info_h).max(50.0)));
                let tex = match &st.preview {
                    Some((k, tex)) if k == &c.path => Some(tex.clone()),
                    _ => st.thumbs.get(&c.path).map(|(t, _)| t.clone()),
                };
                match tex {
                    Some(tex) => {
                        let size = tex.size_vec2();
                        let scale = (area.width() / size.x.max(1.0)).min(area.height() / size.y.max(1.0));
                        let img = Rect::from_center_size(area.center(), size * scale);
                        ui.painter().image(tex.id(), img, Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)), Color32::WHITE);
                    }
                    None => {
                        ui.put(Rect::from_center_size(area.center(), Vec2::splat(24.0)), egui::Spinner::new());
                    }
                }
                ui.allocate_rect(area, Sense::hover());
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui.button(icons::CHEVRON_LEFT).on_hover_text(tr!("ss-previous")).clicked() {
                        step = -1;
                    }
                    if ui.button(icons::CHEVRON_RIGHT).on_hover_text(tr!("ss-next")).clicked() {
                        step = 1;
                    }
                    let mut on = st.current.and_then(|i| st.checked.get(i).copied()).unwrap_or(false);
                    if ui.checkbox(&mut on, tr!("import-this")).on_hover_text(tr!("import-this-hint")).clicked() {
                        toggle = true;
                    }
                });
                let mut line = format!("{}  ·  {}, {}  ·  {}", c.name, fmt_date(c.taken), fmt_time(c.taken), fmt_bytes(c.bytes));
                if c.duplicate {
                    line += &format!("  ·  {}", tr!("import-already"));
                }
                ui.label(RichText::new(line).size(12.5).color(t().text_weak));
            });
        if toggle && let Some(i) = st.current {
            st.toggle(i);
        }
        if step != 0 {
            st.step(step);
        }
    }

    /// For `--screenshot` with ALBUMCRAFT_SHOT_VIEW=import: the sample pictures as if on a card.
    pub fn demo_import(&mut self) {
        let items: Vec<Candidate> = self
            .photos
            .iter()
            .enumerate()
            .take(22)
            .map(|(i, p)| Candidate {
                path: p.key.clone(),
                name: format!("IMG_{:04}.JPG", 4100 + i),
                taken: 1_746_265_000 + (i as i64) * 900 + if i >= 9 { 4 * 3600 } else { 0 } + if i >= 16 { 18 * 3600 } else { 0 },
                bytes: p.bytes * 20,
                duplicate: i >= 19,
            })
            .collect();
        self.open_import();
        let source = PathBuf::from("/media/card/DCIM");
        if let Some(st) = self.import.as_mut() {
            st.sources = vec![("EOS_DIGITAL".into(), source.clone())];
            st.source = Some(source.clone());
            st.name = "Spain by bike".into();
            st.dest_roots = vec!["/home/me/Pictures".into()];
            st.dest_root = "/home/me/Pictures".into();
        }
        self.on_import_list(source, items, false);
        if let Some(st) = self.import.as_mut() {
            st.checked[3] = false;
            st.checked[4] = false;
            st.current = Some(2);
            st.hide_dups = false;
        }
    }
}
