//! The map: the "Places" view (pictures shown where they were taken, grouped when close) and the
//! "Set location" mode (search a place or click the map, drag the pin, then apply).
//!
//! The built-in map (`basemap`) is always there; online tiles (`online`) are drawn over it only
//! when the user turned online maps on. Positions come from `geo`: the place set by the user,
//! else the GPS position in the file, read by a background index.

#![allow(deprecated)] // egui::Id::new, as used elsewhere in this crate

use crate::app::*;
use crate::geo::{self, Found, LatLon, Place};
use crate::library::Photo;
use crate::online::{self, TileId, Tiles};
use crate::theme::t;
use eframe::egui::{self, Color32, Pos2, Rect, RichText, Sense, Stroke, StrokeKind, Vec2};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Receiver;
use std::sync::Arc;

/// Most detailed zoom of the built-in map (beyond it, it is too coarse to help).
const BUILTIN_MAX_ZOOM: f64 = 13.0;
/// Pictures closer than this on screen share one marker.
const CLUSTER_PX: f32 = 64.0;
const MARKER: f32 = 44.0;

// ---------------------------------------------------------------------------------------------
// the visible part of the world

/// What the map shows: `center` in world coordinates (`geo::to_world`; x may leave 0..1 while
/// panning across the date line) at a zoom where the world is 256 × 2^zoom pixels wide.
#[derive(Clone, Copy, Debug)]
pub struct Viewport {
    pub rect: Rect,
    pub center: [f64; 2],
    pub zoom: f64,
}

impl Viewport {
    pub fn scale(&self) -> f64 {
        256.0 * self.zoom.exp2()
    }

    pub fn zoom(&self) -> f64 {
        self.zoom
    }

    /// Screen position of a world point, in copy `k` of the world (k = -1 is the copy to the west).
    pub fn screen_of(&self, w: [f64; 2], k: i32) -> Pos2 {
        let s = self.scale();
        let c = self.rect.center();
        Pos2::new((c.x as f64 + (w[0] + k as f64 - self.center[0]) * s) as f32, (c.y as f64 + (w[1] - self.center[1]) * s) as f32)
    }

    pub fn screen_of_f32(&self, v: [f32; 2], k: i32) -> Pos2 {
        self.screen_of([v[0] as f64, v[1] as f64], k)
    }

    /// World point under a screen position (x not wrapped).
    pub fn world_of(&self, p: Pos2) -> [f64; 2] {
        let s = self.scale();
        let c = self.rect.center();
        [self.center[0] + (p.x - c.x) as f64 / s, self.center[1] + (p.y - c.y) as f64 / s]
    }

    pub fn latlon_at(&self, p: Pos2) -> LatLon {
        let w = self.world_of(p);
        geo::from_world([w[0].rem_euclid(1.0), w[1]])
    }

    fn half(&self) -> (f64, f64) {
        let s = self.scale();
        (self.rect.width() as f64 / 2.0 / s, self.rect.height() as f64 / 2.0 / s)
    }

    /// The copies of the world that are (partly) in view.
    pub fn world_copies(&self) -> std::ops::RangeInclusive<i32> {
        let (hw, _) = self.half();
        ((self.center[0] - hw).floor() as i32)..=((self.center[0] + hw).floor() as i32)
    }

    /// Whether a box of world coordinates [min x, min y, max x, max y] in copy `k` is in view.
    pub fn sees(&self, b: [f32; 4], k: i32) -> bool {
        let (hw, hh) = self.half();
        let (l, r) = (self.center[0] - hw - k as f64, self.center[0] + hw - k as f64);
        let (top, bottom) = (self.center[1] - hh, self.center[1] + hh);
        b[2] as f64 >= l && b[0] as f64 <= r && b[3] as f64 >= top && b[1] as f64 <= bottom
    }

    fn min_zoom(&self) -> f64 {
        (self.rect.height().max(1.0) as f64 / 256.0).log2().max(0.0)
    }

    /// Keep the zoom in range and the map filling the view from top to bottom.
    fn clamp(&mut self, max_zoom: f64) {
        if !self.zoom.is_finite() || !self.center[0].is_finite() || !self.center[1].is_finite() {
            *self = Viewport { rect: self.rect, center: [0.5, 0.45], zoom: 0.0 };
        }
        self.zoom = self.zoom.clamp(self.min_zoom(), max_zoom.max(self.min_zoom()));
        self.center[0] = self.center[0].rem_euclid(1.0);
        let (_, hh) = self.half();
        self.center[1] = if hh >= 0.5 { 0.5 } else { self.center[1].clamp(hh, 1.0 - hh) };
    }

    /// Zoom by `dz` levels keeping the point under `p` in place.
    fn zoom_at(&mut self, p: Pos2, dz: f64, max_zoom: f64) {
        let before = self.world_of(p);
        self.zoom += dz;
        self.clamp(max_zoom);
        let after = self.world_of(p);
        self.center[0] += before[0] - after[0];
        self.center[1] += before[1] - after[1];
        self.clamp(max_zoom);
    }

    fn pan(&mut self, d: Vec2) {
        let s = self.scale();
        self.center[0] -= d.x as f64 / s;
        self.center[1] -= d.y as f64 / s;
    }

    /// Show all these world points (or the whole world when there are none).
    fn fit(&mut self, pts: impl Iterator<Item = [f64; 2]>, max_zoom: f64) {
        let mut b = [f64::MAX, f64::MAX, f64::MIN, f64::MIN];
        let mut n = 0;
        for p in pts {
            b = [b[0].min(p[0]), b[1].min(p[1]), b[2].max(p[0]), b[3].max(p[1])];
            n += 1;
        }
        if n == 0 {
            self.center = [0.5, 0.42];
            self.zoom = 0.0;
        } else {
            self.center = [(b[0] + b[2]) / 2.0, (b[1] + b[3]) / 2.0];
            let (w, h) = ((b[2] - b[0]).max(1e-9), (b[3] - b[1]).max(1e-9));
            let z = ((self.rect.width() as f64 * 0.8 / (w * 256.0)).min(self.rect.height() as f64 * 0.75 / (h * 256.0))).log2();
            self.zoom = z.min(12.0);
        }
        self.clamp(max_zoom);
    }
}

// ---------------------------------------------------------------------------------------------
// state

/// A picture with a known position.
#[derive(Clone, Copy)]
pub struct Located {
    pub id: usize,
    pub at: LatLon,
    pub w: [f64; 2],
}

/// Pictures being placed in "Set location" mode.
pub struct Placing {
    /// By key: picture numbers change when folders are scanned again.
    keys: Vec<String>,
    pin: Option<LatLon>,
    name: String,
    /// The user typed the name: don't replace it with a looked-up one.
    name_edited: bool,
    naming: Option<(LatLon, Receiver<Result<String, String>>)>,
    dragging_pin: bool,
}

