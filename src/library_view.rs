//! The Library view: menu, toolbar, folder list, lightbox of thumbnails and the photo tray.

use crate::icons;
use crate::app::*;
use crate::library::{fmt_bytes, Flag, MAX_RATING};
use eframe::egui::{self, Color32, Pos2, Rect, RichText, Sense, Stroke, StrokeKind, Vec2};

/// A big tray button: icon above a label.
pub fn big_button(ui: &mut egui::Ui, icon: &str, label: &str, tip: &str) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(62.0, 60.0), Sense::click());
    let p = ui.painter();
    if resp.hovered() {
        p.rect_filled(rect, 6, t().hover_wash);
        p.rect_stroke(rect, 6, Stroke::new(1.0, t().select_border), StrokeKind::Inside);
    }
    p.text(
        rect.center_top() + egui::vec2(0.0, 20.0),
        egui::Align2::CENTER_CENTER,
        icon,
        egui::FontId::proportional(24.0),
        t().text,
    );
    p.text(
        rect.center_bottom() - egui::vec2(0.0, 10.0),
        egui::Align2::CENTER_CENTER,
        label,
        egui::FontId::proportional(12.0),
        t().text,
    );
    resp.on_hover_text(tip)
}

/// A five-pointed star drawn with shapes (the UI font has no star glyph).
pub fn paint_star(p: &egui::Painter, c: Pos2, r: f32, fill: Color32, outline: Color32) {
    let pts: Vec<Pos2> = (0..10)
        .map(|i| {
            let a = -std::f32::consts::FRAC_PI_2 + i as f32 * std::f32::consts::PI / 5.0;
            let rr = if i % 2 == 0 { r } else { r * 0.45 };
            Pos2::new(c.x + rr * a.cos(), c.y + rr * a.sin())
        })
        .collect();
    // fill as triangles from the centre so the concave shape renders correctly
    for i in 0..10 {
        let tri = vec![c, pts[i], pts[(i + 1) % 10]];
        p.add(egui::Shape::convex_polygon(tri, fill, Stroke::NONE));
    }
    p.add(egui::Shape::closed_line(pts, Stroke::new(1.0, outline)));
}

/// A pick flag (pole and pennant) centred on `c`, `r` = half its height.
pub fn paint_flag(p: &egui::Painter, c: Pos2, r: f32, fill: Color32, outline: Color32) {
    let x = c.x - r * 0.55;
    let (top, bottom) = (c.y - r, c.y + r);
    let pennant = vec![Pos2::new(x, top), Pos2::new(x + r * 1.25, top + r * 0.45), Pos2::new(x, top + r * 0.9)];
    p.add(egui::Shape::convex_polygon(pennant, fill, Stroke::new(1.0, outline)));
    p.line_segment([Pos2::new(x, top), Pos2::new(x, bottom)], Stroke::new((r * 0.22).max(1.2), outline));
}

/// The reject mark: a crossed circle centred on `c`.
pub fn paint_reject(p: &egui::Painter, c: Pos2, r: f32, color: Color32, bg: Color32) {
    p.circle_filled(c, r, bg);
    p.circle_stroke(c, r, Stroke::new(1.5, color));
    let d = r * 0.45;
    let s = Stroke::new(1.8, color);
    p.line_segment([c + egui::vec2(-d, -d), c + egui::vec2(d, d)], s);
    p.line_segment([c + egui::vec2(-d, d), c + egui::vec2(d, -d)], s);
}

/// `n` small filled stars in a row starting at `left` (vertically centred on it).
pub fn paint_rating(p: &egui::Painter, left: Pos2, n: u8, r: f32, outline: Color32) {
    for i in 0..n {
        let c = left + egui::vec2(r + i as f32 * r * 2.2, 0.0);
        paint_star(p, c, r, t().star, outline);
    }
}

/// A square button showing a pick flag, filled when on.
pub fn pick_button(ui: &mut egui::Ui, on: bool, size: Vec2) -> egui::Response {
    let resp = ui.add_sized(size, egui::Button::new(""));
    let (fill, outline) = if on { (t().pick, t().pick) } else { (Color32::TRANSPARENT, t().text_weak) };
    paint_flag(ui.painter(), resp.rect.center(), size.y.min(size.x) * 0.3, fill, outline);
    resp
}

