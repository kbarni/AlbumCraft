//! The editor's info panel, on the right: one column of sections the user can fold, each
//! remembered (`Store.info_sections`): Properties (the main camera facts), Extended EXIF (the
//! rest, closed at first), Location (a small map, Modify… / Show on map) and Tags (add with
//! suggestions from the library's tags, remove).

use crate::app::App;
use crate::geo::{LatLon, Place};
use crate::icons;
use crate::library::{fmt_bytes, fmt_date, fmt_time};
use crate::theme::t;
use eframe::egui::{self, RichText, Sense, Stroke};
use std::sync::Arc;

/// Width of the name column of the property tables.
const NAME_W: f32 = 92.0;
/// Most tag suggestions shown under the tag field.
const MAX_SUGGESTIONS: usize = 8;

/// One "name  value" table row; nothing when the value is missing or empty.
fn row(ui: &mut egui::Ui, name: &str, value: Option<String>) {
    let Some(v) = value.filter(|v| !v.trim().is_empty()) else { return };
    ui.horizontal_top(|ui| {
        name_cell(ui, name);
        ui.add(egui::Label::new(RichText::new(v).size(12.5).color(t().text)).wrap());
    });
}

/// Library tags containing `typed` (ignoring case) that the picture doesn't have, the ones that
/// start with it first, at most MAX_SUGGESTIONS.
fn suggest<'a>(all: &'a [String], typed: &str, have: &[String]) -> Vec<&'a String> {
    let typed = typed.trim().to_lowercase();
    if typed.is_empty() {
        return vec![];
    }
    let mut out: Vec<&String> =
        all.iter().filter(|t| t.to_lowercase().contains(&typed) && !have.iter().any(|h| h.eq_ignore_ascii_case(t))).collect();
    // stable: alphabetical within each group
    out.sort_by_key(|t| !t.to_lowercase().starts_with(&typed));
    out.truncate(MAX_SUGGESTIONS);
    out
}

/// The name column: left-aligned, fixed width.
fn name_cell(ui: &mut egui::Ui, name: &str) {
    ui.allocate_ui_with_layout(egui::vec2(NAME_W, 16.0), egui::Layout::left_to_right(egui::Align::TOP), |ui| {
        ui.set_min_width(NAME_W);
        ui.add(egui::Label::new(RichText::new(name).size(12.5).color(t().text_weak)).truncate());
    });
}

fn coords(at: LatLon) -> String {
    let ns = if at.lat >= 0.0 { "N" } else { "S" };
    let ew = if at.lon >= 0.0 { "E" } else { "W" };
    format!("{:.5}° {ns}, {:.5}° {ew}", at.lat.abs(), at.lon.abs())
}

fn orientation_text(o: u32) -> Option<String> {
    let s = match o {
        2 => "Mirrored",
        3 => "Upside down",
        4 => "Mirrored, upside down",
        5 => "Mirrored, turned left",
        6 => "Turned right (camera held upright)",
        7 => "Mirrored, turned right",
        8 => "Turned left (camera held upright)",
        _ => return None,
    };
    Some(s.into())
}

