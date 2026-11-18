//! The editor's info panel, on the right: one column of sections the user can fold, each
//! remembered (`Store.info_sections`): Properties (the main camera facts), Extended EXIF (the
//! rest, closed at first), Location (a small map, Modify… / Show on map) and Tags (add with
//! suggestions from the library's tags, remove).

use crate::i18n::tr;
use crate::app::App;
use crate::geo::{LatLon, Place};
use crate::icons;
use crate::library::{fmt_bytes, fmt_date, fmt_time};
use crate::theme::t;
use eframe::egui::{self, RichText, Sense, Stroke};

/// Width of the name column of the property tables.
const NAME_W: f32 = 92.0;
/// Most tag suggestions shown under the tag field.
/// How many of the library's tags the Tags section shows as chips (the Tags dialog has them all).
const TAG_CHIPS: usize = 16;

/// One "name  value" table row; nothing when the value is missing or empty.
fn row(ui: &mut egui::Ui, name: &str, value: Option<String>) {
    let Some(v) = value.filter(|v| !v.trim().is_empty()) else { return };
    ui.horizontal_top(|ui| {
        name_cell(ui, name);
        ui.add(egui::Label::new(RichText::new(v).size(12.5).color(t().text)).wrap());
    });
}

/// The name column: left-aligned, fixed width.
fn name_cell(ui: &mut egui::Ui, name: &str) {
    ui.allocate_ui_with_layout(egui::vec2(NAME_W, 16.0), egui::Layout::left_to_right(egui::Align::TOP), |ui| {
        ui.set_min_width(NAME_W);
        ui.add(egui::Label::new(RichText::new(name).size(12.5).color(t().text_weak)).truncate());
    });
}

fn coords(at: LatLon) -> String {
    let ns = if at.lat >= 0.0 { tr!("coord-n") } else { tr!("coord-s") };
    let ew = if at.lon >= 0.0 { tr!("coord-e") } else { tr!("coord-w") };
    tr!("info-coords", lat = crate::locale::decimal(&format!("{:.5}", at.lat.abs())), ns = ns, lon = crate::locale::decimal(&format!("{:.5}", at.lon.abs())), ew = ew)
}

