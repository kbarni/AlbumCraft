# AGENTS.md: guide for AI agents and contributors

AlbumCraft is a native photo organizer with a simple non-destructive editor, written in **Rust only** as a single egui/eframe desktop app. **No Tauri, Electron or webview shells**, and no JavaScript/TypeScript UI. The product name is always written **AlbumCraft** (`{Function}Craft` in PascalCase, like its siblings in the *Craft* suite: PhotoCraft, ArtCraft, DesignCraft, FilmCraft…) in user-facing text: UI, window titles, About, docs prose. Machine names stay lowercase (`albumcraft` crate and binary, `ALBUMCRAFT_*` environment variables); the data and cache directories are named `AlbumCraft`. The goal is a photo organizer that is simple to use and fast, plus the features planned in `new_features.md`, with every feature eventually drivable by agents (MCP). Read this file first, then `CLAUDE.md` (architecture and build details).

## 1. Orientation

| Read | Why |
|---|---|
| `CLAUDE.md` | Layout, build/run/test commands, module map, key design rules |
| `new_features.md` | Feature ideas, feasibility, phases and current status: the work list |
| `shots/` | Reference screenshots of the app |

## 2. Code map (`src/`)

```text
main.rs          entry point, --screenshot
app.rs           App state + eframe update loop, dialogs, export/collage, screenshot capture
library.rs       Store (library.json), Meta/Album/Folder/Photo, .picasa.ini import, folder scan, background Pool
thumbcache.rs    on-disk thumbnail cache (dirs::cache_dir()/AlbumCraft/thumbs)
imaging.rs       non-destructive edit pipeline (EditOp, Effect, apply_all), load_source, sample images
exifinfo.rs      camera information and embedded thumbnails (kamadak-exif)
shell.rs         library UI (top bar, sidebar, justified grid, tray, status bar)
library_view.rs  shared library helpers, menus, context menu, keys
edit_view.rs     editor UI, compare modes, slideshow
print.rs         print layouts; Win32 GDI printing (the only `unsafe`, Windows only)
print_view.rs    print UI
theme.rs         light/dark tokens and egui style
```

## 3. Golden rules

### Never crash, never lose the user's data (outranks feature work)

People trust AlbumCraft with years of stars, captions, tags and edits. A malformed picture, odd EXIF, a corrupt `library.json`, a full disk or a quick sequence of clicks must give a message the user can act on, never a panic and never silent data loss.

- **New non-test code never panics.** No `unwrap()`, `expect()`, `panic!`, `unreachable!`, `todo!`, `unimplemented!`. Use `?`, `let .. else`, `if let`, `get()`, or `unwrap_or*` where the fallback is truly correct. The only exception is a provably infallible literal, with `.expect("why it can't fail")`. Existing unwraps are being removed (see phase 0 in `new_features.md`); don't add more.
- **Input is hostile.** Picture files, EXIF, `.picasa.ini`, `library.json`, file names (including non-UTF-8 ones) and, later, MCP parameters. Index with `get()`, slice strings only at char boundaries (`String::truncate` panics otherwise), use `checked_*`/`saturating_*` on sizes and offsets, guard division by zero and NaN, and cap allocations sized by input.
- **Photo ids are indexes into `App.photos` and change on every rescan.** Anything that keeps ids across frames (dialogs, tray, edit list, print state) must be cleared in `set_library` or looked up with `photos.get(id)`. Persistent references use the photo key (file path).
- **Background threads must survive a bad file.** Worker jobs run under `catch_unwind`, locks use `lock().unwrap_or_else(PoisonError::into_inner)`, and every job sends a `Done` (with `None` on failure) so the UI never waits forever.
- **Persistence is atomic and conservative.** Write to a temporary file and rename. If `library.json` can't be read or parsed, keep the file (move it aside as a backup), tell the user, and never overwrite it with an empty store.
- **Prove it.** Every crash or data-loss fix comes with a small synthetic regression test that failed before the fix.

### Product rules

1. **Originals are never modified.** Edits, stars, captions, tags, people and albums live in `library.json` (`Store`/`Meta`), keyed by file path. Export writes new files and never overwrites an existing one.
2. **`.picasa.ini` files are read once per folder and never written** (`Store.imported_ini`).
3. **Clean-room.** Other programs may be studied for behaviour and look only; never copy their code, assets or texts, and don't name them in the docs or user-facing text. Supported foreign file formats (such as `.picasa.ini`) are described as file formats. Third-party assets must be permissively licensed, with their license next to them; demo images are generated (`imaging::sample_image`) or public domain, never personal photos.
4. **Keep it simple and fast.** One obvious way to do each thing; no setting that does nothing. Decoding and rendering go through the `Pool`; the UI thread never decodes pictures or blocks on slow I/O. Edits are computed on the 1400 px preview and at full size only for zoom/export. Don't walk the whole library and allocate per photo every frame: cache derived lists (groups, search results, people) and rebuild them when their inputs change. Check heavy operations on a 24 MP picture in release.
5. **Saved data stays compatible.** `Store`, `Meta` and `EditOp` are serde types in `library.json`. New fields get `#[serde(default)]`; existing `EditOp` variants keep deserializing (add new variants, don't rename old ones). Changing `THUMB_DIM` or the thumbnail pipeline is fine: the cache key includes the dimension.
6. **Colours come from `theme::t()` tokens**, never hard-coded in views (pictures, paper and overlays drawn on pictures excepted).
7. **Verify UI changes visually.** `cargo run -- --screenshot out.png` renders one frame offscreen and exits; `ALBUMCRAFT_SHOT_VIEW=edit|print|compare|props|zoom|tray` and `ALBUMCRAFT_THEME=light|dark` pick what is shown. Look at the PNG.
8. **Everything will be a command.** Toward the MCP server: put new user-visible actions in a function on `App` (or a future command module) that takes plain parameters and returns `Result`, and have the UI call it, rather than mutating state inline in UI code. That function is what the MCP tool will call.
9. **Platform code is isolated.** `unsafe` exists only in `print.rs` under `#[cfg(windows)]`; every block keeps its invariants obvious and gets a `SAFETY:` comment when touched. Don't add `unsafe` elsewhere. Platform-specific behaviour (file manager, printing) needs a working or clearly reported fallback on Linux and macOS.

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
