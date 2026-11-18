// The FFmpeg libraries (feature `video`) are shared libraries: the test programs must find them
// when they run. See `ffmpeg_rpath` in the app's build.rs, which does the same for the program.
fn main() {
    ffmpeg_rpath("cargo:rustc-link-arg");
}

include!("../../packaging/ffmpeg/rpath.rs");
