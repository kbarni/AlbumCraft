//! The Timeline (`Source::Timeline`): every picture of the library by month, newest first
//! (`Store.timeline_oldest_first` reverses it), under month headers ("July 2019 · 412"). A date
//! known only to the year (a scanned print) has its own group ("1978 · month unknown") after
//! that year's months (before them when oldest first); pictures without a usable date are last
//! ("No date"). The date is `App::taken_at`'s: the user's, else the file's (file dates before
//! `folder_dates::EARLIEST` come from a clock never set and count as none).
//!
//! The date scrubber: a strip at the right edge of the grid, beside the scroll bar, in the
//! Timeline and in the library's folders. Years are written where they start, each month is a
//! tick, the part in view is marked; pointing at it shows the month there and a click or a drag
//! scrolls the grid to it. Positions on the strip are positions in the grid, so a busy month
//! takes more of it than a quiet one.

#![allow(deprecated)] // egui::Id::new, as used elsewhere in this crate

use crate::app::{App, Group, Head};
use crate::i18n::tr;
use crate::icons;
use crate::library;
use crate::taken::DatePrecision;
use crate::theme::t;
use eframe::egui::{self, Color32, Pos2, Rect, RichText, Sense, Stroke};

/// The scrubber's width (labels and ticks), left of the scroll bar.
pub const SCRUB_W: f32 = 46.0;

/// The width kept for the scrubber's button while the scrubber is hidden.
pub const TOGGLE_W: f32 = 26.0;

/// A group of the grid on the scrubber: its month (None for a group without one), its top and
/// height in the grid.
pub type Mark = (Option<(i32, u32)>, f32, f32);

/// Where each group of the grid is, as last drawn (`App.scrub`), for the scrubber.
#[derive(Default, Debug)]
pub struct Scrub {
    pub marks: Vec<Mark>,
    /// The scroll offset to set at the next frame (a click or drag on the strip).
    pub scroll_to: Option<f32>,
    /// A Shift+drag choosing a range of dates: the month and the place where it started.
    pub range_from: Option<((i32, u32), f32)>,
    /// A drag across the Dates popup's bars: the month where it started.
    pub bars_from: Option<i32>,
    /// Keep the Dates popup open (the `dates` screenshot).
    pub show_popup: bool,
}

/// A month as one number (12 a year), for ranges.
pub fn month_index(year: i32, month: u32) -> i32 {
    year * 12 + month as i32 - 1
}

/// `month_index` back to (year, month 1…12).
pub fn month_of_index(i: i32) -> (i32, u32) {
    (i.div_euclid(12), i.rem_euclid(12) as u32 + 1)
}

/// What a range of months is called: "March 2019", "2019", "2017 – 2019", "March 2019 – August 2021".
pub fn range_title(range: (i32, i32)) -> String {
    range_text(range, month_title)
}

/// `range_title` with short month names, for the chip: "Mar 2019 – Aug 2021".
pub fn range_short(range: (i32, i32)) -> String {
    range_text(range, |year, month| tr!("date-month-year", month = crate::locale::month_short((month as usize).saturating_sub(1)), year = year.to_string()))
}

fn range_text((a, b): (i32, i32), month: impl Fn(i32, u32) -> String) -> String {
    let ((ya, ma), (yb, mb)) = (month_of_index(a), month_of_index(b));
    if a == b {
        return month(ya, ma);
    }
    let (from, to) = if ma == 1 && mb == 12 { (ya.to_string(), yb.to_string()) } else { (month(ya, ma), month(yb, mb)) };
    if from == to { from } else { tr!("timeline-range", from = from, to = to) }
}

