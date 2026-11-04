//! Application state, shared helpers, dialogs and the frame loop.

use crate::imaging::{self, EditOp};
use crate::library::{self, Album, Done, Folder, Job, Loaded, Meta, Photo, Pool, ScannedFolder, Store};
use eframe::egui::{self, Color32, ColorImage, TextureHandle, TextureOptions};
use image::RgbaImage;
use std::collections::{BTreeSet, HashMap, HashSet, VecDeque};
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Mutex};

pub use crate::theme::t;

#[derive(Clone, PartialEq, Debug)]
pub enum Source {
    Library,
    Album(usize),
    Person(String),
    Starred,
}

/// The tray as it was right after a picture output (export, print, collage) with held pictures
/// (armed when pictures are held).
#[derive(Clone, Debug)]
pub struct TrayCheck {
    pub count: usize,
    pub old: Vec<usize>,
}

/// Texts of the "clear old held items" question.
pub const MSG_CLEAR_TRAY: &str = "Would you like to clear your old held items from the tray?";
pub const BTN_CLEAR_TRAY: &str = "Clear Tray";
pub const BTN_DONT_CLEAR: &str = "Don't Clear";

/// A whole set put into the tray by clicking it in the folder list.
#[derive(Clone, PartialEq, Debug)]
pub enum TraySet {
    Folder(usize),
    Album(usize),
    Person(String),
    Starred,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum View {
    Library,
    Edit,
    Slideshow,
    Print,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Tab {
    Basic,
    Tuning,
    Effects,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Tool {
    None,
    Crop,
    Redeye,
    Straighten,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum CropAspect {
    Manual,
    Square,
    FourThree,
    ThreeTwo,
}

impl CropAspect {
    pub fn ratio(&self) -> Option<f32> {
        match self {
            CropAspect::Manual => None,
            CropAspect::Square => Some(1.0),
            CropAspect::FourThree => Some(4.0 / 3.0),
            CropAspect::ThreeTwo => Some(1.5),
        }
    }
    pub fn label(&self) -> &'static str {
        match self {
            CropAspect::Manual => "Manual",
            CropAspect::Square => "Square",
            CropAspect::FourThree => "4 x 3",
            CropAspect::ThreeTwo => "3 x 2 (4x6 print)",
        }
    }
}

pub enum Dialog {
    Info { title: String, text: String },
    NewAlbum { name: String },
    RenameAlbum { idx: usize, name: String },
    Tag { tags: String, people: String },
    Export { max_dim: u32, quality: u8 },
    FolderNote { path: String, text: String },
    FolderManager,
    Collage,
    Movie,
    About,
    /// "Choose Edits" when leaving A|A with two different versions.
    ChooseEdits { then: PendingLeave, dont_ask: bool },
    /// "Would you like to clear your old held items from the tray?"
    ClearTray { old: Vec<usize> },
    /// Print review: (picture, name, pixels per inch, grade 0 bad / 1 good / 2 best).
    PrintReview { grades: Vec<(usize, String, f32, u8)>, before_print: bool },
}

/// What to do after the A|A view has ended.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum PendingLeave {
    Mode(CompareMode),
    Library,
    Photo(usize),
}

pub struct EditState {
    pub id: usize,
    pub list: Vec<usize>,
    pub tab: Tab,
    pub tool: Tool,
    pub aspect: CropAspect,
    pub angle: f32,
    pub drag_start: Option<egui::Pos2>,
    pub sel: Option<[f32; 4]>,
    pub redo: Vec<EditOp>,
    pub show_info: bool,
    pub tag_buf: String,
    pub person_buf: String,
    /// "A", "A|B" or "A|A" view: the three buttons at the top of the editor.
    pub compare: CompareMode,
    /// One picture above the other instead of side by side.
    pub vertical: bool,
    /// In A|A: the second version of the same picture. The version that has the focus lives in
    /// the picture's normal edit list; the other one is kept here.
    pub alt_edits: Vec<EditOp>,
    /// The focus ("Selected", where edits go) is on the right / bottom side (swap_2up_focus).
    pub focus_second: bool,
    /// The picture the thumbnail strip was last scrolled to.
    pub strip_shown: Option<usize>,
    /// Zoom: None = fit the picture in the window; Some(z) = z × the photo's real pixels (1.0 = 100 %).
    pub zoom: Option<f32>,
    /// How far the zoomed picture is moved from the centre, in screen points.
    pub pan: egui::Vec2,
    /// Sharp render for zooming in: (picture, edits, texture, its size in real-photo pixels per texture pixel).
    pub zoom_tex: Option<(usize, Vec<EditOp>, TextureHandle, f32)>,
    /// The sharp render being prepared in the background.
    pub zoom_req: Option<(usize, Vec<EditOp>)>,
    /// The zoom level that "Fit" gives for the current picture and window (for the slider).
    pub fit_zoom: f32,
    /// A running Fit / 1:1 animation.
    pub zoom_anim: Option<ZoomAnim>,
    /// The second picture in "A|B" view.
    pub other: Option<usize>,
}

/// Zoom moving smoothly to a target ("to: None" = Fit) over 0.3 s.
#[derive(Clone, Copy, Debug)]
pub struct ZoomAnim {
    pub from: f32,
    pub to: Option<f32>,
    pub t0: f64,
    pub pan_from: egui::Vec2,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum CompareMode {
    /// One picture.
    Single,
    /// The edited picture next to another picture of the folder.
    TwoPhotos,
    /// The edited picture next to the same picture without edits (before / after).
    BeforeAfter,
}

pub type Histogram = [[u32; 256]; 3];

pub fn histogram(img: &RgbaImage) -> Histogram {
    let mut h = [[0u32; 256]; 3];
    for p in img.pixels() {
        for c in 0..3 {
            h[c][p[c] as usize] += 1;
        }
    }
    h
}

pub struct Slideshow {
    pub list: Vec<usize>,
    pub pos: usize,
    pub playing: bool,
    pub last: f64,
    pub interval: f32,
    pub from: View,
    pub last_move: f64,
}

pub struct BigEntry {
    pub base: RgbaImage,
    pub tex: Option<(Vec<EditOp>, TextureHandle)>,
    /// The picture without any edits (the "before" side).
    pub alt_tex: Option<(Vec<EditOp>, TextureHandle)>,
    /// Histogram of the edited picture shown in the editor.
    pub hist: Option<Histogram>,
}

pub struct App {
    pub store: Store,
    pub dirty: bool,
    pub last_save: f64,
    pub persist: bool,
    /// The last error saving the library (shown once, not every retry).
    pub save_error: Option<String>,
    /// Bumped whenever the library or its notes may have changed: invalidates `list_cache`.
    pub list_rev: u64,
    /// `groups()` for (list_rev, source, search, only_starred).
    #[allow(clippy::type_complexity)]
    list_cache: std::cell::RefCell<Option<((u64, Source, String, bool), Vec<(Option<usize>, Vec<usize>)>)>>,

    pub photos: Vec<Photo>,
    pub folders: Vec<Folder>,
    pub key_index: HashMap<String, usize>,
    pub using_samples: bool,
    pub scanning: bool,

    pub thumb_base: HashMap<usize, RgbaImage>,
    pub thumbs: HashMap<usize, TextureHandle>,
    pub thumb_req: HashSet<usize>,
    pub thumb_failed: HashSet<usize>,
    pub big: HashMap<usize, BigEntry>,
    pub big_order: VecDeque<usize>,
    pub big_req: HashSet<usize>,
    pub effect_previews: Option<(usize, Vec<EditOp>, Vec<TextureHandle>)>,

    pub pool: Pool,
    pub tx: Sender<Done>,
    pub rx: Receiver<Done>,

    pub view: View,
    pub source: Source,
    pub search: String,
    pub only_starred: bool,
    pub selection: BTreeSet<usize>,
    pub anchor: Option<usize>,
    pub held: Vec<usize>,
    pub scroll_to_folder: Option<usize>,
    /// Scroll the library to this picture (after coming back from the editor).
    pub scroll_to_photo: Option<usize>,
    /// The folder / album / person whose pictures fill the tray (when nothing is held or picked).
    pub tray_set: Option<TraySet>,
    /// The folder / album / person clicked last in the list (for its highlight).
    pub list_pick: Option<TraySet>,
    /// Armed after an output with held pictures: asks once to clear them when the tray grows.
    pub tray_check: Option<TrayCheck>,
    /// Real pixel size of each photo (from the file header), for 100 % zoom.
    pub real_dims: HashMap<usize, (u32, u32)>,
    /// Camera information (EXIF) per photo; None = the file has none.
    pub exif_cache: HashMap<usize, Option<Arc<crate::exifinfo::CameraInfo>>>,
    /// The camera's own embedded thumbnail, as a texture.
    pub exif_thumbs: HashMap<usize, TextureHandle>,

    pub edit: EditState,
    pub slide: Option<Slideshow>,
    pub print: Option<crate::print_view::PrintState>,
    pub dialog: Option<Dialog>,
    pub toast: Option<(String, f64)>,

    pub shot: Option<PathBuf>,
    pub frames: u64,
    pub shot_requested: bool,
    pub shot_slot: Arc<Mutex<Option<Arc<ColorImage>>>>,
    pub ctx: egui::Context,
}

pub fn to_color_image(img: &RgbaImage) -> ColorImage {
    ColorImage::from_rgba_unmultiplied([img.width() as usize, img.height() as usize], img.as_raw())
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>, shot: Option<PathBuf>) -> Self {
        let ctx = cc.egui_ctx.clone();
        crate::theme::apply(&ctx, crate::theme::Mode::System, true);
        let (tx, rx) = std::sync::mpsc::channel();
        let threads = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4).clamp(2, 6);
        let pool = Pool::new(threads, tx.clone(), ctx.clone());

        let mut persist = shot.is_none();
        let mut first_run = false;
        let mut load_problem = None;
        let mut store = if persist {
            match Store::load() {
                Loaded::Store(s) => s,
                Loaded::Fresh => {
                    first_run = true;
                    Store::default()
                }
                Loaded::Unreadable { error, backup } => {
                    let text = match backup {
                        Some(b) => format!(
                            "Your library file could not be read ({error}).\n\nIt was kept as:\n{}\n\nAlbumCraft started with an empty library. Your pictures themselves are not affected.",
                            b.display()
                        ),
                        None => {
                            // it could not be moved aside: never save over it
                            persist = false;
                            format!(
                                "Your library file could not be read ({error}).\n\nAlbumCraft started with an empty library and won't save any changes this time, so that the file is not overwritten."
                            )
                        }
                    };
                    load_problem = Some(text);
                    Store::default()
                }
            }
        } else {
            Store::default()
        };
        if let Ok(v) = std::env::var("ALBUMCRAFT_THEME") {
            store.theme = v;
        }
        if first_run {
            if let Some(p) = dirs::picture_dir() {
                store.roots.push(p.to_string_lossy().to_string());
            }
        }

        let mut app = App {
            store,
            dirty: first_run,
            last_save: 0.0,
            persist,
            save_error: None,
            list_rev: 0,
            list_cache: Default::default(),
            photos: vec![],
            folders: vec![],
            key_index: HashMap::new(),
            using_samples: false,
            scanning: false,
            thumb_base: HashMap::new(),
            thumbs: HashMap::new(),
            thumb_req: HashSet::new(),
            thumb_failed: HashSet::new(),
            big: HashMap::new(),
            big_order: VecDeque::new(),
            big_req: HashSet::new(),
            effect_previews: None,
            pool,
            tx,
            rx,
            view: View::Library,
            source: Source::Library,
            search: String::new(),
            only_starred: false,
            selection: BTreeSet::new(),
            anchor: None,
            held: vec![],
            scroll_to_folder: None,
            scroll_to_photo: None,
            tray_set: None,
            list_pick: None,
            tray_check: None,
            real_dims: HashMap::new(),
            exif_cache: HashMap::new(),
            exif_thumbs: HashMap::new(),
            edit: EditState {
                id: 0,
                list: vec![],
                tab: Tab::Basic,
                tool: Tool::None,
                aspect: CropAspect::Manual,
                angle: 0.0,
                drag_start: None,
                sel: None,
                redo: vec![],
                show_info: false,
                compare: CompareMode::Single,
                vertical: false,
                alt_edits: vec![],
                focus_second: false,
                strip_shown: None,
                zoom: None,
                pan: egui::Vec2::ZERO,
                zoom_tex: None,
                zoom_req: None,
                fit_zoom: 1.0,
                zoom_anim: None,
                other: None,
                tag_buf: String::new(),
                person_buf: String::new(),
            },
            slide: None,
            print: None,
            dialog: None,
            toast: None,
            shot,
            frames: 0,
            shot_requested: false,
            shot_slot: Arc::new(Mutex::new(None)),
            ctx,
        };

        if app.shot.is_some() {
            // A fixed, private demo state for the automatic picture.
            app.set_library(library::sample_folders(), true);
            app.store.albums.push(Album { name: "Best of the year".into(), keys: vec![] });
            for (i, p) in [0usize, 3, 10, 17].iter().enumerate() {
                if *p < app.photos.len() {
                    let key = app.photos[*p].key.clone();
                    app.store.meta.entry(key.clone()).or_default().star = true;
                    if i < 3 {
                        app.store.albums[0].keys.push(key);
                    }
                }
            }
            let shot_view = std::env::var("ALBUMCRAFT_SHOT_VIEW").unwrap_or_default();
            if app.photos.len() > 5 {
                let k = app.photos[5].key.clone();
                app.store.meta.entry(k).or_default().people.push("Grandma".into());
                if shot_view == "tray" {
                    // a clicked folder with nothing held: the whole folder in the tray
                    app.pick_set(TraySet::Folder(1));
                } else {
                    app.selection.extend([1usize, 2, 4]);
                    app.held.push(9);
                }
            }
            // Developer aid: ALBUMCRAFT_SHOT_VIEW=edit pictures the editor instead of the library.
            if shot_view == "compare" && app.photos.len() > 2 {
                let k = app.photos[0].key.clone();
                let m = app.store.meta.entry(k).or_default();
                m.edits.push(EditOp::Tune { fill: 0.4, highlights: 0.1, shadows: 0.05, temp: 0.4 });
                m.edits.push(EditOp::Effect(imaging::Effect::Warmify));
                app.open_editor(0);
                app.edit.compare = CompareMode::BeforeAfter;
                app.edit.tab = Tab::Tuning;
            }
            if shot_view == "print" && app.photos.len() > 5 {
                app.selection = [0usize, 1, 2, 3, 5].into_iter().collect();
                app.held.clear();
                app.store.print.layout = crate::print::PrintLayout::Cm9x13;
                app.open_print();
            }
            if shot_view == "props" {
                // a temporary folder with one photo carrying camera information
                let dir = std::env::temp_dir().join("albumcraft-props-demo");
                let _ = std::fs::create_dir_all(&dir);
                if crate::exifinfo::write_demo_jpeg(&dir.join("P8184048.JPG")).is_ok() {
                    let (found, _) = library::scan_roots(&[dir.to_string_lossy().to_string()], 10);
                    app.set_library(found, false);
                    if !app.photos.is_empty() {
                        app.open_editor(0);
                        app.edit.show_info = true;
                    }
                }
            }
            if shot_view == "zoom" && app.photos.len() > 25 {
                app.open_editor(25);
                app.edit.zoom = Some(2.0);
            }
            if shot_view == "edit" && app.photos.len() > 2 {
                let k = app.photos[2].key.clone();
                let m = app.store.meta.entry(k).or_default();
                m.caption = "Evening by the water".into();
                m.edits.push(EditOp::Effect(imaging::Effect::Warmify));
                app.open_editor(2);
                app.edit.show_info = true;
            }
        } else {
            app.rescan();
        }
        if let Some(text) = load_problem {
            app.dialog = Some(Dialog::Info { title: "Library".into(), text });
        }
        app
    }

    // ----------------------------------------------------------------- library

    pub fn rescan(&mut self) {
        let roots = self.store.roots.clone();
        let tx = self.tx.clone();
        let ctx = self.ctx.clone();
        self.scanning = true;
        std::thread::spawn(move || {
            let (found, truncated) = library::scan_roots(&roots, library::SCAN_LIMIT);
            let _ = tx.send(Done::Scan(found, truncated));
            ctx.request_repaint();
        });
    }

    pub fn set_library(&mut self, scanned: Vec<ScannedFolder>, samples: bool) {
        self.list_rev += 1;
        self.photos.clear();
        self.folders.clear();
        self.key_index.clear();
        self.thumb_base.clear();
        self.thumbs.clear();
        self.thumb_req.clear();
        self.thumb_failed.clear();
        self.big.clear();
        // photo numbers change with a new scan, so per-photo caches start over
        self.real_dims.clear();
        self.exif_cache.clear();
        self.exif_thumbs.clear();
        self.edit.zoom_tex = None;
        self.tray_set = None;
        self.list_pick = None;
        self.big_order.clear();
        self.big_req.clear();
        self.effect_previews = None;
        self.selection.clear();
        self.held.clear();
        self.anchor = None;
        self.using_samples = samples;
        // Everything else that remembers photo numbers: they mean other pictures after a rescan.
        self.tray_check = None;
        self.print = None;
        self.edit.list.clear();
        if matches!(self.dialog, Some(Dialog::ClearTray { .. } | Dialog::PrintReview { .. } | Dialog::ChooseEdits { .. })) {
            self.dialog = None;
        }
        if self.slide.take().is_some() {
            self.ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(false));
        }
        if self.view != View::Library {
            self.view = View::Library;
        }

        let mut folders: Vec<(String, Vec<(String, i64, u64)>)> = scanned;
        folders.sort_by_key(|(_, files)| -files.iter().map(|f| f.1).max().unwrap_or(0));
        for (path, files) in folders {
            let fi = self.folders.len();
            let name = path.rsplit(['/', '\\']).find(|s| !s.is_empty()).unwrap_or(&path).to_string();
            let date = files.iter().map(|f| f.1).max().unwrap_or(0);
            let mut ids = vec![];
            for (key, modified, bytes) in files {
                let pname = if key.starts_with(imaging::SAMPLE_PREFIX) {
                    library::sample_file_name(&key)
                } else {
                    key.rsplit(['/', '\\']).next().unwrap_or(&key).to_string()
                };
                let id = self.photos.len();
                self.key_index.insert(key.clone(), id);
                self.photos.push(Photo { key, name: pname, folder: fi, modified, bytes });
                ids.push(id);
            }
            self.folders.push(Folder { path, name, date, photos: ids });
        }
        if !samples {
            self.import_picasa_ini();
        }
        if let Source::Album(i) = self.source {
            if i >= self.store.albums.len() {
                self.source = Source::Library;
            }
        }
    }

