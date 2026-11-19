//! Folders dated by their pictures: each folder's first and last date taken (`App.taken`, the
//! file's date for a picture whose date isn't known yet), shown in the grid's folder header as
//! a range, and the order of the folders (newest first photo first) for the grid and the
//! sidebar's years.
//!
//! Dates arrive in batches while the library is read for the first time (`taken_rev`, at most
//! once a second): the order is worked out again then, and the grid keeps the folder in view
//! where it was (`App.grid_anchor`).

use crate::app::App;
use crate::i18n::tr;
use crate::library;
use std::rc::Rc;

/// Dates before this (1 January 1980) come from a camera whose clock was never set: left out.
pub const EARLIEST: i64 = 315_532_800;

/// See `App::folder_dates`.
#[derive(Debug, Default, PartialEq)]
pub struct FolderDates {
    /// By folder index: the first and last date taken, None when no picture has a usable date.
    pub span: Vec<Option<(i64, i64)>>,
    /// Folder indexes, the newest first photo first.
    pub order: Vec<usize>,
}

/// The span of these dates, leaving out the ones before `EARLIEST`.
pub fn span_of(dates: impl IntoIterator<Item = i64>) -> Option<(i64, i64)> {
    dates.into_iter().filter(|&t| t >= EARLIEST).fold(None, |acc, t| match acc {
        None => Some((t, t)),
        Some((a, b)) => Some((a.min(t), b.max(t))),
    })
}

/// The folders' order: by first date (`fallback`, the folder's own date, for one without a
/// span), newest first; the same date by name.
pub fn order_of(span: &[Option<(i64, i64)>], fallback: &[i64], names: &[&str]) -> Vec<usize> {
    let mut order: Vec<usize> = (0..span.len()).collect();
    let key = |fi: usize| span[fi].map(|(a, _)| a).or(fallback.get(fi).copied()).unwrap_or(0);
    order.sort_by(|&a, &b| key(b).cmp(&key(a)).then_with(|| names.get(a).cmp(&names.get(b))));
    order
}

/// "May 12, 2009", or "May 12, 2009 – June 3, 2009" when the pictures were taken on different
/// days.
pub fn span_text((first, last): (i64, i64)) -> String {
    let (a, b) = (library::fmt_date(first), library::fmt_date(last));
    if a == b { a } else { tr!("folder-dates", first = a, last = b) }
}

impl App {
    /// The folders' dates and order, made again when the library or the dates taken change.
    pub fn folder_dates(&self) -> Rc<FolderDates> {
        let key = (self.list_rev, self.taken_rev);
        let mut cache = self.folder_dates_cache.borrow_mut();
        if let Some((k, d)) = cache.as_ref()
            && *k == key
        {
            return d.clone();
        }
        let span: Vec<Option<(i64, i64)>> = self
            .folders
            .iter()
            .map(|f| span_of(f.photos.iter().filter_map(|&id| self.taken.get(&id).copied().or_else(|| Some(self.photos.get(id)?.modified)))))
            .collect();
        let fallback: Vec<i64> = self.folders.iter().map(|f| f.date).collect();
        let names: Vec<&str> = self.folders.iter().map(|f| f.name.as_str()).collect();
        let order = order_of(&span, &fallback, &names);
        let new = FolderDates { span, order };
        if cache.as_ref().is_some_and(|(_, d)| d.order != new.order) {
            self.folder_order_rev.set(self.folder_order_rev.get() + 1);
        }
        let d = Rc::new(new);
        *cache = Some((key, d.clone()));
        d
    }

    /// What the folder's header says of its date: the span of its pictures, else the date of
    /// its newest file.
    pub fn folder_date_text(&self, fi: usize) -> String {
        match self.folder_dates().span.get(fi).copied().flatten() {
            Some(span) => span_text(span),
            None => self.folders.get(fi).map(|f| library::fmt_date(f.date)).unwrap_or_default(),
        }
    }

    /// The year a folder is listed under in the sidebar: its first picture's.
    pub fn folder_year(&self, fi: usize) -> i32 {
        let d = self.folder_dates();
        let t = d.span.get(fi).copied().flatten().map(|(a, _)| a).or_else(|| self.folders.get(fi).map(|f| f.date)).unwrap_or(0);
        library::local_year(t)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DAY: i64 = 86_400;
    const Y2020: i64 = 1_577_836_800;

    #[test]
    fn span_leaves_out_unset_clocks() {
        assert_eq!(span_of([Y2020 + DAY, 0, Y2020, EARLIEST - DAY]), Some((Y2020, Y2020 + DAY)));
        assert_eq!(span_of([0, 100]), None);
        assert_eq!(span_of([]), None);
    }

    #[test]
    fn order_by_first_date_newest_first() {
        // b's first picture is the newest even though a has a later last one
        let span = [Some((Y2020, Y2020 + 300 * DAY)), Some((Y2020 + 10 * DAY, Y2020 + 11 * DAY)), None, Some((Y2020, Y2020))];
        let fallback = [0, 0, Y2020 + 5 * DAY, 0];
        let names = ["a", "b", "c", "0"];
        assert_eq!(order_of(&span, &fallback, &names), vec![1, 2, 3, 0]);
    }

    #[test]
    fn one_day_shows_one_date() {
        let t = Y2020 + 12 * 3600;
        assert_eq!(span_text((t, t + 3600)), library::fmt_date(t));
        let text = span_text((t, t + 40 * DAY));
        assert!(text.contains(&library::fmt_date(t)) && text.contains(&library::fmt_date(t + 40 * DAY)), "{text}");
    }
}