/// Groups the pictures, given as (id, head, date in seconds), by their head in timeline order;
/// inside a group by date then id, the same way round as the groups. `Head::Undated` is last.
pub fn month_groups(mut items: Vec<(usize, Head, i64)>, oldest_first: bool) -> Vec<Group> {
    // ascending: year, then month (0 for a year alone), then date
    let rank = |h: Head| match h {
        Head::Month { year, month } => (0, year, month),
        Head::YearOnly(year) => (0, year, 0),
        _ => (1, 0, 0),
    };
    items.sort_by(|a, b| {
        let (ra, rb) = (rank(a.1), rank(b.1));
        ra.0.cmp(&rb.0).then_with(|| {
            if ra.0 == 1 {
                return a.0.cmp(&b.0);
            }
            let o = ((ra.1, ra.2), a.2, a.0).cmp(&((rb.1, rb.2), b.2, b.0));
            if oldest_first { o } else { o.reverse() }
        })
    });
    let mut groups: Vec<Group> = vec![];
    for (id, head, _) in items {
        match groups.last_mut() {
            Some((h, ids)) if *h == head => ids.push(id),
            _ => groups.push((head, vec![id])),
        }
    }
    groups
}

impl App {
    /// A picture's place in the timeline and its date (seconds, for the order inside a month).
    pub fn timeline_slot(&self, id: usize) -> (Head, i64) {
        if let Some(d) = self.user_taken(id) {
            let at = crate::taken::seconds(&d).unwrap_or(0);
            let year = i32::from(d.year);
            return match d.precision {
                DatePrecision::Year => (Head::YearOnly(year), at),
                _ => (Head::Month { year, month: d.month.clamp(1, 12) }, at),
            };
        }
        match self.taken.get(&id).copied().filter(|&t| t >= crate::folder_dates::EARLIEST) {
            Some(t) => {
                let (year, month) = library::local_month(t);
                (Head::Month { year, month: month as u8 }, t)
            }
            None => (Head::Undated, 0),
        }
    }

    /// The picture was taken in the months of the date filter.
    pub fn in_dates(&self, id: usize) -> bool {
        let Some((a, b)) = self.filter.dates else { return true };
        match self.timeline_slot(id).0 {
            Head::Month { year, month } => (a..=b).contains(&month_index(year, u32::from(month))),
            Head::YearOnly(year) => month_index(year, 1) <= b && month_index(year, 12) >= a,
            _ => false,
        }
    }

    /// The Timeline's groups: the pictures `shown` lets through, by month.
    pub fn timeline_groups(&self, shown: impl Fn(&usize) -> bool) -> Vec<Group> {
        let items = (0..self.photos.len())
            .filter(|id| shown(id))
            .map(|id| {
                let (head, at) = self.timeline_slot(id);
                (id, head, at)
            })
            .collect();
        month_groups(items, self.store.timeline_oldest_first)
    }

    /// The month a group stands for on the scrubber: a month's, a year's (its January), a
    /// folder's first picture's.
    pub fn head_month(&self, head: Head, dates: &crate::folder_dates::FolderDates) -> Option<(i32, u32)> {
        match head {
            Head::Month { year, month } => Some((year, u32::from(month))),
            Head::YearOnly(year) => Some((year, 1)),
            Head::Folder(fi) => dates.span.get(fi).copied().flatten().map(|(first, _)| library::local_month(first)),
            _ => None,
        }
    }

    /// A date group's title: "July 2019", "1978 · month unknown", "No date".
    pub fn head_title(head: Head) -> String {
        match head {
            Head::Month { year, month } => month_title(year, u32::from(month)),
            Head::YearOnly(year) => tr!("timeline-year-only", year = year.to_string()),
            _ => tr!("timeline-no-date"),
        }
    }

