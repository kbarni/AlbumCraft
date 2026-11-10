//! The collage screen: styles and settings on the left, the page in the middle (drag pictures to
//! swap them, or to move them in a photo pile), saving and printing at the bottom. The same screen
//! makes contact sheets: pages of small pictures with a title and their file names.
//!
//! The layouts and the drawing of the saved pages are `albumcraft_core::collage`; the preview
//! here draws the same shapes with the thumbnails.

use crate::app::*;
use crate::collage::{self, BgKind, Border, Format, Gradient, Nudge, Page, Pic, Shape, ShapeKind, Style, P};
use crate::icons;
use crate::library::{self, Done, Job, BIG_DIM};
use crate::print::{self, CaptionMode, Paper, PrinterChoice};
use eframe::egui::{self, Color32, Pos2, Rect, RichText, Sense, Stroke, StrokeKind, TextureHandle, TextureOptions, Vec2};
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

pub struct CollageState {
    /// A contact sheet instead of a collage.
    pub sheet: bool,
    /// The pictures, in their order on the page.
    pub ids: Vec<usize>,
    pub from: View,
    /// Pictures moved or turned by hand in a photo pile, by picture.
    pub nudges: HashMap<usize, Nudge>,
    top_z: u32,
    pub selected: Option<usize>,
    drag: Option<Drag>,
    /// Contact sheet page shown.
    pub page: usize,
    pub title: String,
    pub subtitle: String,
    /// The background picture's file (a library picture or any file).
    pub bg_key: Option<String>,
    /// Its preview: sharp and soft-focus.
    bg_tex: Option<(String, TextureHandle, TextureHandle)>,
    printers: Vec<String>,
    printer: String,
    /// Pages sent so far while printing (usize::MAX = finished).
    job: Option<(Arc<AtomicUsize>, usize)>,
}

impl CollageState {
    /// After a scan: move to the pictures' new numbers (`to`), dropping the ones that are gone.
    /// False when none is left.
    pub fn keep_across_scan(&mut self, to: impl Fn(usize) -> Option<usize>) -> bool {
        self.ids = self.ids.iter().filter_map(|&id| to(id)).collect();
        self.nudges = std::mem::take(&mut self.nudges).into_iter().filter_map(|(id, n)| Some((to(id)?, n))).collect();
        self.selected = self.selected.and_then(&to);
        self.drag = None;
        self.page = 0;
        !self.ids.is_empty()
    }
}

#[derive(Clone, Copy)]
struct Drag {
    id: usize,
    /// Where the pointer took the picture, on the page.
    from: P,
    start: Nudge,
}

/// Corners of a box of `size` centred at `c`, turned by `rot` (top left, top right, bottom right,
/// bottom left).
fn corners(c: Pos2, size: Vec2, rot: f32) -> [Pos2; 4] {
    let (h, w) = (size.y / 2.0, size.x / 2.0);
    [[-w, -h], [w, -h], [w, h], [-w, h]].map(|p| {
        let r = collage::rotate(p, rot);
        Pos2::new(c.x + r[0], c.y + r[1])
    })
}

fn fill_quad(p: &egui::Painter, q: [Pos2; 4], color: Color32) {
    p.add(egui::Shape::convex_polygon(q.to_vec(), color, Stroke::NONE));
}

fn tex_quad(p: &egui::Painter, q: [Pos2; 4], tex: egui::TextureId, uv: Rect, tint: Color32) {
    let uvs = [uv.left_top(), uv.right_top(), uv.right_bottom(), uv.left_bottom()];
    let mut mesh = egui::Mesh::with_texture(tex);
    for i in 0..4 {
        mesh.vertices.push(egui::epaint::Vertex { pos: q[i], uv: uvs[i], color: tint });
    }
    mesh.indices = vec![0, 1, 2, 0, 2, 3];
    p.add(egui::Shape::mesh(mesh));
}

/// The part of a texture of shape `tex_aspect` that fills a box of shape `aspect`, from the middle.
fn cover_uv(tex_aspect: f32, aspect: f32) -> Rect {
    if tex_aspect > aspect {
        let f = aspect / tex_aspect;
        Rect::from_min_max(Pos2::new((1.0 - f) / 2.0, 0.0), Pos2::new((1.0 + f) / 2.0, 1.0))
    } else {
        let f = tex_aspect / aspect.max(1e-3);
        Rect::from_min_max(Pos2::new(0.0, (1.0 - f) / 2.0), Pos2::new(1.0, (1.0 + f) / 2.0))
    }
}

fn rgba(c: [u8; 4]) -> Color32 {
    Color32::from_rgba_unmultiplied(c[0], c[1], c[2], c[3])
}

/// The page on the screen: its top left corner and pixels per millimetre.
#[derive(Clone, Copy)]
struct View2 {
    origin: Pos2,
    s: f32,
}

impl View2 {
    fn at(&self, p: P) -> Pos2 {
        self.origin + egui::vec2(p[0] * self.s, p[1] * self.s)
    }
    fn page(&self, p: Pos2) -> P {
        [(p.x - self.origin.x) / self.s, (p.y - self.origin.y) / self.s]
    }
}

