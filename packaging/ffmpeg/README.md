# The FFmpeg libraries

AlbumCraft plays videos through a playback-only build of FFmpeg (decoders and file readers
for camera and phone videos, nothing else) plus dav1d for AV1. `build.sh` builds them, from
checksum-pinned sources, as shared libraries:

| System | Run it from | Result |
|---|---|---|
| Linux | any shell | `.tools/ffmpeg/linux-x86_64/` |
| macOS | Terminal | `.tools/ffmpeg/macos-aarch64/` (Apple silicon) |
| Windows | an MSYS2 **UCRT64** shell | `.tools/ffmpeg/windows-x86_64/` (DLLs in `bin/`) |

`build.sh --archive` also packs the result as
`dist/ffmpeg/albumcraft-ffmpeg-<os>-<arch>.tar.gz` with a `.sha256` file, so it can be built on one
machine and used on another.

Licence: LGPL 2.1 or later (FFmpeg, without its GPL and non-free parts) and BSD (dav1d). The
licence texts are in `share/licenses/`; they go into the packages with the libraries.

## Building on a Mac (for someone without one)

You need a Mac with Apple silicon (M1 or later) and about 2 GB of free space. It takes
5–10 minutes. The result works on macOS 13.4 and later, whatever version the Mac building it
runs.

1. Install Apple's command-line tools (compiler, make, git), if they aren't installed yet:

   ```
   xcode-select --install
   ```

2. Install [Homebrew](https://brew.sh) if it isn't installed yet, then the build tools:

   ```
   brew install pkgconf meson ninja
   ```

3. Get AlbumCraft's sources, on the branch with videos:

   ```
   git clone --branch feature/video https://github.com/kbarni/AlbumCraft.git
   cd AlbumCraft
   ```

4. Build and pack:

   ```
   packaging/ffmpeg/build.sh --archive
   ```

   It downloads FFmpeg and dav1d (about 12 MB, checked against pinned checksums), builds them
   and checks the result: that each library finds the others in its own folder, needs nothing
   outside macOS, and is built for macOS 13.4. It ends with:

   ```
   ==> packed: …/AlbumCraft/dist/ffmpeg/albumcraft-ffmpeg-macos-aarch64.tar.gz
   <checksum>  albumcraft-ffmpeg-macos-aarch64.tar.gz
   ```

5. Send back both files from `dist/ffmpeg/`: `albumcraft-ffmpeg-macos-aarch64.tar.gz` and
   `albumcraft-ffmpeg-macos-aarch64.tar.gz.sha256`.

If a step fails, send the whole output of the command that failed. When FFmpeg's step fails,
the end of its configure log is printed too.

Nothing is installed outside the `AlbumCraft` folder, apart from the tools in steps 1 and 2.
To remove everything afterwards, delete the folder (and `brew uninstall pkgconf meson ninja`
if they were installed only for this).

## Using an archive built elsewhere

Check it and unpack it into `.tools/ffmpeg/`:

```
shasum -a 256 -c albumcraft-ffmpeg-macos-aarch64.tar.gz.sha256   # or sha256sum -c
mkdir -p .tools/ffmpeg
tar -xzf albumcraft-ffmpeg-macos-aarch64.tar.gz -C .tools/ffmpeg
```

The pkg-config files name the folder they are in (`${pcfiledir}`), so the folder can be
anywhere. Point cargo at it in `.cargo/config.toml`, one line per target:

```toml
[env]
PKG_CONFIG_PATH_aarch64_apple_darwin = { value = ".tools/ffmpeg/macos-aarch64/lib/pkgconfig", relative = true }
PKG_CONFIG_PATH_x86_64_pc_windows_gnu = { value = ".tools/ffmpeg/windows-x86_64/lib/pkgconfig", relative = true }
PKG_CONFIG_PATH_x86_64_unknown_linux_gnu = { value = ".tools/ffmpeg/linux-x86_64/lib/pkgconfig", relative = true }
```

## How the program finds them

- **Linux**: the libraries look for each other in their own folder (`$ORIGIN`); the program
  looks beside itself, in `../lib/albumcraft` and in the build folder (`rpath.rs`).
- **macOS**: the libraries are named `@rpath/<file>` and look for each other in their own
  folder (`@loader_path`); the app bundle must carry them (e.g. in `Contents/Frameworks`) with
  an rpath to that folder. Not wired into `build.rs` or the `.dmg` yet.
- **Windows**: there is no rpath. The DLLs (`bin/*.dll`) go beside `albumcraft.exe`; to run
  from a Cargo build, put `.tools\ffmpeg\windows-x86_64\bin` on `PATH`. They need only Windows'
  own DLLs (the C runtime comes with Windows 10 and later); gcc's runtime is linked in.
  Building the bindings needs MSYS2's pkg-config and libclang
  (`pacman -S mingw-w64-ucrt-x86_64-pkgconf mingw-w64-ucrt-x86_64-clang`), e.g. in the local
  `.cargo/config.toml`'s `[env]`: `PKG_CONFIG = "C:/msys64/ucrt64/bin/pkg-config.exe"` and
  `LIBCLANG_PATH = "C:/msys64/ucrt64/bin"`.