    /// Bring in stars, captions, tags, rotations, crops, filters, folder descriptions and albums
    /// saved in each folder's .picasa.ini. Done once per folder and
    /// read-only: the file itself is never changed.
    fn import_picasa_ini(&mut self) {
        let mut imported = 0;
        for fi in 0..self.folders.len() {
            let path = self.folders[fi].path.clone();
            if self.store.imported_ini.contains(&path) {
                continue;
            }
            let Some(ini) = library::read_picasa_ini(std::path::Path::new(&path)) else { continue };
            self.store.imported_ini.push(path.clone());
            self.dirty = true;
            imported += 1;
            if let Some(d) = ini.description.clone().or(ini.name.clone()) {
                self.store.folder_notes.entry(path.clone()).or_insert(d);
            }
            let by_name: HashMap<String, usize> =
                self.folders[fi].photos.iter().map(|&id| (self.photos[id].name.to_lowercase(), id)).collect();
            for (file, m) in &ini.photos {
                if let Some(&id) = by_name.get(file) {
                    let key = self.photos[id].key.clone();
                    if !self.store.meta.contains_key(&key) && !m.is_empty() {
                        self.store.meta.insert(key, m.clone());
                    }
                }
            }
            for (file, token) in &ini.memberships {
                let Some(&id) = by_name.get(file) else { continue };
                let name = ini.albums.iter().find(|(t, _)| t == token).map(|(_, n)| n.clone()).unwrap_or_else(|| token.clone());
                let key = self.photos[id].key.clone();
                let idx = match self.store.albums.iter().position(|a| a.name == name) {
                    Some(i) => i,
                    None => {
                        self.store.albums.push(Album { name, keys: vec![] });
                        self.store.albums.len() - 1
                    }
                };
                if !self.store.albums[idx].keys.contains(&key) {
                    self.store.albums[idx].keys.push(key);
                }
            }
        }
        if imported > 0 {
            self.toast(format!("Brought in stars, captions and edits from the .picasa.ini files of {imported} folder(s)."));
        }
    }

