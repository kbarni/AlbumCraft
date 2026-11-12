//! How dates, numbers and sizes are written. The core has no language of its own: the window
//! sets the `Locale` (built from its translation files) when the language changes, and the
//! formatting functions here (`library::fmt_date`, `fmt_bytes`, `exifinfo::trim_num`…) use it.
//! Without a window (`--mcp`) it stays English.

use std::sync::{LazyLock, RwLock};

#[derive(Clone, Debug, PartialEq)]
pub struct Locale {
    pub months_short: Vec<String>,
    pub months_long: Vec<String>,
    /// How a date reads, with `{month}` (short name), `{day}` and `{year}`.
    pub date: String,
    /// The decimal separator and the one between groups of three digits.
    pub decimal: String,
    pub group: String,
    /// Kilo-, mega- and gigabytes.
    pub units: [String; 3],
}

impl Default for Locale {
    fn default() -> Self {
        let words = |s: &str| s.split(',').map(|w| w.trim().to_string()).collect();
        Locale {
            months_short: words("Jan, Feb, Mar, Apr, May, Jun, Jul, Aug, Sep, Oct, Nov, Dec"),
            months_long: words("January, February, March, April, May, June, July, August, September, October, November, December"),
            date: "{month} {day}, {year}".into(),
            decimal: ".".into(),
            group: ",".into(),
            units: ["KB".into(), "MB".into(), "GB".into()],
        }
    }
}

static CURRENT: LazyLock<RwLock<Locale>> = LazyLock::new(|| RwLock::new(Locale::default()));

/// Use `locale` from now on (a list of fewer than 12 months is ignored: the English one stays).
pub fn set(mut locale: Locale) {
    let d = Locale::default();
    if locale.months_short.len() != 12 {
        locale.months_short = d.months_short;
    }
    if locale.months_long.len() != 12 {
        locale.months_long = d.months_long;
    }
    if let Ok(mut c) = CURRENT.write() {
        *c = locale;
    }
}

fn with<T>(f: impl FnOnce(&Locale) -> T) -> T {
    match CURRENT.read() {
        Ok(l) => f(&l),
        Err(_) => f(&Locale::default()),
    }
}

/// A number written with `.` as its decimal point, with the locale's separator instead.
pub fn decimal(s: &str) -> String {
    with(|l| l.decimal_of(s))
}

/// `n` with the locale's separator between groups of three digits: "1,234,567".
pub fn group(n: u64) -> String {
    with(|l| l.group_of(n))
}

/// "May 12, 2009" (month 1…12).
pub fn date(year: i32, month: u32, day: u32) -> String {
    with(|l| l.date_of(year, month, day))
}

impl Locale {
    fn decimal_of(&self, s: &str) -> String {
        if self.decimal == "." { s.to_string() } else { s.replace('.', &self.decimal) }
    }

    fn group_of(&self, n: u64) -> String {
        let s = n.to_string();
        let mut out = String::new();
        for (i, c) in s.chars().enumerate() {
            if i > 0 && (s.len() - i).is_multiple_of(3) {
                out.push_str(&self.group);
            }
            out.push(c);
        }
        out
    }

    fn date_of(&self, year: i32, month: u32, day: u32) -> String {
        let m = self.months_short.get((month as usize).saturating_sub(1)).cloned().unwrap_or_default();
        self.date.replace("{month}", &m).replace("{day}", &day.to_string()).replace("{year}", &year.to_string())
    }
}

/// The full name of a month (0…11).
pub fn month_long(month0: usize) -> String {
    with(|l| l.months_long.get(month0).cloned().unwrap_or_default())
}

/// "KB", "MB" or "GB" (0, 1, 2).
pub fn unit(i: usize) -> String {
    with(|l| l.units.get(i).cloned().unwrap_or_default())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn english_by_default() {
        let en = Locale::default();
        assert_eq!(en.date_of(2009, 5, 12), "May 12, 2009");
        assert_eq!(en.group_of(1_234_567), "1,234,567");
        assert_eq!(en.group_of(999), "999");
        assert_eq!(en.decimal_of("2.5"), "2.5");
        assert_eq!(en.months_long[3], "April");
    }

    #[test]
    fn another_locale_changes_everything() {
        let mut fr = Locale { date: "{day} {month} {year}".into(), decimal: ",".into(), group: "\u{a0}".into(), ..Locale::default() };
        fr.months_short[4] = "mai".into();
        assert_eq!(fr.date_of(2009, 5, 12), "12 mai 2009");
        assert_eq!(fr.group_of(1234), "1\u{a0}234");
        assert_eq!(fr.decimal_of("2.5"), "2,5");
    }
}