    /// A month's header in the grid: its title and count, the slideshow and map buttons; a
    /// click selects its pictures (Ctrl or Shift: adds them), a right-click offers to select them
    /// or set their date. `h`: its height at the last frame.
    pub fn month_header(&mut self, ui: &mut egui::Ui, head: Head, ids: &[usize], h: f32) -> egui::Response {
        let area = Rect::from_min_size(ui.cursor().min, egui::vec2(ui.available_width(), h));
        let back = ui.interact(area, egui::Id::new(("month_head", head)), Sense::click());
        if back.clicked() {
            let m = ui.input(|i| i.modifiers);
            self.select_group(ids, m.command || m.shift);
        }
        back.context_menu(|ui| {
            if ui.button(tr!("timeline-select")).clicked() {
                self.select_group(ids, false);
                ui.close();
            }
            if ui.add(egui::Button::new(crate::color_icons::atoms(ui.ctx(), format!("{} {}", icons::CALENDAR, tr!("ctx-date")), 13.5))).clicked() {
                self.ask_date(ids);
                ui.close();
            }
        });
        let title = Self::head_title(head);
        let inner = ui.vertical(|ui| {
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                ui.label(RichText::new(&title).size(19.0).strong().color(t().text_strong));
                ui.label(RichText::new(crate::app::fmt_count(ids.len())).size(12.5).color(t().text_weak));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if crate::shell::icon_btn(ui, icons::PLAY, &tr!("folder-play-hint")).clicked() {
                        self.start_slideshow(ids.to_vec(), ids[0]);
                    }
                    if head != Head::Undated && crate::shell::icon_btn(ui, icons::MAP, &tr!("folder-map-hint")).clicked() {
                        self.show_set_on_map(title.clone(), ids);
                    }
                    if head == Head::Undated && ui.button(format!("{} {}", icons::CALENDAR, tr!("timeline-set-date"))).clicked() {
                        self.ask_date(ids);
                    }
                });
            });
            ui.add_space(4.0);
        });
        inner.response
    }

    /// Select these pictures (`add`: on top of the selection).
    fn select_group(&mut self, ids: &[usize], add: bool) {
        if !add {
            self.selection.clear();
        }
        self.selection.extend(ids.iter().copied());
        self.tray_set = None;
        self.list_pick = None;
        if let Some(&first) = ids.first() {
            self.anchor = Some(first);
            self.cursor = Some(first);
        }
    }

    /// The button that shows or hides the scrubber (`Store.scrubber_hidden`), its top right
    /// corner at `corner`. Gives where the scrubber's strip starts, under it.
    pub fn scrubber_toggle(&mut self, ui: &mut egui::Ui, corner: Pos2) -> f32 {
        let size = TOGGLE_W - 4.0;
        let rect = Rect::from_min_size(Pos2::new(corner.x - size, corner.y), egui::vec2(size, size));
        let resp = ui.interact(rect, egui::Id::new("date_scrubber_toggle"), Sense::click());
        let hidden = self.store.scrubber_hidden;
        let p = ui.painter();
        if resp.hovered() {
            p.rect_filled(rect, size / 2.0, t().surface_hover);
        }
        let glyph = if hidden { icons::CHEVRON_LEFT } else { icons::CHEVRON_RIGHT };
        crate::color_icons::paint(ui.ctx(), p, glyph, rect.center(), 16.0, t().text_weak);
        let tip = if hidden { tr!("timeline-scrubber-show") } else { tr!("timeline-scrubber-hide") };
        if resp.on_hover_cursor(egui::CursorIcon::PointingHand).on_hover_text(tip).clicked() {
            self.store.scrubber_hidden = !hidden;
            self.scrub.range_from = None;
            self.dirty = true;
        }
        rect.bottom() + 6.0
    }

    /// The scrubber in `strip`, for a grid of height `total` scrolled to `offset` with `view` of
    /// it in sight (all in points).
    pub fn scrubber(&mut self, ui: &mut egui::Ui, strip: Rect, total: f32, offset: f32, view: f32) {
        if total <= view || self.scrub.marks.is_empty() {
            return;
        }
        let y_of = |content: f32| strip.top() + (content / total).clamp(0.0, 1.0) * strip.height();
        let content_of = |y: f32| ((y - strip.top()) / strip.height()).clamp(0.0, 1.0) * total;
        let resp = ui.interact(strip, egui::Id::new("date_scrubber"), Sense::click_and_drag());
        let pointer = resp.hover_pos().or_else(|| resp.interact_pointer_pos()).filter(|p| strip.expand(2.0).contains(*p) || resp.dragged());
        let active = resp.hovered() || resp.dragged();
        let p = ui.painter();
        let strong = t().text_weak;
        let faint = t().text_weak.gamma_multiply(0.35);
        if active {
            p.rect_filled(strip, 4, t().accent.gamma_multiply(0.06));
        }
        // the part of the grid in view
        let seen = Rect::from_min_max(Pos2::new(strip.right() - 4.0, y_of(offset)), Pos2::new(strip.right() - 1.0, y_of(offset + view).max(y_of(offset) + 3.0)));
        p.rect_filled(seen, 1, t().accent.gamma_multiply(0.7));
        // a tick per month, a label where a year starts (unless too close to the last one)
        let font = egui::FontId::proportional(11.0);
        let mut last_year: Option<i32> = None;
        let mut last_label = f32::NEG_INFINITY;
        let mut last_month: Option<(i32, u32)> = None;
        for &(month, top, _) in &self.scrub.marks {
            let Some((year, m)) = month else { continue };
            let y = y_of(top);
            if last_month != Some((year, m)) {
                p.line_segment([Pos2::new(strip.right() - 10.0, y), Pos2::new(strip.right() - 6.0, y)], Stroke::new(1.0, faint));
                last_month = Some((year, m));
            }
            if last_year != Some(year) {
                last_year = Some(year);
                if y - last_label >= 15.0 {
                    p.line_segment([Pos2::new(strip.right() - 12.0, y), Pos2::new(strip.right() - 6.0, y)], Stroke::new(1.0, strong));
                    p.text(Pos2::new(strip.right() - 14.0, y), egui::Align2::RIGHT_CENTER, year.to_string(), font.clone(), strong);
                    last_label = y;
                }
            }
        }
        // pointing: the month there, beside the strip
        if let Some(pos) = pointer.filter(|_| active) {
            let at = content_of(pos.y);
            let month = self.scrub.marks.iter().take_while(|(_, top, _)| *top <= at).filter_map(|(m, _, _)| *m).last().or_else(|| self.scrub.marks.iter().find_map(|(m, _, _)| *m));
            // Shift+drag: a range of dates, shown as a band; let go, it becomes the date filter
            if resp.drag_started() && ui.input(|i| i.modifiers.shift) {
                self.scrub.range_from = month.map(|m| (m, pos.y));
            }
            if let (Some((from, y0)), Some(to)) = (self.scrub.range_from, month) {
                let band = Rect::from_x_y_ranges(strip.x_range(), y0.min(pos.y)..=y0.max(pos.y));
                p.rect_filled(band, 3, t().accent.gamma_multiply(0.25));
                if resp.drag_stopped() {
                    let (a, b) = (month_index(from.0, from.1), month_index(to.0, to.1));
                    self.filter.dates = Some((a.min(b), a.max(b)));
                    self.scrub.range_from = None;
                }
            }
            p.line_segment([Pos2::new(strip.left(), pos.y), Pos2::new(strip.right(), pos.y)], Stroke::new(1.5, t().accent));
            if let Some((year, m)) = month {
                let text = month_title(year, m);
                let galley = p.layout_no_wrap(text, egui::FontId::proportional(13.0), Color32::WHITE);
                let size = galley.size() + egui::vec2(16.0, 8.0);
                let bubble = Rect::from_min_size(Pos2::new(strip.left() - size.x - 6.0, (pos.y - size.y / 2.0).clamp(strip.top(), strip.bottom() - size.y)), size);
                p.rect_filled(bubble, 6, t().accent);
                p.galley(bubble.min + egui::vec2(8.0, 4.0), galley, Color32::WHITE);
            }
            ui.ctx().set_cursor_icon(if resp.dragged() { egui::CursorIcon::Grabbing } else { egui::CursorIcon::PointingHand });
            // the month pointed at comes to the top of the grid
            if (resp.clicked() || resp.dragged()) && self.scrub.range_from.is_none() && resp.interact_pointer_pos().is_some() {
                self.scrub.scroll_to = Some(at.min(total - view).max(0.0));
                ui.ctx().request_repaint();
            }
        }
        if active {
            resp.on_hover_text(tr!("timeline-scrubber-hint"));
        }
    }
}

