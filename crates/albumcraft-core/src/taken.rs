//! When a picture was taken, as the user set it (`Meta.taken`): for scanned prints, or a camera
//! whose clock was wrong. It overrides the file's own date everywhere in AlbumCraft (sorting,
//! folder dates, the info panel), goes into the XMP when the notes are written there, and into
//! exported copies' EXIF. The file itself is never changed.
//!
//! A date may be known only to the year (`1978`) or the month (`1978-07`), as is common for
//! scans: `craft_metadata::DateTime` keeps that precision, reading the missing parts as the
//! 1st and midnight. Times are the local time of the place, with no time zone, as cameras record
//! them.

pub use craft_metadata::{DatePrecision, DateTime};

/// The earliest year accepted (before photography) and the latest.
pub const FIRST_YEAR: u16 = 1826;
pub const LAST_YEAR: u16 = 9999;

/// A date typed by the user or given to a command: `1978`, `1978-07`, `1978-07-14`,
/// `1978-07-14T10:30` or `1978-07-14T10:30:05` (also the EXIF form `1978:07:14 10:30:05`). A
/// time zone, if any, is dropped (the time stays as written).
pub fn parse(text: &str) -> Result<DateTime, String> {
    let t = text.trim();
    let d = DateTime::parse_iso(t).or_else(|| DateTime::parse_exif(t)).ok_or_else(|| crate::msg!("taken-bad-date", "{text} is not a date (write it as 1978, 1978-07, 1978-07-14 or 1978-07-14T10:30)", text = t))?;
    checked(DateTime { offset_minutes: None, millis: 0, ..d })
}

/// The date if its year is in range.
pub fn checked(d: DateTime) -> Result<DateTime, String> {
    if (FIRST_YEAR..=LAST_YEAR).contains(&d.year) {
        Ok(d)
    } else {
        Err(crate::msg!("taken-bad-year", "the year must be between {first} and {last}", first = FIRST_YEAR, last = LAST_YEAR).to_string())
    }
}

/// A date of the given precision: `month` and `day` are ignored beyond it; Err for a day that
/// doesn't exist (February 30) or a year out of range.
pub fn of_parts(year: u16, month: Option<u8>, day: Option<u8>, time: Option<(u8, u8, u8)>) -> Result<DateTime, String> {
    let text = match (month, day) {
        (None, _) => format!("{year:04}"),
        (Some(m), None) => format!("{year:04}-{m:02}"),
        (Some(m), Some(d)) => match time {
            None => format!("{year:04}-{m:02}-{d:02}"),
            Some((h, mi, s)) => format!("{year:04}-{m:02}-{d:02}T{h:02}:{mi:02}:{s:02}"),
        },
    };
    parse(&text)
}

/// The date as Unix seconds of this computer's time zone, like the files' dates are read
/// (`filemeta::local_seconds`): the start of the year or month when only that is known.
pub fn seconds(d: &DateTime) -> Option<i64> {
    crate::filemeta::local_seconds(d)
}

/// Unix seconds as a full date and time in this computer's time zone.
pub fn from_seconds(secs: i64) -> Option<DateTime> {
    use chrono::{Datelike, Timelike};
    let t = chrono::DateTime::from_timestamp(secs, 0)?.with_timezone(&chrono::Local).naive_local();
    let d = DateTime {
        year: u16::try_from(t.year()).ok()?,
        month: t.month() as u8,
        day: t.day() as u8,
        hour: t.hour() as u8,
        minute: t.minute() as u8,
        second: t.second() as u8,
        ..Default::default()
    };
    checked(d).ok()
}

/// A full date moved by `secs` (a camera clock that was off); None for a year or a month alone
/// (there is nothing exact to move) or a result out of range.
pub fn shifted(d: &DateTime, secs: i64) -> Option<DateTime> {
    use chrono::{Datelike, Timelike};
    if !d.is_full() {
        return None;
    }
    let naive = chrono::NaiveDate::from_ymd_opt(d.year.into(), d.month.into(), d.day.into())?.and_hms_opt(d.hour.into(), d.minute.into(), d.second.into())?;
    let t = naive.checked_add_signed(chrono::TimeDelta::try_seconds(secs)?)?;
    let out = DateTime {
        year: u16::try_from(t.year()).ok()?,
        month: t.month() as u8,
        day: t.day() as u8,
        hour: t.hour() as u8,
        minute: t.minute() as u8,
        second: t.second() as u8,
        ..Default::default()
    };
    checked(out).ok()
}

/// Whether a full date has no time of day (midnight: none was given, or none is shown).
pub fn has_time(d: &DateTime) -> bool {
    d.is_full() && (d.hour, d.minute, d.second) != (0, 0, 0)
}

/// The same moment, ignoring the time zone and fractions of seconds.
pub fn same(a: &DateTime, b: &DateTime) -> bool {
    let key = |d: &DateTime| (d.year, d.month, d.day, d.hour, d.minute, d.second, d.precision);
    key(a) == key(b)
}

