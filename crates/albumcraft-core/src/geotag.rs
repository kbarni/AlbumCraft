//! Geotagging from GPS track logs (GPX): where each picture was taken, from the position the
//! logger had at the time the picture was taken (`craft_metadata::Tracklog::locate`: interpolated
//! between the two points around that time, within one continuous stretch of logging; else the
//! nearest point within `max_gap`).
//!
//! The hard part is the time. GPX times are UTC; a camera's date taken is its local time, with
//! no time zone unless the camera wrote one (EXIF `OffsetTimeOriginal`). `Zone` says which time
//! zone the pictures' times are in, and `Options::clock_shift` corrects a camera clock that was
//! off. `best_zone` finds the time zone that places the most pictures, for when the one chosen
//! places none.
//!
//! Reading (`read_gpx`, `photo_times`) is apart from matching (`locate`, `matches`), so the
//! window reads once off its thread and matches again at once whenever an option changes. The
//! result goes into the library (`Meta.place`), like Set location; the files are not changed.

use crate::geo::{LatLon, Place};
use crate::library::Store;
use craft_metadata::{DateTime, Match};
pub use craft_metadata::{Tracklog, parse_gpx};
use std::path::Path;

/// Largest GPX file read.
const MAX_GPX: u64 = 256 << 20;
/// How far from a logged point a picture may be, by default (seconds).
pub const DEFAULT_MAX_GAP: f64 = 600.0;

/// The track points of these GPX files, as one log (each file's tracks and segments kept apart).
pub fn read_gpx(paths: &[impl AsRef<Path>]) -> Result<Tracklog, String> {
    let mut all = Tracklog::default();
    for path in paths {
        let path = path.as_ref();
        let name = path.file_name().map_or_else(|| path.display().to_string(), |n| n.to_string_lossy().into_owned());
        let len = std::fs::metadata(path).map_err(|e| format!("{name}: {e}"))?.len();
        if len > MAX_GPX {
            return Err(crate::msg!("geotag-gpx-too-large", "{name} is too large to be a track log", name = name.as_str()).into());
        }
        let bytes = std::fs::read(path).map_err(|e| format!("{name}: {e}"))?;
        let text = String::from_utf8_lossy(&bytes);
        let log = craft_metadata::parse_gpx(&text).map_err(|e| format!("{name}: {e}"))?;
        add(&mut all, log);
    }
    if all.points.is_empty() {
        return Err(crate::msg!("geotag-no-points", "the track log has no timed points").into());
    }
    Ok(all)
}

/// Add `log` to `all`: its segments and tracks numbered after those already there, every point
/// in time order.
fn add(all: &mut Tracklog, log: Tracklog) {
    let seg = all.points.iter().map(|p| p.segment + 1).max().unwrap_or(0);
    let track = u32::try_from(all.tracks).unwrap_or(u32::MAX);
    all.points.extend(log.points.into_iter().map(|mut p| {
        p.segment = p.segment.saturating_add(seg);
        p.track = p.track.saturating_add(track);
        p
    }));
    all.points.sort_by(|a, b| a.time.total_cmp(&b.time));
    all.tracks += log.tracks;
    all.names.extend(log.names);
    all.untimed += log.untimed;
}

/// Everything geotagging reads: the track logs as one, and the pictures.
#[derive(Clone, Debug, Default)]
pub struct Read {
    pub log: Tracklog,
    pub photos: Vec<PhotoTime>,
}

/// Read the track logs and the pictures (`(key, date set in AlbumCraft, place set in AlbumCraft)`).
pub fn read(gpx: &[impl AsRef<Path>], photos: &[(String, Option<DateTime>, Option<Place>)]) -> Result<Read, String> {
    let log = read_gpx(gpx)?;
    let photos = photos.iter().map(|(k, t, p)| photo_time(k, *t, p.as_ref())).collect();
    Ok(Read { log, photos })
}

/// What is known of a picture for geotagging.
#[derive(Clone, Debug, PartialEq)]
pub struct PhotoTime {
    pub key: String,
    /// When it was taken (the date set in AlbumCraft, else the file's), as the camera's clock
    /// said; None without an exact date.
    pub taken: Option<DateTime>,
    /// It has a location already (set in AlbumCraft, or the file's GPS).
    pub located: bool,
}