/// A square button showing the reject mark, coloured when on.
pub fn reject_button(ui: &mut egui::Ui, on: bool, size: Vec2) -> egui::Response {
    let resp = ui.add_sized(size, egui::Button::new(""));
    let color = if on { t().reject } else { t().text_weak };
    paint_reject(ui.painter(), resp.rect.center(), size.y.min(size.x) * 0.26, color, Color32::TRANSPARENT);
    resp
}

/// Five stars to click: returns the new rating (clicking the current rating clears it).
pub fn rating_input(ui: &mut egui::Ui, base: egui::Id, current: u8, r: f32) -> Option<u8> {
    let step = r * 2.4;
    let (rect, _) = ui.allocate_exact_size(egui::vec2(step * MAX_RATING as f32, r * 2.6), Sense::hover());
    let mut chosen = None;
    let hover_n = ui.ctx().pointer_hover_pos().filter(|p| rect.contains(*p)).map(|p| ((p.x - rect.left()) / step).floor() as u8 + 1);
    for i in 0..MAX_RATING {
        let c = Pos2::new(rect.left() + step * (i as f32 + 0.5), rect.center().y);
        let cell = Rect::from_center_size(c, egui::vec2(step, rect.height()));
        let resp = ui.interact(cell, base.with(i), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand);
        let n = i + 1;
        let lit = hover_n.map_or(n <= current, |h| n <= h);
        let fill = if lit { t().star } else { Color32::TRANSPARENT };
        let outline = if lit { t().star_outline } else { t().text_weak };
        paint_star(ui.painter(), c, r, fill, outline);
        if resp.clicked() {
            chosen = Some(if current == n { 0 } else { n });
        }
    }
    chosen
}

/// "No rating", "1 star", "3 stars".
pub fn rating_label(n: u8) -> String {
    match n {
        0 => "No rating".into(),
        1 => "1 star".into(),
        n => format!("{n} stars"),
    }
}

impl App {
    /// Pick / reject / rating entries for a menu, acting on `ids`.
    pub fn flag_rating_menu(&mut self, ui: &mut egui::Ui, ids: &[usize]) {
        let all = |f: Flag| !ids.is_empty() && ids.iter().all(|&id| self.flag(id) == f);
        let (picked, rejected) = (all(Flag::Pick), all(Flag::Reject));
        if ui.button(if picked { "Unpick              P" } else { "Pick                P" }).clicked() {
            self.toggle_flag(ids, Flag::Pick);
            ui.close();
        }
        if ui.button(if rejected { "Unreject            X" } else { "Reject              X" }).clicked() {
            self.toggle_flag(ids, Flag::Reject);
            ui.close();
        }
        ui.menu_button("Rating", |ui| {
            for n in (0..=MAX_RATING).rev() {
                if ui.button(format!("{:<12}  {n}", rating_label(n))).clicked() {
                    self.set_rating(ids, n);
                    ui.close();
                }
            }
        });
    }

    /// "Copy edits" (from a single picture) and "Paste edits" (onto `ids`).
    pub fn edit_clipboard_menu(&mut self, ui: &mut egui::Ui, ids: &[usize]) {
        let one = if let [one] = ids { Some(*one) } else { None };
        let has = one.is_some_and(|id| !crate::imaging::copyable_edits(&self.edits(id)).is_empty());
        if ui.add_enabled(has, egui::Button::new("Copy edits          Ctrl+Shift+C")).clicked() {
            if let Some(id) = one {
                self.copy_edits(id);
            }
            ui.close();
        }
        let n = self.copied_edits.as_ref().map_or(0, |e| e.len());
        let label = if n > 0 { format!("Paste {n} edit{}       Ctrl+Shift+V", if n == 1 { "" } else { "s" }) } else { "Paste edits         Ctrl+Shift+V".into() };
        if ui.add_enabled(n > 0 && !ids.is_empty(), egui::Button::new(label)).clicked() {
            self.paste_edits(ids);
            ui.close();
        }
    }