pub struct MapState {
    center: [f64; 2],
    zoom: f64,
    /// The view was set up (fitted to the pictures) at least once.
    ready: bool,
    fit_next: bool,
    /// Where Back goes.
    back: View,
    search: String,
    results: Vec<Found>,
    online_results: Vec<Found>,
    searching: Option<Receiver<Result<Vec<Found>, String>>>,
    search_note: Option<String>,
    results_open: bool,
    pub placing: Option<Placing>,
    /// The pictures of the marker clicked last.
    selected: Vec<usize>,
    selected_at: Option<LatLon>,
}

impl Default for MapState {
    fn default() -> Self {
        MapState {
            center: [0.5, 0.42],
            zoom: 0.0,
            ready: false,
            fit_next: false,
            back: View::Library,
            search: String::new(),
            results: vec![],
            online_results: vec![],
            searching: None,
            search_note: None,
            results_open: false,
            placing: None,
            selected: vec![],
            selected_at: None,
        }
    }
}

/// The running GPS index: its results, and the switch that stops it.
type Index = (Receiver<Vec<(String, Option<LatLon>)>>, Arc<AtomicBool>);

/// Everything about places the window keeps.
#[derive(Default)]
pub struct GeoState {
    /// GPS positions stored in the files, by key (None: the file has none).
    pub file_gps: HashMap<String, Option<LatLon>>,
    gps_rev: u64,
    index: Option<Index>,
    located: Arc<Vec<Located>>,
    located_key: Option<(u64, u64, usize)>,
    pub map: MapState,
    tiles: Option<Tiles>,
}

impl GeoState {
    /// Look for GPS positions in these pictures (in the background; cached on disk).
    pub fn start_index(&mut self, photos: &[Photo], lib: &dyn crate::backend::Library) {
        if let Some((_, stop)) = self.index.take() {
            stop.store(true, Ordering::Relaxed);
        }
        // picture numbers change with a scan
        self.map.selected.clear();
        let files: Vec<geo::FileStamp> = photos.iter().filter(|p| !p.is_sample()).map(|p| (p.key.clone(), p.modified, p.bytes)).collect();
        if files.is_empty() {
            return;
        }
        let stop = Arc::new(AtomicBool::new(false));
        let rx = lib.index_gps(files, stop.clone());
        self.index = Some((rx, stop));
    }

    /// Take the positions found since last time.
    pub fn poll(&mut self) {
        let Some((rx, _)) = &self.index else { return };
        let mut finished = false;
        loop {
            match rx.try_recv() {
                Ok(batch) => {
                    self.file_gps.extend(batch);
                    self.gps_rev += 1;
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => break,
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    finished = true;
                    break;
                }
            }
        }
        if finished {
            self.index = None;
        }
    }

    pub fn indexing(&self) -> bool {
        self.index.is_some()
    }

    /// Online map tiles are still on their way (the screenshot waits for them).
    pub fn tiles_pending(&self) -> bool {
        self.tiles.as_ref().is_some_and(|t| t.pending())
    }
}

fn spawn_lookup<T: Send + 'static>(ctx: &egui::Context, work: impl FnOnce() -> Result<T, String> + Send + 'static) -> Receiver<Result<T, String>> {
    let (tx, rx) = std::sync::mpsc::channel();
    let ctx = ctx.clone();
    std::thread::spawn(move || {
        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(work)).unwrap_or_else(|_| Err("the online search failed".into()));
        let _ = tx.send(r);
        ctx.request_repaint();
    });
    rx
}

// ---------------------------------------------------------------------------------------------

impl App {
    fn online_maps(&self) -> bool {
        self.store.online_maps == Some(true)
    }

    fn max_zoom(&self) -> f64 {
        if self.online_maps() { online::MAX_ZOOM as f64 } else { BUILTIN_MAX_ZOOM }
    }

    /// Pictures with a known position (rebuilt when the notes or the GPS index change).
    pub fn located(&mut self) -> Arc<Vec<Located>> {
        let key = (self.list_rev, self.geo.gps_rev, self.photos.len());
        if self.geo.located_key != Some(key) {
            let mut v = vec![];
            for (id, p) in self.photos.iter().enumerate() {
                let place = self.store.meta.get(&p.key).and_then(|m| m.place.as_ref());
                if let Some(at) = geo::effective(place, self.geo.file_gps.get(&p.key).copied().flatten()) {
                    v.push(Located { id, at, w: geo::to_world(at) });
                }
            }
            self.geo.located = Arc::new(v);
            self.geo.located_key = Some(key);
        }
        self.geo.located.clone()
    }

    /// A picture's position: its place, else its file's GPS (when the index has seen it).
    pub fn location(&self, id: usize) -> Option<LatLon> {
        let p = self.photos.get(id)?;
        let place = self.store.meta.get(&p.key).and_then(|m| m.place.as_ref());
        let file = match self.geo.file_gps.get(&p.key) {
            Some(g) => *g,
            None if place.is_none() => self.lib.file_gps(&p.key),
            None => None,
        };
        geo::effective(place, file)
    }

    /// "Bordeaux, France", or None without a position.
    pub fn location_label(&self, id: usize) -> Option<String> {
        let at = self.location(id)?;
        let place = self.photos.get(id).and_then(|p| self.store.meta.get(&p.key)).and_then(|m| m.place.as_ref());
        Some(geo::describe(place, at))
    }

    /// Per frame: take GPS positions found by the background index.
    pub fn geo_tick(&mut self) {
        self.geo.poll();
        if self.view == View::Map && self.geo.indexing() {
            self.ctx.request_repaint_after(std::time::Duration::from_millis(300));
        }
    }

    /// Open the Places map.
    pub fn open_places(&mut self) {
        if self.view != View::Map {
            self.geo.map.back = self.view;
        }
        self.geo.map.placing = None;
        self.geo.map.fit_next = !self.geo.map.ready;
        self.view = View::Map;
    }

    /// Open the Places map on one picture, with it selected.
    pub fn show_on_map(&mut self, id: usize) {
        let Some(at) = self.location(id) else { return };
        self.open_places();
        let m = &mut self.geo.map;
        m.center = geo::to_world(at);
        m.zoom = m.zoom.max(11.0);
        m.ready = true;
        m.fit_next = false;
        m.selected = vec![id];
        m.selected_at = Some(at);
    }

