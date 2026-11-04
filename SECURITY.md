# Security Policy

AlbumCraft is a local desktop photo organizer. It reads picture files, their EXIF metadata, `.picasa.ini` files and its own `library.json` from folders the user adds, and it writes exported copies, collages, its library and a thumbnail cache. Treat every file it reads as potentially malicious: pictures and metadata often come from other people, downloads and memory cards.

## Reporting a vulnerability

Please give the maintainers a reasonable opportunity to investigate and fix a problem before publishing exploitable details, proof-of-concept code or weaponized files.

This project does not yet document a private security contact. Contact the maintainers through the repository owner's channels, give only a minimal non-exploitable summary in public, and ask for a private way to share details. Do not open a public issue containing a working exploit, secrets, a sensitive crash dump or a malicious attachment.

## What to include

- AlbumCraft version or commit;
- operating system, architecture and build profile (dev/release);
- the affected component (decoding, EXIF, `.picasa.ini`, library, export, printing, thumbnail cache, MCP server once it exists);
- impact and what an attacker needs (for example "the victim adds a folder containing this file");
- exact reproduction steps, expected and actual behaviour;
- crash output or backtrace (`RUST_BACKTRACE=1`, dev build) and memory/CPU measurements when relevant;
- any known workaround.

Remove personal paths, private pictures and unrelated system information from logs.

## Malicious test files

Don't attach a harmful or non-redistributable file to a public issue. First give its format, size, SHA-256, observed effect, how it was produced (synthetic, fuzzed…) and how to reproduce. Test files accepted into the repository must be minimized, free of personal data (including EXIF GPS, names and serial numbers) and legally redistributable. A fixed parser defect gets a permanent regression test.

## Scope

- picture decoding (JPEG, PNG, GIF, BMP, WebP, TIFF through the `image` crate), including decompression bombs and oversized dimensions;
- EXIF parsing and embedded thumbnails (`kamadak-exif`), and how AlbumCraft formats the values;
- `.picasa.ini` parsing;
- `library.json` loading and saving (corruption, data loss, absurd values);
- the thumbnail cache (`dirs::cache_dir()/AlbumCraft/thumbs`): file naming, overwrite, symlinks;
- folder scanning (recursion, symlinks, very large trees) and export/collage output paths (overwriting, traversal);
- Windows printing (the only `unsafe` code, Win32 GDI in `print.rs`);
- panics or hangs triggered by file content (a crash loses up to a few seconds of unsaved changes);
- later: the MCP server and any control channel, raw-file parsing, face-recognition models, the HTML gallery output (escaping of captions and file names), movie export through `ffmpeg`;
- dependencies and release artifacts.

General bugs without a security impact go through normal issues.

## Current limitations

- There is no network service: AlbumCraft opens no port and has no automation interface yet. When the MCP server is added it must use stdio, or bind to localhost only and require a random bearer token; tools that touch the filesystem must be restricted to the library's folders and the chosen export folder.
- Pictures are decoded with the `image` crate's default limits (allocations up to 512 MiB per decode) on up to six worker threads at once, so a folder of hostile files can use several GiB of memory. There is no per-file timeout.
- A panic while decoding or editing a picture in a background worker (thumbnails, previews, zoom, export, collage) is caught: that picture fails, the others carry on. Every panic is written with a backtrace to `crash.log` in the data directory; a crash of the main thread is reported at the next start.
- `library.json` is written atomically (temporary file, flushed, then renamed). An unreadable library is moved aside as `library.unreadable-<time>.json`, never overwritten; if it can't be moved, nothing is saved during that session. Values in it are range-checked on load.
- File names that are not valid UTF-8 are not supported (such pictures show as unreadable).
- Captions, tags and file names are shown as plain text in egui (no HTML or script interpretation). Exported files contain only picture data (no metadata is copied).
- `.picasa.ini` files are only read, never written. Original pictures are never modified.
