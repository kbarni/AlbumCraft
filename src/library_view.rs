//! Parts of the Library view shared with other views: the app menu, the per-picture context menu,
//! keys, the selection bar and the status bar, and small painted widgets (stars, flags, buttons).

use crate::icons;
use crate::app::*;
use crate::library::{fmt_bytes, Flag, MAX_RATING};
use eframe::egui::{self, Color32, Pos2, Rect, RichText, Sense, Stroke, Vec2};

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

/// Height of the selection bar under the grid.
pub const SELECTION_BAR_H: f32 = 48.0;
/// The size of the selection bar's square buttons.
const BAR_SQUARE: Vec2 = egui::vec2(34.0, 30.0);

/// A square icon button of the selection bar.
fn bar_icon(ui: &mut egui::Ui, icon: &str, tip: &str) -> egui::Response {
    ui.add_sized(BAR_SQUARE, egui::Button::new(RichText::new(icon).size(17.0))).on_hover_text(tip)
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
        if ui.add(egui::Button::new(if picked { "Unpick" } else { "Pick" }).shortcut_text("P")).clicked() {
            self.toggle_flag(ids, Flag::Pick);
            ui.close();
        }
        if ui.add(egui::Button::new(if rejected { "Unreject" } else { "Reject" }).shortcut_text("X")).clicked() {
            self.toggle_flag(ids, Flag::Reject);
            ui.close();
        }
        ui.menu_button("Rating", |ui| {
            for n in (0..=MAX_RATING).rev() {
                if ui.add(egui::Button::new(rating_label(n)).shortcut_text(n.to_string())).clicked() {
                    self.set_rating(ids, n);
                    ui.close();
                }
            }
        });
    }

    /// A menu button listing the albums, to add `ids` to one of them, and "New album…" (which
    /// takes the tray, see `Dialog::NewAlbum`).
    pub fn album_menu(&mut self, ui: &mut egui::Ui, label: String, ids: &[usize]) {
        let mut add_to: Option<usize> = None;
        let mut new_album = false;
        ui.menu_button(label, |ui| {
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
            self.add_to_album(i, ids);
        }
        if new_album {
            self.dialog = Some(Dialog::NewAlbum { name: String::new() });
        }
    }

    /// "Copy edits" (from a single picture) and "Paste edits" (onto `ids`).
    pub fn edit_clipboard_menu(&mut self, ui: &mut egui::Ui, ids: &[usize]) {
        let one = if let [one] = ids { Some(*one) } else { None };
        let has = one.is_some_and(|id| !crate::imaging::copyable_edits(&self.edits(id)).is_empty());
        if ui.add_enabled(has, egui::Button::new("Copy edits").shortcut_text("Ctrl+Shift+C")).clicked() {
            if let Some(id) = one {
                self.copy_edits(id);
            }
            ui.close();
        }
        let n = self.copied_edits.as_ref().map_or(0, |e| e.len());
        let label = if n > 0 { format!("Paste {n} edit{}", if n == 1 { "" } else { "s" }) } else { "Paste edits".to_string() };
        if ui.add_enabled(n > 0 && !ids.is_empty(), egui::Button::new(label).shortcut_text("Ctrl+Shift+V")).clicked() {
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
        // Ctrl+scroll or a pinch changes the picture size (the slider is hidden in narrow windows)
        let zoom = ctx.input(|i| i.zoom_delta());
        if zoom.is_finite() && (zoom - 1.0).abs() > 0.001 {
            self.store.thumb_size = (self.store.thumb_size * zoom).clamp(64.0, 256.0);
            self.dirty = true;
        }
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
        if !self.selection.is_empty() {
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

    /// The app menu: one flat list holding only what has no visible button elsewhere, or teaches a shortcut.
    pub fn app_menu_items(&mut self, ui: &mut egui::Ui) {
        let tray = self.targets();
        if ui.button("Import pictures…").clicked() {
            self.open_import();
            ui.close();
        }
        if ui.button("Add folder to library…").clicked() {
            self.add_library_folder();
            ui.close();
        }
        if ui.button("Library folders…").clicked() {
            self.dialog = Some(Dialog::FolderManager);
            ui.close();
        }
        if ui.button("Scan folders again").clicked() {
            self.rescan();
            ui.close();
        }
        ui.separator();
        if ui.add(egui::Button::new("Select all").shortcut_text("Ctrl+A")).clicked() {
            self.selection = self.visible_flat().into_iter().collect();
            ui.close();
        }
        if ui.add(egui::Button::new("Select none").shortcut_text("Esc")).clicked() {
            self.selection.clear();
            ui.close();
        }
        ui.separator();
        ui.add_enabled_ui(!tray.is_empty(), |ui| {
            if ui.add(egui::Button::new("Rotate left").shortcut_text("Ctrl+[")).clicked() {
                self.rotate(&tray, -1);
                ui.close();
            }
            if ui.add(egui::Button::new("Rotate right").shortcut_text("Ctrl+]")).clicked() {
                self.rotate(&tray, 1);
                ui.close();
            }
            self.flag_rating_menu(ui, &tray);
        });
        if ui.button("Move rejected pictures to the trash…").on_hover_text("Remove the files of the rejected pictures from their folders, after a confirmation").clicked() {
            self.ask_trash_rejects();
            ui.close();
        }
        if ui.add_enabled(!tray.is_empty(), egui::Button::new("Contact sheet…")).on_hover_text("Pages of small pictures with a title and their file names, to print or save as PDF").clicked() {
            self.open_collage(true);
            ui.close();
        }
        ui.separator();
        self.edit_clipboard_menu(ui, &tray);
        if ui.add_enabled(!tray.is_empty(), egui::Button::new("Undo all edits")).clicked() {
            let mut before = vec![];
            for &id in &tray {
                let edits = std::mem::take(&mut self.meta_mut(id).edits);
                if !edits.is_empty() {
                    before.push((self.photos[id].key.clone(), edits));
                }
                self.refresh_thumb(id);
            }
            if !before.is_empty() {
                let n = before.len();
                self.undoable(format!("Removed the edits of {n} picture{}.", if n == 1 { "" } else { "s" }), Undo::Edits(before));
            }
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
        if ui.button("About AlbumCraft…").clicked() {
            self.dialog = Some(Dialog::About);
            ui.close();
        }
        if ui.button("Exit").clicked() {
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
        }
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
                    if let Some(i) = &self.importing {
                        ui.separator();
                        ui.spinner();
                        ui.add(egui::ProgressBar::new(i.done as f32 / i.total.max(1) as f32).desired_width(120.0));
                        ui.label(RichText::new(format!("Importing {} of {}…", i.done, i.total)).size(12.0));
                        if ui.small_button("Stop").on_hover_text("Stop after the current picture (the ones copied stay)").clicked() {
                            self.stop_import();
                        }
                    }
                    let mut cancel = None;
                    for t in &self.tasks {
                        ui.separator();
                        ui.spinner();
                        if t.total > 0 {
                            ui.add(egui::ProgressBar::new(t.done as f32 / t.total as f32).desired_width(120.0));
                        }
                        let stopping = t.stop.load(std::sync::atomic::Ordering::Relaxed);
                        ui.label(RichText::new(if stopping { format!("{}: stopping…", t.label) } else { format!("{}…", t.label) }).size(12.0));
                        if !stopping && ui.small_button("Cancel").on_hover_text("Stop after the current picture").clicked() {
                            cancel = Some(t.id);
                        }
                    }
                    if let Some(id) = cancel
                        && let Some(t) = self.tasks.iter().find(|t| t.id == id)
                    {
                        t.stop.store(true, std::sync::atomic::Ordering::Relaxed);
                    }
                    if self.using_samples {
                        ui.separator();
                        ui.label(
                            RichText::new("Showing sample pictures. Use Import or Folders to bring in your own.").size(12.0).color(t().accent),
                        );
                    }
                });
            });
    }

    /// The selection bar: what is selected and what can be done with it, shown only while pictures
    /// are selected (one by one, or a folder / album / person as a whole). In a narrow window the
    /// buttons lose their labels, then the bar scrolls sideways: nothing overlaps.
    pub fn selection_bar(&mut self, ui: &mut egui::Ui) {
        let tray = self.tray();
        if tray.is_empty() {
            return;
        }
        egui::Panel::bottom("selection_bar")
            .exact_size(SELECTION_BAR_H)
            .frame(egui::Frame::new().fill(t().tray_bg).inner_margin(egui::Margin::symmetric(10, 6)).stroke(Stroke::new(1.0, t().border)))
            .show(ui, |ui| {
                let width = ui.available_width();
                let labels = width > 1180.0;
                egui::ScrollArea::horizontal().scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::VisibleWhenNeeded).show(ui, |ui| {
                    ui.horizontal_centered(|ui| {
                        ui.spacing_mut().item_spacing.x = 6.0;
                        let start = ui.cursor().left();
                        // what is selected
                        let n = tray.len();
                        let pictures = if n == 1 { "1 picture".to_string() } else { format!("{} pictures", crate::app::fmt_count(n)) };
                        let what = match self.tray_set.as_ref().filter(|_| self.selection.is_empty()) {
                            Some(TraySet::Folder(fi)) => format!("{} · {pictures}", self.folders.get(*fi).map(|f| f.name.as_str()).unwrap_or("Folder")),
                            Some(TraySet::Album(i)) => format!("{} · {pictures}", self.store.albums.get(*i).map(|a| a.name.as_str()).unwrap_or("Album")),
                            Some(TraySet::Person(name)) => format!("{name} · {pictures}"),
                            Some(TraySet::Picks) => format!("Picks · {pictures}"),
                            None => format!("{pictures} selected"),
                        };
                        ui.label(RichText::new(what).size(13.5).strong().color(t().text_strong));
                        if ui.add(egui::Button::new(RichText::new(icons::CLOSE).size(16.0)).frame(false)).on_hover_text("Clear the selection (Esc)").clicked() {
                            self.selection.clear();
                            self.tray_set = None;
                            self.list_pick = None;
                        }
                        ui.separator();
                        // culling
                        let picked = tray.iter().all(|&id| self.is_picked(id));
                        if pick_button(ui, picked, BAR_SQUARE).on_hover_text("Pick / unpick (P)").clicked() {
                            self.toggle_flag(&tray, Flag::Pick);
                        }
                        let rejected = tray.iter().all(|&id| self.flag(id) == Flag::Reject);
                        if reject_button(ui, rejected, BAR_SQUARE).on_hover_text("Reject / unreject (X)").clicked() {
                            self.toggle_flag(&tray, Flag::Reject);
                        }
                        // the stars show the rating when all the selected pictures share it
                        let first = tray.first().map(|&id| self.rating(id)).unwrap_or(0);
                        let shared = if tray.iter().all(|&id| self.rating(id) == first) { first } else { 0 };
                        if let Some(r) = rating_input(ui, ui.make_persistent_id("bar_stars"), shared, 6.5) {
                            self.set_rating(&tray, r);
                        }
                        ui.separator();
                        // changes
                        if bar_icon(ui, icons::ROTATE_LEFT, "Rotate left (Ctrl+[)").clicked() {
                            self.rotate(&tray, -1);
                        }
                        if bar_icon(ui, icons::ROTATE_RIGHT, "Rotate right (Ctrl+])").clicked() {
                            self.rotate(&tray, 1);
                        }
                        if bar_icon(ui, icons::TAG, "Tags and people").clicked() {
                            self.dialog = Some(Dialog::Tag { tags: String::new(), people: String::new() });
                        }
                        if bar_icon(ui, icons::LOCATION, "Set location: where these pictures were taken, on a map").clicked() {
                            self.start_placing(tray.clone());
                        }
                        self.album_menu(ui, if labels { format!("{}  To album", icons::ALBUM) } else { icons::ALBUM.to_string() }, &tray);
                        ui.separator();
                        // making something from them: on the right when there is room
                        let outputs: Vec<(&str, &str, &str)> = [
                            Some((icons::WEB, "Web page", "Make a web gallery of these pictures")),
                            Some((icons::MAIL, "Email", "Small copies, attached to a new message in your mail program")),
                            crate::print::available().then_some((icons::PRINT, "Print", "Print these pictures (Ctrl+P)")),
                            Some((icons::EXPORT, "Export", "Save edited copies to a folder")),
                            Some((icons::COLLAGE, "Collage", "Make a collage or a contact sheet")),
                            Some((icons::MOVIE, "Movie", "Make a movie")),
                        ]
                        .into_iter()
                        .flatten()
                        .collect();
                        let each = if labels { 104.0 } else { BAR_SQUARE.x + 6.0 };
                        let room = width - (ui.cursor().left() - start) - each * outputs.len() as f32;
                        if room > 0.0 {
                            ui.add_space(room);
                        }
                        for (icon, label, tip) in outputs {
                            let text = if labels { format!("{icon}  {label}") } else { icon.to_string() };
                            let size = if labels { egui::vec2(each - 6.0, BAR_SQUARE.y) } else { BAR_SQUARE };
                            let tip = if labels { tip.to_string() } else { format!("{label}: {tip}") };
                            if ui.add_sized(size, egui::Button::new(RichText::new(text).size(15.0))).on_hover_text(tip).clicked() {
                                match label {
                                    "Web page" => self.open_gallery_dialog(),
                                    "Email" => self.email(&tray),
                                    "Print" => self.open_print(),
                                    "Export" => self.dialog = Some(Dialog::Export { max_dim: 0, quality: 90 }),
                                    "Collage" => self.open_collage(false),
                                    _ => self.dialog = Some(Dialog::Movie { hd: false }),
                                }
                            }
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
            self.album_menu(ui, format!("{} Add to album", icons::ALBUM), &tray);
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
pub(crate) fn reveal_in_file_manager(path: &std::path::Path) -> std::io::Result<()> {
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
