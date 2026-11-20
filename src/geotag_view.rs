//! Locations from a GPS track, in the window (`Dialog::Geotag`, from the picture menu and the
//! folder menu): GPX files picked (`FileJob::Gpx`, several at once), read off the window's thread
//! with the pictures' dates (`Library::read_geotag` → `Done::Geotag`), then matched again at once
//! whenever an option changes (core `geotag`): the pictures' time zone, a camera clock that was
//! slow or fast, how far from a logged point a picture may be, and whether pictures that have a
//! location get the track's instead. The counts say what Apply will do; when the chosen zone
//! places few pictures, the zone that places the most is offered. Apply sets the places
//! (`Meta.place`, like Set location) as one `Undo::Places` notice; the files are never changed.

use crate::app::*;
use crate::geotag::{Counts, Options, Outcome, Read, Zone};
use crate::i18n::tr;
use crate::theme::t;
use eframe::egui::{self, RichText};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

/// The number of the last read asked for (an older answer is dropped).
static ASKED: AtomicU64 = AtomicU64::new(0);

/// How the camera's clock was.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Clock {
    Right,
    /// Behind the real time: its times are moved later.
    Slow,
    /// Ahead of the real time: its times are moved earlier.
    Fast,
}

/// The dialog's state (pictures by key: a scan may come in between).
#[derive(Clone, Debug)]
pub struct GeotagDialog {
    pub keys: Vec<String>,
    pub gpx: Vec<PathBuf>,
    asked: u64,
    /// None while reading.
    pub read: Option<Result<Arc<Read>, String>>,
    pub zone: Zone,
    pub clock: Clock,
    pub minutes: u32,
    pub seconds: u32,
    pub replace: bool,
    /// How far from a logged point a picture may be, in minutes.
    pub max_gap_minutes: u32,
    /// The last options matched, with their outcome and the best zone.
    cache: Option<(Options, Arc<Vec<Outcome>>, Counts, Option<(i32, usize)>)>,
}

impl GeotagDialog {
    fn options(&self) -> Options {
        let shift = i64::from(self.minutes) * 60 + i64::from(self.seconds);
        let clock_shift = match self.clock {
            Clock::Right => 0,
            Clock::Slow => shift,
            Clock::Fast => -shift,
        };
        Options { zone: self.zone, clock_shift, max_gap: f64::from(self.max_gap_minutes.max(1)) * 60.0, replace: self.replace }
    }

    /// The pictures matched with the options as they are now (kept until they change).
    fn matched(&mut self) -> Option<(Arc<Vec<Outcome>>, Counts, Option<(i32, usize)>)> {
        let Some(Ok(read)) = &self.read else { return None };
        let opts = self.options();
        if let Some((o, found, c, best)) = &self.cache
            && *o == opts
        {
            return Some((found.clone(), *c, *best));
        }
        let (found, c) = crate::geotag::matches(&read.photos, &read.log, &opts);
        let best = crate::geotag::best_zone(&read.photos, &read.log, &opts);
        let found = Arc::new(found);
        self.cache = Some((opts, found.clone(), c, best));
        Some((found, c, best))
    }
}

