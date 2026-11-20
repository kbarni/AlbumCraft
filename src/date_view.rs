//! When pictures were taken, in the window: the date AlbumCraft uses for a picture
//! (`App::taken_at`: the user's, `Meta.taken`, else the file's), how it is shown (`date_text`:
//! "1978", "July 1978", a day, a day and time) and the Date taken dialog (`Dialog::DateTaken`,
//! from the picture menu and the info panel): one date for all the pictures (a year or a month
//! alone for scanned prints), or every date moved by the same amount (a camera clock that was
//! off: the first picture's real date is typed, the others follow), or back to the files' own
//! dates. Each change is an `Undo::Dates` notice. The files are never changed.

use crate::app::*;
use crate::i18n::tr;
use crate::theme::t;
use crate::taken::{DatePrecision, DateTime};
use eframe::egui::{self, RichText};
use std::collections::HashMap;

/// What the dialog is doing.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum DateMode {
    /// The same date for every picture.
    Set,
    /// Every picture's date moved as much as the first one's.
    Shift,
}

/// The Date taken dialog's state (by key: a scan may come in between).
#[derive(Clone, Debug)]
pub struct DateDialog {
    pub keys: Vec<String>,
    pub mode: DateMode,
    pub year: String,
    /// 0: not known, 1–12.
    pub month: usize,
    pub day: String,
    /// "10:30" or empty.
    pub time: String,
    /// For Shift: the picture whose real date is typed, and its date now (Unix seconds).
    pub reference: Option<(String, i64)>,
    pub error: String,
}

/// A date as shown: the year, the month and year, the day, or the day and time.
pub fn date_text(d: &DateTime) -> String {
    match d.precision {
        DatePrecision::Year => d.year.to_string(),
        DatePrecision::Month => tr!("date-month-year", month = crate::locale::month_long(usize::from(d.month.saturating_sub(1))), year = d.year.to_string()),
        DatePrecision::Full => {
            let day = crate::locale::date(i32::from(d.year), u32::from(d.month), u32::from(d.day));
            if crate::taken::has_time(d) { format!("{day} {:02}:{:02}", d.hour, d.minute) } else { day }
        }
    }
}

/// "+1 year 2 days 3 h 10 min", how far a shift moves the pictures.
fn shift_text(secs: i64) -> String {
    let sign = if secs < 0 { "-" } else { "+" };
    let mut s = secs.unsigned_abs();
    let days = s / 86_400;
    s %= 86_400;
    let (years, days) = (days / 365, days % 365);
    let (h, m, sec) = (s / 3600, (s % 3600) / 60, s % 60);
    let mut parts = vec![];
    if years > 0 {
        parts.push(tr!("date-shift-years", n = years));
    }
    if days > 0 {
        parts.push(tr!("date-shift-days", n = days));
    }
    if h > 0 {
        parts.push(format!("{h} h"));
    }
    if m > 0 {
        parts.push(format!("{m} min"));
    }
    if sec > 0 {
        parts.push(format!("{sec} s"));
    }
    if parts.is_empty() {
        return tr!("date-shift-none");
    }
    format!("{sign}{}", parts.join(" "))
}

impl App {
    /// The date the user set for a picture.
    pub fn user_taken(&self, id: usize) -> Option<DateTime> {
        self.meta(id).and_then(|m| m.taken)
    }

    /// When a picture was taken as AlbumCraft uses it (Unix seconds): the date the user set,
    /// else the file's (`App.taken`, the file's modification date when it has none).
    pub fn taken_at(&self, id: usize) -> Option<i64> {
        self.user_taken(id).and_then(|d| crate::taken::seconds(&d)).or_else(|| self.taken.get(&id).copied())
    }

    /// The file's own date when it has one (not the date of the file itself).
    fn file_taken(&self, id: usize) -> Option<i64> {
        let p = self.photos.get(id)?;
        self.taken.get(&id).copied().filter(|&t| t != p.modified)
    }

