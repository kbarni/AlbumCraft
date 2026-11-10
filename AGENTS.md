# AGENTS.md: guide for AI agents and contributors

AlbumCraft is a native photo organizer with a simple non-destructive editor, written in **Rust only** as a single egui/eframe desktop app. **No Tauri, Electron or webview shells**, and no JavaScript/TypeScript UI. The product name is always written **AlbumCraft** (`{Function}Craft` in PascalCase, like its siblings in the *Craft* suite: PhotoCraft, ArtCraft, DesignCraft, FilmCraft…) in user-facing text: UI, window titles, About, docs prose. Machine names stay lowercase (`albumcraft` crate and binary, `ALBUMCRAFT_*` environment variables); the data and cache directories are named `AlbumCraft`. The goal is a photo organizer that is simple to use and fast, plus the features planned in `new_features.md`, with every feature eventually drivable by agents (MCP). Read this file first, then `CLAUDE.md` (architecture and build details).

## 1. Orientation

| Read | Why |
|---|---|
| `CLAUDE.md` | Layout, build/run/test commands, module map, key design rules |
| `new_features.md` | Feature ideas, feasibility, phases and current status: the work list |
| `docs/screenshots/` | Screenshots used by the README |

## 2. Code map

A Cargo workspace. The browser and a server can't use the window's code, so it is split by what
each part needs:

```text
crates/albumcraft-edit/src/   the edit pipeline only: no files, no threads of its own; builds for wasm32
  imaging.rs       EditOp, Effect, apply_all, sample pictures, copyable_edits
  develop.rs       Adjust sliders and the analysis behind the one-click fixes
  looks.rs         preset looks
  steps.rs         changing an edit list: one adjustment, look, straighten, perspective, crop per picture
  decor.rs         Effects tab: vignette, borders, shadow, text
crates/albumcraft-core/src/   everything without a window (native: desktop app, later the server)
  library.rs       Store (library.json), Meta/Album/Folder/Photo, .picasa.ini import, folder scan, background Pool
  catalogue.rs     the catalogue cache: what the last scan found, shown at once at the next start
  imaging.rs       reading picture files (load_source, oriented_dims) on top of the edit pipeline
  commands.rs      the command layer (Host, Headless); mcp.rs and control.rs: the MCP server and the window's control connection
  export.rs        export_one (edited JPEG copies, never overwriting); import.rs: copying from cards / folders
  exifinfo.rs raw.rs geo.rs thumbcache.rs gallery.rs movie.rs
  mail.rs          Email: small copies attached to a new message in the mail program (Windows: Simple MAPI, `unsafe`)
  online.rs        online map services (tiles on disk, place search), only when allowed
  print.rs         print layouts and page rendering, the PDF writer; slides.rs: slideshow settings and motion
  collage.rs       collage styles and contact sheets: layouts, rendering, PDF / picture files
src/                          the egui app (binary `albumcraft`)
  main.rs          entry point, --screenshot, --mcp; uses the core modules under their old names (crate::library, …)
  app.rs           App state + eframe update loop, screenshot capture
  dialogs.rs       the dialogs and the toast; demo.rs: the demo state behind --screenshot
  backend.rs       the Library interface: how App reads pictures and saves notes (LocalLibrary today)
  shell.rs         library UI (top bar, sidebar, justified grid, selection bar, status bar)
  library_view.rs  shared library helpers, menus, context menu, keys
  edit_view.rs     editor UI, compare modes; slideshow_view.rs: the full-screen slideshow
  map_view.rs      Places map and "Set location" mode; basemap.rs: built-in world map
  search_box.rs    the library's search box and its suggestions
  info_panel.rs    editor's info panel (properties, EXIF, location map, tags)
  import_view.rs   Import view (copy from a card, camera or folder into a new library folder)
  online.rs        map tiles as textures (adds to core's online.rs)
  print.rs         printers: Win32 GDI (`unsafe`, Windows only) and CUPS (adds to core's print.rs)
  print_view.rs    print UI
  collage_view.rs  collage and contact sheet screen
  slides.rs        slideshow music (adds to core's slides.rs)
  theme.rs         light/dark tokens and egui style
```

## 3. Golden rules

### Never crash, never lose the user's data (outranks feature work)

People trust AlbumCraft with years of stars, captions, tags and edits. A malformed picture, odd EXIF, a corrupt `library.json`, a full disk or a quick sequence of clicks must give a message the user can act on, never a panic and never silent data loss.

- **New non-test code never panics.** No `unwrap()`, `expect()`, `panic!`, `unreachable!`, `todo!`, `unimplemented!`. Use `?`, `let .. else`, `if let`, `get()`, or `unwrap_or*` where the fallback is truly correct. The only exception is a provably infallible literal, with `.expect("why it can't fail")`. Existing unwraps are being removed (see phase 0 in `new_features.md`); don't add more.
- **Input is hostile.** Picture files, EXIF, `.picasa.ini`, `library.json`, file names (including non-UTF-8 ones) and, later, MCP parameters. Index with `get()`, slice strings only at char boundaries (`String::truncate` panics otherwise), use `checked_*`/`saturating_*` on sizes and offsets, guard division by zero and NaN, and cap allocations sized by input.
- **Photo ids are indexes into `App.photos` and change on every rescan.** Anything that keeps ids across frames (dialogs, tray, edit list, print and collage state) must be moved to the new ids in `App::keep_across_scan` (by file path; state of pictures that are gone is dropped), or looked up with `photos.get(id)`. Answers from background jobs are matched by key, not id. Persistent references use the photo key (file path).
- **Background threads must survive a bad file.** Worker jobs run under `catch_unwind`, locks use `lock().unwrap_or_else(PoisonError::into_inner)`, and every job sends a `Done` (with `None` on failure) so the UI never waits forever.
- **Persistence is atomic and conservative.** Write to a temporary file and rename. If `library.json` can't be read or parsed, keep the file (move it aside as a backup), tell the user, and never overwrite it with an empty store.
- **Prove it.** Every crash or data-loss fix comes with a small synthetic regression test that failed before the fix.

