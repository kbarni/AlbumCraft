//! `View::Export`: the options on the left (size, quality, signature or logo, what the files
//! say about themselves), the pictures on the right with the signature drawn on them (a click
//! leaves one out), the folder and the Export button at the bottom. The copies are made in the
//! background (`Library::export`); the signature, creator and copyright are kept for next time.

use crate::app::*;
use crate::i18n::tr;
use crate::export::Signature;
use crate::icons;
use crate::library::Job;
use eframe::egui::{self, Color32, Pos2, Rect, RichText, Sense, Stroke, TextureHandle, Vec2};
use std::collections::HashSet;
use std::path::PathBuf;

/// The sizes offered (longest side in pixels; 0 = the picture's own size).
pub(crate) const SIZES: [u32; 4] = [0, 2560, 1600, 800];

/// The name of one of the `SIZES`.
pub(crate) fn size_label(dim: u32) -> String {
    match dim {
        0 => tr!("export-size-original"),
        2560 => tr!("export-size-very-large"),
        1600 => tr!("export-size-large"),
        _ => tr!("export-size-small"),
    }
}

pub struct ExportState {
    pub ids: Vec<usize>,
    /// Pictures left out (clicked in the grid).
    pub skip: HashSet<usize>,
    pub max_dim: u32,
    /// The "Other size" value, kept while another size is chosen.
    pub custom: u32,
    pub quality: u8,
    /// Where Esc and Cancel go back to.
    pub back: View,
    /// The folder the copies go into.
    pub dest: String,
    /// The logo file asked for and its texture once it came (by path).
    pub logo_asked: Option<String>,
    pub logo_tex: Option<(String, TextureHandle)>,
}

impl ExportState {
    fn chosen(&self) -> Vec<usize> {
        self.ids.iter().copied().filter(|id| !self.skip.contains(id)).collect()
    }
}

impl App {
    /// Show the Export view for these pictures.
    pub fn open_export(&mut self, ids: Vec<usize>) {
        if ids.is_empty() {
            self.toast("Select some pictures first.");
            return;
        }
        let dest = self
            .store
            .export_to
            .clone()
            .or_else(|| dirs::picture_dir().map(|d| d.to_string_lossy().to_string()))
            .unwrap_or_default();
        let max_dim = self.store.export_size;
        let custom = if SIZES.contains(&max_dim) { 1200 } else { max_dim };
        let back = if self.view == View::Edit { View::Edit } else { View::Library };
        self.export = Some(Box::new(ExportState { ids, skip: HashSet::new(), max_dim, custom, quality: self.store.export_quality, back, dest, logo_asked: None, logo_tex: None }));
        self.view = View::Export;
    }

    fn leave_export(&mut self) {
        let back = self.export.take().map(|e| e.back).unwrap_or(View::Library);
        self.view = back;
    }

    /// A picture file arrived; true when it was the logo this view asked for.
    pub fn on_export_file(&mut self, key: &str, img: Option<&image::RgbaImage>) -> bool {
        let Some(st) = self.export.as_mut() else { return false };
        if st.logo_asked.as_deref() != Some(key) {
            return false;
        }
        match img {
            Some(img) => {
                let tex = self.ctx.load_texture("export-logo", to_color_image(img), egui::TextureOptions::LINEAR);
                st.logo_tex = Some((key.to_string(), tex));
            }
            None => self.toast(format!("The logo {key} can't be read.")),
        }
        true
    }

    fn start_export(&mut self) {
        let Some(st) = self.export.as_ref() else { return };
        let ids = st.chosen();
        if ids.is_empty() || st.dest.is_empty() {
            return;
        }
        let (dir, max_dim, quality) = (PathBuf::from(&st.dest), st.max_dim, st.quality);
        self.store.export_to = Some(st.dest.clone());
        self.dirty = true;
        let jobs: Vec<crate::commands::ExportJob> = ids.iter().map(|&id| crate::commands::export_job(self, id)).collect();
        let task = self.start_task(format!("Exporting {} picture(s)", jobs.len()));
        let opts = self.store.export_options(max_dim, quality);
        self.lib.export(task, jobs, dir, opts, self.store.export_location);
        self.leave_export();
    }