    /// Open the map to set where these pictures were taken.
    pub fn start_placing(&mut self, ids: Vec<usize>) {
        let ids: Vec<usize> = ids.into_iter().filter(|&id| id < self.photos.len()).collect();
        if ids.is_empty() {
            self.toast("Select some pictures first.");
            return;
        }
        let locs: Vec<Option<LatLon>> = ids.iter().map(|&id| self.location(id)).collect();
        // all at the same spot: start from there
        let same = match locs.first() {
            Some(Some(a)) => locs.iter().all(|l| l.is_some_and(|b| b.km_to(*a) < 0.01)),
            _ => false,
        };
        let pin = if same { locs.first().copied().flatten() } else { None };
        let name = if same { self.location_label(ids[0]).unwrap_or_default() } else { String::new() };
        let keys = ids.iter().map(|&id| self.photos[id].key.clone()).collect();
        if self.view != View::Map {
            self.geo.map.back = self.view;
        }
        self.geo.map.placing = Some(Placing { keys, pin, name, name_edited: same, naming: None, dragging_pin: false });
        self.geo.map.selected.clear();
        self.geo.map.search.clear();
        self.geo.map.results.clear();
        self.geo.map.online_results.clear();
        let m = &mut self.geo.map;
        if let Some(p) = pin {
            m.center = geo::to_world(p);
            m.zoom = m.zoom.max(12.0);
            m.ready = true;
            m.fit_next = false;
        } else {
            m.fit_next = !m.ready;
        }
        self.view = View::Map;
    }

    fn leave_map(&mut self) {
        self.geo.map.placing = None;
        self.geo.map.results_open = false;
        self.view = match self.geo.map.back {
            View::Edit if self.edit.id < self.photos.len() => View::Edit,
            _ => View::Library,
        };
    }

    fn placing_ids(&self) -> Vec<usize> {
        self.geo.map.placing.as_ref().map(|p| p.keys.iter().filter_map(|k| self.key_index.get(k).copied()).collect()).unwrap_or_default()
    }

    fn apply_placing(&mut self) {
        let Some(pl) = &self.geo.map.placing else { return };
        let Some(pin) = pl.pin else { return };
        let name = pl.name.clone();
        let ids = self.placing_ids();
        crate::commands::set_place(self, &ids, Some(Place::set(pin, &name)));
        self.toast(format!("Location set for {} picture{}.", ids.len(), if ids.len() == 1 { "" } else { "s" }));
        self.leave_map();
    }

    fn remove_placing(&mut self) {
        let ids = self.placing_ids();
        crate::commands::remove_place(self, &ids, false);
        self.toast(format!("Location removed from {} picture{}.", ids.len(), if ids.len() == 1 { "" } else { "s" }));
        self.leave_map();
    }

    /// Move the pin (and name it: the nearest city now, the address later when online).
    pub fn set_pin(&mut self, at: LatLon, name: Option<String>) {
        let online = self.online_maps();
        let ctx = self.ctx.clone();
        let Some(pl) = &mut self.geo.map.placing else { return };
        pl.pin = Some(at);
        if let Some(n) = name.filter(|n| !n.is_empty()) {
            pl.name = n;
            pl.name_edited = false;
            pl.naming = None;
            return;
        }
        if !pl.name_edited {
            pl.name = geo::describe(None, at);
        }
        pl.naming = online.then(|| (at, spawn_lookup(&ctx, move || online::name_of(at))));
    }

    fn go_to(&mut self, f: &Found) {
        let max = self.max_zoom();
        let m = &mut self.geo.map;
        m.center = geo::to_world(f.at);
        m.zoom = f.zoom.min(max);
        m.ready = true;
        m.results_open = false;
        if m.placing.is_some() {
            let name = if f.short.is_empty() { None } else { Some(f.short.clone()) };
            self.set_pin(f.at, name);
        }
    }

    /// Take answers from the online searches.
    fn poll_lookups(&mut self) {
        let m = &mut self.geo.map;
        if let Some(rx) = &m.searching
            && let Ok(r) = rx.try_recv()
        {
            m.searching = None;
            match r {
                Ok(found) => {
                    m.search_note = found.is_empty().then(|| "Nothing found online.".to_string());
                    m.online_results = found;
                }
                Err(e) => m.search_note = Some(e),
            }
        }
        if let Some(pl) = &mut m.placing
            && let Some((at, rx)) = &pl.naming
            && let Ok(r) = rx.try_recv()
        {
            if let (Ok(name), Some(pin)) = (r, pl.pin)
                && pin == *at
                && !pl.name_edited
                && !name.is_empty()
            {
                pl.name = name;
            }
            pl.naming = None;
        }
    }

    // -----------------------------------------------------------------------------------------
    // the view