    /// Keys for culling, acting on `ids`: P pick, X reject, U clear the flag, 0–5 rating.
    pub fn flag_rating_keys(&mut self, ctx: &egui::Context, ids: &[usize]) {
        if ids.is_empty() {
            return;
        }
        use egui::Key;
        let keys = [Key::Num0, Key::Num1, Key::Num2, Key::Num3, Key::Num4, Key::Num5];
        let (pick, reject, unflag, rating) = ctx.input(|i| {
            let plain = !i.modifiers.command && !i.modifiers.alt;
            (
                plain && i.key_pressed(Key::P),
                plain && i.key_pressed(Key::X),
                plain && i.key_pressed(Key::U),
                if plain { keys.iter().position(|k| i.key_pressed(*k)) } else { None },
            )
        });
        if pick {
            self.toggle_flag(ids, Flag::Pick);
        }
        if reject {
            self.toggle_flag(ids, Flag::Reject);
        }
        if unflag {
            self.set_flag(ids, Flag::None);
        }
        if let Some(n) = rating {
            self.set_rating(ids, n as u8);
        }
    }
}

pub fn paint_thumb(p: &egui::Painter, cell: Rect, tex: Option<&egui::TextureHandle>, shadow: bool) -> Rect {
    match tex {
        Some(t) => {
            let [w, h] = t.size();
            let s = (cell.width() / w as f32).min(cell.height() / h as f32);
            let r = Rect::from_center_size(cell.center(), egui::vec2(w as f32 * s, h as f32 * s));
            if shadow {
                p.rect_filled(r.translate(egui::vec2(2.0, 2.0)), 1, Color32::from_black_alpha(45));
            }
            p.image(t.id(), r, Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)), Color32::WHITE);
            r
        }
        None => {
            let r = Rect::from_center_size(cell.center(), cell.size() * egui::vec2(1.0, 0.75));
            p.rect_filled(r, 2, t().placeholder);
            r
        }
    }
}

impl App {
    pub fn library_keys(&mut self, ctx: &egui::Context) {
        if self.dialog.is_some() || self.text_focused() {
            return;
        }
        let (all, esc, enter, del, ctrl_r, ctrl_l) = ctx.input(|i| {
            (
                i.modifiers.command && i.key_pressed(egui::Key::A),
                i.key_pressed(egui::Key::Escape),
                i.key_pressed(egui::Key::Enter),
                i.key_pressed(egui::Key::Delete),
                i.modifiers.command && i.key_pressed(egui::Key::CloseBracket),
                i.modifiers.command && i.key_pressed(egui::Key::OpenBracket),
            )
        });
        if all {
            self.selection = self.visible_flat().into_iter().collect();
        }
        if esc {
            self.selection.clear();
            self.tray_set = None;
        }
        if enter
            && let Some(&id) = self.selection.iter().next() {
                self.open_editor(id);
            }
        if del
            && let Source::Album(i) = self.source {
                let keys: Vec<String> = self.selection.iter().map(|&id| self.photos[id].key.clone()).collect();
                if let Some(a) = self.store.albums.get_mut(i) {
                    a.keys.retain(|k| !keys.contains(k));
                    self.dirty = true;
                }
                self.selection.clear();
            }
        if ctrl_r || ctrl_l {
            let t = self.tray();
            self.rotate(&t, if ctrl_r { 1 } else { -1 });
        }
        if !self.selection.is_empty() || !self.held.is_empty() {
            let t = self.tray();
            self.flag_rating_keys(ctx, &t);
            let (copy, paste) = edit_clipboard_keys(ctx);
            if copy {
                if let [one] = t[..] {
                    self.copy_edits(one);
                } else {
                    self.toast("Select one picture to copy its edits.");
                }
            }
            if paste {
                self.paste_edits(&t);
            }
        }
    }