/// The Dates popup's bars: `counts` (month index, pictures; ascending) as one bar per month from
/// the first to the last, the range `on` (both included) highlighted. Gives the month under the
/// pointer, if any.
fn paint_bars(ui: &egui::Ui, rect: Rect, counts: &[(i32, usize)], on: Option<(i32, i32)>) -> Option<i32> {
    let (Some(&(lo, _)), Some(&(hi, _))) = (counts.first(), counts.last()) else { return None };
    let n = (hi - lo + 1) as usize;
    let w = rect.width() / n as f32;
    let bars = Rect::from_min_max(rect.min, Pos2::new(rect.right(), rect.bottom() - 16.0));
    let max = counts.iter().map(|&(_, c)| c).max().unwrap_or(1).max(1) as f32;
    let p = ui.painter();
    p.line_segment([bars.left_bottom(), bars.right_bottom()], Stroke::new(1.0, t().border));
    for &(m, c) in counts {
        let k = (m - lo) as f32;
        // a square root, so that a quiet month still shows beside a busy one
        let h = ((c as f32 / max).sqrt() * (bars.height() - 2.0)).max(2.0);
        let x = bars.left() + k * w;
        let bar = Rect::from_min_max(Pos2::new(x, bars.bottom() - h), Pos2::new(x + (w - 1.0).max(1.0), bars.bottom()));
        let inside = on.is_none_or(|(a, b)| (a..=b).contains(&m));
        p.rect_filled(bar, 1, if inside { t().accent } else { t().text_weak.gamma_multiply(0.35) });
    }
    // the years under the bars, where they start (not closer than 34 points)
    let font = egui::FontId::proportional(11.0);
    let mut last = f32::NEG_INFINITY;
    for k in 0..n {
        let (year, month) = month_of_index(lo + k as i32);
        let x = bars.left() + k as f32 * w;
        if (month == 1 || k == 0) && x - last >= 34.0 {
            p.line_segment([Pos2::new(x, bars.bottom()), Pos2::new(x, bars.bottom() + 3.0)], Stroke::new(1.0, t().text_weak));
            p.text(Pos2::new(x, bars.bottom() + 4.0), egui::Align2::LEFT_TOP, year.to_string(), font.clone(), t().text_weak);
            last = x;
        }
    }
    let x = ui.ctx().pointer_latest_pos().filter(|p| rect.contains(*p))?.x;
    Some(lo + (((x - rect.left()) / w).floor() as i32).clamp(0, n as i32 - 1))
}

