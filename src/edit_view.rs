//! The Edit view (one photo with fix-up tools) and the full-screen slideshow.

use crate::icons;
use crate::app::*;
use crate::develop::{self, Adjust};
use crate::imaging::{self, EditOp};
use crate::decor::{Decor, TextOverlay};
use crate::looks::Look;
use crate::library_view::paint_thumb;
use eframe::egui::{self, Color32, Pos2, Rect, RichText, Sense, Stroke, StrokeKind, TextureOptions, Vec2};

pub(crate) fn fit_rect(area: Rect, size: [usize; 2]) -> Rect {
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
enum AutoFix {
    Lucky,
    Exposure,
    WhiteBalance,
    Straighten,
}

/// The alignment grid: square cells (about fifteen across the long side), so straight horizontal
/// and vertical lines in the picture are easy to check against it.
fn paint_grid(p: &egui::Painter, r: Rect) {
    let cell = (r.width().max(r.height()) / 15.0).max(12.0);
    let stroke = Stroke::new(1.0, Color32::from_white_alpha(120));
    let shadow = Stroke::new(1.0, Color32::from_black_alpha(60));
    let mut x = r.center().x - (r.width() / 2.0 / cell).floor() * cell;
    while x <= r.right() + 0.5 {
        p.line_segment([Pos2::new(x + 1.0, r.top()), Pos2::new(x + 1.0, r.bottom())], shadow);
        p.line_segment([Pos2::new(x, r.top()), Pos2::new(x, r.bottom())], stroke);
        x += cell;
    }
    let mut y = r.center().y - (r.height() / 2.0 / cell).floor() * cell;
    while y <= r.bottom() + 0.5 {
        p.line_segment([Pos2::new(r.left(), y + 1.0), Pos2::new(r.right(), y + 1.0)], shadow);
        p.line_segment([Pos2::new(r.left(), y), Pos2::new(r.right(), y)], stroke);
        y += cell;
    }
}

/// The largest box of `ratio` (width / height, in screen pixels) inside a `w` × `h` picture,
/// centred, as fractions of the picture.
fn largest_box(ratio: f32, w: f32, h: f32) -> [f32; 4] {
    let (bw, bh) = if w / h.max(1.0) > ratio { (h * ratio, h) } else { (w, w / ratio.max(0.01)) };
    let (fx, fy) = ((bw / w.max(1.0)).min(1.0), (bh / h.max(1.0)).min(1.0));
    [(1.0 - fx) / 2.0, (1.0 - fy) / 2.0, (1.0 + fx) / 2.0, (1.0 + fy) / 2.0]
}

/// A slider with its name before it; a double-click on the name sets it back to zero.
fn labelled_slider(ui: &mut egui::Ui, label: &str, v: &mut f32, range: std::ops::RangeInclusive<f32>, decimals: usize, suffix: &str, tip: &str) -> egui::Response {
    ui.horizontal(|ui| {
        let l = ui.add_sized([74.0, 18.0], egui::Label::new(RichText::new(label).size(12.5)).sense(Sense::click())).on_hover_text(format!("{tip}\nDouble-click to reset"));
        if l.double_clicked() {
            *v = 0.0;
        }
        ui.add(egui::Slider::new(v, range).max_decimals(decimals).suffix(suffix))
    })
    .inner
}

fn tool_button(ui: &mut egui::Ui, icon: &str, label: &str) -> egui::Response {
    ui.add_sized([112.0, 46.0], egui::Button::new(RichText::new(format!("{icon}\n{label}")).size(12.5)))
}

impl App {
    /// The picture's adjustment (the Advanced sliders): all neutral when it has none.
    fn current_adjust(&self, id: usize) -> Adjust {
        crate::steps::adjust_of(self.meta(id).map(|m| m.edits.as_slice()).unwrap_or(&[]))
    }

    /// Change the picture's adjustment where it stands in the edit list (or start one); all
    /// neutral removes it. One Undo step per slider movement (`gesture`).
    fn set_adjust(&mut self, id: usize, a: Adjust, gesture: &'static str) {
        self.change_edits(id, gesture, |e| crate::steps::set_adjust(e, a));
    }

    /// The picture as edited so far, without the current adjustment step (what the automatic fixes
    /// look at), from the full preview when it is loaded or else the thumbnail.
    fn analysis_image(&self, id: usize) -> Option<image::RgbaImage> {
        let base = self.big.get(&id).map(|b| &b.base).or_else(|| self.thumb_base.get(&id))?;
        let mut ops = self.edits(id);
        crate::steps::set_adjust(&mut ops, Adjust::default());
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
                        let total = (self.current_geometry(id).0 + angle).clamp(-45.0, 45.0);
                        self.set_geometry(id, EditOp::Straighten(total), "");
                        self.toast(format!("Straightened by {angle:+.1}°."));
                    }
                    Some(_) => self.toast("This picture already looks level."),
                    None => self.toast("No clear horizon or vertical lines found to straighten by."),
                }
                return;
            }
        }
        self.set_adjust(id, a, "");
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
        self.set_adjust(id, a, "");
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
        // set again below for as long as the Original button or its key is held
        self.edit.show_original = false;
        // a slider let go: the next change is a new Undo step
        if !ui.ctx().input(|i| i.pointer.any_down()) && !self.text_focused() {
            self.edit.gesture = None;
        }
        self.edit_keys(ui.ctx());
        let id = self.edit.id;
        self.tick_zoom_anim(ui.ctx());

        // ---- top bar
        egui::Panel::top("edit_top")
            .frame(egui::Frame::new().fill(t().toolbar_bg).inner_margin(egui::Margin::symmetric(8, 6)).stroke(Stroke::new(1.0, t().border)))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    if ui.add(egui::Button::new(RichText::new(format!("{}  Back to Library", icons::ARROW_BACK)).strong()).min_size(egui::vec2(0.0, 28.0))).clicked() {
                        self.edit_go(PendingLeave::Library);
                    }
                    ui.separator();
                    let p = &self.photos[id];
                    ui.label(RichText::new(&self.folders[p.folder].name).size(14.0).strong());
                    ui.label(RichText::new(format!("›  {}", p.name)).size(13.0).color(t().text_weak));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .selectable_label(self.store.info_open, format!("{} Info", icons::INFO))
                            .on_hover_text("Properties, location and tags (I)")
                            .clicked()
                        {
                            self.store.info_open = !self.store.info_open;
                            self.dirty = true;
                        }
                        ui.separator();
                        // one picture, two pictures, or before / after; with two halves shown, two more
                        // buttons: side by side or one above the other, and which half the edits go to
                        if self.edit.compare != CompareMode::Single {
                            let layout_icon = if self.edit.vertical { icons::SIDE_BY_SIDE } else { icons::ABOVE_BELOW };
                            if ui
                                .button(RichText::new(layout_icon).size(15.0))
                                .on_hover_text(if self.edit.vertical { "Show the pictures side by side" } else { "Show one picture above the other" })
                                .clicked()
                            {
                                self.edit.vertical = !self.edit.vertical;
                            }
                            if ui
                                .button(RichText::new(icons::SWAP).size(15.0))
                                .on_hover_text("Change the picture the focus is on (where your edits go)")
                                .clicked()
                            {
                                self.swap_2up_focus();
                            }
                        }
                        let two = self.edit.compare == CompareMode::TwoPhotos;
                        if ui
                            .add(egui::Button::new(RichText::new("Compare").size(12.5)).selected(two))
                            .on_hover_text("This picture next to another one, side by side")
                            .clicked()
                        {
                            self.set_compare(if two { CompareMode::Single } else { CompareMode::TwoPhotos });
                        }
                        // the picture without its edits, for as long as the button (or O) is held
                        let has_edits = self.meta(id).is_some_and(|m| !m.edits.is_empty());
                        let r = ui
                            .add_enabled(has_edits, egui::Button::new(RichText::new("Original").size(12.5)).sense(Sense::drag()).selected(self.edit.show_original))
                            .on_hover_text("Hold to see the picture without your edits (or hold O)");
                        if r.is_pointer_button_down_on() {
                            self.edit.show_original = true;
                        }
                        ui.separator();
                        if ui.button(format!("{} Slideshow", icons::PLAY)).clicked() {
                            let list = self.edit.list.clone();
                            self.start_slideshow(list, id);
                        }
                        if ui
                            .button(RichText::new(icons::FULLSCREEN).size(15.0))
                            .on_hover_text("Full screen (F11): from this picture on, one after the other, with music")
                            .clicked()
                        {
                            let list = self.edit.list.clone();
                            self.start_viewer(list, id);
                        }
                        let pos = self.edit.list.iter().position(|&x| x == id).unwrap_or(0);
                        if ui.add_enabled(pos + 1 < self.edit.list.len(), egui::Button::new(icons::CHEVRON_RIGHT)).on_hover_text("Next picture").clicked() {
                            self.step_photo(1);
                        }
                        ui.label(format!("{} of {}", pos + 1, self.edit.list.len()));
                        if ui.add_enabled(pos > 0, egui::Button::new(icons::CHEVRON_LEFT)).on_hover_text("Previous picture").clicked() {
                            self.step_photo(-1);
                        }
                    });
                });
            });

        self.edit_tools_panel(ui);
        self.edit_bottom_panel(ui);
        if self.store.info_open {
            self.info_panel(ui);
        }
        self.edit_canvas(ui);
    }

    fn edit_keys(&mut self, ctx: &egui::Context) {
        if self.dialog.is_some() || self.text_focused() {
            return;
        }
        let (left, right, esc, enter, undo, redo, original) = ctx.input(|i| {
            let z = i.modifiers.command && i.key_pressed(egui::Key::Z);
            (
                i.key_pressed(egui::Key::ArrowLeft),
                i.key_pressed(egui::Key::ArrowRight),
                i.key_pressed(egui::Key::Escape),
                i.key_pressed(egui::Key::Enter),
                z && !i.modifiers.shift,
                (z && i.modifiers.shift) || (i.modifiers.command && i.key_pressed(egui::Key::Y)),
                !i.modifiers.command && (i.key_down(egui::Key::O) || i.key_down(egui::Key::Backslash)),
            )
        });
        if original {
            self.edit.show_original = true;
        }
        if esc {
            if self.edit.tool != Tool::None {
                self.edit.tool = Tool::None;
                self.edit.sel = None;
            } else {
                self.edit_go(PendingLeave::Library);
            }
        }
        if enter {
            self.apply_tool();
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
        if ctx.input(|i| i.key_pressed(egui::Key::I) && !i.modifiers.command) {
            self.store.info_open = !self.store.info_open;
            self.dirty = true;
        }
        if ctx.input(|i| i.key_pressed(egui::Key::F11)) && self.edit.tool == Tool::None {
            let list = self.edit.list.clone();
            self.start_viewer(list, self.edit.id);
            return;
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
                        if ui.add_enabled(!self.edit.undo.is_empty(), egui::Button::new(format!("{} Undo", icons::UNDO))).on_hover_text("Take back the last change (Ctrl+Z)").clicked() {
                            self.undo();
                        }
                        if ui.add_enabled(!self.edit.redo.is_empty(), egui::Button::new(format!("{} Redo", icons::REDO))).on_hover_text("Ctrl+Shift+Z").clicked() {
                            self.redo();
                        }
                    });
                    ui.label(RichText::new("Edits on this picture:").size(12.0).weak());
                    if edits.is_empty() {
                        ui.label(RichText::new("(none — this is the original)").size(12.0).weak());
                    }
                    // a fixed height for up to six steps: inside a bottom panel, a scroll area left
                    // to size itself can get stuck at the panel's first, tiny height (one line)
                    let row = ui.fonts_mut(|f| f.row_height(&egui::FontId::proportional(12.0))) + ui.spacing().item_spacing.y;
                    let h = row * edits.len().clamp(0, 6) as f32;
                    let mut back_to = None;
                    egui::ScrollArea::vertical().id_salt("edit_steps").min_scrolled_height(h).max_height(h).show(ui, |ui| {
                        for (i, e) in edits.iter().enumerate() {
                            let t = RichText::new(format!("{}. {}", i + 1, e.label())).size(12.0);
                            let last = i + 1 == edits.len();
                            let r = ui.add(egui::Label::new(if last { t.strong() } else { t }).truncate().sense(Sense::click()));
                            if !last && r.on_hover_text(format!("{}\nClick to go back to this step (the later ones are removed; Undo brings them back)", e.label())).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                                back_to = Some(i + 1);
                            }
                        }
                    });
                    if let Some(n) = back_to {
                        self.change_edits(id, "", |e| e.truncate(n));
                    }
                    ui.horizontal(|ui| {
                        if !edits.is_empty() && ui.small_button("Undo all edits").clicked() {
                            self.change_edits(id, "", |e| e.clear());
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
                ui.label("Drag the box's edges or corners, drag inside it to move it, or draw a new box. Enter applies.");
                ui.add_space(4.0);
                ui.label(RichText::new("Shape").size(12.5).color(t().text_weak));
                let before = self.edit.aspect;
                egui::ComboBox::from_id_salt("crop_shape").selected_text(before.label()).width(ui.available_width() - 8.0).show_ui(ui, |ui| {
                    for a in CropAspect::ALL {
                        ui.selectable_value(&mut self.edit.aspect, a, a.label());
                    }
                });
                if self.edit.aspect != before {
                    self.edit.sel = None;
                    self.edit.crop_autofit = self.edit.aspect != CropAspect::Manual;
                }
                let shaped = !matches!(self.edit.aspect, CropAspect::Manual | CropAspect::Square);
                ui.add_space(4.0);
                ui.add_enabled_ui(shaped, |ui| {
                    ui.label(RichText::new("Orientation").size(12.5).color(t().text_weak));
                    ui.horizontal(|ui| {
                        let before = self.edit.crop_portrait;
                        ui.selectable_value(&mut self.edit.crop_portrait, false, "Landscape").on_hover_text("Wider than tall");
                        ui.selectable_value(&mut self.edit.crop_portrait, true, "Portrait").on_hover_text("Taller than wide");
                        if self.edit.crop_portrait != before {
                            self.edit.sel = None;
                            self.edit.crop_autofit = true;
                        }
                    });
                });
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    if ui.add_enabled(self.edit.sel.is_some(), egui::Button::new(format!("{} Apply", icons::CHECK))).clicked() {
                        self.apply_tool();
                    }
                    if ui.button("Cancel").clicked() {
                        self.edit.tool = Tool::None;
                        self.edit.sel = None;
                    }
                    let has_crop = self.meta(id).is_some_and(|m| crate::steps::crop_of(&m.edits).is_some());
                    if has_crop && ui.button("Remove crop").on_hover_text("Show the whole picture again").clicked() {
                        self.change_edits(id, "", |e| crate::steps::set_crop(e, None));
                        self.edit.tool = Tool::None;
                        self.edit.sel = None;
                    }
                });
            }
            Tool::Redeye => {
                ui.label(RichText::new("Redeye").strong().size(15.0));
                ui.label("Drag a box around the red eyes. Only red pupils inside the box are darkened.");
                ui.horizontal(|ui| {
                    if ui.add_enabled(self.edit.sel.is_some(), egui::Button::new(format!("{} Apply", icons::CHECK))).clicked() {
                        self.apply_tool();
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
                    if ui.button(format!("{} Apply", icons::CHECK)).clicked() {
                        self.apply_tool();
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
                    .add_sized([ui.available_width(), 34.0], egui::Button::new(RichText::new(format!("{}  I'm feeling lucky", icons::AUTO_FIX)).size(14.0)))
                    .on_hover_text("Exposure, white balance, black point and a little vibrance, all at once (then fine-tune them in Advanced)");
                if lucky.clicked() {
                    self.auto_fix(id, AutoFix::Lucky);
                }
                ui.add_space(4.0);
                egui::Grid::new("auto").num_columns(2).spacing([6.0, 6.0]).show(ui, |ui| {
                    if tool_button(ui, icons::WB_SUNNY, "Auto exposure").on_hover_text("Brighten or darken to a balanced exposure").clicked() {
                        self.auto_fix(id, AutoFix::Exposure);
                    }
                    if tool_button(ui, icons::PALETTE, "Auto colour").on_hover_text("Auto white balance: remove a colour cast").clicked() {
                        self.auto_fix(id, AutoFix::WhiteBalance);
                    }
                    ui.end_row();
                    if tool_button(ui, icons::CONTRAST, "Auto contrast").on_hover_text("Stretch the darkest and brightest tones").clicked() {
                        self.push_edit(id, EditOp::AutoContrast);
                    }
                    if tool_button(ui, icons::STRAIGHTEN, "Auto straighten").on_hover_text("Level a tilted horizon or verticals").clicked() {
                        self.auto_fix(id, AutoFix::Straighten);
                    }
                    ui.end_row();
                });
                ui.add_space(10.0);
                ui.label(RichText::new("Tools").size(12.0).weak());
                egui::Grid::new("basic").num_columns(2).spacing([6.0, 6.0]).show(ui, |ui| {
                    if tool_button(ui, icons::CROP, "Crop").clicked() {
                        self.start_crop();
                    }
                    if tool_button(ui, icons::STRAIGHTEN, "Straighten").clicked() {
                        self.edit.tool = Tool::Straighten;
                        self.edit.angle = 0.0;
                    }
                    ui.end_row();
                    if tool_button(ui, icons::REDEYE, "Redeye").clicked() {
                        self.edit.tool = Tool::Redeye;
                        self.edit.sel = None;
                    }
                    if tool_button(ui, icons::EYEDROPPER, "Pick neutral").on_hover_text("Click something gray to set the white balance").clicked() {
                        self.edit.tool = Tool::PickNeutral;
                    }
                    ui.end_row();
                    if tool_button(ui, icons::ROTATE_LEFT, "Rotate left").clicked() {
                        self.rotate(&[id], -1);
                    }
                    if tool_button(ui, icons::ROTATE_RIGHT, "Rotate right").clicked() {
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
        self.geometry_section(ui, id);
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
            if ui.button(format!("{} Pick", icons::EYEDROPPER)).on_hover_text("Click something gray in the picture").clicked() {
                self.edit.tool = Tool::PickNeutral;
            }
        });
        let slider = |ui: &mut egui::Ui, label: &str, v: &mut f32, range: std::ops::RangeInclusive<f32>, tip: &str| {
            labelled_slider(ui, label, v, range, if label == "Exposure" { 2 } else { 0 }, "", tip);
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
            self.set_adjust(id, a, "adjust");
        }

        ui.add_space(8.0);
        if ui.add_enabled(!a.is_neutral(), egui::Button::new("Reset all")).clicked() {
            self.set_adjust(id, Adjust::default(), "");
        }
    }

    /// Crop, straighten and perspective, at the top of the Advanced tab (the most used fixes). The
    /// sliders edit the picture's own straighten / perspective step (one of each) and show the
    /// alignment grid on the picture while they move.
    fn geometry_section(&mut self, ui: &mut egui::Ui, id: usize) {
        ui.add_space(6.0);
        ui.label(RichText::new("GEOMETRY").size(12.0).strong().color(t().text_weak));
        let has_crop = self.meta(id).is_some_and(|m| m.edits.iter().any(|e| matches!(e, EditOp::Crop(_))));
        ui.horizontal(|ui| {
            if ui.button(format!("{}  Crop…", icons::CROP)).on_hover_text("Choose what part of the picture to keep, or change the crop").clicked() {
                self.start_crop();
            }
            if has_crop && ui.button("Remove crop").on_hover_text("Show the whole picture again").clicked() {
                self.change_edits(id, "", |e| crate::steps::set_crop(e, None));
            }
        });
        let (mut angle, mut persp) = self.current_geometry(id);
        let (angle0, persp0) = (angle, persp);
        let mut in_use = false;
        let mut geo_slider = |ui: &mut egui::Ui, label: &str, v: &mut f32, range: std::ops::RangeInclusive<f32>, decimals: usize, suffix: &str, tip: &str| {
            let r = labelled_slider(ui, label, v, range, decimals, suffix, tip);
            in_use |= r.dragged() || r.changed() || r.has_focus();
        };
        geo_slider(ui, "Straighten", &mut angle, -45.0..=45.0, 1, "°", "Turn the picture a little to level the horizon (zooms in to hide the corners)");
        let (mut pv, mut ph) = (persp.0 * 100.0, persp.1 * 100.0);
        geo_slider(ui, "Vertical", &mut pv, -100.0..=100.0, 0, "", "Perspective: right widens the top, to stand up buildings that lean back when shot from below");
        geo_slider(ui, "Horizontal", &mut ph, -100.0..=100.0, 0, "", "Perspective: right makes the right side taller, for a wall seen at an angle");
        persp = (pv / 100.0, ph / 100.0);
        if angle != angle0 {
            self.set_geometry(id, EditOp::Straighten(angle), "straighten");
        }
        if persp != persp0 {
            self.set_geometry(id, EditOp::Perspective { vertical: persp.0, horizontal: persp.1 }, "perspective");
        }
        if in_use {
            // the grid stays a moment after the slider is let go
            let now = ui.input(|i| i.time);
            self.edit.grid_until = now + 1.2;
            ui.ctx().request_repaint_after(std::time::Duration::from_millis(1250));
        }
    }

    /// Start the crop tool. It shows the picture uncropped with the crop box it has (to change
    /// it), or without a box; a chosen shape then starts as the largest box of that shape.
    pub fn start_crop(&mut self) {
        let id = self.edit.id;
        if id >= self.photos.len() {
            return;
        }
        // quarter turns made after the crop go before it (the picture stays the same), so that
        // the tool shows the picture the way up it is
        crate::steps::turns_before_crop(&mut self.meta_mut(id).edits);
        let tall = self.thumbs.get(&id).map(|t| t.size()).is_some_and(|[w, h]| h > w);
        let current = self.meta(id).and_then(|m| crate::steps::crop_of(&m.edits));
        self.edit.tool = Tool::Crop;
        self.edit.sel = current;
        self.edit.crop_grip = None;
        // the thumbnail shows the picture as cropped so far: its shape is the box's
        self.edit.crop_portrait = tall;
        self.edit.crop_autofit = current.is_none() && self.edit.aspect != CropAspect::Manual;
    }

    /// Apply what the tool in use has ready (the Apply button, or Enter).
    pub fn apply_tool(&mut self) {
        let id = self.edit.id;
        match self.edit.tool {
            Tool::Crop => {
                if let Some(s) = self.edit.sel.take() {
                    // the whole picture is no crop at all
                    let whole = s[0] <= 0.002 && s[1] <= 0.002 && s[2] >= 0.998 && s[3] >= 0.998;
                    self.change_edits(id, "", |e| crate::steps::set_crop(e, (!whole).then_some(s)));
                    self.edit.tool = Tool::None;
                }
            }
            Tool::Redeye => {
                if let Some(s) = self.edit.sel.take() {
                    self.push_edit(id, EditOp::Redeye(s));
                    self.edit.tool = Tool::None;
                }
            }
            Tool::Straighten => {
                if self.edit.angle.abs() > 0.01 {
                    let angle = (self.current_geometry(id).0 + self.edit.angle).clamp(-45.0, 45.0);
                    self.set_geometry(id, EditOp::Straighten(angle), "");
                }
                self.edit.tool = Tool::None;
                self.edit.angle = 0.0;
            }
            Tool::PickNeutral | Tool::None => {}
        }
    }

    /// The picture's straighten angle and perspective (vertical, horizontal), or zero.
    fn current_geometry(&self, id: usize) -> (f32, (f32, f32)) {
        crate::steps::geometry_of(self.meta(id).map(|m| m.edits.as_slice()).unwrap_or(&[]))
    }

    /// Set the straighten or perspective step: changed where it stands (so moving the slider
    /// never stacks rotations), or added before the crop; back to zero removes it.
    fn set_geometry(&mut self, id: usize, op: EditOp, gesture: &'static str) {
        self.change_edits(id, gesture, |e| crate::steps::set_geometry(e, op));
    }

    /// The picture's preset look and its intensity.
    fn current_look(&self, id: usize) -> Option<(Look, f32)> {
        crate::steps::look_of(self.meta(id).map(|m| m.edits.as_slice()).unwrap_or(&[]))
    }

    /// Choose a look: it takes the place of the picture's look (so trying presets doesn't
    /// stack them). None removes it.
    fn set_look(&mut self, id: usize, look: Option<(Look, f32)>, gesture: &'static str) {
        self.change_edits(id, gesture, |e| crate::steps::set_look(e, look));
    }

    fn presets_tab(&mut self, ui: &mut egui::Ui) {
        let id = self.edit.id;
        let current = self.current_look(id);
        // previews start from the picture before the current look, so they show the alternatives
        let start = crate::steps::without_look(&self.edits(id));
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
                    self.set_look(id, Some((look, amount)), "look");
                }
            });
            if ui.button("Remove preset").clicked() {
                self.set_look(id, None, "");
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
            self.set_look(id, Some((l, amount)), "");
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
        self.change_edits(id, "decor", |e| match e.last_mut() {
            Some(EditOp::Decor(cur)) if cur.same_kind(&d) => *cur = d,
            _ => e.push(EditOp::Decor(d)),
        });
    }

    fn effects_tab(&mut self, ui: &mut egui::Ui) {
        let id = self.edit.id;
        ui.label(RichText::new("Add on or around the picture").size(12.0).weak());
        let white = [255, 255, 255];
        let adds: [(&str, &str, Decor); 6] = [
            (icons::VIGNETTE, "Vignette", Decor::Vignette { amount: 0.5, size: 0.5 }),
            (icons::BORDER, "Border", Decor::Border { width: 0.03, bottom: 0.0, color: white }),
            (icons::FRAME, "Instant frame", Decor::Border { width: 0.05, bottom: 0.15, color: [250, 248, 242] }),
            (icons::SHADOW, "Drop shadow", Decor::Shadow { size: 0.04, opacity: 0.55, background: white }),
            (icons::ROUNDED, "Rounded corners", Decor::Rounded { radius: 0.06, background: white }),
            (icons::TEXT, "Text", Decor::Text(TextOverlay { text: "Your text".into(), ..Default::default() })),
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
                    if ui.add_sized([34.0, 28.0], egui::Button::new(RichText::new(icons::ROTATE_LEFT).size(17.0))).on_hover_text("Rotate left").clicked() {
                        self.rotate(&[id], -1);
                    }
                    if ui.add_sized([34.0, 28.0], egui::Button::new(RichText::new(icons::ROTATE_RIGHT).size(17.0))).on_hover_text("Rotate right").clicked() {
                        self.rotate(&[id], 1);
                    }
                    ui.separator();
                    let mut caption = self.meta(id).map(|m| m.caption.clone()).unwrap_or_default();
                    let r = ui.add(egui::TextEdit::singleline(&mut caption).hint_text("Make a caption!").desired_width((ui.available_width() - 560.0).max(120.0)));
                    if r.changed() {
                        self.meta_mut(id).caption = caption;
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button(format!("{} Export", icons::EXPORT)).clicked() {
                            self.open_export(vec![id]);
                        }
                        if crate::print::available() && ui.button(format!("{} Print", icons::PRINT)).on_hover_text("Print this picture (Ctrl+P)").clicked() {
                            self.open_print();
                        }
                        self.album_menu(ui, format!("{} Album {}", icons::ALBUM, icons::EXPAND_MORE), &[id]);
                        ui.separator();
                        // Zoom: Fit, 1:1 and the zoom slider:
                        // the slider runs from Fit (left) through 1:1 (middle) to 400 % (right).
                        let fit = self.edit.fit_zoom;
                        let z_now = self.edit.zoom.unwrap_or(fit);
                        let mut p = if self.edit.zoom.is_some() { zoom_to_pos(z_now, fit) } else { 0.0 };
                        ui.label(RichText::new(format!("{:.0} %", z_now * 100.0)).size(12.0)).on_hover_text("Zoom (100 % = the photo's real pixels)");
                        ui.spacing_mut().slider_width = 120.0;
                        let s = ui.add(egui::Slider::new(&mut p, 0.0..=1.0).show_value(false));
                        if s.on_hover_text("Zoom: left = fit, middle = 1:1, right = 400 % (mouse wheel works too; drag the picture to zoom to 100 % and move it)").changed() {
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
                            self.edit.zoom_anim = Some(ZoomAnim { from: z_now, to: Some(1.0), t0: now, pan_from: self.edit.pan, focus: None });
                        }
                        if ui
                            .add(egui::Button::new(RichText::new("Fit").size(12.5)).selected(self.edit.zoom.is_none()))
                            .on_hover_text("Fit the whole picture in the window (double-click a zoomed picture does the same)")
                            .clicked()
                            && self.edit.zoom.is_some()
                        {
                            self.edit.zoom_anim = Some(ZoomAnim { from: z_now, to: None, t0: now, pan_from: self.edit.pan, focus: None });
                        }
                    });
                });
                // people chips (tags are in the info panel)
                ui.horizontal_wrapped(|ui| {
                    let people = self.meta(id).map(|m| m.people.clone()).unwrap_or_default();
                    ui.label(RichText::new("People:").size(12.0));
                    for t in &people {
                        if ui.small_button(format!("{} {t} {}", icons::PERSON, icons::CLOSE)).on_hover_text("Remove person").clicked() {
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
                // the list is lent out while the strip is drawn (no copy of it per frame)
                let list = std::mem::take(&mut self.edit.list);
                let mut go = None;
                egui::ScrollArea::horizontal().id_salt("filmstrip").max_height(62.0).show(ui, |ui| {
                    ui.horizontal(|ui| {
                        // cells all have the same width: only the ones in view are made, with
                        // empty space for the others
                        let size = egui::vec2(60.0, 54.0);
                        let pitch = size.x + ui.spacing().item_spacing.x;
                        let (left, top) = (ui.cursor().left(), ui.cursor().top());
                        let clip = ui.clip_rect();
                        let first = (((clip.left() - left) / pitch).floor().max(0.0) as usize).min(list.len());
                        let last = (((clip.right() - left) / pitch).ceil().max(0.0) as usize).min(list.len());
                        // follow the shown picture: centre its thumbnail whenever the picture changes,
                        // even when it is scrolled out of sight
                        if self.edit.strip_shown != Some(id) {
                            self.edit.strip_shown = Some(id);
                            if let Some(pos) = list.iter().position(|&x| x == id) {
                                let cell = Rect::from_min_size(Pos2::new(left + pos as f32 * pitch, top), size);
                                ui.scroll_to_rect(cell, Some(egui::Align::Center));
                            }
                        }
                        ui.add_space(first as f32 * pitch);
                        for &pid in list.get(first..last).unwrap_or(&[]) {
                            let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
                            self.want_thumb(pid);
                            let p = ui.painter();
                            if pid == id {
                                p.rect_filled(rect, 3, t().accent);
                            } else if resp.hovered() {
                                p.rect_filled(rect, 3, t().hover_wash);
                            }
                            paint_thumb(p, rect.shrink(4.0), self.thumbs.get(&pid), false);
                            if resp.clicked() {
                                go = Some(pid);
                            }
                        }
                        ui.add_space((list.len() - last) as f32 * pitch);
                    });
                });
                self.edit.list = list;
                if let Some(pid) = go
                    && pid != id {
                        self.edit_go(PendingLeave::Photo(pid));
                    }
            });
    }

    /// "A|A'" (before / after) and "A|B" (two pictures) views: two halves side by side, the edited
    /// picture marked "Selected". Clicking the other picture in A|B moves the focus to it.
    fn compare_canvas(&mut self, ui: &mut egui::Ui) {
        let id = self.edit.id;
        let full = ui.available_rect_before_wrap();
        let gap = 4.0;
        // first = left (or top), second = right (or bottom), by `edit.vertical`
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
        let (b_tex, b_label) = match self.compare_partner() {
            Some(o) => {
                self.want_thumb(o);
                let t = self.big_texture(o, &[]).or_else(|| self.thumbs.get(&o).cloned());
                (t, self.photos[o].name.clone())
            }
            None => (None, "No other picture in this folder".to_string()),
        };
        let a_label = "Selected";
        // same zoom and position on both sides, so details line up
        let ppp = ui.ctx().pixels_per_point();
        let r_a = if self.big.contains_key(&id) { self.real_scale(id) } else { None };
        let r_b = self.compare_partner().filter(|o| self.big.contains_key(o)).and_then(|o| self.real_scale(o));
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
        let tip = "Click to edit this picture instead";
        let has_other = self.compare_partner().is_some();
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
        if let Some(f) = a.focus {
            // zooming in from "fit" (no pan) around `f`, plus what the mouse moved meanwhile
            let z = a.from + (target - a.from) * e;
            self.edit.zoom = Some(z);
            self.edit.pan = f - f * (z / a.from.max(1e-6)) + a.pan_from;
            if t >= 1.0 {
                self.edit.zoom_anim = None;
            } else {
                ctx.request_repaint();
            }
            return;
        }
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
        self.edit.zoom_anim = Some(ZoomAnim { from, to, t0, pan_from: self.edit.pan, focus: None });
    }

    /// Mouse wheel and dragging over a zoomed view: zoom around the pointer, move the picture.
    fn zoom_input(&mut self, ui: &egui::Ui, resp: &egui::Response, view: Rect) {
        let Some(z) = self.edit.zoom else { return };
        if resp.dragged() {
            match &mut self.edit.zoom_anim {
                // zooming in around the grabbed point: the animation adds the movement
                Some(a) if a.focus.is_some() => a.pan_from += resp.drag_delta(),
                _ => self.edit.pan += resp.drag_delta(),
            }
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
        let magnified = r_prev * z / ppp > 1.05 && !self.edit.show_original;
        if magnified {
            self.want_zoom_render(id, &ops);
        }
        let (tex, r) = match &self.edit.zoom_tex {
            Some((zid, zops, t, r)) if magnified && *zid == id && *zops == ops => (t.clone(), *r),
            _ => (preview.clone(), r_prev),
        };
        let resp = ui.interact(canvas, ui.scope_id().with("edit_canvas"), Sense::click_and_drag());
        self.zoom_input(ui, &resp, canvas);
        let z = self.edit.zoom.unwrap_or(z);
        let size = egui::vec2(tex.size()[0] as f32, tex.size()[1] as f32) * (r * z / ppp);
        self.edit.pan = clamp_pan(self.edit.pan, size, canvas.size());
        let img = Rect::from_center_size(canvas.center() + self.edit.pan, size);
        let p = ui.painter_at(canvas);
        p.image(tex.id(), img, Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)), Color32::WHITE);
        if ui.input(|i| i.time) < self.edit.grid_until {
            paint_grid(&p, img.intersect(canvas));
        }
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
            let original = self.edit.show_original && self.edit.tool == Tool::None;
            let tex = if self.edit.tool == Tool::Crop {
                // the picture uncropped, to draw or change the box on (never the thumbnail,
                // which is cropped already)
                let view = crate::steps::crop_view(&self.edits(id));
                self.big_ops_texture(id, &view)
            } else if original {
                self.big_version_texture(id, &[])
            } else {
                self.big_texture(id, &extra).or_else(|| self.thumbs.get(&id).cloned())
            };
            let Some(tex) = tex else {
                if self.big_failed.contains(&id) {
                    // neither a preview nor a thumbnail: say so instead of waiting forever
                    ui.painter().text(area.center(), egui::Align2::CENTER_CENTER, "This picture can't be read.", egui::FontId::proportional(15.0), Color32::WHITE);
                    return;
                }
                self.want_thumb(id);
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
            // same id as the zoomed view's, so that a drag started here carries on there
            let resp = ui.interact(canvas, ui.scope_id().with("edit_canvas"), Sense::click_and_drag());
            // click and drag on the picture: zoom to 100 % around that point and move it with the mouse
            if self.edit.tool == Tool::None
                && r_prev.is_some()
                && resp.drag_started()
                && let Some(pos) = resp.interact_pointer_pos().filter(|p| img_rect.contains(*p))
            {
                let fit = self.edit.fit_zoom;
                // a small picture already shown at 100 % or more is doubled instead
                let nz = if fit < 1.0 { 1.0 } else { (fit * 2.0).min(ZOOM_MAX.max(fit)) };
                let rel = pos - canvas.center();
                // starts at "fit" and grows smoothly, the grabbed point staying under the mouse
                let t0 = ui.input(|i| i.time);
                self.edit.zoom_anim = Some(ZoomAnim { from: fit, to: Some(nz), t0, pan_from: Vec2::ZERO, focus: Some(rel) });
                self.edit.pan = Vec2::ZERO;
                self.edit.zoom = Some(fit);
                ui.ctx().request_repaint();
            } else if self.edit.tool == Tool::None && r_prev.is_some() && resp.hover_pos().is_some_and(|p| img_rect.contains(p)) {
                ui.ctx().set_cursor_icon(egui::CursorIcon::ZoomIn);
            }
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
            if original {
                let font = egui::FontId::proportional(14.0);
                let galley = p.layout_no_wrap("Original".to_string(), font, Color32::WHITE);
                let pill = Rect::from_min_size(Pos2::new(img_rect.right() - galley.size().x - 34.0, img_rect.top() + 14.0), galley.size() + egui::vec2(20.0, 10.0));
                p.rect_filled(pill, 4, Color32::from_rgba_unmultiplied(45, 45, 45, 215));
                p.galley(pill.min + egui::vec2(10.0, 5.0), galley, Color32::WHITE);
            }
            if !self.big.contains_key(&id) {
                let note = if self.big_failed.contains(&id) { "The full picture can't be read." } else { "Loading full picture…" };
                p.text(img_rect.center_top() + egui::vec2(0.0, 14.0), egui::Align2::CENTER_CENTER, note, egui::FontId::proportional(13.0), Color32::WHITE);
            }
            if self.edit.tool == Tool::None && ui.input(|i| i.time) < self.edit.grid_until {
                paint_grid(p, img_rect);
            }

            match self.edit.tool {
                Tool::Straighten => paint_grid(p, img_rect),
                Tool::Crop | Tool::Redeye => {
                    use crate::steps::{self, Grip};
                    let raw = |pos: Pos2| Pos2::new((pos.x - img_rect.left()) / img_rect.width(), (pos.y - img_rect.top()) / img_rect.height());
                    let to_norm = |pos: Pos2| {
                        let p = raw(pos);
                        Pos2::new(p.x.clamp(0.0, 1.0), p.y.clamp(0.0, 1.0))
                    };
                    let crop = self.edit.tool == Tool::Crop;
                    // the crop box's shape (width / height on screen), None = free
                    let picture = img_rect.width() / img_rect.height().max(1.0);
                    let box_ratio = self.edit.aspect.ratio(picture).filter(|_| crop).map(|r| if self.edit.crop_portrait { 1.0 / r } else { r });
                    // the same shape in fractions of the picture's sides
                    let shape = box_ratio.map(|r| r / picture.max(0.001));
                    if crop && self.edit.crop_autofit {
                        self.edit.crop_autofit = false;
                        self.edit.sel = box_ratio.map(|r| largest_box(r, img_rect.width(), img_rect.height()));
                    }
                    // how near an edge of the box counts as taking hold of it: 10 points
                    let reach = (10.0 / img_rect.width().max(1.0), 10.0 / img_rect.height().max(1.0));
                    let grip_under = |sel: Option<[f32; 4]>, pos: Option<Pos2>| -> Option<Grip> {
                        let (s, p) = (sel.filter(|_| crop)?, raw(pos?));
                        steps::grip_at(s, (p.x, p.y), reach)
                    };
                    if resp.drag_started() {
                        let at = resp.interact_pointer_pos();
                        self.edit.drag_start = at.map(to_norm);
                        // an edge, a corner or the inside of the box: change it; elsewhere: a new box
                        self.edit.crop_grip = match (grip_under(self.edit.sel, at), self.edit.sel, at) {
                            (Some(g), Some(s), Some(at)) => Some((g, s, to_norm(at))),
                            _ => None,
                        };
                    }
                    if resp.dragged()
                        && let (Some(a), Some(pos)) = (self.edit.drag_start, resp.interact_pointer_pos())
                    {
                        let b = to_norm(pos);
                        if let Some((grip, start, at)) = self.edit.crop_grip {
                            self.edit.sel = Some(steps::drag_box(start, grip, (b.x - at.x, b.y - at.y), shape));
                        } else if let Some(r) = steps::drawn_box((a.x, a.y), (b.x, b.y), shape) {
                            self.edit.sel = Some(r);
                        }
                    }
                    if resp.drag_stopped() {
                        self.edit.crop_grip = None;
                    }
                    let shown_grip = self.edit.crop_grip.map(|(g, _, _)| g).or_else(|| grip_under(self.edit.sel, resp.hover_pos()));
                    match shown_grip {
                        Some(Grip::Inside) => ui.ctx().set_cursor_icon(egui::CursorIcon::Grab),
                        Some(Grip::Sides { left, top, right, bottom }) => ui.ctx().set_cursor_icon(match (left || right, top || bottom) {
                            (true, false) => egui::CursorIcon::ResizeHorizontal,
                            (false, true) => egui::CursorIcon::ResizeVertical,
                            _ if (left && top) || (right && bottom) => egui::CursorIcon::ResizeNwSe,
                            _ => egui::CursorIcon::ResizeNeSw,
                        }),
                        None if resp.hovered() => ui.ctx().set_cursor_icon(egui::CursorIcon::Crosshair),
                        None => {}
                    }
                    if let Some(s) = self.edit.sel {
                        let r = Rect::from_min_max(
                            Pos2::new(img_rect.left() + s[0] * img_rect.width(), img_rect.top() + s[1] * img_rect.height()),
                            Pos2::new(img_rect.left() + s[2] * img_rect.width(), img_rect.top() + s[3] * img_rect.height()),
                        );
                        if crop {
                            let dim = Color32::from_black_alpha(150);
                            p.rect_filled(Rect::from_min_max(img_rect.min, Pos2::new(img_rect.right(), r.top())), 0, dim);
                            p.rect_filled(Rect::from_min_max(Pos2::new(img_rect.left(), r.bottom()), img_rect.max), 0, dim);
                            p.rect_filled(Rect::from_min_max(Pos2::new(img_rect.left(), r.top()), Pos2::new(r.left(), r.bottom())), 0, dim);
                            p.rect_filled(Rect::from_min_max(Pos2::new(r.right(), r.top()), Pos2::new(img_rect.right(), r.bottom())), 0, dim);
                            p.rect_stroke(r, 0, Stroke::new(1.5, Color32::WHITE), StrokeKind::Outside);
                            // thirds, to place what matters
                            let thin = Stroke::new(1.0, Color32::from_white_alpha(90));
                            for k in [1.0 / 3.0, 2.0 / 3.0] {
                                let x = r.left() + r.width() * k;
                                let y = r.top() + r.height() * k;
                                p.line_segment([Pos2::new(x, r.top()), Pos2::new(x, r.bottom())], thin);
                                p.line_segment([Pos2::new(r.left(), y), Pos2::new(r.right(), y)], thin);
                            }
                            // handles: the corners and the middle of each edge
                            let c = r.center();
                            for h in [r.left_top(), r.right_top(), r.left_bottom(), r.right_bottom(), Pos2::new(c.x, r.top()), Pos2::new(c.x, r.bottom()), Pos2::new(r.left(), c.y), Pos2::new(r.right(), c.y)] {
                                let handle = Rect::from_center_size(h, Vec2::splat(9.0));
                                p.rect_filled(handle, 2, Color32::WHITE);
                                p.rect_stroke(handle, 2, Stroke::new(1.0, Color32::from_black_alpha(140)), StrokeKind::Outside);
                            }
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
                    // a double-click goes to 100 % (and back to Fit when zoomed in); Esc or
                    // Back to Library leave the editor
                    if resp.double_clicked() && r_prev.is_some() && self.edit.zoom_anim.is_none() {
                        let fit = self.edit.fit_zoom;
                        let to = if fit < 1.0 { 1.0 } else { (fit * 2.0).min(ZOOM_MAX.max(fit)) };
                        self.start_zoom_anim(ui.ctx(), Some(to));
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

#[cfg(test)]
mod geometry_tests {
    use super::largest_box;

    #[test]
    fn largest_centred_box() {
        // a square in a 300 × 200 picture: full height, centred
        let b = largest_box(1.0, 300.0, 200.0);
        assert!((b[1] - 0.0).abs() < 1e-6 && (b[3] - 1.0).abs() < 1e-6);
        assert!(((b[2] - b[0]) * 300.0 - 200.0).abs() < 1e-3);
        assert!(((b[0] + b[2]) / 2.0 - 0.5).abs() < 1e-6);
        // a wide box in a tall picture: full width
        let b = largest_box(16.0 / 9.0, 200.0, 300.0);
        assert!((b[0] - 0.0).abs() < 1e-6 && (b[2] - 1.0).abs() < 1e-6);
        assert!(((b[3] - b[1]) * 300.0 - 200.0 * 9.0 / 16.0).abs() < 1e-3);
        // the picture's own shape: all of it
        assert_eq!(largest_box(1.5, 300.0, 200.0), [0.0, 0.0, 1.0, 1.0]);
    }
}