    pub fn app_menu_items(&mut self, ui: &mut egui::Ui) {
                let tray = self.targets();
                ui.menu_button("File", |ui| {
                    if ui.button("Add folder to library…").clicked() {
                        self.import_folder();
                        ui.close();
                    }
                    if ui.button("Folder manager…").clicked() {
                        self.dialog = Some(Dialog::FolderManager);
                        ui.close();
                    }
                    ui.separator();
                    if ui.button("Export pictures…").clicked() {
                        self.dialog = Some(Dialog::Export { max_dim: 0, quality: 90 });
                        ui.close();
                    }
                    if ui.button("Make a web gallery…").clicked() {
                        self.open_gallery_dialog();
                        ui.close();
                    }
                    if crate::print::available() && ui.button("Print…   Ctrl+P").clicked() {
                        self.open_print();
                        ui.close();
                    }
                    ui.separator();
                    if ui.button("Exit").clicked() {
                        ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });
                ui.menu_button("Edit", |ui| {
                    if ui.button("Select all   Ctrl+A").clicked() {
                        self.selection = self.visible_flat().into_iter().collect();
                        ui.close();
                    }
                    if ui.button("Select none   Esc").clicked() {
                        self.selection.clear();
                        ui.close();
                    }
                    if ui.button("Clear the tray").clicked() {
                        self.selection.clear();
                        self.held.clear();
                        ui.close();
                    }
                });
                ui.menu_button("View", |ui| {
                    ui.checkbox(&mut self.filter.picks, "Only picks");
                    ui.checkbox(&mut self.filter.hide_rejects, "Hide rejected pictures");
                    if ui.button("Larger thumbnails").clicked() {
                        self.store.thumb_size = (self.store.thumb_size + 24.0).min(256.0);
                        self.dirty = true;
                    }
                    if ui.button("Smaller thumbnails").clicked() {
                        self.store.thumb_size = (self.store.thumb_size - 24.0).max(64.0);
                        self.dirty = true;
                    }
                    ui.separator();
                    if ui.button("Slideshow").clicked() {
                        let list = self.visible_flat();
                        let start = self.selection.iter().next().copied().unwrap_or(0);
                        self.start_slideshow(list, start);
                        ui.close();
                    }
                    if ui.button("Places (map)").clicked() {
                        self.open_places();
                        ui.close();
                    }
                });
                ui.menu_button("Folder", |ui| {
                    let current = self.selection.iter().next().map(|&id| self.photos[id].folder);
                    ui.add_enabled_ui(current.is_some(), |ui| {
                        if ui.button("Edit description…").clicked() {
                            if let Some(fi) = current {
                                let path = self.folders[fi].path.clone();
                                let text = self.store.folder_notes.get(&path).cloned().unwrap_or_default();
                                self.dialog = Some(Dialog::FolderNote { path, text });
                            }
                            ui.close();
                        }
                        if ui.button("Play folder as slideshow").clicked() {
                            if let Some(fi) = current {
                                let list = self.folders[fi].photos.clone();
                                let first = list[0];
                                self.start_slideshow(list, first);
                            }
                            ui.close();
                        }
                    });
                    if current.is_none() {
                        ui.weak("Select a picture in a folder first");
                    }
                });
                ui.menu_button("Picture", |ui| {
                    ui.add_enabled_ui(!tray.is_empty(), |ui| {
                        if ui.button("Open in editor").clicked() {
                            self.open_editor(tray[0]);
                            ui.close();
                        }
                        ui.separator();
                        self.flag_rating_menu(ui, &tray);
                        ui.separator();
                        self.edit_clipboard_menu(ui, &tray);
                        ui.separator();
                        if ui.button("Rotate left   Ctrl+[").clicked() {
                            self.rotate(&tray, -1);
                            ui.close();
                        }
                        if ui.button("Rotate right   Ctrl+]").clicked() {
                            self.rotate(&tray, 1);
                            ui.close();
                        }
                        if ui.button("Tags and people…").clicked() {
                            self.dialog = Some(Dialog::Tag { tags: String::new(), people: String::new() });
                            ui.close();
                        }
                        if ui.button("Set location…").clicked() {
                            self.start_placing(tray.clone());
                            ui.close();
                        }
                        if ui.button("Undo all edits").clicked() {
                            for &id in &tray {
                                self.meta_mut(id).edits.clear();
                                self.refresh_thumb(id);
                            }
                            ui.close();
                        }
                    });
                });
                ui.menu_button("Create", |ui| {
                    if ui.button("New album…").clicked() {
                        self.dialog = Some(Dialog::NewAlbum { name: String::new() });
                        ui.close();
                    }
                    if ui.button("Picture collage…").clicked() {
                        self.dialog = Some(Dialog::Collage);
                        ui.close();
                    }
                    if ui.button("Movie…").clicked() {
                        self.dialog = Some(Dialog::Movie { hd: false });
                        ui.close();
                    }
                });
                ui.menu_button("Tools", |ui| {
                    if ui.button("Folder manager…").clicked() {
                        self.dialog = Some(Dialog::FolderManager);
                        ui.close();
                    }
                    if ui.button("Scan folders again").clicked() {
                        self.rescan();
                        ui.close();
                    }
                    ui.separator();
                    let mut online = self.store.online_maps == Some(true);
                    if ui
                        .checkbox(&mut online, "Online maps (OpenStreetMap)")
                        .on_hover_text("Detailed maps and address search from OpenStreetMap. Off: only the built-in world map and city list, no internet connections.")
                        .changed()
                    {
                        self.store.online_maps = Some(online);
                        self.dirty = true;
                    }
                });
                ui.menu_button("Help", |ui| {
                    if ui.button("About…").clicked() {
                        self.dialog = Some(Dialog::About);
                        ui.close();
                    }
                });
    }

    pub fn status_bar(&mut self, ui: &mut egui::Ui) {
        egui::Panel::bottom("status")
            .frame(egui::Frame::new().fill(t().status_bg).inner_margin(egui::Margin::symmetric(8, 3)))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    // what is in the tray, as a status line
                    let info = self.selection_info();
                    ui.label(RichText::new(info).size(12.0));
                    ui.separator();
                    let total_bytes: u64 = self.photos.iter().map(|p| p.bytes).sum();
                    ui.label(
                        RichText::new(format!(
                            "Library: {} folders, {} pictures, {} on disk",
                            self.folders.len(),
                            self.photos.len(),
                            fmt_bytes(total_bytes)
                        ))
                        .size(12.0)
                        .weak(),
                    );
                    if self.scanning {
                        ui.spinner();
                        ui.label(RichText::new("Looking for pictures…").size(12.0));
                    }
                    if self.using_samples {
                        ui.separator();
                        ui.label(
                            RichText::new("Showing sample pictures. Use Import to add your own folders.").size(12.0).color(t().accent),
                        );
                    }
                });
            });
    }

    pub fn tray_panel(&mut self, ui: &mut egui::Ui) {
        egui::Panel::bottom("tray")
            .exact_size(112.0)
            .frame(
                egui::Frame::new()
                    .fill(t().tray_bg)
                    .inner_margin(egui::Margin::symmetric(8, 8))
                    .stroke(Stroke::new(1.0, t().border)),
            )
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    let tray = self.tray();
                    // the tray well
                    let (well, _) = ui.allocate_exact_size(egui::vec2(236.0, 94.0), Sense::hover());
                    let p = ui.painter().clone();
                    p.rect_filled(well, 6, t().tray_well);
                    let whole_set = if self.held.is_empty() && self.selection.is_empty() { self.tray_set.clone() } else { None };
                    if let Some(set) = whole_set.clone().filter(|_| !tray.is_empty()) {
                        // a pile of the set's first pictures with a label, like a
                        // "Folder selected - 29 photos"
                        let c = well.center() + egui::vec2(0.0, -4.0);
                        for (k, &id) in tray.iter().take(3).enumerate().rev() {
                            let off = egui::vec2(k as f32 * 5.0 - 5.0, k as f32 * -4.0 + 4.0);
                            let r = Rect::from_center_size(c + off, egui::vec2(52.0, 52.0));
                            p.rect_filled(r.expand(2.0), 2, t().surface);
                            paint_thumb(&p, r, self.thumbs.get(&id), false);
                            if !self.thumbs.contains_key(&id) {
                                self.request_thumb(id);
                            }
                        }
                        let what = match set {
                            TraySet::Folder(_) => "Folder selected",
                            TraySet::Album(_) => "Album selected",
                            TraySet::Person(_) => "Person selected",
                            TraySet::Picks => "Picks selected",
                        };
                        let text = format!("{what} - {} photo{}", tray.len(), if tray.len() == 1 { "" } else { "s" });
                        let galley = p.layout_no_wrap(text, egui::FontId::proportional(12.5), Color32::WHITE);
                        let pill = Rect::from_center_size(c + egui::vec2(0.0, 10.0), galley.size() + egui::vec2(14.0, 6.0));
                        p.rect_filled(pill, 3, Color32::from_rgba_unmultiplied(70, 120, 170, 225));
                        p.galley(pill.min + egui::vec2(7.0, 3.0), galley, Color32::WHITE);
                    }
                    let label = if whole_set.is_some() {
                        String::new()
                    } else if tray.is_empty() {
                        "Photo tray: nothing selected".to_string()
                    } else if self.held.is_empty() {
                        format!("Selection: {} picture(s)", tray.len())
                    } else {
                        format!("Held: {}  Selected: {}", self.held.len(), tray.len() - self.held.len())
                    };
                    p.text(well.left_top() + egui::vec2(8.0, 6.0), egui::Align2::LEFT_TOP, label, egui::FontId::proportional(12.0), Color32::from_rgb(230, 232, 236));
                    let cell = 34.0;
                    let mut x = well.left() + 8.0;
                    let mut y = well.top() + 24.0;
                    let singles: &[usize] = if whole_set.is_some() { &[] } else { &tray };
                    for (i, &id) in singles.iter().enumerate() {
                        if i >= 12 {
                            p.text(
                                Pos2::new(well.right() - 8.0, well.bottom() - 6.0),
                                egui::Align2::RIGHT_BOTTOM,
                                format!("+{}", tray.len() - 12),
                                egui::FontId::proportional(12.0),
                                Color32::WHITE,
                            );
                            break;
                        }
                        let r = Rect::from_min_size(Pos2::new(x, y), egui::vec2(cell, cell));
                        let ir = paint_thumb(&p, r, self.thumbs.get(&id), false);
                        if self.held.contains(&id) {
                            p.rect_stroke(ir, 0, Stroke::new(1.5, Color32::from_rgb(255, 210, 90)), StrokeKind::Outside);
                        }
                        x += cell + 4.0;
                        if x + cell > well.right() - 4.0 {
                            x = well.left() + 8.0;
                            y += cell + 4.0;
                        }
                    }

                    ui.vertical(|ui| {
                        ui.set_width(112.0);
                        if ui.add_sized([108.0, 24.0], egui::Button::new(format!("{} Hold", icons::PIN))).on_hover_text("Keep these pictures in the tray while you browse").clicked() {
                            for id in tray.clone() {
                                if !self.held.contains(&id) {
                                    self.held.push(id);
                                }
                            }
                            self.tray_set = None;
                        }
                        if ui.add_sized([108.0, 24.0], egui::Button::new(format!("{} Clear", icons::CLOSE))).on_hover_text("Empty the tray").clicked() {
                            self.held.clear();
                            self.selection.clear();
                            self.tray_set = None;
                        }
                        let mut add_to: Option<usize> = None;
                        let mut new_album = false;
                        ui.menu_button(format!("{} To album…", icons::ALBUM), |ui| {
                            for (i, a) in self.store.albums.iter().enumerate() {
                                if ui.button(&a.name).clicked() {
                                    add_to = Some(i);
                                    ui.close();
                                }
                            }
                            if ui.button(format!("{} New album…", icons::ADD)).clicked() {
                                new_album = true;
                                ui.close();
                            }
                        });
                        if let Some(i) = add_to {
                            self.add_to_album(i, &tray);
                        }
                        if new_album {
                            self.dialog = Some(Dialog::NewAlbum { name: String::new() });
                        }
                    });

                    // culling: pick, reject and stars for the whole tray
                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            let picked = !tray.is_empty() && tray.iter().all(|&id| self.is_picked(id));
                            if pick_button(ui, picked, egui::vec2(36.0, 30.0)).on_hover_text("Pick / unpick (P)").clicked() {
                                self.toggle_flag(&tray, Flag::Pick);
                            }
                            let rejected = !tray.is_empty() && tray.iter().all(|&id| self.flag(id) == Flag::Reject);
                            if reject_button(ui, rejected, egui::vec2(36.0, 30.0)).on_hover_text("Reject / unreject (X)").clicked() {
                                self.toggle_flag(&tray, Flag::Reject);
                            }
                        });
                        // the stars show the rating when all the tray's pictures share it
                        let first = tray.first().map(|&id| self.rating(id)).unwrap_or(0);
                        let shared = if tray.iter().all(|&id| self.rating(id) == first) { first } else { 0 };
                        ui.add_enabled_ui(!tray.is_empty(), |ui| {
                            if let Some(n) = rating_input(ui, ui.make_persistent_id("tray_stars"), shared, 6.5).filter(|_| !tray.is_empty()) {
                                self.set_rating(&tray, n);
                            }
                        })
                        .response
                        .on_hover_text("Rating (0–5)");
                    });
                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            if ui.add_sized([36.0, 30.0], egui::Button::new(RichText::new(icons::ROTATE_LEFT).size(18.0))).on_hover_text("Rotate left").clicked() {
                                self.rotate(&tray, -1);
                            }
                            if ui.add_sized([36.0, 30.0], egui::Button::new(RichText::new(icons::ROTATE_RIGHT).size(18.0))).on_hover_text("Rotate right").clicked() {
                                self.rotate(&tray, 1);
                            }
                        });
                        ui.horizontal(|ui| {
                            if ui.add_sized([36.0, 30.0], egui::Button::new(RichText::new(icons::TAG).size(16.0))).on_hover_text("Tags and people").clicked() {
                                self.dialog = Some(Dialog::Tag { tags: String::new(), people: String::new() });
                            }
                        });
                    });

                    ui.separator();
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.spacing_mut().item_spacing.x = 2.0;
                        if big_button(ui, icons::MOVIE, "Movie", "Make a movie from the tray").clicked() {
                            self.dialog = Some(Dialog::Movie { hd: false });
                        }
                        if big_button(ui, icons::COLLAGE, "Collage", "Make a collage from the tray").clicked() {
                            self.dialog = Some(Dialog::Collage);
                        }
                        if big_button(ui, icons::EXPORT, "Export", "Save edited copies to a folder").clicked() {
                            self.dialog = Some(Dialog::Export { max_dim: 0, quality: 90 });
                        }
                        if crate::print::available() && big_button(ui, icons::PRINT, "Print", "Print the pictures in the tray (Ctrl+P)").clicked() {
                            self.open_print();
                        }
                        if big_button(ui, icons::MAIL, "Email", "Send pictures by email").clicked() {
                            self.dialog = Some(Dialog::Export { max_dim: 800, quality: 85 });
                            self.toast("Export small copies, then attach them to an email.");
                        }
                        if big_button(ui, icons::WEB, "Web page", "Make a web gallery of these pictures").clicked() {
                            self.open_gallery_dialog();
                        }
                        if big_button(ui, icons::LOCATION, "Location", "Set where these pictures were taken, on a map").clicked() {
                            self.start_placing(tray.clone());
                        }
                    });
                });
            });
    }

    pub fn cell_menu(&mut self, resp: &egui::Response, id: usize) {
        resp.context_menu(|ui| {
            if !self.selection.contains(&id) {
                self.selection.clear();
                self.selection.insert(id);
            }
            let tray = self.tray();
            if ui.button("Open in editor").clicked() {
                self.open_editor(id);
                ui.close();
            }
            if ui.button("View slideshow from here").clicked() {
                let list = self.visible_flat();
                self.start_slideshow(list, id);
                ui.close();
            }
            ui.separator();
            self.flag_rating_menu(ui, &tray);
            ui.separator();
            self.edit_clipboard_menu(ui, &tray);
            ui.separator();
            if ui.button(format!("{} Rotate left", icons::ROTATE_LEFT)).clicked() {
                self.rotate(&tray, -1);
                ui.close();
            }
            if ui.button(format!("{} Rotate right", icons::ROTATE_RIGHT)).clicked() {
                self.rotate(&tray, 1);
                ui.close();
            }
            if ui.button(format!("{} Hold in tray", icons::PIN)).clicked() {
                for &t in &tray {
                    if !self.held.contains(&t) {
                        self.held.push(t);
                    }
                }
                ui.close();
            }
            ui.menu_button(format!("{} Add to album", icons::ALBUM), |ui| {
                let mut chosen = None;
                for (i, a) in self.store.albums.iter().enumerate() {
                    if ui.button(&a.name).clicked() {
                        chosen = Some(i);
                        ui.close();
                    }
                }
                if ui.button(format!("{} New album…", icons::ADD)).clicked() {
                    self.dialog = Some(Dialog::NewAlbum { name: String::new() });
                    ui.close();
                }
                if let Some(i) = chosen {
                    self.add_to_album(i, &tray);
                }
            });
            if let Source::Album(ai) = self.source
                && ui.button("Remove from album").clicked() {
                    let keys: Vec<String> = tray.iter().map(|&t| self.photos[t].key.clone()).collect();
                    if let Some(a) = self.store.albums.get_mut(ai) {
                        a.keys.retain(|k| !keys.contains(k));
                    }
                    self.dirty = true;
                    self.selection.clear();
                    ui.close();
                }
            if ui.button(format!("{} Tags and people…", icons::TAG)).clicked() {
                self.dialog = Some(Dialog::Tag { tags: String::new(), people: String::new() });
                ui.close();
            }
            if ui.button(format!("{} Set location…", icons::LOCATION)).clicked() {
                self.start_placing(tray.clone());
                ui.close();
            }
            if ui.button(format!("{} Export…", icons::EXPORT)).clicked() {
                self.dialog = Some(Dialog::Export { max_dim: 0, quality: 90 });
                ui.close();
            }
            if !self.photos[id].is_sample() && ui.button("Show in file manager").clicked() {
                let path = self.photos[id].key.clone();
                if let Err(e) = reveal_in_file_manager(std::path::Path::new(&path)) {
                    self.toast(format!("Could not open the file manager: {e}"));
                }
                ui.close();
            }
        });
    }
}