    pub fn meta(&self, id: usize) -> Option<&Meta> {
        self.photos.get(id).and_then(|p| self.store.meta.get(&p.key))
    }

    pub fn meta_mut(&mut self, id: usize) -> &mut Meta {
        self.dirty = true;
        let key = self.photos[id].key.clone();
        self.store.meta.entry(key).or_default()
    }

    pub fn edits(&self, id: usize) -> Vec<EditOp> {
        self.meta(id).map(|m| m.edits.clone()).unwrap_or_default()
    }

    pub fn is_starred(&self, id: usize) -> bool {
        self.meta(id).is_some_and(|m| m.star)
    }

    pub fn matches(&self, id: usize) -> bool {
        let m = self.meta(id);
        if self.only_starred && !m.is_some_and(|m| m.star) {
            return false;
        }
        let q = self.search.trim().to_lowercase();
        if q.is_empty() {
            return true;
        }
        let Some(p) = self.photos.get(id) else { return false };
        let folder = self.folders.get(p.folder).map(|f| f.name.as_str()).unwrap_or("");
        let mut hay = format!("{} {}", p.name, folder).to_lowercase();
        if let Some(m) = m {
            hay.push(' ');
            hay.push_str(&m.caption.to_lowercase());
            for t in m.tags.iter().chain(m.people.iter()) {
                hay.push(' ');
                hay.push_str(&t.to_lowercase());
            }
        }
        q.split_whitespace().all(|w| hay.contains(w))
    }

    /// Groups shown in the lightbox: (folder index if any, photo ids).
    /// The visible pictures grouped by folder (or one group for an album, person, Starred).
    /// Cached: building it walks the whole library, and it is asked for several times a frame.
    pub fn groups(&self) -> Vec<(Option<usize>, Vec<usize>)> {
        let mut cache = self.list_cache.borrow_mut();
        let key = (self.list_rev, self.source.clone(), self.search.clone(), self.only_starred);
        if let Some((k, groups)) = cache.as_ref() {
            if *k == key {
                return groups.clone();
            }
        }
        let groups = self.build_groups();
        *cache = Some((key, groups.clone()));
        groups
    }

    fn build_groups(&self) -> Vec<(Option<usize>, Vec<usize>)> {
        match &self.source {
            Source::Library => self
                .folders
                .iter()
                .enumerate()
                .map(|(fi, f)| (Some(fi), f.photos.iter().copied().filter(|&id| self.matches(id)).collect::<Vec<_>>()))
                .filter(|(_, v)| !v.is_empty())
                .collect(),
            Source::Album(i) => {
                let ids = self
                    .store
                    .albums
                    .get(*i)
                    .map(|a| a.keys.iter().filter_map(|k| self.key_index.get(k).copied()).filter(|&id| self.matches(id)).collect())
                    .unwrap_or_default();
                vec![(None, ids)]
            }
            Source::Person(name) => {
                let ids = (0..self.photos.len())
                    .filter(|&id| self.meta(id).is_some_and(|m| m.people.iter().any(|p| p == name)) && self.matches(id))
                    .collect();
                vec![(None, ids)]
            }
            Source::Starred => {
                let ids = (0..self.photos.len()).filter(|&id| self.is_starred(id) && self.matches(id)).collect();
                vec![(None, ids)]
            }
        }
    }

    pub fn visible_flat(&self) -> Vec<usize> {
        self.groups().into_iter().flat_map(|(_, v)| v).collect()
    }

    pub fn people(&self) -> Vec<(String, usize)> {
        let mut counts: HashMap<String, usize> = HashMap::new();
        for p in &self.photos {
            if let Some(m) = self.store.meta.get(&p.key) {
                for n in &m.people {
                    *counts.entry(n.clone()).or_default() += 1;
                }
            }
        }
        let mut v: Vec<_> = counts.into_iter().collect();
        v.sort();
        v
    }

    /// The photos in the tray: held ones first, then the current selection. With nothing held and no
    /// single picture picked, a clicked folder / album / person puts its whole set in the tray
    /// ("Folder selected - 29 photos").
    pub fn tray(&self) -> Vec<usize> {
        if self.held.is_empty() && self.selection.is_empty() {
            if let Some(set) = &self.tray_set {
                return self.set_photos(set);
            }
        }
        let mut v = self.held.clone();
        for &s in &self.selection {
            if !v.contains(&s) {
                v.push(s);
            }
        }
        v
    }

