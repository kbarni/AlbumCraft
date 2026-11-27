//! The Library view: top bar (title, search, filters, picture size, Folders, Import), sidebar
//! (menu button, Picks, Places, albums, people, folders by year) and the grid of pictures in
//! justified rows, each keeping its real shape. The tray and the status bar under the grid are in
//! `library_view.rs`.

#![allow(deprecated)] // egui::Id::new, as used elsewhere in this crate

use crate::icons;
use crate::app::*;
use crate::i18n::tr;
use crate::library::{Flag, MAX_RATING, ScrollMode};
use crate::library_view::{paint_flag, paint_rating, paint_reject, paint_star};
use eframe::egui::{self, Color32, Pos2, Rect, RichText, Sense, Stroke, StrokeKind, Vec2};
use std::collections::HashMap;
use std::rc::Rc;

const GAP: f32 = 4.0;

/// The selected pictures in the order of the list.
fn flat_selection(flat: &[usize], selection: &std::collections::BTreeSet<usize>) -> Vec<usize> {
    flat.iter().copied().filter(|id| selection.contains(id)).collect()
}

/// While pictures are dragged: their number next to the pointer.
pub fn paint_dragged(ctx: &egui::Context) {
    let Some(d) = egui::DragAndDrop::payload::<DraggedPictures>(ctx) else { return };
    let Some(p) = ctx.pointer_latest_pos() else { return };
    ctx.set_cursor_icon(egui::CursorIcon::Grabbing);
    let n = d.0.len();
    let text = tr!("count-pictures", n = n);
    let painter = ctx.layer_painter(egui::LayerId::new(egui::Order::Tooltip, egui::Id::new("dragged_pictures")));
    let galley = painter.layout_no_wrap(text, egui::FontId::proportional(13.0), t().text_strong);
    let r = Rect::from_min_size(p + egui::vec2(14.0, 10.0), galley.size() + egui::vec2(16.0, 8.0));
    painter.rect_filled(r, 4, t().surface);
    painter.rect_stroke(r, 4, Stroke::new(1.0, t().accent), StrokeKind::Inside);
    painter.galley(r.min + egui::vec2(8.0, 4.0), galley, t().text_strong);
}

/// The picture one step away from `id` in the grid as laid out.
fn grid_step(cache: &GridCache, groups: &Rc<Vec<Group>>, id: usize, step: Step) -> Option<usize> {
    let flat = &cache.flat;
    let pos = flat.iter().position(|&x| x == id)?;
    match step {
        Step::Left => return flat.get(pos.checked_sub(1)?).copied(),
        Step::Right => return flat.get(pos + 1).copied(),
        Step::First => return flat.first().copied(),
        Step::Last => return flat.last().copied(),
        Step::Up | Step::Down => {}
    }
    // every row of the grid in order: (group, first, end, height)
    let rows: Vec<(usize, usize, usize, f32)> =
        cache.layout.iter().enumerate().flat_map(|(gi, g)| g.rows.iter().map(move |&(a, b, h)| (gi, a, b, h))).collect();
    let gi = groups.iter().position(|(_, ids)| ids.contains(&id))?;
    let k = groups.get(gi)?.1.iter().position(|&x| x == id)?;
    let r = rows.iter().position(|&(g, a, b, _)| g == gi && a <= k && k < b)?;
    // the middle of a picture across its row
    let centre = |row: (usize, usize, usize, f32), k: usize| -> f32 {
        let aspects = &cache.layout[row.0].aspects;
        let left: f32 = (row.1..k).map(|j| aspects.get(j).copied().unwrap_or(1.5) * row.3 + GAP).sum();
        left + aspects.get(k).copied().unwrap_or(1.5) * row.3 / 2.0
    };
    let x = centre(rows[r], k);
    let other = match step {
        Step::Up => *rows.get(r.checked_sub(1)?)?,
        _ => *rows.get(r + 1)?,
    };
    let nearest = (other.1..other.2).min_by(|&a, &b| (centre(other, a) - x).abs().total_cmp(&(centre(other, b) - x).abs()))?;
    groups.get(other.0)?.1.get(nearest).copied()
}

/// A move of the keyboard in the grid.
#[derive(Clone, Copy)]
enum Step {
    Left,
    Right,
    Up,
    Down,
    First,
    Last,
}
/// The height of a folder's header before it was drawn once (name line and spacing).
const HEADER_H: f32 = 48.0;

/// The grid's layout, kept between frames (`App::grid_rows`).
pub struct GridCache {
    /// The list it was made for.
    groups: Rc<Vec<Group>>,
    /// Every picture of the list in order (Shift+click ranges, asking for shapes).
    flat: Rc<Vec<usize>>,
    /// How far into `flat` shapes were asked for.
    shapes_asked: usize,
    /// (width, row height aimed at, `App::shape_rev`) the rows were made for.
    key: Option<(u32, u32, u64)>,
    layout: Vec<GroupRows>,
    /// Heights of the headers (folders, months, "Looks like") as last drawn.
    header_h: HashMap<Head, f32>,
}

/// The rows of one group.
struct GroupRows {
    /// Width / height of each picture.
    aspects: Vec<f32>,
    /// (first picture, one past the last, height) of each row.
    rows: Vec<(usize, usize, f32)>,
    /// Each row's distance from the top of the group's pictures.
    tops: Vec<f32>,
    /// All rows with the gaps after them.
    height: f32,
}
const PAD: i8 = 24;
/// How much faster than egui's default the wheel scrolls the grid, and the drag (middle button
/// or finger) compared with the pointer; how fast the drag's momentum dies away (per second).
const WHEEL_GAIN: f32 = 3.0;
/// The grid's scroll bar: wide and tall enough to grab with the mouse.
const BAR_W: f32 = 16.0;
const HANDLE_MIN: f32 = 72.0;
const PAN_GAIN: f32 = 1.6;
const PAN_FRICTION: f32 = 3.0;

/// How many levels of the folder tree (as on disk) are open at first.
const OPEN_LEVELS: usize = 3;

// ---------------------------------------------------------------------------------------------
// small widgets

/// A square, frameless glyph button.
pub(crate) fn icon_btn(ui: &mut egui::Ui, glyph: &str, tip: &str) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(Vec2::splat(34.0), Sense::click());
    if resp.hovered() {
        ui.painter().rect_filled(rect, 17, t().surface_hover);
    }
    crate::color_icons::paint(ui.ctx(), ui.painter(), glyph, rect.center(), 17.0, t().text);
    resp.on_hover_cursor(egui::CursorIcon::PointingHand).on_hover_text(tip)
}

