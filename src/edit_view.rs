//! The Edit view (one photo with fix-up tools) and the full-screen slideshow.

use crate::app::*;
use crate::develop::{self, Adjust};
use crate::imaging::{self, EditOp};
use crate::decor::{Decor, TextOverlay};
use crate::looks::Look;
use crate::library::{fmt_bytes, fmt_date, fmt_time};
use crate::library_view::paint_thumb;
use eframe::egui::{self, Color32, Pos2, Rect, RichText, Sense, Stroke, StrokeKind, TextureOptions, Vec2};

fn fit_rect(area: Rect, size: [usize; 2]) -> Rect {
    let s = (area.width() / size[0] as f32).min(area.height() / size[1] as f32);
    Rect::from_center_size(area.center(), egui::vec2(size[0] as f32 * s, size[1] as f32 * s))
}

/// Largest zoom: 400 % of the photo's real pixels (2^((p − 0.5)·4) at slider end p = 1).
/// The smallest is "Fit" (slider start p = 0).
pub const ZOOM_MAX: f32 = 4.0;
/// Fit / 1:1 animate over 0.3 s.
pub const ZOOM_ANIM_SECS: f64 = 0.3;

/// The zoom slider: position p in 0..1, `fit` = the zoom that fits.
/// p = 0 → fit; 0 < p < 0.5 → fit + (2^(2p) − 1)(1 − fit) (reaches 100 % at p = 0.5);
/// p ≥ 0.5 → 2^(4(p − 0.5)) (100 % … 400 %).
pub fn pos_to_zoom(p: f32, fit: f32) -> f32 {
    let p = p.clamp(0.0, 1.0);
    if p < 0.5 {
        fit + ((2f32).powf(2.0 * p) - 1.0) * (1.0 - fit)
    } else {
        (2f32).powf(4.0 * (p - 0.5))
    }
}

/// Inverse of `pos_to_zoom`, for placing the slider knob.
pub fn zoom_to_pos(z: f32, fit: f32) -> f32 {
    if z >= 1.0 {
        (0.5 + z.log2() / 4.0).clamp(0.5, 1.0)
    } else if (1.0 - fit).abs() > 1e-4 {
        let t = 1.0 + (z - fit) / (1.0 - fit);
        (t.max(1.0).log2() / 2.0).clamp(0.0, 0.5)
    } else {
        0.0
    }
}

/// Where a picture goes inside `area` at the current zoom: fitted when `zoom` is None,
/// otherwise `tex` pixels × `r` real pixels each × zoom, moved by `pan`.
fn place(area: Rect, tex: [usize; 2], r: Option<f32>, zoom: Option<f32>, pan: Vec2, ppp: f32) -> Rect {
    match (zoom, r) {
        (Some(z), Some(r)) => {
            let size = egui::vec2(tex[0] as f32, tex[1] as f32) * (r * z / ppp);
            Rect::from_center_size(area.center() + pan, size)
        }
        _ => fit_rect(area, tex),
    }
}

/// Keep the zoomed picture from being dragged out of sight.
fn clamp_pan(pan: Vec2, size: Vec2, view: Vec2) -> Vec2 {
    let max = ((size - view) * 0.5).max(Vec2::ZERO);
    egui::vec2(pan.x.clamp(-max.x, max.x), pan.y.clamp(-max.y, max.y))
}

/// A row of colour choices; true when one was clicked.
fn swatches(ui: &mut egui::Ui, label: &str, color: &mut [u8; 3]) -> bool {
    const COLOURS: [[u8; 3]; 7] = [[255, 255, 255], [250, 248, 242], [128, 128, 128], [20, 20, 20], [250, 210, 60], [220, 50, 50], [40, 90, 200]];
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.add_sized([70.0, 18.0], egui::Label::new(label));
        for c in COLOURS {
            let (rect, resp) = ui.allocate_exact_size(egui::vec2(18.0, 18.0), Sense::click());
            let fill = Color32::from_rgb(c[0], c[1], c[2]);
            ui.painter().rect_filled(rect, 3, fill);
            let on = *color == c;
            let stroke = if on { Stroke::new(2.0, t().accent) } else { Stroke::new(1.0, t().border) };
            ui.painter().rect_stroke(rect, 3, stroke, StrokeKind::Outside);
            if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                *color = c;
                changed = true;
            }
        }
    });
    changed
}

#[derive(Clone, Copy)]
enum SlideCmd {
    Prev,
    Next,
    PlayPause,
    Stop,
}

/// Ask for a music file; true when one was chosen (music is then switched on).
fn pick_music(s: &mut crate::slides::SlideSettings) -> bool {
    let Some(path) = rfd::FileDialog::new()
        .set_title("Music for the slideshow")
        .add_filter("Music", &["mp3", "ogg", "oga", "flac", "wav", "m4a", "aac", "mp4"])
        .pick_file()
    else {
        return false;
    };
    s.music = Some(path.to_string_lossy().to_string());
    s.music_on = true;
    true
}

#[derive(Clone, Copy)]
enum AutoFix {
    Lucky,
    Exposure,
    WhiteBalance,
    Straighten,
}

fn tool_button(ui: &mut egui::Ui, icon: &str, label: &str) -> egui::Response {
    ui.add_sized([112.0, 46.0], egui::Button::new(RichText::new(format!("{icon}\n{label}")).size(12.5)))
}

impl App {
    /// The Advanced settings being edited: the last edit step when it is an adjustment.
    fn current_adjust(&self, id: usize) -> Adjust {
        match self.meta(id).and_then(|m| m.edits.last()) {
            Some(EditOp::Adjust(a)) => *a,
            _ => Adjust::default(),
        }
    }

    /// Change the current adjustment step (or start one). Moving sliders again right after
    /// changes the same step, so Undo takes all of it back at once; all-neutral removes it.
    fn set_adjust(&mut self, id: usize, a: Adjust) {
        let a = a.clamped();
        let m = self.meta_mut(id);
        match m.edits.last_mut() {
            Some(EditOp::Adjust(cur)) => *cur = a,
            _ => m.edits.push(EditOp::Adjust(a)),
        }
        if a.is_neutral() {
            m.edits.pop();
        }
        self.edit.redo.clear();
        self.refresh_thumb(id);
    }

    /// The picture as edited so far, without the current adjustment step (what the automatic fixes
    /// look at), from the full preview when it is loaded or else the thumbnail.
    fn analysis_image(&self, id: usize) -> Option<image::RgbaImage> {
        let base = self.big.get(&id).map(|b| &b.base).or_else(|| self.thumb_base.get(&id))?;
        let mut ops = self.edits(id);
        if matches!(ops.last(), Some(EditOp::Adjust(_))) {
            ops.pop();
        }
        Some(imaging::apply_all(base, &ops))
    }

    fn auto_fix(&mut self, id: usize, what: AutoFix) {
        let Some(img) = self.analysis_image(id) else {
            self.toast("The picture is still loading.");
            return;
        };
        let mut a = self.current_adjust(id);
        match what {
            AutoFix::Lucky => {
                let l = develop::lucky(&img);
                a = Adjust { clarity: a.clarity, sharpen: a.sharpen, noise: a.noise, ..l };
            }
            AutoFix::Exposure => a.exposure = develop::auto_exposure(&img),
            AutoFix::WhiteBalance => (a.temp, a.tint) = develop::auto_white_balance(&img),
            AutoFix::Straighten => {
                match develop::auto_straighten(&img) {
                    Some(angle) if angle.abs() >= 0.2 => {
                        let angle = (angle * 10.0).round() / 10.0;
                        self.push_edit(id, EditOp::Straighten(angle));
                        self.toast(format!("Straightened by {angle:+.1}°."));
                    }
                    Some(_) => self.toast("This picture already looks level."),
                    None => self.toast("No clear horizon or vertical lines found to straighten by."),
                }
                return;
            }
        }
        self.set_adjust(id, a);
    }

    /// White balance from a clicked point that should be neutral gray (`pos` 0…1 on the picture).
    fn pick_neutral(&mut self, id: usize, pos: Pos2) {
        let Some(img) = self.analysis_image(id) else { return };
        let (w, h) = (img.width(), img.height());
        let (cx, cy) = (((pos.x * w as f32) as u32).min(w - 1), ((pos.y * h as f32) as u32).min(h - 1));
        // average a small square so noise doesn't decide
        let (mut sum, mut n) = ([0u32; 3], 0u32);
        for y in cy.saturating_sub(2)..=(cy + 2).min(h - 1) {
            for x in cx.saturating_sub(2)..=(cx + 2).min(w - 1) {
                let p = img.get_pixel(x, y);
                for c in 0..3 {
                    sum[c] += p[c] as u32;
                }
                n += 1;
            }
        }
        let avg = sum.map(|v| (v / n.max(1)) as u8);
        let mut a = self.current_adjust(id);
        (a.temp, a.tint) = develop::neutralize(avg);
        self.set_adjust(id, a);
    }

    fn step_photo(&mut self, delta: i32) {
        let list = &self.edit.list;
        if let Some(pos) = list.iter().position(|&x| x == self.edit.id) {
            let n = (pos as i32 + delta).clamp(0, list.len() as i32 - 1) as usize;
            let id = list[n];
            if id != self.edit.id {
                self.edit_go(PendingLeave::Photo(id));
            }
        }
    }