    /// Open the Date taken dialog for these pictures, filled with the first one's date.
    pub fn ask_date(&mut self, ids: &[usize]) {
        let ids: Vec<usize> = ids.iter().copied().filter(|&id| self.photos.get(id).is_some_and(|p| !p.is_sample())).collect();
        let Some(&first) = ids.first() else { return };
        let keys = ids.iter().filter_map(|&id| self.photos.get(id).map(|p| p.key.clone())).collect();
        let now = self.user_taken(first).or_else(|| self.file_taken(first).and_then(crate::taken::from_seconds));
        // the first picture with an exact date is the one whose real date is typed to shift
        let reference = ids.iter().find_map(|&id| {
            let at = match self.user_taken(id) {
                Some(d) => d.is_full().then(|| crate::taken::seconds(&d)).flatten(),
                None => self.file_taken(id),
            };
            Some((self.photos.get(id)?.key.clone(), at?))
        });
        let mut st = DateDialog { keys, mode: DateMode::Set, year: String::new(), month: 0, day: String::new(), time: String::new(), reference, error: String::new() };
        if let Some(d) = now {
            fill(&mut st, &d);
        }
        self.dialog = Some(Dialog::DateTaken(st));
    }

    /// Change the dates of these pictures (by key) and offer to undo it.
    fn apply_dates(&mut self, change: impl FnOnce(&mut Self, &[usize]) -> usize, keys: &[String]) {
        let ids: Vec<usize> = keys.iter().filter_map(|k| self.key_index.get(k).copied()).collect();
        let before: Vec<(String, Option<DateTime>)> = keys.iter().map(|k| (k.clone(), self.store.meta.get(k).and_then(|m| m.taken))).collect();
        let changed = change(self, &ids);
        self.dates_changed();
        let left = ids.len().saturating_sub(changed);
        let mut text = tr!("date-changed", n = changed);
        if left > 0 {
            text = format!("{text} {}", tr!("date-left-alone", left = left));
        }
        self.undoable(text, Undo::Dates(before));
    }

    /// The dates changed: the order by date, the folders' dates and the counts follow.
    pub fn dates_changed(&mut self) {
        self.dirty = true;
        self.list_rev += 1;
        self.taken_rev += 1;
    }

    /// Put back the dates an `Undo::Dates` holds.
    pub fn undo_dates(&mut self, before: Vec<(String, Option<DateTime>)>) {
        for (key, d) in before {
            if let Some(m) = self.store.meta.get_mut(&key) {
                m.taken = d;
            } else if d.is_some() {
                self.store.meta.entry(key).or_default().taken = d;
            }
        }
        self.dates_changed();
    }

