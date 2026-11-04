//! The modern Library shell: slim top bar, navigation sidebar, justified photo grid and a floating
//! selection bar. Replaces the old menu bar / toolbar / tray / status bar layout.
//!
//! Ideas borrowed from today's photo apps: no menu bar (everything sits behind one menu button),
//! pictures keep their real shape in tight rows, selection is a check circle on the picture, and the
//! actions for the selection float over the grid instead of living in a permanent tray.

#![allow(deprecated)] // egui::Id::new, as used elsewhere in this crate

use crate::app::*;
use crate::library::{fmt_date, local_year};
use crate::library::Flag;
use crate::library_view::{paint_flag, paint_rating, paint_reject};
use eframe::egui::{self, Color32, Pos2, Rect, RichText, Sense, Stroke, StrokeKind, Vec2};

const GAP: f32 = 4.0;
const PAD: i8 = 24;

// ---------------------------------------------------------------------------------------------
// small widgets

/// A square, frameless glyph button.
fn icon_btn(ui: &mut egui::Ui, glyph: &str, tip: &str) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(Vec2::splat(34.0), Sense::click());
    if resp.hovered() {
        ui.painter().rect_filled(rect, 17, t().surface_hover);
    }
    ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, glyph, egui::FontId::proportional(17.0), t().text);
    resp.on_hover_cursor(egui::CursorIcon::PointingHand).on_hover_text(tip)
}

/// A rounded text button. `primary` fills it with the accent colour.
fn pill(ui: &mut egui::Ui, label: &str, primary: bool) -> egui::Response {
    let galley = ui.painter().layout_no_wrap(
        label.to_string(),
        egui::FontId::proportional(13.5),
        if primary { Color32::WHITE } else { t().text },
    );
    let size = egui::vec2(galley.size().x + 28.0, 34.0);
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
    let fill = match (primary, resp.hovered()) {
        (true, false) => t().accent,
        (true, true) => t().accent.gamma_multiply(0.88),
        (false, false) => t().surface,
        (false, true) => t().surface_hover,
    };
    ui.painter().rect_filled(rect, 17, fill);
    if !primary {
        ui.painter().rect_stroke(rect, 17, Stroke::new(1.0, t().border), StrokeKind::Inside);
    }
    ui.painter().galley(rect.center() - galley.size() / 2.0, galley, Color32::WHITE);
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// A toggle chip (filters).
fn chip(ui: &mut egui::Ui, label: &str, on: bool) -> egui::Response {
    let color = if on { t().accent } else { t().text };
    let galley = ui.painter().layout_no_wrap(label.to_string(), egui::FontId::proportional(13.0), color);
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(galley.size().x + 24.0, 30.0), Sense::click());
    let fill = if on {
        t().select_bg
    } else if resp.hovered() {
        t().surface_hover
    } else {
        Color32::TRANSPARENT
    };
    ui.painter().rect_filled(rect, 15, fill);
    ui.painter().rect_stroke(rect, 15, Stroke::new(1.0, if on { t().accent } else { t().border }), StrokeKind::Inside);
    ui.painter().galley(rect.center() - galley.size() / 2.0, galley, color);
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// One row of the sidebar.
fn nav_row(ui: &mut egui::Ui, icon: &str, label: &str, count: Option<usize>, selected: bool) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 32.0), Sense::click());
    let p = ui.painter();
    if selected {
        p.rect_filled(rect, 9, t().select_bg);
    } else if resp.hovered() {
        p.rect_filled(rect, 9, t().surface_hover);
    }
    let color = if selected { t().accent } else { t().text };
    if !icon.is_empty() {
        p.text(Pos2::new(rect.left() + 18.0, rect.center().y), egui::Align2::CENTER_CENTER, icon, egui::FontId::proportional(15.0), color);
    }
    let galley = p.layout(label.to_string(), egui::FontId::proportional(13.5), color, rect.width() - 80.0);
    p.galley(Pos2::new(rect.left() + 36.0, rect.center().y - galley.size().y / 2.0), galley, color);
    if let Some(n) = count {
        p.text(
            Pos2::new(rect.right() - 10.0, rect.center().y),
            egui::Align2::RIGHT_CENTER,
            n.to_string(),
            egui::FontId::proportional(12.0),
            t().text_weak,
        );
    }
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