    pub fn set_photos(&self, set: &TraySet) -> Vec<usize> {
        match set {
            TraySet::Folder(fi) => self.folders.get(*fi).map(|f| f.photos.clone()).unwrap_or_default(),
            TraySet::Album(i) => self
                .store
                .albums
                .get(*i)
                .map(|a| a.keys.iter().filter_map(|k| self.key_index.get(k).copied()).collect())
                .unwrap_or_default(),
            TraySet::Person(name) => {
                (0..self.photos.len()).filter(|&id| self.meta(id).is_some_and(|m| m.people.iter().any(|p| p == name))).collect()
            }
            TraySet::Starred => (0..self.photos.len()).filter(|&id| self.is_starred(id)).collect(),
        }
    }

    /// A folder, album or person was clicked in the list: nothing is selected one by one; with no held
    /// pictures the whole set goes into the tray, with held pictures nothing is added.
    pub fn pick_set(&mut self, set: TraySet) {
        self.selection.clear();
        self.anchor = None;
        self.list_pick = Some(set.clone());
        self.tray_set = if self.held.is_empty() { Some(set) } else { None };
    }

    /// After a picture output (export, print, collage): remember the tray if pictures are held, so
    /// adding new ones later asks whether to clear the old ones.
    pub fn arm_tray_check(&mut self) {
        self.tray_check =
            if self.held.is_empty() { None } else { Some(TrayCheck { count: self.tray().len(), old: self.held.clone() }) };
    }

    /// Each frame: wait until no mouse button is down; with none of the old
    /// held pictures left, stop watching; if the tray shrank or stayed, remember the new size; if it
    /// grew, ask once whether to clear the old held pictures.
    pub fn tick_tray_check(&mut self, ctx: &egui::Context) {
        let Some(check) = self.tray_check.clone() else { return };
        if self.dialog.is_some() || ctx.input(|i| i.pointer.any_down()) {
            return;
        }
        if !self.held.iter().any(|id| check.old.contains(id)) {
            self.tray_check = None;
            return;
        }
        let n = self.tray().len();
        if n <= check.count {
            if let Some(c) = &mut self.tray_check {
                c.count = n;
            }
        } else {
            self.tray_check = None;
            self.dialog = Some(Dialog::ClearTray { old: check.old });
        }
    }

    /// The status-bar text for what is in the tray.
    pub fn selection_info(&mut self) -> String {
        let tray = self.tray();
        if tray.len() == 1 {
            let id = tray[0];
            let dims = self.real_dims_of(id);
            let date = self
                .camera_info(id)
                .and_then(|c| c.date_taken.clone())
                .map(|d| exif_date_slashes(&d))
                .unwrap_or_else(|| {
                    let t = self.photos[id].modified;
                    format!("{} {}", library::fmt_date(t), library::fmt_time(t))
                });
            let p = &self.photos[id];
            return selection_text(&[(p.name.clone(), p.modified, p.bytes)], Some(&date), dims);
        }
        let items: Vec<(String, i64, u64)> =
            tray.iter().map(|&id| (self.photos[id].name.clone(), self.photos[id].modified, self.photos[id].bytes)).collect();
        selection_text(&items, None, None)
    }

    /// Photos an action applies to: the tray, or the open photo in the editor.
    pub fn targets(&self) -> Vec<usize> {
        if self.view == View::Edit {
            vec![self.edit.id]
        } else {
            self.tray()
        }
    }

    // ----------------------------------------------------------------- edits

    pub fn push_edit(&mut self, id: usize, op: EditOp) {
        self.meta_mut(id).edits.push(op);
        if self.edit.id == id {
            self.edit.redo.clear();
        }
        self.refresh_thumb(id);
    }

    pub fn rotate(&mut self, ids: &[usize], dir: i32) {
        for &id in ids {
            let m = self.meta_mut(id);
            if let Some(EditOp::Rotate(n)) = m.edits.last_mut() {
                *n += dir;
                if n.rem_euclid(4) == 0 {
                    m.edits.pop();
                }
            } else {
                m.edits.push(EditOp::Rotate(dir));
            }
            self.refresh_thumb(id);
        }
    }

    pub fn toggle_star(&mut self, ids: &[usize]) {
        let all = ids.iter().all(|&id| self.is_starred(id));
        for &id in ids {
            self.meta_mut(id).star = !all;
        }
    }

    pub fn undo(&mut self) {
        let id = self.edit.id;
        if id >= self.photos.len() {
            return;
        }
        if let Some(op) = self.meta_mut(id).edits.pop() {
            self.edit.redo.push(op);
            self.refresh_thumb(id);
        }
    }

    pub fn redo(&mut self) {
        let id = self.edit.id;
        if let Some(op) = self.edit.redo.pop() {
            self.meta_mut(id).edits.push(op);
            self.refresh_thumb(id);
        }
    }

    pub fn refresh_thumb(&mut self, id: usize) {
        if let Some(base) = self.thumb_base.get(&id) {
            let edits = self.edits(id);
            let img = if edits.is_empty() { base.clone() } else { imaging::apply_all(base, &edits) };
            let tex = self.ctx.load_texture(format!("thumb{id}"), to_color_image(&img), TextureOptions::LINEAR);
            self.thumbs.insert(id, tex);
        }
    }

    pub fn request_thumb(&mut self, id: usize) {
        if !self.thumb_req.contains(&id) {
            self.thumb_req.insert(id);
            self.pool.push(Job::Thumb { id, key: self.photos[id].key.clone() }, false);
        }
    }

    pub fn request_big(&mut self, id: usize, urgent: bool) {
        if id < self.photos.len() && !self.big.contains_key(&id) && !self.big_req.contains(&id) {
            self.big_req.insert(id);
            self.pool.push(Job::Big { id, key: self.photos[id].key.clone() }, urgent);
        }
    }

    /// The large, edited picture for the editor or slideshow (None while it loads).
    pub fn big_texture(&mut self, id: usize, extra: &[EditOp]) -> Option<TextureHandle> {
        self.request_big(id, true);
        let mut ops = self.edits(id);
        ops.extend_from_slice(extra);
        let entry = self.big.get_mut(&id)?;
        if let Some((cached, tex)) = &entry.tex {
            if *cached == ops {
                return Some(tex.clone());
            }
        }
        let img = imaging::apply_all(&entry.base, &ops);
        entry.hist = Some(histogram(&img));
        let tex = self.ctx.load_texture(format!("big{id}"), to_color_image(&img), TextureOptions::LINEAR);
        entry.tex = Some((ops, tex.clone()));
        Some(tex)
    }

    /// The large picture with a given edit list: the second version in A|A
    /// (an empty list = the picture as it is on disk, the "before").
    pub fn big_version_texture(&mut self, id: usize, ops: &[EditOp]) -> Option<TextureHandle> {
        self.request_big(id, true);
        let entry = self.big.get_mut(&id)?;
        if let Some((cached, tex)) = &entry.alt_tex {
            if cached == ops {
                return Some(tex.clone());
            }
        }
        let img = imaging::apply_all(&entry.base, ops);
        let tex = self.ctx.load_texture(format!("alt{id}"), to_color_image(&img), TextureOptions::LINEAR);
        entry.alt_tex = Some((ops.to_vec(), tex.clone()));
        Some(tex)
    }

    // ------------------------------------------------------- 2up view (A|B, A|A)

    /// Switch the comparison view (A|A and A|B are
    /// exclusive toggles; entering A|A starts the second version from the picture as on disk).
    pub fn set_compare(&mut self, mode: CompareMode) {
        if self.edit.compare == mode {
            return;
        }
        if self.edit.compare == CompareMode::BeforeAfter {
            self.request_leave_2up(PendingLeave::Mode(mode));
            return;
        }
        self.apply_compare(mode);
    }

    fn apply_compare(&mut self, mode: CompareMode) {
        self.edit.compare = mode;
        self.edit.alt_edits.clear();
        self.edit.focus_second = false;
        if mode != CompareMode::Single {
            self.edit.tool = Tool::None;
            self.edit.sel = None;
        }
    }

    /// Move the focus ("Selected", where edits go) to the other side (swap_2up_focus).
    pub fn swap_2up_focus(&mut self) {
        match self.edit.compare {
            CompareMode::BeforeAfter => {
                let id = self.edit.id;
                let other = std::mem::take(&mut self.edit.alt_edits);
                let current = std::mem::replace(&mut self.meta_mut(id).edits, other);
                self.edit.alt_edits = current;
                self.edit.focus_second = !self.edit.focus_second;
                self.edit.redo.clear();
                self.refresh_thumb(id);
            }
            CompareMode::TwoPhotos => {
                if let Some(o) = self.compare_partner() {
                    let cur = self.edit.id;
                    self.set_edit_photo(o);
                    self.edit.other = Some(cur);
                    self.edit.focus_second = !self.edit.focus_second;
                }
            }
            CompareMode::Single => {}
        }
    }