impl App {
    /// The top bar's Dates chip: "Dates", or the range shown (highlighted); a click opens the
    /// popup that chooses it (`dates_popup`).
    /// `wide`: the top bar's width (a range needs more room than "Dates").
    pub fn dates_chip(&mut self, ui: &mut egui::Ui, wide: f32) {
        // narrow: the icon alone (highlighted when a range is on), the range in the tip
        let label = match self.filter.dates {
            Some(range) if wide > 1100.0 => format!("{}  {}", icons::CALENDAR, range_short(range)),
            None if wide > 900.0 => format!("{}  {}", icons::CALENDAR, tr!("filter-dates")),
            _ => icons::CALENDAR.to_string(),
        };
        let hint = match self.filter.dates {
            Some(range) => format!("{}\n{}", range_title(range), tr!("timeline-range-hint")),
            None => tr!("filter-dates-hint"),
        };
        let resp = crate::shell::chip(ui, &label, self.filter.dates.is_some()).on_hover_text(hint);
        let mut popup = egui::Popup::menu(&resp).close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside);
        if self.scrub.show_popup {
            popup = popup.open_memory(Some(egui::SetOpenCommand::Bool(true)));
        }
        popup.show(|ui| self.dates_popup(ui));
    }

    /// The months of what the list shows as bars (drag across them, or click one) and its years
    /// as buttons (Shift adds a year to the range); All dates clears the range.
    fn dates_popup(&mut self, ui: &mut egui::Ui) {
        ui.set_width(380.0);
        ui.label(RichText::new(tr!("dates-title")).strong().color(t().text_strong));
        ui.label(RichText::new(self.filter.dates.map(range_title).unwrap_or_else(|| tr!("dates-all"))).size(12.5).color(t().text_weak));
        ui.add_space(6.0);
        let counts = self.date_counts();
        if counts.is_empty() {
            ui.label(RichText::new(tr!("dates-none")).color(t().text_weak));
            return;
        }
        let (rect, resp) = ui.allocate_exact_size(egui::vec2(380.0, 86.0), Sense::click_and_drag());
        let under = paint_bars(ui, rect, &counts, self.filter.dates);
        if let Some(m) = under {
            if resp.drag_started() {
                self.scrub.bars_from = Some(m);
            }
            if let Some(from) = self.scrub.bars_from.filter(|_| resp.dragged()) {
                self.filter.dates = Some((from.min(m), from.max(m)));
            }
            if resp.clicked() {
                self.filter.dates = Some((m, m));
            }
            let n = counts.iter().find(|(k, _)| *k == m).map(|&(_, c)| c).unwrap_or(0);
            let (year, month) = month_of_index(m);
            resp.on_hover_text_at_pointer(format!("{} · {}", month_title(year, month), tr!("count-pictures", n = n)));
        }
        if !ui.input(|i| i.pointer.any_down()) {
            self.scrub.bars_from = None;
        }
        ui.add_space(6.0);
        // the years, newest first
        let mut years: Vec<i32> = counts.iter().map(|&(m, _)| month_of_index(m).0).collect();
        years.dedup();
        years.reverse();
        ui.horizontal_wrapped(|ui| {
            for y in years {
                let whole = (month_index(y, 1), month_index(y, 12));
                if ui.selectable_label(self.filter.dates == Some(whole), y.to_string()).clicked() {
                    let add = ui.input(|i| i.modifiers.shift);
                    self.filter.dates = match self.filter.dates {
                        Some((a, b)) if add => Some((a.min(whole.0), b.max(whole.1))),
                        _ => Some(whole),
                    };
                }
            }
        });
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            if ui.add_enabled(self.filter.dates.is_some(), egui::Button::new(tr!("dates-all"))).clicked() {
                self.filter.dates = None;
            }
        });
        ui.label(RichText::new(tr!("dates-hint")).size(12.0).color(t().text_weak));
    }
}