impl App {
    /// The section header: a chevron and the title. Returns whether the section is open.
    fn info_section(&mut self, ui: &mut egui::Ui, key: &str, title: &str, default_open: bool) -> bool {
        let open = self.store.info_sections.get(key).copied().unwrap_or(default_open);
        ui.add_space(6.0);
        let (rect, resp) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 26.0), Sense::click());
        let p = ui.painter();
        if resp.hovered() {
            p.rect_filled(rect.expand2(egui::vec2(6.0, 0.0)), 5, t().surface_hover);
        }
        let chevron = if open { icons::EXPAND_MORE } else { icons::CHEVRON_RIGHT };
        p.text(rect.left_center() + egui::vec2(8.0, 0.0), egui::Align2::CENTER_CENTER, chevron, egui::FontId::proportional(16.0), t().text_weak);
        p.text(rect.left_center() + egui::vec2(22.0, 0.0), egui::Align2::LEFT_CENTER, title, egui::FontId::proportional(15.0), t().text_strong);
        if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
            self.store.info_sections.insert(key.to_string(), !open);
            self.dirty = true;
            return !open;
        }
        open
    }

    /// Every tag used in the library, sorted (cached until the notes change).
    fn all_tags(&mut self) -> Arc<Vec<String>> {
        if let Some((rev, tags)) = &self.tag_cache
            && *rev == self.list_rev
        {
            return tags.clone();
        }
        let mut tags: Vec<String> = self.store.meta.values().flat_map(|m| m.tags.iter().cloned()).collect();
        tags.sort_by_key(|t| t.to_lowercase());
        tags.dedup();
        let tags = Arc::new(tags);
        self.tag_cache = Some((self.list_rev, tags.clone()));
        tags
    }

    fn add_tag(&mut self, id: usize, tag: &str) {
        let tag = tag.trim();
        if tag.is_empty() {
            return;
        }
        let m = self.meta_mut(id);
        if !m.tags.iter().any(|t| t.eq_ignore_ascii_case(tag)) {
            m.tags.push(tag.to_string());
        }
    }

    pub fn info_panel(&mut self, ui: &mut egui::Ui) {
        let id = self.edit.id;
        let Some(p) = self.photos.get(id).cloned() else { return };
        let mut close = false;
        egui::Panel::right("info")
            .resizable(true)
            .default_size(300.0)
            .min_size(240.0)
            .max_size(480.0)
            .frame(egui::Frame::new().fill(t().surface).inner_margin(egui::Margin::symmetric(14, 8)).stroke(Stroke::new(1.0, t().border)))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.add(egui::Label::new(RichText::new(&p.name).size(14.0).strong().color(t().text_strong)).truncate());
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.small_button(icons::CLOSE).on_hover_text("Hide this panel (I)").clicked() {
                            close = true;
                        }
                    });
                });
                egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                    ui.spacing_mut().item_spacing.y = 5.0;
                    self.info_properties(ui, id);
                    self.info_exif(ui, id);
                    self.info_location(ui, id);
                    self.info_tags(ui, id);
                    ui.add_space(12.0);
                });
            });
        if close {
            self.store.info_open = false;
            self.dirty = true;
        }
    }

    fn info_properties(&mut self, ui: &mut egui::Ui, id: usize) {
        if !self.info_section(ui, "properties", "Properties", true) {
            return;
        }
        let Some(p) = self.photos.get(id).cloned() else { return };
        let dims = self.real_dims_of(id);
        let cam = self.camera_info(id);
        let c = cam.as_deref();
        let num = crate::exifinfo::trim_num;
        let taken = c.and_then(|c| c.date_taken.as_deref()).and_then(crate::import::parse_exif_date);
        let format = p.name.rsplit_once('.').map(|(_, e)| match e.to_ascii_lowercase().as_str() {
            "jpg" | "jpeg" | "jpe" => "JPEG".to_string(),
            "tif" | "tiff" => "TIFF".to_string(),
            other => other.to_ascii_uppercase(),
        });
        row(ui, "Camera", c.and_then(|c| c.camera()));
        row(ui, "Lens", c.and_then(|c| c.lens.clone()));
        row(
            ui,
            "Taken on",
            Some(match taken {
                Some(ts) => format!("{} {}", fmt_date(ts), fmt_time(ts)),
                None => format!("{} {} (file date)", fmt_date(p.modified), fmt_time(p.modified)),
            }),
        );
        row(
            ui,
            "Size",
            dims.map(|(w, h)| {
                let mp = w as f64 * h as f64 / 1e6;
                if mp >= 1.0 { format!("{w} × {h} ({} MP)", num(mp, 1)) } else { format!("{w} × {h}") }
            }),
        );
        row(ui, "File", Some([format, Some(fmt_bytes(p.bytes))].into_iter().flatten().collect::<Vec<_>>().join(" · ")));
        if let Some(c) = c {
            let focal = c.focal_mm.map(|f| match c.focal_35mm {
                Some(e) if e as f64 != f.round() => format!("{} mm (35 mm: {e} mm)", num(f, 1)),
                _ => format!("{} mm", num(f, 1)),
            });
            row(ui, "Focal length", focal);
            row(ui, "Aperture", c.fnumber.map(|f| format!("f/{}", num(f, 1))));
            row(ui, "Exposure", c.exposure_short());
            row(ui, "ISO", c.iso.map(|i| i.to_string()));
        }
        let folder = if p.is_sample() { Some("Built-in sample picture".to_string()) } else { self.folders.get(p.folder).map(|f| f.path.clone()) };
        row(ui, "Folder", folder);
        let edits = self.meta(id).map(|m| m.edits.iter().map(|e| e.label()).collect::<Vec<_>>().join(", "));
        row(ui, "Edits", edits);
        if c.is_none() && !p.is_sample() {
            ui.label(RichText::new("No camera information (EXIF) in this picture.").size(12.0).color(t().text_weak));
        }
    }

    fn info_exif(&mut self, ui: &mut egui::Ui, id: usize) {
        let Some(cam) = self.camera_info(id) else { return };
        if !self.info_section(ui, "exif", "Extended EXIF", false) {
            return;
        }
        let c = cam.as_ref();
        let num = crate::exifinfo::trim_num;
        row(ui, "Flash", c.flash.clone());
        row(ui, "Exposure bias", c.exposure_bias.filter(|b| *b != 0.0).map(|b| format!("{} EV", num(b, 2))));
        row(ui, "Program", c.program.clone());
        row(ui, "Metering", c.metering.clone());
        row(ui, "White balance", c.white_balance.clone());
        row(ui, "Distance", c.subject_distance.map(|d| format!("{} m", num(d, 2))));
        row(ui, "Colour space", c.color_space.clone());
        row(ui, "Orientation", c.orientation.and_then(orientation_text));
        // the file's own position, as the background index found it (never read here, per frame)
        let gps = self.photos.get(id).and_then(|p| self.geo.file_gps.get(&p.key).copied().flatten());
        row(ui, "GPS position", gps.map(coords));
        row(ui, "Digitized", c.date_digitized.clone());
        row(ui, "Modified", c.date_modified.clone());
        row(ui, "Software", c.software.clone());
        if let Some(th) = self.exif_thumbs.get(&id).cloned() {
            let [w, h] = th.size();
            ui.horizontal_top(|ui| {
                name_cell(ui, "Camera preview");
                let s = (120.0 / w.max(h) as f32).min(1.0);
                ui.add(egui::Image::new((th.id(), egui::vec2(w as f32 * s, h as f32 * s)))).on_hover_text(format!("{w} × {h} pixels, stored in the file"));
            });
        }
        if !c.all.is_empty() {
            egui::CollapsingHeader::new(RichText::new(format!("All EXIF data ({} entries)", c.all.len())).size(12.5))
                .id_salt(("allexif", id))
                .default_open(false)
                .show(ui, |ui| {
                    egui::Grid::new(("exif_all", id)).num_columns(2).striped(true).spacing([8.0, 3.0]).show(ui, |ui| {
                        for (k, v) in &c.all {
                            ui.label(RichText::new(k).size(11.0).color(t().text_weak));
                            ui.add(egui::Label::new(RichText::new(v).size(11.0)).wrap());
                            ui.end_row();
                        }
                    });
                });
        }
    }

    fn info_location(&mut self, ui: &mut egui::Ui, id: usize) {
        if !self.info_section(ui, "location", "Location", true) {
            return;
        }
        let at = self.location(id);
        let by_user = self.meta(id).is_some_and(|m| matches!(m.place, Some(Place::Set { .. })));
        let mut modify = false;
        let mut show = false;
        match at {
            Some(at) => {
                if self.mini_map(ui, at, 150.0).clicked() {
                    show = true;
                }
                if let Some(name) = self.location_label(id) {
                    ui.label(RichText::new(name).size(13.0).color(t().text));
                }
                ui.label(RichText::new(if by_user { "Set in AlbumCraft" } else { "From the camera's GPS" }).size(11.5).color(t().text_weak));
                ui.horizontal(|ui| {
                    if ui.button(format!("{}  Modify…", icons::EDIT)).on_hover_text("Move the pin, rename the place or remove it").clicked() {
                        modify = true;
                    }
                    if ui.button(format!("{}  Show on map", icons::MAP)).on_hover_text("Open Places on this picture").clicked() {
                        show = true;
                    }
                });
            }
            None => {
                ui.label(RichText::new("No location yet.").size(12.5).color(t().text_weak));
                if ui.button(format!("{}  Set location…", icons::LOCATION)).on_hover_text("Find the place or click it on a map").clicked() {
                    modify = true;
                }
            }
        }
        if modify {
            self.start_placing(vec![id]);
        } else if show {
            self.show_on_map(id);
        }
    }

    fn info_tags(&mut self, ui: &mut egui::Ui, id: usize) {
        if !self.info_section(ui, "tags", "Tags", true) {
            return;
        }
        let tags = self.meta(id).map(|m| m.tags.clone()).unwrap_or_default();
        let mut remove = None;
        if tags.is_empty() {
            ui.label(RichText::new("No tags yet.").size(12.5).color(t().text_weak));
        } else {
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(6.0, 6.0);
                for tag in &tags {
                    let chip = egui::Button::new(RichText::new(format!("{tag}  {}", icons::CLOSE)).size(12.5)).corner_radius(12).fill(t().select_bg);
                    if ui.add(chip).on_hover_text("Remove this tag").clicked() {
                        remove = Some(tag.clone());
                    }
                }
            });
        }
        if let Some(tag) = remove {
            self.meta_mut(id).tags.retain(|t| t != &tag);
        }
        ui.add_space(2.0);
        let mut add: Option<String> = None;
        ui.horizontal(|ui| {
            let field = egui::TextEdit::singleline(&mut self.edit.tag_buf).hint_text("Add a tag…").desired_width(ui.available_width() - 60.0);
            let r = ui.add(field);
            let enter = r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
            if (ui.add_enabled(!self.edit.tag_buf.trim().is_empty(), egui::Button::new("Add")).clicked() || enter) && !self.edit.tag_buf.trim().is_empty() {
                add = Some(self.edit.tag_buf.trim().to_string());
                if enter {
                    r.request_focus();
                }
            }
        });
        // tags of the library that start with (or contain) what is typed, not on this picture yet
        if !self.edit.tag_buf.trim().is_empty() {
            let all = self.all_tags();
            let matches = suggest(&all, &self.edit.tag_buf, &tags);
            if !matches.is_empty() {
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(6.0, 6.0);
                    for t_ in matches {
                        if ui.add(egui::Button::new(RichText::new(format!("{}  {t_}", icons::ADD)).size(12.0)).corner_radius(12)).clicked() {
                            add = Some(t_.clone());
                        }
                    }
                });
            }
        }
        if let Some(tag) = add {
            self.add_tag(id, &tag);
            self.edit.tag_buf.clear();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tag_suggestions() {
        let all: Vec<String> = ["Beach", "Bordeaux", "Old town", "sea", "Sunset"].iter().map(|s| s.to_string()).collect();
        let have = vec!["sea".to_string()];
        let s: Vec<&str> = suggest(&all, "s", &have).iter().map(|t| t.as_str()).collect();
        // starts with "s" first ("sea" is already on the picture), then the rest containing it
        assert_eq!(s, vec!["Sunset"]);
        let s: Vec<&str> = suggest(&all, "O", &have).iter().map(|t| t.as_str()).collect();
        assert_eq!(s, vec!["Old town", "Bordeaux"]);
        assert!(suggest(&all, "  ", &have).is_empty());
        assert!(suggest(&all, "zz", &have).is_empty());
    }
}