    pub fn map_ui(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        self.poll_lookups();
        if self.geo.map.placing.as_ref().is_some_and(|_| self.placing_ids().is_empty()) {
            // the pictures went away (folders scanned again)
            self.leave_map();
            return;
        }
        if self.online_maps() && self.geo.tiles.is_none() {
            self.geo.tiles = Some(Tiles::new(&ctx));
        }
        let located = self.located();
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) && self.dialog.is_none() {
            if self.geo.map.results_open {
                self.geo.map.results_open = false;
            } else if !self.geo.map.selected.is_empty() {
                self.geo.map.selected.clear();
            } else {
                self.leave_map();
                return;
            }
        }
        self.map_top_bar(ui, &located);
        if self.geo.map.placing.is_some() {
            self.placing_panel(ui);
        } else if !self.geo.map.selected.is_empty() {
            self.selection_strip(ui);
        }
        egui::CentralPanel::default().frame(egui::Frame::new().fill(t().map_water)).show(ui, |ui| {
            self.map_canvas(ui, &located);
        });
        self.search_results_popup(&ctx);
    }

    fn map_top_bar(&mut self, ui: &mut egui::Ui, located: &[Located]) {
        let placing = self.geo.map.placing.as_ref().map(|p| p.keys.len());
        egui::Panel::top("map_top")
            .exact_size(60.0)
            .frame(egui::Frame::new().fill(t().lightbox_bg).inner_margin(egui::Margin::symmetric(16, 8)).stroke(Stroke::new(1.0, t().border)))
            .show(ui, |ui| {
                ui.horizontal_centered(|ui| {
                    let back = if self.geo.map.back == View::Edit { "‹  Editor" } else { "‹  Library" };
                    if ui.add(egui::Button::new(RichText::new(back).size(13.5)).min_size(egui::vec2(0.0, 32.0))).on_hover_text("Go back (Esc)").clicked() {
                        self.leave_map();
                        return;
                    }
                    ui.add_space(10.0);
                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing.y = 0.0;
                        let (title, sub) = match placing {
                            Some(n) => ("Set location".to_string(), format!("{n} picture{}", if n == 1 { "" } else { "s" })),
                            None => {
                                let n = located.len();
                                let sub = if self.geo.indexing() {
                                    format!("{n} with a location · looking for GPS positions…")
                                } else {
                                    format!("{n} picture{} with a location", if n == 1 { "" } else { "s" })
                                };
                                ("Places".to_string(), sub)
                            }
                        };
                        ui.label(RichText::new(title).size(20.0).strong().color(t().text_strong));
                        ui.label(RichText::new(sub).size(12.0).color(t().text_weak));
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.spacing_mut().item_spacing.x = 8.0;
                        let online = self.online_maps();
                        let label = if online { "🌐  Online map" } else { "🗺  Built-in map" };
                        let tip = if online {
                            "Showing OpenStreetMap. Click to use the built-in map only (no internet connections)."
                        } else {
                            "Showing the built-in world map. Click to show detailed maps from OpenStreetMap (downloads map pictures from the internet)."
                        };
                        if ui.add(egui::Button::new(label).selected(online).min_size(egui::vec2(0.0, 30.0))).on_hover_text(tip).clicked() {
                            self.store.online_maps = Some(!online);
                            self.dirty = true;
                            self.geo.map.online_results.clear();
                        }
                        if placing.is_none()
                            && ui.add(egui::Button::new("Show all").min_size(egui::vec2(0.0, 30.0))).on_hover_text("Zoom to show every picture").clicked()
                        {
                            self.geo.map.fit_next = true;
                        }
                        ui.add_space(4.0);
                        self.search_box(ui);
                    });
                });
            });
    }

    fn search_box(&mut self, ui: &mut egui::Ui) {
        let online = self.online_maps();
        let r = egui::Frame::new()
            .fill(t().surface)
            .stroke(Stroke::new(1.0, t().border))
            .corner_radius(18)
            .inner_margin(egui::Margin::symmetric(12, 3))
            .show(ui, |ui| {
                ui.allocate_ui_with_layout(egui::vec2(300.0, 26.0), egui::Layout::left_to_right(egui::Align::Center), |ui| {
                    ui.label(RichText::new("🔍").size(13.0).color(t().text_weak));
                    let hint = if online { "Search a place or address" } else { "Search a city or coordinates" };
                    let r = ui.add(
                        egui::TextEdit::singleline(&mut self.geo.map.search)
                            .id(egui::Id::new("map_search"))
                            .hint_text(hint)
                            .frame(egui::Frame::NONE)
                            .desired_width(250.0),
                    );
                    let m = &mut self.geo.map;
                    if r.changed() {
                        m.results = geo::search(&m.search, 6);
                        m.online_results.clear();
                        m.search_note = None;
                        m.results_open = !m.search.trim().is_empty();
                    }
                    if r.gained_focus() && !m.search.trim().is_empty() {
                        m.results_open = true;
                    }
                    let enter = r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                    if !m.search.is_empty() && ui.add(egui::Label::new(RichText::new("×").size(16.0).color(t().text_weak)).sense(Sense::click())).clicked() {
                        m.search.clear();
                        m.results.clear();
                        m.online_results.clear();
                        m.results_open = false;
                    }
                    enter
                })
                .inner
            });
        if r.inner {
            self.search_enter();
        }
        let rect = r.response.rect;
        ui.ctx().data_mut(|d| d.insert_temp(egui::Id::new("map_search_rect"), rect));
    }

    /// Put text in the map's search box, as if typed (for the screenshot).
    pub fn type_map_search(&mut self, q: &str) {
        let m = &mut self.geo.map;
        m.search = q.to_string();
        m.results = geo::search(q, 6);
        m.results_open = true;
    }

    /// Enter in the search box: search online when allowed, else take the first result.
    fn search_enter(&mut self) {
        let q = self.geo.map.search.trim().to_string();
        if q.is_empty() {
            return;
        }
        if self.online_maps() && geo::parse_coords(&q).is_none() {
            let ctx = self.ctx.clone();
            self.geo.map.searching = Some(spawn_lookup(&ctx, move || online::search(&q)));
            self.geo.map.search_note = None;
            self.geo.map.results_open = true;
        } else if let Some(f) = self.geo.map.results.first().cloned() {
            self.go_to(&f);
        } else {
            self.geo.map.search_note = Some("No city by that name in the built-in list.".into());
            self.geo.map.results_open = true;
        }
    }

    fn search_results_popup(&mut self, ctx: &egui::Context) {
        let m = &self.geo.map;
        if !m.results_open {
            return;
        }
        let Some(anchor) = ctx.data(|d| d.get_temp::<Rect>(egui::Id::new("map_search_rect"))) else { return };
        let online = self.online_maps();
        let mut chosen: Option<Found> = None;
        let mut search_online = false;
        let resp = egui::Area::new(egui::Id::new("map_results"))
            .order(egui::Order::Foreground)
            .fixed_pos(anchor.left_bottom() + egui::vec2(0.0, 4.0))
            .show(ctx, |ui| {
                egui::Frame::popup(ui.style()).show(ui, |ui| {
                    ui.set_width(anchor.width().max(320.0));
                    let row = |ui: &mut egui::Ui, f: &Found| -> bool {
                        let text = RichText::new(&f.name).size(13.0);
                        ui.add(egui::Button::new(text).frame(false).wrap_mode(egui::TextWrapMode::Truncate)).clicked()
                    };
                    if m.results.is_empty() && m.online_results.is_empty() && m.searching.is_none() && m.search_note.is_none() {
                        ui.label(RichText::new("No city by that name. Try coordinates, like 44.84, -0.58.").size(12.0).color(t().text_weak));
                    }
                    if !m.results.is_empty() {
                        ui.label(RichText::new(if online { "CITIES" } else { "CITIES (BUILT-IN LIST)" }).size(10.5).strong().color(t().text_weak));
                        for f in &m.results {
                            if row(ui, f) {
                                chosen = Some(f.clone());
                            }
                        }
                    }
                    if online {
                        ui.separator();
                        if m.searching.is_some() {
                            ui.horizontal(|ui| {
                                ui.spinner();
                                ui.label(RichText::new("Searching online…").size(12.0));
                            });
                        } else if !m.online_results.is_empty() {
                            ui.label(RichText::new("PLACES AND ADDRESSES (OPENSTREETMAP)").size(10.5).strong().color(t().text_weak));
                            for f in &m.online_results {
                                if row(ui, f) {
                                    chosen = Some(f.clone());
                                }
                            }
                        } else if ui.add(egui::Button::new(RichText::new("🔍  Search addresses online   (Enter)").size(12.5)).frame(false)).clicked() {
                            search_online = true;
                        }
                    }
                    if let Some(note) = &m.search_note {
                        ui.label(RichText::new(note).size(12.0).color(t().text_weak));
                    }
                });
            });
        if let Some(f) = chosen {
            self.go_to(&f);
        }
        if search_online {
            self.search_enter();
        }
        // a click anywhere else closes the list
        let clicked_outside = ctx.input(|i| i.pointer.any_pressed())
            && ctx.input(|i| i.pointer.interact_pos()).is_some_and(|p| !resp.response.rect.contains(p) && !anchor.contains(p));
        if clicked_outside {
            self.geo.map.results_open = false;
        }
    }

    /// The bottom panel in "Set location" mode.
    fn placing_panel(&mut self, ui: &mut egui::Ui) {
        let ids = self.placing_ids();
        let mut apply = false;
        let mut remove = false;
        let mut cancel = false;
        egui::Panel::bottom("placing")
            .exact_size(104.0)
            .frame(egui::Frame::new().fill(t().tray_bg).inner_margin(egui::Margin::symmetric(16, 10)).stroke(Stroke::new(1.0, t().border)))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    // the pictures being placed, as a small pile
                    let (well, _) = ui.allocate_exact_size(egui::vec2(150.0, 84.0), Sense::hover());
                    let p = ui.painter().clone();
                    for (k, &id) in ids.iter().take(4).enumerate().rev() {
                        let off = egui::vec2(k as f32 * 16.0, (k % 2) as f32 * -4.0);
                        let r = Rect::from_min_size(well.left_top() + egui::vec2(4.0, 10.0) + off, egui::vec2(64.0, 64.0));
                        p.rect_filled(r.expand(2.0), 3, Color32::WHITE);
                        paint_square_thumb(&p, r, self.thumbs.get(&id), 1.0);
                        if !self.thumbs.contains_key(&id) {
                            self.request_thumb(id);
                        }
                    }
                    if ids.len() > 4 {
                        p.text(well.right_bottom() - egui::vec2(4.0, 4.0), egui::Align2::RIGHT_BOTTOM, format!("+{}", ids.len() - 4), egui::FontId::proportional(12.0), t().text);
                    }
                    ui.add_space(8.0);
                    let online = self.online_maps();
                    let Some(pl) = &mut self.geo.map.placing else { return };
                    ui.vertical(|ui| {
                        ui.add_space(4.0);
                        match pl.pin {
                            None => {
                                ui.label(RichText::new("Where were these taken?").size(15.0).strong().color(t().text_strong));
                                ui.label(
                                    RichText::new("Search for a place above, or click on the map to drop a pin. You can drag the pin afterwards.")
                                        .size(12.5)
                                        .color(t().text_weak),
                                );
                            }
                            Some(at) => {
                                ui.horizontal(|ui| {
                                    ui.label(RichText::new("📍").size(16.0));
                                    let r = ui.add(egui::TextEdit::singleline(&mut pl.name).hint_text("Name of the place (optional)").desired_width(300.0));
                                    if r.changed() {
                                        pl.name_edited = true;
                                    }
                                    if pl.naming.is_some() {
                                        ui.spinner();
                                    }
                                });
                                let how = if online { "Drag the pin to adjust it." } else { "Drag the pin to adjust it. Online maps show streets." };
                                ui.label(RichText::new(format!("{}   ·   {how}", at.text())).size(12.0).color(t().text_weak));
                            }
                        }
                    });
                    let has_pin = pl.pin.is_some();
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.spacing_mut().item_spacing.x = 8.0;
                        let n = ids.len();
                        let label = format!("Set location for {n} picture{}", if n == 1 { "" } else { "s" });
                        let set = egui::Button::new(RichText::new(label).color(Color32::WHITE).strong()).fill(t().accent).min_size(egui::vec2(0.0, 34.0));
                        if ui.add_enabled(has_pin, set).clicked() {
                            apply = true;
                        }
                        if ui.add(egui::Button::new("Remove location").min_size(egui::vec2(0.0, 34.0))).on_hover_text("These pictures will have no location, even if the camera stored one (the files are not changed)").clicked() {
                            remove = true;
                        }
                        if ui.add(egui::Button::new("Cancel").min_size(egui::vec2(0.0, 34.0))).clicked() {
                            cancel = true;
                        }
                    });
                });
            });
        if apply {
            self.apply_placing();
        } else if remove {
            self.remove_placing();
        } else if cancel {
            self.leave_map();
        }
    }

    /// The bottom strip with the pictures of the clicked marker.
    fn selection_strip(&mut self, ui: &mut egui::Ui) {
        let ids: Vec<usize> = self.geo.map.selected.iter().copied().filter(|&id| id < self.photos.len()).collect();
        let label = self.geo.map.selected_at.map(|at| {
            let place = ids.first().and_then(|&id| self.store.meta.get(&self.photos[id].key)).and_then(|m| m.place.as_ref());
            geo::describe(place, at)
        });
        let mut open: Option<usize> = None;
        let mut change = false;
        let mut to_tray = false;
        let mut close = false;
        egui::Panel::bottom("map_selection")
            .exact_size(132.0)
            .frame(egui::Frame::new().fill(t().tray_bg).inner_margin(egui::Margin::symmetric(16, 8)).stroke(Stroke::new(1.0, t().border)))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    let n = ids.len();
                    ui.label(RichText::new(format!("{n} picture{}", if n == 1 { "" } else { "s" })).strong().size(14.0).color(t().text_strong));
                    if let Some(l) = &label {
                        ui.label(RichText::new(format!("·  {l}")).size(13.0).color(t().text));
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.small_button("✖").on_hover_text("Close (Esc)").clicked() {
                            close = true;
                        }
                        if ui.button("📍 Change location…").clicked() {
                            change = true;
                        }
                        if ui.button("Select in library").on_hover_text("Put these pictures in the photo tray").clicked() {
                            to_tray = true;
                        }
                    });
                });
                egui::ScrollArea::horizontal().auto_shrink([false, true]).show(ui, |ui| {
                    ui.horizontal(|ui| {
                        for &id in ids.iter().take(500) {
                            let (r, resp) = ui.allocate_exact_size(egui::vec2(84.0, 84.0), Sense::click());
                            if ui.is_rect_visible(r) {
                                if !self.thumbs.contains_key(&id) {
                                    self.request_thumb(id);
                                }
                                if resp.hovered() {
                                    ui.painter().rect_filled(r.expand(2.0), 4, t().accent);
                                }
                                paint_square_thumb(ui.painter(), r, self.thumbs.get(&id), 1.0);
                            }
                            if resp.on_hover_text(&self.photos[id].name).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                                open = Some(id);
                            }
                        }
                    });
                });
            });
        if close {
            self.geo.map.selected.clear();
        }
        if change {
            self.start_placing(ids.clone());
        }
        if to_tray {
            self.selection = ids.iter().copied().collect();
            self.held.clear();
            self.source = Source::Library;
            self.geo.map.back = View::Library;
            self.leave_map();
        }
        if let Some(id) = open {
            self.open_editor(id);
            self.edit.list = ids;
        }
    }

    fn map_canvas(&mut self, ui: &mut egui::Ui, located: &[Located]) {
        let (rect, resp) = ui.allocate_exact_size(ui.available_size(), Sense::click_and_drag());
        let ctx = ui.ctx().clone();
        let max_zoom = self.max_zoom();
        let mut view = Viewport { rect, center: self.geo.map.center, zoom: self.geo.map.zoom };
        if self.geo.map.fit_next || !self.geo.map.ready {
            let placing_pts: Vec<[f64; 2]> = self.placing_ids().iter().filter_map(|&id| self.location(id)).map(geo::to_world).collect();
            if self.geo.map.placing.is_some() && !placing_pts.is_empty() {
                view.fit(placing_pts.into_iter(), max_zoom);
            } else {
                view.fit(located.iter().map(|l| l.w), max_zoom);
            }
            self.geo.map.fit_next = false;
            self.geo.map.ready = true;
        }
        view.clamp(max_zoom);

        // zoom: wheel, pinch, double-click, the + / − buttons
        if resp.hovered()
            && let Some(p) = resp.hover_pos()
        {
            let (scroll, pinch) = ui.input(|i| (i.smooth_scroll_delta.y, i.zoom_delta()));
            let dz = scroll as f64 / 240.0 + (pinch as f64).log2();
            if dz.abs() > 1e-4 {
                view.zoom_at(p, dz, max_zoom);
            }
        }
        if resp.double_clicked()
            && let Some(p) = resp.interact_pointer_pos()
        {
            view.zoom_at(p, 1.0, max_zoom);
        }
        let pointer = resp.interact_pointer_pos().or(resp.hover_pos());

        // the pin being placed: grab it to drag it
        let pin_screen = self.geo.map.placing.as_ref().and_then(|pl| pl.pin).map(|at| nearest_copy(&view, geo::to_world(at)));
        if resp.drag_started()
            && let (Some(pl), Some(pin), Some(p)) = (&mut self.geo.map.placing, pin_screen, pointer)
        {
            let head = pin - egui::vec2(0.0, 24.0);
            pl.dragging_pin = p.distance(head) < 20.0 || p.distance(pin) < 12.0;
        }
        let dragging_pin = self.geo.map.placing.as_ref().is_some_and(|pl| pl.dragging_pin);
        if resp.dragged() {
            if dragging_pin {
                if let (Some(p), Some(pl)) = (pointer, &mut self.geo.map.placing) {
                    pl.pin = Some(view.latlon_at(p + egui::vec2(0.0, 0.0)));
                }
            } else {
                view.pan(resp.drag_delta());
                view.clamp(max_zoom);
            }
        }
        if resp.drag_stopped() && dragging_pin {
            if let Some(pl) = &mut self.geo.map.placing {
                pl.dragging_pin = false;
            }
            if let Some(at) = self.geo.map.placing.as_ref().and_then(|pl| pl.pin) {
                self.set_pin(at, None);
            }
        }

        let clusters = clusters(&view, located);
        let hovered = pointer.filter(|_| resp.hovered()).and_then(|p| clusters.iter().position(|c| marker_rect(c.pos).contains(p)));

        // a click: drop the pin, or open a marker's pictures
        if resp.clicked()
            && let Some(p) = pointer
        {
            self.geo.map.results_open = false;
            if self.geo.map.placing.is_some() {
                self.set_pin(view.latlon_at(p), None);
            } else if let Some(c) = hovered.and_then(|i| clusters.get(i)) {
                self.geo.map.selected = c.ids.clone();
                self.geo.map.selected_at = Some(c.at);
            } else {
                self.geo.map.selected.clear();
            }
        }

        // paint: map, markers, pin, overlays
        let painter = ui.painter_at(rect);
        match crate::basemap::get(&ctx) {
            Some(b) => b.paint(&painter, &view),
            None => {
                painter.rect_filled(rect, 0, t().map_water);
                painter.text(rect.center(), egui::Align2::CENTER_CENTER, "Preparing the map…", egui::FontId::proportional(14.0), t().text_weak);
            }
        }
        let online = self.online_maps();
        if online && let Some(tiles) = &mut self.geo.tiles {
            tiles.begin_frame(&ctx);
            paint_tiles(&painter, &view, tiles);
        }
        let placing = self.geo.map.placing.is_some();
        let selected: std::collections::HashSet<usize> = self.geo.map.selected.iter().copied().collect();
        let mut wanted = 0;
        for (i, c) in clusters.iter().enumerate() {
            let first = c.ids[0];
            if !self.thumbs.contains_key(&first) && wanted < 64 {
                self.request_thumb(first);
                wanted += 1;
            }
            let is_sel = c.ids.iter().any(|id| selected.contains(id));
            paint_marker(&painter, c.pos, self.thumbs.get(&first), c.ids.len(), is_sel, hovered == Some(i), if placing { 0.45 } else { 1.0 });
        }
        if let Some(pl) = &self.geo.map.placing
            && let Some(at) = pl.pin
        {
            paint_pin(&painter, nearest_copy(&view, geo::to_world(at)), pl.dragging_pin);
        }

        // hover: how many pictures, and where
        if let Some(c) = hovered.and_then(|i| clusters.get(i))
            && !placing
        {
            let n = c.ids.len();
            let text = format!("{n} picture{} · {}", if n == 1 { "" } else { "s" }, geo::describe(None, c.at));
            resp.clone().on_hover_text_at_pointer(text);
        }
        let cursor = if dragging_pin || (resp.dragged() && !dragging_pin) {
            egui::CursorIcon::Grabbing
        } else if hovered.is_some() && !placing {
            egui::CursorIcon::PointingHand
        } else if placing {
            egui::CursorIcon::Crosshair
        } else {
            egui::CursorIcon::Grab
        };
        if resp.hovered() || resp.dragged() {
            ctx.set_cursor_icon(cursor);
        }

        // zoom buttons (top left)
        let zoom_btn = |ui: &mut egui::Ui, r: Rect, glyph: &str, id: &str| -> bool {
            let resp = ui.interact(r, egui::Id::new(("mapzoom", id)), Sense::click());
            let p = ui.painter();
            p.rect_filled(r, 6, if resp.hovered() { t().surface_hover } else { t().surface });
            p.rect_stroke(r, 6, Stroke::new(1.0, t().border), StrokeKind::Inside);
            p.text(r.center(), egui::Align2::CENTER_CENTER, glyph, egui::FontId::proportional(18.0), t().text);
            resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked()
        };
        let plus = Rect::from_min_size(rect.left_top() + egui::vec2(12.0, 12.0), egui::vec2(32.0, 32.0));
        let minus = plus.translate(egui::vec2(0.0, 36.0));
        if zoom_btn(ui, plus, "+", "in") {
            view.zoom_at(rect.center(), 1.0, max_zoom);
        }
        if zoom_btn(ui, minus, "−", "out") {
            view.zoom_at(rect.center(), -1.0, max_zoom);
        }

        // attribution (bottom right), as the map data's licences ask
        let tiles_problem = self.geo.tiles.as_ref().and_then(|t| t.problem.clone()).filter(|_| online);
        let credit = if online { online::ATTRIBUTION } else { "Built-in map: Natural Earth · GeoNames" };
        let galley = painter.layout_no_wrap(credit.to_string(), egui::FontId::proportional(11.0), t().text);
        let cr = Rect::from_min_size(rect.right_bottom() - galley.size() - egui::vec2(14.0, 8.0), galley.size() + egui::vec2(10.0, 4.0));
        painter.rect_filled(cr, 3, t().surface.gamma_multiply(0.85));
        painter.galley(cr.min + egui::vec2(5.0, 2.0), galley, t().text);
        if online {
            let r = ui.interact(cr, egui::Id::new("osm_credit"), Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand).on_hover_text(online::COPYRIGHT_URL);
            if r.clicked() {
                ctx.open_url(egui::OpenUrl::new_tab(online::COPYRIGHT_URL));
            }
        }
        if let Some(problem) = tiles_problem {
            let text = format!("{problem}. Showing the built-in map.");
            let galley = painter.layout(text, egui::FontId::proportional(12.0), t().text, rect.width() - 120.0);
            let r = Rect::from_min_size(rect.left_top() + egui::vec2(56.0, 12.0), galley.size() + egui::vec2(16.0, 8.0));
            painter.rect_filled(r, 6, t().surface);
            painter.rect_stroke(r, 6, Stroke::new(1.0, t().border), StrokeKind::Inside);
            painter.galley(r.min + egui::vec2(8.0, 4.0), galley, t().text);
        }

        // nothing to show yet: say how to get pictures on the map
        if located.is_empty() && !placing {
            let text = if self.geo.indexing() {
                "Looking for GPS positions in your pictures…".to_string()
            } else {
                "No pictures with a location yet.\n\nPictures from phones and GPS cameras show up here by themselves. To place others, select them and click 📍 Location in the photo tray.".to_string()
            };
            let galley = painter.layout(text, egui::FontId::proportional(13.5), t().text, 360.0);
            let r = Rect::from_center_size(rect.center(), galley.size() + egui::vec2(36.0, 28.0));
            painter.rect_filled(r, 10, t().surface);
            painter.rect_stroke(r, 10, Stroke::new(1.0, t().border), StrokeKind::Inside);
            painter.galley(r.min + egui::vec2(18.0, 14.0), galley, t().text);
        }

        self.geo.map.center = view.center;
        self.geo.map.zoom = view.zoom;
    }
}