/// Read when these pictures were taken and whether they have a location (reads the files).
pub fn photo_times(store: &Store, keys: &[String]) -> Vec<PhotoTime> {
    keys.iter()
        .map(|key| {
            let meta = store.meta.get(key);
            photo_time(key, meta.and_then(|m| m.taken), meta.and_then(|m| m.place.as_ref()))
        })
        .collect()
}

/// One picture's `PhotoTime`, from the date and place set in AlbumCraft (`user_taken`,
/// `place`) and, when they don't say, its file.
pub fn photo_time(key: &str, user_taken: Option<DateTime>, place: Option<&Place>) -> PhotoTime {
    let taken = user_taken.or_else(|| file_taken(key)).filter(DateTime::is_full);
    let located = match place {
        Some(Place::Set { .. }) => true,
        // the user hid the file's position: nothing to keep
        Some(Place::Removed) => false,
        None => crate::geo::file_gps(key).is_some(),
    };
    PhotoTime { key: key.to_string(), taken, located }
}

/// The file's own date taken, with its time zone when the camera wrote one.
fn file_taken(key: &str) -> Option<DateTime> {
    let from_meta = crate::filemeta::metadata(Path::new(key)).capture_time.filter(DateTime::is_full);
    from_meta.or_else(|| crate::import::date_taken(key).and_then(crate::taken::from_seconds))
}

/// Which time zone the pictures' times are in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Zone {
    /// The zone each picture says (EXIF offset), else this computer's (at that date).
    Pictures,
    /// This many minutes ahead of UTC, for every picture (`+120`: UTC+02:00).
    Fixed(i32),
}

impl Zone {
    /// "UTC+02:00", "UTC-03:30", "UTC".
    pub fn offset_text(minutes: i32) -> String {
        if minutes == 0 {
            return "UTC".into();
        }
        let sign = if minutes < 0 { '-' } else { '+' };
        let m = minutes.unsigned_abs();
        format!("UTC{sign}{:02}:{:02}", m / 60, m % 60)
    }

    /// The time zones offered: every half hour from UTC−12 to UTC+14, and the three zones on a
    /// quarter hour (Nepal, Chatham Islands, Eucla).
    pub fn offsets() -> Vec<i32> {
        let mut v: Vec<i32> = (-24..=28).map(|h| h * 30).chain([345, 525, 765]).collect();
        v.sort_unstable();
        v
    }
}

/// How pictures are matched to the track.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Options {
    pub zone: Zone,
    /// Seconds added to every picture's time: a camera clock that was 2 minutes slow is +120.
    pub clock_shift: i64,
    /// A picture more than this many seconds from a logged point gets no position.
    pub max_gap: f64,
    /// Replace the location of pictures that have one.
    pub replace: bool,
}

impl Default for Options {
    fn default() -> Self {
        Options { zone: Zone::Pictures, clock_shift: 0, max_gap: DEFAULT_MAX_GAP, replace: false }
    }
}

/// The moment a picture was taken (Unix seconds, UTC).
pub fn utc_seconds(taken: &DateTime, zone: Zone, clock_shift: i64) -> Option<i64> {
    let at = match zone {
        Zone::Pictures if taken.offset_minutes.is_some() => taken.unix_seconds(),
        Zone::Pictures => crate::filemeta::local_seconds(taken)?,
        Zone::Fixed(m) => DateTime { offset_minutes: None, ..*taken }.unix_seconds() - i64::from(m) * 60,
    };
    at.checked_add(clock_shift)
}

/// What became of a picture.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Outcome {
    /// Placed here; `interpolated` between two points (else the nearest one).
    Placed { at: LatLon, interpolated: bool },
    /// It has a location, kept (`Options::replace` is off).
    Kept,
    /// It has no exact date.
    NoDate,
    /// Taken when nothing was logged (before, after, or in a gap of the track).
    OutsideTrack,
}