impl GeotagDialog {
    /// The dialog for `--screenshot` (`ALBUMCRAFT_SHOT_VIEW=geotag`): a made-up walk logged on
    /// 2024-06-01 from 10:00 to 11:00 UTC, the pictures taken over that hour in UTC+02:00, one
    /// with a location of its own, one before the walk, one without a date. The zone chosen
    /// (this computer's) may place none: the best zone is then offered.
    pub fn demo(keys: Vec<String>) -> GeotagDialog {
        let mut gpx = String::from(r#"<gpx version="1.1"><trk><name>Walk</name><trkseg>"#);
        for i in 0..=60 {
            let (h, m) = (10 + i / 60, i % 60);
            gpx += &format!(r#"<trkpt lat="{}" lon="{}"><time>2024-06-01T{h:02}:{m:02}:00Z</time></trkpt>"#, 44.84 + f64::from(i) * 0.0008, -0.58 + f64::from(i) * 0.0005);
        }
        gpx += "</trkseg></trk></gpx>";
        let log = crate::geotag::parse_gpx(&gpx).unwrap_or_default();
        let photos = keys
            .iter()
            .enumerate()
            .map(|(i, k)| {
                let taken = match i % 5 {
                    4 => None,
                    3 => crate::taken::parse("2024-06-01T09:00:00").ok(),
                    _ => crate::taken::parse(&format!("2024-06-01T12:{:02}:30", (i * 7) % 60)).ok(),
                };
                crate::geotag::PhotoTime { key: k.clone(), taken, located: i == 2 }
            })
            .collect();
        GeotagDialog {
            keys,
            gpx: vec![PathBuf::from("walk-2024-06-01.gpx")],
            asked: 0,
            read: Some(Ok(Arc::new(Read { log, photos }))),
            zone: Zone::Pictures,
            clock: Clock::Slow,
            minutes: 1,
            seconds: 30,
            replace: false,
            max_gap_minutes: 10,
            cache: None,
        }
    }
}

/// The GPX file picker (several files at once).
pub fn gpx_dialog() -> craft_dialogs::FileDialog {
    craft_dialogs::FileDialog::open_many().title(tr!("geotag-pick-title")).filter(tr!("geotag-filter"), &["gpx", "GPX"])
}

/// Local time of Unix seconds, as a date and time.
fn local_text(secs: f64) -> String {
    crate::taken::from_seconds(secs as i64).map(|d| crate::date_view::date_text(&d)).unwrap_or_default()
}

impl App {
    /// Open the dialog for these pictures, and the GPX file picker with it.
    pub fn ask_geotag(&mut self, ids: &[usize]) {
        let keys: Vec<String> = ids.iter().filter_map(|&id| self.photos.get(id)).filter(|p| !p.is_sample()).map(|p| p.key.clone()).collect();
        if keys.is_empty() {
            self.toast(tr!("export-select-first"));
            return;
        }
        let st = GeotagDialog {
            keys,
            gpx: vec![],
            asked: 0,
            read: None,
            zone: Zone::Pictures,
            clock: Clock::Right,
            minutes: 0,
            seconds: 0,
            replace: false,
            max_gap_minutes: (crate::geotag::DEFAULT_MAX_GAP / 60.0) as u32,
            cache: None,
        };
        self.dialog = Some(Dialog::Geotag(Box::new(st)));
        self.ask_file(gpx_dialog(), crate::file_dialogs::FileJob::Gpx);
    }

    /// GPX files were picked: read them with the pictures' dates.
    pub fn gpx_chosen(&mut self, paths: Vec<PathBuf>) {
        let Some(Dialog::Geotag(st)) = &self.dialog else { return };
        let photos = st
            .keys
            .iter()
            .map(|k| {
                let m = self.store.meta.get(k);
                (k.clone(), m.and_then(|m| m.taken), m.and_then(|m| m.place.clone()))
            })
            .collect();
        let asked = ASKED.fetch_add(1, Ordering::Relaxed) + 1;
        self.lib.read_geotag(asked, paths.clone(), photos);
        if let Some(Dialog::Geotag(st)) = &mut self.dialog {
            st.gpx = paths;
            st.asked = asked;
            st.read = None;
            st.cache = None;
        }
    }

    /// The answer of `Library::read_geotag`.
    pub fn on_geotag(&mut self, asked: u64, read: Result<Arc<Read>, String>) {
        if let Some(Dialog::Geotag(st)) = &mut self.dialog
            && st.asked == asked
        {
            st.read = Some(read);
            st.cache = None;
        }
    }

    /// Put back the places an `Undo::Places` holds.
    pub fn undo_places(&mut self, before: Vec<(String, Option<crate::geo::Place>)>) {
        for (key, place) in before {
            if let Some(m) = self.store.meta.get_mut(&key) {
                m.place = place;
            }
        }
        self.dirty = true;
        self.list_rev += 1;
    }

    /// Set the places the track gives; `show`: then open the map on those pictures.
    fn apply_geotag(&mut self, st: &GeotagDialog, found: &[Outcome], show: bool) {
        let Some(Ok(read)) = &st.read else { return };
        let mut before = vec![];
        let mut ids = vec![];
        for (p, o) in read.photos.iter().zip(found) {
            let (Outcome::Placed { at, .. }, Some(&id)) = (o, self.key_index.get(&p.key)) else { continue };
            before.push((p.key.clone(), self.store.meta.get(&p.key).and_then(|m| m.place.clone())));
            crate::commands::set_place(self, &[id], Some(crate::geo::Place::set(*at, "")));
            ids.push(id);
        }
        self.dirty = true;
        self.list_rev += 1;
        let n = ids.len();
        self.undoable(tr!("geotag-done", n = n), Undo::Places(before));
        if show {
            self.show_set_on_map(tr!("geotag-map-title"), &ids);
        }
    }

    pub(crate) fn geotag_dialog(&mut self, ctx: &egui::Context, mut st: Box<GeotagDialog>) {
        let mut close = false;
        let mut pick = false;
        let n = st.keys.len();
        let matched = st.matched();
        #[allow(deprecated)]
        let modal = egui::Modal::new(egui::Id::new("geotag")).show(ctx, |ui| {
            ui.set_width(460.0);
            ui.heading(tr!("geotag-title"));
            ui.add_space(6.0);
            ui.label(RichText::new(tr!("geotag-intro", n = n)).color(t().text_weak));
            ui.add_space(8.0);
            // the track files
            ui.horizontal_wrapped(|ui| {
                if st.gpx.is_empty() {
                    ui.label(tr!("geotag-no-file"));
                } else {
                    let names: Vec<String> = st.gpx.iter().map(|p| p.file_name().map_or_else(|| p.display().to_string(), |n| n.to_string_lossy().into_owned())).collect();
                    ui.add(egui::Label::new(RichText::new(names.join(", ")).strong()).truncate());
                }
                if ui.button(tr!("geotag-pick")).clicked() {
                    pick = true;
                }
            });
            match &st.read {
                None if !st.gpx.is_empty() => {
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.label(tr!("geotag-reading"));
                    });
                }
                Some(Err(e)) => {
                    ui.label(RichText::new(e.as_str()).color(t().bad));
                }
                Some(Ok(read)) => {
                    if let Some((a, b)) = read.log.span() {
                        let points = read.log.points.len();
                        ui.label(RichText::new(tr!("geotag-track-span", points = points, from = local_text(a), to = local_text(b))).size(12.0).color(t().text_weak));
                    }
                }
                None => {}
            }
            ui.add_space(8.0);
            egui::Grid::new("geotag-options").num_columns(2).spacing([10.0, 8.0]).show(ui, |ui| {
                ui.label(tr!("geotag-zone"));
                let zone_text = |z: Zone| match z {
                    Zone::Pictures => tr!("geotag-zone-pictures"),
                    Zone::Fixed(m) => Zone::offset_text(m),
                };
                egui::ComboBox::from_id_salt("geotag-zone").selected_text(zone_text(st.zone)).width(240.0).height(320.0).show_ui(ui, |ui| {
                    ui.selectable_value(&mut st.zone, Zone::Pictures, zone_text(Zone::Pictures));
                    for m in Zone::offsets() {
                        ui.selectable_value(&mut st.zone, Zone::Fixed(m), Zone::offset_text(m));
                    }
                });
                ui.end_row();
                ui.label(tr!("geotag-clock"));
                ui.horizontal_wrapped(|ui| {
                    let clock_text = |c: Clock| match c {
                        Clock::Right => tr!("geotag-clock-right"),
                        Clock::Slow => tr!("geotag-clock-slow"),
                        Clock::Fast => tr!("geotag-clock-fast"),
                    };
                    egui::ComboBox::from_id_salt("geotag-clock").selected_text(clock_text(st.clock)).width(120.0).show_ui(ui, |ui| {
                        for c in [Clock::Right, Clock::Slow, Clock::Fast] {
                            ui.selectable_value(&mut st.clock, c, clock_text(c));
                        }
                    });
                    if st.clock != Clock::Right {
                        ui.label(tr!("geotag-by"));
                        ui.add(egui::DragValue::new(&mut st.minutes).range(0..=10_080).suffix(" min"));
                        ui.add(egui::DragValue::new(&mut st.seconds).range(0..=59).suffix(" s"));
                    }
                });
                ui.end_row();
                ui.label(tr!("geotag-gap"));
                ui.horizontal(|ui| {
                    ui.add(egui::DragValue::new(&mut st.max_gap_minutes).range(1..=1440).suffix(" min"));
                    ui.label(RichText::new(tr!("geotag-gap-hint")).size(12.0).color(t().text_weak));
                });
                ui.end_row();
            });
            let any_located = matches!(&st.read, Some(Ok(r)) if r.photos.iter().any(|p| p.located));
            if any_located {
                ui.checkbox(&mut st.replace, tr!("geotag-replace"));
            }
            ui.add_space(8.0);
            let mut use_zone = None;
            if let Some((_, c, best)) = &matched {
                ui.label(RichText::new(tr!("geotag-will-place", n = c.placed)).strong());
                let mut rest = vec![];
                if c.outside > 0 {
                    rest.push(tr!("geotag-outside", n = c.outside));
                }
                if c.kept > 0 {
                    rest.push(tr!("geotag-kept", n = c.kept));
                }
                if c.no_date > 0 {
                    rest.push(tr!("geotag-no-date", n = c.no_date));
                }
                if !rest.is_empty() {
                    ui.label(RichText::new(rest.join(" · ")).color(t().text_weak));
                }
                if let Some((m, would)) = *best {
                    ui.horizontal_wrapped(|ui| {
                        let zone = Zone::offset_text(m);
                        ui.label(RichText::new(tr!("geotag-best-zone", zone = zone, n = would)).color(t().text_weak));
                        if ui.small_button(tr!("geotag-use-zone")).clicked() {
                            use_zone = Some(m);
                        }
                    });
                }
            }
            if let Some(m) = use_zone {
                st.zone = Zone::Fixed(m);
            }
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                let found = matched.as_ref().filter(|(_, c, _)| c.placed > 0).map(|(f, _, _)| f.clone());
                let ok = found.is_some();
                let apply = ui.add_enabled(ok, egui::Button::new(tr!("geotag-apply"))).clicked();
                let show = ui.add_enabled(ok, egui::Button::new(tr!("geotag-apply-show"))).clicked();
                if (apply || show)
                    && let Some(found) = &found
                {
                    self.apply_geotag(&st, found, show);
                    close = true;
                }
                if ui.button(tr!("button-cancel")).clicked() {
                    close = true;
                }
            });
        });
        if pick {
            self.ask_file(gpx_dialog(), crate::file_dialogs::FileJob::Gpx);
        }
        if self.dialog.is_none() && !close && !modal.should_close() {
            self.dialog = Some(Dialog::Geotag(st));
        }
    }
}