    pub fn export_ui(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        if self.export.is_none() {
            self.view = View::Library;
            return;
        }
        if self.dialog.is_none() && !ctx.memory(|m| m.focused().is_some()) && ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.leave_export();
            return;
        }
        // the logo, as a texture for the preview
        let sig = self.store.export_signature.clone();
        if let Some(st) = self.export.as_mut()
            && sig.on
            && sig.use_logo
            && let Some(path) = sig.logo.clone()
            && st.logo_asked.as_deref() != Some(path.as_str())
        {
            st.logo_asked = Some(path.clone());
            st.logo_tex = None;
            self.lib.request(Job::File { key: path, dim: crate::library::THUMB_DIM }, true);
        }
        self.export_top_bar(ui);
        self.export_bottom_bar(ui);
        let width = ui.available_width();
        egui::Panel::left("export_options")
            .exact_size((width / 4.0).clamp(290.0, 420.0))
            .frame(egui::Frame::new().fill(t().left_bg).inner_margin(egui::Margin::same(14)).stroke(Stroke::new(1.0, t().border)))
            .show(ui, |ui| {
                egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| self.export_options(ui));
            });
        egui::CentralPanel::default().frame(egui::Frame::new().fill(t().lightbox_bg).inner_margin(12)).show(ui, |ui| self.export_grid(ui));
    }

    fn export_top_bar(&mut self, ui: &mut egui::Ui) {
        let mut leave = false;
        let Some(st) = self.export.as_ref() else { return };
        let (n, total, back) = (st.chosen().len(), st.ids.len(), st.back);
        egui::Panel::top("export_top")
            .exact_size(60.0)
            .frame(egui::Frame::new().fill(t().toolbar_bg).inner_margin(egui::Margin::symmetric(16, 8)).stroke(Stroke::new(1.0, t().border)))
            .show(ui, |ui| {
                ui.horizontal_centered(|ui| {
                    let label = format!("{}  {}", icons::ARROW_BACK, if back == View::Edit { "Editor" } else { "Library" });
                    if ui.add(egui::Button::new(RichText::new(label).size(13.5)).min_size(egui::vec2(0.0, 32.0))).on_hover_text("Go back (Esc)").clicked() {
                        leave = true;
                    }
                    ui.add_space(10.0);
                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing.y = 0.0;
                        ui.label(RichText::new("Export").size(20.0).strong().color(t().text_strong));
                        let sub = if n == total { format!("{total} picture{}", plural(total)) } else { format!("{n} of {total} pictures") };
                        ui.label(RichText::new(sub).size(12.0).color(t().text_weak));
                    });
                });
            });
        if leave {
            self.leave_export();
        }
    }

    fn export_bottom_bar(&mut self, ui: &mut egui::Ui) {
        let (mut go, mut cancel, mut choose) = (false, false, false);
        let Some(st) = self.export.as_ref() else { return };
        let n = st.chosen().len();
        let dest = st.dest.clone();
        egui::Panel::bottom("export_bottom")
            .frame(egui::Frame::new().fill(t().toolbar_bg).inner_margin(egui::Margin::symmetric(16, 10)).stroke(Stroke::new(1.0, t().border)))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Save into").color(t().text_weak));
                    let shown = if dest.is_empty() { "Choose a folder…".to_string() } else { dest.clone() };
                    if ui.button(format!("{}  {shown}", icons::FOLDER)).on_hover_text("Choose the folder for the copies").clicked() {
                        choose = true;
                    }
                });
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Nothing is overwritten; the originals are not changed.").size(12.5).color(t().text_weak));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let ok = n > 0 && !dest.is_empty();
                        let label = format!("Export {n} picture{}", plural(n));
                        let b = egui::Button::new(RichText::new(label).strong().color(if ok { Color32::WHITE } else { t().text_weak }))
                            .fill(if ok { t().accent } else { t().surface })
                            .min_size(egui::vec2(150.0, 32.0));
                        if ui.add_enabled(ok, b).clicked() {
                            go = true;
                        }
                        if ui.add(egui::Button::new("Cancel").min_size(egui::vec2(0.0, 32.0))).clicked() {
                            cancel = true;
                        }
                    });
                });
            });
        if choose {
            let mut dialog = rfd::FileDialog::new().set_title("Export pictures to…");
            if !dest.is_empty() {
                dialog = dialog.set_directory(&dest);
            }
            if let Some(dir) = dialog.pick_folder()
                && let Some(st) = self.export.as_mut()
            {
                st.dest = dir.to_string_lossy().to_string();
            }
        }
        if go {
            self.start_export();
        } else if cancel {
            self.leave_export();
        }
    }

    /// The left panel: size, signature and what the files say about themselves.
    fn export_options(&mut self, ui: &mut egui::Ui) {
        let mut changed = false;
        if let Some(st) = self.export.as_mut() {
            heading(ui, "Size");
            for dim in SIZES {
                ui.radio_value(&mut st.max_dim, dim, size_label(dim));
            }
            ui.horizontal(|ui| {
                let other = !SIZES.contains(&st.max_dim);
                if ui.radio(other, "Other:").clicked() {
                    st.max_dim = st.custom;
                }
                if ui.add(egui::DragValue::new(&mut st.custom).range(64..=20_000).suffix(" pixels")).changed() {
                    st.max_dim = st.custom;
                }
            });
            note(ui, "The longest side. Pictures are never made larger than they are.");
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label("JPEG quality");
                ui.add(egui::Slider::new(&mut st.quality, 50..=100));
            });
        }

        heading(ui, "Signature");
        let creator = self.store.export_creator.trim().to_string();
        let sig = &mut self.store.export_signature;
        let r = ui.checkbox(&mut sig.on, "Add a signature or a logo");
        if r.changed() {
            changed = true;
            if sig.on && sig.text.trim().is_empty() && !creator.is_empty() {
                sig.text = format!("© {creator}");
            }
        }
        ui.add_enabled_ui(sig.on, |ui| {
            ui.horizontal(|ui| {
                changed |= ui.radio_value(&mut sig.use_logo, false, "Text").changed();
                changed |= ui.radio_value(&mut sig.use_logo, true, "Picture (logo)").changed();
            });
            if sig.use_logo {
                ui.horizontal(|ui| {
                    let name = sig.logo.as_deref().and_then(|l| std::path::Path::new(l).file_name()).map(|n| n.to_string_lossy().to_string());
                    if ui.button(if name.is_some() { "Other picture…" } else { "Choose a picture…" }).clicked()
                        && let Some(f) = rfd::FileDialog::new().set_title("Logo or signature picture").add_filter("Pictures", &["png", "jpg", "jpeg", "webp", "gif", "bmp", "tif", "tiff"]).pick_file()
                    {
                        sig.logo = Some(f.to_string_lossy().to_string());
                        changed = true;
                    }
                    if let Some(n) = name {
                        ui.label(RichText::new(n).weak());
                    }
                });
                note(ui, "A PNG with a transparent background works best.");
            } else {
                changed |= ui.add(egui::TextEdit::singleline(&mut sig.text).hint_text("Your name, or © Your Name").desired_width(f32::INFINITY)).changed();
            }
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label("Position");
                changed |= anchor_picker(ui, &mut sig.anchor);
            });
            egui::Grid::new("signature_grid").num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
                ui.label("Size");
                let size = if sig.use_logo { &mut sig.logo_size } else { &mut sig.text_size };
                let range = if sig.use_logo { 0.05..=0.6 } else { 0.01..=0.15 };
                changed |= ui.add(egui::Slider::new(size, range).custom_formatter(|v, _| format!("{:.0} %", v * 100.0)).custom_parser(percent)).on_hover_text("Compared with the picture's shorter side").changed();
                ui.end_row();
                ui.label("Opacity");
                changed |= ui.add(egui::Slider::new(&mut sig.opacity, 0.1..=1.0).custom_formatter(|v, _| format!("{:.0} %", v * 100.0)).custom_parser(percent)).changed();
                ui.end_row();
                if !sig.use_logo {
                    ui.label("Colour");
                    ui.horizontal(|ui| {
                        changed |= ui.color_edit_button_srgb(&mut sig.color).changed();
                        changed |= ui.checkbox(&mut sig.halo, "Shadow").on_hover_text("A soft halo so the text reads on any picture").changed();
                    });
                    ui.end_row();
                }
            });
        });

        heading(ui, "Information in the files");
        changed |= ui
            .checkbox(&mut self.store.export_location, "Where the pictures were taken (GPS)")
            .on_hover_text("Turn off before sharing pictures if you don't want others to see the location.")
            .changed();
        changed |= ui.checkbox(&mut self.store.export_exif, "Camera information (EXIF)").on_hover_text("Camera, lens, exposure and the date taken, copied from the original file.").changed();
        ui.add_space(4.0);
        egui::Grid::new("credit_grid").num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
            ui.label("Creator");
            changed |= ui.add(egui::TextEdit::singleline(&mut self.store.export_creator).hint_text("Your name").desired_width(f32::INFINITY)).changed();
            ui.end_row();
            ui.label("Copyright");
            changed |= ui.add(egui::TextEdit::singleline(&mut self.store.export_copyright).hint_text("© Your Name").desired_width(f32::INFINITY)).changed();
            ui.end_row();
        });
        note(ui, "Written into each copy. The signature, creator and copyright are kept for next time (also in Settings, Export).");
        if changed {
            self.dirty = true;
        }
    }

    /// The pictures, with the signature drawn where it will be; a click leaves one out or takes
    /// it back.
    fn export_grid(&mut self, ui: &mut egui::Ui) {
        let Some(st) = self.export.as_ref() else { return };
        let ids = st.ids.clone();
        let skip = st.skip.clone();
        let logo = st.logo_tex.clone();
        let sig = self.store.export_signature.clone();
        let avail = ui.available_width();
        let cols = ((avail / 240.0).ceil() as usize).max(1);
        let cell = (avail - 8.0) / cols as f32;
        let rows = ids.len().div_ceil(cols);
        let mut toggle = None;
        egui::ScrollArea::vertical().auto_shrink([false, false]).show_rows(ui, cell, rows, |ui, range| {
            for row in range {
                let (line, _) = ui.allocate_exact_size(egui::vec2(avail - 8.0, cell), Sense::hover());
                for c in 0..cols {
                    let Some(&id) = ids.get(row * cols + c) else { break };
                    let r = Rect::from_min_size(Pos2::new(line.left() + c as f32 * cell, line.top()), Vec2::splat(cell)).shrink(8.0);
                    let resp = ui.interact(r, ui.scope_id().with(("export_cell", id)), Sense::click());
                    let p = ui.painter_at(r.expand(4.0));
                    self.want_thumb(id);
                    let pic = crate::library_view::paint_thumb(&p, r, self.thumbs.get(&id), true);
                    let out = skip.contains(&id);
                    if self.thumbs.contains_key(&id) {
                        paint_signature(&p, pic, &sig, logo.as_ref().map(|l| &l.1));
                    }
                    if out {
                        p.rect_filled(pic, 0, Color32::from_black_alpha(150));
                    }
                    if resp.hovered() {
                        p.rect_stroke(pic.expand(2.0), 3, Stroke::new(2.0, t().select_border), egui::StrokeKind::Outside);
                    }
                    // the check circle, top left
                    let dot = pic.left_top() + egui::vec2(14.0, 14.0);
                    if out {
                        p.circle(dot, 10.0, Color32::from_black_alpha(90), Stroke::new(1.5, Color32::WHITE));
                    } else {
                        p.circle_filled(dot, 10.0, t().accent);
                        p.text(dot, egui::Align2::CENTER_CENTER, icons::CHECK, egui::FontId::proportional(13.0), Color32::WHITE);
                    }
                    let resp = resp.on_hover_text(if out { "Left out: click to export it too" } else { "Click to leave this picture out" });
                    if resp.clicked() {
                        toggle = Some(id);
                    }
                }
            }
        });
        if let (Some(id), Some(st)) = (toggle, self.export.as_mut())
            && !st.skip.remove(&id)
        {
            st.skip.insert(id);
        }
    }
}

