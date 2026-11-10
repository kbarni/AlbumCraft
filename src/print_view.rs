//! The print screen: "Page Layout" choices and printer settings on the
//! left, a page "Preview" in the middle, and Review / Print / Cancel at the bottom right.

use crate::icons;
use crate::app::*;
use crate::library::Done;
use crate::print::{self, BorderStyle, CaptionMode, Paper, PrintLayout, PrinterChoice};
use eframe::egui::{self, Color32, Pos2, Rect, RichText, Sense, Stroke, StrokeKind};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

pub struct PrintState {
    pub ids: Vec<usize>,
    pub from: View,
    pub printers: Vec<String>,
    pub choice: PrinterChoice,
    pub paper: Paper,
    /// false = no printer answered; an A4 page is shown instead
    pub paper_known: bool,
    /// Paper sizes offered for the printer (CUPS; empty on Windows).
    pub paper_sizes: Vec<String>,
    pub page: usize,
    pub show_options: bool,
    /// pages sent so far / total, while a print job runs
    pub job: Option<(Arc<AtomicUsize>, usize)>,
}

/// Draw a texture into `rect` showing the part `vis` (fractions of the picture as it appears on the
/// print, i.e. after the optional quarter turn clockwise).
fn paint_print_image(p: &egui::Painter, tex: egui::TextureId, rect: Rect, vis: [f32; 4], rotate: bool) {
    // a point (x', y') on the turned picture comes from (y', 1 - x') of the texture
    let map = |x: f32, y: f32| if rotate { Pos2::new(y, 1.0 - x) } else { Pos2::new(x, y) };
    let corners = [rect.left_top(), rect.right_top(), rect.right_bottom(), rect.left_bottom()];
    let uvs = [map(vis[0], vis[1]), map(vis[2], vis[1]), map(vis[2], vis[3]), map(vis[0], vis[3])];
    let mut mesh = egui::Mesh::with_texture(tex);
    for i in 0..4 {
        mesh.vertices.push(egui::epaint::Vertex { pos: corners[i], uv: uvs[i], color: Color32::WHITE });
    }
    mesh.indices = vec![0, 1, 2, 0, 2, 3];
    p.add(egui::Shape::mesh(mesh));
}

impl App {
    /// Open the print screen for the pictures in the tray (or the picture in the editor).
    pub fn open_print(&mut self) {
        let ids = self.targets();
        if ids.is_empty() {
            self.toast("Select the pictures you want to print first.");
            return;
        }
        let printers = print::printers();
        let wanted = self.store.print.printer.clone().filter(|p| printers.contains(p));
        let name = wanted.or_else(print::default_printer).or_else(|| printers.first().cloned()).unwrap_or_default();
        let choice = PrinterChoice { name, ..Default::default() };
        let paper = if choice.name.is_empty() { None } else { print::paper(&choice) };
        let paper_sizes = print::paper_sizes(&choice.name);
        self.print = Some(PrintState {
            ids,
            from: self.view,
            printers,
            choice,
            paper_known: paper.is_some(),
            paper: paper.unwrap_or_else(Paper::a4),
            paper_sizes,
            page: 0,
            show_options: false,
            job: None,
        });
        self.view = View::Print;
    }

    fn close_print(&mut self) {
        if let Some(ps) = self.print.take() {
            self.view = if ps.from == View::Edit { View::Edit } else { View::Library };
        } else {
            self.view = View::Library;
        }
    }

    /// The pictures in print order, each repeated "copies per picture" times.
    fn print_items(&self) -> Vec<usize> {
        let Some(ps) = &self.print else { return vec![] };
        let copies = self.store.print.copies.max(1) as usize;
        ps.ids.iter().flat_map(|&id| std::iter::repeat_n(id, copies)).collect()
    }

    fn set_printer(&mut self, choice: PrinterChoice) {
        let paper = print::paper(&choice);
        let sizes = if self.print.as_ref().is_some_and(|p| p.choice.name == choice.name) {
            self.print.as_ref().map(|p| p.paper_sizes.clone()).unwrap_or_default()
        } else {
            print::paper_sizes(&choice.name)
        };
        self.store.print.printer = Some(choice.name.clone());
        self.dirty = true;
        if let Some(ps) = self.print.as_mut() {
            ps.paper_known = paper.is_some();
            ps.paper = paper.unwrap_or_else(Paper::a4);
            ps.choice = choice;
            ps.paper_sizes = sizes;
            ps.page = 0;
        }
    }