    /// The two versions of the same picture differ and both carry edits: ask
    /// "Choose Edits" when the 2up view ends.
    pub fn twoup_conflict(&self) -> bool {
        self.edit.compare == CompareMode::BeforeAfter
            && !self.edit.alt_edits.is_empty()
            && self.edit.alt_edits != self.edits(self.edit.id)
    }

    /// Leave A|A (change view, change picture, back to the library), asking which version to keep.
    pub fn request_leave_2up(&mut self, then: PendingLeave) {
        if self.twoup_conflict() && !self.store.dont_ask_end_2up {
            self.dialog = Some(Dialog::ChooseEdits { then, dont_ask: false });
            return;
        }
        // "always use the selected image": the focused version is already the picture's edit list
        self.finish_leave(then);
    }

    /// Keep the version on the given side (false = left/top, true = right/bottom), then continue.
    pub fn keep_2up_side(&mut self, second: bool, then: PendingLeave) {
        if second != self.edit.focus_second {
            let id = self.edit.id;
            let other = std::mem::take(&mut self.edit.alt_edits);
            self.meta_mut(id).edits = other;
            self.refresh_thumb(id);
        }
        self.finish_leave(then);
    }

    fn finish_leave(&mut self, then: PendingLeave) {
        self.edit.alt_edits.clear();
        self.edit.focus_second = false;
        match then {
            PendingLeave::Mode(m) => self.apply_compare(m),
            PendingLeave::Library => {
                self.apply_compare(self.edit.compare);
                self.view = View::Library;
                self.edit.tool = Tool::None;
                self.scroll_to_photo = Some(self.edit.id);
            }
            PendingLeave::Photo(id) => {
                self.set_edit_photo(id);
                self.edit.other = None;
            }
        }
    }

    /// Go to another picture or back to the library, passing through the A|A rules.
    pub fn edit_go(&mut self, then: PendingLeave) {
        if self.edit.compare == CompareMode::BeforeAfter {
            self.request_leave_2up(then);
        } else {
            self.finish_leave(then);
        }
    }

    /// Real photo pixels per pixel of the preview picture (None until the preview is loaded).
    pub fn real_scale(&mut self, id: usize) -> Option<f32> {
        let base_w = self.big.get(&id)?.base.width().max(1);
        let (w, _) = self.real_dims_of(id).unwrap_or((base_w, 1));
        Some(w.max(1) as f32 / base_w as f32)
    }

    /// The photo's real size in pixels, read once from the file header.
    pub fn real_dims_of(&mut self, id: usize) -> Option<(u32, u32)> {
        if !self.real_dims.contains_key(&id) {
            let d = library::source_dims(&self.photos.get(id)?.key)?;
            self.real_dims.insert(id, d);
        }
        self.real_dims.get(&id).copied()
    }

    /// Camera information (EXIF) of a photo, read once and kept.
    pub fn camera_info(&mut self, id: usize) -> Option<Arc<crate::exifinfo::CameraInfo>> {
        if !self.exif_cache.contains_key(&id) {
            let p = self.photos.get(id)?;
            let info = if p.is_sample() { None } else { crate::exifinfo::read(&p.key).map(Arc::new) };
            if let Some(t) = info.as_ref().and_then(|i| i.thumbnail.as_ref()) {
                let tex = self.ctx.load_texture(format!("exifthumb{id}"), to_color_image(t), TextureOptions::LINEAR);
                self.exif_thumbs.insert(id, tex);
            }
            self.exif_cache.insert(id, info);
        }
        self.exif_cache.get(&id).cloned().flatten()
    }

    /// Ask for a sharp render of the picture with these edits (one at a time; a newer request
    /// follows when the edits changed while it was being made).
    pub fn want_zoom_render(&mut self, id: usize, ops: &[EditOp]) {
        if matches!(&self.edit.zoom_tex, Some((zid, zops, _, _)) if *zid == id && zops == ops) {
            return;
        }
        if self.edit.zoom_req.is_some() {
            return;
        }
        self.edit.zoom_req = Some((id, ops.to_vec()));
        self.pool.push(Job::Zoom { id, key: self.photos[id].key.clone(), ops: ops.to_vec() }, true);
    }

    /// The picture shown next to the edited one in "A|B" view.
    pub fn compare_partner(&self) -> Option<usize> {
        let id = self.edit.id;
        if let Some(o) = self.edit.other.filter(|&o| o != id && o < self.photos.len()) {
            return Some(o);
        }
        let list = &self.edit.list;
        let pos = list.iter().position(|&x| x == id)?;
        list.get(pos + 1).or_else(|| pos.checked_sub(1).and_then(|p| list.get(p))).copied()
    }


    // ----------------------------------------------------------------- actions

    pub fn open_editor(&mut self, id: usize) {
        let mut list = self.visible_flat();
        if !list.contains(&id) {
            list = vec![id];
        }
        self.edit.list = list;
        self.set_edit_photo(id);
        // a new editing session starts a fresh second version in A|A
        self.edit.alt_edits.clear();
        self.edit.focus_second = false;
        self.edit.other = None;
        self.view = View::Edit;
    }

    pub fn set_edit_photo(&mut self, id: usize) {
        if self.edit.id != id {
            self.edit.redo.clear();
        }
        self.edit.id = id;
        self.edit.tool = Tool::None;
        self.edit.sel = None;
        self.edit.drag_start = None;
        self.edit.angle = 0.0;
        // keep the zoom level when moving between pictures, but start centred
        self.edit.pan = egui::Vec2::ZERO;
        self.selection.clear();
        self.selection.insert(id);
        // preload neighbours
        if let Some(pos) = self.edit.list.iter().position(|&x| x == id) {
            if let Some(&n) = self.edit.list.get(pos + 1) {
                self.request_big(n, false);
            }
        }
    }