// ---------------------------------------------------------------------------------------------
// markers

struct Cluster {
    pos: Pos2,
    at: LatLon,
    ids: Vec<usize>,
}

/// Group pictures that are close on screen into one marker each.
fn clusters(view: &Viewport, located: &[Located]) -> Vec<Cluster> {
    let mut cells: HashMap<(i32, i32), usize> = HashMap::new();
    let mut out: Vec<(Vec2, usize, LatLon, Vec<usize>)> = vec![];
    let area = view.rect.expand(MARKER);
    for k in view.world_copies() {
        for l in located {
            let p = view.screen_of(l.w, k);
            if !area.contains(p) {
                continue;
            }
            let cell = (((p.x - view.rect.left()) / CLUSTER_PX).floor() as i32, ((p.y - view.rect.top()) / CLUSTER_PX).floor() as i32);
            match cells.get(&cell) {
                Some(&i) => {
                    let c = &mut out[i];
                    c.0 += p.to_vec2();
                    c.1 += 1;
                    c.3.push(l.id);
                }
                None => {
                    cells.insert(cell, out.len());
                    out.push((p.to_vec2(), 1, l.at, vec![l.id]));
                }
            }
        }
    }
    out.into_iter().map(|(sum, n, at, ids)| Cluster { pos: (sum / n as f32).to_pos2(), at, ids }).collect()
}