fn orientation_text(o: u32) -> Option<String> {
    let s = match o {
        2 => tr!("info-orient-2"),
        3 => tr!("info-orient-3"),
        4 => tr!("info-orient-4"),
        5 => tr!("info-orient-5"),
        6 => tr!("info-orient-6"),
        7 => tr!("info-orient-7"),
        8 => tr!("info-orient-8"),
        _ => return None,
    };
    Some(s)
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

    fn add_tag(&mut self, id: usize, tag: &str) {
        let tag = tag.trim();
        if tag.is_empty() {
            return;
        }
        let m = self.meta_mut(id);
        if !m.tags.iter().any(|t| t.eq_ignore_ascii_case(tag)) {
            m.tags.push(tag.to_string());
        }
        // added after all: it does show it
        m.not_tags.retain(|t| !t.eq_ignore_ascii_case(tag));
    }

    pub fn info_panel(&mut self, ui: &mut egui::Ui) {
        let id = self.edit.id;
        let Some(p) = self.photos.get(id).cloned() else { return };
        let mut close = false;
        let range = 240.0..=560.0;
        let mut w = crate::edit_view::panel_width(ui.ctx(), self.store.info_w, 300.0, range.clone());
        let panel = egui::Panel::right("info")
            .exact_size(w)
            .frame(egui::Frame::new().fill(t().surface).inner_margin(egui::Margin { left: 12, right: 14, top: 8, bottom: 8 }).stroke(Stroke::new(1.0, t().border)))
            .show(ui, |outer| {
                // The content lives in a child of exactly the panel's size, which does not report
                // its size to the panel: anything wider is cut on the right. Otherwise a right
                // panel wider than it should be keeps its right edge, and the picture is drawn
                // over its left part.
                let r = outer.available_rect_before_wrap();
                let mut fenced = outer.new_child(egui::UiBuilder::new().max_rect(r).layout(egui::Layout::top_down(egui::Align::Min)));
                fenced.shrink_clip_rect(r.expand2(egui::vec2(4.0, 0.0)));
                let ui = &mut fenced;
                ui.horizontal(|ui| {
                    ui.add(egui::Label::new(RichText::new(&p.name).size(14.0).strong().color(t().text_strong)).truncate());
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.small_button(icons::CLOSE).on_hover_text(tr!("info-hide-hint")).clicked() {
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
                outer.advance_cursor_after_rect(r);
            });
        if crate::edit_view::side_grip(ui, "info_grip", panel.response.rect, false, &mut w, range) {
            self.dirty = true;
        }
        self.store.info_w = w;
        if close {
            self.store.info_open = false;
            self.dirty = true;
        }
    }

    fn info_properties(&mut self, ui: &mut egui::Ui, id: usize) {
        if !self.info_section(ui, "properties", &tr!("info-properties"), true) {
            return;
        }
        let Some(p) = self.photos.get(id).cloned() else { return };
        let dims = self.real_dims_of(id);
        let cam = self.camera_info(id);
        let c = cam.as_deref();
        let num = crate::exifinfo::trim_num;
        let video = crate::video::is_video(&p.key);
        // a video's own date was read with its shape (`App.taken`; the file's date when it has none)
        let taken = if video { self.taken.get(&id).copied().filter(|&t| t != p.modified) } else { c.and_then(|c| c.date_taken.as_deref()).and_then(crate::import::parse_exif_date) };
        let format = p.name.rsplit_once('.').map(|(_, e)| match e.to_ascii_lowercase().as_str() {
            "jpg" | "jpeg" | "jpe" => "JPEG".to_string(),
            "tif" | "tiff" => "TIFF".to_string(),
            other => other.to_ascii_uppercase(),
        });
        row(ui, &tr!("info-camera"), c.and_then(|c| c.camera()));
        row(ui, &tr!("info-lens"), c.and_then(|c| c.lens.clone()));
        row(
            ui,
            &tr!("info-taken"),
            Some(match taken {
                Some(ts) => format!("{} {}", fmt_date(ts), fmt_time(ts)),
                None => tr!("info-file-date", date = format!("{} {}", fmt_date(p.modified), fmt_time(p.modified))),
            }),
        );
        row(
            ui,
            &tr!("info-size"),
            dims.map(|(w, h)| {
                let mp = w as f64 * h as f64 / 1e6;
                if mp >= 1.0 { tr!("info-megapixels", w = w, h = h, mp = num(mp, 1)) } else { format!("{w} × {h}") }
            }),
        );
        if video {
            row(ui, &tr!("info-length"), self.durations.get(&id).map(|&s| crate::video::fmt_duration(f64::from(s))));
        }
        row(ui, &tr!("info-file"), Some([format, Some(fmt_bytes(p.bytes))].into_iter().flatten().collect::<Vec<_>>().join(" · ")));
        if let Some(c) = c {
            let focal = c.focal_mm.map(|f| match c.focal_35mm {
                Some(e) if e as f64 != f.round() => tr!("info-focal-35", f = num(f, 1), e = e),
                _ => format!("{} mm", num(f, 1)),
            });
            row(ui, &tr!("info-focal"), focal);
            row(ui, &tr!("info-aperture"), c.fnumber.map(|f| format!("f/{}", num(f, 1))));
            row(ui, &tr!("info-exposure"), c.exposure_short());
            row(ui, "ISO", c.iso.map(|i| i.to_string()));
        }
        let folder = if p.is_sample() { Some(tr!("info-sample")) } else { self.folders.get(p.folder).map(|f| f.path.clone()) };
        row(ui, &tr!("info-folder"), folder);
        let edits = self.meta(id).map(|m| m.edits.iter().map(crate::labels::op).collect::<Vec<_>>().join(", "));
        row(ui, &tr!("info-edits"), edits);
        if c.is_none() && !p.is_sample() && !video {
            ui.label(RichText::new(tr!("info-no-exif")).size(12.0).color(t().text_weak));
        }
    }

    fn info_exif(&mut self, ui: &mut egui::Ui, id: usize) {
        let Some(cam) = self.camera_info(id) else { return };
        if !self.info_section(ui, "exif", &tr!("info-exif"), false) {
            return;
        }
        let c = cam.as_ref();
        let num = crate::exifinfo::trim_num;
        row(ui, &tr!("info-flash"), c.flash.clone());
        row(ui, &tr!("info-bias"), c.exposure_bias.filter(|b| *b != 0.0).map(|b| format!("{} EV", num(b, 2))));
        row(ui, &tr!("info-program"), c.program.clone());
        row(ui, &tr!("info-metering"), c.metering.clone());
        row(ui, &tr!("edit-wb-title"), c.white_balance.clone());
        row(ui, &tr!("info-distance"), c.subject_distance.map(|d| format!("{} m", num(d, 2))));
        row(ui, &tr!("info-colour-space"), c.color_space.clone());
        row(ui, &tr!("edit-crop-orientation"), c.orientation.and_then(orientation_text));
        // the file's own position, as the background index found it (never read here, per frame)
        let gps = self.photos.get(id).and_then(|p| self.geo.file_gps.get(&p.key).copied().flatten());
        row(ui, &tr!("info-gps"), gps.map(coords));
        row(ui, &tr!("info-digitized"), c.date_digitized.clone());
        row(ui, &tr!("info-modified"), c.date_modified.clone());
        row(ui, &tr!("info-software"), c.software.clone());
        if let Some(th) = self.exif_thumbs.get(&id).cloned() {
            let [w, h] = th.size();
            ui.horizontal_top(|ui| {
                name_cell(ui, &tr!("info-preview"));
                let s = (120.0 / w.max(h) as f32).min(1.0);
                ui.add(egui::Image::new((th.id(), egui::vec2(w as f32 * s, h as f32 * s)))).on_hover_text(tr!("info-preview-hint", w = w, h = h));
            });
        }
        if !c.all.is_empty() {
            // a header of its own, cut short when narrow (egui's collapsing header never wraps
            // and would widen the panel)
            let open = self.store.info_sections.get("allexif").copied().unwrap_or(false);
            let chevron = if open { icons::EXPAND_MORE } else { icons::CHEVRON_RIGHT };
            let n = c.all.len();
            let head = ui
                .add(egui::Label::new(RichText::new(format!("{chevron} {}", tr!("info-all-exif", n = n))).size(12.5)).truncate().sense(Sense::click()))
                .on_hover_cursor(egui::CursorIcon::PointingHand);
            if head.clicked() {
                self.store.info_sections.insert("allexif".into(), !open);
                self.dirty = true;
            }
            if open {
                // rows rather than a grid: a grid's columns grow with long values
                for (k, v) in &c.all {
                    ui.horizontal_top(|ui| {
                        ui.allocate_ui_with_layout(egui::vec2(NAME_W, 14.0), egui::Layout::left_to_right(egui::Align::TOP), |ui| {
                            ui.set_min_width(NAME_W);
                            ui.add(egui::Label::new(RichText::new(k).size(11.0).color(t().text_weak)).truncate());
                        });
                        ui.add(egui::Label::new(RichText::new(v).size(11.0)).wrap());
                    });
                }
            }
        }
    }

    fn info_location(&mut self, ui: &mut egui::Ui, id: usize) {
        if !self.info_section(ui, "location", &tr!("info-location"), true) {
            return;
        }
        let at = self.location_now(id);
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
                ui.label(RichText::new(if by_user { tr!("info-set-in-app") } else { tr!("info-from-gps") }).size(11.5).color(t().text_weak));
                ui.horizontal_wrapped(|ui| {
                    if ui.add(egui::Button::new(crate::color_icons::atoms(ui.ctx(), format!("{}  {}", icons::EDIT, tr!("info-modify")), 13.5))).on_hover_text(tr!("info-modify-hint")).clicked() {
                        modify = true;
                    }
                    if ui.add(egui::Button::new(crate::color_icons::atoms(ui.ctx(), format!("{}  {}", icons::MAP, tr!("album-show-map")), 13.5))).on_hover_text(tr!("info-show-map-hint")).clicked() {
                        show = true;
                    }
                });
            }
            None => {
                ui.label(RichText::new(tr!("info-no-location")).size(12.5).color(t().text_weak));
                if ui.add(egui::Button::new(crate::color_icons::atoms(ui.ctx(), format!("{}  {}", icons::LOCATION, tr!("ctx-location")), 13.5))).on_hover_text(tr!("info-set-location-hint")).clicked() {
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
        if !self.info_section(ui, "tags", &tr!("info-tags"), true) {
            return;
        }
        let tags = self.meta(id).map(|m| m.tags.clone()).unwrap_or_default();
        let mut remove = None;
        if tags.is_empty() {
            ui.label(RichText::new(tr!("info-no-tags")).size(12.5).color(t().text_weak));
        } else {
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(6.0, 6.0);
                for tag in &tags {
                    let chip = egui::Button::new(RichText::new(format!("{tag}  {}", icons::CLOSE)).size(12.5)).corner_radius(12).fill(t().select_bg);
                    if ui.add(chip).on_hover_text(tr!("info-remove-tag")).clicked() {
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
        // the button first, from the right, so the field takes what it leaves in any language
        let line = egui::vec2(ui.available_width(), ui.spacing().interact_size.y);
        ui.allocate_ui_with_layout(line, egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let button = ui.add_enabled(!self.edit.tag_buf.trim().is_empty(), egui::Button::new(tr!("tags-add")));
            let field = egui::TextEdit::singleline(&mut self.edit.tag_buf).hint_text(tr!("info-add-tag")).desired_width(ui.available_width());
            let r = ui.add(field);
            let enter = r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
            if (button.clicked() || enter) && !self.edit.tag_buf.trim().is_empty() {
                add = Some(self.edit.tag_buf.trim().to_string());
                if enter {
                    r.request_focus();
                }
            }
        });
        // the library's tags (most used first; those containing what is typed): a click puts one
        // on the picture or takes it off
        self.tag_picker(ui, &[id], &self.edit.tag_buf.clone(), TAG_CHIPS, true);
        ui.horizontal_wrapped(|ui| {
            if ui.add(egui::Button::new(RichText::new(tr!("info-all-tags")).size(12.0)).frame(false)).on_hover_text(tr!("info-all-tags-hint")).clicked() {
                self.dialog = Some(crate::app::Dialog::Tag { tags: String::new(), people: String::new() });
            }
            if ui.add(egui::Button::new(RichText::new(tr!("tags-manage")).size(12.0)).frame(false)).clicked() {
                self.open_tag_manager(None);
            }
        });
        // proposed tags (when a search pack has analysed the picture)
        let (proposed, not) = self.proposal_chips(ui, &[id]);
        if let Some(tag) = not {
            self.not_this_tag(&[id], &tag);
        }
        if let Some(tag) = add {
            self.add_tag(id, &tag);
            self.edit.tag_buf.clear();
        } else if let Some(tag) = proposed {
            self.add_tag(id, &tag);
        }
    }
}