/// Draw the shapes that aren't pictures, and the pictures with `tex` (None: a grey box).
fn paint_shape(p: &egui::Painter, v: View2, shape: &Shape, tex: Option<&TextureHandle>, flat: Option<Color32>) {
    match shape {
        Shape::Photo(ph) => {
            let (oc, os) = ph.outer();
            let outer = corners(v.at(oc), egui::vec2(os[0] * v.s, os[1] * v.s), ph.rot);
            if ph.shadow {
                let m = os[0].min(os[1]);
                let (off, soft) = ((m * 0.012 + 0.35) * v.s, (m * 0.025 + 0.6) * v.s);
                for k in 0..4 {
                    let grow = soft * (k as f32 / 3.0 * 1.4 - 0.5);
                    let c = v.at(oc) + egui::vec2(off * 0.6, off);
                    fill_quad(p, corners(c, egui::vec2(os[0] * v.s + grow * 2.0, os[1] * v.s + grow * 2.0), ph.rot), Color32::from_black_alpha(22));
                }
            }
            if ph.frame != [0.0; 4] {
                fill_quad(p, outer, rgba(ph.frame_color));
            }
            let q = corners(v.at(ph.c), egui::vec2(ph.size[0] * v.s, ph.size[1] * v.s), ph.rot);
            let alpha = ph.alpha.clamp(0.0, 1.0);
            match (tex, flat) {
                (_, Some(c)) => fill_quad(p, q, c.gamma_multiply(alpha.max(0.5))),
                (Some(t), None) => {
                    let [tw, th] = t.size();
                    let uv = cover_uv(tw as f32 / th.max(1) as f32, ph.size[0] / ph.size[1].max(1e-3));
                    tex_quad(p, q, t.id(), uv, Color32::WHITE.gamma_multiply(alpha));
                }
                (None, None) => fill_quad(p, q, Color32::from_gray(200).gamma_multiply(alpha)),
            }
        }
        Shape::Rect { c, size, rot, color, .. } => fill_quad(p, corners(v.at(*c), egui::vec2(size[0] * v.s, size[1] * v.s), *rot), rgba(*color)),
        Shape::Line { pts, width, color } => {
            p.add(egui::Shape::line(pts.iter().map(|q| v.at(*q)).collect(), Stroke::new((width * v.s).max(1.0), rgba(*color))));
        }
        Shape::Text { c, text, size, rot, color } => {
            if flat.is_some() {
                // style icons: a line where the text goes
                let w = collage::text_width(text, *size) * v.s;
                fill_quad(p, corners(v.at(*c), egui::vec2(w, (size * v.s * 0.35).max(1.0)), *rot), Color32::from_gray(150));
                return;
            }
            let px = size * v.s;
            if px < 2.5 {
                return;
            }
            let galley = p.layout_no_wrap(text.clone(), egui::FontId::proportional(px), rgba(*color));
            let half = collage::rotate([galley.size().x / 2.0, galley.size().y / 2.0], *rot);
            let pos = v.at(*c) - egui::vec2(half[0], half[1]);
            p.add(egui::epaint::TextShape::new(pos, galley, rgba(*color)).with_angle(*rot));
        }
    }
}

/// A small drawing of a style, made from its real layout.
fn paint_style_icon(p: &egui::Painter, rect: Rect, style: Option<Style>) {
    let fake: Vec<Pic> = (0..9).map(|i| Pic { aspect: if i % 3 == 1 { 0.8 } else { 1.45 }, name: "IMG_0000.JPG".into(), ..Default::default() }).collect();
    let page = match style {
        Some(style) => {
            let s = collage::CollageSettings { style, border: style.natural_border(), shadow: false, format: Format::Cm10x15, landscape: true, gap: 0.6, seed: 3, ..Default::default() };
            collage::collage(&s, &fake[..if style == Style::Blend { 3 } else { 9 }], &[])
        }
        None => {
            let s = collage::SheetSettings { format: Format::Cm10x15, landscape: true, thumb_mm: 22.0, shadow: false, page_numbers: false, ..Default::default() };
            collage::contact_sheet(&s, &fake, "Title", "").into_iter().next().unwrap_or_default()
        }
    };
    let s = (rect.width() / page.size[0]).min(rect.height() / page.size[1]);
    let sheet = Rect::from_center_size(rect.center(), egui::vec2(page.size[0] * s, page.size[1] * s));
    p.rect_filled(sheet, 2, if style.is_some() { Color32::from_rgb(236, 230, 218) } else { Color32::WHITE });
    let v = View2 { origin: sheet.min, s };
    let palette = [Color32::from_rgb(86, 140, 200), Color32::from_rgb(214, 128, 84), Color32::from_rgb(98, 168, 112), Color32::from_rgb(200, 170, 70), Color32::from_rgb(150, 110, 180)];
    let clip = p.with_clip_rect(sheet);
    for sh in &page.shapes {
        let flat = if let Shape::Photo(ph) = sh { palette[ph.pic % palette.len()] } else { Color32::GRAY };
        paint_shape(&clip, v, sh, None, Some(flat));
    }
}

impl App {
    /// Open the collage screen (or the contact sheet) for the pictures in the tray.
    pub fn open_collage(&mut self, sheet: bool) {
        let ids = self.targets();
        if ids.is_empty() || (!sheet && ids.len() < 2) {
            self.toast(if sheet { "Put the pictures for the contact sheet in the tray first." } else { "Put at least two pictures in the tray to make a collage." });
            return;
        }
        // the title: the album or the folder they all come from
        let keys: Vec<&str> = ids.iter().filter_map(|&id| self.photos.get(id).map(|p| p.key.as_str())).collect();
        let album = self.store.albums.iter().find(|a| keys.iter().all(|k| a.keys.iter().any(|ak| ak == k))).map(|a| a.name.clone());
        let folder = ids.first().and_then(|&id| self.photos.get(id)).map(|p| p.folder).filter(|f| ids.iter().all(|&id| self.photos.get(id).is_some_and(|p| p.folder == *f)));
        let title = album.or_else(|| folder.and_then(|f| self.folders.get(f)).map(|f| f.name.clone())).unwrap_or_default();
        let times: Vec<i64> = ids.iter().filter_map(|&id| self.photos.get(id).map(|p| p.modified)).collect();
        let printers = if print::available() { print::printers() } else { vec![] };
        let printer = self.store.print.printer.clone().filter(|p| printers.contains(p)).or_else(print::default_printer).or_else(|| printers.first().cloned()).unwrap_or_default();
        let bg_key = if self.store.collage.background.kind == BgKind::Picture { ids.first().map(|&id| self.photos[id].key.clone()) } else { None };
        self.collage = Some(Box::new(CollageState {
            sheet,
            ids,
            from: self.view,
            nudges: HashMap::new(),
            top_z: 0,
            selected: None,
            drag: None,
            page: 0,
            title,
            subtitle: collage::sheet_subtitle(&times),
            bg_key: None,
            bg_tex: None,
            printers,
            printer,
            job: None,
        }));
        if let Some(k) = bg_key {
            self.set_collage_bg(k);
        }
        self.view = View::Collage;
    }

