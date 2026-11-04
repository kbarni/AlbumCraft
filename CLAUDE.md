# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

**AlbumCraft**: a native Rust/egui photo organizer with a simple non-destructive editor, part of the *Craft* apps suite. See `AGENTS.md` for the rules and `new_features.md` for the plan and status.

This directory is the git repository and the Cargo project. The parent directory holds local research notes and generated tool bookkeeping; they are deliberately not in the repository.

## Build / run / test

Run from the repository root:

```
cargo build            # dev profile: opt-level 1 for this crate, 3 for dependencies
cargo run
cargo test             # tests are inline #[cfg(test)] modules (library.rs ini_tests, imaging.rs filter_tests, exifinfo.rs, edit_view.rs, app.rs tray_tests, print.rs, thumbcache.rs)
cargo test <name>      # single test, e.g. cargo test ini_tests
cargo run -- --screenshot out.png   # renders a frame, saves it, and exits (used for the screenshots in `shots/`)
```

- `rust-toolchain.toml` selects plain `stable` (plus clippy and rustfmt) on every platform. On a Windows machine without the MSVC linker, run `rustup set default-host x86_64-pc-windows-gnu` once; a local, gitignored `.cargo/config.toml` can hold that machine's GNU linker workarounds (paths to `.tools/`, used only for the `x86_64-pc-windows-gnu` target). The `windows` crate is a `[target.'cfg(windows)'.dependencies]` entry and is only used under `#[cfg(windows)]` in `print.rs`; non-Windows has a stub `mod other`. `cargo test`: 13 pass, 1 ignored (`prints_to_pdf`).
- CI: `.github/workflows/ci.yml` calls the suite's shared gate (`storytold/craft-common-actions`, `gate.yml@main`) on Ubuntu, Windows and macOS: `cargo build --locked --all-targets && cargo test --locked`. Keep `Cargo.lock` committed and up to date (the gate fails if the build changes it).
- Dev profile deliberately sets `debug = false`, `strip = "debuginfo"`: full debug info produced a 450 MB binary that crashed at startup.
- `ALBUMCRAFT_TRACE=1` prints startup trace lines. `ALBUMCRAFT_THEME=light|dark` forces a theme and `ALBUMCRAFT_SHOT_VIEW=edit|print|compare|props|zoom|tray` picks the view for `--screenshot`.
- Release builds set `windows_subsystem = "windows"` (no console).

## Architecture

Single-window eframe/egui app (edition 2024). `main.rs` parses `--screenshot` and runs `app::App`.

- `app.rs` — the `App` struct (all UI state: catalogue `Store`, photos/folders, texture caches, selection, tray/held pictures, edit state, slideshow, print state, dialogs) and the `eframe::App::update` loop. `View` selects Library / Edit / etc.; `Dialog`, `Tool`, `Tab`, `CompareMode` enumerate modes. Also theme setup and screenshot capture.
- `library.rs` — the catalogue data model (`Store`, `Meta`, `Album`, `Folder`, `Photo`), JSON persistence, `.picasa.ini` import, folder scan, and the background loader `Pool` (worker threads fed by a `Job` queue, returning `Done` messages over an mpsc channel; thumbnails at 256 px, "big" at 1400 px, sharp zoom render up to 5000 px with edits applied).
- `thumbcache.rs` — on-disk thumbnail cache in `dirs::cache_dir()/AlbumCraft/thumbs`, keyed by path, mtime, size and thumbnail size.
- `theme.rs` — light/dark colour tokens (`t()` returns the active `Tokens`; never hard-code UI colours in views) and the egui style built from them. Mode (system/light/dark) is saved in `Store.theme`, resolved each frame in `App::logic`, and cycled by the theme button in the top bar. Paper sheets in print preview and overlays drawn on top of pictures intentionally stay white/dark in both themes.
- `shell.rs` — the Library UI: top bar (search pill, Picks chip, rating/reject filter chip, size slider, theme toggle, Import), sidebar (app title with the ☰ menu button, Picks, albums/people/folders), justified-row photo grid (aspect-preserving, check circle + pick flag on hover; pick flag, rating stars and a dimmed reject mark shown on tiles), photo tray and status bar at the bottom (`library_view::tray_panel` / `status_bar`). The full menus live behind the ☰ button (`library_view::app_menu_items`).
- `library_view.rs` — shared library helpers: `paint_star`, key handling (`library_keys`), the menu items and the per-picture context menu (`cell_menu`).
- `edit_view.rs` — editor UI and slideshow. Tabs: **Edit** (Basic: one-click fixes and tools; Advanced: the sliders, editing the last `EditOp::Adjust` step in place) and **Effects**; crop/straighten/red-eye/pick-neutral tools draw on the single view; compare modes; histogram with clipping warning.
- `develop.rs` — `Adjust` (white balance, exposure, highlights, shadows, blacks, contrast, saturation, vibrance, clarity, sharpen, noise reduction; rayon-parallel, radii scale with picture size) and the analysis behind the one-click fixes (`auto_exposure`, `auto_white_balance`, `lucky`, `neutralize`, `auto_straighten`). `cargo test --release develop_speed -- --ignored --nocapture` times it (release, 20 cores: preview 1400 px ~10 ms tone/colour, ~90 ms with all detail sliders; 24 MP ~1.2 s).
- `imaging.rs` — non-destructive edit pipeline: `EditOp` / `Effect` applied on the fly by `apply_all`; image loading (`load_source`); built-in sample images (keys prefixed `SAMPLE_PREFIX`) used when no real folders are scanned.
- `raw.rs` — camera raw files (CR2, CR3, NEF, ARW, DNG, ORF, RW2, RAF…) shown through their largest embedded JPEG preview, turned by the raw file's EXIF orientation; `imaging::load_source` / `oriented_dims` route raw files here. No sensor-data developing. `ALBUMCRAFT_RAW_SAMPLE=<file> cargo test real_raw_file -- --ignored --nocapture` checks a real file.
- `exifinfo.rs` — EXIF/camera info and embedded thumbnails (kamadak-exif).
- `print.rs` / `print_view.rs` — print layouts, paper/borders/captions; real printing through Win32 GDI on Windows only.

Key design rules:
- **Originals are never modified.** Edits, stars, captions, tags, people, albums are stored as `Meta` in `library.json` under the user data dir (`dirs::data_dir()/AlbumCraft/library.json`, see `library::store_path`; a library saved under the earlier working name `PhotoShelf/` is copied over on first start). Photos are keyed by file path string.
- **Culling:** `Meta.flag` (`Flag::None/Pick/Reject`) and `Meta.rating` (0–5). Keys P / X / U / 0–5 act on the tray (library) or the current picture (editor); `library_view::flag_rating_menu` / `flag_rating_keys`. The yes/no `star` of older libraries and `.picasa.ini` `star=yes` become picks (`Meta::upgrade`, on load).
- **Copy / paste edits:** `App::copy_edits` / `paste_edits` (Ctrl+Shift+C / Ctrl+Shift+V; plain Ctrl+C/V reach egui as clipboard events, not keys). Pasting appends the copied ops (red-eye fixes excluded, `copyable_edits`) to each picture's own edits.
- **`.picasa.ini` files are read once per folder and never written** (tracked via `Store.imported_ini`).
- All decoding/rendering goes through the `Pool`; UI code requests via `Job`s and consumes `Done` results each frame (textures are cached in `App` maps keyed by photo id).

## Other files

- `new_features.md` — feature plan, phases, review findings (phase 0) and status.
- `shots/` — screenshots of the app for comparison; `scratch/` (gitignored) is throwaway.
