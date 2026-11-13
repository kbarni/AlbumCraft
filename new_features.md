Features to add to AlbumCraft:

## Importing

- Basic raw file support: use the included jpg file.

## Processing

Reorganize the whole processing part in the following way:
- Edit tab with two interfaces:
  - Basic editing: one-click effects:
    - *I feel lucky* - apply all auto settings
    - Auto straighten
    - Auto exposure
    - Auto white balance
    - etc...
  - Advanced: controls (mostly sliders) for the following settings (based on Bibble or Aftershot Pro basic settings, see screenshots in the `attachments` folder)
    - Color temperature preset (combo box)
    - Pick neutral color
    - Color temperature slider
    - Exposure
    - Shadows
    - Highlights
    - Blacks
    - Contrast
    - Saturation
    - Vibrance
    - Sharpen / clarity
    - Noise reduction
- Presets tab: propose several effects, check what other similar apps propose and try to replicate it (like Velvia colors, Polaroid, dramatic B&W, soft focus, etc). Some effects can have an intensity slider or other setting.
- Effects tab: frames, text overlay, drop shadow, vignetting etc...

These processing functions should be reimplemented. Normally there's nothing complicated.

## Organizing

- More organizing options: pick, reject, 1-5 stars...
- In a later phase: Face recognition: first extract faces with face detector (maybe Haar cascades or other methods) then use a face recognizer
- Automatic tag suggestions: a small neural network run on the thumbnail proposes the 5 most probable tags; the user clicks one to add it, or searches for other tags (see "Tag suggestions" below).

## General optimization

Think about the possibility to add gpu acceleration. This is only an open question, to implement only if needed (app not fast enough) and in a later phase.

## Exporting

- HTML Gallery: lightbox style webpage, selectable style.
- Order prints: This will probably be a larger project, for a later phase. Propose online printing services (if possible with affiliate programs - for example Ceve) and propose ordering prints and other products.
- Slideshows: more settings: crossfade between the images, more dynamism (discrete pan or zoom on the images), auto advance after X seconds, play music during slideshow.
- Also implement export movie.

## Automation

- MCP server, in line with the *Craft* apps suite: expose most of the functionality (library queries, ratings/tags/captions, edits and presets, export, slideshow/gallery/movie export, screenshots) as tools an agent can call. To do it cleanly, user actions should first go through a small command layer (id + params → result) shared by the UI and the MCP server, rather than the UI mutating `App` state directly.

---

# Assessment and plan (2026-11-04)

Guiding rule: keep the app simple to use and fast. Heavy work stays in the background `Pool`; edits are computed on the 1400 px preview and at full size only for zoom/export.

## Feasibility

