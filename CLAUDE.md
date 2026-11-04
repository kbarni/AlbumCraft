# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

**AlbumCraft**: a native Rust/egui photo organizer with a simple non-destructive editor, part of the *Craft* apps suite. See `AGENTS.md` for the rules and `new_features.md` for the plan and status.

This directory is the git repository and the Cargo project. The parent directory holds local research notes and generated tool bookkeeping; they are deliberately not in the repository.

## Using it from an agent

Register the MCP server with an MCP client, e.g. for Claude Code: `claude mcp add albumcraft -- /path/to/albumcraft --mcp`. Works with or without the window open.

## Build / run / test

Run from the repository root:

```
cargo build            # dev profile: opt-level 1 for this crate, 3 for dependencies
cargo run
cargo test             # tests are inline #[cfg(test)] modules (library.rs ini_tests, imaging.rs filter_tests, exifinfo.rs, edit_view.rs, app.rs tray_tests, print.rs, thumbcache.rs)
cargo test <name>      # single test, e.g. cargo test ini_tests
cargo run -- --screenshot out.png   # renders a frame, saves it, and exits (used for `docs/screenshots/`; on a tiling window manager retry until the window is wide)
```

- `rust-toolchain.toml` selects plain `stable` (plus clippy and rustfmt) on every platform. On a Windows machine without the MSVC linker, run `rustup set default-host x86_64-pc-windows-gnu` once; a local, gitignored `.cargo/config.toml` can hold that machine's GNU linker workarounds (paths to `.tools/`, used only for the `x86_64-pc-windows-gnu` target). The `windows` crate is a `[target.'cfg(windows)'.dependencies]` entry and is only used under `#[cfg(windows)]` in `print.rs`; non-Windows has a stub `mod other`. `cargo test`: 13 pass, 1 ignored (`prints_to_pdf`).
- CI: `.github/workflows/ci.yml` calls the suite's shared gate (`storytold/craft-common-actions`, `gate.yml@main`) on Ubuntu, Windows and macOS: `cargo build --locked --all-targets && cargo test --locked`. Keep `Cargo.lock` committed and up to date (the gate fails if the build changes it).
- Dev profile deliberately sets `debug = false`, `strip = "debuginfo"`: full debug info produced a 450 MB binary that crashed at startup.
- `ALBUMCRAFT_TRACE=1` prints startup trace lines. `ALBUMCRAFT_THEME=light|dark` forces a theme and `ALBUMCRAFT_SHOT_VIEW=edit|presets|effects|slideshow|movie|print|compare|props|zoom|tray` picks the view for `--screenshot`.
- Release builds set `windows_subsystem = "windows"` (no console).

## Architecture

Single-window eframe/egui app (edition 2024). `main.rs` parses `--screenshot` and runs `app::App`.