    fn close_collage(&mut self) {
        let from = self.collage.take().map(|c| c.from).unwrap_or(View::Library);
        self.view = if from == View::Edit { View::Edit } else { View::Library };
    }

    /// Use this file (a library picture or any picture file) as the background.
    fn set_collage_bg(&mut self, key: String) {
        let Some(st) = self.collage.as_mut() else { return };
        if st.bg_key.as_deref() == Some(key.as_str()) {
            return;
        }
        st.bg_key = Some(key.clone());
        st.bg_tex = None;
        self.lib.request(Job::File { key, dim: BIG_DIM }, true);
    }

    /// The background picture arrived (see `set_collage_bg`).
    pub fn on_collage_background(&mut self, key: String, img: Option<image::RgbaImage>) {
        let edits = self.key_index.get(&key).map(|&id| self.edits(id)).unwrap_or_default();
        let Some(st) = self.collage.as_mut() else { return };
        if st.bg_key.as_deref() != Some(key.as_str()) {
            return;
        }
        let Some(img) = img else {
            st.bg_key = None;
            // back to a colour, so the same picture isn't tried again and again
            self.store.collage.background.kind = BgKind::Color;
            self.toast("That picture could not be read.");
            return;
        };
        let img = crate::imaging::apply_all(&img, &edits);
        let sharp = self.ctx.load_texture("collage-bg", to_color_image(&img), TextureOptions::LINEAR);
        let soft = self.ctx.load_texture("collage-bg-soft", to_color_image(&collage::soft_picture(&img)), TextureOptions::LINEAR);
        if let Some(st) = self.collage.as_mut() {
            st.bg_tex = Some((key, sharp, soft));
        }
    }

    /// Width / height of a picture as shown (edited), from what is known now.
    fn shown_aspect(&self, id: usize) -> f32 {
        if let Some(t) = self.thumbs.get(&id) {
            let [w, h] = t.size();
            return w as f32 / h.max(1) as f32;
        }
        self.thumb_aspect.get(&id).copied().or_else(|| self.real_dims.get(&id).map(|&(w, h)| w as f32 / h.max(1) as f32)).unwrap_or(1.5)
    }

    fn collage_pics(&mut self) -> Vec<Pic> {
        let ids = self.collage.as_ref().map(|c| c.ids.clone()).unwrap_or_default();
        ids.iter()
            .map(|&id| {
                self.want_thumb(id);
                let p = &self.photos[id];
                Pic {
                    aspect: self.shown_aspect(id),
                    name: p.name.clone(),
                    caption: self.meta(id).map(|m| m.caption.clone()).unwrap_or_default(),
                    date: library::fmt_date(p.modified),
                }
            })
            .collect()
    }

    /// The pages as they are now (one for a collage).
    fn collage_pages(&mut self) -> Vec<Page> {
        let pics = self.collage_pics();
        let Some(st) = self.collage.as_ref() else { return vec![] };
        if st.sheet {
            collage::contact_sheet(&self.store.contact_sheet, &pics, &st.title, &st.subtitle)
        } else {
            let nudges: Vec<Nudge> = st.ids.iter().map(|id| st.nudges.get(id).copied().unwrap_or_default()).collect();
            vec![collage::collage(&self.store.collage, &pics, &nudges)]
        }
    }