| Feature | Feasible | Effort | Notes |
|---|---|---|---|
| Raw via embedded JPEG | Yes | S | CR2/NEF/ARW/DNG/ORF/RW2 are TIFF-based and embed a full-size JPEG preview (find it via EXIF tags or the largest embedded JPEG). CR3 (ISO-BMFF) needs a small separate parser. Limit: edits work on 8-bit data, little headroom. Real demosaicing can come later. |
| Edit › Basic (one-click) | Yes | S | `Lucky`, `AutoContrast`, `AutoColor`, `Straighten` already exist. Auto straighten = edge detection + Hough transform on the dominant lines; can guess wrong, so keep it one undo away. |
| Edit › Advanced sliders | Yes | M | One `Adjust { temp, tint, exposure, shadows, highlights, blacks, contrast, saturation, vibrance, clarity, sharpen, noise }` op replacing `Tune`. Most sliders = one LUT/per-pixel pass; clarity/sharpen = blur-based; noise reduction is the costly one (preview res live, full res on export). Old `Tune`/`EditOp` variants must keep deserializing from `library.json` and `.picasa.ini`. |
| Presets tab | Yes | S | A preset = saved `Adjust` values + effects; intensity = blend with the original. Velvia, Portra, Polaroid, cross-process, Lomo, dramatic B&W, faded/matte, teal & orange, high-key, soft focus. User-saved presets for free. |
| Effects tab | Yes | S–M | Vignette, frames/borders, drop shadow, rounded corners: easy. Text overlay needs on-picture placement UI; `ab_glyph` already renders text. |
| Pick / reject / 1–5 stars | Yes | S | `Meta.star: bool` → `rating: u8` + pick/reject flag, with migration of old data (`star=yes` in `.picasa.ini`). Lightroom keys: `P`, `X`, `0`–`5`. |
| Face recognition | Later | L | Not Haar cascades (dated, poor recall). Small ONNX models (YuNet detection, SFace embeddings, a few MB) via a pure-Rust runtime (`tract`), then cluster embeddings. Runs in the background `Pool`. |
| Tag suggestions | Yes | M | Small model via `tract` (ONNX) on the 256 px thumbnail, once per picture in the background, results cached. Shares the runtime with face recognition. Model choice still open: see "Tag suggestions" below. |
| GPU acceleration | Only if measured slow | — | egui already draws on the GPU; edits are CPU on the preview, with rayon in `image`. Measure first; wgpu compute shaders are possible later. |
| HTML gallery | Yes | S | Resized JPEGs + one HTML file with inline CSS/JS lightbox; 2–3 styles (grid, masonry, dark). No new dependency. |
| Order prints | Mostly not code | — | Labs (CEWE etc.) rarely offer a public ordering API for desktop apps; affiliate programs usually pay for web links (terms to verify). Simple version: "Export for print" (crop to print ratio, size for the DPI) + open the lab's upload page. |
| Slideshow upgrades | Yes | S–M | Crossfade, Ken Burns pan/zoom, auto-advance: easy in egui. Music needs `rodio` (new dependency). |
| Movie export | Yes | M | Render slideshow frames off-screen and pipe them to `ffmpeg` when it is on the PATH (option hidden otherwise). Pure-Rust encoders are slow (AV1) or carry licensing strings (H.264). |
| MCP server | Yes | M | Needs the command layer first (see Automation). Stdio MCP for agents; any TCP control channel must require a token and bind to localhost. |

## Other missing features (small)

1. Copy/paste edits; batch-apply a preset to the selection.
2. Full-screen culling mode: arrows + `P`/`X`/stars.
3. Histogram in the Advanced panel, with clipping warning.
4. Folder watching (new/removed files picked up automatically).
5. Duplicate finder (perceptual hash from the cached thumbnails).
6. Optional XMP sidecars for ratings/keywords/edits (interop with Lightroom/darktable; originals stay untouched).
7. Housekeeping: prune the thumbnail cache; keep metadata when files are moved/renamed (match on size + capture date instead of path only).

## Phases

0. Code review and fixes (bugs, robustness, security) before new features.
1. Ratings / pick / reject, copy/paste edits, raw previews.
2. New Edit tab (Basic + Advanced with histogram), then Presets and Effects.
3. Slideshow upgrades, HTML gallery, then movie export.
4. Command layer + MCP server.
5. Later: tag suggestions, then face recognition on the same model runtime; GPU (only if measured slow), print ordering.

## AI features (2026-11-10; phase A done 2026-11-11)

Faces grouped by person, search by describing a picture, tag proposals; all computed on the user's computer, with networks downloaded only when the user asks. This replaces the runtime and model choices of "Tag suggestions" below (its interface notes stay valid).