- `app.rs` — the `App` struct (all UI state: catalogue `Store`, photos/folders, texture caches, selection, tray/held pictures, edit state, slideshow, print state, dialogs) and the `eframe::App::update` loop. `View` selects Library / Edit / etc.; `Dialog`, `Tool`, `Tab`, `CompareMode` enumerate modes. Also theme setup and screenshot capture.
- `library.rs` — the catalogue data model (`Store`, `Meta`, `Album`, `Folder`, `Photo`), JSON persistence, `.picasa.ini` import, folder scan, and the background loader `Pool` (worker threads fed by a `Job` queue, returning `Done` messages over an mpsc channel; thumbnails at 256 px, "big" at 1400 px, sharp zoom render up to 5000 px with edits applied).
- `thumbcache.rs` — on-disk thumbnail cache in `dirs::cache_dir()/AlbumCraft/thumbs`, keyed by path, mtime, size and thumbnail size.
- `theme.rs` — light/dark colour tokens (`t()` returns the active `Tokens`; never hard-code UI colours in views) and the egui style built from them. Mode (system/light/dark) is saved in `Store.theme`, resolved each frame in `App::logic`, and cycled by the theme button in the top bar. Paper sheets in print preview and overlays drawn on top of pictures intentionally stay white/dark in both themes.
- `shell.rs` — the Library UI: top bar (search pill, Picks chip, rating/reject filter chip, size slider, theme toggle, Import), sidebar (app title with the ☰ menu button, Picks, albums/people/folders), justified-row photo grid (aspect-preserving, check circle + pick flag on hover; pick flag, rating stars and a dimmed reject mark shown on tiles), photo tray and status bar at the bottom (`library_view::tray_panel` / `status_bar`). The full menus live behind the ☰ button (`library_view::app_menu_items`).
- `library_view.rs` — shared library helpers: `paint_star`, key handling (`library_keys`), the menu items and the per-picture context menu (`cell_menu`).
- `edit_view.rs` — editor UI and slideshow. Tabs: **Edit** (Basic: one-click fixes and tools; Advanced: the sliders, editing the last `EditOp::Adjust` step in place), **Presets** and **Effects**; crop/straighten/red-eye/pick-neutral tools draw on the single view; compare modes; histogram with clipping warning.
- `looks.rs` — preset looks (`Look`: vivid, soft film, instant, cross process, lomo, matte, teal & orange, high key, cool, warm, B & W, dramatic B & W, sepia, soft focus, glow, film grain, focal B & W) as `EditOp::Look { look, amount }`: the full look blended with the picture by intensity. Choosing a look right after another replaces it (`set_look` in edit_view).
- `decor.rs` — the Effects tab: `Decor` (vignette, border / instant frame, drop shadow, rounded corners, text with anchor, size, colour and halo) as `EditOp::Decor`; sizes are fractions of the picture so preview and export match. The Effects tab edits the last step when it is the same kind; the add buttons always add a new step. Older one-click `EditOp::Effect` steps still render (several are reused by presets).
- `develop.rs` — `Adjust` (white balance, exposure, highlights, shadows, blacks, contrast, saturation, vibrance, clarity, sharpen, noise reduction; rayon-parallel, radii scale with picture size) and the analysis behind the one-click fixes (`auto_exposure`, `auto_white_balance`, `lucky`, `neutralize`, `auto_straighten`). `cargo test --release develop_speed -- --ignored --nocapture` times it (release, 20 cores: preview 1400 px ~10 ms tone/colour, ~90 ms with all detail sliders; 24 MP ~1.2 s).
- `imaging.rs` — non-destructive edit pipeline: `EditOp` / `Effect` applied on the fly by `apply_all`; image loading (`load_source`); built-in sample images (keys prefixed `SAMPLE_PREFIX`) used when no real folders are scanned.
- `raw.rs` — camera raw files (CR2, CR3, NEF, ARW, DNG, ORF, RW2, RAF…) shown through their largest embedded JPEG preview, turned by the raw file's EXIF orientation; `imaging::load_source` / `oriented_dims` route raw files here. No sensor-data developing. `ALBUMCRAFT_RAW_SAMPLE=<file> cargo test real_raw_file -- --ignored --nocapture` checks a real file.
- `slides.rs` — slideshow settings (`Store.slideshow`: seconds per picture, crossfade, pan-and-zoom motion, music file), `ken_burns_uv` / `fade_in` (shared with the movie export) and `Music` (rodio, looping; Linux builds need the ALSA headers, `libasound2-dev`).
- `gallery.rs` — web gallery: a new folder (never an existing one) with `images/`, `thumbs/` and a self-contained `index.html` (inline CSS + lightbox script; light grid, dark grid or masonry). Captions and titles are HTML-escaped and the lightbox data is JSON with `<` escaped. `ALBUMCRAFT_GALLERY_OUT=<dir> cargo test demo_gallery -- --ignored` writes demos.
- `movie.rs` — movie export: frames rendered with the slideshow's timing (`at_time`), crossfade and pan-and-zoom, piped as raw RGBA to the `ffmpeg` program (H.264 if available, else MPEG-4; looped music muxed with `-shortest`). Needs FFmpeg on the PATH; the Movie dialog says so when it is missing. The test makes a real short movie when FFmpeg is installed and skips otherwise.
- `commands.rs` — the command layer: 23 named commands with JSON-schema parameters (search, flag, rating, caption, tags/people, albums, add folder, edit steps, auto fixes, undo, copy edits, `render_preview` returning a PNG, export, web gallery, movie, show in editor). They run on a `Host`: the window (`App`, impl at the end of app.rs) or `Headless` (library.json without a window). Pictures are named by key (file path). Slow commands return `Outcome::Later` (a closure run off the UI thread). Bad parameters are `Err`, never panics (`bad_input_is_an_error_not_a_crash`).
- `mcp.rs` — `albumcraft --mcp`: MCP server (JSON-RPC 2.0, one message per line on stdin/stdout; `initialize`, `tools/list`, `tools/call`, `ping`). Each call goes to the open window through `control.rs` when there is one, otherwise to a `Headless` library that is reloaded before and saved after each changing call. Nothing but protocol messages may be written to stdout in this mode.
- `control.rs` — the window's control connection: 127.0.0.1 on a free port, a random 256-bit token, both in `<data dir>/AlbumCraft/control.json` (mode 0600, removed when the window closes); one JSON line per request, 1 MB limit, 4 connections, wrong token closes the connection. The window runs requests in `App::run_control_requests` each frame.
- `exifinfo.rs` — EXIF/camera info and embedded thumbnails (kamadak-exif).
- `print.rs` / `print_view.rs` — print layouts, paper/borders/captions; printing through Win32 GDI on Windows and through CUPS on Linux/macOS (`print.rs` `mod cups`: `lpstat`/`lpoptions` for printers, paper sizes and resolution; the pages go to `lp` as a PDF written by `pdf_of_pages`, JPEG pages placed on the printable area, 5 mm margins). `print::available()` hides every print entry point when neither works (no CUPS scheduler). Tests never send a job: they write the PDF to a file (`ALBUMCRAFT_PDF_OUT=out.pdf cargo test cups_pdf_demo -- --ignored` for a sample).