/// A rounded text button. `primary` fills it with the accent colour.
fn pill(ui: &mut egui::Ui, label: &str, primary: bool) -> egui::Response {
    let colour = if primary { Color32::WHITE } else { t().text };
    let text = crate::color_icons::IconLabel::new(ui.painter(), label, egui::FontId::proportional(13.5), colour);
    let size = egui::vec2(text.size().x + 28.0, 34.0);
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
    let hovered = resp.hovered();
    if primary {
        // the main button: a gradient in the Bright style, the accent colour in the Calm one
        let (top, bottom) = (t().primary_top, t().primary_bottom);
        let k = if hovered { 0.88 } else { 1.0 };
        crate::theme::paint_gradient(ui.painter(), rect, 17.0, top.gamma_multiply(k), bottom.gamma_multiply(k));
    } else {
        ui.painter().rect_filled(rect, 17, if hovered { t().surface_hover } else { t().surface });
        ui.painter().rect_stroke(rect, 17, Stroke::new(1.0, t().border), StrokeKind::Inside);
    }
    text.paint(ui.ctx(), ui.painter(), rect.center(), colour);
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// A toggle chip (filters).
pub(crate) fn chip(ui: &mut egui::Ui, label: &str, on: bool) -> egui::Response {
    chip_with_flag(ui, label, on, false)
}

/// A toggle chip, optionally with a pick flag drawn before its label (no font has a flag
/// symbol that looks the same everywhere).
fn chip_with_flag(ui: &mut egui::Ui, label: &str, on: bool, flag: bool) -> egui::Response {
    let color = if on { t().accent } else { t().text };
    let text = crate::color_icons::IconLabel::new(ui.painter(), label, egui::FontId::proportional(13.0), color);
    let icon = if flag { if label.is_empty() { 14.0 } else { 20.0 } } else { 0.0 };
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(text.size().x + 24.0 + icon, 30.0), Sense::click());
    let rect_text = Rect::from_min_max(Pos2::new(rect.left() + icon, rect.top()), rect.max);
    let fill = if on {
        t().select_bg
    } else if resp.hovered() {
        t().surface_hover
    } else {
        Color32::TRANSPARENT
    };
    ui.painter().rect_filled(rect, 15, fill);
    ui.painter().rect_stroke(rect, 15, Stroke::new(1.0, if on { t().accent } else { t().border }), StrokeKind::Inside);
    if flag {
        // drawn after the fill so that it stays visible
        let c = Pos2::new(rect.left() + 12.0 + 7.0, rect.center().y);
        paint_flag(ui.painter(), c, 6.5, if on { t().pick } else { Color32::TRANSPARENT }, color);
    }
    text.paint(ui.ctx(), ui.painter(), rect_text.center(), color);
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// The two buttons beside the Folders heading: by year (calendar) or as on disk (tree), the one
/// in use highlighted. Gives the view clicked when it is not the one in use (`true`: the tree).
fn view_toggle(ui: &mut egui::Ui, tree: bool) -> Option<bool> {
    let cell = egui::vec2(28.0, 22.0);
    let (rect, _) = ui.allocate_exact_size(egui::vec2(cell.x * 2.0, cell.y), Sense::hover());
    let p = ui.painter();
    p.rect_stroke(rect, 6, Stroke::new(1.0, t().border), StrokeKind::Inside);
    let mut chosen = None;
    for (i, (is_tree, glyph, tip)) in [(false, icons::CALENDAR, tr!("folders-by-year")), (true, icons::TREE, tr!("folders-as-on-disk"))].into_iter().enumerate() {
        let r = Rect::from_min_size(rect.min + egui::vec2(cell.x * i as f32, 0.0), cell);
        let resp = ui.interact(r, ui.id().with(("folder-view", i)), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand).on_hover_text(tip);
        let on = is_tree == tree;
        let inner = r.shrink(2.0);
        if on {
            crate::theme::paint_gradient(ui.painter(), inner, 4.0, t().select_top, t().select_bottom);
        } else if resp.hovered() {
            ui.painter().rect_filled(inner, 4, t().surface_hover);
        }
        let colour = if on { t().accent } else { t().text_weak };
        crate::color_icons::paint(ui.ctx(), ui.painter(), glyph, r.center(), 14.0, colour);
        if resp.clicked() && !on {
            chosen = Some(is_tree);
        }
    }
    chosen
}

/// One row of the sidebar.
pub(crate) fn nav_row(ui: &mut egui::Ui, icon: &str, label: &str, count: Option<usize>, selected: bool) -> egui::Response {
    nav_row_with(ui, icon, label, None, count, selected)
}

/// A sidebar row that can also be dragged (a person, onto another).
pub(crate) fn nav_row_drag(ui: &mut egui::Ui, icon: &str, label: &str, count: Option<usize>, selected: bool) -> egui::Response {
    nav_row_sensed(ui, icon, label, None, count, selected, Sense::click_and_drag())
}

/// A row of the sidebar, with a dimmed second line under the label when `sub` is given (a
/// folder's caption).
fn nav_row_with(ui: &mut egui::Ui, icon: &str, label: &str, sub: Option<&str>, count: Option<usize>, selected: bool) -> egui::Response {
    nav_row_sensed(ui, icon, label, sub, count, selected, Sense::click())
}

fn nav_row_sensed(ui: &mut egui::Ui, icon: &str, label: &str, sub: Option<&str>, count: Option<usize>, selected: bool, sense: Sense) -> egui::Response {
    let h = if sub.is_some() { 44.0 } else { 32.0 };
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(ui.available_width(), h), sense);
    if !ui.is_rect_visible(rect) {
        return resp;
    }
    let p = ui.painter();
    if selected {
        crate::theme::paint_gradient(p, rect, 9.0, t().select_top, t().select_bottom);
    } else if resp.hovered() {
        p.rect_filled(rect, 9, t().surface_hover);
    }
    let color = if selected { t().accent } else { t().text };
    if !icon.is_empty() {
        crate::color_icons::paint(ui.ctx(), p, icon, Pos2::new(rect.left() + 18.0, rect.center().y), 15.0, color);
    }
    let wrap = rect.width() - 80.0;
    let galley = p.layout(label.to_string(), egui::FontId::proportional(13.5), color, wrap);
    match sub {
        Some(sub) => {
            // one line each: the label, then the caption cut short
            let mut job = egui::text::LayoutJob::simple_singleline(sub.to_string(), egui::FontId::proportional(11.5), t().text_weak);
            job.wrap = egui::text::TextWrapping::truncate_at_width(wrap);
            let small = p.layout_job(job);
            let top = rect.center().y - (galley.size().y + small.size().y) / 2.0;
            let gh = galley.size().y;
            p.galley(Pos2::new(rect.left() + 36.0, top), galley, color);
            p.galley(Pos2::new(rect.left() + 36.0, top + gh), small, t().text_weak);
        }
        None => p.galley(Pos2::new(rect.left() + 36.0, rect.center().y - galley.size().y / 2.0), galley, color),
    }
    if let Some(n) = count {
        p.text(
            Pos2::new(rect.right() - 10.0, rect.center().y),
            egui::Align2::RIGHT_CENTER,
            n.to_string(),
            egui::FontId::proportional(12.0),
            t().text_weak,
        );
    }
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// A soft shade over `rect`, from `top` to `bottom` (behind the buttons drawn on a picture).
fn shade(p: &egui::Painter, rect: Rect, top: Color32, bottom: Color32) {
    let mut mesh = egui::Mesh::default();
    for (pos, color) in [(rect.left_top(), top), (rect.right_top(), top), (rect.right_bottom(), bottom), (rect.left_bottom(), bottom)] {
        mesh.vertices.push(egui::epaint::Vertex { pos, uv: egui::epaint::WHITE_UV, color });
    }
    mesh.indices.extend([0, 1, 2, 0, 2, 3]);
    p.add(egui::Shape::mesh(mesh));
}

/// Splits `aspects` into rows that fill `width`: (start, end, row height).
fn justify(aspects: &[f32], width: f32, target: f32) -> Vec<(usize, usize, f32)> {
    let mut rows = vec![];
    let (mut start, mut sum) = (0usize, 0.0f32);
    for (i, a) in aspects.iter().enumerate() {
        sum += a;
        let n = (i - start) as f32;
        if sum * target + GAP * n >= width {
            rows.push((start, i + 1, (width - GAP * n) / sum));
            start = i + 1;
            sum = 0.0;
        }
    }
    if start < aspects.len() {
        rows.push((start, aspects.len(), target));
    }
    rows
}

// ---------------------------------------------------------------------------------------------

impl App {
    pub fn library_ui(&mut self, ui: &mut egui::Ui) {
        self.library_keys(ui.ctx());
        if !self.selection.is_empty() {
            self.tray_set = None;
        }
        self.status_bar(ui);
        self.selection_bar(ui);
        self.sidebar(ui);
        self.top_bar(ui);
        self.grid(ui);
        paint_dragged(ui.ctx());
    }

    fn source_title(&self) -> String {
        match &self.source {
            Source::Library => tr!("nav-all"),
            Source::Timeline => tr!("nav-timeline"),
            Source::Album(i) => self.store.albums.get(*i).map(|a| a.name.clone()).unwrap_or_default(),
            Source::Person(n) => n.clone(),
            Source::Group(_) => tr!("people-unnamed"),
            Source::People => tr!("nav-people"),
            Source::Faces(g) => tr!("faces-page-title", name = crate::faces::name_of(&self.store, *g).map(str::to_string).unwrap_or_else(|| tr!("people-unnamed"))),
            Source::Picks => tr!("nav-picks"),
        }
    }

    fn top_bar(&mut self, ui: &mut egui::Ui) {
        egui::Panel::top("topbar")
            .exact_size(60.0)
            .frame(egui::Frame::new().fill(t().lightbox_bg).inner_margin(egui::Margin::symmetric(PAD, 8)))
            .show(ui, |ui| {
                crate::theme::panel_gradient(ui, egui::Margin::symmetric(PAD, 8), t().band_top, t().band_bottom);
                let bottom = ui.max_rect().bottom() + 8.0;
                ui.painter().hline(ui.max_rect().x_range().expand(PAD as f32), bottom, Stroke::new(1.0, t().border));
                // the title gets a share of the bar and is cut short when it is longer (a longer
                // language must not push it under the buttons)
                let wide = ui.available_width();
                let title_w = (wide * 0.2).clamp(110.0, 240.0);
                ui.horizontal_centered(|ui| {
                    let shown = self.visible_count();
                    let person = self.shown_person();
                    let texts = person.as_ref().and_then(|(_, n)| n.as_deref()).map(|n| self.person_texts(n)).unwrap_or_default();
                    ui.allocate_ui_with_layout(egui::vec2(title_w, 44.0), egui::Layout::top_down(egui::Align::Min), |ui| {
                        ui.spacing_mut().item_spacing.y = 0.0;
                        let mut title = ui.add(egui::Label::new(RichText::new(self.source_title()).size(20.0).strong().color(t().text_strong)).truncate().sense(Sense::click()));
                        if person.is_some() {
                            // a double-click on a person's name changes it; the description is the tip
                            if !texts.1.is_empty() {
                                title = title.on_hover_text(texts.1.clone());
                            }
                            if title.double_clicked() {
                                self.edit_shown_person();
                            }
                        }
                        let sub = if self.scanning {
                            tr!("scanning")
                        } else if self.source == Source::People {
                            let n = self.people_count();
                            tr!("count-people", n = n)
                        } else if let Source::Faces(g) = self.source {
                            let n = self.faces_of(g).len();
                            tr!("count-faces", n = n)
                        } else if texts.0.is_empty() {
                            tr!("count-pictures", n = shown)
                        } else {
                            format!("{} · {}", texts.0, tr!("count-pictures", n = shown))
                        };
                        ui.add(egui::Label::new(RichText::new(sub).size(12.0).color(t().text_weak)).truncate());
                    });
                    if person.is_some() && icon_btn(ui, icons::EDIT, &tr!("person-edit-hint")).clicked() {
                        self.edit_shown_person();
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.spacing_mut().item_spacing.x = 8.0;
                        if pill(ui, &format!("{}  {}", icons::DRIVE, tr!("bar-import")), true)
                            .on_hover_text(tr!("bar-import-hint"))
                            .clicked()
                        {
                            self.open_import();
                        }
                        // icon only when the window is narrow (a longer language needs the room)
                        let folders = if wide >= 760.0 { pill(ui, &format!("{}  {}", icons::FOLDER, tr!("bar-folders")), false) } else { icon_btn(ui, icons::FOLDER, &tr!("bar-folders")) };
                        if folders.on_hover_text(tr!("bar-folders-hint")).clicked() {
                            self.open_settings(crate::settings_view::SettingsSection::Folders);
                        }
                        ui.add_space(4.0);
                        if wide > 980.0 {
                            ui.spacing_mut().slider_width = 84.0;
                            if ui.add(egui::Slider::new(&mut self.store.thumb_size, 64.0..=256.0).show_value(false)).on_hover_text(tr!("bar-size-hint")).changed() {
                                self.dirty = true;
                            }
                            ui.add_space(2.0);
                        }
                        // the order of the pictures (a small menu)
                        use crate::library::Sort;
                        let sorts = [(Sort::Name, tr!("sort-name")), (Sort::Taken, tr!("sort-taken")), (Sort::Rating, tr!("sort-rating"))];
                        let current = sorts.iter().find(|(s, _)| *s == self.store.sort).map(|(_, l)| l.clone()).unwrap_or_else(|| tr!("sort-name"));
                        let label = if wide > 1100.0 { format!("{current}  {}", icons::EXPAND_MORE) } else { icons::EXPAND_MORE.to_string() };
                        let resp = chip(ui, &label, false).on_hover_text(tr!("sort-hint", order = current.to_lowercase()));
                        egui::Popup::menu(&resp).show(|ui| {
                            for (sort, text) in sorts {
                                if ui.radio(self.store.sort == sort, text).clicked() {
                                    self.store.sort = sort;
                                    self.dirty = true;
                                }
                            }
                        });
                        // rating / rejects filter (a small menu), then the Picks toggle
                        let f = self.filter.clone();
                        let label = match (f.min_rating, f.hide_rejects) {
                            (0, false) => format!("{}  {}", icons::STAR, tr!("filter-any")),
                            (0, true) => format!("{}  {}", icons::STAR, tr!("filter-any-no-rejects")),
                            (n, false) => format!("{}  {}", icons::STAR, tr!("filter-min", n = n)),
                            (n, true) => format!("{}  {}", icons::STAR, tr!("filter-min-no-rejects", n = n)),
                        };
                        let resp = chip(ui, &label, f.min_rating > 0 || f.hide_rejects || f.people != crate::app::PeopleFilter::Any).on_hover_text(tr!("filter-hint"));
                        egui::Popup::menu(&resp).show(|ui| {
                            for n in 0..=crate::library::MAX_RATING {
                                let text = if n == 0 { tr!("filter-any-rating") } else { tr!("filter-or-more", rating = crate::library_view::rating_label(n)) };
                                if ui.radio(self.filter.min_rating == n, text).clicked() {
                                    self.filter.min_rating = n;
                                }
                            }
                            ui.separator();
                            ui.checkbox(&mut self.filter.hide_rejects, tr!("bar-hide-rejects"));
                            self.people_filter_menu(ui);
                            if ui.button(tr!("menu-trash-rejects")).clicked() {
                                self.ask_trash_rejects();
                                ui.close();
                            }
                        });
                        // the date filter: "Dates" or the range, a popup chooses it
                        self.dates_chip(ui, wide);
                        // the tags: the tree in a popup, a click shows a tag's pictures
                        let tag_on = self.search_terms.iter().any(|s| matches!(s, SearchTerm::Tag(_)));
                        let label = if wide > 900.0 { format!("{}  {}", icons::TAG, tr!("bar-tags")) } else { icons::TAG.to_string() };
                        let resp = chip(ui, &label, tag_on).on_hover_text(tr!("bar-tags-hint"));
                        egui::Popup::menu(&resp).close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside).show(|ui| self.tags_menu(ui));
                        let on = self.filter.picks;
                        if chip_with_flag(ui, &if wide > 800.0 { tr!("nav-picks") } else { String::new() }, on, true).on_hover_text(tr!("bar-picks-hint")).clicked() {
                            self.filter.picks = !on;
                        }
                        ui.add_space(4.0);
                        // the search box takes what the buttons left (its frame adds about 80)
                        let sw = (ui.available_width() - 80.0).clamp(40.0, 190.0 + 40.0 * self.search_terms.len().min(3) as f32);
                        self.library_search(ui, sw);
                    });
                });
            });
    }

    fn sidebar(&mut self, ui: &mut egui::Ui) {
        egui::Panel::left("folders")
            .resizable(true)
            .default_size(248.0)
            .size_range(200.0..=380.0)
            .frame(egui::Frame::new().fill(t().left_bg).inner_margin(egui::Margin::symmetric(12, 14)).stroke(Stroke::new(1.0, t().border)))
            .show(ui, |ui| {
                crate::theme::panel_gradient(ui, egui::Margin::symmetric(12, 14), t().side_top, t().side_bottom);
                // brand
                ui.horizontal(|ui| {
                    let (r, _) = ui.allocate_exact_size(Vec2::splat(28.0), Sense::hover());
                    let icon = self.app_icon();
                    ui.painter().image(icon.id(), r, Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)), Color32::WHITE);
                    ui.label(RichText::new("AlbumCraft").size(16.0).strong().color(t().text_strong));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.menu_button(RichText::new(icons::MENU).size(17.0), |ui| {
                            ui.set_min_width(190.0);
                            self.app_menu_items(ui);
                        });
                    });
                });
                ui.add_space(8.0);

                egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                    ui.spacing_mut().item_spacing.y = 2.0;
                    let all_on = self.source == Source::Library && !self.searching() && self.list_pick.is_none();
                    if nav_row(ui, icons::PHOTOS, &tr!("nav-all"), Some(self.photos.len()), all_on).clicked() {
                        self.source = Source::Library;
                        self.selection.clear();
                        self.tray_set = None;
                        self.list_pick = None;
                    }
                    if nav_row(ui, icons::CALENDAR, &tr!("nav-timeline"), Some(self.photos.len()), self.source == Source::Timeline).on_hover_text(tr!("nav-timeline-hint")).clicked() {
                        self.source = Source::Timeline;
                        self.selection.clear();
                        self.tray_set = None;
                        self.list_pick = None;
                    }
                    let counts = self.sidebar_counts();
                    let r = nav_row(ui, "", &tr!("nav-picks"), Some(counts.picks), self.source == Source::Picks);
                    paint_flag(ui.painter(), Pos2::new(r.rect.left() + 18.0, r.rect.center().y), 7.0, t().pick, t().pick);
                    if r.clicked() {
                        self.source = Source::Picks;
                        self.pick_set(TraySet::Picks);
                    }
                    let nplaces = self.located().len();
                    if nav_row(ui, icons::MAP, &tr!("nav-places"), Some(nplaces), false).on_hover_text(tr!("nav-places-hint")).clicked() {
                        self.open_places();
                    }

                    let albums_open = self.section_header(ui, "albums", &tr!("nav-albums"), |app, ui| {
                        if icon_btn(ui, icons::ADD, &tr!("dialog-new-album")).clicked() {
                            app.dialog = Some(Dialog::NewAlbum { name: String::new() });
                        }
                    });
                    let mut delete = None;
                    for i in (0..self.store.albums.len()).filter(|_| albums_open) {
                        let n = counts.albums.get(i).copied().unwrap_or(0);
                        let name = self.store.albums[i].name.clone();
                        let r = nav_row(ui, icons::ALBUM, &name, Some(n), self.source == Source::Album(i));
                        if r.dnd_hover_payload::<DraggedPictures>().is_some() {
                            ui.painter().rect_stroke(r.rect, 6, Stroke::new(2.0, t().accent), StrokeKind::Inside);
                        }
                        if let Some(dropped) = r.dnd_release_payload::<DraggedPictures>() {
                            self.add_to_album(i, &dropped.0);
                        }
                        if r.clicked() {
                            self.source = Source::Album(i);
                            self.pick_set(TraySet::Album(i));
                        }
                        r.context_menu(|ui| {
                            if ui.button(tr!("album-rename")).clicked() {
                                self.dialog = Some(Dialog::RenameAlbum { idx: i, name: self.store.albums[i].name.clone() });
                                ui.close();
                            }
                            if ui.button(tr!("album-delete")).clicked() {
                                delete = Some(i);
                                ui.close();
                            }
                            if ui.button(tr!("album-show-map")).clicked() {
                                let ids: Vec<usize> = self.store.albums[i].keys.iter().filter_map(|k| self.key_index.get(k).copied()).collect();
                                let name = self.store.albums[i].name.clone();
                                self.show_set_on_map(name, &ids);
                                ui.close();
                            }
                            if ui.button(tr!("album-slideshow")).clicked() {
                                self.source = Source::Album(i);
                                let list = self.visible_flat();
                                let first = list.first().copied().unwrap_or(0);
                                self.start_slideshow(list, first);
                                ui.close();
                            }
                        });
                    }
                    if let Some(i) = delete.filter(|&i| i < self.store.albums.len()) {
                        let album = self.store.albums.remove(i);
                        self.dirty = true;
                        // the albums after it moved up one place
                        self.source = match self.source {
                            Source::Album(s) if s == i => Source::Library,
                            Source::Album(s) if s > i => Source::Album(s - 1),
                            ref other => other.clone(),
                        };
                        self.tray_set = None;
                        self.list_pick = None;
                        self.undoable(tr!("album-deleted", name = album.name.as_str()), Undo::Album { at: i, album });
                    }

                    if self.section_header(ui, "people", &tr!("nav-people"), |_, _| {}) {
                        self.people_sidebar(ui, &counts.people);
                    }

                    let tag_nodes = self.tag_tree();
                    if !tag_nodes.is_empty() {
                        let manage = tr!("tags-manage");
                        let open = self.section_header(ui, "tags", &tr!("nav-tags"), |app, ui| {
                            if icon_btn(ui, icons::TAG, &manage).clicked() {
                                app.open_tag_manager(None);
                            }
                        });
                        if open {
                            ui.scope(|ui| {
                                ui.spacing_mut().indent = 10.0;
                                for node in tag_nodes.iter() {
                                    self.tag_row(ui, node, 0);
                                }
                            });
                        }
                    }

                    let folders_open = self.folders_caption(ui);
                    let current_folder = match &self.list_pick {
                        Some(TraySet::Folder(fi)) if self.selection.is_empty() => Some(*fi),
                        _ => self.selection.iter().next().map(|&id| self.photos[id].folder),
                    };
                    let tree = self.store.folder_view == "tree";
                    if tree && folders_open {
                        ui.scope(|ui| {
                            // a narrow step per level: names stay on one line in a narrow sidebar
                            ui.spacing_mut().indent = 10.0;
                            for node in &counts.tree {
                                self.tree_row(ui, node, 0, current_folder);
                            }
                        });
                    }
                    for (year, fis) in counts.by_year.iter().filter(|_| !tree && folders_open) {
                        let year = *year;
                        egui::CollapsingHeader::new(RichText::new(year.to_string()).size(13.0).strong().color(t().text))
                            .id_salt(("year", year))
                            .default_open(true)
                            .show(ui, |ui| {
                                for &fi in fis {
                                    let Some(f) = self.folders.get(fi) else { continue };
                                    let sub = self.store.folder_captions.get(&f.path).map(String::as_str);
                                    let r = nav_row_with(ui, icons::FOLDER, &f.name.clone(), sub, Some(f.photos.len()), current_folder == Some(fi) && self.source == Source::Library);
                                    if r.clicked() {
                                        self.source = Source::Library;
                                        self.scroll_to_folder = Some(fi);
                                        self.pick_set(TraySet::Folder(fi));
                                    }
                                    r.context_menu(|ui| self.folder_menu(ui, fi));
                                    r.on_hover_text(&self.folders[fi].path);
                                }
                            });
                    }
                });
            });
    }

    /// The heading of a sidebar section: a triangle and the name, a click on either folds the
    /// section away (remembered in `Store.closed_sections`), `extra` is drawn at the right.
    /// Returns whether the section is open.
    fn section_header(&mut self, ui: &mut egui::Ui, key: &str, title: &str, extra: impl FnOnce(&mut App, &mut egui::Ui)) -> bool {
        let open = !self.store.closed_sections.iter().any(|k| k == key);
        ui.add_space(14.0);
        let mut toggled = false;
        ui.horizontal(|ui| {
            let (rect, resp) = ui.allocate_exact_size(egui::vec2(12.0, 16.0), Sense::click());
            let c = rect.center();
            let pts = if open {
                vec![c + egui::vec2(-4.0, -2.0), c + egui::vec2(4.0, -2.0), c + egui::vec2(0.0, 3.0)]
            } else {
                vec![c + egui::vec2(-2.0, -4.0), c + egui::vec2(3.0, 0.0), c + egui::vec2(-2.0, 4.0)]
            };
            ui.painter().add(egui::Shape::convex_polygon(pts, t().text_weak, Stroke::NONE));
            let label = ui.add(egui::Label::new(RichText::new(title.to_uppercase()).size(11.0).strong().color(t().text_weak)).sense(Sense::click()));
            if resp.clicked() || label.clicked() {
                toggled = true;
            }
            if resp.hovered() || label.hovered() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| extra(self, ui));
        });
        ui.add_space(2.0);
        if toggled {
            if open {
                self.store.closed_sections.push(key.to_string());
            } else {
                self.store.closed_sections.retain(|k| k != key);
            }
            self.dirty = true;
        }
        open && !toggled
    }

    /// The Folders heading, with the choice of how they are listed: by year or as on disk.
    /// Returns whether the section is open.
    fn folders_caption(&mut self, ui: &mut egui::Ui) -> bool {
        self.section_header(ui, "folders", &tr!("nav-folders"), |app, ui| {
            let tree = app.store.folder_view == "tree";
            if let Some(tree) = view_toggle(ui, tree) {
                app.store.folder_view = if tree { "tree" } else { "years" }.into();
                app.dirty = true;
            }
        })
    }

    /// A tag of the sidebar's tag tree and, when it is open, the tags under it. A click shows
    /// the pictures with that tag (and those of the tags under it), a second one lets go.
    fn tag_row(&mut self, ui: &mut egui::Ui, node: &crate::tags::Node, depth: usize) {
        let key = crate::tags::key(&node.name);
        let is_current = self.search_terms.iter().any(|s| matches!(s, SearchTerm::Tag(t) if crate::tags::key(t) == key));
        let draw = |app: &mut App, ui: &mut egui::Ui| {
            let r = nav_row(ui, icons::TAG, &node.name, Some(node.total), is_current);
            if r.clicked() {
                app.search_terms.retain(|s| !matches!(s, SearchTerm::Tag(_)));
                if !is_current {
                    app.search_terms.push(SearchTerm::Tag(node.name.clone()));
                    app.source = Source::Library;
                    app.selection.clear();
                    app.tray_set = None;
                    app.list_pick = None;
                }
            }
        };
        if node.children.is_empty() {
            ui.horizontal(|ui| {
                ui.add_space((16.0 - ui.spacing().item_spacing.x).max(0.0));
                draw(self, ui);
            });
            return;
        }
        let id = ui.make_persistent_id(("tag-tree", &key));
        egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), id, depth < OPEN_LEVELS)
            .show_header(ui, |ui| draw(self, ui))
            .body(|ui| {
                for child in &node.children {
                    self.tag_row(ui, child, depth + 1);
                }
            });
    }

    /// A folder of the tree and, when it is open, the folders in it (the first `OPEN_LEVELS`
    /// levels are open at first).
    fn tree_row(&mut self, ui: &mut egui::Ui, node: &crate::folder_tree::Node, depth: usize, current: Option<usize>) {
        let is_current = node.folder.is_some() && node.folder == current && self.source == Source::Library;
        let draw = |app: &mut App, ui: &mut egui::Ui| {
            let (count, sub) = match node.folder.and_then(|fi| app.folders.get(fi).map(|f| (fi, f))) {
                Some((_, f)) => (f.photos.len(), app.store.folder_captions.get(&f.path).cloned()),
                None => (node.total, None),
            };
            let r = nav_row_with(ui, icons::FOLDER, &node.name, sub.as_deref(), Some(count), is_current);
            if let Some(fi) = node.folder {
                if r.clicked() {
                    app.source = Source::Library;
                    app.scroll_to_folder = Some(fi);
                    app.pick_set(TraySet::Folder(fi));
                }
                r.context_menu(|ui| app.folder_menu(ui, fi));
            }
            r.on_hover_text(&node.path)
        };
        if node.children.is_empty() {
            ui.horizontal(|ui| {
                // level with the rows beside it that have an open / close triangle (16 points
                // in front of their row, measured)
                ui.add_space((16.0 - ui.spacing().item_spacing.x).max(0.0));
                draw(self, ui);
            });
            return;
        }
        let id = ui.make_persistent_id(("folder-tree", &node.path));
        let mut clicked = false;
        egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), id, depth < OPEN_LEVELS)
            .show_header(ui, |ui| {
                let r = draw(self, ui);
                clicked = node.folder.is_none() && r.clicked();
            })
            .body(|ui| {
                for child in &node.children {
                    self.tree_row(ui, child, depth + 1, current);
                }
            });
        // a folder with no pictures of its own opens and closes when clicked
        if clicked {
            let mut state = egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), id, depth < OPEN_LEVELS);
            state.toggle(ui);
            state.store(ui.ctx());
        }
    }

    /// Width / height of a picture as far as it is known.
    pub fn aspect_of(&self, id: usize) -> f32 {
        if let Some(tex) = self.thumbs.get(&id) {
            let [w, h] = tex.size();
            return w as f32 / h.max(1) as f32;
        }
        if let Some(&a) = self.thumb_aspect.get(&id) {
            return a;
        }
        if let Some(&(w, h)) = self.real_dims.get(&id) {
            return w as f32 / h.max(1) as f32;
        }
        1.5
    }

    /// The grid's rows for this list, width and picture size: kept from the last frame when
    /// nothing they depend on changed, made again otherwise (a walk over every picture shown).
    fn grid_rows(&mut self, groups: &Rc<Vec<Group>>, width: f32, target: f32) -> GridCache {
        let mut cache = match self.grid_cache.take() {
            Some(c) if Rc::ptr_eq(&c.groups, groups) => c,
            _ => GridCache {
                groups: groups.clone(),
                flat: Rc::new(groups.iter().flat_map(|(_, v)| v.iter().copied()).collect()),
                shapes_asked: 0,
                key: None,
                layout: vec![],
                header_h: HashMap::new(),
            },
        };
        let key = (width.to_bits(), target.to_bits(), self.shape_rev);
        if cache.key != Some(key) {
            cache.key = Some(key);
            cache.layout = groups
                .iter()
                .map(|(_, ids)| {
                    let aspects: Vec<f32> = ids.iter().map(|&id| self.aspect_of(id)).collect();
                    let rows = justify(&aspects, width, target);
                    let mut tops = Vec::with_capacity(rows.len());
                    let mut height = 0.0;
                    for &(_, _, h) in &rows {
                        tops.push(height);
                        height += h + GAP;
                    }
                    GroupRows { aspects, rows, tops, height }
                })
                .collect();
        }
        cache
    }

    /// The keyboard in the grid: arrows move to the next picture (up and down: the one above or
    /// below, by its place in the rows), Shift+arrows extend the selection from where it started,
    /// Home and End go to the first and last picture, Space opens the picture. The grid scrolls
    /// to it.
    fn grid_keys(&mut self, ctx: &egui::Context, cache: &GridCache, groups: &Rc<Vec<Group>>) {
        use egui::{Key, Modifiers};
        if self.dialog.is_some() || self.text_focused() || cache.flat.is_empty() {
            return;
        }
        let (mut step, mut shift, mut space) = (None, false, false);
        ctx.input_mut(|i| {
            for (key, s) in [(Key::ArrowLeft, Step::Left), (Key::ArrowRight, Step::Right), (Key::ArrowUp, Step::Up), (Key::ArrowDown, Step::Down), (Key::Home, Step::First), (Key::End, Step::Last)] {
                if i.consume_key(Modifiers::NONE, key) {
                    step = Some(s);
                } else if i.consume_key(Modifiers::SHIFT, key) {
                    (step, shift) = (Some(s), true);
                }
            }
            space = i.consume_key(Modifiers::NONE, Key::Space);
        });
        let flat = &cache.flat;
        let current = self.cursor.or(self.anchor).or_else(|| self.selection.iter().next().copied()).filter(|id| flat.contains(id));
        if space {
            if let Some(id) = current {
                self.open_editor(id);
            }
            return;
        }
        let Some(step) = step else { return };
        // nothing chosen yet: the first key chooses the first picture
        let to = match current {
            None => flat.first().copied(),
            Some(id) => grid_step(cache, groups, id, step),
        };
        let Some(to) = to else { return };
        if shift {
            let from = self.anchor.filter(|a| flat.contains(a)).or(current).unwrap_or(to);
            let (Some(a), Some(b)) = (flat.iter().position(|&x| x == from), flat.iter().position(|&x| x == to)) else { return };
            self.selection = flat[a.min(b)..=a.max(b)].iter().copied().collect();
            self.anchor = Some(from);
        } else {
            self.selection.clear();
            self.selection.insert(to);
            self.anchor = Some(to);
        }
        self.tray_set = None;
        self.cursor = Some(to);
        self.reveal = Some(to);
    }

    /// Learn the real shape of pictures whose thumbnail is not loaded yet, in the background, a
    /// batch at a time (reading headers from a slow disk must not hold up the window). The list
    /// is walked once, a little further each time, not from its start on every frame.
    fn ask_for_shapes(&mut self, cache: &mut GridCache) {
        if self.dims_pending {
            return;
        }
        let mut batch: Vec<(usize, String)> = vec![];
        while batch.len() < 128 {
            let Some(&id) = cache.flat.get(cache.shapes_asked) else { break };
            cache.shapes_asked += 1;
            // the shape (unless a thumbnail shows it) and the date taken: for the layout, the
            // order by date, the status bar and the catalogue cache
            let shape_known = self.thumbs.contains_key(&id) || self.thumb_aspect.contains_key(&id) || self.real_dims.contains_key(&id);
            if (!shape_known || !self.taken.contains_key(&id))
                && let Some(p) = self.photos.get(id)
            {
                batch.push((id, p.key.clone()));
            }
        }
        if !batch.is_empty() {
            self.dims_pending = true;
            self.lib.request(crate::library::Job::Dims(batch), false);
        }
    }

    /// Scrolling the grid by dragging it, with the middle button or a finger: faster than the
    /// pointer, and it goes on after the release, slowing down.
    fn pan_grid(&mut self, ui: &mut egui::Ui) {
        let clip = ui.clip_rect();
        let (dt, delta, hold, started) = ui.input(|i| {
            let held = i.pointer.middle_down() || (i.any_touches() && i.pointer.primary_down());
            let started = (i.pointer.button_pressed(egui::PointerButton::Middle) || (i.any_touches() && i.pointer.primary_pressed()))
                && i.pointer.interact_pos().is_some_and(|p| clip.contains(p));
            (i.stable_dt.min(0.1), i.pointer.delta().y, held, started)
        });
        // Two fingers on a touchpad come as wheel events in points (a wheel's are in lines). The
        // system adds no momentum on Linux and Windows (macOS does, and sends it on), so the
        // speed of the fingers is kept here and carried on with once they stop.
        let (finger, notch) = ui.input(|i| {
            let mut finger = 0.0;
            let mut notch = false;
            for e in &i.raw.events {
                if let egui::Event::MouseWheel { unit, delta, .. } = e {
                    if *unit == egui::MouseWheelUnit::Point { finger += delta.y } else { notch = true }
                }
            }
            (finger, notch)
        });
        if !cfg!(target_os = "macos") && !self.grid_pan && clip.contains(ui.input(|i| i.pointer.hover_pos()).unwrap_or_default()) {
            if notch {
                self.grid_vel = 0.0;
            } else if finger != 0.0 {
                let v = finger * WHEEL_GAIN / dt.max(0.001);
                self.grid_vel = self.grid_vel * 0.5 + v * 0.5;
                ui.ctx().request_repaint();
                return;
            }
        }
        if started {
            self.grid_pan = true;
            self.grid_vel = 0.0;
        }
        if self.grid_pan && hold {
            let d = delta * PAN_GAIN;
            if d != 0.0 {
                ui.scroll_with_delta(egui::vec2(0.0, d));
            }
            // the speed of the last moments, to carry on with at the release
            let v = d / dt.max(0.001);
            self.grid_vel = self.grid_vel * 0.6 + v * 0.4;
            ui.ctx().request_repaint();
        } else {
            self.grid_pan = false;
            if self.grid_vel.abs() > 20.0 {
                ui.scroll_with_delta(egui::vec2(0.0, self.grid_vel * dt));
                self.grid_vel *= (-dt * PAN_FRICTION).exp();
                ui.ctx().request_repaint();
            } else {
                self.grid_vel = 0.0;
            }
        }
    }

    /// The scroll bar's Speed mode: a handle that rests in the middle of the track at the right
    /// edge; how far it is dragged up or down sets how fast the grid scrolls (`grid_rate`, used
    /// by the scroll area on the next frame), and it springs back when let go.
    fn speed_handle(&mut self, ui: &mut egui::Ui, panel: Rect) {
        let track = Rect::from_min_max(Pos2::new(panel.right() - BAR_W, panel.top()), panel.max);
        let half = (track.height() / 2.0 - HANDLE_MIN / 2.0).max(1.0);
        let r = ui.interact(track, egui::Id::new("grid_speed_handle"), Sense::drag());
        if r.dragged()
            && let Some(p) = ui.ctx().pointer_latest_pos()
        {
            self.grid_handle = (p.y - track.center().y).clamp(-half, half);
        } else {
            self.grid_handle = 0.0;
        }
        // squared, so that small drags are fine and the far end is fast
        let x = self.grid_handle / half;
        self.grid_rate = -x.signum() * x * x * track.height() * 5.0;
        let hovered = r.hovered() || r.dragged();
        let handle = Rect::from_center_size(Pos2::new(track.center().x, track.center().y + self.grid_handle), egui::vec2(BAR_W, HANDLE_MIN));
        let p = ui.painter();
        p.rect_filled(track, 0, t().accent.gamma_multiply(if hovered { 0.12 } else { 0.06 }));
        let mid = track.center();
        p.line_segment([Pos2::new(mid.x, track.top() + 4.0), Pos2::new(mid.x, track.bottom() - 4.0)], Stroke::new(1.0, t().accent.gamma_multiply(0.3)));
        p.rect_filled(handle.shrink2(egui::vec2(2.0, 0.0)), 5, t().accent.gamma_multiply(if hovered { 0.9 } else { 0.65 }));
        for dy in [-4.0, 0.0, 4.0] {
            let y = handle.center().y + dy;
            p.line_segment([Pos2::new(handle.center().x - 3.0, y), Pos2::new(handle.center().x + 3.0, y)], Stroke::new(1.0, Color32::WHITE.gamma_multiply(0.8)));
        }
        if hovered {
            ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeVertical);
        }
        if r.dragged() || self.grid_rate != 0.0 {
            ui.ctx().request_repaint();
        }
    }

    fn grid(&mut self, ui: &mut egui::Ui) {
        egui::CentralPanel::default().frame(egui::Frame::new().fill(t().lightbox_bg)).show(ui, |ui| {
            if self.source == Source::People {
                self.people_page(ui);
                return;
            }
            if let Source::Faces(g) = self.source {
                self.faces_page(ui, g);
                return;
            }
            let groups = self.groups();
            if groups.iter().all(|(_, v)| v.is_empty()) {
                self.empty_state(ui);
                return;
            }
            let target = (self.store.thumb_size * 1.25).clamp(90.0, 320.0);
            let ppp = ui.ctx().pixels_per_point();
            let mut open: Option<usize> = None;
            let mut clicked: Option<(usize, egui::Modifiers)> = None;
            let mut background_click = false;
            let mut flat: Rc<Vec<usize>> = Rc::new(vec![]);

            let panel = ui.max_rect();
            let speed = self.store.scroll_mode == ScrollMode::Speed;
            {
                let sc = &mut ui.spacing_mut().scroll;
                *sc = egui::style::ScrollStyle::solid();
                sc.bar_width = BAR_W;
                sc.handle_min_length = HANDLE_MIN;
            }
            let mut area = egui::ScrollArea::vertical().auto_shrink([false, false]).wheel_scroll_multiplier(egui::vec2(1.0, WHEEL_GAIN));
            if speed {
                area = area.scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden);
            }
            // the date scrubber: in the Timeline and the library's folders, when there are dates
            let scrubbing = matches!(self.source, Source::Library | Source::Timeline) && groups.iter().filter(|(h, _)| h.folder().is_some() || h.is_date()).nth(1).is_some();
            if let Some(y) = self.scrub.scroll_to.take() {
                area = area.vertical_scroll_offset(y);
            }
            // hidden, only its button keeps a narrow column of its own
            let shown = scrubbing && !self.store.scrubber_hidden;
            let right = if shown { PAD + crate::timeline::SCRUB_W as i8 } else if scrubbing { PAD + crate::timeline::TOGGLE_W as i8 } else { PAD };
            let out = area.show(ui, |ui| {
                if speed && self.grid_rate != 0.0 {
                    let dt = ui.input(|i| i.stable_dt).min(0.1);
                    ui.scroll_with_delta(egui::vec2(0.0, self.grid_rate * dt));
                    ui.ctx().request_repaint();
                }
                egui::Frame::new().inner_margin(egui::Margin { left: PAD, right, top: 8, bottom: 16 }).show(ui, |ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(GAP, GAP);
                    let width = ui.available_width();
                    // the background, under the pictures (they are added later and win the
                    // pointer): a click clears the selection, a drag draws a selection box
                    let origin = ui.min_rect().min.to_vec2();
                    let back = ui.interact(ui.clip_rect(), egui::Id::new("grid_background"), Sense::click_and_drag());
                    self.pan_grid(ui);
                    // on a touch screen a drag scrolls (see pan_grid); the box is for the mouse
                    let touch = ui.input(|i| i.any_touches());
                    if back.drag_started()
                        && !touch
                        && let Some(p) = back.interact_pointer_pos()
                    {
                        let m = ui.input(|i| i.modifiers);
                        let base = if m.command || m.shift { self.selection.clone() } else { Default::default() };
                        self.box_select = Some(BoxSelect { start: p.to_vec2() - origin, base, inside: Default::default() });
                    }
                    if back.clicked() {
                        background_click = true;
                    }
                    let boxed = match (&self.box_select, back.dragged(), ui.ctx().pointer_latest_pos()) {
                        (Some(b), true, Some(p)) => Some(Rect::from_two_pos((b.start + origin).to_pos2(), p)),
                        _ => None,
                    };
                    if back.drag_stopped() {
                        self.box_select = None;
                    }
                    // near the top or bottom edge, the grid scrolls on under the box
                    if let (Some(_), Some(p)) = (boxed, ui.ctx().pointer_latest_pos()) {
                        let clip = ui.clip_rect();
                        let dy = if p.y < clip.top() + 30.0 { 12.0 } else if p.y > clip.bottom() - 30.0 { -12.0 } else { 0.0 };
                        if dy != 0.0 {
                            ui.scroll_with_delta(egui::vec2(0.0, dy));
                            ui.ctx().request_repaint();
                        }
                    }
                    let mut cache = self.grid_rows(&groups, width, target);
                    self.ask_for_shapes(&mut cache);
                    flat = cache.flat.clone();
                    self.grid_keys(ui.ctx(), &cache, &groups);
                    // only what is in view is laid out and drawn; the rest is empty space of
                    // the height the rows would take
                    let clip = ui.clip_rect();
                    // where to scroll to, if asked (dropped when it is not in the list)
                    let to_photo = self.scroll_to_photo.take();
                    // reached with the keys: scrolled only as far as needed to be in view
                    let reveal = self.reveal.take();
                    let to_folder = self.scroll_to_folder.take();
                    // the folders' order changed (dates taken arrived): the folder that was at
                    // the top stays there
                    let order_rev = self.folder_order_rev.get();
                    let keep = self.grid_anchor.filter(|a| a.2 != order_rev && self.source == Source::Library);
                    let mut anchor: Option<(usize, f32, u64)> = None;
                    let dates = self.folder_dates();
                    let mut marks = Vec::with_capacity(groups.len());
                    for (gi, (folder, ids)) in groups.iter().enumerate() {
                        let Some(rows) = cache.layout.get(gi) else { break };
                        let (left, top) = (ui.cursor().left(), ui.cursor().top());
                        // a header not drawn yet is taken to be one line high
                        let head = match folder {
                            Head::Plain => 0.0,
                            h => cache.header_h.get(h).copied().unwrap_or(HEADER_H) + GAP,
                        };
                        let total = head + rows.height + 14.0;
                        marks.push((self.head_month(*folder, &dates), top - origin.y, total));
                        if let Some(fi) = folder.folder() {
                            if anchor.is_none() && top + total >= clip.top() {
                                anchor = Some((fi, clip.top() - top, order_rev));
                            }
                            if let Some((kf, above, _)) = keep
                                && kf == fi
                                && to_folder.is_none()
                            {
                                let moved = top - (clip.top() - above);
                                if moved.abs() > 0.5 {
                                    ui.scroll_with_delta(egui::vec2(0.0, -moved));
                                    ui.ctx().request_repaint();
                                }
                            }
                        }
                        if folder.folder().is_some() && to_folder == folder.folder() {
                            ui.scroll_to_rect(Rect::from_min_size(Pos2::new(left, top), egui::vec2(width, head.max(1.0))), Some(egui::Align::TOP));
                        }
                        if let Some(k) = to_photo.and_then(|id| ids.iter().position(|&x| x == id))
                            && let Some(r) = rows.rows.iter().position(|&(a, b, _)| a <= k && k < b)
                        {
                            let row = Rect::from_min_size(Pos2::new(left, top + head + rows.tops[r]), egui::vec2(width, rows.rows[r].2));
                            ui.scroll_to_rect(row, Some(egui::Align::Center));
                        }
                        if let Some(k) = reveal.and_then(|id| ids.iter().position(|&x| x == id))
                            && let Some(r) = rows.rows.iter().position(|&(a, b, _)| a <= k && k < b)
                        {
                            let row = Rect::from_min_size(Pos2::new(left, top + head + rows.tops[r]), egui::vec2(width, rows.rows[r].2));
                            ui.scroll_to_rect(row.expand(GAP * 2.0), None);
                        }
                        if top + total < clip.top() || top > clip.bottom() {
                            ui.add_space(total);
                            continue;
                        }
                        match folder {
                            Head::Folder(fi) => {
                                let h = cache.header_h.get(folder).copied().unwrap_or(HEADER_H);
                                let r = self.group_header(ui, *fi, ids, h);
                                cache.header_h.insert(*folder, r.rect.height());
                            }
                            Head::LooksLike => {
                                let r = self.looks_like_header(ui, ids);
                                cache.header_h.insert(*folder, r.rect.height());
                            }
                            Head::Month { .. } | Head::YearOnly(_) | Head::Undated => {
                                let h = cache.header_h.get(folder).copied().unwrap_or(HEADER_H);
                                let r = self.month_header(ui, *folder, ids, h);
                                cache.header_h.insert(*folder, r.rect.height());
                            }
                            Head::Plain => {}
                        }
                        let body = ui.cursor().top();
                        let n = rows.rows.len();
                        let first = (0..n).find(|&r| body + rows.tops[r] + rows.rows[r].2 >= clip.top()).unwrap_or(n);
                        let last = (first..n).find(|&r| body + rows.tops[r] > clip.bottom()).unwrap_or(n);
                        if first > 0 {
                            ui.add_space(rows.tops.get(first).copied().unwrap_or(rows.height));
                        }
                        for r in first..last {
                            let (a, b, h) = rows.rows[r];
                            let (row, _) = ui.allocate_exact_size(egui::vec2(width, h), Sense::hover());
                            // when the row is the last, stretched or not, pictures keep their own width
                            let mut x = row.left();
                            for k in a..b {
                                let (Some(&id), Some(&aspect)) = (ids.get(k), rows.aspects.get(k)) else { continue };
                                let w = aspect * h;
                                let rect = Rect::from_min_size(Pos2::new(x, row.top()), egui::vec2(w, h));
                                x += w + GAP;
                                self.want_sharp_thumb(id, w.max(h) * ppp);
                                if let (Some(b), Some(area)) = (self.box_select.as_mut(), boxed) {
                                    if area.intersects(rect) {
                                        b.inside.insert(id);
                                    } else {
                                        b.inside.remove(&id);
                                    }
                                }
                                let resp = ui.interact(rect, egui::Id::new(("tile", id)), Sense::click_and_drag());
                                // dragged onto an album in the sidebar: the selection, or this one
                                if resp.drag_started() {
                                    let ids = if self.selection.contains(&id) { flat_selection(&cache.flat, &self.selection) } else { vec![id] };
                                    resp.dnd_set_drag_payload(DraggedPictures(ids));
                                }
                                let hovered = resp.contains_pointer();
                                self.paint_tile(ui, rect, id, hovered);
                                let action = self.tile_buttons(ui, rect, id, hovered);
                                if action == TileAction::Check {
                                    let mut m = ui.input(|i| i.modifiers);
                                    m.command = true;
                                    m.shift = false;
                                    clicked = Some((id, m));
                                } else if action == TileAction::Pick {
                                    self.toggle_flag(&[id], Flag::Pick);
                                } else if action == TileAction::Reject {
                                    self.toggle_flag(&[id], Flag::Reject);
                                } else if let TileAction::Rate(n) = action {
                                    self.set_rating(&[id], n);
                                } else if resp.double_clicked() {
                                    open = Some(id);
                                } else if resp.clicked() {
                                    clicked = Some((id, ui.input(|i| i.modifiers)));
                                }
                                let resp = if resp.hovered() { resp.on_hover_text(self.photos[id].name.as_str()) } else { resp };
                                self.cell_menu(&resp, id);
                            }
                        }
                        // the rows below the window, and the gap before the next folder
                        let drawn = if last < n { rows.tops[last] } else { rows.height };
                        ui.add_space(rows.height - drawn + 14.0);
                    }
                    // when the grid was just moved to keep a folder in place, that folder is
                    // the one at the top (the move shows at the next frame)
                    self.grid_anchor = match keep {
                        Some((fi, above, _)) if groups.iter().any(|(h, _)| h.folder() == Some(fi)) => Some((fi, above, order_rev)),
                        _ => anchor,
                    };
                    self.grid_cache = Some(cache);
                    self.scrub.marks = marks;
                    if let (Some(b), Some(area)) = (self.box_select.as_ref(), boxed) {
                        self.selection = b.base.iter().chain(b.inside.iter()).copied().collect();
                        self.tray_set = None;
                        let p = ui.painter();
                        p.rect_filled(area, 2, t().accent.gamma_multiply(0.15));
                        p.rect_stroke(area, 2, Stroke::new(1.0, t().accent), StrokeKind::Inside);
                    }
                });
            });
            if speed {
                self.speed_handle(ui, panel);
            }
            if scrubbing {
                let top = self.scrubber_toggle(ui, Pos2::new(panel.right() - BAR_W - 2.0, panel.top() + 6.0));
                if shown {
                    let strip = Rect::from_min_max(Pos2::new(panel.right() - BAR_W - crate::timeline::SCRUB_W, top), Pos2::new(panel.right() - BAR_W - 2.0, panel.bottom() - 8.0));
                    // the marks are measured from the top of the frame inside the scroll area
                    self.scrubber(ui, strip, out.content_size.y, out.state.offset().y - 8.0, out.inner_rect.height());
                }
            }

            if let Some((id, m)) = clicked {
                if m.shift {
                    if let Some(a) = self.anchor {
                        let (pa, pb) = (flat.iter().position(|&x| x == a), flat.iter().position(|&x| x == id));
                        if let (Some(pa), Some(pb)) = (pa, pb) {
                            let (lo, hi) = (pa.min(pb), pa.max(pb));
                            if !m.command {
                                self.selection.clear();
                            }
                            self.selection.extend(flat[lo..=hi].iter().copied());
                        }
                    } else {
                        self.selection.insert(id);
                    }
                } else if m.command {
                    if !self.selection.remove(&id) {
                        self.selection.insert(id);
                    }
                    self.anchor = Some(id);
                    self.cursor = Some(id);
                } else {
                    let before = std::mem::take(&mut self.selection);
                    self.before_click = (before.len() > 1 && before.contains(&id)).then_some(before);
                    self.selection.insert(id);
                    self.anchor = Some(id);
                    self.cursor = Some(id);
                }
            } else if background_click {
                self.selection.clear();
                self.tray_set = None;
                self.list_pick = None;
            }
            if let Some(id) = open {
                // the first click of the double-click chose this picture alone: the selection
                // it replaced comes back
                if let Some(before) = self.before_click.take().filter(|b| b.contains(&id)) {
                    self.selection = before;
                }
                self.open_editor(id);
            }
        });
    }

    fn empty_state(&mut self, ui: &mut egui::Ui) {
        ui.vertical_centered(|ui| {
            ui.add_space(ui.available_height() * 0.3);
            let (icon, msg) = if self.scanning {
                (icons::HOURGLASS, tr!("empty-scanning"))
            } else if self.searching() || self.filter.is_active() {
                (icons::SEARCH, tr!("empty-no-match"))
            } else {
                match self.source {
                    Source::Album(_) => (icons::ALBUM, tr!("empty-album")),
                    Source::Person(_) | Source::Group(_) | Source::People | Source::Faces(_) => (icons::PERSON, tr!("empty-person")),
                    Source::Picks => (icons::FLAG, tr!("empty-picks")),
                    Source::Library if !self.store.roots.is_empty() => {
                        (icons::FOLDER, tr!("empty-unreachable"))
                    }
                    Source::Library | Source::Timeline => (icons::PHOTOS, tr!("empty-library")),
                }
            };
            let unreachable = matches!(self.source, Source::Library) && !self.store.roots.is_empty() && self.photos.is_empty() && !self.scanning;
            ui.label(RichText::new(icon).size(42.0));
            ui.add_space(6.0);
            ui.label(RichText::new(msg).size(15.0).color(t().text_weak));
            if unreachable && !self.searching() && !self.filter.is_active() {
                ui.add_space(12.0);
                ui.horizontal(|ui| {
                    // centred: two pills of about 290 px together
                    ui.add_space((ui.available_width() - 290.0).max(0.0) / 2.0);
                    if pill(ui, &format!("{}  {}", icons::REFRESH, tr!("scan-again")), true).clicked() {
                        self.rescan();
                    }
                    if pill(ui, &format!("{}  {}", icons::FOLDER, tr!("bar-folders")), false).on_hover_text(tr!("empty-folders-hint")).clicked() {
                        self.open_settings(crate::settings_view::SettingsSection::Folders);
                    }
                });
            } else if matches!(self.source, Source::Library) && !self.searching() && !self.scanning {
                ui.add_space(12.0);
                ui.horizontal(|ui| {
                    // centred: two pills of about 330 px together
                    ui.add_space((ui.available_width() - 330.0).max(0.0) / 2.0);
                    if pill(ui, &format!("{}  {}", icons::DRIVE, tr!("empty-import")), true)
                        .on_hover_text(tr!("empty-import-hint"))
                        .clicked()
                    {
                        self.open_import();
                    }
                    if pill(ui, &format!("{}  {}", icons::FOLDER, tr!("empty-add")), false)
                        .on_hover_text(tr!("empty-add-hint"))
                        .clicked()
                    {
                        self.add_library_folder();
                    }
                });
            }
        });
    }

    /// A folder's header in the grid: name, date and count, its buttons, caption and description.
    /// `h`: its height at the last frame (where a right-click opens the folder's menu).
    fn group_header(&mut self, ui: &mut egui::Ui, fi: usize, ids: &[usize], h: f32) -> egui::Response {
        let f = self.folders[fi].clone();
        let note = self.store.folder_notes.get(&f.path).cloned();
        let cap = self.store.folder_captions.get(&f.path).cloned();
        // behind the header's own buttons, which are added after it and so come first
        let area = Rect::from_min_size(ui.cursor().min, egui::vec2(ui.available_width(), h));
        let back = ui.interact(area, egui::Id::new(("folder_head", fi)), Sense::click());
        back.context_menu(|ui| self.folder_menu(ui, fi));
        let inner = ui.vertical(|ui| {
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                ui.label(RichText::new(&f.name).size(19.0).strong().color(t().text_strong));
                ui.label(RichText::new(format!("{}  ·  {}", self.folder_date_text(fi), ids.len())).size(12.5).color(t().text_weak));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if icon_btn(ui, icons::PLAY, &tr!("folder-play-hint")).clicked() {
                        self.start_slideshow(ids.to_vec(), ids[0]);
                    }
                    if icon_btn(ui, icons::EDIT, &tr!("folder-edit-hint")).clicked() {
                        self.edit_folder_texts(&f.path);
                    }
                    if icon_btn(ui, icons::MAP, &tr!("folder-map-hint")).clicked() {
                        self.show_set_on_map(f.name.clone(), ids);
                    }
                });
            });
            if let Some(c) = &cap {
                ui.label(RichText::new(c).size(14.0).color(t().text));
            }
            if let Some(n) = &note {
                ui.label(RichText::new(n).size(13.0).color(t().text_weak));
            }
            ui.add_space(4.0);
        });
        inner.response
    }

    fn paint_tile(&self, ui: &egui::Ui, rect: Rect, id: usize, hovered: bool) {
        let p = ui.painter();
        let selected = self.selection.contains(&id);
        let radius = 6u8;
        let img = if selected {
            p.rect_filled(rect, radius + 2, t().select_bg);
            rect.shrink((rect.width().min(rect.height()) * 0.07).clamp(6.0, 14.0))
        } else {
            rect
        };
        match self.thumbs.get(&id) {
            Some(tex) if !self.thumb_failed.contains(&id) => {
                p.add(
                    egui::epaint::RectShape::filled(img, radius, Color32::WHITE)
                        .with_texture(tex.id(), Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0))),
                );
            }
            _ => {
                p.rect_filled(img, radius, t().placeholder);
                if self.thumb_failed.contains(&id) {
                    p.text(img.center(), egui::Align2::CENTER_CENTER, "?", egui::FontId::proportional(20.0), t().text_weak);
                }
            }
        }
        if hovered || selected {
            // soft shade behind the check circle and star
            let strip = Rect::from_min_size(img.min + egui::vec2(2.0, 1.0), egui::vec2(img.width() - 4.0, 48.0_f32.min(img.height())));
            shade(p, strip, Color32::from_black_alpha(80), Color32::TRANSPARENT);
        }
        let stars_shown = hovered && img.width() >= 110.0 && img.height() >= 80.0;
        if stars_shown {
            // the same shade at the bottom, behind the stars to click
            let strip = Rect::from_min_max(Pos2::new(img.left() + 2.0, img.bottom() - 40.0), Pos2::new(img.right() - 2.0, img.bottom() - 1.0));
            shade(p, strip, Color32::TRANSPARENT, Color32::from_black_alpha(90));
        }
        if !selected && hovered {
            p.rect_filled(img, radius, Color32::from_white_alpha(14));
        }
        let m = self.meta(id);
        let flag = m.map(|m| m.flag).unwrap_or_default();
        if flag == Flag::Reject {
            // rejected: dimmed, with the reject mark
            p.rect_filled(img, radius, Color32::from_black_alpha(120));
            if !hovered {
                paint_reject(p, Pos2::new(img.right() - 14.0, img.top() + 14.0), 8.0, t().reject, Color32::from_black_alpha(140));
            }
        }
        // bottom left: pick flag, then the rating stars
        let mut x = img.left() + 14.0;
        let y = img.bottom() - 14.0;
        if flag == Flag::Pick && !hovered {
            paint_flag(p, Pos2::new(x, y), 7.0, t().pick, Color32::from_black_alpha(160));
            x += 16.0;
        }
        let rating = m.map(|m| m.rating).unwrap_or(0);
        if rating > 0 && img.width() > 70.0 && !stars_shown {
            paint_rating(p, Pos2::new(x - 6.0, y), rating, 5.0, Color32::from_black_alpha(140));
        }
        // a video: its length in a pill at the bottom right (the edited mark beside it)
        let mut right = img.right();
        if self.photos.get(id).is_some_and(|p| crate::video::is_video(&p.key)) && img.width() >= 48.0 {
            right = paint_video_badge(p, Pos2::new(img.right() - 6.0, img.bottom() - 6.0), self.durations.get(&id).copied()) - 4.0;
        }
        if m.is_some_and(|m| !m.edits.is_empty()) {
            let c = Pos2::new(right - 12.0, img.bottom() - 12.0);
            p.circle_filled(c, 5.0, Color32::from_rgb(80, 190, 110));
            p.circle_stroke(c, 5.0, Stroke::new(1.0, Color32::WHITE));
        }
    }

    /// The check circle (top left), the reject and pick buttons (top right) and, on hover, five
    /// stars (bottom left); returns what was clicked.
    fn tile_buttons(&self, ui: &mut egui::Ui, rect: Rect, id: usize, hovered: bool) -> TileAction {
        let selected = self.selection.contains(&id);
        let inset = if selected { (rect.width().min(rect.height()) * 0.07).clamp(6.0, 14.0) } else { 0.0 };
        let img = rect.shrink(inset);
        if !(hovered || selected) || img.width() < 60.0 || img.height() < 50.0 {
            return TileAction::None;
        }
        let c = img.left_top() + egui::vec2(18.0, 18.0);
        let check_r = Rect::from_center_size(c, Vec2::splat(30.0));
        let check = ui.interact(check_r, egui::Id::new(("chk", id)), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand);
        let p = ui.painter();
        if selected {
            p.circle_filled(c, 10.0, t().accent);
            p.circle_stroke(c, 10.0, Stroke::new(1.5, Color32::WHITE));
            let s = Stroke::new(2.0, Color32::WHITE);
            p.line_segment([c + egui::vec2(-4.5, 0.5), c + egui::vec2(-1.5, 3.5)], s);
            p.line_segment([c + egui::vec2(-1.5, 3.5), c + egui::vec2(4.5, -3.0)], s);
        } else {
            p.circle_filled(c, 10.0, if check.hovered() { Color32::from_black_alpha(110) } else { Color32::from_black_alpha(60) });
            p.circle_stroke(c, 10.0, Stroke::new(1.8, Color32::WHITE));
        }
        let fc = img.right_top() + egui::vec2(-18.0, 18.0);
        let flag_r = Rect::from_center_size(fc, Vec2::splat(30.0));
        let flag = ui.interact(flag_r, egui::Id::new(("pick", id)), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand).on_hover_text(tr!("tile-pick-hint"));
        let on = self.is_picked(id);
        let p = ui.painter();
        let r = if flag.hovered() { 9.0 } else { 8.0 };
        paint_flag(p, fc, r, if on { t().pick } else { Color32::TRANSPARENT }, Color32::WHITE);

        // reject, just left of the pick flag
        let mut reject = false;
        if img.width() >= 96.0 {
            let rc = fc - egui::vec2(30.0, 0.0);
            let rej = ui
                .interact(Rect::from_center_size(rc, Vec2::splat(28.0)), egui::Id::new(("reject", id)), Sense::click())
                .on_hover_cursor(egui::CursorIcon::PointingHand)
                .on_hover_text(tr!("tile-reject-hint"));
            let rejected = self.flag(id) == Flag::Reject;
            let color = if rejected { t().reject } else { Color32::WHITE };
            let bg = if rej.hovered() { Color32::from_black_alpha(110) } else { Color32::from_black_alpha(50) };
            paint_reject(ui.painter(), rc, if rej.hovered() { 8.5 } else { 7.5 }, color, bg);
            reject = rej.clicked();
        }

        // five stars along the bottom, on hover (clicking the current rating clears it)
        let mut rate = None;
        if hovered && img.width() >= 110.0 && img.height() >= 80.0 {
            let current = self.rating(id);
            let (sr, step) = (7.0, 17.0);
            let base = Pos2::new(img.left() + 10.0 + step / 2.0, img.bottom() - 14.0);
            let hover_n = (0..MAX_RATING).find(|&i| {
                let cell = Rect::from_center_size(base + egui::vec2(i as f32 * step, 0.0), egui::vec2(step, 24.0));
                ui.ctx().pointer_hover_pos().is_some_and(|p| cell.contains(p))
            });
            for i in 0..MAX_RATING {
                let c = base + egui::vec2(i as f32 * step, 0.0);
                let cell = ui
                    .interact(Rect::from_center_size(c, egui::vec2(step, 24.0)), egui::Id::new(("star", id, i)), Sense::click())
                    .on_hover_cursor(egui::CursorIcon::PointingHand);
                let n = i + 1;
                let lit = hover_n.map_or(n <= current, |h| i <= h);
                let fill = if lit { t().star } else { Color32::from_black_alpha(60) };
                paint_star(ui.painter(), c, sr, fill, if lit { Color32::from_black_alpha(140) } else { Color32::WHITE });
                if cell.clicked() {
                    rate = Some(if current == n { 0 } else { n });
                }
            }
        }

        if check.clicked() {
            TileAction::Check
        } else if flag.clicked() {
            TileAction::Pick
        } else if reject {
            TileAction::Reject
        } else if let Some(n) = rate {
            TileAction::Rate(n)
        } else {
            TileAction::None
        }
    }
}