    pub fn start_slideshow(&mut self, list: Vec<usize>, start: usize) {
        if list.is_empty() {
            self.toast("There are no pictures to show.");
            return;
        }
        let now = self.ctx.input(|i| i.time);
        let pos = list.iter().position(|&x| x == start).unwrap_or(0);
        self.slide = Some(Slideshow { list, pos, playing: true, last: now, interval: 3.0, from: self.view, last_move: now });
        self.view = View::Slideshow;
        if self.shot.is_none() {
            self.ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(true));
        }
    }

    pub fn stop_slideshow(&mut self) {
        if let Some(s) = self.slide.take() {
            self.view = if s.from == View::Edit { View::Edit } else { View::Library };
            if s.from == View::Edit {
                if let Some(&id) = s.list.get(s.pos) {
                    self.set_edit_photo(id);
                }
            }
        } else {
            self.view = View::Library;
        }
        self.ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(false));
    }

    pub fn toast(&mut self, text: impl Into<String>) {
        let now = self.ctx.input(|i| i.time);
        self.toast = Some((text.into(), now));
    }

    pub fn import_folder(&mut self) {
        if let Some(dir) = rfd::FileDialog::new().set_title("Choose a folder of pictures").pick_folder() {
            let s = dir.to_string_lossy().to_string();
            if !self.store.roots.contains(&s) {
                self.store.roots.push(s);
                self.dirty = true;
            }
            self.rescan();
        }
    }

    pub fn add_to_album(&mut self, idx: usize, ids: &[usize]) {
        let keys: Vec<String> = ids.iter().map(|&id| self.photos[id].key.clone()).collect();
        if let Some(a) = self.store.albums.get_mut(idx) {
            let mut n = 0;
            for k in keys {
                if !a.keys.contains(&k) {
                    a.keys.push(k);
                    n += 1;
                }
            }
            let name = a.name.clone();
            self.dirty = true;
            self.toast(format!("Added {n} picture(s) to \"{name}\"."));
        }
    }

    pub fn export(&mut self, ids: Vec<usize>, max_dim: u32, quality: u8) {
        if ids.is_empty() {
            self.toast("Select some pictures first.");
            return;
        }
        let Some(dir) = rfd::FileDialog::new().set_title("Export pictures to…").pick_folder() else { return };
        let jobs: Vec<(String, String, Vec<EditOp>)> =
            ids.iter().map(|&id| (self.photos[id].key.clone(), self.photos[id].name.clone(), self.edits(id))).collect();
        let tx = self.tx.clone();
        let ctx = self.ctx.clone();
        self.toast(format!("Exporting {} picture(s)…", jobs.len()));
        std::thread::spawn(move || {
            let mut ok = 0;
            let mut failed: Vec<String> = vec![];
            for (key, name, edits) in &jobs {
                let stem = name.rsplit_once('.').map(|(s, _)| s).unwrap_or(name);
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| export_one(key, edits, &dir, stem, max_dim, quality)))
                    .unwrap_or_else(|_| Err("unexpected error".into()));
                match result {
                    Ok(()) => ok += 1,
                    Err(e) => failed.push(format!("{name}: {e}")),
                }
            }
            let mut msg = format!("Exported {ok} of {} picture(s) to {}", jobs.len(), dir.display());
            if !failed.is_empty() {
                msg.push_str("\n\nNot exported:");
                for f in failed.iter().take(10) {
                    msg.push_str("\n• ");
                    msg.push_str(f);
                }
                if failed.len() > 10 {
                    msg.push_str(&format!("\n… and {} more", failed.len() - 10));
                }
            }
            let _ = tx.send(Done::Message(msg));
            ctx.request_repaint();
        });
        self.arm_tray_check();
    }

    pub fn collage(&mut self, ids: Vec<usize>) {
        if ids.len() < 2 {
            self.toast("Put at least two pictures in the tray to make a collage.");
            return;
        }
        let Some(path) = rfd::FileDialog::new().set_file_name("Collage.png").add_filter("PNG picture", &["png"]).save_file()
        else {
            return;
        };
        let jobs: Vec<(String, Vec<EditOp>)> = ids.iter().map(|&id| (self.photos[id].key.clone(), self.edits(id))).collect();
        let tx = self.tx.clone();
        let ctx = self.ctx.clone();
        self.toast("Making the collage…");
        // the picker may return a name without the extension; the picture is always PNG
        let path = if path.extension().is_some_and(|e| e.eq_ignore_ascii_case("png")) { path } else { path.with_extension("png") };
        std::thread::spawn(move || {
            let made = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let pics: Vec<RgbaImage> = jobs
                    .iter()
                    .filter_map(|(k, e)| imaging::load_source(k, 900).map(|i| imaging::apply_all(&i, e)))
                    .collect();
                imaging::make_collage(&pics, 600)
            }));
            let msg = match made.map(|out| out.save(&path)) {
                Ok(Ok(())) => format!("Collage saved to {}", path.display()),
                Ok(Err(e)) => format!("Could not save the collage: {e}"),
                Err(_) => "Could not make the collage: unexpected error.".to_string(),
            };
            let _ = tx.send(Done::Message(msg));
            ctx.request_repaint();
        });
        self.arm_tray_check();
    }

    pub fn service_gone(&mut self, what: &str) {
        self.dialog = Some(Dialog::Info {
            title: what.to_string(),
            text: format!(
                "\"{what}\" used an online service that no longer exists, so it isn't available in this version.\n\n\
                 Tip: use Export to save copies of your pictures, then share them any way you like."
            ),
        });
    }

    // ----------------------------------------------------------------- frame

    fn process_results(&mut self) {
        while let Ok(done) = self.rx.try_recv() {
            match done {
                Done::Thumb { id, key, img } => {
                    if self.photos.get(id).is_some_and(|p| p.key == key) {
                        match img {
                            Some(img) => {
                                self.thumb_base.insert(id, img);
                                self.refresh_thumb(id);
                            }
                            None => {
                                self.thumb_failed.insert(id);
                            }
                        }
                    }
                }
                Done::Big { id, key, img } => {
                    if self.photos.get(id).is_some_and(|p| p.key == key) {
                        self.big_req.remove(&id);
                        if let Some(img) = img {
                            self.big.insert(id, BigEntry { base: img, tex: None, alt_tex: None, hist: None });
                            self.big_order.push_back(id);
                            while self.big_order.len() > 8 {
                                if let Some(old) = self.big_order.pop_front() {
                                    if old != self.edit.id {
                                        self.big.remove(&old);
                                    } else {
                                        self.big_order.push_back(old);
                                    }
                                }
                            }
                        } else {
                            self.thumb_failed.insert(id);
                        }
                    }
                }
                Done::Zoom { id, key, ops, img, unedited_width } => {
                    self.edit.zoom_req = None;
                    if self.photos.get(id).is_some_and(|p| p.key == key) && id == self.edit.id {
                        if let Some(img) = img {
                            let real_w = library::source_dims(&key).map(|d| d.0).unwrap_or(unedited_width);
                            let r = real_w.max(1) as f32 / unedited_width.max(1) as f32;
                            let tex = self.ctx.load_texture(format!("zoom{id}"), to_color_image(&img), TextureOptions::LINEAR);
                            self.edit.zoom_tex = Some((id, ops, tex, r));
                        }
                    }
                }
                Done::Scan(found, truncated) => {
                    self.scanning = false;
                    if found.is_empty() {
                        self.set_library(library::sample_folders(), true);
                    } else {
                        self.set_library(found, false);
                    }
                    if truncated {
                        self.toast(format!(
                            "Only the first {} pictures are shown: remove some folders from the library to see the rest.",
                            fmt_count(library::SCAN_LIMIT)
                        ));
                    }
                }
                Done::Message(m) => {
                    // don't throw away a dialog the user is filling in
                    if self.dialog.is_none() {
                        self.dialog = Some(Dialog::Info { title: "Done".into(), text: m });
                    } else {
                        self.toast(m);
                    }
                }
            }
        }
    }

    fn handle_screenshot(&mut self, ctx: &egui::Context) {
        let Some(path) = self.shot.clone() else { return };
        self.frames += 1;
        ctx.request_repaint();
        let taken = self.shot_slot.lock().unwrap().take();
        if let Some(img) = taken {
            let [w, h] = img.size;
            let raw: Vec<u8> = img.pixels.iter().flat_map(|c| c.to_array()).collect();
            if let Some(buf) = RgbaImage::from_raw(w as u32, h as u32, raw) {
                if let Some(dir) = path.parent() {
                    let _ = std::fs::create_dir_all(dir);
                }
                if let Err(e) = buf.save(&path) {
                    eprintln!("could not save screenshot: {e}");
                }
            }
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            return;
        }
        let loaded = self.thumb_req.iter().all(|id| self.thumbs.contains_key(id) || self.thumb_failed.contains(id))
            && self.edit.zoom_req.is_none()
            && self.big_req.is_empty();
        if !self.shot_requested && self.frames >= 120 && (loaded || self.frames > 900) {
            self.shot_requested = true;
            let slot = self.shot_slot.clone();
            let c2 = ctx.clone();
            ctx.request_screenshot(move |img| {
                *slot.lock().unwrap() = Some(img);
                c2.request_repaint();
            });
        }
        if self.frames > 3000 {
            eprintln!("screenshot timed out");
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }

    pub fn text_focused(&self) -> bool {
        self.ctx.memory(|m| m.focused().is_some())
    }

    fn dialogs(&mut self, ctx: &egui::Context) {
        let Some(mut dialog) = self.dialog.take() else { return };
        let mut open = true;
        let mut close = false;
        let title = match &dialog {
            Dialog::Info { title, .. } => title.clone(),
            Dialog::NewAlbum { .. } => "New album".into(),
            Dialog::RenameAlbum { .. } => "Rename album".into(),
            Dialog::Tag { .. } => "Tags and people".into(),
            Dialog::Export { .. } => "Export pictures".into(),
            Dialog::FolderNote { .. } => "Folder description".into(),
            Dialog::FolderManager => "Folder manager".into(),
            Dialog::Collage => "Make a collage".into(),
            Dialog::Movie => "Movie".into(),
            Dialog::About => "About AlbumCraft".into(),
            Dialog::ChooseEdits { .. } => "Choose Edits".into(),
            Dialog::PrintReview { .. } => "Review".into(),
            Dialog::ClearTray { .. } => "Photo tray".into(),
        };
        let targets = self.targets();
        egui::Window::new(title)
            .collapsible(false)
            .resizable(false)
            .open(&mut open)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .default_width(380.0)
            .show(ctx, |ui| match &mut dialog {
                Dialog::Info { text, .. } => {
                    ui.label(text.as_str());
                    ui.add_space(8.0);
                    if ui.button("OK").clicked() {
                        close = true;
                    }
                }
                Dialog::NewAlbum { name } => {
                    ui.label("Album name:");
                    let r = ui.text_edit_singleline(name);
                    r.request_focus();
                    if !targets.is_empty() {
                        ui.label(format!("The {} picture(s) in the tray will be added.", targets.len()));
                    }
                    ui.horizontal(|ui| {
                        let enter = r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                        if (ui.button("Create").clicked() || enter) && !name.trim().is_empty() {
                            self.store.albums.push(Album { name: name.trim().to_string(), keys: vec![] });
                            let idx = self.store.albums.len() - 1;
                            self.dirty = true;
                            if !targets.is_empty() {
                                self.add_to_album(idx, &targets);
                            }
                            self.source = Source::Album(idx);
                            close = true;
                        }
                        if ui.button("Cancel").clicked() {
                            close = true;
                        }
                    });
                }
                Dialog::RenameAlbum { idx, name } => {
                    ui.text_edit_singleline(name);
                    ui.horizontal(|ui| {
                        if ui.button("Rename").clicked() && !name.trim().is_empty() {
                            if let Some(a) = self.store.albums.get_mut(*idx) {
                                a.name = name.trim().to_string();
                                self.dirty = true;
                            }
                            close = true;
                        }
                        if ui.button("Cancel").clicked() {
                            close = true;
                        }
                    });
                }
                Dialog::Tag { tags, people } => {
                    ui.label(format!("Add to the {} picture(s) in the tray. Separate words with commas.", targets.len()));
                    egui::Grid::new("tagdlg").num_columns(2).show(ui, |ui| {
                        ui.label("Tags:");
                        ui.text_edit_singleline(tags);
                        ui.end_row();
                        ui.label("People:");
                        ui.text_edit_singleline(people);
                        ui.end_row();
                    });
                    ui.horizontal(|ui| {
                        if ui.button("Add").clicked() {
                            let split = |s: &str| -> Vec<String> {
                                s.split(',').map(|t| t.trim().to_string()).filter(|t| !t.is_empty()).collect()
                            };
                            let (t, p) = (split(tags), split(people));
                            for &id in &targets {
                                let m = self.meta_mut(id);
                                for x in &t {
                                    if !m.tags.contains(x) {
                                        m.tags.push(x.clone());
                                    }
                                }
                                for x in &p {
                                    if !m.people.contains(x) {
                                        m.people.push(x.clone());
                                    }
                                }
                            }
                            close = true;
                        }
                        if ui.button("Cancel").clicked() {
                            close = true;
                        }
                    });
                }
                Dialog::Export { max_dim, quality } => {
                    ui.label(format!("{} picture(s) will be saved as new JPEG files with your edits applied.", targets.len()));
                    ui.label("Your original files are never changed.");
                    ui.add_space(4.0);
                    ui.label("Image size:");
                    ui.radio_value(max_dim, 0, "Use original size");
                    ui.radio_value(max_dim, 1600, "Large (1600 pixels)");
                    ui.radio_value(max_dim, 800, "Small, for email (800 pixels)");
                    ui.add(egui::Slider::new(quality, 50..=100).text("JPEG quality"));
                    ui.horizontal(|ui| {
                        if ui.button("Choose folder and export…").clicked() {
                            let (m, q) = (*max_dim, *quality);
                            self.export(targets.clone(), m, q);
                            close = true;
                        }
                        if ui.button("Cancel").clicked() {
                            close = true;
                        }
                    });
                }
                Dialog::FolderNote { path, text } => {
                    ui.label("A short description shown under the folder name:");
                    ui.text_edit_multiline(text);
                    ui.horizontal(|ui| {
                        if ui.button("Save").clicked() {
                            if text.trim().is_empty() {
                                self.store.folder_notes.remove(path.as_str());
                            } else {
                                self.store.folder_notes.insert(path.clone(), text.trim().to_string());
                            }
                            self.dirty = true;
                            close = true;
                        }
                        if ui.button("Cancel").clicked() {
                            close = true;
                        }
                    });
                }
                Dialog::FolderManager => {
                    ui.label("These folders on your computer are watched for pictures:");
                    let mut remove = None;
                    if self.store.roots.is_empty() {
                        ui.weak("(none yet — sample pictures are shown instead)");
                    }
                    for (i, r) in self.store.roots.iter().enumerate() {
                        ui.horizontal(|ui| {
                            ui.label(format!("📁 {r}"));
                            if ui.small_button("Remove").clicked() {
                                remove = Some(i);
                            }
                        });
                    }
                    if let Some(i) = remove {
                        self.store.roots.remove(i);
                        self.dirty = true;
                        self.rescan();
                    }
                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        if ui.button("➕ Add folder…").clicked() {
                            self.import_folder();
                        }
                        if ui.button("⟳ Scan again").clicked() {
                            self.rescan();
                        }
                        if ui.button("Close").clicked() {
                            close = true;
                        }
                    });
                }
                Dialog::Collage => {
                    ui.label(format!("Arrange the {} picture(s) in the tray in a grid and save it as a picture.", targets.len()));
                    ui.horizontal(|ui| {
                        if ui.button("Save collage…").clicked() {
                            self.collage(targets.clone());
                            close = true;
                        }
                        if ui.button("Cancel").clicked() {
                            close = true;
                        }
                    });
                }
                Dialog::Movie => {
                    ui.label("Making a movie file isn't part of this version.");
                    ui.label("You can play the same pictures as a full-screen slideshow instead.");
                    ui.horizontal(|ui| {
                        if ui.button("▶ Play slideshow").clicked() {
                            let list = if targets.len() > 1 { targets.clone() } else { self.visible_flat() };
                            let first = list.first().copied().unwrap_or(0);
                            self.start_slideshow(list, first);
                            close = true;
                        }
                        if ui.button("Cancel").clicked() {
                            close = true;
                        }
                    });
                }
                Dialog::ChooseEdits { then, dont_ask } => {
                    ui.label("The same image has two different edits.  Which one would you like to keep?");
                    ui.add_space(6.0);
                    let (first, second) = if self.edit.vertical { ("Top", "Bottom") } else { ("Left", "Right") };
                    let then = *then;
                    let mut keep = None;
                    ui.horizontal(|ui| {
                        if ui.button(first).clicked() {
                            keep = Some(false);
                        }
                        if ui.button(second).clicked() {
                            keep = Some(true);
                        }
                    });
                    ui.checkbox(dont_ask, "Don't ask again, always use the selected image");
                    if let Some(second) = keep {
                        if *dont_ask {
                            self.store.dont_ask_end_2up = true;
                            self.dirty = true;
                        }
                        self.keep_2up_side(second, then);
                        close = true;
                    }
                }
                Dialog::ClearTray { old } => {
                    ui.label(MSG_CLEAR_TRAY);
                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        // "Clear Tray" removes only the pictures held before; the
                        // new ones stay
                        if ui.button(BTN_CLEAR_TRAY).clicked() {
                            self.held.retain(|id| !old.contains(id));
                            close = true;
                        }
                        if ui.button(BTN_DONT_CLEAR).clicked() {
                            close = true;
                        }
                    });
                }
                Dialog::PrintReview { grades, before_print } => {
                    let bad: Vec<usize> = grades.iter().filter(|g| g.3 == 0).map(|g| g.0).collect();
                    ui.label(if bad.is_empty() { crate::print::MSG_READY } else { crate::print::MSG_TOO_SMALL });
                    ui.add_space(6.0);
                    egui::ScrollArea::vertical().max_height(260.0).show(ui, |ui| {
                        egui::Grid::new("print_review").num_columns(2).striped(true).spacing([12.0, 3.0]).show(ui, |ui| {
                            for (_, name, ppi, grade) in grades.iter() {
                                ui.label(name.as_str());
                                let colour = match grade {
                                    2 => t().good,
                                    1 => t().text_weak,
                                    _ => t().bad,
                                };
                                ui.label(egui::RichText::new(crate::print::quality_text(*grade, *ppi)).color(colour));
                                ui.end_row();
                            }
                        });
                    });
                    ui.add_space(6.0);
                    let before = *before_print;
                    ui.horizontal(|ui| {
                        if before && !bad.is_empty() {
                            if ui.button("Remove These Pictures").clicked() {
                                close = true;
                                self.print_remove_bad(&bad);
                            }
                            if ui.button("Print Anyway").clicked() {
                                close = true;
                                self.start_print_job();
                            }
                            if ui.button("Cancel").clicked() {
                                close = true;
                            }
                        } else if ui.button("OK").clicked() {
                            close = true;
                        }
                    });
                }
                Dialog::About => {
                    ui.heading("AlbumCraft");
                    ui.label("A desktop photo organizer: browse your picture folders, make albums, star favourites, fix and touch up photos, and play slideshows.");
                    ui.label("Edits never change your original files; they are applied when you look at or export a picture.");
                    if ui.button("OK").clicked() {
                        close = true;
                    }
                }
            });
        if open && !close {
            self.dialog = Some(dialog);
        }
    }

    fn toast_ui(&mut self, ctx: &egui::Context) {
        if let Some((text, t)) = &self.toast {
            let now = ctx.input(|i| i.time);
            if now - t > 3.5 {
                self.toast = None;
                return;
            }
            let text = text.clone();
            #[allow(deprecated)]
            let toast_id = egui::Id::new("toast");
            egui::Area::new(toast_id)
                .anchor(egui::Align2::CENTER_BOTTOM, [0.0, -150.0])
                .show(ctx, |ui| {
                    egui::Frame::new()
                        .fill(Color32::from_rgba_unmultiplied(40, 44, 52, 235))
                        .corner_radius(6)
                        .inner_margin(egui::Margin::symmetric(14, 8))
                        .show(ui, |ui| ui.label(egui::RichText::new(text).color(Color32::WHITE)));
                });
            ctx.request_repaint_after(std::time::Duration::from_millis(250));
        }
    }
}

