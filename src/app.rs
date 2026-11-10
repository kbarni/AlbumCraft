//! Application state, shared helpers, dialogs and the frame loop.

use crate::imaging::{self, EditOp};
use crate::backend::{Library, LocalLibrary};
use crate::library::{self, Album, Done, Flag, Folder, IniFolder, Job, Loaded, Meta, Photo, ScannedFolder, Store};
use eframe::egui::{self, ColorImage, TextureHandle, TextureOptions};
use image::RgbaImage;
use std::collections::{BTreeSet, HashMap, HashSet, VecDeque};
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Mutex};

pub use crate::theme::t;

/// One group of the library list: its folder (None for an album, a person, Picks) and pictures.
pub type Group = (Option<usize>, Vec<usize>);

/// See `App::sidebar_counts`.
pub struct SidebarCounts {
    pub picks: usize,
    /// Pictures of each album that are in the library, by album index.
    pub albums: Vec<usize>,
    pub people: Vec<(String, usize)>,
    /// Folder indexes by year, newest first.
    pub by_year: Vec<(i32, Vec<usize>)>,
}

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

/// A message shown over the bottom of the window.
pub struct Notice {
    pub text: String,
    /// Something went wrong.
    pub error: bool,
    /// When it appeared (window time).
    pub born: f64,
    /// Seconds it stays; None: until the user closes it.
    pub life: Option<f64>,
    pub action: Option<NoticeAction>,
}

/// The button a notice offers.
pub enum NoticeAction {
    /// "Show in folder": a file or folder that was made.
    Show(PathBuf),
    Undo(Undo),
}

/// What an "Undo" button on a notice puts back.
pub enum Undo {
    /// A deleted album, at its place in the list.
    Album { at: usize, album: Album },
    /// A folder removed from the library.
    Root { at: usize, path: String },
    /// Edits cleared from pictures (by key).
    Edits(Vec<(String, Vec<EditOp>)>),
}

/// Background work the user watches: see `App::start_task`.
pub struct RunningTask {
    pub id: u64,
    pub label: String,
    pub done: usize,
    pub total: usize,
    pub stop: Arc<std::sync::atomic::AtomicBool>,
}

/// How the message about a failed save starts.
const SAVE_FAILED: &str = "Your library could not be saved:";

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
    /// Collages and contact sheets.
    Collage,
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
    /// The picture's own shape.
    Original,
    Square,
    FourThree,
    ThreeTwo,
    SixteenNine,
}

impl CropAspect {
    pub const ALL: [CropAspect; 6] =
        [CropAspect::Manual, CropAspect::Original, CropAspect::Square, CropAspect::FourThree, CropAspect::ThreeTwo, CropAspect::SixteenNine];

    /// Width / height of the long side over the short side; `picture` is the picture's own.
    pub fn ratio(&self, picture: f32) -> Option<f32> {
        match self {
            CropAspect::Manual => None,
            CropAspect::Original => Some(if picture >= 1.0 { picture } else { 1.0 / picture.max(0.01) }),
            CropAspect::Square => Some(1.0),
            CropAspect::FourThree => Some(4.0 / 3.0),
            CropAspect::ThreeTwo => Some(1.5),
            CropAspect::SixteenNine => Some(16.0 / 9.0),
        }
    }
    pub fn label(&self) -> &'static str {
        match self {
            CropAspect::Manual => "Free",
            CropAspect::Original => "Original shape",
            CropAspect::Square => "Square",
            CropAspect::FourThree => "4 × 3",
            CropAspect::ThreeTwo => "3 × 2 (10 × 15 print)",
            CropAspect::SixteenNine => "16 × 9 (wide screen)",
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
    /// Make a movie of the tray (`hd` = 1920×1080, else 1280×720).
    Movie { hd: bool },
    About,
    /// "Would you like to clear your old held items from the tray?"
    ClearTray { old: Vec<usize> },
    /// Print review: (picture, name, pixels per inch, grade 0 bad / 1 good / 2 best).
    PrintReview { grades: Vec<(usize, String, f32, u8)>, before_print: bool },
    /// Asked once: may maps and place search use the internet?
    OnlineMaps,
}

/// Where the editor goes next.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum PendingLeave {
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
    /// The crop box is upright (taller than wide) for shapes other than square.
    pub crop_portrait: bool,
    /// Draw the largest box of the chosen shape, centred, at the next frame.
    pub crop_autofit: bool,
    /// Show the alignment grid on the picture until this time (geometry sliders in use).
    pub grid_until: f64,
    /// The picture's edit lists as they were before each change made here, oldest first (Undo).
    pub undo: Vec<Vec<EditOp>>,
    /// The lists taken back by Undo, to put back with Redo.
    pub redo: Vec<Vec<EditOp>>,
    /// What the user is in the middle of (dragging a slider…): further changes of the same kind
    /// belong to the same Undo step. Cleared when the mouse button is let go.
    pub gesture: Option<&'static str>,
    /// Show the picture without its edits (the Original button or the O key, while held).
    pub show_original: bool,
    /// The crop box being dragged: what was taken hold of, the box and the pointer then.
    pub crop_grip: Option<(crate::steps::Grip, [f32; 4], egui::Pos2)>,
    pub tag_buf: String,
    pub person_buf: String,
    /// "A", "A|B" or "A|A" view: the three buttons at the top of the editor.
    pub compare: CompareMode,
    /// One picture above the other instead of side by side.
    pub vertical: bool,
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
    /// The picture after the first steps of its edit list (those steps, the result): while one
    /// step is being changed, only it and the ones after it are applied again.
    pub stage: Option<(Vec<EditOp>, RgbaImage)>,
    pub tex: Option<(Vec<EditOp>, TextureHandle)>,
    /// The picture without any edits (the "before" side).
    pub alt_tex: Option<(Vec<EditOp>, TextureHandle)>,
    /// Histogram of the edited picture shown in the editor.
    pub hist: Option<Histogram>,
}

