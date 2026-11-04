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
5. Later: face recognition, GPU (only if measured slow), print ordering.

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
- Next: phase 5 (later: face recognition, GPU only if measured slow, print ordering).