/// The photo square of a marker standing on `at`.
fn marker_rect(at: Pos2) -> Rect {
    Rect::from_center_size(at - egui::vec2(0.0, MARKER / 2.0 + 9.0), egui::vec2(MARKER, MARKER)).expand(3.0)
}

/// A thumbnail cut to a square (the middle of the picture).
fn paint_square_thumb(p: &egui::Painter, r: Rect, tex: Option<&egui::TextureHandle>, alpha: f32) {
    match tex {
        Some(tex) => {
            let [w, h] = tex.size();
            let a = w as f32 / h.max(1) as f32;
            let uv = if a >= 1.0 {
                let m = (1.0 - 1.0 / a) / 2.0;
                Rect::from_min_max(Pos2::new(m, 0.0), Pos2::new(1.0 - m, 1.0))
            } else {
                let m = (1.0 - a) / 2.0;
                Rect::from_min_max(Pos2::new(0.0, m), Pos2::new(1.0, 1.0 - m))
            };
            p.image(tex.id(), r, uv, Color32::WHITE.gamma_multiply(alpha));
        }
        None => {
            p.rect_filled(r, 2, t().placeholder.gamma_multiply(alpha));
        }
    }
}

fn paint_marker(p: &egui::Painter, at: Pos2, tex: Option<&egui::TextureHandle>, count: usize, selected: bool, hovered: bool, alpha: f32) {
    let frame = marker_rect(at);
    let photo = frame.shrink(3.0);
    let white = Color32::WHITE.gamma_multiply(alpha);
    p.rect_filled(frame.translate(egui::vec2(0.0, 2.0)).expand(1.0), 7, Color32::from_black_alpha((70.0 * alpha) as u8));
    p.add(egui::Shape::convex_polygon(vec![frame.center_bottom() + egui::vec2(-7.0, -1.0), frame.center_bottom() + egui::vec2(7.0, -1.0), at], white, Stroke::NONE));
    p.rect_filled(frame, 6, white);
    paint_square_thumb(p, photo, tex, alpha);
    if selected || hovered {
        let c = if selected { t().accent } else { t().accent.gamma_multiply(0.6) };
        p.rect_stroke(frame, 6, Stroke::new(2.5, c), StrokeKind::Outside);
    }
    if count > 1 {
        let text = if count > 999 { "999+".to_string() } else { count.to_string() };
        let galley = p.layout_no_wrap(text, egui::FontId::proportional(11.5), Color32::WHITE);
        let w = (galley.size().x + 10.0).max(20.0);
        let badge = Rect::from_center_size(frame.right_top() + egui::vec2(-2.0, 2.0), egui::vec2(w, 20.0));
        p.rect_filled(badge, 10, t().accent.gamma_multiply(alpha));
        p.rect_stroke(badge, 10, Stroke::new(1.5, white), StrokeKind::Outside);
        p.galley(badge.center() - galley.size() / 2.0, galley, Color32::WHITE);
    }
}

