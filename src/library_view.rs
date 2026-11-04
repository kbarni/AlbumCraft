//! The Library view: menu, toolbar, folder list, lightbox of thumbnails and the photo tray.

use crate::app::*;
use crate::library::fmt_bytes;
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

/// A square button showing a filled or hollow star.
pub fn star_button(ui: &mut egui::Ui, on: bool, size: Vec2) -> egui::Response {
    let resp = ui.add_sized(size, egui::Button::new(""));
    let fill = if on { t().star } else { Color32::TRANSPARENT };
    let outline = if on { t().star_outline } else { t().text_weak };
    paint_star(ui.painter(), resp.rect.center(), size.y.min(size.x) * 0.32, fill, outline);
    resp
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
        if enter {
            if let Some(&id) = self.selection.iter().next() {
                self.open_editor(id);
            }
        }
        if del {
            if let Source::Album(i) = self.source {
                let keys: Vec<String> = self.selection.iter().map(|&id| self.photos[id].key.clone()).collect();
                if let Some(a) = self.store.albums.get_mut(i) {
                    a.keys.retain(|k| !keys.contains(k));
                    self.dirty = true;
                }
                self.selection.clear();
            }
        }
        if ctrl_r || ctrl_l {
            let t = self.tray();
            self.rotate(&t, if ctrl_r { 1 } else { -1 });
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
                    if ui.button("Print…   Ctrl+P").clicked() {
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
                    if ui.checkbox(&mut self.only_starred, "Only starred pictures").clicked() {
                        ui.close();
                    }
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
                        if ui.button("Star / unstar").clicked() {
                            self.toggle_star(&tray);
                            ui.close();
                        }
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
                        self.dialog = Some(Dialog::Movie);
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
                            TraySet::Starred => "Starred pictures selected",
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
                        if ui.add_sized([108.0, 24.0], egui::Button::new("📌 Hold")).on_hover_text("Keep these pictures in the tray while you browse").clicked() {
                            for id in tray.clone() {
                                if !self.held.contains(&id) {
                                    self.held.push(id);
                                }
                            }
                            self.tray_set = None;
                        }
                        if ui.add_sized([108.0, 24.0], egui::Button::new("✖ Clear")).on_hover_text("Empty the tray").clicked() {
                            self.held.clear();
                            self.selection.clear();
                            self.tray_set = None;
                        }
                        let mut add_to: Option<usize> = None;
                        let mut new_album = false;
                        ui.menu_button("📚 To album…", |ui| {
                            for (i, a) in self.store.albums.iter().enumerate() {
                                if ui.button(&a.name).clicked() {
                                    add_to = Some(i);
                                    ui.close();
                                }
                            }
                            if ui.button("➕ New album…").clicked() {
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

                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            let starred = !tray.is_empty() && tray.iter().all(|&id| self.is_starred(id));
                            if star_button(ui, starred, egui::vec2(36.0, 30.0)).on_hover_text("Star / unstar").clicked() {
                                self.toggle_star(&tray);
                            }
                            if ui.add_sized([36.0, 30.0], egui::Button::new(RichText::new("⟲").size(18.0))).on_hover_text("Rotate left").clicked() {
                                self.rotate(&tray, -1);
                            }
                        });
                        ui.horizontal(|ui| {
                            if ui.add_sized([36.0, 30.0], egui::Button::new(RichText::new("⟳").size(18.0))).on_hover_text("Rotate right").clicked() {
                                self.rotate(&tray, 1);
                            }
                            if ui.add_sized([36.0, 30.0], egui::Button::new(RichText::new("🏷").size(16.0))).on_hover_text("Tags and people").clicked() {
                                self.dialog = Some(Dialog::Tag { tags: String::new(), people: String::new() });
                            }
                        });
                    });

                    ui.separator();
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.spacing_mut().item_spacing.x = 2.0;
                        if big_button(ui, "🌍", "Geo-Tag", "Place pictures on a map").clicked() {
                            self.service_gone("Geo-Tag");
                        }
                        if big_button(ui, "🎞", "Movie", "Make a movie from the tray").clicked() {
                            self.dialog = Some(Dialog::Movie);
                        }
                        if big_button(ui, "🖼", "Collage", "Make a collage from the tray").clicked() {
                            self.dialog = Some(Dialog::Collage);
                        }
                        if big_button(ui, "📝", "BlogThis!", "Post pictures to a blog").clicked() {
                            self.service_gone("BlogThis!");
                        }
                        if big_button(ui, "🛒", "Shop", "Order prints online").clicked() {
                            self.service_gone("Shop");
                        }
                        if big_button(ui, "💾", "Export", "Save edited copies to a folder").clicked() {
                            self.dialog = Some(Dialog::Export { max_dim: 0, quality: 90 });
                        }
                        if big_button(ui, "🖨", "Print", "Print the pictures in the tray (Ctrl+P)").clicked() {
                            self.open_print();
                        }
                        if big_button(ui, "📧", "Email", "Send pictures by email").clicked() {
                            self.dialog = Some(Dialog::Export { max_dim: 800, quality: 85 });
                            self.toast("Export small copies, then attach them to an email.");
                        }
                        if big_button(ui, "📤", "Upload", "Upload to web albums").clicked() {
                            self.service_gone("Upload");
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
            if ui.button("Star / unstar").clicked() {
                self.toggle_star(&tray);
                ui.close();
            }
            if ui.button("⟲ Rotate left").clicked() {
                self.rotate(&tray, -1);
                ui.close();
            }
            if ui.button("⟳ Rotate right").clicked() {
                self.rotate(&tray, 1);
                ui.close();
            }
            if ui.button("📌 Hold in tray").clicked() {
                for &t in &tray {
                    if !self.held.contains(&t) {
                        self.held.push(t);
                    }
                }
                ui.close();
            }
            ui.menu_button("📚 Add to album", |ui| {
                let mut chosen = None;
                for (i, a) in self.store.albums.iter().enumerate() {
                    if ui.button(&a.name).clicked() {
                        chosen = Some(i);
                        ui.close();
                    }
                }
                if ui.button("➕ New album…").clicked() {
                    self.dialog = Some(Dialog::NewAlbum { name: String::new() });
                    ui.close();
                }
                if let Some(i) = chosen {
                    self.add_to_album(i, &tray);
                }
            });
            if let Source::Album(ai) = self.source {
                if ui.button("Remove from album").clicked() {
                    let keys: Vec<String> = tray.iter().map(|&t| self.photos[t].key.clone()).collect();
                    if let Some(a) = self.store.albums.get_mut(ai) {
                        a.keys.retain(|k| !keys.contains(k));
                    }
                    self.dirty = true;
                    self.selection.clear();
                    ui.close();
                }
            }
            if ui.button("🏷 Tags and people…").clicked() {
                self.dialog = Some(Dialog::Tag { tags: String::new(), people: String::new() });
                ui.close();
            }
            if ui.button("💾 Export…").clicked() {
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

#[cfg(test)]
mod reveal_tests {
    #[test]
    fn file_uris_are_encoded() {
        assert_eq!(super::percent_encode_path("/home/a b/été,1.jpg"), "/home/a%20b/%C3%A9t%C3%A9%2C1.jpg");
    }
}
