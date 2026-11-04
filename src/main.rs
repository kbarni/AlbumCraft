#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod decor;
mod develop;
mod edit_view;
mod exifinfo;
mod gallery;
mod imaging;
mod library;
mod library_view;
mod looks;
mod shell;
mod slides;
mod print;
mod print_view;
mod raw;
mod theme;
mod thumbcache;

use std::path::PathBuf;

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
            .with_min_inner_size([900.0, 560.0]),
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