/// Open the system file manager on the picture's folder, with the picture selected where the
/// platform allows it.
fn reveal_in_file_manager(path: &std::path::Path) -> std::io::Result<()> {
    use std::process::Command;
    if cfg!(target_os = "windows") {
        // explorer takes "/select," and the path as one argument
        let mut arg = std::ffi::OsString::from("/select,");
        arg.push(path);
        Command::new("explorer").arg(arg).spawn().map(drop)
    } else if cfg!(target_os = "macos") {
        Command::new("open").arg("-R").arg(path).spawn().map(drop)
    } else {
        // freedesktop: ask the file manager to select the file, else just open the folder. On a
        // thread: D-Bus activation of a missing file manager can take a while.
        let uri = format!("file://{}", percent_encode_path(&path.to_string_lossy()));
        let dir = path.parent().unwrap_or(path).to_path_buf();
        std::thread::spawn(move || {
            let selected = Command::new("dbus-send")
                .args(["--session", "--print-reply", "--reply-timeout=3000", "--dest=org.freedesktop.FileManager1"])
                .arg("/org/freedesktop/FileManager1")
                .arg("org.freedesktop.FileManager1.ShowItems")
                .arg(format!("array:string:{uri}"))
                .arg("string:")
                .output()
                .is_ok_and(|o| o.status.success());
            if !selected {
                let _ = Command::new("xdg-open").arg(&dir).spawn();
            }
        });
        Ok(())
    }
}