/// Export one picture with its edits as `<stem>.jpg` in `dir` (or `<stem>-1.jpg`, … when taken).
/// Never overwrites a file, and removes its own half-written file when something fails.
fn export_one(key: &str, edits: &[EditOp], dir: &std::path::Path, stem: &str, max_dim: u32, quality: u8) -> Result<(), String> {
    let src = imaging::load_source(key, max_dim).ok_or("the picture could not be read")?;
    let img = imaging::apply_all(&src, edits);
    let rgb = image::DynamicImage::ImageRgba8(img).to_rgb8();
    let mut n = 0;
    let (file, out) = loop {
        let out = if n == 0 { dir.join(format!("{stem}.jpg")) } else { dir.join(format!("{stem}-{n}.jpg")) };
        match std::fs::OpenOptions::new().write(true).create_new(true).open(&out) {
            Ok(f) => break (f, out),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists && n < 10_000 => n += 1,
            Err(e) => return Err(e.to_string()),
        }
    };
    let write = || -> Result<(), String> {
        use std::io::Write;
        let mut w = std::io::BufWriter::new(file);
        let enc = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut w, quality);
        rgb.write_with_encoder(enc).map_err(|e| e.to_string())?;
        w.flush().map_err(|e| e.to_string())
    };
    write().inspect_err(|_| {
        let _ = std::fs::remove_file(&out);
    })
}