    pub(crate) fn date_dialog(&mut self, ctx: &egui::Context, mut st: DateDialog) {
        let mut close = false;
        let enter = ctx.input(|i| i.key_pressed(egui::Key::Enter));
        let n = st.keys.len();
        let ids: Vec<usize> = st.keys.iter().filter_map(|k| self.key_index.get(k).copied()).collect();
        let any_set = ids.iter().any(|&id| self.user_taken(id).is_some());
        #[allow(deprecated)]
        let modal = egui::Modal::new(egui::Id::new("date-taken")).show(ctx, |ui| {
            ui.set_width(420.0);
            ui.heading(tr!("date-title"));
            ui.add_space(6.0);
            ui.label(RichText::new(tr!("date-intro", n = n)).color(t().text_weak));
            ui.add_space(6.0);
            if n > 1 && st.reference.is_some() {
                let before = st.mode;
                ui.radio_value(&mut st.mode, DateMode::Set, tr!("date-mode-set"));
                ui.radio_value(&mut st.mode, DateMode::Shift, tr!("date-mode-shift"));
                if st.mode != before {
                    st.error.clear();
                    if st.mode == DateMode::Shift
                        && let Some(d) = st.reference.as_ref().and_then(|(_, at)| crate::taken::from_seconds(*at))
                    {
                        fill(&mut st, &d);
                    }
                }
                ui.add_space(4.0);
            }
            let shift = st.mode == DateMode::Shift;
            if shift && let Some((key, at)) = &st.reference {
                let name = key.rsplit(['/', '\\']).next().unwrap_or(key);
                let now = crate::taken::from_seconds(*at).map(|d| date_text(&d)).unwrap_or_default();
                ui.label(tr!("date-shift-intro", name = name, date = now));
                ui.add_space(4.0);
            }
            fields(ui, &mut st);
            let typed = typed(&st, shift);
            ui.add_space(4.0);
            match (&typed, shift, &st.reference) {
                (Ok(d), true, Some((_, at))) => {
                    let by = crate::taken::seconds(d).map(|s| s - at).unwrap_or(0);
                    ui.label(RichText::new(tr!("date-shift-by", by = shift_text(by))).color(t().text_weak));
                }
                (Ok(_), _, _) => {
                    ui.label(RichText::new(tr!("date-unknown-parts")).size(12.0).color(t().text_weak));
                }
                (Err(e), _, _) if !st.year.trim().is_empty() => {
                    ui.label(RichText::new(e.as_str()).color(t().bad));
                }
                _ => {}
            }
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                let ok = typed.is_ok();
                if (ui.add_enabled(ok, egui::Button::new(tr!("date-apply"))).clicked() || (enter && ok))
                    && let Ok(d) = &typed
                {
                    let keys = st.keys.clone();
                    match (shift, &st.reference) {
                        (true, Some((_, at))) => {
                            let by = crate::taken::seconds(d).map(|s| s - at).unwrap_or(0);
                            let files: HashMap<String, i64> = ids.iter().filter_map(|&id| Some((self.photos.get(id)?.key.clone(), self.file_taken(id)?))).collect();
                            self.apply_dates(|app, ids| crate::commands::shift_taken(app, ids, by, &|k: &str| files.get(k).copied()), &keys);
                        }
                        _ => {
                            let d = *d;
                            self.apply_dates(
                                |app, ids| {
                                    crate::commands::set_taken(app, ids, Some(d));
                                    ids.len()
                                },
                                &keys,
                            );
                        }
                    }
                    close = true;
                }
                if any_set && ui.button(tr!("date-reset")).on_hover_text(tr!("date-reset-hint")).clicked() {
                    let keys = st.keys.clone();
                    self.apply_dates(
                        |app, ids| {
                            crate::commands::set_taken(app, ids, None);
                            ids.len()
                        },
                        &keys,
                    );
                    close = true;
                }
                if ui.button(tr!("button-cancel")).clicked() {
                    close = true;
                }
            });
        });
        if self.dialog.is_none() && !close && !modal.should_close() {
            self.dialog = Some(Dialog::DateTaken(st));
        }
    }
}

/// The fields filled with a date.
fn fill(st: &mut DateDialog, d: &DateTime) {
    st.year = d.year.to_string();
    st.month = if d.precision == DatePrecision::Year { 0 } else { usize::from(d.month) };
    st.day = if d.is_full() { d.day.to_string() } else { String::new() };
    st.time = if crate::taken::has_time(d) { format!("{:02}:{:02}", d.hour, d.minute) } else { String::new() };
}

/// Year, month, day and time; the month, day and time may stay empty (shifting needs the day:
/// `typed` says so).
fn fields(ui: &mut egui::Ui, st: &mut DateDialog) {
    egui::Grid::new("date-fields").num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
        ui.label(tr!("date-year"));
        ui.add(egui::TextEdit::singleline(&mut st.year).desired_width(60.0).char_limit(4));
        ui.end_row();
        ui.label(tr!("date-month"));
        let month_name = |m: usize| if m == 0 { tr!("date-unknown") } else { crate::locale::month_long(m - 1) };
        egui::ComboBox::from_id_salt("date-month").selected_text(month_name(st.month)).width(140.0).show_ui(ui, |ui| {
            for m in 0..=12 {
                ui.selectable_value(&mut st.month, m, month_name(m));
            }
        });
        ui.end_row();
        ui.label(tr!("date-day"));
        ui.add_enabled(st.month > 0, egui::TextEdit::singleline(&mut st.day).desired_width(40.0).char_limit(2));
        ui.end_row();
        ui.label(tr!("date-time"));
        ui.horizontal(|ui| {
            ui.add_enabled(st.month > 0 && !st.day.trim().is_empty(), egui::TextEdit::singleline(&mut st.time).desired_width(60.0).char_limit(8).hint_text("10:30"));
            ui.label(RichText::new(tr!("date-time-hint")).size(12.0).color(t().text_weak));
        });
        ui.end_row();
    });
}

