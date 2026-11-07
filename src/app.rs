//! Application state, shared helpers, dialogs and the frame loop.

use crate::icons;
use crate::imaging::{self, EditOp};
use crate::backend::{Library, LocalLibrary};
use crate::library::{self, Album, Done, Flag, Folder, Job, Loaded, Meta, Photo, ScannedFolder, Store};
use eframe::egui::{self, Color32, ColorImage, TextureHandle, TextureOptions};
use image::RgbaImage;
use std::collections::{BTreeSet, HashMap, HashSet, VecDeque};
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Mutex};

pub use crate::theme::t;

/// What the library list depends on (its cache key).
type ListKey = (u64, u64, Source, String, Vec<SearchTerm>, Filter);

/// A word chosen from the search suggestions: matches exactly (any letter case), where typed text
/// matches anywhere.
#[derive(Clone, PartialEq, Debug)]
pub enum SearchTerm {
    Tag(String),
    /// A place name set by the user, a city, a region or a country.
    Place(String),
    Person(String),
}

#[derive(Clone, PartialEq, Debug)]
pub enum Source {
    Library,
    Album(usize),
    Person(String),
    Picks,
}

/// Which pictures are shown, on top of the source and the search.
#[derive(Clone, PartialEq, Debug, Default)]
pub struct Filter {
    /// Only picks.
    pub picks: bool,
    /// At least this many stars (0 = any).
    pub min_rating: u8,
    /// Leave rejected pictures out.
    pub hide_rejects: bool,
}