/// The pin of "Set location": a drop shape whose point is on the spot.
fn paint_pin(p: &egui::Painter, at: Pos2, lifted: bool) {
    let lift = if lifted { 8.0 } else { 0.0 };
    let head = at - egui::vec2(0.0, 26.0 + lift);
    let red = t().reject;
    p.circle_filled(at, 4.0, Color32::from_black_alpha(60));
    p.add(egui::Shape::convex_polygon(vec![head + egui::vec2(-9.0, 5.0), head + egui::vec2(9.0, 5.0), at - egui::vec2(0.0, lift)], red, Stroke::NONE));
    p.circle_filled(head, 12.0, red);
    p.circle_stroke(head, 12.0, Stroke::new(1.5, Color32::WHITE));
    p.circle_filled(head, 4.5, Color32::WHITE);
}

/// Screen position of a world point in the copy of the world nearest to the middle of the view.
fn nearest_copy(view: &Viewport, w: [f64; 2]) -> Pos2 {
    let k = (view.center[0] - w[0]).round() as i32;
    view.screen_of(w, k)
}

/// Draw the online tiles in view (with coarser ones standing in while they load) and ask for the
/// missing ones, nearest to the middle first.
fn paint_tiles(painter: &egui::Painter, view: &Viewport, tiles: &mut Tiles) {
    let z = view.zoom.round().clamp(0.0, online::MAX_ZOOM as f64) as u8;
    let n = 1u32 << z;
    let nf = n as f64;
    let tile_px = (view.scale() / nf) as f32;
    let tl = view.world_of(view.rect.left_top());
    let br = view.world_of(view.rect.right_bottom());
    let (x0, x1) = ((tl[0] * nf).floor() as i64, (br[0] * nf).floor() as i64);
    let (y0, y1) = ((tl[1] * nf).floor().max(0.0) as i64, (br[1] * nf).floor().min(nf - 1.0) as i64);
    if x1 - x0 > 64 || y1 - y0 > 64 {
        return;
    }
    let center = view.rect.center();
    let mut wanted: Vec<(f32, TileId)> = vec![];
    for ty in y0..=y1 {
        for tx in x0..=x1 {
            let id = TileId { z, x: tx.rem_euclid(n as i64) as u32, y: ty as u32 };
            let min = view.screen_of([tx as f64 / nf, ty as f64 / nf], 0);
            // half a pixel of overlap hides the seams between tiles
            let r = Rect::from_min_size(min, egui::vec2(tile_px, tile_px)).expand(0.5);
            if let Some(tex) = tiles.get(id) {
                painter.image(tex.id(), r, Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)), Color32::WHITE);
                continue;
            }
            wanted.push((r.center().distance_sq(center), id));
            for dz in 1..=z.min(5) {
                let parent = TileId { z: z - dz, x: id.x >> dz, y: id.y >> dz };
                if let Some(tex) = tiles.get(parent) {
                    let k = (1u32 << dz) as f32;
                    let (ox, oy) = ((id.x - (parent.x << dz)) as f32 / k, (id.y - (parent.y << dz)) as f32 / k);
                    let uv = Rect::from_min_size(Pos2::new(ox, oy), egui::vec2(1.0 / k, 1.0 / k));
                    painter.image(tex.id(), r, uv, Color32::WHITE);
                    break;
                }
            }
        }
    }
    wanted.sort_by(|a, b| a.0.total_cmp(&b.0));
    tiles.want(wanted.into_iter().map(|(_, id)| id).collect());
}

