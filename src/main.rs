#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod edit_view;
mod exifinfo;
mod imaging;
mod library;
mod library_view;
mod shell;
mod print;
mod print_view;
mod theme;
mod thumbcache;

use std::path::PathBuf;

fn main() -> eframe::Result<()> {
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