Key design rules:
- **Originals are never modified.** Edits, stars, captions, tags, people, albums are stored as `Meta` in `library.json` under the user data dir (`dirs::data_dir()/AlbumCraft/library.json`, see `library::store_path`; a library saved under the earlier working name `PhotoShelf/` is copied over on first start). Photos are keyed by file path string.
- **Culling:** `Meta.flag` (`Flag::None/Pick/Reject`) and `Meta.rating` (0–5). Keys P / X / U / 0–5 act on the tray (library) or the current picture (editor); `library_view::flag_rating_menu` / `flag_rating_keys`. The yes/no `star` of older libraries and `.picasa.ini` `star=yes` become picks (`Meta::upgrade`, on load).
- **Copy / paste edits:** `App::copy_edits` / `paste_edits` (Ctrl+Shift+C / Ctrl+Shift+V; plain Ctrl+C/V reach egui as clipboard events, not keys). Pasting appends the copied ops (red-eye fixes excluded, `copyable_edits`) to each picture's own edits.
- **`.picasa.ini` files are read once per folder and never written** (tracked via `Store.imported_ini`).
- All decoding/rendering goes through the `Pool`; UI code requests via `Job`s and consumes `Done` results each frame (textures are cached in `App` maps keyed by photo id).

## Other files

- `new_features.md` — feature plan, phases, review findings (phase 0) and status.
- `docs/screenshots/` — README screenshots (JPEG, 1400 px wide); `scratch/` (gitignored) is throwaway.
- Releases: push a tag `v<version>` (matching `Cargo.toml`); `.github/workflows/release.yml` builds Linux x86_64 (tar.gz with `.desktop` file), Windows x86_64 (zip) and macOS arm64 (ad-hoc signed `.app` zip) and publishes a GitHub release (pre-release when the version has a `-`), with notes from `docs/release-notes/<version>.md`. `packaging/` holds the `.desktop` file and `Info.plist`. License: MIT OR Apache-2.0 (`LICENSE-MIT`, `LICENSE-APACHE`, `NOTICE`).
- `assets/app-icon/` — the gecko app icon: `albumcraft.svg` is the hand-made source; PNG sizes, `hicolor/` layout and `.ico` are exported from it (commands in its README). `main.rs` embeds the 64 px (sidebar badge, `App::app_icon`) and 256 px (window icon) PNGs.