/// "July 2019" (month 1…12).
pub fn month_title(year: i32, month: u32) -> String {
    tr!("date-month-year", month = crate::locale::month_long((month as usize).saturating_sub(1)), year = year.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn heads(groups: &[Group]) -> Vec<Head> {
        groups.iter().map(|(h, _)| *h).collect()
    }

    const fn m(year: i32, month: u8) -> Head {
        Head::Month { year, month }
    }

    #[test]
    fn months_newest_first_year_alone_after_its_months_undated_last() {
        let items = vec![
            (0, m(2019, 7), 300),
            (1, Head::Undated, 0),
            (2, m(2019, 7), 100),
            (3, Head::YearOnly(2019), 50),
            (4, m(2021, 1), 900),
            (5, m(2019, 2), 10),
            (6, Head::Undated, 0),
        ];
        let g = month_groups(items.clone(), false);
        assert_eq!(heads(&g), vec![m(2021, 1), m(2019, 7), m(2019, 2), Head::YearOnly(2019), Head::Undated]);
        // inside a month the same way round as the months: newest first
        assert_eq!(g[1].1, vec![0, 2]);
        assert_eq!(g[4].1, vec![1, 6]);

        let g = month_groups(items, true);
        assert_eq!(heads(&g), vec![Head::YearOnly(2019), m(2019, 2), m(2019, 7), m(2021, 1), Head::Undated]);
        assert_eq!(g[2].1, vec![2, 0]);
        assert_eq!(g[4].1, vec![1, 6]);
    }

    #[test]
    fn month_numbers_go_both_ways() {
        for (y, m) in [(1978, 1), (2019, 7), (2024, 12), (-1, 3)] {
            assert_eq!(month_of_index(month_index(y, m)), (y, m));
        }
        assert_eq!(month_index(2019, 12) + 1, month_index(2020, 1));
    }

    #[test]
    fn same_date_keeps_the_ids_in_order() {
        let g = month_groups(vec![(5, m(2020, 3), 7), (2, m(2020, 3), 7), (9, m(2020, 3), 7)], true);
        assert_eq!(g[0].1, vec![2, 5, 9]);
        assert!(month_groups(vec![], false).is_empty());
    }
}
