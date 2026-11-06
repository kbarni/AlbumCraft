# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

**AlbumCraft**: a native Rust/egui photo organizer with a simple non-destructive editor, part of the *Craft* apps suite. See `AGENTS.md` for the rules and `new_features.md` for the plan and status.

This directory is the git repository and the Cargo workspace (see Code layout below). The parent directory holds local research notes and generated tool bookkeeping; they are deliberately not in the repository.

## Using it from an agent

Register the MCP server with an MCP client, e.g. for Claude Code: `claude mcp add albumcraft -- /path/to/albumcraft --mcp`. Works with or without the window open.

## Build / run / test

Run from the repository root:

```
cargo build            # dev profile: opt-level 1 for the workspace crates, 3 for dependencies
cargo run
cargo test             # every crate of the workspace; tests are inline #[cfg(test)] modules next to the code
cargo test -p albumcraft-core   # one crate
cargo test <name>      # single test, e.g. cargo test ini_tests
cargo run -- --screenshot out.png   # renders a frame, saves it, and exits (used for `docs/screenshots/`; on a tiling window manager retry until the window is wide)
```

- `rust-toolchain.toml` selects plain `stable` (plus clippy and rustfmt) on every platform. On a Windows machine without the MSVC linker, run `rustup set default-host x86_64-pc-windows-gnu` once; a local, gitignored `.cargo/config.toml` can hold that machine's GNU linker workarounds (paths to `.tools/`, used only for the `x86_64-pc-windows-gnu` target). The `windows` crate is a `[target.'cfg(windows)'.dependencies]` entry and is only used under `#[cfg(windows)]` in `print.rs`; non-Windows has a stub `mod other`. `cargo test`: 87 pass, 7 ignored (demos, data converters, network and real-file checks).
- CI: `.github/workflows/ci.yml` calls the suite's shared gate (`storytold/craft-common-actions`, `gate.yml@main`) on Ubuntu, Windows and macOS: `cargo build --locked --all-targets && cargo test --locked`. Keep `Cargo.lock` committed and up to date (the gate fails if the build changes it).
- Dev profile deliberately sets `debug = false`, `strip = "debuginfo"`: full debug info produced a 450 MB binary that crashed at startup.
- `ALBUMCRAFT_TRACE=1` prints startup trace lines. `ALBUMCRAFT_THEME=light|dark` forces a theme and `ALBUMCRAFT_SHOT_VIEW=edit|presets|effects|slideshow|movie|print|compare|props|zoom|tray|map|place|pin|maps|geometry` picks the view for `--screenshot` (`map`: Places with sample locations, `place`/`pin`: Set location, `maps`: the online-maps question); `ALBUMCRAFT_SHOT_ONLINE=1` shows OpenStreetMap tiles in the map shots (downloads them; the screenshot waits for them).
- Windows without the MSVC linker (this machine): `$env:Path = "C:\msys64\ucrt64\bin;" + $env:Path; cargo +stable-x86_64-pc-windows-gnu test` (MinGW from MSYS2 provides `dlltool`). Edit files with the editor, not PowerShell 5.1 `Get-Content`/`Set-Content` (they mangle UTF-8).
- Release builds set `windows_subsystem = "windows"` (no console).

## Code layout

A Cargo workspace (edition 2024) of three crates, split so that a planned server (Docker) and browser (WebAssembly) version can share the code:

- `crates/albumcraft-edit` — the edit pipeline only (`imaging`, `develop`, `looks`, `decor`): pictures in memory, no file access, no threads of its own. Builds for `wasm32-unknown-unknown` (`cargo build -p albumcraft-edit --target wasm32-unknown-unknown`); keep it that way.
- `crates/albumcraft-core` — everything without a window: `library`, `commands`, `mcp`, `control`, `exifinfo`, `raw`, `geo`, `thumbcache`, `export`, `gallery`, `movie`, `online` (services), `print` (layouts), `slides` (settings). Re-exports the edit crate's modules; its `imaging` adds file loading (`load_source`, `oriented_dims`) to the pipeline.
- `albumcraft` (the root package, `src/`) — the egui app. `main.rs` re-exports the core modules (`pub use albumcraft_core::{library, imaging, …}`), so app code keeps writing `crate::library::…`. Its `online.rs`, `print.rs` and `slides.rs` glob-re-export the core module of the same name and add the window's part (tile textures, printers, music).
- `src/backend.rs` — the `Library` interface between `App` and the pictures: load / save the notes, scan folders, thumbnail / preview / zoom jobs, picture sizes, EXIF, file GPS and the GPS index, `.picasa.ini`, a picture reader for print and collage, export, web gallery, movie, collage. Slow answers come back as `library::Done` on `App.rx`. `LocalLibrary` is today's behaviour (folders on this computer, `library.json`, the `Pool`); a remote library can implement the same trait. `App` and the views must not read files themselves.