/// "2009:05:12 10:00:00" -> "2009/05/12 10:00:00" (only the date part is
/// changed so the time stays readable).
pub fn exif_date_slashes(d: &str) -> String {
    let (date, rest) = d.split_at(d.find(' ').unwrap_or(d.len()));
    format!("{}{}", date.replace(':', "/"), rest)
}

/// A count with thousands separators from 1000 up.
pub fn fmt_count(n: usize) -> String {
    let s = n.to_string();
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    out
}

/// The selection line:
/// none -> "No Selection"; one -> "%s     %s     %dx%d pixels     %s" (name, date, size, bytes);
/// several -> "%s pictures" + "     %s to %s     %s on disk" (earliest / latest date, total bytes),
/// or "     %s      %s on disk" when both dates read the same.
pub fn selection_text(items: &[(String, i64, u64)], one_date: Option<&str>, dims: Option<(u32, u32)>) -> String {
    match items {
        [] => "No Selection".into(),
        [(name, modified, bytes)] => {
            let date = one_date.map(str::to_string).unwrap_or_else(|| library::fmt_date(*modified));
            let (w, h) = dims.unwrap_or((0, 0));
            format!("{name}     {date}     {w}x{h} pixels     {}", library::fmt_bytes(*bytes))
        }
        _ => {
            let first = items.iter().map(|i| i.1).min().unwrap_or(0);
            let last = items.iter().map(|i| i.1).max().unwrap_or(0);
            let total: u64 = items.iter().map(|i| i.2).sum();
            let (a, b) = (library::fmt_date(first), library::fmt_date(last));
            let mut s = format!("{} pictures", fmt_count(items.len()));
            if a == b {
                s += &format!("     {a}      {} on disk", library::fmt_bytes(total));
            } else {
                s += &format!("     {a} to {b}     {} on disk", library::fmt_bytes(total));
            }
            s
        }
    }
}

#[cfg(test)]
mod tray_tests {
    use super::*;

    #[test]
    fn export_never_overwrites() {
        let dir = std::env::temp_dir().join(format!("albumcraft-export-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("a.jpg"), b"keep me").unwrap();
        let key = format!("{}1", imaging::SAMPLE_PREFIX);
        export_one(&key, &[], &dir, "a", 64, 80).unwrap();
        export_one(&key, &[], &dir, "a", 64, 80).unwrap();
        let unreadable = export_one("/no/such/picture.jpg", &[], &dir, "b", 64, 80);
        let kept = std::fs::read(dir.join("a.jpg")).unwrap();
        let made = [dir.join("a-1.jpg"), dir.join("a-2.jpg")].map(|p| image::image_dimensions(p).ok());
        let b_exists = dir.join("b.jpg").exists();
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(kept, b"keep me");
        assert!(made.iter().all(|d| d.is_some_and(|(w, h)| w.max(h) == 64)));
        assert!(unreadable.is_err() && !b_exists);
    }

    #[test]
    fn selection_line() {
        assert_eq!(selection_text(&[], None, None), "No Selection");
        let day = 1_242_122_400; // 12 May 2009
        let one = [("a.jpg".to_string(), day, 2 << 20)];
        assert_eq!(selection_text(&one, Some("2009/05/12 10:00:00"), Some((3000, 2000))), "a.jpg     2009/05/12 10:00:00     3000x2000 pixels     2.0 MB");
        let two = [("a.jpg".to_string(), day, 1 << 20), ("b.jpg".to_string(), day + 60, 1 << 20)];
        assert_eq!(selection_text(&two, None, None), "2 pictures     May 12, 2009      2.0 MB on disk");
        let apart = [("a.jpg".to_string(), day, 1 << 20), ("b.jpg".to_string(), day + 86_400 * 2, 1 << 20)];
        assert_eq!(selection_text(&apart, None, None), "2 pictures     May 12, 2009 to May 14, 2009     2.0 MB on disk");
        assert_eq!(fmt_count(999), "999");
        assert_eq!(fmt_count(1234), "1,234");
        assert_eq!(fmt_count(1234567), "1,234,567");
        assert_eq!(exif_date_slashes("2009:05:12 10:00:00"), "2009/05/12 10:00:00");
    }
}

impl eframe::App for App {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        crate::theme::apply(ctx, crate::theme::Mode::from_name(&self.store.theme), false);
        self.process_results();
        self.handle_screenshot(ctx);
        let now = ctx.input(|i| i.time);
        let closing = ctx.input(|i| i.viewport().close_requested());
        // every change to the library's notes sets `dirty`: rebuild the cached lists after it
        if self.dirty {
            self.list_rev += 1;
        }
        if self.persist && self.dirty && (closing || now - self.last_save > 3.0) {
            self.last_save = now;
            match self.store.save() {
                Ok(()) => {
                    self.dirty = false;
                    self.save_error = None;
                }
                Err(e) => {
                    // stay dirty: it is tried again in a few seconds
                    if self.save_error.as_ref() != Some(&e) {
                        self.toast(format!("Your library could not be saved: {e}"));
                        self.save_error = Some(e);
                    }
                }
            }
        }
        if self.dirty && self.persist {
            ctx.request_repaint_after(std::time::Duration::from_secs(3));
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        match self.view {
            View::Library => self.library_ui(ui),
            View::Edit => self.edit_ui(ui),
            View::Slideshow => self.slideshow_ui(ui),
            View::Print => self.print_ui(ui),
        }
        // Ctrl+P opens printing from the library or the editor
        if matches!(self.view, View::Library | View::Edit)
            && self.dialog.is_none()
            && ctx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::P))
        {
            self.open_print();
        }
        self.tick_tray_check(&ctx);
        self.dialogs(&ctx);
        self.toast_ui(&ctx);
    }
}