#[cfg(test)]
mod tests {
    use super::*;

    fn view() -> Viewport {
        Viewport { rect: Rect::from_min_size(Pos2::new(100.0, 50.0), egui::vec2(800.0, 600.0)), center: [0.5, 0.5], zoom: 3.0 }
    }

    #[test]
    fn screen_and_world_agree() {
        let v = view();
        for p in [Pos2::new(100.0, 50.0), Pos2::new(512.3, 401.7), Pos2::new(899.0, 649.0)] {
            let back = v.screen_of(v.world_of(p), 0);
            assert!(back.distance(p) < 1e-3, "{p:?} -> {back:?}");
        }
        assert_eq!(v.screen_of([0.5, 0.5], 0), v.rect.center());
    }

    #[test]
    fn zooming_keeps_the_point_under_the_pointer() {
        let mut v = view();
        let p = Pos2::new(300.0, 200.0);
        let before = v.latlon_at(p);
        v.zoom_at(p, 1.5, 19.0);
        let after = v.latlon_at(p);
        assert!(before.km_to(after) < 1.0, "{before:?} {after:?}");
        assert!((v.zoom - 4.5).abs() < 1e-9);
    }

    #[test]
    fn view_stays_in_range() {
        let mut v = view();
        v.zoom = 50.0;
        v.center = [7.3, -4.0];
        v.clamp(13.0);
        assert_eq!(v.zoom, 13.0);
        assert!((0.0..1.0).contains(&v.center[0]) && (0.0..=1.0).contains(&v.center[1]));
        v.center = [f64::NAN, 0.5];
        v.clamp(13.0);
        assert!(v.center[0].is_finite());
        // across the date line: two copies of the world are in view
        let mut v = view();
        v.zoom = 2.0;
        v.center = [0.98, 0.5];
        assert!(v.world_copies().count() >= 2);
    }

    #[test]
    fn close_pictures_share_a_marker() {
        // the whole world, once
        let v = Viewport { rect: Rect::from_min_size(Pos2::ZERO, egui::vec2(500.0, 500.0)), center: [0.5, 0.5], zoom: 1.0 };
        assert_eq!(v.world_copies().count(), 1);
        let at = |lat, lon| {
            let at = LatLon { lat, lon };
            (at, geo::to_world(at))
        };
        let (a, wa) = at(44.84, -0.58);
        let (b, wb) = at(44.85, -0.57);
        let (c, wc) = at(-33.87, 151.21);
        let located = [Located { id: 0, at: a, w: wa }, Located { id: 1, at: b, w: wb }, Located { id: 2, at: c, w: wc }];
        let cl = clusters(&v, &located);
        let sizes: Vec<usize> = cl.iter().map(|c| c.ids.len()).collect();
        assert!(sizes.contains(&2), "{sizes:?}");
        assert_eq!(sizes.iter().sum::<usize>(), 3);
    }

    #[test]
    fn fitting_shows_every_point() {
        let mut v = view();
        let pts = [geo::to_world(LatLon { lat: 44.8, lon: -0.6 }), geo::to_world(LatLon { lat: 48.9, lon: 2.35 })];
        v.fit(pts.into_iter(), 19.0);
        for p in pts {
            assert!(v.rect.contains(v.screen_of(p, 0)));
        }
        v.fit(std::iter::empty(), 19.0);
        assert!(v.zoom.is_finite());
    }
}