fn plural(n: usize) -> &'static str {
    if n == 1 { "" } else { "s" }
}

fn heading(ui: &mut egui::Ui, text: &str) {
    ui.add_space(12.0);
    ui.label(RichText::new(text).size(15.0).strong().color(t().text_strong));
    ui.add_space(4.0);
}

fn note(ui: &mut egui::Ui, text: &str) {
    ui.label(RichText::new(text).size(12.0).color(t().text_weak));
}

fn percent(s: &str) -> Option<f64> {
    s.trim().trim_end_matches('%').trim().parse::<f64>().ok().map(|v| v / 100.0)
}

/// Nine squares for the nine places a signature can go; true when one was clicked.
fn anchor_picker(ui: &mut egui::Ui, anchor: &mut u8) -> bool {
    const CELL: Vec2 = egui::vec2(20.0, 14.0);
    const GAP: f32 = 2.0;
    let (rect, _) = ui.allocate_exact_size(egui::vec2(CELL.x * 3.0 + GAP * 2.0, CELL.y * 3.0 + GAP * 2.0), Sense::hover());
    let mut changed = false;
    for k in 0..9u8 {
        let (col, row) = ((k % 3) as f32, (k / 3) as f32);
        let r = Rect::from_min_size(rect.min + egui::vec2(col * (CELL.x + GAP), row * (CELL.y + GAP)), CELL);
        let resp = ui.interact(r, ui.scope_id().with(("export_anchor", k)), Sense::click());
        let on = *anchor == k;
        let fill = if on { t().accent } else if resp.hovered() { t().surface_hover } else { t().surface };
        ui.painter().rect_filled(r, 2, fill);
        ui.painter().rect_stroke(r, 2, Stroke::new(1.0, t().border_strong), egui::StrokeKind::Inside);
        if resp.clicked() && !on {
            *anchor = k;
            changed = true;
        }
    }
    changed
}

