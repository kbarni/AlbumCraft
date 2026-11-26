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
pub fn range_title((a, b): (i32, i32)) -> String {
    let ((ya, ma), (yb, mb)) = (month_of_index(a), month_of_index(b));
    if a == b {
        return month_title(ya, ma);
    }
    let (from, to) = if ma == 1 && mb == 12 { (ya.to_string(), yb.to_string()) } else { (month_title(ya, ma), month_title(yb, mb)) };
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
    fn same_date_keeps_the_ids_in_order() {
        let g = month_groups(vec![(5, m(2020, 3), 7), (2, m(2020, 3), 7), (9, m(2020, 3), 7)], true);
        assert_eq!(g[0].1, vec![2, 5, 9]);
        assert!(month_groups(vec![], false).is_empty());
    }
}