## Architecture

Single-window eframe/egui app. `main.rs` parses `--screenshot` / `--mcp` and runs `app::App`.

- `app.rs` — the `App` struct (all UI state: catalogue `Store`, its `lib: Box<dyn Library>`, photos/folders, texture caches, selection, tray/held pictures, edit state, slideshow, print state, dialogs) and the `eframe::App::update` loop. `View` selects Library / Edit / etc.; `Dialog`, `Tool`, `Tab`, `CompareMode` enumerate modes. Also theme setup and screenshot capture.
- `library.rs` — the catalogue data model (`Store`, `Meta`, `Album`, `Folder`, `Photo`), JSON persistence, `.picasa.ini` import, folder scan, and the background loader `Pool` (worker threads fed by a `Job` queue, returning `Done` messages over an mpsc channel and calling a `Wake` callback; thumbnails at 256 px, "big" at 1400 px, sharp zoom render up to 5000 px with edits applied).
- `thumbcache.rs` — on-disk thumbnail cache in `dirs::cache_dir()/AlbumCraft/thumbs`, keyed by path, mtime, size and thumbnail size.
- `theme.rs` — light/dark colour tokens (`t()` returns the active `Tokens`; never hard-code UI colours in views) and the egui style built from them. Mode (system/light/dark) is saved in `Store.theme`, resolved each frame in `App::logic`, and cycled by the theme button in the top bar. Paper sheets in print preview and overlays drawn on top of pictures intentionally stay white/dark in both themes. `set_fonts` adds the bundled Hack font as a fallback for ordinary text. Every character in UI text must be in the bundled fonts (Ubuntu Light, Hack, egui icons) or be a real emoji (drawn from the system's colour emoji font); other symbols (☰, ＋, ★, ⚑…) show as boxes on some systems. The test `every_ui_character_is_in_the_fonts` enforces it.
- `shell.rs` — the Library UI: top bar (search pill, Picks chip, rating/reject filter chip, size slider, theme toggle, Import), sidebar (app title with the ☰ menu button, Picks, albums/people/folders), justified-row photo grid (aspect-preserving, check circle + pick flag on hover; pick flag, rating stars and a dimmed reject mark shown on tiles), photo tray and status bar at the bottom (`library_view::tray_panel` / `status_bar`). The full menus live behind the ☰ button (`library_view::app_menu_items`).
- `library_view.rs` — shared library helpers: `paint_star`, key handling (`library_keys`), the menu items and the per-picture context menu (`cell_menu`).
- `edit_view.rs` — editor UI and slideshow. Tabs: **Edit** (Basic: one-click fixes and tools; Advanced: the sliders, editing the last `EditOp::Adjust` step in place), **Presets** and **Effects**; crop/straighten/red-eye/pick-neutral tools draw on the single view; compare modes; histogram with clipping warning.
- `looks.rs` — preset looks (`Look`: vivid, soft film, instant, cross process, lomo, matte, teal & orange, high key, cool, warm, B & W, dramatic B & W, sepia, soft focus, glow, film grain, focal B & W) as `EditOp::Look { look, amount }`: the full look blended with the picture by intensity. Choosing a look right after another replaces it (`set_look` in edit_view).
- `decor.rs` — the Effects tab: `Decor` (vignette, border / instant frame, drop shadow, rounded corners, text with anchor, size, colour and halo) as `EditOp::Decor`; sizes are fractions of the picture so preview and export match. The Effects tab edits the last step when it is the same kind; the add buttons always add a new step. Older one-click `EditOp::Effect` steps still render (several are reused by presets).
- `develop.rs` — `Adjust` (white balance, exposure, highlights, shadows, blacks, contrast, saturation, vibrance, clarity, sharpen, noise reduction; rayon-parallel, radii scale with picture size) and the analysis behind the one-click fixes (`auto_exposure`, `auto_white_balance`, `lucky`, `neutralize`, `auto_straighten`). `cargo test --release develop_speed -- --ignored --nocapture` times it (release, 20 cores: preview 1400 px ~10 ms tone/colour, ~90 ms with all detail sliders; 24 MP ~1.2 s).
- `imaging.rs` — non-destructive edit pipeline (edit crate): `EditOp` / `Effect` applied on the fly by `apply_all`; built-in sample images (keys prefixed `SAMPLE_PREFIX`, `sample_source`) used when no real folders are scanned. Image loading (`load_source`, `oriented_dims`) is core's `imaging.rs`.
- `export.rs` (core) — `export_one`: an edited JPEG copy, never overwriting a file.
- `raw.rs` — camera raw files (CR2, CR3, NEF, ARW, DNG, ORF, RW2, RAF…) shown through their largest embedded JPEG preview, turned by the raw file's EXIF orientation; `imaging::load_source` / `oriented_dims` route raw files here. No sensor-data developing. `ALBUMCRAFT_RAW_SAMPLE=<file> cargo test real_raw_file -- --ignored --nocapture` checks a real file.
- `slides.rs` — slideshow settings (`Store.slideshow`: seconds per picture, crossfade, pan-and-zoom motion, music file), `ken_burns_uv` / `fade_in` (shared with the movie export) and `Music` (rodio, looping; Linux builds need the ALSA headers, `libasound2-dev`).
- `gallery.rs` — web gallery: a new folder (never an existing one) with `images/`, `thumbs/` and a self-contained `index.html` (inline CSS + lightbox script; light grid, dark grid or masonry). Captions and titles are HTML-escaped and the lightbox data is JSON with `<` escaped. `ALBUMCRAFT_GALLERY_OUT=<dir> cargo test demo_gallery -- --ignored` writes demos.
- `movie.rs` — movie export: frames rendered with the slideshow's timing (`at_time`), crossfade and pan-and-zoom, piped as raw RGBA to the `ffmpeg` program (H.264 if available, else MPEG-4; looped music muxed with `-shortest`). Needs FFmpeg on the PATH; the Movie dialog says so when it is missing. The test makes a real short movie when FFmpeg is installed and skips otherwise.
- `geo.rs` — locations: `LatLon`, `Place` (`Meta.place`: `Set { lat, lon, name }` overrides the file's GPS, `Removed` hides it; `effective` combines them), typed-coordinate parsing, EXIF GPS reading (`file_gps`, `gps_of`) and writing for exports (`exif_gps_fields`), the background GPS index with its cache (`index_files`, `cache_dir()/AlbumCraft/gps.json`), the built-in GeoNames city list (`assets/geo/cities.bin`: `search`, `nearest`, `describe` → "Near Bordeaux, France") and the Web Mercator projection (`to_world`/`from_world`). The data converters are ignored tests (`assets/geo/README.md`).
- `basemap.rs` — the built-in world map (Natural Earth land, lakes, borders in `assets/geo/basemap.bin`), triangulated once with `earcut` on a background thread (`basemap::get`), drawn with the `map_*` theme tokens.
- `online.rs` — online maps, only when `Store.online_maps == Some(true)` (asked once at the first start, `Dialog::OnlineMaps`): OpenStreetMap tiles (2 download threads, only tiles in view, disk cache `cache_dir()/AlbumCraft/tiles` fresh 14 days, 500 MB cap, textures LRU) and Nominatim search / reverse lookup (≥ 1 s apart, on demand only). `ureq` with rustls. `cargo test online_services -- --ignored` talks to the real servers.
- `map_view.rs` — `View::Map`: the Places map (clustered photo markers, selection strip, search box with built-in and online results) and "Set location" mode (`App::start_placing`: search or click to drop a pin, drag it, name it, apply / remove). `GeoState` (in `App.geo`) holds the GPS index results (`file_gps`, keyed by path), the cached located list and the map state. `Viewport` does the screen ↔ world math and repeats the world across the date line.
- `commands.rs` — the command layer: 26 named commands with JSON-schema parameters (search, also by `has_location` / `near`, flag, rating, caption, tags/people, albums, add folder, edit steps, auto fixes, undo, copy edits, `render_preview` returning a PNG, export (with the location in EXIF), web gallery, movie, show in editor, `set_location`, `remove_location`, `find_place`). The UI calls `set_place` / `remove_place` too. They run on a `Host`: the window (`App`, impl at the end of app.rs) or `Headless` (library.json without a window). Pictures are named by key (file path). Slow commands return `Outcome::Later` (a closure run off the UI thread). Bad parameters are `Err`, never panics (`bad_input_is_an_error_not_a_crash`).
- `mcp.rs` — `albumcraft --mcp`: MCP server (JSON-RPC 2.0, one message per line on stdin/stdout; `initialize`, `tools/list`, `tools/call`, `ping`). Each call goes to the open window through `control.rs` when there is one, otherwise to a `Headless` library that is reloaded before and saved after each changing call. Nothing but protocol messages may be written to stdout in this mode.
- `control.rs` — the window's control connection: 127.0.0.1 on a free port, a random 256-bit token, both in `<data dir>/AlbumCraft/control.json` (mode 0600, removed when the window closes); one JSON line per request, 1 MB limit, 4 connections, wrong token closes the connection. The window runs requests in `App::run_control_requests` each frame.
- `exifinfo.rs` — EXIF/camera info and embedded thumbnails (kamadak-exif).
- `print.rs` / `print_view.rs` — print layouts, paper/borders/captions; printing through Win32 GDI on Windows and through CUPS on Linux/macOS (`print.rs` `mod cups`: `lpstat`/`lpoptions` for printers, paper sizes and resolution; the pages go to `lp` as a PDF written by `pdf_of_pages`, JPEG pages placed on the printable area, 5 mm margins). `print::available()` hides every print entry point when neither works (no CUPS scheduler). Tests never send a job: they write the PDF to a file (`ALBUMCRAFT_PDF_OUT=out.pdf cargo test cups_pdf_demo -- --ignored` for a sample).

Key design rules:
- **Originals are never modified.** Edits, stars, captions, tags, people, albums are stored as `Meta` in `library.json` under the user data dir (`dirs::data_dir()/AlbumCraft/library.json`, see `library::store_path`; a library saved under the earlier working name `PhotoShelf/` is copied over on first start). Photos are keyed by file path string.
- **Locations:** `Meta.place` (see `geo.rs`); the file's own GPS is read by the background index, never written. Exports carry the location in EXIF when `Store.export_location`. No network unless `Store.online_maps == Some(true)` (only `online.rs` connects).
- **Culling:** `Meta.flag` (`Flag::None/Pick/Reject`) and `Meta.rating` (0–5). Keys P / X / U / 0–5 act on the tray (library) or the current picture (editor); `library_view::flag_rating_menu` / `flag_rating_keys`. The yes/no `star` of older libraries and `.picasa.ini` `star=yes` become picks (`Meta::upgrade`, on load).
- **Copy / paste edits:** `App::copy_edits` / `paste_edits` (Ctrl+Shift+C / Ctrl+Shift+V; plain Ctrl+C/V reach egui as clipboard events, not keys). Pasting appends the copied ops (red-eye fixes excluded, `copyable_edits`) to each picture's own edits.
- **`.picasa.ini` files are read once per folder and never written** (tracked via `Store.imported_ini`).
- All decoding/rendering goes through the `Library` (`LocalLibrary` → `Pool`); UI code requests via `Job`s (`App.lib.request`) and consumes `Done` results each frame (textures are cached in `App` maps keyed by photo id).

## Other files

- `new_features.md` — feature plan, phases, review findings (phase 0) and status.
- `docs/screenshots/` — README screenshots (JPEG, 1400 px wide); `scratch/` (gitignored) is throwaway.
- Releases: push a tag `v<version>` (matching `Cargo.toml`); `.github/workflows/release.yml` builds Linux x86_64 (tar.gz with `.desktop` file), Windows x86_64 (zip) and macOS arm64 (ad-hoc signed `.app` zip) and publishes a GitHub release (pre-release when the version has a `-`), with notes from `docs/release-notes/<version>.md`. `packaging/` holds the `.desktop` file and `Info.plist`. License: MIT OR Apache-2.0 (`LICENSE-MIT`, `LICENSE-APACHE`, `NOTICE`).
- `assets/app-icon/` — the gecko app icon: `albumcraft.svg` is the hand-made source; PNG sizes, `hicolor/` layout and `.ico` are exported from it (commands in its README). `main.rs` embeds the 64 px (sidebar badge, `App::app_icon`) and 256 px (window icon) PNGs.