/// What was clicked on a tile's overlay.
#[derive(Clone, Copy, PartialEq)]
enum TileAction {
    None,
    Check,
    Pick,
    Reject,
    Rate(u8),
}

/// A video's mark on a thumbnail: a play triangle and its length (when known) in a dark pill
/// whose bottom right corner is `corner`. Returns the pill's left edge. Drawn on top of the
/// picture, so white on dark in both themes.
pub fn paint_video_badge(p: &egui::Painter, corner: Pos2, secs: Option<f32>) -> f32 {
    let font = egui::FontId::proportional(11.0);
    let text = secs.map(|s| crate::video::fmt_duration(f64::from(s))).unwrap_or_default();
    let galley = p.layout_no_wrap(text, font, Color32::WHITE);
    let h = 18.0;
    let tri = 8.0;
    let gap = if galley.size().x > 0.0 { 4.0 } else { 0.0 };
    let w = 6.0 + tri + gap + galley.size().x + 6.0;
    let rect = Rect::from_min_max(Pos2::new(corner.x - w, corner.y - h), corner);
    p.rect_filled(rect, h / 2.0, Color32::from_black_alpha(150));
    let cx = rect.left() + 6.0;
    let cy = rect.center().y;
    p.add(egui::Shape::convex_polygon(
        vec![Pos2::new(cx, cy - tri / 2.0 - 0.5), Pos2::new(cx + tri, cy), Pos2::new(cx, cy + tri / 2.0 + 0.5)],
        Color32::WHITE,
        Stroke::NONE,
    ));
    p.galley(Pos2::new(cx + tri + gap, cy - galley.size().y / 2.0), galley, Color32::WHITE);
    rect.left()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Two folders: rows of pictures 0 1 2 | 3 4 (folder 0) and 5 6 7 (folder 1), all the same
    /// shape. Up and down go to the picture above or below, also across folders.
    #[test]
    fn arrow_keys_follow_the_rows() {
        let groups: Rc<Vec<Group>> = Rc::new(vec![(Head::Folder(0), vec![0, 1, 2, 3, 4]), (Head::Folder(1), vec![5, 6, 7])]);
        let rows = |n: usize, rows: Vec<(usize, usize, f32)>| GroupRows { aspects: vec![1.0; n], tops: vec![0.0; rows.len()], height: 0.0, rows };
        let cache = GridCache {
            groups: groups.clone(),
            flat: Rc::new((0..8).collect()),
            shapes_asked: 0,
            key: None,
            layout: vec![rows(5, vec![(0, 3, 100.0), (3, 5, 150.0)]), rows(3, vec![(0, 3, 100.0)])],
            header_h: HashMap::new(),
        };
        let go = |id, s| grid_step(&cache, &groups, id, s);
        assert_eq!(go(2, Step::Right), Some(3));
        assert_eq!(go(0, Step::Left), None);
        // centres: first row 50, 154, 258; second row (taller) 75, 229
        assert_eq!(go(0, Step::Down), Some(3));
        assert_eq!(go(1, Step::Down), Some(4));
        assert_eq!(go(4, Step::Up), Some(2));
        // from the last row of a folder into the first row of the next
        assert_eq!(go(4, Step::Down), Some(7));
        assert_eq!(go(5, Step::Up), Some(3));
        assert_eq!(go(6, Step::Down), None);
        assert_eq!((go(4, Step::First), go(0, Step::Last)), (Some(0), Some(7)));
    }
}

