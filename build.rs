// On Windows, embed the gecko icon (assets/app-icon/albumcraft.ico) and the version in
// albumcraft.exe, so Explorer, the Start menu shortcut and taskbar pins show it.
fn main() {
    println!("cargo:rerun-if-changed=assets/app-icon/albumcraft.ico");
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