fn caption(ui: &mut egui::Ui, text: &str) {
    ui.add_space(14.0);
    ui.label(RichText::new(text.to_uppercase()).size(11.0).strong().color(t().text_weak));
    ui.add_space(2.0);
}

/// Splits `aspects` into rows that fill `width`: (start, end, row height).
fn justify(aspects: &[f32], width: f32, target: f32) -> Vec<(usize, usize, f32)> {
    let mut rows = vec![];
    let (mut start, mut sum) = (0usize, 0.0f32);
    for (i, a) in aspects.iter().enumerate() {
        sum += a;
        let n = (i - start) as f32;
        if sum * target + GAP * n >= width {
            rows.push((start, i + 1, (width - GAP * n) / sum));
            start = i + 1;
            sum = 0.0;
        }
    }
    if start < aspects.len() {
        rows.push((start, aspects.len(), target));
    }
    rows
}

// ---------------------------------------------------------------------------------------------

impl App {
    pub fn library_ui(&mut self, ui: &mut egui::Ui) {
        self.library_keys(ui.ctx());
        if !self.selection.is_empty() {
            self.tray_set = None;
        }
        self.status_bar(ui);
        self.tray_panel(ui);
        self.sidebar(ui);
        self.top_bar(ui);
        self.grid(ui);
    }

    fn source_title(&self) -> String {
        match &self.source {
            Source::Library => "All pictures".to_string(),
            Source::Album(i) => self.store.albums.get(*i).map(|a| a.name.clone()).unwrap_or_default(),
            Source::Person(n) => n.clone(),
            Source::Picks => "Picks".to_string(),
        }
    }