    pub fn print_ui(&mut self, ui: &mut egui::Ui) {
        if self.print.is_none() {
            self.view = View::Library;
            return;
        }
        let busy = self.print.as_ref().is_some_and(|p| p.job.is_some());
        if !busy && self.dialog.is_none() && ui.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.close_print();
            return;
        }
        // a finished job
        if let Some((done, total)) = self.print.as_ref().and_then(|p| p.job.clone()) {
            if done.load(Ordering::SeqCst) == usize::MAX {
                if let Some(ps) = self.print.as_mut() {
                    ps.job = None;
                }
            } else {
                let _ = total;
                ui.ctx().request_repaint_after(std::time::Duration::from_millis(200));
            }
        }

        self.print_bottom_bar(ui);
        self.print_left_panel(ui);
        self.print_preview(ui);
    }

    fn print_left_panel(&mut self, ui: &mut egui::Ui) {
        egui::Panel::left("print_left")
            .exact_size(300.0)
            .frame(egui::Frame::new().fill(t().left_bg).inner_margin(egui::Margin::same(12)).stroke(Stroke::new(1.0, t().border)))
            .show(ui, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.label(RichText::new("Page Layout").size(16.0).color(t().text_weak));
                    ui.add_space(6.0);
                    let paper = self.print.as_ref().map(|p| p.paper).unwrap_or_else(Paper::a4);
                    let mut chosen = None;
                    egui::Grid::new("print_layouts").num_columns(3).spacing([10.0, 8.0]).show(ui, |ui| {
                        for (i, layout) in PrintLayout::ALL.iter().enumerate() {
                            let (rect, resp) = ui.allocate_exact_size(egui::vec2(80.0, 78.0), Sense::click());
                            let p = ui.painter();
                            let selected = self.store.print.layout == *layout;
                            let icon = Rect::from_min_size(rect.min + egui::vec2(10.0, 2.0), egui::vec2(60.0, 54.0));
                            p.rect_filled(icon, 4, if selected { t().select_bg } else if resp.hovered() { t().surface_hover } else { t().surface });
                            p.rect_stroke(icon, 4, Stroke::new(if selected { 2.0 } else { 1.0 }, if selected { t().accent } else { t().border_strong }), StrokeKind::Inside);
                            // a tiny sheet with the real grid of prints for the current paper
                            let s = 44.0 / paper.sheet_w.max(paper.sheet_h);
                            let sheet = Rect::from_center_size(icon.center(), egui::vec2(paper.sheet_w * s, paper.sheet_h * s));
                            p.rect_filled(sheet, 0, Color32::WHITE);
                            p.rect_stroke(sheet, 0, Stroke::new(1.0, t().text_weak), StrokeKind::Inside);
                            for c in print::page_cells(&paper, *layout) {
                                let r = Rect::from_min_size(sheet.min + egui::vec2((paper.area_x + c.x) * s, (paper.area_y + c.y) * s), egui::vec2(c.w * s, c.h * s));
                                p.rect_filled(r.shrink(0.5), 0, t().accent);
                            }
                            p.text(rect.center_bottom() - egui::vec2(0.0, 9.0), egui::Align2::CENTER_CENTER, layout.label(), egui::FontId::proportional(12.5), t().text);
                            if resp.clicked() {
                                chosen = Some(*layout);
                            }
                            if i % 3 == 2 {
                                ui.end_row();
                            }
                        }
                    });
                    if let Some(l) = chosen {
                        self.store.print.layout = l;
                        self.dirty = true;
                        if let Some(ps) = self.print.as_mut() {
                            ps.page = 0;
                        }
                    }
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        if ui.add(egui::Button::new("Shrink to Fit").selected(!self.store.print.crop)).on_hover_text("The whole picture is printed, with white space where the shapes differ").clicked() {
                            self.store.print.crop = false;
                            self.dirty = true;
                        }
                        if ui.add(egui::Button::new("Crop to Fit").selected(self.store.print.crop)).on_hover_text("The picture fills the print size; edges that don't fit are cut off").clicked() {
                            self.store.print.crop = true;
                            self.dirty = true;
                        }
                    });
                    ui.separator();

                    if ui.add(egui::Button::new("Border and Text Options").selected(self.print.as_ref().is_some_and(|p| p.show_options))).clicked()
                        && let Some(ps) = self.print.as_mut() {
                            ps.show_options = !ps.show_options;
                        }
                    if self.print.as_ref().is_some_and(|p| p.show_options) {
                        egui::Frame::new().fill(t().surface).inner_margin(egui::Margin::same(8)).corner_radius(4).show(ui, |ui| {
                            ui.label(RichText::new("Border").strong());
                            let s = &mut self.store.print;
                            let before = (s.border, s.caption);
                            ui.radio_value(&mut s.border, BorderStyle::None, "No border");
                            ui.radio_value(&mut s.border, BorderStyle::ThinBlack, "Thin black line");
                            ui.radio_value(&mut s.border, BorderStyle::White, "White border");
                            ui.add_space(4.0);
                            ui.label(RichText::new("Text under each picture").strong());
                            ui.radio_value(&mut s.caption, CaptionMode::None, "No text");
                            ui.radio_value(&mut s.caption, CaptionMode::FileName, "File name");
                            ui.radio_value(&mut s.caption, CaptionMode::Caption, "Caption");
                            if before != (s.border, s.caption) {
                                self.dirty = true;
                            }
                        });
                    }
                    ui.add_space(8.0);

                    // printer
                    ui.label(RichText::new("Printer").strong());
                    let (printers, current) = self.print.as_ref().map(|p| (p.printers.clone(), p.choice.name.clone())).unwrap_or_default();
                    let mut pick = None;
                    egui::ComboBox::from_id_salt("printer_pick")
                        .width(260.0)
                        .selected_text(if current.is_empty() { "No printer found".to_string() } else { current.clone() })
                        .show_ui(ui, |ui| {
                            for name in &printers {
                                if ui.selectable_label(*name == current, name).clicked() {
                                    pick = Some(name.clone());
                                }
                            }
                        });
                    if let Some(name) = pick {
                        self.set_printer(PrinterChoice { name, ..Default::default() });
                    }
                    ui.add_space(6.0);
                    let choice = self.print.as_ref().map(|p| p.choice.clone()).unwrap_or_default();
                    if cfg!(windows) {
                        ui.horizontal(|ui| {
                            if ui.button("Printer Setup…").on_hover_text("Paper size, orientation and the printer's own settings").clicked()
                                && let Some(c) = print::setup_dialog(&choice)
                            {
                                self.set_printer(c);
                            }
                        });
                    } else if !choice.name.is_empty() {
                        // CUPS: choose the paper here (the printer's default is preselected)
                        let sizes = self.print.as_ref().map(|p| p.paper_sizes.clone()).unwrap_or_default();
                        if !sizes.is_empty() {
                            let mut picked = None;
                            ui.horizontal(|ui| {
                                ui.label("Paper");
                                egui::ComboBox::from_id_salt("paper_pick")
                                    .selected_text(choice.media.clone().unwrap_or_else(|| "Printer default".into()))
                                    .show_ui(ui, |ui| {
                                        if ui.selectable_label(choice.media.is_none(), "Printer default").clicked() {
                                            picked = Some(None);
                                        }
                                        for s in &sizes {
                                            if ui.selectable_label(choice.media.as_deref() == Some(s.as_str()), s).clicked() {
                                                picked = Some(Some(s.clone()));
                                            }
                                        }
                                    });
                            });
                            if let Some(media) = picked {
                                self.set_printer(PrinterChoice { media, ..choice.clone() });
                            }
                        }
                    }
                    let (paper, known) = self.print.as_ref().map(|p| (p.paper, p.paper_known)).unwrap_or((Paper::a4(), false));
                    ui.label(RichText::new(if known { paper.describe() } else { "No printer answered — showing an A4 page".into() }).size(12.0).color(t().text));
                    ui.add_space(10.0);

                    // copies
                    ui.label(RichText::new("Copies per picture").strong());
                    ui.horizontal(|ui| {
                        let c = self.store.print.copies.max(1);
                        if ui.add_enabled(c > 1, egui::Button::new(icons::REMOVE)).clicked() {
                            self.store.print.copies = c - 1;
                            self.dirty = true;
                        }
                        if ui.add_enabled(c < 99, egui::Button::new(icons::ADD)).clicked() {
                            self.store.print.copies = c + 1;
                            self.dirty = true;
                        }
                        let c = self.store.print.copies.max(1);
                        ui.label(format!("{c} {}", if c == 1 { "copy" } else { "copies" }));
                    });
                });
            });
    }

    fn print_bottom_bar(&mut self, ui: &mut egui::Ui) {
        egui::Panel::bottom("print_bottom")
            .frame(egui::Frame::new().fill(t().toolbar_bg).inner_margin(egui::Margin::symmetric(12, 8)))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    let job = self.print.as_ref().and_then(|p| p.job.clone());
                    if let Some((done, total)) = &job {
                        ui.spinner();
                        let d = done.load(Ordering::SeqCst).min(*total);
                        ui.label(format!("Sending page {} of {} to the printer…", (d + 1).min(*total), total));
                    } else {
                        let n = self.print_items().len();
                        ui.label(RichText::new(format!("{} picture(s), {} print(s)", self.print.as_ref().map(|p| p.ids.len()).unwrap_or(0), n)).size(12.5));
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let busy = job.is_some();
                        if ui.add_enabled(!busy, egui::Button::new(RichText::new(format!("Cancel  {}", icons::CLOSE)).size(14.0)).min_size(egui::vec2(110.0, 30.0))).clicked() {
                            self.close_print();
                            return;
                        }
                        let can_print = self.print.as_ref().is_some_and(|p| !p.choice.name.is_empty() && p.paper_known);
                        if ui
                            .add_enabled(!busy && can_print, egui::Button::new(RichText::new(format!("Print  {}", icons::CHECK)).size(14.0).strong()).min_size(egui::vec2(110.0, 30.0)))
                            .on_disabled_hover_text("No printer is available")
                            .clicked()
                        {
                            self.print_pressed();
                        }
                        if ui.add_enabled(!busy, egui::Button::new(RichText::new("Review").size(14.0)).min_size(egui::vec2(110.0, 30.0))).on_hover_text("Check that each picture has enough pixels for its print size").clicked() {
                            self.print_review();
                        }
                    });
                });
            });
    }

    /// Picture size as it will be printed: the edited picture's pixel size in the original photo.
    fn print_pixels(&mut self, id: usize) -> Option<(f32, f32)> {
        let real = self.real_dims_of(id).map(|(w, h)| (w as f32, h as f32));
        let tex = self.big.get(&id).and_then(|e| e.tex.as_ref()).map(|(_, t)| t.size()).or_else(|| self.thumbs.get(&id).map(|t| t.size()));
        match (tex, self.real_scale(id)) {
            (Some(s), Some(r)) => Some((s[0] as f32 * r, s[1] as f32 * r)),
            (Some(s), None) => {
                let (rw, rh) = real?;
                // scale the thumbnail up to the photo's size
                let k = rw.max(rh) / (s[0].max(s[1]) as f32);
                Some((s[0] as f32 * k, s[1] as f32 * k))
            }
            _ => real,
        }
    }

    /// Pixels per inch and quality grade of each picture at the chosen print size.
    pub fn print_grades(&mut self) -> Vec<(usize, String, f32, u8)> {
        let Some(ps) = self.print.as_ref() else { return vec![] };
        let (paper, ids) = (ps.paper, ps.ids.clone());
        let cells = print::page_cells(&paper, self.store.print.layout);
        let Some(cell) = cells.first().copied() else { return vec![] };
        let settings = self.store.print.clone();
        let mut out = vec![];
        for id in ids {
            let name = self.photos[id].name.clone();
            let Some((w, h)) = self.print_pixels(id) else { continue };
            let (b, rotate) = print::picture_box(&cell, w as u32, h as u32, &settings);
            let (iw, ih) = if rotate { (h, w) } else { (w, h) };
            let (sx, sy) = (b.w / iw, b.h / ih); // mm per pixel needed to cover / to fit
            let mm_per_px = if settings.crop { sx.max(sy) } else { sx.min(sy) };
            let ppi = 25.4 / mm_per_px.max(1e-6);
            out.push((id, name, ppi, print::quality_grade(ppi)));
        }
        out
    }

    /// "Review": every picture with its quality.
    fn print_review(&mut self) {
        let grades = self.print_grades();
        self.dialog = Some(Dialog::PrintReview { grades, before_print: false });
    }

    /// "Print": first the same check; when a picture is too small, ask first.
    fn print_pressed(&mut self) {
        let grades = self.print_grades();
        if grades.iter().any(|g| g.3 == 0) {
            self.dialog = Some(Dialog::PrintReview { grades, before_print: true });
        } else {
            self.start_print_job();
        }
    }

    /// "Remove these pictures" from the too-small question.
    pub fn print_remove_bad(&mut self, bad: &[usize]) {
        if let Some(ps) = self.print.as_mut() {
            ps.ids.retain(|id| !bad.contains(id));
            ps.page = 0;
            if ps.ids.is_empty() {
                self.dialog = Some(Dialog::Info { title: "Print".into(), text: print::MSG_NONE_LEFT.into() });
                return;
            }
        }
        self.start_print_job();
    }

    pub fn start_print_job(&mut self) {
        let items = self.print_items();
        let Some(ps) = self.print.as_ref() else { return };
        let (paper, choice) = (ps.paper, ps.choice.clone());
        let settings = self.store.print.clone();
        let cells = print::page_cells(&paper, settings.layout);
        let per_page = cells.len().max(1);
        let total = items.len().div_ceil(per_page);
        let jobs: Vec<(String, Vec<crate::imaging::EditOp>, String)> = items
            .iter()
            .map(|&id| {
                let m = self.meta(id).cloned().unwrap_or_default();
                let caption = match settings.caption {
                    CaptionMode::None => String::new(),
                    CaptionMode::FileName => self.photos[id].name.clone(),
                    CaptionMode::Caption => m.caption.clone(),
                };
                (self.photos[id].key.clone(), m.edits.clone(), caption)
            })
            .collect();
        let progress = Arc::new(AtomicUsize::new(0));
        if let Some(ps) = self.print.as_mut() {
            ps.job = Some((progress.clone(), total));
        }
        let tx = self.tx.clone();
        let ctx = self.ctx.clone();
        let pictures = self.lib.pictures();
        std::thread::spawn(move || {
            // pages are drawn at the printer's resolution, but at most 300 dpi
            let dpi = paper.dpi.clamp(72, 300);
            let k = dpi as f32 / 25.4;
            let longest = cells.iter().map(|c| c.w.max(c.h)).fold(0.0f32, f32::max);
            let load_dim = ((longest * k * 1.2) as u32).clamp(400, crate::library::ZOOM_DIM);
            let mut pages = (0..total).map(|p| {
                let items: Vec<print::PageItem> = jobs[p * per_page..((p + 1) * per_page).min(jobs.len())]
                    .iter()
                    .map(|(key, edits, caption)| {
                        let img = pictures(key, load_dim).map(|i| crate::imaging::apply_all(&i, edits)).unwrap_or_else(|| image::RgbaImage::from_pixel(4, 3, image::Rgba([230, 230, 230, 255])));
                        print::PageItem { image: img, caption: caption.clone() }
                    })
                    .collect();
                let page = print::render_page(&paper, dpi, &cells, &items, &settings);
                progress.store(p + 1, Ordering::SeqCst);
                ctx.request_repaint();
                page
            });
            let result = print::print_pages(&choice, "AlbumCraft pictures", &mut pages, None);
            progress.store(usize::MAX, Ordering::SeqCst);
            let report = match result {
                Ok(n) => crate::library::Report::done(format!("Sent {n} page(s) to {}.", choice.name)),
                Err(e) => crate::library::Report::failed(format!("Printing didn't finish: {e}")),
            };
            let _ = tx.send(Done::Report(report));
            ctx.request_repaint();
        });
        self.arm_tray_check();
    }

    fn print_preview(&mut self, ui: &mut egui::Ui) {
        egui::CentralPanel::default().frame(egui::Frame::new().fill(t().status_bg).inner_margin(egui::Margin::same(14))).show(ui, |ui| {
            ui.label(RichText::new("Preview").size(16.0).color(t().text_weak));
            let Some(ps) = self.print.as_ref() else { return };
            let paper = ps.paper;
            let settings = self.store.print.clone();
            let cells = print::page_cells(&paper, settings.layout);
            let items = self.print_items();
            let per_page = cells.len().max(1);
            let pages = print::page_count(self.print.as_ref().map(|p| p.ids.len()).unwrap_or(0), settings.copies, per_page).max(1);
            let page = self.print.as_ref().map(|p| p.page).unwrap_or(0).min(pages - 1);

            let full = ui.available_rect_before_wrap();
            let nav_h = 40.0;
            let area = Rect::from_min_max(full.min, Pos2::new(full.max.x, full.max.y - nav_h)).shrink(10.0);
            let s = (area.width() / paper.sheet_w).min(area.height() / paper.sheet_h);
            let sheet = Rect::from_center_size(area.center(), egui::vec2(paper.sheet_w * s, paper.sheet_h * s));
            let p = ui.painter().clone();
            p.rect_filled(sheet.translate(egui::vec2(4.0, 4.0)), 0, Color32::from_black_alpha(40));
            p.rect_filled(sheet, 0, Color32::WHITE);
            p.rect_stroke(sheet, 0, Stroke::new(1.0, t().border), StrokeKind::Inside);
            let origin = sheet.min + egui::vec2(paper.area_x * s, paper.area_y * s);

            for (i, cell) in cells.iter().enumerate() {
                let Some(&id) = items.get(page * per_page + i) else { break };
                let tex = self.big_texture(id, &[]).or_else(|| {
                    self.request_thumb(id);
                    self.thumbs.get(&id).cloned()
                });
                let Some(tex) = tex else { continue };
                let [tw, th] = tex.size();
                let (b, rotate) = print::picture_box(cell, tw as u32, th as u32, &settings);
                let aspect = if rotate { th as f32 / tw as f32 } else { tw as f32 / th as f32 };
                let (r, vis) = print::fit_or_crop(&b, aspect, settings.crop);
                let rect = Rect::from_min_size(origin + egui::vec2(r.x * s, r.y * s), egui::vec2(r.w * s, r.h * s));
                paint_print_image(&p, tex.id(), rect, vis, rotate);
                if settings.border == BorderStyle::ThinBlack {
                    p.rect_stroke(rect, 0, Stroke::new(1.0, Color32::BLACK), StrokeKind::Outside);
                }
                if settings.caption != CaptionMode::None {
                    let text = match settings.caption {
                        CaptionMode::FileName => self.photos[id].name.clone(),
                        _ => self.meta(id).map(|m| m.caption.clone()).unwrap_or_default(),
                    };
                    if !text.is_empty() {
                        p.text(
                            origin + egui::vec2((b.x + b.w / 2.0) * s, (b.y + b.h + 1.2) * s),
                            egui::Align2::CENTER_TOP,
                            text,
                            egui::FontId::proportional((3.2 * s).max(6.0)),
                            Color32::from_rgb(40, 40, 40),
                        );
                    }
                }
            }

            // page navigation: "◀ 1 of 2 ▶"
            let nav = Rect::from_center_size(Pos2::new(full.center().x, full.max.y - nav_h / 2.0), egui::vec2(200.0, 30.0));
            let mut child = ui.new_child(egui::UiBuilder::new().max_rect(nav).layout(egui::Layout::left_to_right(egui::Align::Center)));
            let mut new_page = page;
            if child.add_enabled(page > 0, egui::Button::new(icons::CHEVRON_LEFT)).clicked() {
                new_page = page - 1;
            }
            child.label(format!("  {} of {}  ", page + 1, pages));
            if child.add_enabled(page + 1 < pages, egui::Button::new(icons::CHEVRON_RIGHT)).clicked() {
                new_page = page + 1;
            }
            if let Some(ps) = self.print.as_mut() {
                ps.page = new_page;
            }
        });
    }
}