    /// "Histogram" box in the bottom-left panel: red, green and blue curves of the
    /// edited picture, drawn over each other with see-through fills.
    fn histogram_box(&mut self, ui: &mut egui::Ui, id: usize) {
        let cam = self.camera_info(id);
        egui::Frame::new()
            .fill(t().surface)
            .stroke(Stroke::new(1.0, t().border))
            .corner_radius(3)
            .inner_margin(egui::Margin::same(8))
            .show(ui, |ui| {
                ui.label(RichText::new("Histogram and camera info").size(12.5));
                // the main camera settings, always visible under the curves (
                // make / model and focal length on the left; shutter, aperture, ISO on the right)
                let draw_camera = |ui: &mut egui::Ui| {
                    let small = |txt: String| RichText::new(txt).size(11.0).color(t().text);
                    match cam.as_deref().filter(|c| c.has_camera_data()) {
                        Some(c) => {
                            egui::Grid::new("camera_box").num_columns(2).spacing([10.0, 1.0]).show(ui, |ui| {
                                ui.label(small(c.camera().unwrap_or_else(|| "Unknown camera".into())));
                                ui.label(small(c.exposure_short().unwrap_or_default()));
                                ui.end_row();
                                // formats: "(35mm equivalent: %3.0fmm)", "f/%3.1f",
                                // "Focal Length: %3.1fmm", "ISO: %2d"
                                ui.label(small(c.focal_35mm.map(|e| format!("(35mm equivalent: {e:3.0}mm)")).or(c.lens.clone()).unwrap_or_default()));
                                ui.label(small(c.fnumber.map(|f| format!("f/{f:3.1}")).unwrap_or_default()));
                                ui.end_row();
                                ui.label(small(c.focal_mm.map(|f| format!("Focal Length: {f:3.1}mm")).unwrap_or_default()));
                                ui.label(small(c.iso.map(|i| format!("ISO: {i:2}")).unwrap_or_default()));
                                ui.end_row();
                            });
                        }
                        None => {
                            ui.label(RichText::new("No camera information in this picture").size(11.0).weak());
                        }
                    }
                };
                let (rect, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 56.0), Sense::hover());
                let p = &ui.painter_at(rect.expand(1.0));
                p.line_segment([rect.left_bottom(), rect.right_bottom()], Stroke::new(1.0, t().text_weak));
                let Some(h) = self.big.get(&id).and_then(|e| e.hist.as_ref()) else {
                    p.text(rect.center(), egui::Align2::CENTER_CENTER, "…", egui::FontId::proportional(12.0), t().text_weak);
                    draw_camera(ui);
                    return;
                };
                // ignore the two extreme bins when scaling so clipped pixels don't flatten the curve
                let peak = h.iter().flat_map(|c| c[1..255].iter()).copied().max().unwrap_or(1).max(1) as f32;
                let colours = [
                    Color32::from_rgba_unmultiplied(230, 60, 60, 90),
                    Color32::from_rgba_unmultiplied(60, 180, 70, 90),
                    Color32::from_rgba_unmultiplied(70, 90, 230, 90),
                ];
                for (c, col) in colours.iter().enumerate() {
                    let pts: Vec<Pos2> = (0..256)
                        .map(|i| {
                            let x = rect.left() + rect.width() * i as f32 / 255.0;
                            let y = rect.bottom() - rect.height() * (h[c][i] as f32 / peak).min(1.0);
                            Pos2::new(x, y)
                        })
                        .collect();
                    for w in pts.windows(2) {
                        let quad = vec![w[0], w[1], Pos2::new(w[1].x, rect.bottom()), Pos2::new(w[0].x, rect.bottom())];
                        p.add(egui::Shape::convex_polygon(quad, *col, Stroke::NONE));
                    }
                    let line = Color32::from_rgb(col.r(), col.g(), col.b());
                    p.add(egui::Shape::line(pts, Stroke::new(1.0, line)));
                }
                // clipping: the share of pixels with a channel at pure black or pure white
                let total = h[1].iter().sum::<u32>().max(1) as f32;
                let dark = (0..3).map(|c| h[c][0]).max().unwrap_or(0) as f32 / total * 100.0;
                let bright = (0..3).map(|c| h[c][255]).max().unwrap_or(0) as f32 / total * 100.0;
                if dark >= 0.5 || bright >= 0.5 {
                    let mut parts = vec![];
                    if dark >= 0.5 {
                        parts.push(format!("shadows {dark:.1} %"));
                    }
                    if bright >= 0.5 {
                        parts.push(format!("highlights {bright:.1} %"));
                    }
                    ui.label(RichText::new(format!("Clipped: {}", parts.join(", "))).size(11.0).color(t().reject))
                        .on_hover_text("Share of the picture that is pure black or pure white in at least one colour");
                }
                ui.add_space(2.0);
                draw_camera(ui);
            });
    }

    pub fn edit_ui(&mut self, ui: &mut egui::Ui) {
        let id = self.edit.id;
        if id >= self.photos.len() {
            self.view = View::Library;
            return;
        }
        self.edit_keys(ui.ctx());
        let id = self.edit.id;
        self.tick_zoom_anim(ui.ctx());

        // ---- top bar
        egui::Panel::top("edit_top")
            .frame(egui::Frame::new().fill(t().toolbar_bg).inner_margin(egui::Margin::symmetric(8, 6)).stroke(Stroke::new(1.0, t().border)))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    if ui.add(egui::Button::new(RichText::new("◀  Back to Library").strong()).min_size(egui::vec2(0.0, 28.0))).clicked() {
                        self.edit_go(PendingLeave::Library);
                    }
                    ui.separator();
                    let p = &self.photos[id];
                    ui.label(RichText::new(&self.folders[p.folder].name).size(14.0).strong());
                    ui.label(RichText::new(format!("›  {}", p.name)).size(13.0).color(t().text_weak));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.selectable_label(self.edit.show_info, "ℹ Properties").clicked() {
                            self.edit.show_info = !self.edit.show_info;
                        }
                        ui.separator();
                        // View buttons in the editor toolbar: A, A|B, A|A.
                        // In 2up view there are two more buttons:
                        // swap_2up_layout (side by side <-> one above the other) and
                        // swap_2up_focus ("Change the picture the focus is on").
                        if self.edit.compare != CompareMode::Single {
                            let layout_icon = if self.edit.vertical { "⬌" } else { "⬍" };
                            if ui
                                .button(RichText::new(layout_icon).size(15.0))
                                .on_hover_text(if self.edit.vertical { "Show the pictures side by side" } else { "Show one picture above the other" })
                                .clicked()
                            {
                                self.edit.vertical = !self.edit.vertical;
                            }
                            if ui
                                .button(RichText::new("⇆").size(15.0))
                                .on_hover_text("Change the picture the focus is on (where your edits go)")
                                .clicked()
                            {
                                self.swap_2up_focus();
                            }
                        }
                        for (mode, label, tip) in [
                            (CompareMode::BeforeAfter, "A | A'", "Before and after: two versions of this picture; the second starts as the original"),
                            (CompareMode::TwoPhotos, "A | B", "Two pictures side by side"),
                            (CompareMode::Single, " A ", "One picture"),
                        ] {
                            let r = ui.add(egui::Button::new(RichText::new(label).monospace().size(12.5)).selected(self.edit.compare == mode));
                            if r.on_hover_text(tip).clicked() {
                                self.set_compare(mode);
                            }
                        }
                        ui.separator();
                        if ui.button("▶ Slideshow").clicked() {
                            let list = self.edit.list.clone();
                            self.start_slideshow(list, id);
                        }
                        let pos = self.edit.list.iter().position(|&x| x == id).unwrap_or(0);
                        if ui.add_enabled(pos + 1 < self.edit.list.len(), egui::Button::new("▶")).on_hover_text("Next picture").clicked() {
                            self.step_photo(1);
                        }
                        ui.label(format!("{} of {}", pos + 1, self.edit.list.len()));
                        if ui.add_enabled(pos > 0, egui::Button::new("◀")).on_hover_text("Previous picture").clicked() {
                            self.step_photo(-1);
                        }
                    });
                });
            });

        self.edit_tools_panel(ui);
        self.edit_bottom_panel(ui);
        if self.edit.show_info {
            self.properties_panel(ui);
        }
        self.edit_canvas(ui);
    }

    fn edit_keys(&mut self, ctx: &egui::Context) {
        if self.dialog.is_some() || self.text_focused() {
            return;
        }
        let (left, right, esc, undo, redo) = ctx.input(|i| {
            (
                i.key_pressed(egui::Key::ArrowLeft),
                i.key_pressed(egui::Key::ArrowRight),
                i.key_pressed(egui::Key::Escape),
                i.modifiers.command && i.key_pressed(egui::Key::Z),
                i.modifiers.command && i.key_pressed(egui::Key::Y),
            )
        });
        if esc {
            if self.edit.tool != Tool::None {
                self.edit.tool = Tool::None;
                self.edit.sel = None;
            } else {
                self.edit_go(PendingLeave::Library);
            }
        }
        if self.edit.tool == Tool::None {
            if left {
                self.step_photo(-1);
            }
            if right {
                self.step_photo(1);
            }
        }
        if undo {
            self.undo();
        }
        if redo {
            self.redo();
        }
        let id = self.edit.id;
        if id < self.photos.len() {
            self.flag_rating_keys(ctx, &[id]);
            let (copy, paste) = crate::library_view::edit_clipboard_keys(ctx);
            if copy {
                self.copy_edits(id);
            }
            if paste {
                self.paste_edits(&[id]);
            }
        }
    }

    fn edit_tools_panel(&mut self, ui: &mut egui::Ui) {
        let id = self.edit.id;
        egui::Panel::left("edit_tools")
            .exact_size(260.0)
            .frame(egui::Frame::new().fill(t().left_bg).inner_margin(egui::Margin::same(8)).stroke(Stroke::new(1.0, t().border)))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    for (t, label) in [(Tab::Edit, "Edit"), (Tab::Presets, "Presets"), (Tab::Effects, "Effects")] {
                        if ui.selectable_label(self.edit.tab == t, RichText::new(label).strong()).clicked() {
                            self.edit.tab = t;
                        }
                    }
                });
                ui.separator();

                // history at the bottom
                egui::Panel::bottom("history").frame(egui::Frame::new().inner_margin(egui::Margin::symmetric(0, 6))).show(ui, |ui| {
                    let edits = self.edits(id);
                    ui.horizontal(|ui| {
                        let last = edits.last().map(|e| e.label());
                        if ui
                            .add_enabled(last.is_some(), egui::Button::new(format!("↶ Undo {}", last.clone().unwrap_or_default())))
                            .clicked()
                        {
                            self.undo();
                        }
                        let next = self.edit.redo.last().map(|e| e.label());
                        if ui.add_enabled(next.is_some(), egui::Button::new("↷ Redo")).on_hover_text(next.unwrap_or_default()).clicked() {
                            self.redo();
                        }
                    });
                    ui.label(RichText::new("Edits on this picture:").size(12.0).weak());
                    if edits.is_empty() {
                        ui.label(RichText::new("(none — this is the original)").size(12.0).weak());
                    }
                    egui::ScrollArea::vertical().max_height(90.0).show(ui, |ui| {
                        for (i, e) in edits.iter().enumerate() {
                            let t = RichText::new(format!("{}. {}", i + 1, e.label())).size(12.0);
                            ui.label(if i + 1 == edits.len() { t.strong() } else { t });
                        }
                    });
                    ui.horizontal(|ui| {
                        if !edits.is_empty() && ui.small_button("Undo all edits").clicked() {
                            let all = std::mem::take(&mut self.meta_mut(id).edits);
                            self.edit.redo.extend(all.into_iter().rev());
                            self.refresh_thumb(id);
                        }
                        if !edits.is_empty() && ui.small_button("Copy edits").on_hover_text("Copy these edits to paste them on other pictures (Ctrl+Shift+C)").clicked() {
                            self.copy_edits(id);
                        }
                        if let Some(n) = self.copied_edits.as_ref().map(|e| e.len())
                            && ui.small_button(format!("Paste {n}")).on_hover_text("Add the copied edits to this picture (Ctrl+Shift+V)").clicked() {
                                self.paste_edits(&[id]);
                            }
                    });
                    ui.add_space(6.0);
                    self.histogram_box(ui, id);
                });

                match self.edit.tab {
                    Tab::Edit => self.edit_tab(ui),
                    Tab::Presets => self.presets_tab(ui),
                    Tab::Effects => self.effects_tab(ui),
                }
            });
    }

    fn edit_tab(&mut self, ui: &mut egui::Ui) {
        let id = self.edit.id;
        if self.edit.tool == Tool::None {
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.edit.advanced, false, "Basic");
                ui.selectable_value(&mut self.edit.advanced, true, "Advanced");
            });
            ui.add_space(4.0);
            if self.edit.advanced {
                egui::ScrollArea::vertical().show(ui, |ui| self.advanced_panel(ui));
                return;
            }
        }
        match self.edit.tool {
            Tool::PickNeutral => {
                ui.label(RichText::new("White balance").strong().size(15.0));
                ui.label("Click something in the picture that should be neutral gray or white.");
                if ui.button("Cancel").clicked() {
                    self.edit.tool = Tool::None;
                }
            }
            Tool::Crop => {
                ui.label(RichText::new("Crop").strong().size(15.0));
                ui.label("Drag a box on the picture to choose what to keep.");
                ui.add_space(4.0);
                for a in [CropAspect::Manual, CropAspect::Square, CropAspect::FourThree, CropAspect::ThreeTwo] {
                    if ui.radio_value(&mut self.edit.aspect, a, a.label()).changed() {
                        self.edit.sel = None;
                    }
                }
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    if ui.add_enabled(self.edit.sel.is_some(), egui::Button::new("✔ Apply")).clicked() {
                        if let Some(s) = self.edit.sel.take() {
                            self.push_edit(id, EditOp::Crop(s));
                        }
                        self.edit.tool = Tool::None;
                    }
                    if ui.button("Cancel").clicked() {
                        self.edit.tool = Tool::None;
                        self.edit.sel = None;
                    }
                });
            }
            Tool::Redeye => {
                ui.label(RichText::new("Redeye").strong().size(15.0));
                ui.label("Drag a box around the red eyes. Only red pupils inside the box are darkened.");
                ui.horizontal(|ui| {
                    if ui.add_enabled(self.edit.sel.is_some(), egui::Button::new("✔ Apply")).clicked() {
                        if let Some(s) = self.edit.sel.take() {
                            self.push_edit(id, EditOp::Redeye(s));
                        }
                        self.edit.tool = Tool::None;
                    }
                    if ui.button("Cancel").clicked() {
                        self.edit.tool = Tool::None;
                        self.edit.sel = None;
                    }
                });
            }
            Tool::Straighten => {
                ui.label(RichText::new("Straighten").strong().size(15.0));
                ui.label("Move the slider until the horizon lines up with the grid.");
                ui.add(egui::Slider::new(&mut self.edit.angle, -12.0..=12.0).suffix("°").step_by(0.1));
                ui.horizontal(|ui| {
                    if ui.button("✔ Apply").clicked() {
                        if self.edit.angle.abs() > 0.01 {
                            let a = self.edit.angle;
                            self.push_edit(id, EditOp::Straighten(a));
                        }
                        self.edit.tool = Tool::None;
                        self.edit.angle = 0.0;
                    }
                    if ui.button("Cancel").clicked() {
                        self.edit.tool = Tool::None;
                        self.edit.angle = 0.0;
                    }
                });
            }
            Tool::None => {
                ui.label(RichText::new("One-click fixes").size(12.0).weak());
                let lucky = ui
                    .add_sized([ui.available_width(), 34.0], egui::Button::new(RichText::new("✨  I'm feeling lucky").size(14.0)))
                    .on_hover_text("Exposure, white balance, black point and a little vibrance, all at once (then fine-tune them in Advanced)");
                if lucky.clicked() {
                    self.auto_fix(id, AutoFix::Lucky);
                }
                ui.add_space(4.0);
                egui::Grid::new("auto").num_columns(2).spacing([6.0, 6.0]).show(ui, |ui| {
                    if tool_button(ui, "☀", "Auto exposure").on_hover_text("Brighten or darken to a balanced exposure").clicked() {
                        self.auto_fix(id, AutoFix::Exposure);
                    }
                    if tool_button(ui, "🎨", "Auto colour").on_hover_text("Auto white balance: remove a colour cast").clicked() {
                        self.auto_fix(id, AutoFix::WhiteBalance);
                    }
                    ui.end_row();
                    if tool_button(ui, "◐", "Auto contrast").on_hover_text("Stretch the darkest and brightest tones").clicked() {
                        self.push_edit(id, EditOp::AutoContrast);
                    }
                    if tool_button(ui, "📐", "Auto straighten").on_hover_text("Level a tilted horizon or verticals").clicked() {
                        self.auto_fix(id, AutoFix::Straighten);
                    }
                    ui.end_row();
                });
                ui.add_space(10.0);
                ui.label(RichText::new("Tools").size(12.0).weak());
                egui::Grid::new("basic").num_columns(2).spacing([6.0, 6.0]).show(ui, |ui| {
                    if tool_button(ui, "✂", "Crop").clicked() {
                        self.edit.tool = Tool::Crop;
                        self.edit.sel = None;
                    }
                    if tool_button(ui, "📐", "Straighten").clicked() {
                        self.edit.tool = Tool::Straighten;
                        self.edit.angle = 0.0;
                    }
                    ui.end_row();
                    if tool_button(ui, "👁", "Redeye").clicked() {
                        self.edit.tool = Tool::Redeye;
                        self.edit.sel = None;
                    }
                    if tool_button(ui, "◌", "Pick neutral").on_hover_text("Click something gray to set the white balance").clicked() {
                        self.edit.tool = Tool::PickNeutral;
                    }
                    ui.end_row();
                    if tool_button(ui, "⟲", "Rotate left").clicked() {
                        self.rotate(&[id], -1);
                    }
                    if tool_button(ui, "⟳", "Rotate right").clicked() {
                        self.rotate(&[id], 1);
                    }
                    ui.end_row();
                });
            }
        }
    }

    fn advanced_panel(&mut self, ui: &mut egui::Ui) {
        let id = self.edit.id;
        let mut a = self.current_adjust(id);
        let before = a;
        let section = |ui: &mut egui::Ui, title: &str| {
            ui.add_space(6.0);
            ui.label(RichText::new(title).size(12.0).strong().color(t().text_weak));
        };
        ui.spacing_mut().slider_width = 108.0;
        section(ui, "WHITE BALANCE");
        let preset = develop::WB_PRESETS.iter().find(|(_, t, ti)| *t == a.temp && *ti == a.tint).map(|(n, _, _)| *n).unwrap_or("Custom");
        ui.horizontal(|ui| {
            egui::ComboBox::from_id_salt("wb_preset").selected_text(preset).width(120.0).show_ui(ui, |ui| {
                for (name, t, ti) in develop::WB_PRESETS {
                    if ui.selectable_label(preset == name, name).clicked() {
                        (a.temp, a.tint) = (t, ti);
                    }
                }
                if ui.selectable_label(false, "Auto").clicked()
                    && let Some(img) = self.analysis_image(id)
                {
                    (a.temp, a.tint) = develop::auto_white_balance(&img);
                }
            });
            if ui.button("◌ Pick").on_hover_text("Click something gray in the picture").clicked() {
                self.edit.tool = Tool::PickNeutral;
            }
        });
        let slider = |ui: &mut egui::Ui, label: &str, v: &mut f32, range: std::ops::RangeInclusive<f32>, tip: &str| {
            ui.horizontal(|ui| {
                let l = ui.add_sized([74.0, 18.0], egui::Label::new(RichText::new(label).size(12.5)).sense(Sense::click())).on_hover_text(format!("{tip}\nDouble-click to reset"));
                if l.double_clicked() {
                    *v = 0.0;
                }
                ui.add(egui::Slider::new(v, range).max_decimals(if label == "Exposure" { 2 } else { 0 }));
            });
        };
        slider(ui, "Temperature", &mut a.temp, -100.0..=100.0, "Bluer (left) or warmer (right)");
        slider(ui, "Tint", &mut a.tint, -100.0..=100.0, "Greener (left) or more magenta (right)");
        section(ui, "LIGHT");
        slider(ui, "Exposure", &mut a.exposure, -3.0..=3.0, "Overall brightness, in stops");
        slider(ui, "Highlights", &mut a.highlights, -100.0..=100.0, "Bright areas: left recovers detail, right brightens");
        slider(ui, "Shadows", &mut a.shadows, -100.0..=100.0, "Dark areas: right opens them up");
        slider(ui, "Blacks", &mut a.blacks, -100.0..=100.0, "Black point: left deepens, right lifts");
        slider(ui, "Contrast", &mut a.contrast, -100.0..=100.0, "Difference between dark and bright");
        section(ui, "COLOUR");
        slider(ui, "Saturation", &mut a.saturation, -100.0..=100.0, "All colours");
        slider(ui, "Vibrance", &mut a.vibrance, -100.0..=100.0, "Mostly the dull colours; spares skin and already vivid ones");
        section(ui, "DETAIL");
        slider(ui, "Clarity", &mut a.clarity, -100.0..=100.0, "Local contrast in the midtones");
        slider(ui, "Sharpen", &mut a.sharpen, 0.0..=100.0, "Crisper fine detail");
        slider(ui, "Noise", &mut a.noise, 0.0..=100.0, "Noise reduction: smooths grain while keeping edges");
        if a != before {
            self.set_adjust(id, a);
        }
        ui.add_space(8.0);
        if ui.add_enabled(!a.is_neutral(), egui::Button::new("Reset all")).clicked() {
            self.set_adjust(id, Adjust::default());
        }
        ui.label(RichText::new("Moving a slider again right after changes the same step, so Undo takes it back in one go.").size(11.5).weak());
    }

    /// The preset look being edited: the last edit step when it is a look.
    fn current_look(&self, id: usize) -> Option<(Look, f32)> {
        match self.meta(id).and_then(|m| m.edits.last()) {
            Some(EditOp::Look { look, amount }) => Some((*look, *amount)),
            _ => None,
        }
    }

    /// Choose a look: it replaces the look chosen just before (so trying presets doesn't stack
    /// them), otherwise it is added at full strength.
    fn set_look(&mut self, id: usize, look: Look, amount: f32) {
        let op = EditOp::Look { look, amount: amount.clamp(0.0, 1.0) };
        let m = self.meta_mut(id);
        match m.edits.last_mut() {
            Some(last @ EditOp::Look { .. }) => *last = op,
            _ => m.edits.push(op),
        }
        self.edit.redo.clear();
        self.refresh_thumb(id);
    }

    fn presets_tab(&mut self, ui: &mut egui::Ui) {
        let id = self.edit.id;
        let current = self.current_look(id);
        // previews start from the picture before the current look, so they show the alternatives
        let mut start = self.edits(id);
        if current.is_some() {
            start.pop();
        }
        let fresh = matches!(&self.look_previews, Some((pid, e, _)) if *pid == id && *e == start);
        if !fresh {
            if let Some(base) = self.thumb_base.get(&id) {
                let (w, h) = imaging::fit(base.width(), base.height(), 96);
                let small = image::imageops::resize(base, w, h, image::imageops::FilterType::Triangle);
                let cur = imaging::apply_all(&small, &start);
                let texs = Look::ALL
                    .iter()
                    .map(|l| {
                        let mut img = cur.clone();
                        crate::looks::apply(&mut img, *l, 1.0);
                        self.ctx.load_texture(format!("look{}", l.label()), to_color_image(&img), TextureOptions::LINEAR)
                    })
                    .collect();
                self.look_previews = Some((id, start.clone(), texs));
            } else {
                self.request_thumb(id);
            }
        }
        if let Some((look, mut amount)) = current {
            ui.label(RichText::new(look.label()).strong().size(15.0));
            ui.horizontal(|ui| {
                ui.label("Intensity");
                ui.spacing_mut().slider_width = 120.0;
                let mut pct = amount * 100.0;
                if ui.add(egui::Slider::new(&mut pct, 0.0..=100.0).suffix(" %").max_decimals(0)).changed() {
                    amount = pct / 100.0;
                    self.set_look(id, look, amount);
                }
            });
            if ui.button("Remove preset").clicked() {
                self.undo();
            }
            ui.separator();
        } else {
            ui.label(RichText::new("Looks for the whole picture. Click one, then set its intensity.").size(12.0).weak());
        }
        let mut chosen = None;
        egui::ScrollArea::vertical().show(ui, |ui| {
            egui::Grid::new("looks").num_columns(3).spacing([4.0, 4.0]).show(ui, |ui| {
                for (i, l) in Look::ALL.iter().enumerate() {
                    let (rect, resp) = ui.allocate_exact_size(egui::vec2(76.0, 80.0), Sense::click());
                    let p = ui.painter();
                    let on = current.is_some_and(|(c, _)| c == *l);
                    if on {
                        p.rect_filled(rect, 4, t().select_bg);
                    } else if resp.hovered() {
                        p.rect_filled(rect, 4, t().hover_wash);
                    }
                    let img_area = Rect::from_min_size(rect.min + egui::vec2(4.0, 4.0), egui::vec2(68.0, 52.0));
                    let tex = self.look_previews.as_ref().and_then(|(_, _, t)| t.get(i));
                    paint_thumb(p, img_area, tex, true);
                    p.text(rect.center_bottom() - egui::vec2(0.0, 12.0), egui::Align2::CENTER_CENTER, l.label(), egui::FontId::proportional(11.0), t().text);
                    if resp.on_hover_text(l.tip()).clicked() {
                        chosen = Some(*l);
                    }
                    if i % 3 == 2 {
                        ui.end_row();
                    }
                }
            });
        });
        if let Some(l) = chosen {
            let amount = current.map_or(1.0, |(_, a)| a);
            self.set_look(id, l, amount);
        }
    }

    /// The effect being edited: the last edit step when it is one.
    fn current_decor(&self, id: usize) -> Option<Decor> {
        match self.meta(id).and_then(|m| m.edits.last()) {
            Some(EditOp::Decor(d)) => Some(d.clone()),
            _ => None,
        }
    }

    /// Change the last effect when it is of the same kind, otherwise add this one.
    fn set_decor(&mut self, id: usize, d: Decor) {
        let m = self.meta_mut(id);
        match m.edits.last_mut() {
            Some(EditOp::Decor(cur)) if cur.same_kind(&d) => *cur = d,
            _ => m.edits.push(EditOp::Decor(d)),
        }
        self.edit.redo.clear();
        self.refresh_thumb(id);
    }

    fn effects_tab(&mut self, ui: &mut egui::Ui) {
        let id = self.edit.id;
        ui.label(RichText::new("Add on or around the picture").size(12.0).weak());
        let white = [255, 255, 255];
        let adds: [(&str, &str, Decor); 6] = [
            ("◐", "Vignette", Decor::Vignette { amount: 0.5, size: 0.5 }),
            ("▢", "Border", Decor::Border { width: 0.03, bottom: 0.0, color: white }),
            ("🖼", "Instant frame", Decor::Border { width: 0.05, bottom: 0.15, color: [250, 248, 242] }),
            ("▣", "Drop shadow", Decor::Shadow { size: 0.04, opacity: 0.55, background: white }),
            ("▢", "Rounded corners", Decor::Rounded { radius: 0.06, background: white }),
            ("🔤", "Text", Decor::Text(TextOverlay { text: "Your text".into(), ..Default::default() })),
        ];
        let mut add = None;
        egui::Grid::new("decor_add").num_columns(2).spacing([6.0, 6.0]).show(ui, |ui| {
            for (i, (icon, label, d)) in adds.iter().enumerate() {
                if tool_button(ui, icon, label).clicked() {
                    add = Some(d.clone());
                }
                if i % 2 == 1 {
                    ui.end_row();
                }
            }
        });
        if let Some(d) = add {
            // always a new step (adding two borders makes a double frame)
            self.push_edit(id, EditOp::Decor(d));
        }
        let Some(mut d) = self.current_decor(id) else {
            ui.add_space(8.0);
            ui.label(RichText::new("Effects go on top of every other edit. Add one to adjust it here.").size(11.5).weak());
            return;
        };
        let before = d.clone();
        ui.separator();
        ui.label(RichText::new(d.label()).strong().size(15.0));
        ui.spacing_mut().slider_width = 120.0;
        let pct = |ui: &mut egui::Ui, label: &str, v: &mut f32, lo: f32, hi: f32| {
            ui.horizontal(|ui| {
                ui.add_sized([70.0, 18.0], egui::Label::new(label));
                let mut p = *v * 100.0;
                if ui.add(egui::Slider::new(&mut p, lo..=hi).suffix(" %").max_decimals(0)).changed() {
                    *v = p / 100.0;
                }
            });
        };
        match &mut d {
            Decor::Vignette { amount, size } => {
                pct(ui, "Amount", amount, -100.0, 100.0);
                pct(ui, "Size", size, 0.0, 100.0);
                ui.label(RichText::new("Left of the middle lightens the corners instead.").size(11.0).weak());
            }
            Decor::Border { width, bottom, color } => {
                pct(ui, "Width", width, 0.0, 20.0);
                pct(ui, "Bottom", bottom, 0.0, 30.0);
                swatches(ui, "Colour", color);
            }
            Decor::Shadow { size, opacity, background } => {
                pct(ui, "Size", size, 1.0, 10.0);
                pct(ui, "Darkness", opacity, 0.0, 100.0);
                swatches(ui, "Background", background);
            }
            Decor::Rounded { radius, background } => {
                pct(ui, "Radius", radius, 1.0, 25.0);
                swatches(ui, "Background", background);
            }
            Decor::Text(t) => {
                ui.add(egui::TextEdit::singleline(&mut t.text).hint_text("Your text").desired_width(f32::INFINITY));
                pct(ui, "Size", &mut t.size, 2.0, 25.0);
                swatches(ui, "Colour", &mut t.color);
                ui.checkbox(&mut t.halo, "Soft halo (readable on any picture)");
                ui.label("Position");
                egui::Grid::new("text_anchor").spacing([2.0, 2.0]).show(ui, |ui| {
                    for a in 0..9u8 {
                        let (rect, resp) = ui.allocate_exact_size(egui::vec2(28.0, 20.0), Sense::click());
                        let on = t.anchor == a;
                        let fill = if on { crate::theme::t().accent } else if resp.hovered() { crate::theme::t().surface_hover } else { crate::theme::t().surface };
                        ui.painter().rect_filled(rect, 3, fill);
                        ui.painter().rect_stroke(rect, 3, Stroke::new(1.0, crate::theme::t().border), StrokeKind::Inside);
                        if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                            t.anchor = a;
                        }
                        if a % 3 == 2 {
                            ui.end_row();
                        }
                    }
                });
            }
        }
        if d != before {
            self.set_decor(id, d);
        }
        ui.add_space(6.0);
        if ui.button("Remove this effect").clicked() {
            self.undo();
        }
    }

    fn edit_bottom_panel(&mut self, ui: &mut egui::Ui) {
        let id = self.edit.id;
        egui::Panel::bottom("edit_bottom")
            .frame(egui::Frame::new().fill(t().tray_bg).inner_margin(egui::Margin::symmetric(8, 6)).stroke(Stroke::new(1.0, t().border)))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    use crate::library::Flag;
                    use crate::library_view::{pick_button, rating_input, reject_button};
                    if pick_button(ui, self.flag(id) == Flag::Pick, egui::vec2(34.0, 28.0)).on_hover_text("Pick (P)").clicked() {
                        self.toggle_flag(&[id], Flag::Pick);
                    }
                    if reject_button(ui, self.flag(id) == Flag::Reject, egui::vec2(34.0, 28.0)).on_hover_text("Reject (X)").clicked() {
                        self.toggle_flag(&[id], Flag::Reject);
                    }
                    if let Some(n) = rating_input(ui, egui::Id::unique(("edit_rating", id)), self.rating(id), 8.0) {
                        self.set_rating(&[id], n);
                    }
                    ui.add_space(6.0);
                    if ui.add_sized([34.0, 28.0], egui::Button::new(RichText::new("⟲").size(17.0))).on_hover_text("Rotate left").clicked() {
                        self.rotate(&[id], -1);
                    }
                    if ui.add_sized([34.0, 28.0], egui::Button::new(RichText::new("⟳").size(17.0))).on_hover_text("Rotate right").clicked() {
                        self.rotate(&[id], 1);
                    }
                    ui.separator();
                    let mut caption = self.meta(id).map(|m| m.caption.clone()).unwrap_or_default();
                    let r = ui.add(egui::TextEdit::singleline(&mut caption).hint_text("Make a caption!").desired_width((ui.available_width() - 560.0).max(120.0)));
                    if r.changed() {
                        self.meta_mut(id).caption = caption;
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("💾 Export").clicked() {
                            self.dialog = Some(Dialog::Export { max_dim: 0, quality: 90 });
                        }
                        if crate::print::available() && ui.button("🖨 Print").on_hover_text("Print this picture (Ctrl+P)").clicked() {
                            self.open_print();
                        }
                        let mut add = None;
                        ui.menu_button("📚 Album ▾", |ui| {
                            for (i, a) in self.store.albums.iter().enumerate() {
                                if ui.button(&a.name).clicked() {
                                    add = Some(i);
                                    ui.close();
                                }
                            }
                            if ui.button("➕ New album…").clicked() {
                                self.dialog = Some(Dialog::NewAlbum { name: String::new() });
                                ui.close();
                            }
                        });
                        if let Some(i) = add {
                            self.add_to_album(i, &[id]);
                        }
                        ui.separator();
                        // Zoom: Fit, 1:1 and the zoom slider:
                        // the slider runs from Fit (left) through 1:1 (middle) to 400 % (right).
                        let fit = self.edit.fit_zoom;
                        let z_now = self.edit.zoom.unwrap_or(fit);
                        let mut p = if self.edit.zoom.is_some() { zoom_to_pos(z_now, fit) } else { 0.0 };
                        ui.label(RichText::new(format!("{:.0} %", z_now * 100.0)).size(12.0)).on_hover_text("Zoom (100 % = the photo's real pixels)");
                        ui.spacing_mut().slider_width = 120.0;
                        let s = ui.add(egui::Slider::new(&mut p, 0.0..=1.0).show_value(false));
                        if s.on_hover_text("Zoom: left = fit, middle = 1:1, right = 400 % (mouse wheel works too; drag the picture to move it)").changed() {
                            self.edit.zoom_anim = None;
                            if p <= 0.001 {
                                self.edit.zoom = None;
                                self.edit.pan = Vec2::ZERO;
                            } else {
                                self.edit.zoom = Some(pos_to_zoom(p, fit));
                            }
                        }
                        let now = ui.input(|i| i.time);
                        if ui
                            .add(egui::Button::new(RichText::new("1:1").size(12.5)).selected(self.edit.zoom.is_some_and(|z| (z - 1.0).abs() < 0.005)))
                            .on_hover_text("Original size (100 %): one photo pixel per screen pixel")
                            .clicked()
                        {
                            self.edit.zoom_anim = Some(ZoomAnim { from: z_now, to: Some(1.0), t0: now, pan_from: self.edit.pan });
                        }
                        if ui
                            .add(egui::Button::new(RichText::new("Fit").size(12.5)).selected(self.edit.zoom.is_none()))
                            .on_hover_text("Fit the whole picture in the window (double-click a zoomed picture does the same)")
                            .clicked()
                            && self.edit.zoom.is_some()
                        {
                            self.edit.zoom_anim = Some(ZoomAnim { from: z_now, to: None, t0: now, pan_from: self.edit.pan });
                        }
                    });
                });
                // tags and people chips
                ui.horizontal_wrapped(|ui| {
                    let (tags, people) = self.meta(id).map(|m| (m.tags.clone(), m.people.clone())).unwrap_or_default();
                    ui.label(RichText::new("Tags:").size(12.0));
                    for t in &tags {
                        if ui.small_button(format!("{t} ✖")).on_hover_text("Remove tag").clicked() {
                            self.meta_mut(id).tags.retain(|x| x != t);
                        }
                    }
                    let r = ui.add(egui::TextEdit::singleline(&mut self.edit.tag_buf).hint_text("add tag").desired_width(90.0));
                    if r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) && !self.edit.tag_buf.trim().is_empty() {
                        let t = self.edit.tag_buf.trim().to_string();
                        self.edit.tag_buf.clear();
                        let m = self.meta_mut(id);
                        if !m.tags.contains(&t) {
                            m.tags.push(t);
                        }
                    }
                    ui.add_space(10.0);
                    ui.label(RichText::new("People:").size(12.0));
                    for t in &people {
                        if ui.small_button(format!("👤 {t} ✖")).on_hover_text("Remove person").clicked() {
                            self.meta_mut(id).people.retain(|x| x != t);
                        }
                    }
                    let r = ui.add(egui::TextEdit::singleline(&mut self.edit.person_buf).hint_text("add person").desired_width(90.0));
                    if r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) && !self.edit.person_buf.trim().is_empty() {
                        let t = self.edit.person_buf.trim().to_string();
                        self.edit.person_buf.clear();
                        let m = self.meta_mut(id);
                        if !m.people.contains(&t) {
                            m.people.push(t);
                        }
                    }
                });
                // filmstrip
                let list = self.edit.list.clone();
                let mut go = None;
                egui::ScrollArea::horizontal().id_salt("filmstrip").max_height(62.0).show(ui, |ui| {
                    ui.horizontal(|ui| {
                        for &pid in &list {
                            let (rect, resp) = ui.allocate_exact_size(egui::vec2(60.0, 54.0), Sense::click());
                            if ui.is_rect_visible(rect) {
                                if !self.thumbs.contains_key(&pid) {
                                    self.request_thumb(pid);
                                }
                                let p = ui.painter();
                                if pid == id {
                                    p.rect_filled(rect, 3, t().accent);
                                } else if resp.hovered() {
                                    p.rect_filled(rect, 3, t().hover_wash);
                                }
                                paint_thumb(p, rect.shrink(4.0), self.thumbs.get(&pid), false);
                            }
                            // follow the shown picture: centre its thumbnail whenever the picture changes,
                            // even when it is scrolled out of sight
                            if pid == id && self.edit.strip_shown != Some(id) {
                                resp.scroll_to_me(Some(egui::Align::Center));
                                self.edit.strip_shown = Some(id);
                            }
                            if resp.clicked() {
                                go = Some(pid);
                            }
                        }
                    });
                });
                if let Some(pid) = go
                    && pid != id {
                        self.edit_go(PendingLeave::Photo(pid));
                    }
            });
    }

    fn properties_panel(&mut self, ui: &mut egui::Ui) {
        let id = self.edit.id;
        let p = self.photos[id].clone();
        let dims = self.real_dims_of(id);
        let cam = self.camera_info(id);
        let thumb = self.exif_thumbs.get(&id).cloned();
        let m = self.meta(id).cloned().unwrap_or_default();
        let mut close = false;
        egui::Panel::right("props")
            .exact_size(270.0)
            .frame(egui::Frame::new().fill(t().surface).inner_margin(egui::Margin::same(0)).stroke(Stroke::new(1.0, t().border)))
            .show(ui, |ui| {
                // title bar of the "Properties" pane
                egui::Frame::new().fill(t().tray_well).inner_margin(egui::Margin::symmetric(10, 5)).show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Properties").strong().size(14.0).color(Color32::WHITE));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.small_button("✖").on_hover_text("Close").clicked() {
                                close = true;
                            }
                        });
                    });
                });
                egui::Frame::new().inner_margin(egui::Margin::symmetric(10, 6)).show(ui, |ui| {
                    ui.label(RichText::new(format!("Properties of {}", p.name)).strong().size(13.0));
                });
                egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                    egui::Frame::new().inner_margin(egui::Margin { left: 10, right: 10, top: 0, bottom: 10 }).show(ui, |ui| {
                        // one property: bold name with the value underneath, a thin line between rows
                        let row = |ui: &mut egui::Ui, k: &str, v: Option<String>| {
                            let Some(v) = v.filter(|v| !v.is_empty()) else { return };
                            ui.label(RichText::new(k).strong().size(11.5).color(t().text));
                            ui.horizontal(|ui| {
                                ui.add_space(6.0);
                                ui.add(egui::Label::new(RichText::new(v).size(12.5).color(t().text)).wrap_mode(egui::TextWrapMode::Wrap));
                            });
                            let r = ui.available_rect_before_wrap();
                            ui.painter().hline(r.x_range(), r.top() + 2.0, Stroke::new(1.0, t().border));
                            ui.add_space(5.0);
                        };
                        let location = if p.is_sample() { Some("Built-in sample picture".into()) } else { Some(self.folders[p.folder].path.clone()) };
                        // labels and value formats of the metadata table
                        row(ui, "Location", location);
                        row(ui, "File Size", Some(fmt_bytes(p.bytes)));
                        row(ui, "Dimensions", dims.map(|(w, h)| format!("{w} x {h} pixels")));
                        if let Some(c) = cam.as_deref() {
                            row(ui, "Camera Make", c.make.clone());
                            row(ui, "Camera Model", c.model.clone());
                            row(ui, "Lens", c.lens.clone());
                            row(ui, "Camera Date", c.date_taken.clone());
                            row(ui, "Digitized Date", c.date_digitized.clone());
                            row(ui, "Modified Date", c.date_modified.clone());
                        }
                        row(ui, "File Date", Some(format!("{} {}", fmt_date(p.modified), fmt_time(p.modified))));
                        if let Some(c) = cam.as_deref() {
                            let num = crate::exifinfo::trim_num;
                            row(ui, "Flash", c.flash.clone());
                            row(ui, "Focal Length", c.focal_mm.map(|f| format!("{}mm", num(f, 1))));
                            row(ui, "Focal Length in 35mm Film", c.focal_35mm.map(|e| format!("{e}mm")));
                            row(ui, "Exposure Time", c.exposure_long());
                            row(ui, "F Number", c.fnumber.map(|f| format!("f/{}", num(f, 1))));
                            row(ui, "ISO", c.iso.map(|i| i.to_string()));
                            row(ui, "Exposure Bias", c.exposure_bias.filter(|b| *b != 0.0).map(|b| num(b, 2)));
                            row(ui, "Subject Distance", c.subject_distance.map(|d| format!("{}m", num(d, 2))));
                            row(ui, "Metering Mode", c.metering.clone());
                            row(ui, "Exposure Program", c.program.clone());
                            row(ui, "White Balance", c.white_balance.clone());
                            row(ui, "Color Space", c.color_space.clone());
                            row(
                                ui,
                                "Orientation",
                                c.orientation.filter(|&o| o != 1).map(|o| {
                                    match o {
                                        2 => "Mirrored",
                                        3 => "Upside down",
                                        4 => "Mirrored, upside down",
                                        5 => "Mirrored, turned left",
                                        6 => "Turned right (camera held upright)",
                                        7 => "Mirrored, turned right",
                                        8 => "Turned left (camera held upright)",
                                        _ => "Unknown",
                                    }
                                    .to_string()
                                }),
                            );
                            row(ui, "Software", c.software.clone());
                        } else {
                            ui.label(RichText::new("No camera information (EXIF) in this picture.").size(12.0).weak());
                            ui.add_space(6.0);
                        }
                        if let Some(th) = &thumb {
                            let [w, h] = th.size();
                            ui.label(RichText::new(format!("Thumbnail ({w} x {h} pixels)")).strong().size(11.5).color(t().text));
                            let s = (200.0 / w.max(h) as f32).min(1.0);
                            ui.add(egui::Image::new((th.id(), egui::vec2(w as f32 * s, h as f32 * s))));
                            ui.add_space(6.0);
                        }
                        row(ui, "Caption", Some(m.caption.clone()));
                        row(ui, "Tags", Some(m.tags.join(", ")));
                        row(ui, "People", Some(m.people.join(", ")));
                        let flag = match m.flag {
                            crate::library::Flag::Pick => Some("Pick".to_string()),
                            crate::library::Flag::Reject => Some("Rejected".to_string()),
                            crate::library::Flag::None => None,
                        };
                        row(ui, "Flag", flag);
                        row(ui, "Rating", (m.rating > 0).then(|| crate::library_view::rating_label(m.rating)));
                        row(ui, "Edits", (!m.edits.is_empty()).then(|| m.edits.iter().map(|e| e.label()).collect::<Vec<_>>().join(", ")));

                        // every EXIF entry the file holds
                        if let Some(c) = cam.as_deref().filter(|c| !c.all.is_empty()) {
                            ui.add_space(4.0);
                            egui::CollapsingHeader::new(RichText::new(format!("All EXIF data ({} entries)", c.all.len())).strong().size(12.5))
                                .id_salt(("allexif", id))
                                .default_open(false)
                                .show(ui, |ui| {
                                    egui::Grid::new(("exif_all", id)).num_columns(2).striped(true).spacing([8.0, 3.0]).show(ui, |ui| {
                                        for (k, v) in &c.all {
                                            ui.label(RichText::new(k).size(11.0).strong());
                                            ui.add(egui::Label::new(RichText::new(v).size(11.0)).wrap());
                                            ui.end_row();
                                        }
                                    });
                                });
                        }
                    });
                });
            });
        if close {
            self.edit.show_info = false;
        }
    }

    /// "A|A'" (before / after) and "A|B" (two pictures) views: two halves side by side, the edited
    /// picture marked "Selected". Clicking the other picture in A|B moves the focus to it.
    fn compare_canvas(&mut self, ui: &mut egui::Ui) {
        let id = self.edit.id;
        let full = ui.available_rect_before_wrap();
        let gap = 4.0;
        // first = left (or top), second = right (or bottom); swap_2up_layout flips the split direction
        let (first, second, divider) = if self.edit.vertical {
            let h = (full.height() - gap) / 2.0;
            (
                Rect::from_min_size(full.min, egui::vec2(full.width(), h)),
                Rect::from_min_size(Pos2::new(full.left(), full.top() + h + gap), egui::vec2(full.width(), h)),
                Rect::from_min_size(Pos2::new(full.left(), full.top() + h), egui::vec2(full.width(), gap)),
            )
        } else {
            let w = (full.width() - gap) / 2.0;
            (
                Rect::from_min_size(full.min, egui::vec2(w, full.height())),
                Rect::from_min_size(Pos2::new(full.left() + w + gap, full.top()), egui::vec2(w, full.height())),
                Rect::from_min_size(Pos2::new(full.left() + w, full.top()), egui::vec2(gap, full.height())),
            )
        };
        // the focused side ("Selected") holds the picture being edited
        let (a_rect, b_rect) = if self.edit.focus_second { (second, first) } else { (first, second) };

        let a_tex = self.big_texture(id, &[]).or_else(|| self.thumbs.get(&id).cloned());
        let (b_tex, b_label) = match self.edit.compare {
            CompareMode::BeforeAfter => {
                let alt = self.edit.alt_edits.clone();
                let label = if alt.is_empty() { "Original".to_string() } else { "Other version".to_string() };
                (self.big_version_texture(id, &alt), label)
            }
            _ => match self.compare_partner() {
                Some(o) => {
                    if !self.thumbs.contains_key(&o) {
                        self.request_thumb(o);
                    }
                    let t = self.big_texture(o, &[]).or_else(|| self.thumbs.get(&o).cloned());
                    (t, self.photos[o].name.clone())
                }
                None => (None, "No other picture in this folder".to_string()),
            },
        };
        let a_label = "Selected";
        // same zoom and position on both sides, so details line up
        let ppp = ui.ctx().pixels_per_point();
        let r_a = if self.big.contains_key(&id) { self.real_scale(id) } else { None };
        let r_b = match self.edit.compare {
            CompareMode::BeforeAfter => r_a,
            _ => self.compare_partner().filter(|o| self.big.contains_key(o)).and_then(|o| self.real_scale(o)),
        };
        if let (Some(t), Some(r)) = (&a_tex, r_a) {
            self.edit.fit_zoom = fit_rect(a_rect.shrink(10.0), t.size()).width() * ppp / (t.size()[0] as f32 * r);
        }

        let resp_a = ui.allocate_rect(a_rect, Sense::click_and_drag());
        let resp_b = ui.allocate_rect(b_rect, Sense::click_and_drag());
        if self.edit.zoom.is_none() && r_a.is_some() && (resp_a.hovered() || resp_b.hovered()) && ui.input(|i| i.smooth_scroll_delta.y) > 0.0 {
            self.edit.zoom = Some(self.edit.fit_zoom);
        }
        self.zoom_input(ui, &resp_a, a_rect.shrink(10.0));
        self.zoom_input(ui, &resp_b, b_rect.shrink(10.0));
        if self.edit.zoom.is_some() && self.edit.zoom_anim.is_none() && (resp_a.double_clicked() || resp_b.double_clicked()) {
            self.start_zoom_anim(ui.ctx(), None);
        }
        if let (Some(z), Some(t), Some(r)) = (self.edit.zoom, &a_tex, r_a) {
            let size = egui::vec2(t.size()[0] as f32, t.size()[1] as f32) * (r * z / ppp);
            self.edit.pan = clamp_pan(self.edit.pan, size, a_rect.shrink(10.0).size());
        }
        let (zoom, pan) = (self.edit.zoom, self.edit.pan);
        let p = ui.painter().clone();
        // thin dark line between the halves
        p.rect_filled(divider, 0, Color32::from_rgb(70, 70, 70));

        let draw = |rect: Rect, tex: Option<&egui::TextureHandle>, r: Option<f32>, label: &str, focused: bool| {
            let area = rect.shrink(10.0);
            let clip = p.with_clip_rect(area);
            let img = match tex {
                Some(t) => {
                    let r = place(area, t.size(), r, zoom, pan, ppp);
                    if zoom.is_none() {
                        clip.rect_filled(r.translate(egui::vec2(3.0, 3.0)), 0, Color32::from_black_alpha(80));
                    }
                    clip.image(t.id(), r, Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)), Color32::WHITE);
                    r.intersect(area)
                }
                None => {
                    p.text(area.center(), egui::Align2::CENTER_CENTER, "Loading…", egui::FontId::proportional(14.0), Color32::WHITE);
                    area
                }
            };
            let font = egui::FontId::proportional(14.0);
            let galley = p.layout_no_wrap(label.to_string(), font, Color32::WHITE);
            let pill = Rect::from_min_size(
                Pos2::new(img.right() - galley.size().x - 34.0, img.top() + 24.0),
                galley.size() + egui::vec2(20.0, 10.0),
            );
            let fill = if focused { Color32::from_rgba_unmultiplied(45, 45, 45, 215) } else { Color32::from_rgba_unmultiplied(90, 90, 90, 190) };
            p.rect_filled(pill, 4, fill);
            p.galley(pill.min + egui::vec2(10.0, 5.0), galley, Color32::WHITE);
        };
        draw(a_rect, a_tex.as_ref(), r_a, a_label, true);
        draw(b_rect, b_tex.as_ref(), r_b, &b_label, false);

        // clicking the other side moves the focus there
        let tip = match self.edit.compare {
            CompareMode::BeforeAfter => "Click to edit this version instead",
            _ => "Click to edit this picture instead",
        };
        let has_other = self.edit.compare == CompareMode::BeforeAfter || self.compare_partner().is_some();
        if has_other && resp_b.on_hover_text(tip).clicked() {
            self.swap_2up_focus();
        }
        let _ = resp_a;
        if let Some(c) = self.meta(id).map(|m| m.caption.clone()).filter(|c| !c.is_empty()) {
            p.text(
                Pos2::new(a_rect.center().x, full.bottom() - 10.0),
                egui::Align2::CENTER_CENTER,
                c,
                egui::FontId::proportional(13.0),
                Color32::from_rgb(240, 240, 240),
            );
        }
    }

    /// Advance a Fit / 1:1 change (0.3 s;
    /// a smooth ease-in-out is used).
    fn tick_zoom_anim(&mut self, ctx: &egui::Context) {
        let Some(a) = self.edit.zoom_anim else { return };
        let fit = self.edit.fit_zoom;
        let t = ((ctx.input(|i| i.time) - a.t0) / ZOOM_ANIM_SECS).clamp(0.0, 1.0) as f32;
        let e = t * t * (3.0 - 2.0 * t);
        let target = a.to.unwrap_or(fit);
        if t >= 1.0 {
            self.edit.zoom = a.to;
            self.edit.pan = Vec2::ZERO;
            self.edit.zoom_anim = None;
        } else {
            self.edit.zoom = Some(a.from + (target - a.from) * e);
            self.edit.pan = a.pan_from * (1.0 - e);
            ctx.request_repaint();
        }
    }

    fn start_zoom_anim(&mut self, ctx: &egui::Context, to: Option<f32>) {
        let from = self.edit.zoom.unwrap_or(self.edit.fit_zoom);
        let t0 = ctx.input(|i| i.time);
        self.edit.zoom_anim = Some(ZoomAnim { from, to, t0, pan_from: self.edit.pan });
    }

    /// Mouse wheel and dragging over a zoomed view: zoom around the pointer, move the picture.
    fn zoom_input(&mut self, ui: &egui::Ui, resp: &egui::Response, view: Rect) {
        let Some(z) = self.edit.zoom else { return };
        if resp.dragged() {
            self.edit.pan += resp.drag_delta();
            ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
        } else if resp.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
        }
        if resp.hovered() {
            let s = ui.input(|i| i.smooth_scroll_delta.y);
            if s != 0.0 {
                // between "fit" (the smallest) and 400 %
                let fit = self.edit.fit_zoom;
                let nz = (z * (s * 0.0025).exp()).min(ZOOM_MAX.max(fit));
                self.edit.zoom_anim = None;
                if nz <= fit {
                    self.edit.zoom = None;
                    self.edit.pan = Vec2::ZERO;
                    return;
                }
                if let Some(pos) = resp.hover_pos() {
                    let rel = pos - (view.center() + self.edit.pan);
                    self.edit.pan += rel - rel * (nz / z);
                }
                self.edit.zoom = Some(nz);
            }
        }
    }

    /// One picture at a chosen zoom. When the preview would be magnified, a sharp render made
    /// from the full photo (up to 5000 pixels) replaces it as soon as it is ready.
    fn zoomed_canvas(&mut self, ui: &mut egui::Ui, canvas: Rect, id: usize, preview: egui::TextureHandle, r_prev: f32, z: f32) {
        let ppp = ui.ctx().pixels_per_point();
        let ops = self.edits(id);
        let magnified = r_prev * z / ppp > 1.05;
        if magnified {
            self.want_zoom_render(id, &ops);
        }
        let (tex, r) = match &self.edit.zoom_tex {
            Some((zid, zops, t, r)) if magnified && *zid == id && *zops == ops => (t.clone(), *r),
            _ => (preview.clone(), r_prev),
        };
        let resp = ui.allocate_rect(canvas, Sense::click_and_drag());
        self.zoom_input(ui, &resp, canvas);
        let z = self.edit.zoom.unwrap_or(z);
        let size = egui::vec2(tex.size()[0] as f32, tex.size()[1] as f32) * (r * z / ppp);
        self.edit.pan = clamp_pan(self.edit.pan, size, canvas.size());
        let img = Rect::from_center_size(canvas.center() + self.edit.pan, size);
        let p = ui.painter_at(canvas);
        p.image(tex.id(), img, Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)), Color32::WHITE);
        if resp.double_clicked() && self.edit.zoom_anim.is_none() {
            self.start_zoom_anim(ui.ctx(), None);
        }
        if magnified && self.edit.zoom_req.is_some() && !matches!(&self.edit.zoom_tex, Some((zid, zops, _, _)) if *zid == id && *zops == ops) {
            p.text(canvas.center_top() + egui::vec2(0.0, 16.0), egui::Align2::CENTER_CENTER, "Sharpening…", egui::FontId::proportional(13.0), Color32::WHITE);
        }

        // overview box in the corner ("Zoom 153 %")
        let nav_max = 130.0;
        let s = nav_max / (preview.size()[0].max(preview.size()[1]) as f32);
        let nav_size = egui::vec2(preview.size()[0] as f32 * s, preview.size()[1] as f32 * s);
        let frame = Rect::from_min_size(canvas.right_bottom() - nav_size - egui::vec2(22.0, 40.0), nav_size + egui::vec2(12.0, 30.0));
        p.rect_filled(frame, 4, Color32::from_rgba_unmultiplied(40, 40, 40, 200));
        p.text(frame.left_top() + egui::vec2(6.0, 4.0), egui::Align2::LEFT_TOP, format!("Zoom {:.0} %", z * 100.0), egui::FontId::proportional(12.0), Color32::WHITE);
        let nav = Rect::from_min_size(frame.left_top() + egui::vec2(6.0, 22.0), nav_size);
        p.image(preview.id(), nav, Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)), Color32::WHITE);
        // the part that is visible on screen
        let vis = canvas.intersect(img);
        if vis.is_positive() {
            let to_nav = |q: Pos2| Pos2::new(nav.left() + (q.x - img.left()) / img.width() * nav.width(), nav.top() + (q.y - img.top()) / img.height() * nav.height());
            p.rect_stroke(Rect::from_min_max(to_nav(vis.min), to_nav(vis.max)), 0, Stroke::new(1.5, Color32::from_rgb(255, 210, 80)), StrokeKind::Inside);
        }
        // clicking or dragging in the overview moves the view there
        let nav_resp = ui.interact(nav, ui.scope_id().with("zoom_nav"), Sense::click_and_drag());
        if let Some(pos) = nav_resp.interact_pointer_pos().filter(|_| nav_resp.clicked() || nav_resp.dragged()) {
            let fx = ((pos.x - nav.left()) / nav.width()).clamp(0.0, 1.0);
            let fy = ((pos.y - nav.top()) / nav.height()).clamp(0.0, 1.0);
            self.edit.pan = egui::vec2((0.5 - fx) * size.x, (0.5 - fy) * size.y);
        }

        if let Some(c) = self.meta(id).map(|m| m.caption.clone()).filter(|c| !c.is_empty()) {
            p.text(canvas.center_bottom() - egui::vec2(0.0, 10.0), egui::Align2::CENTER_CENTER, c, egui::FontId::proportional(13.0), Color32::from_rgb(240, 240, 240));
        }
    }

    fn edit_canvas(&mut self, ui: &mut egui::Ui) {
        let id = self.edit.id;
        egui::CentralPanel::default().frame(egui::Frame::new().fill(t().edit_bg)).show(ui, |ui| {
            let area = ui.available_rect_before_wrap().shrink(18.0);
            // Tools that draw on the picture (crop, redeye, straighten) always use the single view.
            if self.edit.compare != CompareMode::Single && self.edit.tool == Tool::None {
                self.compare_canvas(ui);
                return;
            }
            let extra: Vec<EditOp> = if self.edit.tool == Tool::Straighten && self.edit.angle.abs() > 0.01 {
                vec![EditOp::Straighten(self.edit.angle)]
            } else {
                vec![]
            };
            let tex = self.big_texture(id, &extra).or_else(|| self.thumbs.get(&id).cloned());
            let Some(tex) = tex else {
                if !self.thumbs.contains_key(&id) {
                    self.request_thumb(id);
                }
                ui.put(Rect::from_center_size(area.center(), Vec2::splat(40.0)), egui::Spinner::new().size(32.0));
                return;
            };
            let canvas = ui.available_rect_before_wrap();
            let r_prev = if self.big.contains_key(&id) { self.real_scale(id) } else { None };
            let ppp = ui.ctx().pixels_per_point();
            if let Some(r) = r_prev {
                self.edit.fit_zoom = fit_rect(area, tex.size()).width() * ppp / (tex.size()[0] as f32 * r);
            }
            // zoomed in or out (tools that draw boxes on the picture always work on the fitted view)
            if let (Some(z), Some(r), Tool::None) = (self.edit.zoom, r_prev, self.edit.tool) {
                self.zoomed_canvas(ui, canvas, id, tex, r, z);
                return;
            }
            let img_rect = fit_rect(area, tex.size());
            let resp = ui.allocate_rect(canvas, Sense::click_and_drag());
            if self.edit.tool == Tool::None && r_prev.is_some() && resp.hovered() {
                let s = ui.input(|i| i.smooth_scroll_delta.y);
                if s > 0.0 {
                    // the wheel zooms in from "fit", around the mouse pointer
                    let fit = self.edit.fit_zoom;
                    let nz = (fit * (s * 0.0025).exp()).min(ZOOM_MAX.max(fit));
                    self.edit.zoom_anim = None;
                    if let Some(pos) = resp.hover_pos() {
                        let rel = pos - canvas.center();
                        self.edit.pan = rel - rel * (nz / fit);
                    }
                    self.edit.zoom = Some(nz);
                }
            }
            let p = ui.painter();
            p.rect_filled(img_rect.translate(egui::vec2(3.0, 3.0)), 0, Color32::from_black_alpha(90));
            p.image(tex.id(), img_rect, Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)), Color32::WHITE);
            if !self.big.contains_key(&id) {
                p.text(img_rect.center_top() + egui::vec2(0.0, 14.0), egui::Align2::CENTER_CENTER, "Loading full picture…", egui::FontId::proportional(13.0), Color32::WHITE);
            }

            match self.edit.tool {
                Tool::Straighten => {
                    let stroke = Stroke::new(1.0, Color32::from_white_alpha(110));
                    for i in 1..8 {
                        let x = img_rect.left() + img_rect.width() * i as f32 / 8.0;
                        p.line_segment([Pos2::new(x, img_rect.top()), Pos2::new(x, img_rect.bottom())], stroke);
                        let y = img_rect.top() + img_rect.height() * i as f32 / 8.0;
                        p.line_segment([Pos2::new(img_rect.left(), y), Pos2::new(img_rect.right(), y)], stroke);
                    }
                }
                Tool::Crop | Tool::Redeye => {
                    let to_norm = |pos: Pos2| -> Pos2 {
                        Pos2::new(
                            ((pos.x - img_rect.left()) / img_rect.width()).clamp(0.0, 1.0),
                            ((pos.y - img_rect.top()) / img_rect.height()).clamp(0.0, 1.0),
                        )
                    };
                    if resp.drag_started() {
                        self.edit.drag_start = resp.interact_pointer_pos().map(to_norm);
                    }
                    if resp.dragged()
                        && let (Some(a), Some(pos)) = (self.edit.drag_start, resp.interact_pointer_pos()) {
                            let mut b = to_norm(pos);
                            if self.edit.tool == Tool::Crop
                                && let Some(ratio) = self.edit.aspect.ratio() {
                                    // keep the chosen shape, in either orientation of the picture
                                    let ratio = if img_rect.width() >= img_rect.height() { ratio } else { 1.0 / ratio };
                                    let wpx = (b.x - a.x) * img_rect.width();
                                    let hpx = wpx.abs() / ratio * (b.y - a.y).signum();
                                    b.y = (a.y + hpx / img_rect.height()).clamp(0.0, 1.0);
                                }
                            if (b.x - a.x).abs() > 0.01 && (b.y - a.y).abs() > 0.01 {
                                self.edit.sel = Some([a.x.min(b.x), a.y.min(b.y), a.x.max(b.x), a.y.max(b.y)]);
                            }
                        }
                    if let Some(s) = self.edit.sel {
                        let r = Rect::from_min_max(
                            Pos2::new(img_rect.left() + s[0] * img_rect.width(), img_rect.top() + s[1] * img_rect.height()),
                            Pos2::new(img_rect.left() + s[2] * img_rect.width(), img_rect.top() + s[3] * img_rect.height()),
                        );
                        if self.edit.tool == Tool::Crop {
                            let dim = Color32::from_black_alpha(150);
                            p.rect_filled(Rect::from_min_max(img_rect.min, Pos2::new(img_rect.right(), r.top())), 0, dim);
                            p.rect_filled(Rect::from_min_max(Pos2::new(img_rect.left(), r.bottom()), img_rect.max), 0, dim);
                            p.rect_filled(Rect::from_min_max(Pos2::new(img_rect.left(), r.top()), Pos2::new(r.left(), r.bottom())), 0, dim);
                            p.rect_filled(Rect::from_min_max(Pos2::new(r.right(), r.top()), Pos2::new(img_rect.right(), r.bottom())), 0, dim);
                            p.rect_stroke(r, 0, Stroke::new(1.5, Color32::WHITE), StrokeKind::Outside);
                        } else {
                            p.rect_stroke(r, 0, Stroke::new(2.0, Color32::from_rgb(255, 80, 80)), StrokeKind::Outside);
                        }
                    } else {
                        p.text(
                            img_rect.center_bottom() - egui::vec2(0.0, 16.0),
                            egui::Align2::CENTER_CENTER,
                            "Drag on the picture to draw a box",
                            egui::FontId::proportional(14.0),
                            Color32::WHITE,
                        );
                    }
                }
                Tool::PickNeutral => {
                    if resp.hovered() {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::Crosshair);
                    }
                    if resp.clicked()
                        && let Some(pos) = resp.interact_pointer_pos()
                        && img_rect.contains(pos)
                    {
                        let n = Pos2::new((pos.x - img_rect.left()) / img_rect.width(), (pos.y - img_rect.top()) / img_rect.height());
                        self.pick_neutral(id, n);
                        self.edit.tool = Tool::None;
                    }
                    p.text(
                        img_rect.center_bottom() - egui::vec2(0.0, 16.0),
                        egui::Align2::CENTER_CENTER,
                        "Click something that should be gray or white",
                        egui::FontId::proportional(14.0),
                        Color32::WHITE,
                    );
                }
                Tool::None => {
                    if resp.double_clicked() {
                        self.edit_go(PendingLeave::Library);
                    }
                }
            }
            if let Some(c) = self.meta(id).map(|m| m.caption.clone()).filter(|c| !c.is_empty()) {
                p.text(
                    Pos2::new(img_rect.center().x, (img_rect.bottom() + 9.0).min(ui.max_rect().bottom() - 9.0)),
                    egui::Align2::CENTER_CENTER,
                    c,
                    egui::FontId::proportional(13.0),
                    Color32::from_rgb(230, 230, 230),
                );
            }
        });
    }

    // ------------------------------------------------------------------ slideshow

    pub fn slideshow_ui(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        let now = ctx.input(|i| i.time);
        let settings = self.store.slideshow.clone();
        let step = settings.step_secs() as f64;
        let Some(s) = self.slide.as_mut() else {
            self.view = View::Library;
            return;
        };
        let len = s.list.len();
        if len == 0 {
            self.stop_slideshow();
            return;
        }
        if ctx.input(|i| i.pointer.delta() != Vec2::ZERO) {
            s.last_move = now;
        }
        // keys
        let (mut stop, mut toggle, mut go) = (false, false, 0i32);
        if !ctx.memory(|m| m.focused().is_some()) {
            ctx.input(|i| {
                stop = i.key_pressed(egui::Key::Escape);
                toggle = i.key_pressed(egui::Key::Space);
                if i.key_pressed(egui::Key::ArrowRight) {
                    go = 1;
                }
                if i.key_pressed(egui::Key::ArrowLeft) {
                    go = -1;
                }
            });
        }
        if s.playing && s.paused_at.is_none() && now - s.started >= step {
            go = 1;
        }
        // draw
        let clock = s.paused_at.unwrap_or(now);
        let (id, pos, started, prev) = (s.list[s.pos % len], s.pos, s.started, s.prev);
        let next = s.list[(s.pos + 1) % len];
        let (playing, last_move) = (s.playing, s.last_move);
        self.request_big(next, false);
        ctx.request_repaint();
        let mut clicked: Option<SlideCmd> = None;
        egui::CentralPanel::default().frame(egui::Frame::new().fill(Color32::BLACK)).show(ui, |ui| {
            let full = ui.max_rect();
            let since = (clock - started) as f32;
            let alpha = crate::slides::fade_in(since, &settings);
            if let Some((pid, pindex, pstarted)) = prev
                && alpha < 1.0
            {
                self.paint_slide(ui, full, pid, pindex, (clock - pstarted) as f32, 1.0 - alpha, &settings);
            }
            self.paint_slide(ui, full, id, pos, since, alpha, &settings);
            if let Some(c) = self.meta(id).map(|m| m.caption.clone()).filter(|c| !c.is_empty()) {
                let shade = Color32::from_white_alpha((alpha * 255.0) as u8);
                ui.painter().text(full.center_bottom() - egui::vec2(0.0, 80.0), egui::Align2::CENTER_CENTER, c, egui::FontId::proportional(20.0), shade);
            }
            if now - last_move < 3.0 || !playing {
                clicked = self.slideshow_controls(ui, full, pos, len, playing);
            }
        });
        match clicked {
            Some(SlideCmd::Prev) => go = -1,
            Some(SlideCmd::Next) => go = 1,
            Some(SlideCmd::PlayPause) => toggle = true,
            Some(SlideCmd::Stop) => stop = true,
            None => {}
        }
        if stop {
            self.stop_slideshow();
            return;
        }
        let Some(s) = self.slide.as_mut() else { return };
        if toggle {
            s.playing = !s.playing;
            if s.playing {
                // resume: the clocks skip the time spent paused
                if let Some(p) = s.paused_at.take() {
                    let d = now - p;
                    s.started += d;
                    if let Some(prev) = s.prev.as_mut() {
                        prev.2 += d;
                    }
                }
            } else {
                s.paused_at = Some(now);
            }
            if let Some(m) = &s.music {
                m.set_playing(s.playing);
            }
        }
        if go != 0 {
            let cur = s.list[s.pos % len];
            s.prev = Some((cur, s.pos, s.started));
            s.pos = (s.pos as i64 + go as i64).rem_euclid(len as i64) as usize;
            s.started = s.paused_at.unwrap_or(now);
        }
    }

    /// One slideshow picture, faded by `alpha`, with the pan-and-zoom motion at `since` seconds.
    #[allow(clippy::too_many_arguments)]
    fn paint_slide(&mut self, ui: &mut egui::Ui, full: Rect, id: usize, index: usize, since: f32, alpha: f32, settings: &crate::slides::SlideSettings) {
        let tex = self.big_texture(id, &[]).or_else(|| self.thumbs.get(&id).cloned());
        let Some(tex) = tex else {
            self.request_thumb(id);
            return;
        };
        let r = fit_rect(full.shrink(10.0), tex.size());
        let uv = if settings.motion {
            let [u0, v0, u1, v1] = crate::slides::ken_burns_uv(index, since / settings.step_secs().max(0.1));
            Rect::from_min_max(Pos2::new(u0, v0), Pos2::new(u1, v1))
        } else {
            Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0))
        };
        ui.painter().image(tex.id(), r, uv, Color32::from_white_alpha((alpha.clamp(0.0, 1.0) * 255.0) as u8));
    }

    fn slideshow_controls(&mut self, ui: &mut egui::Ui, full: Rect, pos: usize, len: usize, playing: bool) -> Option<SlideCmd> {
        let mut cmd = None;
        let bar = Rect::from_center_size(full.center_bottom() - egui::vec2(0.0, 40.0), egui::vec2(700.0, 50.0));
        ui.painter().rect_filled(bar, 10, Color32::from_rgba_unmultiplied(30, 30, 30, 210));
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(bar.shrink(8.0)).layout(egui::Layout::left_to_right(egui::Align::Center)));
        child.visuals_mut().override_text_color = Some(Color32::WHITE);
        child.visuals_mut().widgets.inactive.weak_bg_fill = Color32::from_rgb(60, 60, 64);
        child.visuals_mut().widgets.hovered.weak_bg_fill = Color32::from_rgb(80, 80, 90);
        let big = |t: &str| egui::Button::new(RichText::new(t).size(18.0).color(Color32::WHITE));
        if child.add(big("⏮")).on_hover_text("Previous (←)").clicked() {
            cmd = Some(SlideCmd::Prev);
        }
        if child.add(big(if playing { "⏸" } else { "▶" })).on_hover_text("Play / pause (Space)").clicked() {
            cmd = Some(SlideCmd::PlayPause);
        }
        if child.add(big("⏭")).on_hover_text("Next (→)").clicked() {
            cmd = Some(SlideCmd::Next);
        }
        child.label(format!("{} / {}", pos + 1, len));
        child.separator();
        let st = &mut self.store.slideshow;
        let mut changed = false;
        child.spacing_mut().slider_width = 90.0;
        changed |= child.add(egui::Slider::new(&mut st.interval, 2.0..=15.0).suffix(" s").max_decimals(0)).on_hover_text("Seconds per picture").changed();
        changed |= child.toggle_value(&mut st.fade, "Fade").on_hover_text("Crossfade between pictures").changed();
        changed |= child.toggle_value(&mut st.motion, "Motion").on_hover_text("Slow pan and zoom").changed();
        let music_label = if st.music.is_some() { "♪ Music" } else { "♪ Choose music…" };
        let mut music_changed = false;
        if st.music.is_some() {
            music_changed |= child.toggle_value(&mut st.music_on, music_label).on_hover_text(st.music.clone().unwrap_or_default()).changed();
            if child.small_button("…").on_hover_text("Choose another music file").clicked() {
                music_changed |= pick_music(st);
            }
        } else if child.button(music_label).clicked() {
            music_changed |= pick_music(st);
        }
        if child.add(big("✖")).on_hover_text("Exit (Esc)").clicked() {
            cmd = Some(SlideCmd::Stop);
        }
        if changed || music_changed {
            self.dirty = true;
        }
        if music_changed {
            let music = self.start_music();
            if let Some(s) = self.slide.as_mut() {
                s.music = music;
                if let Some(m) = &s.music {
                    m.set_playing(s.playing);
                }
            }
        }
        cmd
    }
}

#[cfg(test)]
mod zoom_tests {
    use super::*;
    #[test]
    fn slider_curve() {
        let fit = 0.25;
        assert!((pos_to_zoom(0.0, fit) - fit).abs() < 1e-6);
        assert!((pos_to_zoom(0.5, fit) - 1.0).abs() < 1e-6);
        assert!((pos_to_zoom(1.0, fit) - 4.0).abs() < 1e-6);
        for p in [0.1f32, 0.3, 0.6, 0.9] {
            assert!((zoom_to_pos(pos_to_zoom(p, fit), fit) - p).abs() < 1e-4);
        }
    }
}
