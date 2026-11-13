// On Windows, embed the gecko icon (assets/app-icon/albumcraft.ico) and the version in
// albumcraft.exe, so Explorer, the Start menu shortcut and taskbar pins show it.
fn main() {
    println!("cargo:rerun-if-changed=assets/app-icon/albumcraft.ico");
    apple_availability_check();
    #[cfg(windows)]
    {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("assets/app-icon/albumcraft.ico");
        res.set("ProductName", "AlbumCraft");
        res.set("FileDescription", "AlbumCraft");
        if let Err(e) = res.compile() {
            // ALBUMCRAFT_REQUIRE_ICON=1 (the release build) makes a missing icon an error
            if std::env::var_os("ALBUMCRAFT_REQUIRE_ICON").is_some() {
                eprintln!("could not embed the icon: {e}");
                std::process::exit(1);
            }
            println!("cargo:warning=could not embed the icon: {e}");
        }
    }
}

// ONNX Runtime's CoreML code calls `__isPlatformVersionAtLeast` (what `@available` compiles to).
// Rust's std defines it, but link-time optimisation (the release profile) drops it before the
// C++ objects are linked: link clang's runtime library, which has it too, for the AI builds.
fn apple_availability_check() {
    let macos = std::env::var("CARGO_CFG_TARGET_OS").is_ok_and(|os| os == "macos");
    if !macos || std::env::var_os("CARGO_FEATURE_ONNX").is_none() {
        return;
    }
    let Ok(out) = std::process::Command::new("xcrun").args(["clang", "--print-runtime-dir"]).output() else {
        println!("cargo:warning=xcrun not found: clang's runtime library is not linked");
        return;
    };
    let dir = String::from_utf8_lossy(&out.stdout).trim().to_string();
    let lib = std::path::Path::new(&dir).join("libclang_rt.osx.a");
    if lib.exists() {
        println!("cargo:rustc-link-arg-bins={}", lib.display());
    } else {
        println!("cargo:warning=not found: {}", lib.display());
    }
}