    fn top_bar(&mut self, ui: &mut egui::Ui) {
        egui::Panel::top("topbar")
            .exact_size(60.0)
            .frame(egui::Frame::new().fill(t().lightbox_bg).inner_margin(egui::Margin::symmetric(PAD, 8)))
            .show(ui, |ui| {
                let bottom = ui.max_rect().bottom() + 8.0;
                ui.painter().hline(ui.max_rect().x_range().expand(PAD as f32), bottom, Stroke::new(1.0, t().border));
                let wide = ui.available_width();
                ui.horizontal_centered(|ui| {
                    let shown = self.visible_flat().len();
                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing.y = 0.0;
                        ui.label(RichText::new(self.source_title()).size(20.0).strong().color(t().text_strong));
                        let sub = if self.scanning {
                            "Looking for pictures…".to_string()
                        } else {
                            format!("{shown} picture{}", if shown == 1 { "" } else { "s" })
                        };
                        ui.label(RichText::new(sub).size(12.0).color(t().text_weak));
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.spacing_mut().item_spacing.x = 8.0;
                        if pill(ui, "＋  Import", true).on_hover_text("Add a folder of pictures to the library").clicked() {
                            self.import_folder();
                        }
                        use crate::theme::Mode;
                        let mode = Mode::from_name(&self.store.theme);
                        if icon_btn(ui, mode.icon(), &format!("Appearance: {} (click to change)", mode.label())).clicked() {
                            self.store.theme = mode.next().name().into();
                            self.dirty = true;
                        }
                        ui.add_space(4.0);
                        if wide > 980.0 {
                            ui.spacing_mut().slider_width = 84.0;
                            if ui.add(egui::Slider::new(&mut self.store.thumb_size, 64.0..=256.0).show_value(false)).on_hover_text("Picture size").changed() {
                                self.dirty = true;
                            }
                            ui.add_space(2.0);
                        }
                        // rating / rejects filter (a small menu), then the Picks toggle
                        let f = self.filter.clone();
                        let label = match (f.min_rating, f.hide_rejects) {
                            (0, false) => "★  Any".to_string(),
                            (0, true) => "★  Any, no rejects".to_string(),
                            (n, false) => format!("★  {n}+"),
                            (n, true) => format!("★  {n}+, no rejects"),
                        };
                        let resp = chip(ui, &label, f.min_rating > 0 || f.hide_rejects).on_hover_text("Filter by rating; hide rejected pictures");
                        egui::Popup::menu(&resp).show(|ui| {
                            for n in 0..=crate::library::MAX_RATING {
                                let text = if n == 0 { "Any rating".to_string() } else { format!("{} or more", crate::library_view::rating_label(n)) };
                                if ui.radio(self.filter.min_rating == n, text).clicked() {
                                    self.filter.min_rating = n;
                                }
                            }
                            ui.separator();
                            ui.checkbox(&mut self.filter.hide_rejects, "Hide rejected pictures");
                        });
                        let on = self.filter.picks;
                        if chip(ui, if wide > 800.0 { "⚑  Picks" } else { "⚑" }, on).on_hover_text("Show only picks").clicked() {
                            self.filter.picks = !on;
                        }
                        ui.add_space(4.0);
                        egui::Frame::new()
                            .fill(t().surface)
                            .stroke(Stroke::new(1.0, t().border))
                            .corner_radius(18)
                            .inner_margin(egui::Margin::symmetric(12, 3))
                            .show(ui, |ui| {
                                let sw = ((wide - 640.0) * 0.5).clamp(70.0, 190.0);
                                ui.allocate_ui_with_layout(egui::vec2(sw + 52.0, 26.0), egui::Layout::left_to_right(egui::Align::Center), |ui| {
                                    ui.label(RichText::new("🔍").size(13.0).color(t().text_weak));
                                    let r = ui.add(
                                        egui::TextEdit::singleline(&mut self.search)
                                            .hint_text("Search pictures")
                                            .frame(egui::Frame::NONE)
                                            .desired_width(sw),
                                    );
                                    if r.changed() {
                                        self.selection.clear();
                                    }
                                    if !self.search.is_empty() && ui.add(egui::Label::new(RichText::new("✕").color(t().text_weak)).sense(Sense::click())).clicked() {
                                        self.search.clear();
                                    }
                                });
                            });
                    });
                });
            });
    }

    fn sidebar(&mut self, ui: &mut egui::Ui) {
        egui::Panel::left("folders")
            .resizable(true)
            .default_size(248.0)
            .size_range(200.0..=380.0)
            .frame(egui::Frame::new().fill(t().left_bg).inner_margin(egui::Margin::symmetric(12, 14)).stroke(Stroke::new(1.0, t().border)))
            .show(ui, |ui| {
                // brand
                ui.horizontal(|ui| {
                    let (r, _) = ui.allocate_exact_size(Vec2::splat(28.0), Sense::hover());
                    ui.painter().rect_filled(r, 9, t().accent);
                    ui.painter().text(r.center(), egui::Align2::CENTER_CENTER, "A", egui::FontId::proportional(16.0), Color32::WHITE);
                    ui.label(RichText::new("AlbumCraft").size(16.0).strong().color(t().text_strong));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.menu_button(RichText::new("☰").size(17.0), |ui| {
                            ui.set_min_width(190.0);
                            self.app_menu_items(ui);
                        });
                    });
                });
                ui.add_space(8.0);

                egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                    ui.spacing_mut().item_spacing.y = 2.0;
                    let all_on = self.source == Source::Library && self.search.is_empty() && self.list_pick.is_none();
                    if nav_row(ui, "🖼", "All pictures", Some(self.photos.len()), all_on).clicked() {
                        self.source = Source::Library;
                        self.selection.clear();
                        self.tray_set = None;
                        self.list_pick = None;
                    }
                    let npicks = (0..self.photos.len()).filter(|&i| self.is_picked(i)).count();
                    let r = nav_row(ui, "", "Picks", Some(npicks), self.source == Source::Picks);
                    paint_flag(ui.painter(), Pos2::new(r.rect.left() + 18.0, r.rect.center().y), 7.0, t().pick, t().pick);
                    if r.clicked() {
                        self.source = Source::Picks;
                        self.pick_set(TraySet::Picks);
                    }

                    ui.add_space(14.0);
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("ALBUMS").size(11.0).strong().color(t().text_weak));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if icon_btn(ui, "＋", "New album").clicked() {
                                self.dialog = Some(Dialog::NewAlbum { name: String::new() });
                            }
                        });
                    });
                    let mut delete = None;
                    for i in 0..self.store.albums.len() {
                        let a = &self.store.albums[i];
                        let n = a.keys.iter().filter(|k| self.key_index.contains_key(*k)).count();
                        let r = nav_row(ui, "📚", &a.name.clone(), Some(n), self.source == Source::Album(i));
                        if r.clicked() {
                            self.source = Source::Album(i);
                            self.pick_set(TraySet::Album(i));
                        }
                        r.context_menu(|ui| {
                            if ui.button("Rename…").clicked() {
                                self.dialog = Some(Dialog::RenameAlbum { idx: i, name: self.store.albums[i].name.clone() });
                                ui.close();
                            }
                            if ui.button("Delete album").clicked() {
                                delete = Some(i);
                                ui.close();
                            }
                            if ui.button("Play as slideshow").clicked() {
                                self.source = Source::Album(i);
                                let list = self.visible_flat();
                                let first = list.first().copied().unwrap_or(0);
                                self.start_slideshow(list, first);
                                ui.close();
                            }
                        });
                    }
                    if let Some(i) = delete {
                        self.store.albums.remove(i);
                        self.dirty = true;
                        self.source = Source::Library;
                    }

                    caption(ui, "People");
                    let people = self.people();
                    if people.is_empty() {
                        ui.label(RichText::new("Tag people from the selection bar.").size(12.0).color(t().text_weak));
                    }
                    for (name, n) in people {
                        let sel = self.source == Source::Person(name.clone());
                        if nav_row(ui, "👤", &name, Some(n), sel).clicked() {
                            self.source = Source::Person(name.clone());
                            self.pick_set(TraySet::Person(name));
                        }
                    }

                    caption(ui, "Folders");
                    let mut by_year: Vec<(i32, Vec<usize>)> = vec![];
                    for (fi, f) in self.folders.iter().enumerate() {
                        let y = local_year(f.date);
                        match by_year.last_mut() {
                            Some((yy, v)) if *yy == y => v.push(fi),
                            _ => by_year.push((y, vec![fi])),
                        }
                    }
                    let current_folder = match &self.list_pick {
                        Some(TraySet::Folder(fi)) if self.selection.is_empty() => Some(*fi),
                        _ => self.selection.iter().next().map(|&id| self.photos[id].folder),
                    };
                    for (year, fis) in by_year {
                        egui::CollapsingHeader::new(RichText::new(year.to_string()).size(13.0).strong().color(t().text))
                            .id_salt(("year", year))
                            .default_open(true)
                            .show(ui, |ui| {
                                for fi in fis {
                                    let f = &self.folders[fi];
                                    let r = nav_row(ui, "📁", &f.name.clone(), Some(f.photos.len()), current_folder == Some(fi) && self.source == Source::Library);
                                    if r.clicked() {
                                        self.source = Source::Library;
                                        self.scroll_to_folder = Some(fi);
                                        self.pick_set(TraySet::Folder(fi));
                                    }
                                    r.on_hover_text(&self.folders[fi].path);
                                }
                            });
                    }
                });
            });
    }

    /// Width / height of a picture as far as it is known.
    fn aspect_of(&self, id: usize) -> f32 {
        if let Some(tex) = self.thumbs.get(&id) {
            let [w, h] = tex.size();
            return w as f32 / h.max(1) as f32;
        }
        if let Some(&(w, h)) = self.real_dims.get(&id) {
            return w as f32 / h.max(1) as f32;
        }
        1.5
    }

    fn grid(&mut self, ui: &mut egui::Ui) {
        egui::CentralPanel::default().frame(egui::Frame::new().fill(t().lightbox_bg)).show(ui, |ui| {
            let groups = self.groups();
            if groups.iter().all(|(_, v)| v.is_empty()) {
                self.empty_state(ui);
                return;
            }
            let flat: Vec<usize> = groups.iter().flat_map(|(_, v)| v.iter().copied()).collect();
            // learn the real shape of pictures whose thumbnail is not loaded yet (a few per frame)
            let mut budget = 120;
            for &id in &flat {
                if budget == 0 {
                    ui.ctx().request_repaint();
                    break;
                }
                if !self.thumbs.contains_key(&id) && !self.real_dims.contains_key(&id) {
                    self.real_dims_of(id);
                    budget -= 1;
                }
            }

            let target = (self.store.thumb_size * 1.25).clamp(90.0, 320.0);
            let mut open: Option<usize> = None;
            let mut clicked: Option<(usize, egui::Modifiers)> = None;
            let mut background_click = false;

            egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                egui::Frame::new().inner_margin(egui::Margin { left: PAD, right: PAD, top: 8, bottom: 16 }).show(ui, |ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(GAP, GAP);
                    let width = ui.available_width();
                    for (folder, ids) in &groups {
                        if let Some(fi) = folder {
                            let r = self.group_header(ui, *fi, ids);
                            if self.scroll_to_folder == Some(*fi) {
                                r.scroll_to_me(Some(egui::Align::TOP));
                                self.scroll_to_folder = None;
                            }
                        }
                        let aspects: Vec<f32> = ids.iter().map(|&id| self.aspect_of(id)).collect();
                        for (a, b, h) in justify(&aspects, width, target) {
                            let (row, _) = ui.allocate_exact_size(egui::vec2(width, h), Sense::hover());
                            if !ui.is_rect_visible(row) {
                                continue;
                            }
                            // when the row is the last, stretched or not, pictures keep their own width
                            let mut x = row.left();
                            for k in a..b {
                                let id = ids[k];
                                let w = aspects[k] * h;
                                let rect = Rect::from_min_size(Pos2::new(x, row.top()), egui::vec2(w, h));
                                x += w + GAP;
                                if !self.thumbs.contains_key(&id) {
                                    self.request_thumb(id);
                                }
                                if self.scroll_to_photo == Some(id) {
                                    ui.scroll_to_rect(rect, Some(egui::Align::Center));
                                    self.scroll_to_photo = None;
                                }
                                let resp = ui.interact(rect, egui::Id::new(("tile", id)), Sense::click());
                                let hovered = resp.contains_pointer();
                                self.paint_tile(ui, rect, id, hovered);
                                let (check, pick) = self.tile_buttons(ui, rect, id, hovered);
                                if check {
                                    let mut m = ui.input(|i| i.modifiers);
                                    m.command = true;
                                    m.shift = false;
                                    clicked = Some((id, m));
                                } else if pick {
                                    self.toggle_flag(&[id], Flag::Pick);
                                } else if resp.double_clicked() {
                                    open = Some(id);
                                } else if resp.clicked() {
                                    clicked = Some((id, ui.input(|i| i.modifiers)));
                                }
                                let name = self.photos[id].name.clone();
                                let resp = resp.on_hover_text(name);
                                self.cell_menu(&resp, id);
                            }
                        }
                        ui.add_space(14.0);
                    }
                    let rest = ui.available_rect_before_wrap();
                    if rest.height() > 10.0 && ui.allocate_rect(rest, Sense::click()).clicked() {
                        background_click = true;
                    }
                });
            });

            if let Some((id, m)) = clicked {
                if m.shift {
                    if let Some(a) = self.anchor {
                        let (pa, pb) = (flat.iter().position(|&x| x == a), flat.iter().position(|&x| x == id));
                        if let (Some(pa), Some(pb)) = (pa, pb) {
                            let (lo, hi) = (pa.min(pb), pa.max(pb));
                            if !m.command {
                                self.selection.clear();
                            }
                            self.selection.extend(flat[lo..=hi].iter().copied());
                        }
                    } else {
                        self.selection.insert(id);
                    }
                } else if m.command {
                    if !self.selection.remove(&id) {
                        self.selection.insert(id);
                    }
                    self.anchor = Some(id);
                } else {
                    self.selection.clear();
                    self.selection.insert(id);
                    self.anchor = Some(id);
                }
            } else if background_click {
                self.selection.clear();
                self.tray_set = None;
                self.list_pick = None;
            }
            if let Some(id) = open {
                self.open_editor(id);
            }
        });
    }

    fn empty_state(&mut self, ui: &mut egui::Ui) {
        ui.vertical_centered(|ui| {
            ui.add_space(ui.available_height() * 0.3);
            let (icon, msg) = if self.scanning {
                ("⏳", "Looking for pictures…")
            } else if !self.search.is_empty() || self.filter.is_active() {
                ("🔍", "Nothing matches your search or filter.")
            } else {
                match self.source {
                    Source::Album(_) => ("📚", "This album is empty. Select pictures and add them from the selection bar."),
                    Source::Person(_) => ("👤", "Nobody tagged yet."),
                    Source::Picks => ("⚑", "No picks yet. Hover a picture and click its flag, or press P."),
                    Source::Library => ("🖼", "Your library is empty. Import a folder to get started."),
                }
            };
            ui.label(RichText::new(icon).size(42.0));
            ui.add_space(6.0);
            ui.label(RichText::new(msg).size(15.0).color(t().text_weak));
            if matches!(self.source, Source::Library) && self.search.is_empty() && !self.scanning {
                ui.add_space(12.0);
                if pill(ui, "＋  Import a folder", true).clicked() {
                    self.import_folder();
                }
            }
        });
    }

    fn group_header(&mut self, ui: &mut egui::Ui, fi: usize, ids: &[usize]) -> egui::Response {
        let f = self.folders[fi].clone();
        let note = self.store.folder_notes.get(&f.path).cloned();
        let inner = ui.vertical(|ui| {
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                ui.label(RichText::new(&f.name).size(19.0).strong().color(t().text_strong));
                ui.label(RichText::new(format!("{}  ·  {}", fmt_date(f.date), ids.len())).size(12.5).color(t().text_weak));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if icon_btn(ui, "▶", "Play this folder as a slideshow").clicked() {
                        self.start_slideshow(ids.to_vec(), ids[0]);
                    }
                    if icon_btn(ui, "✏", "Edit description").clicked() {
                        self.dialog = Some(Dialog::FolderNote { path: f.path.clone(), text: note.clone().unwrap_or_default() });
                    }
                });
            });
            if let Some(n) = &note {
                ui.label(RichText::new(n).size(13.0).color(t().text));
            }
            ui.add_space(4.0);
        });
        inner.response
    }

    fn paint_tile(&self, ui: &egui::Ui, rect: Rect, id: usize, hovered: bool) {
        let p = ui.painter();
        let selected = self.selection.contains(&id);
        let radius = 6u8;
        let img = if selected {
            p.rect_filled(rect, radius + 2, t().select_bg);
            rect.shrink((rect.width().min(rect.height()) * 0.07).clamp(6.0, 14.0))
        } else {
            rect
        };
        match self.thumbs.get(&id) {
            Some(tex) if !self.thumb_failed.contains(&id) => {
                p.add(
                    egui::epaint::RectShape::filled(img, radius, Color32::WHITE)
                        .with_texture(tex.id(), Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0))),
                );
            }
            _ => {
                p.rect_filled(img, radius, t().placeholder);
                if self.thumb_failed.contains(&id) {
                    p.text(img.center(), egui::Align2::CENTER_CENTER, "?", egui::FontId::proportional(20.0), t().text_weak);
                }
            }
        }
        if hovered || selected {
            // soft shade behind the check circle and star
            let strip = Rect::from_min_size(img.min + egui::vec2(2.0, 1.0), egui::vec2(img.width() - 4.0, 48.0_f32.min(img.height())));
            let mut mesh = egui::Mesh::default();
            let (top, clear) = (Color32::from_black_alpha(80), Color32::TRANSPARENT);
            for (pos, col) in [(strip.left_top(), top), (strip.right_top(), top), (strip.right_bottom(), clear), (strip.left_bottom(), clear)] {
                mesh.vertices.push(egui::epaint::Vertex { pos, uv: egui::epaint::WHITE_UV, color: col });
            }
            mesh.indices.extend([0, 1, 2, 0, 2, 3]);
            p.add(egui::Shape::mesh(mesh));
        }
        if !selected && hovered {
            p.rect_filled(img, radius, Color32::from_white_alpha(14));
        }
        let m = self.meta(id);
        let flag = m.map(|m| m.flag).unwrap_or_default();
        if flag == Flag::Reject {
            // rejected: dimmed, with the reject mark
            p.rect_filled(img, radius, Color32::from_black_alpha(120));
            if !hovered {
                paint_reject(p, Pos2::new(img.right() - 14.0, img.top() + 14.0), 8.0, t().reject, Color32::from_black_alpha(140));
            }
        }
        // bottom left: pick flag, then the rating stars
        let mut x = img.left() + 14.0;
        let y = img.bottom() - 14.0;
        if flag == Flag::Pick && !hovered {
            paint_flag(p, Pos2::new(x, y), 7.0, t().pick, Color32::from_black_alpha(160));
            x += 16.0;
        }
        let rating = m.map(|m| m.rating).unwrap_or(0);
        if rating > 0 && img.width() > 70.0 {
            paint_rating(p, Pos2::new(x - 6.0, y), rating, 5.0, Color32::from_black_alpha(140));
        }
        if m.is_some_and(|m| !m.edits.is_empty()) {
            let c = Pos2::new(img.right() - 12.0, img.bottom() - 12.0);
            p.circle_filled(c, 5.0, Color32::from_rgb(80, 190, 110));
            p.circle_stroke(c, 5.0, Stroke::new(1.0, Color32::WHITE));
        }
        if self.held.contains(&id) {
            p.text(Pos2::new(img.right() - 6.0, img.bottom() - 28.0), egui::Align2::RIGHT_BOTTOM, "📌", egui::FontId::proportional(13.0), Color32::WHITE);
        }
    }

    /// The check circle (top left) and pick flag (top right); returns what was clicked.
    fn tile_buttons(&self, ui: &mut egui::Ui, rect: Rect, id: usize, hovered: bool) -> (bool, bool) {
        let selected = self.selection.contains(&id);
        let inset = if selected { (rect.width().min(rect.height()) * 0.07).clamp(6.0, 14.0) } else { 0.0 };
        let img = rect.shrink(inset);
        if !(hovered || selected) || img.width() < 60.0 || img.height() < 50.0 {
            return (false, false);
        }
        let c = img.left_top() + egui::vec2(18.0, 18.0);
        let check_r = Rect::from_center_size(c, Vec2::splat(30.0));
        let check = ui.interact(check_r, egui::Id::new(("chk", id)), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand);
        let p = ui.painter();
        if selected {
            p.circle_filled(c, 10.0, t().accent);
            p.circle_stroke(c, 10.0, Stroke::new(1.5, Color32::WHITE));
            let s = Stroke::new(2.0, Color32::WHITE);
            p.line_segment([c + egui::vec2(-4.5, 0.5), c + egui::vec2(-1.5, 3.5)], s);
            p.line_segment([c + egui::vec2(-1.5, 3.5), c + egui::vec2(4.5, -3.0)], s);
        } else {
            p.circle_filled(c, 10.0, if check.hovered() { Color32::from_black_alpha(110) } else { Color32::from_black_alpha(60) });
            p.circle_stroke(c, 10.0, Stroke::new(1.8, Color32::WHITE));
        }
        let fc = img.right_top() + egui::vec2(-18.0, 18.0);
        let flag_r = Rect::from_center_size(fc, Vec2::splat(30.0));
        let flag = ui.interact(flag_r, egui::Id::new(("pick", id)), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand).on_hover_text("Pick (P)");
        let on = self.is_picked(id);
        let p = ui.painter();
        let r = if flag.hovered() { 9.0 } else { 8.0 };
        paint_flag(p, fc, r, if on { t().pick } else { Color32::TRANSPARENT }, Color32::WHITE);
        (check.clicked(), flag.clicked())
    }
}