/// The signature drawn on a picture shown at `pic`, the way `export::sign` will draw it.
fn paint_signature(p: &egui::Painter, pic: Rect, sig: &Signature, logo: Option<&TextureHandle>) {
    if !sig.shows() {
        return;
    }
    let short = pic.width().min(pic.height());
    let alpha = (sig.opacity.clamp(0.0, 1.0) * 255.0) as u8;
    if sig.use_logo {
        let Some(tex) = logo else { return };
        let [w, h] = tex.size();
        let s = sig.logo_size * short / w.max(h).max(1) as f32;
        let size = egui::vec2(w as f32 * s, h as f32 * s);
        let (x, y) = crate::decor::anchored(sig.anchor, (pic.width(), pic.height()), (size.x, size.y), short * 0.03);
        let r = Rect::from_min_size(pic.min + egui::vec2(x, y), size);
        p.image(tex.id(), r, Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)), Color32::from_white_alpha(alpha));
        return;
    }
    let px = (sig.text_size * short).max(4.0);
    let [cr, cg, cb] = sig.color;
    let galley = p.layout_no_wrap(sig.text.trim().to_string(), egui::FontId::proportional(px), Color32::from_rgba_unmultiplied(cr, cg, cb, alpha));
    let (x, y) = crate::decor::anchored(sig.anchor, (pic.width(), pic.height()), (galley.size().x, galley.size().y), px * 0.6);
    let at = pic.min + egui::vec2(x, y);
    if sig.halo {
        let light = cr as u32 + cg as u32 + cb as u32 > 382;
        let shade = if light { Color32::from_black_alpha(alpha / 2) } else { Color32::from_white_alpha(alpha / 2) };
        let halo = p.layout_no_wrap(sig.text.trim().to_string(), egui::FontId::proportional(px), shade);
        let d = (px * 0.06).max(1.0);
        for off in [egui::vec2(d, d), egui::vec2(-d, d), egui::vec2(d, -d), egui::vec2(-d, -d)] {
            p.galley(at + off, halo.clone(), shade);
        }
    }
    p.galley(at, galley, Color32::WHITE);
}
