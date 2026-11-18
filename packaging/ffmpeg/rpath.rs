// Included by the build scripts of the app and of the core crate (`include!`).
//
// Where the program finds the FFmpeg libraries (feature `video`) at run time:
// - Linux: beside the program (`.tar.gz`), in `../lib/albumcraft` (`.deb`, AppImage, Flatpak;
//   where libonnxruntime.so goes);
// - macOS: in the app bundle's `Contents/Frameworks` (`../Frameworks` from the program);
// - and, for programs run from the build tree, in the folder pkg-config found them in
//   (packaging/ffmpeg/build.sh's `.tools/ffmpeg/<os>-<arch>/lib`). The macOS packaging removes
//   that one.
// Windows has no rpath: the DLLs go beside the program (packaging/windows/package.ps1).
// `directive` is the cargo instruction: `cargo:rustc-link-arg-bins`, or `cargo:rustc-link-arg`
// for every target including tests.
fn ffmpeg_rpath(directive: &str) {
    let env = |k: &str| std::env::var(k).unwrap_or_default();
    let os = env("CARGO_CFG_TARGET_OS");
    if std::env::var_os("CARGO_FEATURE_VIDEO").is_none() || !matches!(os.as_str(), "linux" | "macos") {
        return;
    }
    let target = env("TARGET").replace('-', "_");
    for k in [format!("PKG_CONFIG_PATH_{target}"), "TARGET_PKG_CONFIG_PATH".into(), "PKG_CONFIG_PATH".into()] {
        println!("cargo:rerun-if-env-changed={k}");
    }
    let mut paths = if os == "linux" {
        vec!["$ORIGIN".to_string(), "$ORIGIN/../lib/albumcraft".to_string()]
    } else {
        vec!["@executable_path/../Frameworks".to_string()]
    };
    let search = [env(&format!("PKG_CONFIG_PATH_{target}")), env("TARGET_PKG_CONFIG_PATH"), env("PKG_CONFIG_PATH")].join(":");
    for dir in search.split(':').filter(|d| !d.is_empty()) {
        let dir = std::path::Path::new(dir);
        if let Some(lib) = dir.parent()
            && dir.join("libavcodec.pc").exists()
        {
            paths.push(lib.display().to_string());
            break;
        }
    }
    if os == "linux" {
        println!("{directive}=-Wl,-rpath,{}", paths.join(":"));
    } else {
        // Apple's linker takes one folder per -rpath
        for path in paths {
            println!("{directive}=-Wl,-rpath,{path}");
        }
    }
}