/// The date the fields say, or why they don't say one.
fn typed(st: &DateDialog, shift: bool) -> Result<DateTime, String> {
    let num = |s: &str| s.trim().parse::<u16>().ok();
    let year = num(&st.year).ok_or_else(|| tr!("date-bad-year"))?;
    let month = (st.month > 0).then_some(st.month as u8);
    let day = match (month, st.day.trim()) {
        (None, _) | (_, "") => None,
        (Some(_), d) => Some(d.parse::<u8>().map_err(|_| tr!("date-bad-day"))?),
    };
    let time = match (day, st.time.trim()) {
        (None, _) | (_, "") => None,
        (Some(_), t) => Some(parse_time(t).ok_or_else(|| tr!("date-bad-time"))?),
    };
    if shift && day.is_none() {
        return Err(tr!("date-shift-needs-day"));
    }
    crate::taken::of_parts(year, month, day, time).map_err(|_| tr!("date-bad-day"))
}

/// "10:30", "10:30:05" or "10".
fn parse_time(t: &str) -> Option<(u8, u8, u8)> {
    let mut parts = t.split([':', '.', 'h']).map(str::trim);
    let h: u8 = parts.next()?.parse().ok()?;
    let m: u8 = parts.next().filter(|s| !s.is_empty()).map_or(Some(0), |s| s.parse().ok())?;
    let s: u8 = parts.next().filter(|s| !s.is_empty()).map_or(Some(0), |s| s.parse().ok())?;
    (parts.next().is_none() && h < 24 && m < 60 && s < 60).then_some((h, m, s))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn st(year: &str, month: usize, day: &str, time: &str) -> DateDialog {
        DateDialog { keys: vec![], mode: DateMode::Set, year: year.into(), month, day: day.into(), time: time.into(), reference: None, error: String::new() }
    }

    #[test]
    fn fields_make_dates_of_any_precision() {
        let iso = |s: &DateDialog, shift| typed(s, shift).map(|d| d.to_iso());
        assert_eq!(iso(&st("1978", 0, "14", "10:30"), false).as_deref(), Ok("1978"));
        assert_eq!(iso(&st("1978", 7, "", ""), false).as_deref(), Ok("1978-07"));
        assert_eq!(iso(&st(" 1978 ", 7, "14", ""), false).as_deref(), Ok("1978-07-14T00:00:00"));
        assert_eq!(iso(&st("1978", 7, "14", "9:05"), false).as_deref(), Ok("1978-07-14T09:05:00"));
        assert_eq!(iso(&st("1978", 7, "14", "21h15"), false).as_deref(), Ok("1978-07-14T21:15:00"));
        for bad in [st("", 0, "", ""), st("78x", 0, "", ""), st("1600", 0, "", ""), st("1978", 2, "30", ""), st("1978", 7, "14", "25:00"), st("1978", 7, "x", "")] {
            assert!(typed(&bad, false).is_err(), "{bad:?}");
        }
        // shifting needs the day
        assert!(typed(&st("1978", 7, "", ""), true).is_err());
    }

    #[test]
    fn shown_as_known() {
        assert_eq!(date_text(&crate::taken::parse("1978").unwrap()), "1978");
        assert_eq!(date_text(&crate::taken::parse("1978-07").unwrap()), "July 1978");
        assert_eq!(date_text(&crate::taken::parse("1978-07-14").unwrap()), "Jul 14, 1978");
        assert_eq!(date_text(&crate::taken::parse("1978-07-14T09:05").unwrap()), "Jul 14, 1978 09:05");
        let mut back = st("", 0, "", "");
        fill(&mut back, &crate::taken::parse("1978-07").unwrap());
        assert_eq!((back.year.as_str(), back.month, back.day.as_str()), ("1978", 7, ""));
    }

    #[test]
    fn shifts_read_plainly() {
        assert_eq!(shift_text(3600), "+1 h");
        assert_eq!(shift_text(-(366 * 86_400 + 90)), "-1 year 1 day 1 min 30 s");
        assert_eq!(shift_text(0), "no change");
    }
}