pub struct App {
    pub store: Store,
    /// Set by every change to the library's notes or settings; taken at the start of the next
    /// frame, which then rebuilds the cached lists once and marks the library `unsaved`.
    pub dirty: bool,
    /// Changes not written to disk yet.
    pub unsaved: bool,
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
    list_cache: std::cell::RefCell<Option<(ListKey, Rc<Vec<Group>>)>>,
    /// `search_hay()` for (list_rev, place_rev).
    #[allow(clippy::type_complexity)]
    hay_cache: std::cell::RefCell<Option<((u64, u64), Rc<Vec<String>>)>>,
    sidebar_cache: Option<(u64, Rc<SidebarCounts>)>,
    /// Bumped when a picture's shape (width / height) becomes known or changes: the grid's
    /// row layout is made again.
    pub shape_rev: u64,
    /// The grid's rows, kept between frames (shell.rs).
    pub grid_cache: Option<crate::shell::GridCache>,

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
    /// Thumbnails to make again because their picture's edits changed (`refresh_thumb`).
    pub stale_thumbs: HashSet<usize>,
    /// How many of them may still be made in this frame.
    thumb_budget: u32,
    pub big: HashMap<usize, BigEntry>,
    pub big_order: VecDeque<usize>,
    pub big_req: HashSet<usize>,
    /// Pictures whose preview could not be read: not asked for again (until the next scan).
    pub big_failed: HashSet<usize>,
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
    pub collage: Option<Box<crate::collage_view::CollageState>>,
    pub dialog: Option<Dialog>,
    /// Messages shown over the bottom of the window: short notes, results of background work,
    /// errors that stay until closed, and offers to undo (see dialogs.rs).
    pub notices: Vec<Notice>,
    /// Background work shown in the status bar with its progress and a Cancel button.
    pub tasks: Vec<RunningTask>,
    next_task: u64,
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
            unsaved: false,
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
            hay_cache: Default::default(),
            sidebar_cache: None,
            shape_rev: 0,
            grid_cache: None,
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
            stale_thumbs: HashSet::new(),
            thumb_budget: 0,
            big: HashMap::new(),
            big_order: VecDeque::new(),
            big_req: HashSet::new(),
            big_failed: HashSet::new(),
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
                crop_portrait: false,
                crop_autofit: false,
                grid_until: 0.0,
                angle: 0.0,
                drag_start: None,
                sel: None,
                undo: vec![],
                redo: vec![],
                gesture: None,
                show_original: false,
                crop_grip: None,
                compare: CompareMode::Single,
                vertical: false,
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
            collage: None,
            dialog: None,
            notices: vec![],
            tasks: vec![],
            next_task: 1,
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
            app.demo_setup();
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
        self.lib.scan(self.store.roots.clone(), self.store.imported_ini.clone());
    }

    /// Show these folders as the library. What the window holds for pictures that are still
    /// there (selection, the open picture, thumbnails…) is kept: see `keep_across_scan`.
    pub fn set_library(&mut self, scanned: Vec<ScannedFolder>, samples: bool) {
        self.set_library_with(scanned, samples, vec![]);
    }

    /// `set_library`, with the `.picasa.ini` notes the scan read in folders not imported yet.
    pub fn set_library_with(&mut self, scanned: Vec<ScannedFolder>, samples: bool, inis: Vec<(String, IniFolder)>) {
        let t0 = std::time::Instant::now();
        self.set_library_inner(scanned, samples, inis);
        library::trace_since("set_library (window thread)", t0, 0);
    }

    fn set_library_inner(&mut self, scanned: Vec<ScannedFolder>, samples: bool, inis: Vec<(String, IniFolder)>) {
        self.list_rev += 1;
        let old_photos = std::mem::take(&mut self.photos);
        let old_folders = std::mem::take(&mut self.folders);
        self.key_index.clear();
        self.using_samples = samples;

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
        self.keep_across_scan(&old_photos, &old_folders);
        if !samples {
            self.import_picasa_ini(inis);
        }
        if let Source::Album(i) = self.source
            && i >= self.store.albums.len() {
                self.source = Source::Library;
            }
        self.geo.start_index(&self.photos, &*self.lib);
    }

    /// Picture numbers are positions in `photos` and change with every scan. Everything that
    /// remembers them is moved to the new numbers here, by file path: the selection, the open
    /// picture, the tray, print and collage pictures, the slideshow. Thumbnails, previews and
    /// what was read from a file are kept only when the file is unchanged (same date and size).
    /// What belonged to pictures that are gone is dropped; a view whose pictures are all gone
    /// closes.
    fn keep_across_scan(&mut self, old_photos: &[Photo], old_folders: &[Folder]) {
        let same: Vec<Option<usize>> = old_photos.iter().map(|p| self.key_index.get(&p.key).copied()).collect();
        let fresh: Vec<Option<usize>> = old_photos
            .iter()
            .zip(&same)
            .map(|(p, n)| n.filter(|&n| self.photos.get(n).is_some_and(|q| q.modified == p.modified && q.bytes == p.bytes)))
            .collect();
        // the picture's new number / the same, only when its file is unchanged
        let to = |id: usize| same.get(id).copied().flatten();
        let keep = |id: usize| fresh.get(id).copied().flatten();
        fn map_of<V>(m: &mut HashMap<usize, V>, f: impl Fn(usize) -> Option<usize>) {
            *m = std::mem::take(m).into_iter().filter_map(|(id, v)| Some((f(id)?, v))).collect();
        }
        fn set_of(s: &mut HashSet<usize>, f: impl Fn(usize) -> Option<usize>) {
            *s = s.iter().filter_map(|&id| f(id)).collect();
        }
        fn list_of(v: &mut Vec<usize>, f: impl Fn(usize) -> Option<usize>) {
            *v = v.iter().filter_map(|&id| f(id)).collect();
        }

        // read from the files
        map_of(&mut self.thumb_base, keep);
        map_of(&mut self.thumbs, keep);
        map_of(&mut self.thumb_seen, keep);
        map_of(&mut self.thumb_aspect, keep);
        map_of(&mut self.real_dims, keep);
        map_of(&mut self.exif_cache, keep);
        map_of(&mut self.exif_thumbs, keep);
        map_of(&mut self.big, keep);
        self.big_order = self.big_order.iter().filter_map(|&id| keep(id)).filter(|id| self.big.contains_key(id)).collect();
        // answers on their way are matched by file path when they arrive (`process_results`)
        set_of(&mut self.thumb_req, keep);
        set_of(&mut self.stale_thumbs, keep);
        set_of(&mut self.big_req, keep);
        // a file that could not be read may be readable now
        self.thumb_failed.clear();
        self.big_failed.clear();
        self.look_previews = self.look_previews.take().and_then(|(id, ops, texs)| Some((keep(id)?, ops, texs)));
        self.edit.zoom_tex = self.edit.zoom_tex.take().and_then(|(id, ops, tex, r)| Some((keep(id)?, ops, tex, r)));
        self.edit.zoom_req = self.edit.zoom_req.take().and_then(|(id, ops)| Some((to(id)?, ops)));

        // chosen by the user
        self.selection = self.selection.iter().filter_map(|&id| to(id)).collect();
        self.anchor = self.anchor.and_then(to);
        list_of(&mut self.held, to);
        self.scroll_to_photo = self.scroll_to_photo.and_then(to);
        let folder_to = |fi: usize| {
            let path = &old_folders.get(fi)?.path;
            self.folders.iter().position(|f| &f.path == path)
        };
        let set_to = |set: Option<TraySet>| match set {
            Some(TraySet::Folder(fi)) => folder_to(fi).map(TraySet::Folder),
            other => other,
        };
        self.tray_set = set_to(self.tray_set.take());
        self.list_pick = set_to(self.list_pick.take());
        self.scroll_to_folder = self.scroll_to_folder.and_then(folder_to);
        // these questions list pictures by number: asked again when they still apply
        self.tray_check = None;
        if matches!(self.dialog, Some(Dialog::ClearTray { .. } | Dialog::PrintReview { .. })) {
            self.dialog = None;
        }

        // the editor
        list_of(&mut self.edit.list, to);
        self.edit.other = self.edit.other.and_then(to);
        self.edit.strip_shown = None;
        match to(self.edit.id) {
            Some(id) => {
                self.edit.id = id;
            }
            None => {
                // the open picture is gone
                self.edit.id = 0;
                self.edit.undo.clear();
                self.edit.redo.clear();
                self.edit.focus_second = false;
                self.edit.tool = Tool::None;
                self.edit.sel = None;
                if self.view == View::Edit {
                    self.view = View::Library;
                }
            }
        }

        // print, collage, slideshow
        if let Some(ps) = self.print.as_mut() {
            let before = ps.ids.len();
            list_of(&mut ps.ids, to);
            if ps.ids.len() != before {
                ps.page = 0;
            }
            if ps.ids.is_empty() {
                self.print = None;
            }
        }
        if self.collage.as_mut().is_some_and(|c| !c.keep_across_scan(to)) {
            self.collage = None;
        }
        if let Some(s) = self.slide.as_mut() {
            let current = s.list.get(s.pos).copied().and_then(to);
            list_of(&mut s.list, to);
            s.pos = current.and_then(|c| s.list.iter().position(|&id| id == c)).unwrap_or(0);
            s.prev = None;
            if s.list.is_empty() {
                self.slide = None;
                self.ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(false));
            }
        }
        let gone = match self.view {
            View::Print => self.print.is_none(),
            View::Collage => self.collage.is_none(),
            View::Slideshow => self.slide.is_none(),
            _ => false,
        };
        if gone {
            self.view = View::Library;
        }
    }

    /// Bring in stars, captions, tags, rotations, crops, filters, folder descriptions and albums
    /// saved in each folder's .picasa.ini. Done once per folder and
    /// read-only: the file itself is never changed. `inis` is what the scan read (off the
    /// window's thread), by folder path, for folders not brought in before.
    fn import_picasa_ini(&mut self, inis: Vec<(String, IniFolder)>) {
        let mut imported = 0;
        let done: HashSet<String> = self.store.imported_ini.iter().cloned().collect();
        let folder_at: HashMap<&str, usize> = self.folders.iter().enumerate().map(|(fi, f)| (f.path.as_str(), fi)).collect();
        let found: Vec<(usize, String, IniFolder)> =
            inis.into_iter().filter(|(path, _)| !done.contains(path)).filter_map(|(path, ini)| Some((*folder_at.get(path.as_str())?, path, ini))).collect();
        for (fi, path, ini) in found {
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

    /// Whether a picture passes the filter and the search. `words` is the typed text in lower
    /// case, split; `hay` the text each picture is searched in (`search_hay`).
    fn matches(&self, id: usize, words: &[String], hay: &[String]) -> bool {
        let (flag, rating) = (self.flag(id), self.rating(id));
        let f = &self.filter;
        if (f.picks && flag != Flag::Pick) || rating < f.min_rating || (f.hide_rejects && flag == Flag::Reject) {
            return false;
        }
        if !self.search_terms.is_empty() {
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
        }
        words.is_empty() || hay.get(id).is_some_and(|h| words.iter().all(|w| h.contains(w.as_str())))
    }

    /// What typed text is looked for in, per picture, in lower case: file and folder name,
    /// caption, tags, people and places. Made once and kept until the notes or places change,
    /// so that typing in the search box costs one text search per picture.
    fn search_hay(&self) -> Rc<Vec<String>> {
        let key = (self.list_rev, self.geo.place_rev);
        let mut cache = self.hay_cache.borrow_mut();
        if let Some((k, hay)) = cache.as_ref()
            && *k == key
        {
            return hay.clone();
        }
        let hay: Vec<String> = self
            .photos
            .iter()
            .map(|p| {
                let folder = self.folders.get(p.folder).map(|f| f.name.as_str()).unwrap_or("");
                let mut text = format!("{} {}", p.name, folder);
                if let Some(m) = self.store.meta.get(&p.key) {
                    for t in std::iter::once(&m.caption).chain(m.tags.iter()).chain(m.people.iter()) {
                        text.push(' ');
                        text.push_str(t);
                    }
                }
                if let Some((_, _, words)) = self.geo.place_terms.get(&p.key) {
                    for w in words {
                        text.push(' ');
                        text.push_str(w);
                    }
                }
                text.to_lowercase()
            })
            .collect();
        let hay = Rc::new(hay);
        *cache = Some((key, hay.clone()));
        hay
    }

    /// The visible pictures grouped by folder (or one group for an album, person, Picks):
    /// (folder index if any, photo ids). Cached and shared: building it walks the whole library.
    pub fn groups(&self) -> Rc<Vec<Group>> {
        let mut cache = self.list_cache.borrow_mut();
        let key = (self.list_rev, self.geo.place_rev, self.source.clone(), self.search.clone(), self.search_terms.clone(), self.filter.clone());
        if let Some((k, groups)) = cache.as_ref()
            && *k == key {
                return groups.clone();
            }
        let groups = Rc::new(self.build_groups());
        *cache = Some((key, groups.clone()));
        groups
    }

    fn build_groups(&self) -> Vec<Group> {
        let words: Vec<String> = self.search.to_lowercase().split_whitespace().map(str::to_string).collect();
        let hay = if words.is_empty() { Rc::new(vec![]) } else { self.search_hay() };
        let shown = |id: &usize| self.matches(*id, &words, &hay);
        match &self.source {
            Source::Library => self
                .folders
                .iter()
                .enumerate()
                .map(|(fi, f)| (Some(fi), f.photos.iter().copied().filter(shown).collect::<Vec<_>>()))
                .filter(|(_, v)| !v.is_empty())
                .collect(),
            Source::Album(i) => {
                let ids = self
                    .store
                    .albums
                    .get(*i)
                    .map(|a| a.keys.iter().filter_map(|k| self.key_index.get(k).copied()).filter(shown).collect())
                    .unwrap_or_default();
                vec![(None, ids)]
            }
            Source::Person(name) => {
                let ids = (0..self.photos.len())
                    .filter(|&id| self.meta(id).is_some_and(|m| m.people.iter().any(|p| p == name)))
                    .filter(shown)
                    .collect();
                vec![(None, ids)]
            }
            Source::Picks => {
                let ids = (0..self.photos.len()).filter(|&id| self.is_picked(id)).filter(shown).collect();
                vec![(None, ids)]
            }
        }
    }

    pub fn visible_flat(&self) -> Vec<usize> {
        self.groups().iter().flat_map(|(_, v)| v.iter().copied()).collect()
    }

    /// How many pictures the list shows.
    pub fn visible_count(&self) -> usize {
        self.groups().iter().map(|(_, v)| v.len()).sum()
    }

    /// What the sidebar counts (picks, pictures per album, people, folders by year), made once
    /// per change of the library or its notes instead of on every frame.
    pub fn sidebar_counts(&mut self) -> Rc<SidebarCounts> {
        if let Some((rev, counts)) = &self.sidebar_cache
            && *rev == self.list_rev
        {
            return counts.clone();
        }
        let mut by_year: Vec<(i32, Vec<usize>)> = vec![];
        for (fi, f) in self.folders.iter().enumerate() {
            let y = library::local_year(f.date);
            match by_year.last_mut() {
                Some((yy, v)) if *yy == y => v.push(fi),
                _ => by_year.push((y, vec![fi])),
            }
        }
        let counts = Rc::new(SidebarCounts {
            picks: (0..self.photos.len()).filter(|&i| self.is_picked(i)).count(),
            albums: self.store.albums.iter().map(|a| a.keys.iter().filter(|k| self.key_index.contains_key(*k)).count()).collect(),
            people: self.people(),
            by_year,
        });
        self.sidebar_cache = Some((self.list_rev, counts.clone()));
        counts
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
        if v.is_empty() {
            v.extend(self.selection.iter().copied());
        } else {
            // a set, so that a large selection is not searched once per picture
            let held: HashSet<usize> = v.iter().copied().collect();
            v.extend(self.selection.iter().copied().filter(|s| !held.contains(s)));
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
        // several pictures: only their dates and sizes count (no copies of their names)
        let of = |id: &usize| self.photos.get(*id).map(|p| (p.modified, p.bytes));
        several_text(tray.len(), tray.iter().filter_map(of))
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

    /// Change a picture's edit steps. Every change made in the window goes through here: for
    /// the picture open in the editor the list as it was is kept for Undo, the thumbnail is
    /// made again, and the library is marked as changed.
    ///
    /// `gesture` names what the user is in the middle of ("adjust" while a slider of the
    /// adjustment is dragged…): changes with the same name made until the mouse button is let
    /// go are one Undo step. "" is a change on its own (a button).
    pub fn change_edits(&mut self, id: usize, gesture: &'static str, change: impl FnOnce(&mut Vec<EditOp>)) {
        if id >= self.photos.len() {
            return;
        }
        let before = self.edits(id);
        let mut after = before.clone();
        change(&mut after);
        if after == before {
            return;
        }
        self.meta_mut(id).edits = after;
        if self.edit.id == id {
            let same = !gesture.is_empty() && self.edit.gesture == Some(gesture);
            if !same {
                self.edit.undo.push(before);
                if self.edit.undo.len() > 200 {
                    self.edit.undo.remove(0);
                }
            }
            self.edit.gesture = (!gesture.is_empty()).then_some(gesture);
            self.edit.redo.clear();
        }
        self.refresh_thumb(id);
    }

    pub fn push_edit(&mut self, id: usize, op: EditOp) {
        self.change_edits(id, "", |e| e.push(op));
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
            self.change_edits(id, "", |e| e.extend(ops.iter().cloned()));
        }
        let n = ids.len();
        self.toast(format!("Pasted the edits onto {n} picture{}.", if n == 1 { "" } else { "s" }));
    }

    pub fn rotate(&mut self, ids: &[usize], dir: i32) {
        for &id in ids {
            self.change_edits(id, "", |e| {
                if let Some(EditOp::Rotate(n)) = e.last_mut() {
                    *n += dir;
                    if n.rem_euclid(4) == 0 {
                        e.pop();
                    }
                } else {
                    e.push(EditOp::Rotate(dir));
                }
            });
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

    /// Take back the last change made to the open picture's edits in this editing session.
    pub fn undo(&mut self) {
        let id = self.edit.id;
        if id >= self.photos.len() {
            return;
        }
        if let Some(list) = self.edit.undo.pop() {
            let now = std::mem::replace(&mut self.meta_mut(id).edits, list);
            self.edit.redo.push(now);
            self.edit.gesture = None;
            self.refresh_thumb(id);
        }
    }

    pub fn redo(&mut self) {
        let id = self.edit.id;
        if id >= self.photos.len() {
            return;
        }
        if let Some(list) = self.edit.redo.pop() {
            let now = std::mem::replace(&mut self.meta_mut(id).edits, list);
            self.edit.undo.push(now);
            self.edit.gesture = None;
            self.refresh_thumb(id);
        }
    }

    /// The picture's edits changed: its thumbnail has to be made again. Without edits that is
    /// done at once; with edits, when the thumbnail is next drawn (`want_thumb`), a few per
    /// frame, so that pasting edits onto many pictures does not hold up the window.
    pub fn refresh_thumb(&mut self, id: usize) {
        if !self.thumb_base.contains_key(&id) {
            return;
        }
        if self.meta(id).is_none_or(|m| m.edits.is_empty()) {
            self.stale_thumbs.remove(&id);
            self.render_thumb(id);
        } else {
            self.stale_thumbs.insert(id);
            self.ctx.request_repaint();
        }
    }

    fn render_thumb(&mut self, id: usize) {
        if let Some(base) = self.thumb_base.get(&id) {
            let edits = self.edits(id);
            let img = if edits.is_empty() { base.clone() } else { imaging::apply_all(base, &edits) };
            // a new shape (first seen, turned, cropped): the grid's rows change
            let shape = img.width() as f32 / img.height().max(1) as f32;
            if (self.aspect_of(id) / shape - 1.0).abs() > 0.005 {
                self.shape_rev += 1;
            }
            let tex = self.ctx.load_texture(format!("thumb{id}"), to_color_image(&img), TextureOptions::LINEAR);
            self.thumbs.insert(id, tex);
        }
    }

    /// A thumbnail about to be drawn: remember it is in use, and ask for it when it is missing.
    pub fn want_thumb(&mut self, id: usize) {
        self.thumb_seen.insert(id, self.ctx.input(|i| i.time));
        if self.stale_thumbs.contains(&id) {
            // not while a slider of this picture is being dragged: once it is let go
            let dragging = self.view == View::Edit && id == self.edit.id && self.edit.gesture.is_some();
            if !dragging && self.thumb_budget > 0 {
                self.thumb_budget -= 1;
                self.stale_thumbs.remove(&id);
                self.render_thumb(id);
            } else {
                self.ctx.request_repaint();
            }
        }
        if !self.thumbs.contains_key(&id) && !self.stale_thumbs.contains(&id) {
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
            self.stale_thumbs.remove(&id);
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
        if id < self.photos.len() && !self.big.contains_key(&id) && !self.big_req.contains(&id) && !self.big_failed.contains(&id) {
            self.big_req.insert(id);
            self.lib.request(Job::Big { id, key: self.photos[id].key.clone() }, urgent);
        }
    }

    /// The large, edited picture for the editor or slideshow (None while it loads).
    pub fn big_texture(&mut self, id: usize, extra: &[EditOp]) -> Option<TextureHandle> {
        let mut ops = self.edits(id);
        ops.extend_from_slice(extra);
        self.big_ops_texture(id, &ops)
    }

    /// The large picture with these edit steps. The picture as it is before the first step that
    /// differs from the last render is kept, so that changing one step (a slider being dragged)
    /// applies only that step and the ones after it, not the whole list.
    pub fn big_ops_texture(&mut self, id: usize, ops: &[EditOp]) -> Option<TextureHandle> {
        self.request_big(id, true);
        let entry = self.big.get_mut(&id)?;
        let mut same = 0;
        if let Some((cached, tex)) = &entry.tex {
            if cached.as_slice() == ops {
                return Some(tex.clone());
            }
            same = cached.iter().zip(ops).take_while(|(a, b)| a == b).count();
        }
        // never the whole list: the last step is always applied to the kept picture
        let same = same.min(ops.len().saturating_sub(1));
        let img = if same == 0 {
            entry.stage = None;
            imaging::apply_all(&entry.base, ops)
        } else {
            if !entry.stage.as_ref().is_some_and(|(steps, _)| steps.as_slice() == &ops[..same]) {
                entry.stage = Some((ops[..same].to_vec(), imaging::apply_all(&entry.base, &ops[..same])));
            }
            match &entry.stage {
                Some((_, stage)) => imaging::apply_all(stage, &ops[same..]),
                None => imaging::apply_all(&entry.base, ops),
            }
        };
        entry.hist = Some(histogram(&img));
        let tex = self.ctx.load_texture(format!("big{id}"), to_color_image(&img), TextureOptions::LINEAR);
        entry.tex = Some((ops.to_vec(), tex.clone()));
        Some(tex)
    }

    /// The large picture with a given edit list, next to the edited one (an empty list = the
    /// picture as it is on disk: the original shown while its button is held).
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

    // ------------------------------------------------------- two pictures side by side

    /// One picture, or two side by side.
    pub fn set_compare(&mut self, mode: CompareMode) {
        if self.edit.compare == mode {
            return;
        }
        self.edit.compare = mode;
        self.edit.focus_second = false;
        if mode != CompareMode::Single {
            self.edit.tool = Tool::None;
            self.edit.sel = None;
        }
    }

    /// With two pictures shown: edit the other one.
    pub fn swap_2up_focus(&mut self) {
        if self.edit.compare == CompareMode::TwoPhotos
            && let Some(o) = self.compare_partner()
        {
            let cur = self.edit.id;
            self.set_edit_photo(o);
            self.edit.other = Some(cur);
            self.edit.focus_second = !self.edit.focus_second;
        }
    }

    /// Go to another picture or back to the library.
    pub fn edit_go(&mut self, then: PendingLeave) {
        self.edit.focus_second = false;
        match then {
            PendingLeave::Library => {
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
        self.edit.focus_second = false;
        self.edit.other = None;
        self.view = View::Edit;
    }

    pub fn set_edit_photo(&mut self, id: usize) {
        if self.edit.id != id {
            // Undo is per picture, for as long as it is open
            self.edit.undo.clear();
            self.edit.redo.clear();
        }
        self.edit.gesture = None;
        self.edit.crop_grip = None;
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

    /// A short note that goes away by itself.
    pub fn toast(&mut self, text: impl Into<String>) {
        self.notice(text.into(), false, Some(3.5), None);
    }

    /// Something went wrong: the message stays until the user closes it.
    pub fn error(&mut self, text: impl Into<String>) {
        self.notice(text.into(), true, None, None);
    }

    /// What was just done, with a button to take it back for a few seconds.
    pub fn undoable(&mut self, text: impl Into<String>, undo: Undo) {
        self.notice(text.into(), false, Some(8.0), Some(NoticeAction::Undo(undo)));
    }

    fn notice(&mut self, text: String, error: bool, life: Option<f64>, action: Option<NoticeAction>) {
        let born = self.ctx.input(|i| i.time);
        // the same words again replace the earlier ones (a repeated note, a retried save)
        self.notices.retain(|n| n.text != text);
        self.notices.push(Notice { text, error, born, life, action });
        // a handful at most: the oldest short notes make room
        while self.notices.len() > 5 {
            let at = self.notices.iter().position(|n| n.life.is_some()).unwrap_or(0);
            self.notices.remove(at);
        }
        self.ctx.request_repaint();
    }

    /// Put back what a notice's Undo button stands for.
    pub fn undo_action(&mut self, undo: Undo) {
        match undo {
            Undo::Album { at, album } => {
                let at = at.min(self.store.albums.len());
                // albums after it move down again
                if let Source::Album(i) = &mut self.source
                    && *i >= at
                {
                    *i += 1;
                }
                self.store.albums.insert(at, album);
                self.dirty = true;
            }
            Undo::Root { at, path } => {
                if !self.store.roots.contains(&path) {
                    let at = at.min(self.store.roots.len());
                    self.store.roots.insert(at, path);
                    self.dirty = true;
                    self.rescan();
                }
            }
            Undo::Edits(list) => {
                for (key, edits) in list {
                    if let Some(&id) = self.key_index.get(&key) {
                        self.meta_mut(id).edits = edits;
                        self.refresh_thumb(id);
                    }
                }
            }
        }
    }

    /// Start a piece of background work the user watches in the status bar (progress, Cancel).
    pub fn start_task(&mut self, label: impl Into<String>) -> crate::backend::Task {
        let id = self.next_task;
        self.next_task += 1;
        let stop = Arc::new(std::sync::atomic::AtomicBool::new(false));
        self.tasks.push(RunningTask { id, label: label.into(), done: 0, total: 0, stop: stop.clone() });
        crate::backend::Task { id, stop }
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
        let task = self.start_task(format!("Exporting {} picture(s)", jobs.len()));
        self.lib.export(task, jobs, dir, max_dim, quality, self.store.export_location);
        self.arm_tray_check();
    }

    /// A name for what is in the tray: the album, the person, or the folder of its first picture.
    pub fn tray_title(&self) -> String {
        match &self.source {
            Source::Album(i) => self.store.albums.get(*i).map(|a| a.name.clone()),
            Source::Person(n) => Some(n.clone()),
            _ => self.tray().first().and_then(|&id| self.photos.get(id)).and_then(|p| self.folders.get(p.folder)).map(|f| f.name.clone()),
        }
        .unwrap_or_else(|| "My pictures".into())
    }

    /// Open the web-gallery dialog for the tray, titled after what is shown.
    pub fn open_gallery_dialog(&mut self) {
        let title = self.tray_title();
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
        let task = self.start_task(format!("Making the web gallery of {} picture(s)", items.len()));
        self.lib.make_gallery(task, items, parent, opts);
        self.arm_tray_check();
    }

    pub fn make_movie(&mut self, ids: Vec<usize>, size: (u32, u32)) {
        let Some(out) = rfd::FileDialog::new().set_title("Save the movie as…").set_file_name("Slideshow.mp4").add_filter("MP4 video", &["mp4"]).save_file() else {
            return;
        };
        let out = if out.extension().is_some_and(|e| e.eq_ignore_ascii_case("mp4")) { out } else { out.with_extension("mp4") };
        let pictures: Vec<(String, Vec<EditOp>)> = ids.iter().filter_map(|&id| Some((self.photos.get(id)?.key.clone(), self.edits(id)))).collect();
        let task = self.start_task("Making the movie");
        self.lib.make_movie(task, pictures, out, size, self.store.slideshow.clone());
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
                    // by file path: the picture's number may have changed (a scan) since the request
                    let _ = id;
                    if let Some(&id) = self.key_index.get(&key) {
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
                    let _ = id;
                    if let Some(&id) = self.key_index.get(&key) {
                        self.big_req.remove(&id);
                        if let Some(img) = img {
                            self.big.insert(id, BigEntry { base: img, stage: None, tex: None, alt_tex: None, hist: None });
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
                            // not asked for again: the editor would otherwise decode a damaged
                            // or vanished file over and over
                            self.big_failed.insert(id);
                            self.thumb_failed.insert(id);
                        }
                    }
                }
                Done::Zoom { id, key, ops, img, unedited_width } => {
                    self.edit.zoom_req = None;
                    let _ = id;
                    if let Some(&id) = self.key_index.get(&key)
                        && id == self.edit.id
                        && let Some(img) = img {
                            let real_w = self.lib.source_dims(&key).map(|d| d.0).unwrap_or(unedited_width);
                            let r = real_w.max(1) as f32 / unedited_width.max(1) as f32;
                            let tex = self.ctx.load_texture(format!("zoom{id}"), to_color_image(&img), TextureOptions::LINEAR);
                            self.edit.zoom_tex = Some((id, ops, tex, r));
                        }
                }
                Done::Scan(found, truncated, inis) => {
                    self.scanning = false;
                    // sample pictures only for a library without folders: folders that can't
                    // be reached (a disk not plugged in) show as an empty library that says so
                    if found.is_empty() && self.store.roots.is_empty() {
                        self.set_library(library::sample_folders(), true);
                    } else {
                        self.set_library_with(found, false, inis);
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
                Done::Dropped(keys) => {
                    // not made: asked for again when the picture is drawn again
                    for key in keys {
                        if let Some(id) = self.key_index.get(&key) {
                            self.thumb_req.remove(id);
                        }
                    }
                }
                Done::File { key, dim, img } => {
                    if dim == library::BIG_DIM && self.collage.as_ref().is_some_and(|c| c.bg_key.as_deref() == Some(key.as_str())) {
                        self.on_collage_background(key, img);
                    } else {
                        self.on_import_file(key, dim, img);
                    }
                }
                Done::Dims(found) => {
                    self.dims_pending = false;
                    self.shape_rev += 1;
                    for (_, key, dims) in found {
                        if let Some(&id) = self.key_index.get(&key) {
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
                Done::Progress { task, done, total } => {
                    if let Some(t) = self.tasks.iter_mut().find(|t| t.id == task) {
                        (t.done, t.total) = (done, total);
                    }
                }
                Done::Report(report) => {
                    if let Some(id) = report.task {
                        self.tasks.retain(|t| t.id != id);
                    }
                    if report.ok {
                        // long enough to read and to click "Show in folder"
                        let action = report.show.map(NoticeAction::Show);
                        self.notice(report.text, false, Some(if action.is_some() { 12.0 } else { 6.0 }), action);
                    } else {
                        let action = report.show.map(NoticeAction::Show);
                        self.notice(report.text, true, None, action);
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

/// The selection line of the status bar: "No Selection"; for one picture its name, date, pixel
/// size and file size; for several their number, earliest and latest date (once when they read
/// the same) and total size on disk.
pub fn selection_text(items: &[(String, i64, u64)], one_date: Option<&str>, dims: Option<(u32, u32)>) -> String {
    match items {
        [] => "No Selection".into(),
        [(name, modified, bytes)] => {
            let date = one_date.map(str::to_string).unwrap_or_else(|| library::fmt_date(*modified));
            let (w, h) = dims.unwrap_or((0, 0));
            format!("{name}     {date}     {w}x{h} pixels     {}", library::fmt_bytes(*bytes))
        }
        _ => several_text(items.len(), items.iter().map(|i| (i.1, i.2))),
    }
}

/// The selection line for `count` pictures, from the (date, bytes) of each.
pub fn several_text(count: usize, items: impl Iterator<Item = (i64, u64)>) -> String {
    let (mut first, mut last, mut total) = (i64::MAX, i64::MIN, 0u64);
    for (date, bytes) in items {
        first = first.min(date);
        last = last.max(date);
        total = total.saturating_add(bytes);
    }
    if first > last {
        (first, last) = (0, 0);
    }
    let (a, b) = (library::fmt_date(first), library::fmt_date(last));
    let mut s = format!("{} pictures", fmt_count(count));
    if a == b {
        s += &format!("     {a}      {} on disk", library::fmt_bytes(total));
    } else {
        s += &format!("     {a} to {b}     {} on disk", library::fmt_bytes(total));
    }
    s
}

impl App {
    fn save_finished(&mut self, result: Result<(), String>) {
        match result {
            Ok(()) => {
                if self.save_error.take().is_some() {
                    self.notices.retain(|n| !n.text.starts_with(SAVE_FAILED));
                    self.toast("Your library is saved again.");
                }
            }
            Err(e) => {
                // still unsaved: it is tried again in a few seconds
                self.unsaved = true;
                if self.save_error.as_ref() != Some(&e) {
                    // stays until it works again or the user closes it
                    self.notices.retain(|n| !n.text.starts_with(SAVE_FAILED));
                    self.error(format!("{SAVE_FAILED} {e}\nYour latest changes are only in this window for now; AlbumCraft keeps trying."));
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
        self.thumb_budget = 8;
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
        // every change to the library's notes sets `dirty`: rebuild the cached lists once after
        // it (not on every frame until the save), and remember that there is something to save
        if std::mem::take(&mut self.dirty) {
            self.list_rev += 1;
            self.unsaved = true;
        }
        // collect a finished background save (when closing, wait for it)
        if let Some(result) = self.lib.take_saved(closing) {
            self.save_finished(result);
        }
        // one save at a time: they share the temporary file
        if self.persist && self.unsaved && !self.lib.saving() && (closing || now - self.last_save > 3.0) {
            self.last_save = now;
            // changes made from here on set `dirty` again and are saved next time
            self.unsaved = false;
            // the window closes after this frame: write now
            self.lib.save(&mut self.store, closing);
            if let Some(result) = self.lib.take_saved(closing) {
                self.save_finished(result);
            }
        }
        if self.lib.saving() {
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
        } else if self.unsaved && self.persist {
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
            View::Collage => self.collage_ui(ui),
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
        // a change made in this frame is taken up (lists rebuilt, save planned) by the next one
        if self.dirty {
            ctx.request_repaint();
        }
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
    fn show_map(&mut self, ids: &[usize], title: String) -> Result<(), String> {
        if !ids.iter().any(|&id| self.location(id).is_some()) {
            return Err("none of these pictures has a location".into());
        }
        self.show_set_on_map(title, ids);
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
        // the same line from dates and sizes alone, and no pictures at all is not a crash
        assert_eq!(several_text(2, [(day, 1 << 20), (day + 60, 1 << 20)].into_iter()), selection_text(&two, None, None));
        assert!(several_text(0, std::iter::empty()).starts_with("0 pictures"));
        assert_eq!(fmt_count(999), "999");
        assert_eq!(fmt_count(1234), "1,234");
        assert_eq!(fmt_count(1234567), "1,234,567");
        assert_eq!(exif_date_slashes("2009:05:12 10:00:00"), "2009/05/12 10:00:00");
    }
}
