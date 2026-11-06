#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod backend;
mod basemap;
mod edit_view;
mod library_view;
mod map_view;
mod online;
mod print;
mod print_view;
mod shell;
mod slides;
mod theme;

// Everything without a window lives in the albumcraft-core and albumcraft-edit crates. Their
// modules are used here under the same names (`crate::library`, `crate::imaging`, …).
// `online`, `print` and `slides` above add the window's parts (map tiles as textures, printers,
// music) to the core modules of the same name.
pub use albumcraft_core::{commands, control, decor, develop, exifinfo, export, gallery, geo, imaging, library, looks, mcp, movie, raw, thumbcache};

use std::path::PathBuf;

/// The gecko app icon (original artwork, `assets/app-icon/albumcraft.svg`), as PNGs.
pub const ICON_64: &[u8] = include_bytes!("../assets/app-icon/hicolor/64x64/apps/albumcraft.png");
const ICON_256: &[u8] = include_bytes!("../assets/app-icon/hicolor/256x256/apps/albumcraft.png");

fn window_icon() -> eframe::egui::IconData {
    match image::load_from_memory(ICON_256) {
        Ok(img) => {
            let img = img.to_rgba8();
            eframe::egui::IconData { width: img.width(), height: img.height(), rgba: img.into_raw() }
        }
        Err(_) => eframe::egui::IconData::default(),
    }
}

/// Where panics are written down (release builds have no console on Windows).
pub fn crash_log_path() -> Option<PathBuf> {
    Some(dirs::data_dir()?.join("AlbumCraft").join("crash.log"))
}

/// Marker left by a panic on the main thread (the app then closes); the next start tells the user.
pub fn crash_marker_path() -> Option<PathBuf> {
    Some(dirs::data_dir()?.join("AlbumCraft").join("crashed"))
}

/// Log every panic (with a backtrace) to crash.log before the default handling. Background
/// workers recover from theirs; a panic on the main thread ends the program, so it also leaves
/// a marker for the next start.
fn install_panic_hook() {
    let default = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let thread = std::thread::current().name().unwrap_or("unnamed").to_string();
        if let Some(path) = crash_log_path() {
            if let Some(dir) = path.parent() {
                let _ = std::fs::create_dir_all(dir);
            }
            // keep the log small: start over (keeping one old copy) past 1 MB
            if std::fs::metadata(&path).is_ok_and(|m| m.len() > 1 << 20) {
                let _ = std::fs::rename(&path, path.with_extension("old.log"));
            }
            let when = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
            let backtrace = std::backtrace::Backtrace::force_capture();
            let entry = format!("--- {when} AlbumCraft {} (thread {thread})\n{info}\n{backtrace}\n", env!("CARGO_PKG_VERSION"));
            if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(&path) {
                use std::io::Write;
                let _ = f.write_all(entry.as_bytes());
            }
        }
        if thread == "main"
            && let Some(marker) = crash_marker_path() {
                let _ = std::fs::write(marker, b"");
            }
        default(info);
    }));
}

fn main() -> eframe::Result<()> {
    install_panic_hook();
    if std::env::args().skip(1).any(|a| a == "--mcp") {
        // MCP server on stdin/stdout; no window
        let code = match mcp::Config::default_paths() {
            Ok(cfg) => match mcp::serve(std::io::stdin().lock(), std::io::stdout().lock(), &cfg) {
                Ok(()) => 0,
                Err(e) => {
                    eprintln!("albumcraft --mcp: {e}");
                    1
                }
            },
            Err(e) => {
                eprintln!("albumcraft --mcp: {e}");
                1
            }
        };
        std::process::exit(code);
    }
    let trace = std::env::var_os("ALBUMCRAFT_TRACE").is_some();
    if trace {
        eprintln!("start");
    }
    let mut screenshot: Option<PathBuf> = None;
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        if a == "--screenshot" {
            screenshot = args.next().map(PathBuf::from);
        } else if let Some(v) = a.strip_prefix("--screenshot=") {
            screenshot = Some(PathBuf::from(v));
        }
    }

    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title("AlbumCraft")
            .with_inner_size([1280.0, 800.0])
            .with_min_inner_size([900.0, 560.0])
            .with_icon(window_icon()),
        ..Default::default()
    };
    eframe::run_native(
        "AlbumCraft",
        options,
        Box::new(move |cc| {
            if trace {
                eprintln!("creating app");
            }
            Ok(Box::new(app::App::new(cc, screenshot)))
        }),
    )
}