/// Where the track puts one picture.
pub fn locate(p: &PhotoTime, log: &Tracklog, opts: &Options) -> Outcome {
    if p.located && !opts.replace {
        return Outcome::Kept;
    }
    let Some(t) = p.taken.as_ref().and_then(|d| utc_seconds(d, opts.zone, opts.clock_shift)) else { return Outcome::NoDate };
    match log.locate(t as f64, opts.max_gap) {
        Some((gps, how)) => match LatLon::new(gps.latitude, gps.longitude) {
            Some(at) => Outcome::Placed { at, interpolated: how == Match::Interpolated },
            None => Outcome::OutsideTrack,
        },
        None => Outcome::OutsideTrack,
    }
}

/// How many pictures came out each way.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Counts {
    pub placed: usize,
    pub kept: usize,
    pub no_date: usize,
    pub outside: usize,
}

/// Every picture matched to the track, and the counts.
pub fn matches(photos: &[PhotoTime], log: &Tracklog, opts: &Options) -> (Vec<Outcome>, Counts) {
    let mut c = Counts::default();
    let out: Vec<Outcome> = photos.iter().map(|p| locate(p, log, opts)).collect();
    for o in &out {
        match o {
            Outcome::Placed { .. } => c.placed += 1,
            Outcome::Kept => c.kept += 1,
            Outcome::NoDate => c.no_date += 1,
            Outcome::OutsideTrack => c.outside += 1,
        }
    }
    (out, c)
}