### Product rules

1. **Originals are never modified.** Edits, stars, captions, tags, people, albums and locations live in `library.json` (`Store`/`Meta`), keyed by file path. Export writes new files and never overwrites an existing one.
1b. **No network unless the user allowed it.** Only `online.rs` connects, and only when `Store.online_maps == Some(true)`; everything must keep working offline (built-in map and city list).
2. **`.picasa.ini` files are read once per folder and never written** (`Store.imported_ini`).
3. **Clean-room.** Other programs may be studied for behaviour and look only; never copy their code, assets or texts, and don't name them in the docs or user-facing text. Supported foreign file formats (such as `.picasa.ini`) are described as file formats. Third-party assets must be permissively licensed, with their license next to them; demo images are generated (`imaging::sample_image`) or public domain, never personal photos.
4. **Keep it simple, intuitive and fast.** Avoid bloat; features that help organize photos are welcome when they stay easy to use. One obvious way to do each thing; no setting that does nothing. Decoding and rendering go through the `Pool`; the UI thread never decodes pictures or blocks on slow I/O. Edits are computed on the 1400 px preview and at full size only for zoom/export. Don't walk the whole library and allocate per photo every frame: cache derived lists (groups, search results, people) and rebuild them when their inputs change. Check heavy operations on a 24 MP picture in release.
5. **Saved data stays compatible.** `Store`, `Meta` and `EditOp` are serde types in `library.json`. New fields get `#[serde(default)]`; existing `EditOp` variants keep deserializing (add new variants, don't rename old ones). Changing `THUMB_DIM` or the thumbnail pipeline is fine: the cache key includes the dimension.
6. **Colours come from `theme::t()` tokens**, never hard-coded in views (pictures, paper and overlays drawn on pictures excepted).
6b. **Icons come from `src/icons.rs`** (Material Symbols, one font for every platform), never emoji or Unicode symbols. Need a new one? Add it there and rebuild the font subset (`assets/fonts/README.md`).
7. **Verify UI changes visually.** `cargo run -- --screenshot out.png` renders one frame offscreen and exits; `ALBUMCRAFT_SHOT_VIEW=edit|presets|effects|slideshow|movie|print|compare|props|zoom|tray` and `ALBUMCRAFT_THEME=light|dark` pick what is shown. Look at the PNG.
8. **Everything is a command.** New user-visible actions get a command in `src/commands.rs` (name, description, JSON schema, `run` arm working on `Host`, test), so agents can use them through the MCP server (`albumcraft --mcp`). Commands take pictures by key, validate every parameter and return `Err` for bad input; slow work goes in `Outcome::Later`. Prefer having the UI call the same code. (Many older UI actions still change state directly; move them over when touching them.)
9. **Platform code is isolated.** `unsafe` exists only in `print.rs` (printing) and core's `mail.rs` (Simple MAPI for Email), both under `#[cfg(windows)]`; every block keeps its invariants obvious and gets a `SAFETY:` comment when touched. Don't add `unsafe` elsewhere. Platform-specific behaviour (file manager, printing) needs a working or clearly reported fallback on Linux and macOS.
10. **The window reads pictures and notes only through its `Library`** (`src/backend.rs`): scans, thumbnails and previews, picture sizes, EXIF, GPS, `.picasa.ini`, saving the notes, export / gallery / movie / collage. Don't call `load_source`, `std::fs` or `Store::load` from `App` or the views; add a method to `Library` (implemented in `LocalLibrary`) instead, so a library on a server can stand in later. Code that must also run in a browser (the edit pipeline) goes in `albumcraft-edit`, which must keep building with `cargo build -p albumcraft-edit --target wasm32-unknown-unknown`.

## 4. Picking work

1. `new_features.md` › Phases and Status: take the current phase.
2. Phase 0 (bugs, robustness, security) comes before new features.
3. When something lands, update its Status line in `new_features.md` (with the date).

## 5. Before you finish a task

Run from the repository root (CI runs the first two on Linux, Windows and macOS):

```sh
cargo build --locked --all-targets
cargo test --locked
cargo clippy --all-targets     # no new warnings in code you touched
cargo run -- --screenshot out.png   # if you changed the UI: look at it
```

Commit in small steps with a message that says what changed and why. Keep the tree building at every commit.

## 6. Parallel agents

- Use your own target dir (`CARGO_TARGET_DIR=target/agent-<name>`) to avoid the Cargo build lock.
- `app.rs` is shared by almost everything: keep edits there small and surgical, re-read before editing, and put new features in a new module.
- If someone else's in-progress edit breaks the build, wait and retry; don't fix their files.
- Delete your `target/agent-*` dir when done.