/// Open a file with the system's default program (a web page in the browser).
pub fn open_with_system(path: &std::path::Path) -> std::io::Result<()> {
    use std::process::Command;
    if cfg!(target_os = "windows") {
        Command::new("explorer").arg(path).spawn().map(drop)
    } else if cfg!(target_os = "macos") {
        Command::new("open").arg(path).spawn().map(drop)
    } else {
        Command::new("xdg-open").arg(path).spawn().map(drop)
    }
}

/// Percent-encode a path for a file:// URI (keeps '/' and unreserved characters).
fn percent_encode_path(p: &str) -> String {
    let mut out = String::with_capacity(p.len());
    for b in p.bytes() {
        if b.is_ascii_alphanumeric() || b"/-_.~".contains(&b) {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

/// Ctrl+Shift+C / Ctrl+Shift+V: copy / paste edits. (Plain Ctrl+C / Ctrl+V reach egui as
/// clipboard events, not keys, so the shifted forms are used.)
pub fn edit_clipboard_keys(ctx: &egui::Context) -> (bool, bool) {
    ctx.input(|i| {
        let m = i.modifiers.command && i.modifiers.shift;
        (m && i.key_pressed(egui::Key::C), m && i.key_pressed(egui::Key::V))
    })
}

#[cfg(test)]
mod reveal_tests {
    #[test]
    fn file_uris_are_encoded() {
        assert_eq!(super::percent_encode_path("/home/a b/été,1.jpg"), "/home/a%20b/%C3%A9t%C3%A9%2C1.jpg");
    }
}