/// The fixed time zone that places the most pictures, and how many, when it places more than
/// `opts` does (the zone closest to UTC wins a tie).
pub fn best_zone(photos: &[PhotoTime], log: &Tracklog, opts: &Options) -> Option<(i32, usize)> {
    let now = matches(photos, log, opts).1.placed;
    let mut best: Option<(i32, usize)> = None;
    for m in Zone::offsets() {
        let n = matches(photos, log, &Options { zone: Zone::Fixed(m), ..*opts }).1.placed;
        let better = match best {
            None => n > 0,
            Some((bm, bn)) => n > bn || (n == bn && m.abs() < bm.abs()),
        };
        if better {
            best = Some((m, n));
        }
    }
    best.filter(|&(_, n)| n > now)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A walk logged every 10 s on 2024-06-01 from 10:00:00 UTC, going north 0.001° per point,
    /// with a gap of an hour after 11:00.
    fn gpx() -> String {
        let mut s = String::from(r#"<?xml version="1.0"?><gpx version="1.1" creator="test" xmlns="http://www.topografix.com/GPX/1/1"><trk><name>Walk</name><trkseg>"#);
        for i in 0..=360 {
            let (h, m, sec) = (10 + i * 10 / 3600, (i * 10 / 60) % 60, (i * 10) % 60);
            s += &format!(r#"<trkpt lat="{:.4}" lon="-0.5800"><time>2024-06-01T{h:02}:{m:02}:{sec:02}Z</time></trkpt>"#, 44.8 + i as f64 * 0.001);
        }
        s += "</trkseg><trkseg>";
        s += r#"<trkpt lat="45.5" lon="-0.58"><time>2024-06-01T12:00:00Z</time></trkpt><trkpt lat="45.6" lon="-0.58"><time>2024-06-01T12:10:00Z</time></trkpt>"#;
        s += "</trkseg></trk></gpx>";
        s
    }

    fn log() -> Tracklog {
        craft_metadata::parse_gpx(&gpx()).unwrap()
    }

    fn at(iso: &str) -> PhotoTime {
        PhotoTime { key: iso.into(), taken: Some(crate::taken::parse(iso).unwrap()), located: false }
    }

    fn placed(o: Outcome) -> LatLon {
        match o {
            Outcome::Placed { at, .. } => at,
            other => panic!("not placed: {other:?}"),
        }
    }

    #[test]
    fn pictures_are_placed_between_track_points() {
        // 12:00:05 in UTC+02:00 is 10:00:05 UTC: half way between the first two points
        let opts = Options { zone: Zone::Fixed(120), ..Default::default() };
        let o = locate(&at("2024-06-01T12:00:05"), &log(), &opts);
        assert!(matches!(o, Outcome::Placed { interpolated: true, .. }), "{o:?}");
        assert!((placed(o).lat - 44.8005).abs() < 1e-6);
        // the camera clock 30 s slow: +30 s, three points further
        let shifted = Options { clock_shift: 30, ..opts };
        assert!((placed(locate(&at("2024-06-01T12:00:00"), &log(), &shifted)).lat - 44.803).abs() < 1e-6);
    }

    #[test]
    fn a_time_zone_written_by_the_camera_is_used() {
        let mut p = at("2024-06-01T12:00:10");
        p.taken.as_mut().unwrap().offset_minutes = Some(120);
        assert!((placed(locate(&p, &log(), &Options::default())).lat - 44.801).abs() < 1e-6);
    }

    #[test]
    fn outside_the_track_and_kept_and_undated() {
        let opts = Options { zone: Zone::Fixed(0), ..Default::default() };
        // an hour before the log starts, and in the gap between the segments (11:30)
        assert_eq!(locate(&at("2024-06-01T09:00:00"), &log(), &opts), Outcome::OutsideTrack);
        assert_eq!(locate(&at("2024-06-01T11:30:00"), &log(), &opts), Outcome::OutsideTrack);
        // a few minutes after the first segment ends: its last point, not a line across the gap
        let near = locate(&at("2024-06-01T11:03:00"), &log(), &opts);
        assert!(matches!(near, Outcome::Placed { interpolated: false, .. }) && (placed(near).lat - 45.16).abs() < 1e-6, "{near:?}");
        let mut located = at("2024-06-01T10:00:00");
        located.located = true;
        assert_eq!(locate(&located, &log(), &opts), Outcome::Kept);
        assert!(matches!(locate(&located, &log(), &Options { replace: true, ..opts }), Outcome::Placed { .. }));
        let undated = PhotoTime { key: "x".into(), taken: None, located: false };
        let (_, c) = matches(&[undated, located, at("2024-06-01T10:30:00"), at("2024-06-01T08:00:00")], &log(), &opts);
        assert_eq!(c, Counts { placed: 1, kept: 1, no_date: 1, outside: 1 });
    }

    #[test]
    fn the_best_time_zone_is_found() {
        // taken in UTC−04:00 (10:00 UTC is 06:00 there): UTC places none, UTC−04:00 all three
        let photos = [at("2024-06-01T06:00:00"), at("2024-06-01T06:20:00"), at("2024-06-01T06:40:00")];
        let opts = Options { zone: Zone::Fixed(0), ..Default::default() };
        assert_eq!(best_zone(&photos, &log(), &opts), Some((-240, 3)));
        assert_eq!(best_zone(&photos, &log(), &Options { zone: Zone::Fixed(-240), ..opts }), None);
    }

    #[test]
    fn several_files_make_one_log() {
        let dir = std::env::temp_dir().join(format!("albumcraft-geotag-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let (a, b, bad) = (dir.join("a.gpx"), dir.join("b.gpx"), dir.join("bad.gpx"));
        std::fs::write(&a, gpx()).unwrap();
        let later = gpx().replace("2024-06-01", "2024-06-02");
        std::fs::write(&b, later).unwrap();
        std::fs::write(&bad, "not xml at all").unwrap();
        let both = read_gpx(&[&b, &a]);
        let refused = read_gpx(&[&bad]);
        let _ = std::fs::remove_dir_all(&dir);
        let both = both.unwrap();
        assert_eq!(both.tracks, 2);
        assert!(both.points.windows(2).all(|w| w[0].time <= w[1].time));
        // the two files' segments stay apart
        let segments = |day2: bool| both.points.iter().filter(|p| (p.time >= 1_717_286_400.0) == day2).map(|p| p.segment).collect::<std::collections::HashSet<_>>();
        assert!(segments(false).is_disjoint(&segments(true)));
        assert!(refused.is_err());
        let opts = Options { zone: Zone::Fixed(0), ..Default::default() };
        assert!(matches!(locate(&at("2024-06-02T10:00:00"), &both, &opts), Outcome::Placed { .. }));
    }
}