impl Filter {
    pub fn is_active(&self) -> bool {
        *self != Filter::default()
    }
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
    Picks,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum View {
    Library,
    Edit,
    Slideshow,
    Print,
    /// Places: the map of where pictures were taken (also used to set a location).
    Map,
    /// Copying pictures from a card, camera or folder into the library.
    Import,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Tab {
    /// One-click fixes and tools (Basic) or the adjustment sliders (Advanced).
    Edit,
    Presets,
    Effects,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Tool {
    None,
    Crop,
    Redeye,
    Straighten,
    /// Click something that should be gray to set the white balance.
    PickNeutral,
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
    /// Make a web gallery of the tray.
    Gallery { title: String, style: crate::gallery::Style, size: u32, captions: bool },
    FolderNote { path: String, text: String },
    FolderManager,
    Collage,
    /// Make a movie of the tray (`hd` = 1920×1080, else 1280×720).
    Movie { hd: bool },
    About,
    /// "Choose Edits" when leaving A|A with two different versions.
    ChooseEdits { then: PendingLeave, dont_ask: bool },
    /// "Would you like to clear your old held items from the tray?"
    ClearTray { old: Vec<usize> },
    /// Print review: (picture, name, pixels per inch, grade 0 bad / 1 good / 2 best).
    PrintReview { grades: Vec<(usize, String, f32, u8)>, before_print: bool },
    /// Asked once: may maps and place search use the internet?
    OnlineMaps,
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
    /// The Edit tab shows the sliders (Advanced) instead of the one-click fixes (Basic).
    pub advanced: bool,
    pub tool: Tool,
    pub aspect: CropAspect,
    pub angle: f32,
    pub drag_start: Option<egui::Pos2>,
    pub sel: Option<[f32; 4]>,
    pub redo: Vec<EditOp>,
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
    /// Zoom around this point (from the view's centre) and keep it in place, instead of ending
    /// centred. Used when a drag on the picture zooms in; the drag's movement is added to `pan_from`.
    pub focus: Option<egui::Vec2>,
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
    /// When the current picture appeared (the clock of its motion and of auto-advance).
    pub started: f64,
    /// The picture before, still fading out: (photo id, position in the list, when it appeared).
    pub prev: Option<(usize, usize, f64)>,
    /// While paused: when the pause began (motion and timers stand still).
    pub paused_at: Option<f64>,
    pub from: View,
    pub last_move: f64,
    pub music: Option<crate::slides::Music>,
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
    /// Places: GPS index, map view and "Set location" state (map_view.rs).
    pub geo: crate::map_view::GeoState,
    /// The control connection for the MCP server (see control.rs) and its queue of commands.
    control: Option<(crate::control::Server, Receiver<crate::control::Request>)>,
    icon_tex: Option<TextureHandle>,
    /// Whether FFmpeg (for movies) was found; looked up when the Movie window first opens.
    pub ffmpeg: Option<bool>,
    /// Edits copied from a picture, ready to paste onto others.
    pub copied_edits: Option<Vec<EditOp>>,
    /// Bumped whenever the library or its notes may have changed: invalidates `list_cache`.
    pub list_rev: u64,
    /// `groups()` for (list_rev, source, search, filter).
    #[allow(clippy::type_complexity)]
    list_cache: std::cell::RefCell<Option<(ListKey, Vec<(Option<usize>, Vec<usize>)>)>>,

    pub photos: Vec<Photo>,
    pub folders: Vec<Folder>,
    pub key_index: HashMap<String, usize>,
    pub using_samples: bool,
    pub scanning: bool,

    pub thumb_base: HashMap<usize, RgbaImage>,
    pub thumbs: HashMap<usize, TextureHandle>,
    pub thumb_req: HashSet<usize>,
    /// When each thumbnail was last drawn (window time), to forget the oldest (`trim_thumbs`).
    pub thumb_seen: HashMap<usize, f64>,
    /// Width / height of thumbnails that were forgotten (keeps the grid's layout steady).
    pub thumb_aspect: HashMap<usize, f32>,
    /// Every tag of the library, sorted, and the `list_rev` it was made at (info panel).
    pub tag_cache: Option<(u64, Arc<Vec<String>>)>,
    pub thumb_failed: HashSet<usize>,
    pub big: HashMap<usize, BigEntry>,
    pub big_order: VecDeque<usize>,
    pub big_req: HashSet<usize>,
    /// Small previews of every preset look for (picture, edits they start from).
    pub look_previews: Option<(usize, Vec<EditOp>, Vec<TextureHandle>)>,

    /// Where the pictures and notes are read and saved (see backend.rs).
    pub lib: Box<dyn Library>,
    pub tx: Sender<Done>,
    pub rx: Receiver<Done>,

    pub view: View,
    pub source: Source,
    pub search: String,
    /// Words chosen from the suggestions, shown as chips in the search box; all must match.
    pub search_terms: Vec<SearchTerm>,
    /// The highlighted suggestion (↑ / ↓), 0 = "search for the typed text".
    pub search_hl: usize,
    /// The suggestion list under the search box is open.
    pub search_popup: bool,
    /// Tags, places and people with counts, for the suggestions (cached until they change).
    pub search_index: Option<((u64, u64, usize), Arc<crate::search_box::SearchIndex>)>,
    pub filter: Filter,
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
    /// A `Job::Dims` is out (one at a time, so it never holds up the thumbnails).
    pub dims_pending: bool,
    /// Camera information (EXIF) per photo; None = the file has none.
    pub exif_cache: HashMap<usize, Option<Arc<crate::exifinfo::CameraInfo>>>,
    /// The camera's own embedded thumbnail, as a texture.
    pub exif_thumbs: HashMap<usize, TextureHandle>,

    pub edit: EditState,
    pub slide: Option<Slideshow>,
    pub print: Option<crate::print_view::PrintState>,
    pub dialog: Option<Dialog>,
    pub toast: Option<(String, f64)>,
    /// The import view's state (only while it is open).
    pub import: Option<Box<crate::import_view::ImportState>>,
    /// A copy running in the background (shown in the status bar).
    pub importing: Option<crate::import_view::Importing>,
    /// The folder to show once the scan after an import is done.
    pub show_after_scan: Option<PathBuf>,

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
        let t_new = std::time::Instant::now();
        let ctx = cc.egui_ctx.clone();
        crate::theme::apply(&ctx, crate::theme::Mode::System, true);
        crate::theme::set_fonts(&ctx);
        let (tx, rx) = std::sync::mpsc::channel();
        let wake_ctx = ctx.clone();
        let mut lib: Box<dyn Library> = Box::new(LocalLibrary::new(tx.clone(), Arc::new(move || wake_ctx.request_repaint())));

        let mut persist = shot.is_none();
        let mut first_run = false;
        let mut load_problem = None;
        let mut store = if persist {
            match lib.load() {
                Loaded::Store(s) => *s,
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
        if first_run
            && let Some(p) = dirs::picture_dir() {
                store.roots.push(p.to_string_lossy().to_string());
            }

        let mut app = App {
            store,
            dirty: first_run,
            last_save: 0.0,
            persist,
            save_error: None,
            geo: Default::default(),
            list_rev: 0,
            copied_edits: None,
            ffmpeg: None,
            icon_tex: None,
            control: None,
            list_cache: Default::default(),
            photos: vec![],
            folders: vec![],
            key_index: HashMap::new(),
            using_samples: false,
            scanning: false,
            thumb_base: HashMap::new(),
            thumbs: HashMap::new(),
            thumb_req: HashSet::new(),
            thumb_seen: HashMap::new(),
            thumb_aspect: HashMap::new(),
            tag_cache: None,
            thumb_failed: HashSet::new(),
            big: HashMap::new(),
            big_order: VecDeque::new(),
            big_req: HashSet::new(),
            look_previews: None,
            lib,
            tx,
            rx,
            view: View::Library,
            source: Source::Library,
            search: String::new(),
            search_terms: vec![],
            search_hl: 0,
            search_popup: false,
            search_index: None,
            filter: Filter::default(),
            selection: BTreeSet::new(),
            anchor: None,
            held: vec![],
            scroll_to_folder: None,
            scroll_to_photo: None,
            tray_set: None,
            list_pick: None,
            tray_check: None,
            real_dims: HashMap::new(),
            dims_pending: false,
            exif_cache: HashMap::new(),
            exif_thumbs: HashMap::new(),
            edit: EditState {
                id: 0,
                list: vec![],
                tab: Tab::Edit,
                advanced: false,
                tool: Tool::None,
                aspect: CropAspect::Manual,
                angle: 0.0,
                drag_start: None,
                sel: None,
                redo: vec![],
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
            import: None,
            importing: None,
            show_after_scan: None,
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
                    let m = app.store.meta.entry(key.clone()).or_default();
                    m.flag = Flag::Pick;
                    m.rating = [5, 4, 3, 4][i];
                    if i < 3 {
                        app.store.albums[0].keys.push(key);
                    }
                }
            }
            // one rejected picture, to show the mark
            if let Some(p) = app.photos.get(6) {
                app.store.meta.entry(p.key.clone()).or_default().flag = Flag::Reject;
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
                m.edits.push(EditOp::Adjust(crate::develop::Adjust {
                    temp: 18.0,
                    exposure: 0.3,
                    shadows: 30.0,
                    contrast: 15.0,
                    vibrance: 25.0,
                    clarity: 20.0,
                    ..Default::default()
                }));
                app.open_editor(0);
                app.edit.compare = CompareMode::BeforeAfter;
                app.edit.tab = Tab::Edit;
                app.edit.advanced = true;
            }
            if shot_view == "import" {
                app.demo_import();
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
                        // a place and tags, and every section open, to show the whole panel
                        let key = app.photos[0].key.clone();
                        let m = app.store.meta.entry(key).or_default();
                        m.place = Some(crate::geo::Place::Set { lat: 46.0717, lon: 24.7928, name: "Sighișoara".into() });
                        m.tags = vec!["Travel".into(), "Old town".into()];
                        app.open_editor(0);
                        app.store.info_open = true;
                        app.edit.tag_buf = "o".into();
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
                m.edits.push(EditOp::AutoContrast);
                m.edits.push(EditOp::Look { look: crate::looks::Look::Vivid, amount: 0.6 });
                app.open_editor(2);
                app.store.info_open = true;
            }
            if shot_view == "effects" && app.photos.len() > 4 {
                let k = app.photos[4].key.clone();
                let m = app.store.meta.entry(k).or_default();
                m.edits.push(EditOp::Decor(crate::decor::Decor::Border { width: 0.05, bottom: 0.15, color: [250, 248, 242] }));
                m.edits.push(EditOp::Decor(crate::decor::Decor::Text(crate::decor::TextOverlay { text: "Summer 2009".into(), anchor: 7, size: 0.07, color: [40, 40, 40], halo: false })));
                app.open_editor(4);
                app.edit.tab = Tab::Effects;
            }
            if shot_view == "slideshow" {
                let list: Vec<usize> = (0..app.photos.len()).collect();
                app.start_slideshow(list, 0);
            }
            if shot_view == "movie" {
                app.selection = [0usize, 1, 2, 3].into_iter().collect();
                app.dialog = Some(Dialog::Movie { hd: true });
            }
            if shot_view == "presets" && app.photos.len() > 3 {
                let k = app.photos[3].key.clone();
                app.store.meta.entry(k).or_default().edits.push(EditOp::Look { look: crate::looks::Look::Polaroid, amount: 0.8 });
                app.open_editor(3);
                app.edit.tab = Tab::Presets;
            }
            if matches!(shot_view.as_str(), "map" | "place" | "pin") {
                // ALBUMCRAFT_SHOT_ONLINE=1: show the online map (the screenshot saves nothing)
                if std::env::var_os("ALBUMCRAFT_SHOT_ONLINE").is_some() {
                    app.store.online_maps = Some(true);
                }
                // sample pictures spread over a few places, several at each
                let spots = [(44.8378, -0.5792), (48.8566, 2.3522), (43.2965, 5.3698), (41.9028, 12.4964), (51.5072, -0.1276), (40.4168, -3.7038), (52.52, 13.405), (45.764, 4.8357)];
                for id in 0..app.photos.len().min(28) {
                    let (lat, lon) = spots[id % spots.len()];
                    let jitter = (id / spots.len()) as f64 * 0.01;
                    let k = app.photos[id].key.clone();
                    app.store.meta.entry(k).or_default().place = crate::geo::LatLon::new(lat + jitter, lon + jitter).map(|at| crate::geo::Place::set(at, ""));
                }
                if shot_view == "map" {
                    app.open_places();
                } else {
                    let n = app.photos.len();
                    app.start_placing((28..n.min(31)).collect());
                    if shot_view == "pin" {
                        if let Some(at) = crate::geo::LatLon::new(44.8412, -0.5695) {
                            app.set_pin(at, None);
                        }
                    } else {
                        app.type_map_search("bord");
                    }
                }
            }
            if shot_view == "search" {
                // places in Scotland and Spain, a few tags, "sc" typed, one tag already chosen
                let spots = [(56.4907, -4.2026), (55.9533, -3.1883), (57.4778, -4.2247), (43.3614, -5.8593), (43.4623, -3.8100)];
                let tags = ["Sea", "Castle", "Scenery", "Beach", "Hiking", "Old town"];
                for id in 0..app.photos.len().min(25) {
                    let (lat, lon) = spots[id % spots.len()];
                    let k = app.photos[id].key.clone();
                    let m = app.store.meta.entry(k).or_default();
                    m.place = crate::geo::LatLon::new(lat, lon).map(|at| crate::geo::Place::set(at, ""));
                    m.tags = vec![tags[id % tags.len()].into(), tags[(id / 2) % tags.len()].into()];
                }
                app.search_terms = vec![SearchTerm::Tag("Sea".into())];
                app.search = "sc".into();
                app.search_popup = true;
            }
            if shot_view == "geometry" && app.photos.len() > 5 {
                let k = app.photos[5].key.clone();
                let m = app.store.meta.entry(k).or_default();
                m.edits.push(EditOp::Straighten(2.5));
                m.edits.push(EditOp::Perspective { vertical: 0.6, horizontal: 0.0 });
                app.open_editor(5);
                app.edit.advanced = true;
            }
            if shot_view == "maps" {
                app.dialog = Some(Dialog::OnlineMaps);
            }
        } else {
            app.rescan();
            app.lib.tidy();
            if persist {
                app.start_control();
            }
        }
        if let Some(text) = load_problem {
            app.dialog = Some(Dialog::Info { title: "Library".into(), text });
        } else if persist && crate::crash_marker_path().is_some_and(|m| std::fs::remove_file(m).is_ok()) {
            let log = crate::crash_log_path().map(|p| p.display().to_string()).unwrap_or_default();
            app.dialog = Some(Dialog::Info {
                title: "AlbumCraft closed unexpectedly".into(),
                text: format!(
                    "AlbumCraft stopped because of an error the last time it ran. Changes made in the last few seconds before that may be missing.\n\nDetails were written to:\n{log}\n\nPlease include that file when reporting the problem."
                ),
            });
        } else if persist && app.store.online_maps.is_none() {
            // asked once (and again at the next start if something more urgent was shown instead)
            app.dialog = Some(Dialog::OnlineMaps);
        }
        library::trace_since("App::new", t_new, 0);
        app
    }

    // ----------------------------------------------------------------- library

    pub fn rescan(&mut self) {
        self.scanning = true;
        self.lib.scan(self.store.roots.clone());
    }

    pub fn set_library(&mut self, scanned: Vec<ScannedFolder>, samples: bool) {
        let t0 = std::time::Instant::now();
        self.set_library_inner(scanned, samples);
        library::trace_since("set_library (window thread)", t0, 0);
    }

    fn set_library_inner(&mut self, scanned: Vec<ScannedFolder>, samples: bool) {
        self.list_rev += 1;
        self.photos.clear();
        self.folders.clear();
        self.key_index.clear();
        self.thumb_base.clear();
        self.thumbs.clear();
        self.thumb_req.clear();
        self.thumb_seen.clear();
        self.thumb_aspect.clear();
        self.thumb_failed.clear();
        self.big.clear();
        // photo numbers change with a new scan, so per-photo caches start over
        self.real_dims.clear();
        self.dims_pending = false;
        self.exif_cache.clear();
        self.exif_thumbs.clear();
        self.edit.zoom_tex = None;
        self.tray_set = None;
        self.list_pick = None;
        self.big_order.clear();
        self.big_req.clear();
        self.look_previews = None;
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

        let mut folders: Vec<ScannedFolder> = scanned;
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
            let t0 = std::time::Instant::now();
            self.import_picasa_ini();
            library::trace_since(".picasa.ini import (window thread)", t0, 0);
        }
        if let Source::Album(i) = self.source
            && i >= self.store.albums.len() {
                self.source = Source::Library;
            }
        self.geo.start_index(&self.photos, &*self.lib);
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
            let Some(ini) = self.lib.folder_ini(&path) else { continue };
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

    pub fn flag(&self, id: usize) -> Flag {
        self.meta(id).map(|m| m.flag).unwrap_or_default()
    }

    pub fn is_picked(&self, id: usize) -> bool {
        self.flag(id) == Flag::Pick
    }

    pub fn rating(&self, id: usize) -> u8 {
        self.meta(id).map(|m| m.rating).unwrap_or(0)
    }

    /// A search is on (typed text or chosen words).
    pub fn searching(&self) -> bool {
        !self.search.trim().is_empty() || !self.search_terms.is_empty()
    }

    pub fn matches(&self, id: usize) -> bool {
        let (flag, rating) = (self.flag(id), self.rating(id));
        let f = &self.filter;
        if (f.picks && flag != Flag::Pick) || rating < f.min_rating || (f.hide_rejects && flag == Flag::Reject) {
            return false;
        }
        let m = self.meta(id);
        let Some(p) = self.photos.get(id) else { return false };
        let places = self.geo.place_terms.get(&p.key).map(|(_, _, w)| w.as_slice()).unwrap_or(&[]);
        for term in &self.search_terms {
            let ok = match term {
                SearchTerm::Tag(t) => m.is_some_and(|m| m.tags.iter().any(|x| x.eq_ignore_ascii_case(t))),
                SearchTerm::Person(t) => m.is_some_and(|m| m.people.iter().any(|x| x.eq_ignore_ascii_case(t))),
                SearchTerm::Place(t) => places.iter().any(|x| x.eq_ignore_ascii_case(t)),
            };
            if !ok {
                return false;
            }
        }
        let q = self.search.trim().to_lowercase();
        if q.is_empty() {
            return true;
        }
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
        for w in places {
            hay.push(' ');
            hay.push_str(&w.to_lowercase());
        }
        q.split_whitespace().all(|w| hay.contains(w))
    }

    /// Groups shown in the lightbox: (folder index if any, photo ids).
    /// The visible pictures grouped by folder (or one group for an album, person, Picks).
    /// Cached: building it walks the whole library, and it is asked for several times a frame.
    pub fn groups(&self) -> Vec<(Option<usize>, Vec<usize>)> {
        let mut cache = self.list_cache.borrow_mut();
        let key = (self.list_rev, self.geo.place_rev, self.source.clone(), self.search.clone(), self.search_terms.clone(), self.filter.clone());
        if let Some((k, groups)) = cache.as_ref()
            && *k == key {
                return groups.clone();
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
            Source::Picks => {
                let ids = (0..self.photos.len()).filter(|&id| self.is_picked(id) && self.matches(id)).collect();
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
        if self.held.is_empty() && self.selection.is_empty()
            && let Some(set) = &self.tray_set {
                return self.set_photos(set);
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
            TraySet::Picks => (0..self.photos.len()).filter(|&id| self.is_picked(id)).collect(),
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

    /// Copy a picture's edits (all but red-eye fixes, which belong to that picture's eyes).
    pub fn copy_edits(&mut self, id: usize) {
        let ops = imaging::copyable_edits(&self.edits(id));
        if ops.is_empty() {
            self.toast("This picture has no edits to copy.");
            return;
        }
        let n = ops.len();
        self.copied_edits = Some(ops);
        self.toast(format!("Copied {n} edit{}. Select pictures and paste them (Ctrl+Shift+V).", if n == 1 { "" } else { "s" }));
    }

    /// Add the copied edits after each picture's own edits (undo them one by one in the editor,
    /// or with "Undo all edits").
    pub fn paste_edits(&mut self, ids: &[usize]) {
        let Some(ops) = self.copied_edits.clone() else {
            self.toast("Copy the edits of a picture first (Ctrl+Shift+C).");
            return;
        };
        let ids: Vec<usize> = ids.iter().copied().filter(|&id| id < self.photos.len()).collect();
        for &id in &ids {
            self.meta_mut(id).edits.extend(ops.iter().cloned());
            if self.edit.id == id {
                self.edit.redo.clear();
            }
            self.refresh_thumb(id);
        }
        let n = ids.len();
        self.toast(format!("Pasted the edits onto {n} picture{}.", if n == 1 { "" } else { "s" }));
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

    /// Set the flag on all of `ids`; when they all have it already, clear it instead.
    pub fn toggle_flag(&mut self, ids: &[usize], flag: Flag) {
        let ids: Vec<usize> = ids.iter().copied().filter(|&id| id < self.photos.len()).collect();
        let all = !ids.is_empty() && ids.iter().all(|&id| self.flag(id) == flag);
        let new = if all { Flag::None } else { flag };
        for id in ids {
            self.meta_mut(id).flag = new;
        }
    }

    pub fn set_flag(&mut self, ids: &[usize], flag: Flag) {
        let n = self.photos.len();
        for &id in ids.iter().filter(|&&id| id < n) {
            self.meta_mut(id).flag = flag;
        }
    }

    /// Give `ids` this many stars (0 clears the rating).
    pub fn set_rating(&mut self, ids: &[usize], stars: u8) {
        let stars = stars.min(library::MAX_RATING);
        let n = self.photos.len();
        for &id in ids.iter().filter(|&&id| id < n) {
            self.meta_mut(id).rating = stars;
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

    /// A thumbnail about to be drawn: remember it is in use, and ask for it when it is missing.
    pub fn want_thumb(&mut self, id: usize) {
        self.thumb_seen.insert(id, self.ctx.input(|i| i.time));
        if !self.thumbs.contains_key(&id) {
            self.request_thumb(id);
        }
    }

    /// Keep at most MAX_THUMBS thumbnails (about 200 KB each, in memory and on the graphics
    /// card): forget the ones not drawn for the longest time. Their shape is kept for the layout;
    /// they come back from the disk cache when needed.
    fn trim_thumbs(&mut self) {
        const MAX_THUMBS: usize = 1200;
        const KEEP: usize = 1000;
        if self.thumbs.len() <= MAX_THUMBS {
            return;
        }
        let mut by_age: Vec<(f64, usize)> = self.thumbs.keys().map(|id| (self.thumb_seen.get(id).copied().unwrap_or(0.0), *id)).collect();
        by_age.sort_by(|a, b| a.0.total_cmp(&b.0));
        for &(_, id) in by_age.iter().take(self.thumbs.len().saturating_sub(KEEP)) {
            if let Some(tex) = self.thumbs.remove(&id) {
                let [w, h] = tex.size();
                self.thumb_aspect.insert(id, w as f32 / h.max(1) as f32);
            }
            self.thumb_base.remove(&id);
            self.thumb_req.remove(&id);
            self.thumb_seen.remove(&id);
        }
    }

    pub fn request_thumb(&mut self, id: usize) {
        if !self.thumb_req.contains(&id) {
            self.thumb_req.insert(id);
            self.lib.request(Job::Thumb { id, key: self.photos[id].key.clone() }, false);
        }
    }

    pub fn request_big(&mut self, id: usize, urgent: bool) {
        if id < self.photos.len() && !self.big.contains_key(&id) && !self.big_req.contains(&id) {
            self.big_req.insert(id);
            self.lib.request(Job::Big { id, key: self.photos[id].key.clone() }, urgent);
        }
    }

    /// The large, edited picture for the editor or slideshow (None while it loads).
    pub fn big_texture(&mut self, id: usize, extra: &[EditOp]) -> Option<TextureHandle> {
        self.request_big(id, true);
        let mut ops = self.edits(id);
        ops.extend_from_slice(extra);
        let entry = self.big.get_mut(&id)?;
        if let Some((cached, tex)) = &entry.tex
            && *cached == ops {
                return Some(tex.clone());
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
        if let Some((cached, tex)) = &entry.alt_tex
            && cached == ops {
                return Some(tex.clone());
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
            let d = self.lib.source_dims(&self.photos.get(id)?.key)?;
            self.real_dims.insert(id, d);
        }
        self.real_dims.get(&id).copied()
    }

    /// Camera information (EXIF) of a photo, read once and kept.
    pub fn camera_info(&mut self, id: usize) -> Option<Arc<crate::exifinfo::CameraInfo>> {
        if !self.exif_cache.contains_key(&id) {
            let p = self.photos.get(id)?;
            let info = self.lib.camera_info(&p.key).map(Arc::new);
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
        self.lib.request(Job::Zoom { id, key: self.photos[id].key.clone(), ops: ops.to_vec() }, true);
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
        if let Some(pos) = self.edit.list.iter().position(|&x| x == id)
            && let Some(&n) = self.edit.list.get(pos + 1) {
                self.request_big(n, false);
            }
    }

    pub fn start_slideshow(&mut self, list: Vec<usize>, start: usize) {
        if list.is_empty() {
            self.toast("There are no pictures to show.");
            return;
        }
        let now = self.ctx.input(|i| i.time);
        let pos = list.iter().position(|&x| x == start).unwrap_or(0);
        let music = self.start_music();
        self.slide = Some(Slideshow { list, pos, playing: true, started: now, prev: None, paused_at: None, from: self.view, last_move: now, music });
        self.view = View::Slideshow;
        if self.shot.is_none() {
            self.ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(true));
        }
    }

    /// The gecko icon as a texture (for the sidebar), made once.
    pub fn app_icon(&mut self) -> TextureHandle {
        if let Some(t) = &self.icon_tex {
            return t.clone();
        }
        let img = image::load_from_memory(crate::ICON_64).map(|i| i.to_rgba8()).unwrap_or_else(|_| RgbaImage::from_pixel(1, 1, image::Rgba([217, 71, 138, 255])));
        let tex = self.ctx.load_texture("app_icon", to_color_image(&img), TextureOptions::LINEAR);
        self.icon_tex = Some(tex.clone());
        tex
    }

    /// Listen for commands from the MCP server (problems only disable it).
    fn start_control(&mut self) {
        let Some(file) = crate::control::control_path() else { return };
        let (tx, rx) = std::sync::mpsc::channel();
        let ctx = self.ctx.clone();
        match crate::control::start(file, tx, move || ctx.request_repaint()) {
            Ok(server) => self.control = Some((server, rx)),
            Err(e) => eprintln!("control connection not available: {e}"),
        }
    }

    /// Run the commands the MCP server sent; slow ones finish on their own thread.
    fn run_control_requests(&mut self) {
        let requests: Vec<crate::control::Request> = match &self.control {
            Some((_, rx)) => rx.try_iter().collect(),
            None => return,
        };
        for r in requests {
            let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| crate::commands::run(self, &r.command, &r.args)))
                .unwrap_or_else(|_| Err("unexpected error".into()));
            match outcome {
                Ok(crate::commands::Outcome::Now(reply)) => {
                    let _ = r.reply.send(Ok(reply));
                }
                Ok(crate::commands::Outcome::Later(work)) => {
                    let reply = r.reply;
                    std::thread::spawn(move || {
                        let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(work)).unwrap_or_else(|_| Err("unexpected error".into()));
                        let _ = reply.send(res);
                    });
                }
                Err(e) => {
                    let _ = r.reply.send(Err(e));
                }
            }
        }
    }

    /// The slideshow's music, when it is switched on (problems are reported, not fatal).
    pub fn start_music(&mut self) -> Option<crate::slides::Music> {
        let s = &self.store.slideshow;
        if !s.music_on || self.shot.is_some() {
            return None;
        }
        let path = s.music.clone()?;
        match crate::slides::Music::start(&path) {
            Ok(m) => Some(m),
            Err(e) => {
                self.toast(format!("No music: {e}"));
                None
            }
        }
    }

    pub fn stop_slideshow(&mut self) {
        if let Some(s) = self.slide.take() {
            self.view = if s.from == View::Edit { View::Edit } else { View::Library };
            if s.from == View::Edit
                && let Some(&id) = s.list.get(s.pos) {
                    self.set_edit_photo(id);
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

    pub fn add_library_folder(&mut self) {
        if let Some(dir) = rfd::FileDialog::new().set_title("Choose a folder of pictures").pick_folder() {
            let s = dir.to_string_lossy().to_string();
            match self.store.add_root(&s) {
                Ok(()) => {
                    self.dirty = true;
                    self.rescan();
                }
                Err(e) => self.toast(e),
            }
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
        let jobs: Vec<crate::commands::ExportJob> = ids.iter().map(|&id| crate::commands::export_job(self, id)).collect();
        self.toast(format!("Exporting {} picture(s)…", jobs.len()));
        self.lib.export(jobs, dir, max_dim, quality, self.store.export_location);
        self.arm_tray_check();
    }

    /// Open the web-gallery dialog for the tray, titled after what is shown.
    pub fn open_gallery_dialog(&mut self) {
        let title = match &self.source {
            Source::Album(i) => self.store.albums.get(*i).map(|a| a.name.clone()),
            Source::Person(n) => Some(n.clone()),
            _ => self.tray().first().and_then(|&id| self.photos.get(id)).and_then(|p| self.folders.get(p.folder)).map(|f| f.name.clone()),
        }
        .unwrap_or_else(|| "My pictures".into());
        self.dialog = Some(Dialog::Gallery { title, style: crate::gallery::Style::Light, size: 1280, captions: true });
    }

    pub fn make_gallery(&mut self, ids: Vec<usize>, opts: crate::gallery::Options) {
        if ids.is_empty() {
            self.toast("Select some pictures first.");
            return;
        }
        let Some(parent) = rfd::FileDialog::new().set_title("Make the web gallery in…").pick_folder() else { return };
        let items: Vec<crate::gallery::Item> = ids
            .iter()
            .filter_map(|&id| {
                let p = self.photos.get(id)?;
                let caption = self.meta(id).map(|m| m.caption.clone()).unwrap_or_default();
                Some((p.key.clone(), p.name.clone(), self.edits(id), caption))
            })
            .collect();
        self.toast(format!("Making the web gallery of {} picture(s)…", items.len()));
        self.lib.make_gallery(items, parent, opts);
        self.arm_tray_check();
    }

    pub fn make_movie(&mut self, ids: Vec<usize>, size: (u32, u32)) {
        let Some(out) = rfd::FileDialog::new().set_title("Save the movie as…").set_file_name("Slideshow.mp4").add_filter("MP4 video", &["mp4"]).save_file() else {
            return;
        };
        let out = if out.extension().is_some_and(|e| e.eq_ignore_ascii_case("mp4")) { out } else { out.with_extension("mp4") };
        let pictures: Vec<(String, Vec<EditOp>)> = ids.iter().filter_map(|&id| Some((self.photos.get(id)?.key.clone(), self.edits(id)))).collect();
        self.toast("Making the movie…");
        self.lib.make_movie(pictures, out, size, self.store.slideshow.clone());
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
        self.toast("Making the collage…");
        // the picker may return a name without the extension; the picture is always PNG
        let path = if path.extension().is_some_and(|e| e.eq_ignore_ascii_case("png")) { path } else { path.with_extension("png") };
        self.lib.make_collage(jobs, path);
        self.arm_tray_check();
    }

    // ----------------------------------------------------------------- frame

    fn process_results(&mut self) {
        // a frame's share: when a flood of results arrives, the rest waits for the next frame
        let start = std::time::Instant::now();
        while start.elapsed() < std::time::Duration::from_millis(8) {
            let Ok(done) = self.rx.try_recv() else { return };
            match done {
                Done::Thumb { id, key, img } => {
                    if library::tracing() && self.thumbs.len() % 100 == 99 {
                        eprintln!("[trace] {} thumbnails at {:.1} s", self.thumbs.len() + 1, self.ctx.input(|i| i.time));
                    }
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
                    if self.photos.get(id).is_some_and(|p| p.key == key) && id == self.edit.id
                        && let Some(img) = img {
                            let real_w = self.lib.source_dims(&key).map(|d| d.0).unwrap_or(unedited_width);
                            let r = real_w.max(1) as f32 / unedited_width.max(1) as f32;
                            let tex = self.ctx.load_texture(format!("zoom{id}"), to_color_image(&img), TextureOptions::LINEAR);
                            self.edit.zoom_tex = Some((id, ops, tex, r));
                        }
                }
                Done::Scan(found, truncated) => {
                    self.scanning = false;
                    if found.is_empty() {
                        self.set_library(library::sample_folders(), true);
                    } else {
                        self.set_library(found, false);
                    }
                    self.show_imported_folder();
                    if truncated {
                        self.toast(format!(
                            "Only the first {} pictures are shown: remove some folders from the library to see the rest.",
                            fmt_count(library::SCAN_LIMIT)
                        ));
                    }
                }
                Done::Toast(m) => self.toast(m),
                Done::File { key, dim, img } => self.on_import_file(key, dim, img),
                Done::Dims(found) => {
                    self.dims_pending = false;
                    for (id, key, dims) in found {
                        if self.photos.get(id).is_some_and(|p| p.key == key) {
                            // unknown (unreadable header): the usual 3:2, and don't ask again
                            self.real_dims.insert(id, dims.unwrap_or((3, 2)));
                        }
                    }
                }
                Done::ImportList { source, items, truncated } => self.on_import_list(source, items, truncated),
                Done::ImportProgress { done, total } => {
                    if let Some(i) = self.importing.as_mut() {
                        i.done = done;
                        i.total = total;
                    }
                }
                Done::ImportDone { dest, summary, copied, problems } => self.on_import_done(dest, summary, copied, problems),
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
        // out of time with results still waiting: come back next frame
        self.ctx.request_repaint();
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
            && self.big_req.is_empty()
            && !self.geo.tiles_pending();
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
            Dialog::Gallery { .. } => "Web gallery".into(),
            Dialog::FolderNote { .. } => "Folder description".into(),
            Dialog::FolderManager => "Library folders".into(),
            Dialog::Collage => "Make a collage".into(),
            Dialog::Movie { .. } => "Movie".into(),
            Dialog::About => "About AlbumCraft".into(),
            Dialog::OnlineMaps => "Maps".into(),
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
                    if ui
                        .checkbox(&mut self.store.export_location, "Include where the pictures were taken (GPS)")
                        .on_hover_text("Turn off before sharing pictures if you don't want others to see the location.")
                        .changed()
                    {
                        self.dirty = true;
                    }
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
                Dialog::Gallery { title, style, size, captions } => {
                    ui.label(format!("A web page with {} picture(s) and your edits, viewable in any browser or uploadable to a web host.", targets.len()));
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        ui.label("Title:");
                        ui.text_edit_singleline(title);
                    });
                    ui.label("Style:");
                    for st in crate::gallery::Style::ALL {
                        ui.radio_value(style, st, st.label());
                    }
                    ui.label("Picture size:");
                    ui.radio_value(size, 1280, "Medium (1280 pixels, small files)");
                    ui.radio_value(size, 2048, "Large (2048 pixels, for big screens)");
                    ui.checkbox(captions, "Show captions");
                    ui.horizontal(|ui| {
                        if ui.add_enabled(!targets.is_empty(), egui::Button::new("Choose folder and make…")).clicked() {
                            let opts = crate::gallery::Options { title: title.clone(), style: *style, size: *size, captions: *captions };
                            self.make_gallery(targets.clone(), opts);
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
                    ui.label("The library shows the pictures in these folders and their subfolders, where they are: nothing is copied or moved.");
                    ui.weak("To copy pictures from a memory card or a camera, use Import.");
                    ui.add_space(4.0);
                    let mut remove = None;
                    if self.store.roots.is_empty() {
                        ui.weak("(none yet — sample pictures are shown instead)");
                    }
                    for (i, r) in self.store.roots.iter().enumerate() {
                        ui.horizontal(|ui| {
                            ui.label(format!("{} {r}", icons::FOLDER));
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
                        if ui.button(format!("{} Add folder…", icons::ADD)).clicked() {
                            self.add_library_folder();
                        }
                        if ui.button(format!("{} Scan again", icons::REFRESH)).clicked() {
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
                Dialog::Movie { hd } => {
                    let list = if targets.len() > 1 { targets.clone() } else { self.visible_flat() };
                    let ffmpeg = *self.ffmpeg.get_or_insert_with(crate::movie::ffmpeg_available);
                    let st = self.store.slideshow.clone();
                    ui.label(format!(
                        "A video of {} picture(s) as in the slideshow: {} s each{}{}.",
                        list.len(),
                        st.interval.round(),
                        if st.fade { ", crossfades" } else { "" },
                        if st.motion { ", pan and zoom" } else { "" }
                    ));
                    ui.label(egui::RichText::new(format!("About {:.0} s long. Change the timing and effects in the slideshow's control bar.", crate::movie::duration(list.len(), &st))).weak());
                    ui.add_space(4.0);
                    ui.radio_value(hd, false, "720p (1280 × 720)");
                    ui.radio_value(hd, true, "1080p (1920 × 1080)");
                    let s = &mut self.store.slideshow;
                    ui.horizontal(|ui| {
                        if s.music.is_some() {
                            if ui.checkbox(&mut s.music_on, "Music").changed() {
                                self.dirty = true;
                            }
                            let name = s.music.as_deref().and_then(|m| std::path::Path::new(m).file_name()).map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
                            ui.label(egui::RichText::new(name).weak());
                        }
                        if ui.button(if s.music.is_some() { "Other music…" } else { "Add music…" }).clicked()
                            && let Some(p) = rfd::FileDialog::new().add_filter("Music", &["mp3", "ogg", "oga", "flac", "wav", "m4a", "aac", "mp4"]).pick_file()
                        {
                            s.music = Some(p.to_string_lossy().to_string());
                            s.music_on = true;
                            self.dirty = true;
                        }
                    });
                    if !ffmpeg {
                        ui.add_space(6.0);
                        ui.label(egui::RichText::new("Making movies needs the free FFmpeg program, which wasn't found.").color(crate::theme::t().reject));
                        ui.label("Install it from ffmpeg.org (or your system's package manager), then open this window again.");
                        ui.hyperlink_to("ffmpeg.org/download", "https://ffmpeg.org/download.html");
                    }
                    ui.horizontal(|ui| {
                        if ui.add_enabled(ffmpeg && !list.is_empty(), egui::Button::new("Choose file and make…")).clicked() {
                            let size = if *hd { (1920, 1080) } else { (1280, 720) };
                            self.make_movie(list.clone(), size);
                            close = true;
                        }
                        if ui.button(format!("{} Preview as slideshow", icons::PLAY)).clicked() {
                            let first = list.first().copied().unwrap_or(0);
                            self.start_slideshow(list.clone(), first);
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
                    ui.label("A desktop photo organizer: browse your picture folders, make albums, pick, reject and rate pictures, fix and touch up photos, and play slideshows.");
                    ui.label("Edits never change your original files; they are applied when you look at or export a picture.");
                    ui.add_space(6.0);
                    ui.label(egui::RichText::new("Built-in map: Natural Earth (public domain). City list: GeoNames (CC BY 4.0). Online maps: © OpenStreetMap contributors (ODbL).").size(11.5).weak());
                    if ui.button("OK").clicked() {
                        close = true;
                    }
                }
                Dialog::OnlineMaps => {
                    ui.label(egui::RichText::new("Show detailed maps from the internet?").strong().size(15.0));
                    ui.add_space(4.0);
                    ui.label("AlbumCraft can show where your pictures were taken. With online maps, it downloads street maps from OpenStreetMap and can look up addresses when you set a location.");
                    ui.add_space(4.0);
                    ui.label("Your pictures and their locations are never sent. The map servers see your internet address, the areas you look at and the places you search for.");
                    ui.add_space(4.0);
                    ui.label("Without online maps, AlbumCraft uses its built-in world map and list of cities, and makes no internet connections.");
                    ui.add_space(4.0);
                    ui.label(egui::RichText::new("You can change this later on the map, or in the main menu: Tools › Online maps.").size(12.0).weak());
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        if ui.add(egui::Button::new(egui::RichText::new("Use online maps").color(Color32::WHITE).strong()).fill(t().accent)).clicked() {
                            self.store.online_maps = Some(true);
                            self.dirty = true;
                            close = true;
                        }
                        if ui.button("Built-in map only").clicked() {
                            self.store.online_maps = Some(false);
                            self.dirty = true;
                            close = true;
                        }
                    });
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
        if i > 0 && (s.len() - i).is_multiple_of(3) {
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

impl App {
    fn save_finished(&mut self, result: Result<(), String>) {
        match result {
            Ok(()) => self.save_error = None,
            Err(e) => {
                // stay dirty: it is tried again in a few seconds
                self.dirty = true;
                if self.save_error.as_ref() != Some(&e) {
                    self.toast(format!("Your library could not be saved: {e}"));
                    self.save_error = Some(e);
                }
            }
        }
    }
}

impl eframe::App for App {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        crate::theme::apply(ctx, crate::theme::Mode::from_name(&self.store.theme), false);
        let t0 = std::time::Instant::now();
        self.process_results();
        library::trace_since("frame: results", t0, 30);
        self.trim_thumbs();
        let t0 = std::time::Instant::now();
        self.geo_tick();
        library::trace_since("frame: geo", t0, 30);
        self.run_control_requests();
        self.handle_screenshot(ctx);
        let now = ctx.input(|i| i.time);
        let closing = ctx.input(|i| i.viewport().close_requested());
        // every change to the library's notes sets `dirty`: rebuild the cached lists after it
        if self.dirty {
            self.list_rev += 1;
        }
        // collect a finished background save (when closing, wait for it)
        if let Some(result) = self.lib.take_saved(closing) {
            self.save_finished(result);
        }
        // one save at a time: they share the temporary file
        if self.persist && self.dirty && !self.lib.saving() && (closing || now - self.last_save > 3.0) {
            self.last_save = now;
            // changes made from here on set `dirty` again and are saved next time
            self.dirty = false;
            // the window closes after this frame: write now
            self.lib.save(&mut self.store, closing);
            if let Some(result) = self.lib.take_saved(closing) {
                self.save_finished(result);
            }
        }
        if self.lib.saving() {
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
        } else if self.dirty && self.persist {
            ctx.request_repaint_after(std::time::Duration::from_secs(3));
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        let t0 = std::time::Instant::now();
        match self.view {
            View::Library => self.library_ui(ui),
            View::Edit => self.edit_ui(ui),
            View::Slideshow => self.slideshow_ui(ui),
            View::Print => self.print_ui(ui),
            View::Map => self.map_ui(ui),
            View::Import => self.import_ui(ui),
        }
        library::trace_since(&format!("frame: {:?} view", self.view), t0, 30);
        // Ctrl+P opens printing from the library or the editor
        if crate::print::available()
            && matches!(self.view, View::Library | View::Edit)
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

impl crate::commands::Host for App {
    fn store(&self) -> &Store {
        &self.store
    }
    fn store_mut(&mut self) -> &mut Store {
        self.dirty = true;
        &mut self.store
    }
    fn photos(&self) -> &[Photo] {
        &self.photos
    }
    fn folders(&self) -> &[Folder] {
        &self.folders
    }
    fn find(&self, key: &str) -> Option<usize> {
        self.key_index.get(key).copied()
    }
    fn changed(&mut self, id: usize) {
        self.dirty = true;
        self.refresh_thumb(id);
    }
    fn rescan(&mut self) {
        App::rescan(self);
    }
    fn show(&mut self, id: usize) -> Result<(), String> {
        if id >= self.photos.len() {
            return Err("no such picture".into());
        }
        self.open_editor(id);
        Ok(())
    }
    fn file_gps(&mut self) -> &std::collections::HashMap<String, Option<crate::geo::LatLon>> {
        &self.geo.file_gps
    }
}

#[cfg(test)]
mod tray_tests {
    use super::*;

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