/// The EXIF text for an exported copy: a year or month alone as its first day (EXIF can't say
/// less; the copy's XMP keeps the precision).
pub fn exif_text(d: &DateTime) -> String {
    d.to_exif()
}

/// For `#[serde(with = "crate::taken::iso")]`: an optional date as its ISO text (`"1978-07"`),
/// so that library.json stays readable. A text that isn't a date reads as none.
pub mod iso {
    use craft_metadata::DateTime;
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(d: &Option<DateTime>, s: S) -> Result<S::Ok, S::Error> {
        match d {
            Some(d) => s.serialize_some(&d.to_iso()),
            None => s.serialize_none(),
        }
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Option<DateTime>, D::Error> {
        // anything else than a text (a damaged file) is read and dropped
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Any {
            Text(String),
            Other(serde::de::IgnoredAny),
        }
        Ok(match Option::<Any>::deserialize(d)? {
            Some(Any::Text(t)) => super::parse(&t).ok(),
            Some(Any::Other(_)) | None => None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_dates_of_every_precision() {
        assert_eq!(parse("1978").unwrap().precision, DatePrecision::Year);
        assert_eq!(parse(" 1978-07 ").unwrap().precision, DatePrecision::Month);
        let d = parse("1978-07-14").unwrap();
        assert!(d.is_full() && !has_time(&d));
        let t = parse("1978-07-14T10:30").unwrap();
        assert_eq!((t.hour, t.minute, t.second), (10, 30, 0));
        assert!(has_time(&t));
        // EXIF form, and a time zone dropped with the time kept as written
        assert!(same(&parse("1978:07:14 10:30:00").unwrap(), &t));
        let z = parse("1978-07-14T10:30:00+02:00").unwrap();
        assert_eq!((z.hour, z.offset_minutes), (10, None));
    }

    #[test]
    fn refuses_what_is_not_a_date() {
        for bad in ["", "July", "1978-13", "1978-02-30", "1978-07-14T25:00", "0001", "1700", "12345", "-1978"] {
            assert!(parse(bad).is_err(), "{bad:?}");
        }
    }

    #[test]
    fn builds_dates_from_fields() {
        assert_eq!(of_parts(1978, None, Some(3), None).unwrap().to_iso(), "1978");
        assert_eq!(of_parts(1978, Some(7), None, None).unwrap().to_iso(), "1978-07");
        assert_eq!(of_parts(1978, Some(7), Some(14), Some((9, 5, 0))).unwrap().to_iso(), "1978-07-14T09:05:00");
        assert!(of_parts(1978, Some(2), Some(29), None).is_err());
        assert!(of_parts(1980, Some(2), Some(29), None).is_ok());
    }

    #[test]
    fn shifts_full_dates_only() {
        let d = parse("2019-12-31T23:30:00").unwrap();
        assert_eq!(shifted(&d, 3600).unwrap().to_iso(), "2020-01-01T00:30:00");
        assert_eq!(shifted(&d, -365 * 86_400).unwrap().to_iso(), "2018-12-31T23:30:00");
        assert!(shifted(&parse("2019-12").unwrap(), 60).is_none());
        assert!(shifted(&parse("9999-12-31T23:00:00").unwrap(), 7200).is_none());
    }

    #[test]
    fn seconds_go_both_ways() {
        let d = parse("2006-08-18T10:34:25").unwrap();
        let s = seconds(&d).unwrap();
        assert_eq!(s, crate::import::parse_exif_date("2006:08:18 10:34:25").unwrap());
        assert!(same(&from_seconds(s).unwrap(), &d));
        // a year alone counts from its first day
        assert_eq!(seconds(&parse("2006").unwrap()), crate::import::parse_exif_date("2006:01:01 00:00:00"));
    }

    #[test]
    fn kept_as_text_in_the_library() {
        #[derive(serde::Serialize, serde::Deserialize, Default)]
        struct M {
            #[serde(default, with = "super::iso", skip_serializing_if = "Option::is_none")]
            taken: Option<DateTime>,
        }
        let m = M { taken: Some(parse("1978-07").unwrap()) };
        let json = serde_json::to_string(&m).unwrap();
        assert_eq!(json, r#"{"taken":"1978-07"}"#);
        assert_eq!(serde_json::from_str::<M>(&json).unwrap().taken.map(|d| d.to_iso()).as_deref(), Some("1978-07"));
        // damaged or missing: none, never an unreadable library
        assert!(serde_json::from_str::<M>(r#"{"taken":"soon"}"#).unwrap().taken.is_none());
        assert!(serde_json::from_str::<M>(r#"{"taken":12}"#).unwrap().taken.is_none());
        assert!(serde_json::from_str::<M>("{}").unwrap().taken.is_none());
    }
}