    pub fn collage_ui(&mut self, ui: &mut egui::Ui) {
        if self.collage.is_none() {
            self.view = View::Library;
            return;
        }
        if let Some((done, _)) = self.collage.as_ref().and_then(|c| c.job.clone()) {
            if done.load(Ordering::SeqCst) == usize::MAX {
                if let Some(st) = self.collage.as_mut() {
                    st.job = None;
                }
            } else {
                ui.ctx().request_repaint_after(std::time::Duration::from_millis(200));
            }
        }
        let typing = ui.ctx().memory(|m| m.focused().is_some());
        if self.dialog.is_none() && !typing {
            if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                self.close_collage();
                return;
            }
            if ui.input(|i| i.key_pressed(egui::Key::Delete)) {
                self.collage_remove_selected();
            }
        }
        let pages = self.collage_pages();
        self.collage_bottom_bar(ui, &pages);
        self.collage_left_panel(ui);
        self.collage_canvas(ui, &pages);
    }

    fn collage_remove_selected(&mut self) {
        let Some(st) = self.collage.as_mut() else { return };
        let Some(sel) = st.selected.take() else { return };
        if st.ids.len() <= 1 {
            return;
        }
        st.ids.retain(|&id| id != sel);
        st.nudges.remove(&sel);
    }

    fn collage_left_panel(&mut self, ui: &mut egui::Ui) {
        egui::Panel::left("collage_left")
            .exact_size(304.0)
            .frame(egui::Frame::new().fill(t().left_bg).inner_margin(egui::Margin::same(12)).stroke(Stroke::new(1.0, t().border)))
            .show(ui, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    let sheet = self.collage.as_ref().is_some_and(|c| c.sheet);
                    // the styles, with the contact sheet last
                    let mut chosen: Option<Option<Style>> = None;
                    egui::Grid::new("collage_styles").num_columns(3).spacing([8.0, 8.0]).show(ui, |ui| {
                        let all: Vec<Option<Style>> = Style::ALL.iter().map(|s| Some(*s)).chain(std::iter::once(None)).collect();
                        for (i, style) in all.iter().enumerate() {
                            let label = style.map(|s| s.label()).unwrap_or("Contact sheet");
                            let tip = style.map(|s| s.describe()).unwrap_or("Pages of small pictures with a title and their file names");
                            let (rect, resp) = ui.allocate_exact_size(egui::vec2(84.0, 74.0), Sense::click());
                            let p = ui.painter();
                            let selected = if sheet { style.is_none() } else { *style == Some(self.store.collage.style) };
                            let tile = Rect::from_min_size(rect.min + egui::vec2(2.0, 0.0), egui::vec2(80.0, 54.0));
                            p.rect_filled(tile, 4, if selected { t().select_bg } else if resp.hovered() { t().surface_hover } else { t().surface });
                            p.rect_stroke(tile, 4, Stroke::new(if selected { 2.0 } else { 1.0 }, if selected { t().accent } else { t().border_strong }), StrokeKind::Inside);
                            paint_style_icon(p, tile.shrink(6.0), *style);
                            p.text(rect.center_bottom() - egui::vec2(0.0, 9.0), egui::Align2::CENTER_CENTER, label, egui::FontId::proportional(12.0), t().text);
                            if resp.on_hover_text(tip).clicked() {
                                chosen = Some(*style);
                            }
                            if i % 3 == 2 {
                                ui.end_row();
                            }
                        }
                    });
                    if let Some(style) = chosen {
                        self.choose_collage_style(style);
                    }
                    ui.add_space(6.0);
                    ui.separator();
                    if self.collage.as_ref().is_some_and(|c| c.sheet) {
                        self.sheet_options(ui);
                    } else {
                        self.collage_options(ui);
                    }
                });
            });
    }

    fn choose_collage_style(&mut self, style: Option<Style>) {
        let Some(st) = self.collage.as_mut() else { return };
        match style {
            None => st.sheet = true,
            Some(s) => {
                st.sheet = false;
                if self.store.collage.style != s {
                    self.store.collage.style = s;
                    self.store.collage.border = s.natural_border();
                }
            }
        }
        st.page = 0;
        st.drag = None;
        self.dirty = true;
    }

    fn collage_options(&mut self, ui: &mut egui::Ui) {
        let before = self.store.collage.clone();
        let style = before.style;
        ui.label(RichText::new(style.describe()).size(12.5).color(t().text_weak));
        ui.add_space(6.0);
        let mut new_seed = false;
        let mut shuffle = false;
        ui.horizontal(|ui| {
            if ui.button(format!("{}  New layout", icons::REFRESH)).on_hover_text("Other positions, turns and sizes").clicked() {
                new_seed = true;
            }
            if ui.button(format!("{}  Shuffle", icons::SWAP)).on_hover_text("Put the pictures in a random order").clicked() {
                shuffle = true;
            }
        });
        let s = &mut self.store.collage;
        if new_seed {
            s.seed = s.seed.wrapping_add(1);
            if let Some(st) = self.collage.as_mut() {
                st.nudges.clear();
            }
        }
        if shuffle && let Some(st) = self.collage.as_mut() {
            // a Fisher–Yates shuffle with a clock-based seed
            let mut x = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos() as u64).unwrap_or(7) | 1;
            for i in (1..st.ids.len()).rev() {
                x ^= x << 13;
                x ^= x >> 7;
                x ^= x << 17;
                st.ids.swap(i, (x % (i as u64 + 1)) as usize);
            }
            st.nudges.clear();
        }
        ui.add_space(6.0);
        let s = &mut self.store.collage;
        if style == Style::Shape {
            ui.horizontal(|ui| {
                ui.label("Shape");
                for k in ShapeKind::ALL {
                    ui.selectable_value(&mut s.shape, k, k.label());
                }
            });
        }
        if style.has_gap() {
            ui.horizontal(|ui| {
                ui.label("Space between pictures");
                ui.add(egui::Slider::new(&mut s.gap, 0.0..=1.0).show_value(false));
            });
        }
        if style.has_border() {
            ui.horizontal(|ui| {
                ui.label("Border");
                ui.selectable_value(&mut s.border, Border::None, "None");
                ui.selectable_value(&mut s.border, Border::White, "White");
                ui.selectable_value(&mut s.border, Border::Instant, "Instant");
            });
            if s.border == Border::Instant {
                ui.horizontal(|ui| {
                    ui.label("Text on the frames");
                    egui::ComboBox::from_id_salt("collage_caps")
                        .selected_text(match s.captions {
                            CaptionMode::None => "None",
                            CaptionMode::FileName => "File name",
                            CaptionMode::Caption => "Caption",
                        })
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut s.captions, CaptionMode::None, "None");
                            ui.selectable_value(&mut s.captions, CaptionMode::FileName, "File name");
                            ui.selectable_value(&mut s.captions, CaptionMode::Caption, "Caption");
                        });
                });
            }
        }
        if style.has_shadow() {
            ui.checkbox(&mut s.shadow, "Shadows");
        }
        ui.add_space(8.0);

        // background
        ui.label(RichText::new("Background").strong());
        let s = &mut self.store.collage;
        ui.horizontal(|ui| {
            ui.selectable_value(&mut s.background.kind, BgKind::Color, "Colour");
            ui.selectable_value(&mut s.background.kind, BgKind::Gradient, "Gradient");
            ui.selectable_value(&mut s.background.kind, BgKind::Picture, "Picture");
        });
        let mut pick_bg: Option<String> = None;
        let mut pick_file = false;
        match s.background.kind {
            BgKind::Color => {
                ui.horizontal(|ui| {
                    ui.color_edit_button_srgb(&mut s.background.color);
                    for c in [[255, 255, 255], [245, 240, 230], [40, 40, 44], [12, 12, 14], [214, 226, 240], [250, 228, 224]] {
                        let (r, resp) = ui.allocate_exact_size(egui::vec2(20.0, 20.0), Sense::click());
                        ui.painter().rect_filled(r, 3, Color32::from_rgb(c[0], c[1], c[2]));
                        ui.painter().rect_stroke(r, 3, Stroke::new(1.0, t().border_strong), StrokeKind::Inside);
                        if resp.clicked() {
                            s.background.color = c;
                        }
                    }
                });
            }
            BgKind::Gradient => {
                ui.horizontal(|ui| {
                    ui.color_edit_button_srgb(&mut s.background.color);
                    ui.label("to");
                    ui.color_edit_button_srgb(&mut s.background.color2);
                    egui::ComboBox::from_id_salt("collage_grad").selected_text(s.background.gradient.label()).show_ui(ui, |ui| {
                        for g in Gradient::ALL {
                            ui.selectable_value(&mut s.background.gradient, g, g.label());
                        }
                    });
                });
            }
            BgKind::Picture => {
                let st = self.collage.as_ref();
                let sel = st.and_then(|c| c.selected).and_then(|id| self.photos.get(id)).map(|p| p.key.clone());
                let current = st.and_then(|c| c.bg_key.clone());
                ui.horizontal(|ui| {
                    if ui.add_enabled(sel.is_some(), egui::Button::new("Use the selected picture")).on_disabled_hover_text("Click a picture of the collage first").clicked() {
                        pick_bg = sel.clone();
                    }
                    if ui.button("Choose a file…").clicked() {
                        pick_file = true;
                    }
                });
                let s = &mut self.store.collage;
                ui.checkbox(&mut s.background.blur, "Soft focus");
                let name = current.as_deref().map(|k| k.rsplit(['/', '\\']).next().unwrap_or(k).to_string());
                ui.label(RichText::new(name.unwrap_or_else(|| "No picture chosen yet".into())).size(12.0).color(t().text_weak));
                if current.is_none() && pick_bg.is_none() {
                    pick_bg = sel.or_else(|| self.collage.as_ref().and_then(|c| c.ids.first()).map(|&id| self.photos[id].key.clone()));
                }
            }
        }
        if pick_file
            && let Some(path) = rfd::FileDialog::new().set_title("Background picture").add_filter("Pictures", &["jpg", "jpeg", "png", "webp", "tif", "tiff", "bmp", "gif"]).pick_file()
        {
            pick_bg = Some(path.to_string_lossy().to_string());
        }
        if let Some(k) = pick_bg {
            self.set_collage_bg(k);
        }
        ui.add_space(8.0);

        // page
        ui.label(RichText::new("Page").strong());
        let s = &mut self.store.collage;
        page_format(ui, "collage_format", &mut s.format, &mut s.landscape);
        ui.horizontal(|ui| {
            ui.label("Picture size");
            egui::ComboBox::from_id_salt("collage_px").selected_text(format!("{} pixels", s.out_px)).show_ui(ui, |ui| {
                for px in collage::OUT_SIZES {
                    ui.selectable_value(&mut s.out_px, px, format!("{px} pixels"));
                }
            });
        });

        // the selected picture
        let sel = self.collage.as_ref().and_then(|c| c.selected);
        if let Some(id) = sel {
            ui.add_space(8.0);
            ui.label(RichText::new(self.photos.get(id).map(|p| p.name.clone()).unwrap_or_default()).strong());
            ui.horizontal(|ui| {
                if style == Style::Frame && ui.button("Put in the middle").clicked()
                    && let Some(st) = self.collage.as_mut()
                    && let Some(pos) = st.ids.iter().position(|&i| i == id)
                {
                    let v = st.ids.remove(pos);
                    st.ids.insert(0, v);
                }
                if style.free() && self.collage.as_ref().is_some_and(|c| c.nudges.contains_key(&id)) && ui.button("Put back").on_hover_text("Undo the moves of this picture").clicked()
                    && let Some(st) = self.collage.as_mut()
                {
                    st.nudges.remove(&id);
                }
                if ui.button(format!("{}  Leave out", icons::CLOSE)).on_hover_text("Take this picture out of the collage (Delete)").clicked() {
                    self.collage_remove_selected();
                }
            });
        }
        if format!("{:?}", before) != format!("{:?}", self.store.collage) {
            self.dirty = true;
        }
    }

    fn sheet_options(&mut self, ui: &mut egui::Ui) {
        let before = format!("{:?}", self.store.contact_sheet);
        if let Some(st) = self.collage.as_mut() {
            ui.label(RichText::new("Title").strong());
            ui.add(egui::TextEdit::singleline(&mut st.title).hint_text("No title").desired_width(270.0));
            ui.add(egui::TextEdit::singleline(&mut st.subtitle).hint_text("No second line").desired_width(270.0));
        }
        let s = &mut self.store.contact_sheet;
        ui.checkbox(&mut s.header, "Title on the first page");
        ui.add_space(6.0);
        ui.label(RichText::new("Under each picture").strong());
        ui.checkbox(&mut s.names, "File name");
        ui.checkbox(&mut s.captions, "Caption");
        ui.checkbox(&mut s.dates, "Date");
        ui.add_space(6.0);
        ui.label(RichText::new("Pictures").strong());
        ui.horizontal(|ui| {
            ui.label("Size");
            ui.add(egui::Slider::new(&mut s.thumb_mm, collage::THUMB_MM_MIN..=collage::THUMB_MM_MAX).suffix(" mm").step_by(1.0));
        });
        ui.checkbox(&mut s.shadow, "Shadows");
        ui.checkbox(&mut s.page_numbers, "Page numbers");
        ui.add_space(6.0);
        ui.label(RichText::new("Page").strong());
        page_format(ui, "sheet_format", &mut s.format, &mut s.landscape);
        if before != format!("{:?}", self.store.contact_sheet) {
            self.dirty = true;
        }
    }

    fn collage_bottom_bar(&mut self, ui: &mut egui::Ui, pages: &[Page]) {
        egui::Panel::bottom("collage_bottom")
            .frame(egui::Frame::new().fill(t().toolbar_bg).inner_margin(egui::Margin::symmetric(12, 8)))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    let Some(st) = self.collage.as_ref() else { return };
                    let (sheet, n, job) = (st.sheet, st.ids.len(), st.job.clone());
                    if let Some((done, total)) = &job {
                        ui.spinner();
                        let d = done.load(Ordering::SeqCst).min(*total);
                        ui.label(format!("Sending page {} of {} to the printer…", (d + 1).min(*total), total));
                    } else {
                        let pages_text = if sheet { format!(", {} page{}", pages.len(), if pages.len() == 1 { "" } else { "s" }) } else { String::new() };
                        ui.label(RichText::new(format!("{n} picture{}{pages_text}", if n == 1 { "" } else { "s" })).size(12.5));
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let busy = job.is_some();
                        let big = |text: String| egui::Button::new(RichText::new(text).size(14.0)).min_size(egui::vec2(100.0, 30.0));
                        if ui.add_enabled(!busy, big(format!("Close  {}", icons::CLOSE))).clicked() {
                            self.close_collage();
                            return;
                        }
                        if print::available() {
                            let can = self.collage.as_ref().is_some_and(|c| !c.printer.is_empty());
                            if ui.add_enabled(!busy && can, big(format!("{}  Print", icons::PRINT))).on_disabled_hover_text("No printer is available").clicked() {
                                self.collage_print(pages.to_vec());
                            }
                            let (printers, current) = self.collage.as_ref().map(|c| (c.printers.clone(), c.printer.clone())).unwrap_or_default();
                            if printers.len() > 1 {
                                egui::ComboBox::from_id_salt("collage_printer").selected_text(current.clone()).width(150.0).show_ui(ui, |ui| {
                                    for p in &printers {
                                        if ui.selectable_label(*p == current, p).clicked()
                                            && let Some(st) = self.collage.as_mut()
                                        {
                                            st.printer = p.clone();
                                        }
                                    }
                                });
                            }
                        }
                        if ui.add_enabled(!busy, big(format!("{}  Save as PDF…", icons::EXPORT))).clicked() {
                            self.collage_save(pages.to_vec(), true);
                        }
                        if !sheet && ui.add_enabled(!busy, big(format!("{}  Save picture…", icons::EXPORT))).on_hover_text("Save the collage as a JPEG or PNG picture").clicked() {
                            self.collage_save(pages.to_vec(), false);
                        }
                    });
                });
            });
    }

    fn collage_job(&self, pages: Vec<Page>) -> collage::Job {
        let st = self.collage.as_deref();
        let ids = st.map(|c| c.ids.clone()).unwrap_or_default();
        let sheet = st.is_some_and(|c| c.sheet);
        let background = if sheet { collage::Background::white() } else { self.store.collage.background };
        let bg_picture = if sheet { None } else { st.and_then(|c| c.bg_key.clone()).map(|k| {
            let edits = self.key_index.get(&k).map(|&id| self.edits(id)).unwrap_or_default();
            (k, edits)
        }) };
        collage::Job { pages, background, bg_picture, pictures: ids.iter().map(|&id| (self.photos[id].key.clone(), self.edits(id))).collect() }
    }

    fn collage_save(&mut self, pages: Vec<Page>, pdf: bool) {
        let sheet = self.collage.as_ref().is_some_and(|c| c.sheet);
        let name = if sheet { "Contact sheet" } else { "Collage" };
        let dialog = rfd::FileDialog::new().set_title(if pdf { "Save as PDF" } else { "Save the collage as a picture" });
        let picked = if pdf {
            dialog.set_file_name(format!("{name}.pdf")).add_filter("PDF document", &["pdf"]).save_file()
        } else {
            dialog.set_file_name(format!("{name}.jpg")).add_filter("JPEG picture", &["jpg", "jpeg"]).add_filter("PNG picture", &["png"]).save_file()
        };
        let Some(path) = picked else { return };
        let ext = path.extension().map(|e| e.to_string_lossy().to_ascii_lowercase()).unwrap_or_default();
        // the picker may return a name without an extension
        let path = match (pdf, ext.as_str()) {
            (true, "pdf") | (false, "jpg" | "jpeg" | "png") => path,
            (true, _) => path.with_extension("pdf"),
            (false, _) => path.with_extension("jpg"),
        };
        let job = self.collage_job(pages);
        self.toast(format!("Saving the {}…", name.to_lowercase()));
        self.lib.make_collage(job, path, self.store.collage.out_px);
        self.arm_tray_check();
    }

    fn collage_print(&mut self, pages: Vec<Page>) {
        let Some(st) = self.collage.as_ref() else { return };
        let choice = PrinterChoice { name: st.printer.clone(), ..Default::default() };
        self.store.print.printer = Some(choice.name.clone());
        self.dirty = true;
        let job = self.collage_job(pages);
        let total = job.pages.len();
        let progress = Arc::new(AtomicUsize::new(0));
        if let Some(st) = self.collage.as_mut() {
            st.job = Some((progress.clone(), total));
        }
        let (tx, ctx, pictures) = (self.tx.clone(), self.ctx.clone(), self.lib.pictures());
        std::thread::spawn(move || {
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let paper = print::paper(&choice).unwrap_or_else(Paper::a4);
                let dpi = paper.dpi.clamp(150, 300);
                let read = |k: &str, d: u32| pictures(k, d);
                let mut pages = (0..total).filter_map(|i| {
                    let page = collage::render_job_page(&job, i, dpi as f32 / 25.4, &read)?;
                    progress.store(i + 1, Ordering::SeqCst);
                    ctx.request_repaint();
                    Some(collage::on_paper(&page, &paper, dpi))
                });
                print::print_pages(&choice, "AlbumCraft collage", &mut pages, None)
            }))
            .unwrap_or_else(|_| Err("unexpected error".into()));
            progress.store(usize::MAX, Ordering::SeqCst);
            let _ = tx.send(Done::Message(match result {
                Ok(n) => format!("Sent {n} page(s) to {}.", choice.name),
                Err(e) => format!("Printing didn't finish: {e}"),
            }));
            ctx.request_repaint();
        });
        self.arm_tray_check();
    }

    fn collage_canvas(&mut self, ui: &mut egui::Ui, pages: &[Page]) {
        egui::CentralPanel::default().frame(egui::Frame::new().fill(t().status_bg).inner_margin(egui::Margin::same(14))).show(ui, |ui| {
            let Some(st) = self.collage.as_ref() else { return };
            let (sheet, ids, selected, drag) = (st.sheet, st.ids.clone(), st.selected, st.drag);
            let n_pages = pages.len().max(1);
            let page_no = st.page.min(n_pages - 1);
            let Some(page) = pages.get(page_no) else { return };
            let free = !sheet && self.store.collage.style.free();

            let hint = if !free {
                "Drag a picture onto another to swap them. Click one to select it (Delete leaves it out)."
            } else {
                "Drag a picture to move it, scroll over it to turn it. Click one to select it (Delete leaves it out)."
            };
            ui.label(RichText::new(hint).size(12.5).color(t().text_weak));
            let full = ui.available_rect_before_wrap();
            let nav_h = if n_pages > 1 { 40.0 } else { 0.0 };
            let area = Rect::from_min_max(full.min, Pos2::new(full.max.x, full.max.y - nav_h)).shrink(8.0);
            let s = (area.width() / page.size[0]).min(area.height() / page.size[1]).max(0.01);
            let sheet_rect = Rect::from_center_size(area.center(), egui::vec2(page.size[0] * s, page.size[1] * s));
            let v = View2 { origin: sheet_rect.min, s };
            let p = ui.painter().with_clip_rect(sheet_rect);
            ui.painter().rect_filled(sheet_rect.translate(egui::vec2(3.0, 4.0)), 0, Color32::from_black_alpha(50));

            // background
            let bg = if sheet { collage::Background::white() } else { self.store.collage.background };
            let bg_tex = self.collage.as_ref().and_then(|c| c.bg_tex.as_ref()).map(|(_, a, b)| if bg.blur { b.clone() } else { a.clone() });
            match (bg.kind, bg_tex) {
                (BgKind::Picture, Some(tex)) => {
                    let [tw, th] = tex.size();
                    let uv = cover_uv(tw as f32 / th.max(1) as f32, page.size[0] / page.size[1]);
                    tex_quad(&p, [sheet_rect.left_top(), sheet_rect.right_top(), sheet_rect.right_bottom(), sheet_rect.left_bottom()], tex.id(), uv, Color32::WHITE);
                }
                _ => {
                    // a grid of coloured points: the same colours as the saved page
                    const N: usize = 16;
                    let mut mesh = egui::Mesh::default();
                    for j in 0..=N {
                        for i in 0..=N {
                            let (u, w) = (i as f32 / N as f32, j as f32 / N as f32);
                            let c = bg.color_at(u, w);
                            mesh.colored_vertex(sheet_rect.min + egui::vec2(u * sheet_rect.width(), w * sheet_rect.height()), Color32::from_rgb(c[0], c[1], c[2]));
                        }
                    }
                    for j in 0..N {
                        for i in 0..N {
                            let a = (j * (N + 1) + i) as u32;
                            let b = a + (N + 1) as u32;
                            mesh.add_triangle(a, a + 1, b + 1);
                            mesh.add_triangle(a, b + 1, b);
                        }
                    }
                    p.add(egui::Shape::mesh(mesh));
                }
            }

            // the pictures: big previews when there are few and they are large on the screen
            let few = ids.len() <= 6;
            let dragged = drag.map(|d| d.id);
            for shape in &page.shapes {
                let tex = match shape {
                    Shape::Photo(ph) => {
                        let Some(&id) = ids.get(ph.pic) else { continue };
                        let long = ph.size[0].max(ph.size[1]) * s;
                        let big = if few && long > 300.0 { self.big_texture(id, &[]) } else { None };
                        big.or_else(|| {
                            self.want_thumb(id);
                            self.thumbs.get(&id).cloned()
                        })
                    }
                    _ => None,
                };
                paint_shape(&p, v, shape, tex.as_ref(), None);
                if let Shape::Photo(ph) = shape
                    && ids.get(ph.pic).is_some_and(|&id| Some(id) == selected || (!free && Some(id) == dragged))
                {
                    let (oc, os) = ph.outer();
                    let q = corners(v.at(oc), egui::vec2(os[0] * s, os[1] * s), ph.rot);
                    p.add(egui::Shape::closed_line(q.to_vec(), Stroke::new(2.5, t().accent)));
                }
            }

            // pointer: select, drag, turn
            let resp = ui.interact(sheet_rect, ui.scope_id().with("collage_canvas"), Sense::click_and_drag());
            let pic_at = |pos: Pos2| -> Option<(usize, usize)> {
                let i = page.hit(v.page(pos))?;
                let Shape::Photo(ph) = &page.shapes[i] else { return None };
                Some((ids.get(ph.pic).copied()?, i))
            };
            let hover = resp.hover_pos().and_then(pic_at);
            if hover.is_some() || drag.is_some() {
                ui.ctx().set_cursor_icon(if drag.is_some() { egui::CursorIcon::Grabbing } else { egui::CursorIcon::Grab });
            }
            if resp.drag_started()
                && let Some(pos) = resp.interact_pointer_pos()
                && let Some((id, _)) = pic_at(pos)
                && let Some(st) = self.collage.as_mut()
            {
                st.selected = Some(id);
                let mut start = st.nudges.get(&id).copied().unwrap_or_default();
                if free {
                    st.top_z += 1;
                    start.z = st.top_z;
                    st.nudges.insert(id, start);
                }
                st.drag = Some(Drag { id, from: v.page(pos), start });
            }
            if let (Some(d), Some(pos)) = (drag, ui.ctx().pointer_latest_pos()) {
                let at = v.page(pos);
                if free {
                    if let Some(st) = self.collage.as_mut() {
                        st.nudges.insert(d.id, Nudge { dx: d.start.dx + at[0] - d.from[0], dy: d.start.dy + at[1] - d.from[1], ..d.start });
                    }
                } else {
                    // the picture follows the pointer; the one under it gets a frame
                    if let Some((target, i)) = pic_at(pos).filter(|(t, _)| *t != d.id)
                        && let Shape::Photo(ph) = &page.shapes[i]
                    {
                        let (oc, os) = ph.outer();
                        let _ = target;
                        p.add(egui::Shape::closed_line(corners(v.at(oc), egui::vec2(os[0] * s, os[1] * s), ph.rot).to_vec(), Stroke::new(3.0, t().select_border)));
                    }
                    if let Some(tex) = self.thumbs.get(&d.id).cloned() {
                        let [tw, th] = tex.size();
                        let a = tw as f32 / th.max(1) as f32;
                        let size = if a >= 1.0 { egui::vec2(90.0, 90.0 / a) } else { egui::vec2(90.0 * a, 90.0) };
                        let r = Rect::from_center_size(pos, size);
                        ui.painter().rect_filled(r.expand(3.0), 2, Color32::WHITE);
                        tex_quad(ui.painter(), [r.left_top(), r.right_top(), r.right_bottom(), r.left_bottom()], tex.id(), Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)), Color32::WHITE.gamma_multiply(0.9));
                    }
                }
            }
            if resp.drag_stopped() || (drag.is_some() && !ui.input(|i| i.pointer.any_down())) {
                if let (Some(d), Some(pos)) = (drag, ui.ctx().pointer_latest_pos())
                    && !free
                    && let Some((target, _)) = pic_at(pos)
                    && target != d.id
                    && let Some(st) = self.collage.as_mut()
                    && let (Some(a), Some(b)) = (st.ids.iter().position(|&i| i == d.id), st.ids.iter().position(|&i| i == target))
                {
                    st.ids.swap(a, b);
                }
                if let Some(st) = self.collage.as_mut() {
                    st.drag = None;
                }
            }
            if resp.clicked()
                && let Some(st) = self.collage.as_mut()
            {
                st.selected = resp.interact_pointer_pos().and_then(pic_at).map(|(id, _)| id);
            }
            if free && let Some((id, _)) = hover {
                let scroll = ui.input(|i| i.smooth_scroll_delta.y);
                if scroll != 0.0
                    && let Some(st) = self.collage.as_mut()
                {
                    let n = st.nudges.entry(id).or_default();
                    n.rot += scroll * 0.004;
                }
            }

            // contact sheet pages
            if n_pages > 1 {
                let nav = Rect::from_center_size(Pos2::new(full.center().x, full.max.y - nav_h / 2.0), egui::vec2(200.0, 30.0));
                let mut child = ui.new_child(egui::UiBuilder::new().max_rect(nav).layout(egui::Layout::left_to_right(egui::Align::Center)));
                let mut new_page = page_no;
                if child.add_enabled(page_no > 0, egui::Button::new(icons::CHEVRON_LEFT)).clicked() {
                    new_page = page_no - 1;
                }
                child.label(format!("  Page {} of {}  ", page_no + 1, n_pages));
                if child.add_enabled(page_no + 1 < n_pages, egui::Button::new(icons::CHEVRON_RIGHT)).clicked() {
                    new_page = page_no + 1;
                }
                if let Some(st) = self.collage.as_mut() {
                    st.page = new_page;
                }
            }
        });
    }
}

/// Page size and orientation.
fn page_format(ui: &mut egui::Ui, id: &str, format: &mut Format, landscape: &mut bool) {
    ui.horizontal(|ui| {
        egui::ComboBox::from_id_salt(id).selected_text(format.label()).width(110.0).show_ui(ui, |ui| {
            for f in Format::ALL {
                ui.selectable_value(format, f, f.label());
            }
        });
        if *format != Format::Square {
            ui.selectable_value(landscape, true, "Landscape");
            ui.selectable_value(landscape, false, "Portrait");
        }
    });
}