| Phase | Content | Status |
|---|---|---|
| A | `albumcraft-ai` crate with ONNX Runtime (`ort`, static), download of packs of networks, background analysis queue, analysis files | done 2026-11-11 |
| B | Picture vectors with the Light pack (OpenCLIP ViT-B/32: MobileCLIP's weights are research-only); "Looks like" search; similar pictures; MCP commands (`semantic_search`, `similar_pictures`); the Multilingual pack (SigLIP 2 B/32-256) and the choice between them | code done 2026-11-12; packs published on release `model-1` |
| C | Tag proposals (the user's own tags, a built-in vocabulary, per-tag classifiers) | done 2026-11-12 (vocabulary of 218 words, its vectors made on the user's computer) |
| D | Faces: detection (YuNet, bundled), automatic grouping (SFace), People view, names in search; then corrections and import of existing face regions | |
| E | Stacks and near-duplicates, events, face-aware framing, clutter filter | |
| F | Text in pictures, captions on demand, subject mask, retouch | |

## Tag suggestions (planned, 2026-11-07)

Suggest tags for a picture; never add them without the user's click.

- **Runtime:** a pure-Rust ONNX runtime (`tract`), shared with face recognition later. Jobs run in the background `Pool` at normal (low) priority on the cached 256 px thumbnail (models take ~224 px), about 10–30 ms per picture on the CPU. Results (scores or an embedding) are cached on disk next to the thumbnails, keyed like them, so opening a picture shows the suggestions at once.
- **Model, two options to compare:**
  1. An image classifier (MobileNet class, 5–15 MB). ImageNet's 1000 classes fit personal photos poorly (dog breeds, "tench"…), so it needs a curated map to a few hundred friendly tags; a scene model (Places365: beach, mountain, kitchen…) fits photos better and can be combined.
  2. A small image–text model (CLIP style, ~40–100 MB): compares the picture with any word, so it can rank the user's own library tags as well as a built-in list (personal suggestions), and the same embeddings allow searching untagged pictures by words ("snow"). Preferred if a small model with a permissive licence exists.
- **Constraints:** no network unless allowed, so the model ships with the app (size cost) or is downloaded only after the user agrees; permissive licence for the weights and acceptable training data (clean-room rule 3), licence file next to the model.
- **Interface:** in the info panel's Tags section, "Suggested: beach · sea · sunset" as dimmed chips under the add field, one click to add; at most 5, and only above a confidence threshold. The existing add field with library-tag suggestions stays.
- **First step:** a research pass (candidate models: size, speed, licence, quality) and a test on real holiday pictures with an ignored `perf_`-style test, before any UI.

## Phase 0: review findings (2026-11-04)

Critical / high:
1. Unreadable `library.json` (corrupt, truncated, or written by a newer version) is treated as a first run, and the empty store overwrites it within 3 s: all stars, captions, tags, albums and edits are lost (`app.rs` `Store::load().unwrap_or_else`). Keep a backup and don't save over it.
2. `Store::save` writes in place, not atomically, and ignores errors: a crash or full disk during save truncates the library, which then triggers 1.
3. EXIF orientation is never applied when loading (`imaging::load_source`): portrait phone/camera pictures show sideways in grid, editor, export and print.
4. A panic in a `Pool` worker (decoder or edit) kills the thread and poisons the queue mutex (`lock().unwrap()`), which then kills every other worker; thumbnails stop loading and the pending sets never clear. Same for export/collage threads (no message).
5. `exifinfo::read` truncates long values with `String::truncate(117)`, which panics on a non-ASCII value when byte 117 isn't a char boundary: one picture with such metadata crashes the app when its properties are shown.

Medium:
6. Photo ids kept across a rescan (`Dialog::ClearTray`, `Dialog::PrintReview`, `PendingLeave::Photo`, print state) can index `photos[id]` out of bounds or point at the wrong picture.
7. "Show in file explorer" runs `explorer` on every platform: does nothing on Linux/macOS.
8. Export: `File::create` after an `exists()` check can still overwrite a file created in between (use `create_new`); failures aren't reported per file and leave partial files; EXIF (date, camera) is not carried over.
9. A `Done::Zoom` result for an old request clears `zoom_req` of a newer one; `Done::Message` replaces whatever dialog is open.
10. `fmt_time` shows UTC, not local time.
11. Non-UTF-8 file names: keys are `to_string_lossy`, so such pictures can't be loaded.
12. Folder scan silently stops at 20,000 pictures.

Performance:
13. `groups()` / `visible_flat()` / `people()` run several times per frame over the whole library, lowercasing the search and formatting a string per photo: slow with large libraries. Cache them per (source, search, filter, library revision).
14. `library.json` is fully re-serialized on the UI thread every 3 s while dirty.
15. Thumbnails decode the full-size image first (now cached on disk, but the first scan is slow).

Low / clean-up:
16. `print.rs` casts a `Vec<u8>` buffer to `PRINTER_INFO_4W` (alignment not guaranteed); add `SAFETY:` comments to the Win32 blocks.
17. No panic hook in release (`windows_subsystem = "windows"`): a crash disappears without a message; save the library from the hook.
18. Values read from `library.json` (`thumb_size`, print `copies`) aren't clamped on load.
19. Thumbnail cache is never pruned; the cache test leaves one file in the real cache dir.
20. 27 clippy warnings (mostly collapsible `if`).

## Status

- Done: persistent on-disk thumbnail cache (`src/thumbcache.rs`, `dirs::cache_dir()/AlbumCraft/thumbs`).
- Already present before this plan: export of edited copies (size/quality), basic slideshow, printing, one-click edit tools and 12 effects.
- Phase 0 (branch `fix/phase0`, 2026-11-04): fixed 1–7, 10, 12, 13, 16–20; 8 fixed except carrying EXIF over to exported files; 9: the zoom part was not a bug (only one zoom render is requested at a time), the dialog part is fixed.
- Phase 0 left open: 8 (EXIF in exported files, needs a metadata writer), 11 (non-UTF-8 file names: keys would have to become `PathBuf`), 14 (saving off the UI thread), 15 (faster first thumbnails). None loses data; 14 and 15 only matter for very large libraries.
- Not in the list but noticed: the tray buttons overlap in a narrow window. (The placeholder buttons for defunct services were removed on 2026-11-04: Upload became the web gallery, Movie was implemented, Shop, BlogThis! and Geo-Tag are gone; printing is hidden where it isn't implemented, i.e. outside Windows.)
- Phase 1 (2026-11-04), one branch per feature, stacked: `feature/ratings` (pick/reject flags, 1–5 star ratings, filters, P/X/U/0–5 keys; old stars and `.picasa.ini` stars become picks), `feature/copy-edits` (copy/paste edits, Ctrl+Shift+C/V), `feature/raw-previews` (raw files via their embedded JPEG preview; checked on a real Nikon D3S NEF: 4256×2832 preview in 0.1 s).
- Phase 1 left open: raw+JPEG pairs of the same shot show as two pictures; CR3/RAF/other formats were only tested with synthetic files.
- Phase 2 (2026-11-04), stacked on phase 1: `feature/edit-tab` (Edit tab, Basic: I'm feeling lucky, auto exposure, auto colour, auto contrast, auto straighten, crop/straighten/red-eye/pick neutral/rotate; Advanced: white-balance presets and picker, temperature, tint, exposure, highlights, shadows, blacks, contrast, saturation, vibrance, clarity, sharpen, noise reduction; histogram clipping warning), `feature/presets` (17 looks with intensity), `feature/effects` (vignette, border, instant frame, drop shadow, rounded corners, text).
- Phase 2 left open: text can't be dragged on the picture (9 positions); only one font (the UI's); no clipping overlay on the picture itself (the histogram shows the share); the old one-click effects Sharpen, Tint, Saturation, Filtered B & W and Graduated Tint are no longer offered (sliders and presets cover them; old edits still render).
- Phase 3 (2026-11-04), stacked on phase 2: `feature/slideshow` (crossfade, pan and zoom, 2–15 s per picture, looping music via rodio; settings saved), `feature/html-gallery` (self-contained web gallery in 3 styles with a lightbox; replaces the defunct Upload button), `feature/movie-export` (720p/1080p MP4 through FFmpeg with the slideshow's effects and music).
- Phase 3 left open: no per-picture duration or transition choice beyond crossfade; movie export needs FFmpeg installed separately (not bundled); the gallery has no upload step (copy the folder to any web host); music playback is untested on macOS/Windows here.
- Phase 4 (2026-11-04), stacked on phase 3: `feature/commands` (23 commands with JSON schemas over a `Host`: the window or the library without one), `feature/mcp` (`albumcraft --mcp` stdio MCP server; talks to the open window through a token-protected localhost connection, otherwise edits library.json itself). Checked end to end with the real binary, with and without the window.
- Phase 4 left open: no screenshot tool for the window; most UI actions still change state directly instead of calling the commands; no slideshow control over MCP.
- Printing on Linux and macOS through CUPS (2026-11-04, `feature/cups-printing`): printers, default printer, paper sizes and resolution from `lpstat`/`lpoptions`, paper choice in the print screen, pages sent to `lp` as a PDF. Checked: printer list and options on a real CUPS queue, PDF validity with pdfinfo/pdftoppm; no real job was sent. Left open: the exact unprintable margins (5 mm assumed), duplex and other printer options.
- 2026-11-04: gecko app icon, README, MIT OR Apache-2.0 license, release workflow; version 0.1.0-alpha.1.
- 2026-11-05: phase 0 item 14 done: the library is saved as compact JSON, written on a background thread.
- Geo-tagging and Places map (2026-11-05, `src/geo.rs`, `basemap.rs`, `online.rs`, `map_view.rs`): GPS read from the pictures by a background index (cached in `gps.json`); a place set by the user (`Meta.place`) overrides it or hides it; Places map with clustered photo markers (sidebar, View menu, editor Properties › Show on map); "Set location" mode (tray button, context menu, Picture menu, Properties) with offline city search (GeoNames, 34,000 places, typed coordinates too), click or drag a pin, editable place name; built-in world map (Natural Earth) always, OpenStreetMap tiles and address search only after the user agrees (asked at the first start, toggle on the map and in Tools); exports write the location into the copy's EXIF (checkbox in the Export window; this also covers the GPS part of phase 0 item 8); library search matches place names; commands `set_location`, `remove_location`, `find_place`, and `search_photos` filters `has_location` / `near`. Checked against the real OpenStreetMap servers once (ignored test `online_services`).
- Geo-tagging left open: the original files are never written (by design); no drag-and-drop of pictures onto the map; the built-in map stops at 1:50 million detail (zoom 13); the map follows a folder, album or the tray when asked (2026-11-07), not the search or filter; pictures near the date line are fine but clusters don't merge across it.
- 2026-11-05, from `plan/ideas.md`: in the editor, click and drag on the fitted picture zooms smoothly to 100 % around that point and pans; grid tiles show a reject button (next to pick) and five clickable stars on hover, and the photo tray has reject and rating next to pick; the "Edits on this picture" list keeps a height for up to six steps (it could shrink to one line); Advanced tab › Geometry: straighten slider (edits the picture's straighten step in place) and vertical / horizontal perspective (`EditOp::Perspective`, a keystone correction zoomed to hide empty corners).
- Left open from those: the perspective correction is a simple keystone (rows or columns scaled linearly), not a full projective transform; no guide lines are drawn while moving the geometry sliders.
- 2026-11-06: code split into a Cargo workspace (albumcraft-edit, albumcraft-core, the app) behind a `Library` interface; version 0.2.0.
- 2026-11-06: release packages: AppImage, .deb, Flatpak (from the .deb) and a Windows MSI; version 0.2.1.
- 2026-11-07: one icon set: every UI icon is a Material Symbols character from `src/icons.rs`, drawn from a bundled 8 KB font subset (`assets/fonts/`), instead of a mix of Unicode symbols and each system's colour emoji; the glyph test now rejects emoji in UI text.
- 2026-11-07: importing is separate from adding a folder. Import (top bar, File menu, empty library) opens the import view: pick a memory card / camera (detected) or any folder, pictures grouped by time with group checkboxes, the ones already in the library hidden (same content, even renamed), a preview, then copy into a new named folder of the library (`import.rs`: never overwrites, skips identical files, keeps file dates; background copy with progress and Stop in the status bar; the new folder opens when done). Folders (top bar) opens the folder manager, which only scans. Commands `import_pictures` and `rescan`.
- Import left open: no deleting from the card after copying (by design for now); no video files (the library has none); duplicates are found against the library only, not against the destination's other folders outside it.
- 2026-11-07, performance (measured on 1,089 12 MP phone JPEGs on a USB disk, and the library of 2,208 pictures with 175 RW2): the library hung for 1–7 s per frame because the grid read up to 120 picture headers per frame on the window thread (each a whole-file read); now `Job::Dims` in the background. Thumbnails from the camera's embedded preview (3 ms instead of 108 ms, ~100 KB read instead of the whole file; checked against full decodes); 1400 px previews decoded at half scale (167 → 128 ms); raw files: embedded JPEGs found by header (thumbnail 71 → 27 ms); import list dates from the header (0.28 → 0.01 s for 203 pictures). Two pool queues (urgent, newest first); results handled at most 8 ms per frame; at most 1200 thumbnails kept. Nested library folders were scanned twice (pictures shown twice): the scan skips a folder inside another, and adding one is refused or replaces the inner ones (`Store::add_root`).
- 2026-11-07: the editor's Properties pane became an info panel with foldable sections (remembered): Properties, Extended EXIF, Location (small map with the pin, Modify… / Show on map) and Tags (chips, add with suggestions from the library). Tags moved there from the editor's bottom bar; people stay in the bottom bar until face recognition. Shown by default; Info button or I.
- 2026-11-07: one search box for everything: typed text also matches places (the name set, or the nearest city, region and country of the GPS position, with no name typed); suggestions under the box for tags, places, people (chosen ones become chips that match exactly and combine), albums and folders (open them), with picture counts; keyboard ↑/↓/Enter/Esc/Backspace. `geo::nearest` uses a latitude index (was a scan of 34,000 cities per call). `search_photos` gets `tag` and `place`.
- 2026-11-07: a folder, an album or the photo tray on the map: map button on folder headers, "Show on map" in an album's menu; Places then shows only those pictures (title "Places · <name>", "n of m with a location", "Show the whole library"). Command `show_on_map`. In the tray, the small button next to Tags is Set location; the big Location button on the right is gone.
- 2026-11-07: editor geometry: in the Advanced tab, Geometry comes first (the most used corrections) with Crop… and Remove crop; a square alignment grid shows on the picture while the straighten / perspective sliders move (and in the Straighten tool). The crop tool has more shapes (original, 16 × 9), a landscape / portrait orientation choice, starts with the largest centred box of the chosen shape, can be moved by dragging inside it, and shows thirds lines.
- 2026-11-07: version 0.3.0 (import, unified search with places, info panel, map of a folder or album, editor geometry, one icon set, performance on slow disks).
- 2026-11-10: menu trim done: the ☰ button opens one flat list (16 entries in groups) instead of 8 menus; entries that already had a visible button are gone. Copy / Paste edits moved to Edit with Undo all edits; Scan folders again moved to File. Ctrl+scroll or a pinch changes the picture size (the slider is hidden in narrow windows).
- 2026-11-10: collages and contact sheets (`feature/collage`): the tray's Collage button opens a collage screen with 9 styles (photo pile, mosaic, centre stage, grid, film strip, clothesline, fan, heart / circle / star, multiple exposure), borders (none, white, instant with text), shadows, a colour, gradient or picture background (a picture of the collage or a file, soft focus), page sizes (A4 … 20 x 30 cm, square, 16:9) in landscape or portrait; drag a picture onto another to swap them, or in a pile drag to move and scroll to turn; New layout / Shuffle; Save picture (JPEG / PNG up to 4800 px), Save as PDF, Print. The old grid collage became the contact sheet (a tile of the same screen, and Contact sheet… in the menu): a title (the album's or folder's name) and "37 pictures, April 2006", file names / captions / dates under the pictures, picture size, page numbers, several pages, PDF and print. Commands `make_collage` and `make_contact_sheet`. The PDF writer moved to the core crate (it was CUPS only).
- 2026-11-11: version 0.4.0 (collages and contact sheets, the selection bar instead of the tray with box and keyboard selection, catalogue cache and sort by date taken, editor with one adjustment and one crop per picture and undo per change, Email with attachments, rejected pictures to the trash, the code review's fixes).
- 2026-11-12: version 0.5.0 (interface languages: English, French, German, Spanish, Italian, Portuguese (Brazil), with dates, numbers and sizes in the language; the Settings dialog; the Export view with signature, creator and copyright; the full-screen viewer; manual slideshow advance; compact bar or tray; smoother library scrolling; Email attachment names for Thunderbird).
- Collage left open: no free resizing of a picture in the pile; text only on instant frames (no title text on collages); no per-picture crop inside a mosaic cell (always the middle); the preview uses thumbnails (sharp previews when there are at most 6 pictures).
- Next: narrow windows (below ~1000 px the photo tray and the editor's bottom bar overlap, the search box covers the title); crop corner handles; "Remove crop" that Undo can take back. Later: phase 5 (tag suggestions, face recognition, GPU only if measured slow, print ordering).
- 2026-11-11, AI features phase A (`feature/ai-infrastructure`): new crate `albumcraft-ai` (ONNX Runtime through `ort` 2.0.0-rc.15, linked statically, Cargo feature `onnx`; packs of networks listed in a built-in `models.json` and downloaded from the project's releases, resumable, size and SHA-256 checked; results in files of fixed-size records under `<data dir>/AlbumCraft/analysis/`, keyed by path, date and size, repaired after a cut-off write); a background queue in the `Pool` (runs only when nothing else waits, at most two workers) and `analysis::Analysis` feeding it; "Analysing pictures n / m" with Pause in the status bar; Settings › AI features lists the packs with size, licence, Download / Remove; commands `ai_status`, `download_model_pack`, `remove_model_pack`, `pause_analysis`. Checked with a 100-byte network written by hand in the tests (runs on every CI system) and a local web server for the downloads.
- AI phase A left open: no network is published yet, so `models.json` is empty, nothing is analysed and the first-start question is not asked (it comes with the first pack, phase B). **The prebuilt ONNX Runtime needs glibc 2.38 / GCC 13's libstdc++**: the CI gate (Ubuntu 24.04) is fine, but the release's Linux job on Ubuntu 22.04 can no longer link it; to decide before the next release: build the Linux packages on 24.04 (they then need Ubuntu 24.04 / Debian 13 or newer; the Flatpak is not concerned) or build ONNX Runtime from source on 22.04. To check on the first CI run: the Windows build (the library may need the Visual C++ runtime `msvcp140.dll` on the user's computer) and macOS. Not built: sizes of the analysis files in Settings, removing results of pictures that left the library, matching results when files move.

- 2026-11-13, context menus and folder captions (`feature/context-menus`): a right-click on a folder (sidebar row or grid header) gives Select, Play as slideshow, Show on map, Add to album, Export, Caption and description, Open in file manager, Copy path, Refresh thumbnails, Rename, Move to, Remove from library (with Undo; files untouched). The picture menu gains Open in default program, Open with (Linux: the programs listed for the file type in the desktop entries; Windows: the system's chooser; macOS: none), Copy path, Rename, Move to folder, Move to the trash. Rename and move change the files on disk (the second exception to "originals are never modified", decided by the user): one rename on the same disk, never over an existing file, Undo moves them back, and every note (meta, albums, folder texts, library and left-out folders, `.picasa.ini` done) follows the new path (`Store::moved`). Folders have a caption (one line, under the name in the sidebar and the grid header) besides the description. Commands: `trash_pictures`, `rename_picture`, `move_pictures`, `rename_folder`, `move_folder`, `set_folder_text`, `remove_folder`.
- Context menus left open: moving across disks is refused (no copy-then-delete); moved pictures are analysed again by the AI features (their vectors are keyed by path); no "Open with" list on macOS.

### Menu trim (done 2026-11-10)

Rule: drop what already has a visible button; keep what exists only in the menu or teaches a keyboard shortcut. From 8 menus (about 35 entries) to one flat list of 16 entries (17 since Contact sheet… was added); the groups below are separated by lines, without menu names.

| Menu | Keep | Remove (already at) |
|---|---|---|
| File | Import pictures…, Add folder to library…, Library folders…, Scan folders again (from Tools), Exit | Export, Web gallery, Print (tray buttons; Ctrl+P stays) |
| Edit | Select all (Ctrl+A), Select none (Esc), **new:** Copy edits (Ctrl+Shift+C), Paste edits (Ctrl+Shift+V), Undo all edits (all three from Picture) | Clear the tray (tray's Clear) |
| Picture | Rotate left (Ctrl+[), Rotate right (Ctrl+]), Pick / Reject / Rating (P, X, 0–5) | Open in editor (double-click), Tags and people, Set location (tray buttons) |
| Tools | Online maps (the first-start question points there) | Folder manager (same as File › Library folders) |
| Help | About… | |
| View (removed) | | Only picks, Hide rejected (top-bar chips), thumbnail size (slider, or Ctrl+scroll / pinch), Slideshow (folder ▶), Places (sidebar) |
| Folder (removed) | | Edit description, Play as slideshow (folder header buttons) |
| Create (removed) | | New album (sidebar +), Collage, Movie (tray buttons) |
