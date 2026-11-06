//! Picking the display server on Linux and the BSDs.
//!
//! winit uses Wayland whenever `WAYLAND_DISPLAY` is set and does not try X11 when that fails.
//! A variable left over from a Wayland session inside an X11 one (e.g. Hyprland, then Cinnamon on
//! Xorg) then stops the app with "NoCompositor". `use_x11_if_wayland_is_gone` asks winit for
//! X11 in that case.

/// Makes the window use X11 when `WAYLAND_DISPLAY` names a socket that does not exist and X11 is
/// available. Does nothing on other platforms.
pub fn use_x11_if_wayland_is_gone(options: &mut eframe::NativeOptions, trace: bool) {
    #[cfg(any(
        target_os = "linux",
        target_os = "dragonfly",
        target_os = "freebsd",
        target_os = "netbsd",
        target_os = "openbsd"
    ))]
    if unix::wayland_unreachable(
        std::env::var_os("WAYLAND_DISPLAY"),
        std::env::var_os("WAYLAND_SOCKET").is_some(),
        std::env::var_os("XDG_RUNTIME_DIR"),
        std::env::var_os("DISPLAY").is_some(),
    ) {
        if trace {
            eprintln!("WAYLAND_DISPLAY has no compositor behind it; using X11");
        }
        options.event_loop_builder = Some(Box::new(|builder| {
            use winit::platform::x11::EventLoopBuilderExtX11;
            builder.with_x11();
        }));
    }
    let _ = (options, trace);
}

#[cfg(any(
    target_os = "linux",
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd"
))]
mod unix {
    use std::ffi::OsString;
    use std::path::PathBuf;

    /// True when `WAYLAND_DISPLAY` is set but its socket does not exist, and X11 is there to fall
    /// back to. An inherited `WAYLAND_SOCKET` wins over `WAYLAND_DISPLAY`, so it is left alone.
    pub fn wayland_unreachable(
        wayland_display: Option<OsString>,
        wayland_socket: bool,
        runtime_dir: Option<OsString>,
        x11_display: bool,
    ) -> bool {
        let Some(name) = wayland_display.filter(|n| !n.is_empty()) else {
            return false; // winit already uses X11
        };
        if wayland_socket || !x11_display {
            return false;
        }
        let name = PathBuf::from(name);
        let path = if name.is_absolute() {
            name
        } else {
            match runtime_dir {
                Some(dir) => PathBuf::from(dir).join(name),
                None => return false, // no socket path to check; let winit report the problem
            }
        };
        !path.exists()
    }

    #[cfg(test)]
    mod tests {
        use super::wayland_unreachable;
        use std::ffi::OsString;

        #[test]
        fn stale_wayland_display_falls_back_to_x11() {
            let dir = std::env::temp_dir();
            let run = || Some(dir.clone().into_os_string());
            let missing = || Some(OsString::from("albumcraft-no-such-wayland-socket"));
            // left-over WAYLAND_DISPLAY in an X11 session: use X11
            assert!(wayland_unreachable(missing(), false, run(), true));
            // no X11, an inherited socket, or no WAYLAND_DISPLAY: leave winit alone
            assert!(!wayland_unreachable(missing(), false, run(), false));
            assert!(!wayland_unreachable(missing(), true, run(), true));
            assert!(!wayland_unreachable(None, false, run(), true));
            assert!(!wayland_unreachable(
                Some(OsString::new()),
                false,
                run(),
                true
            ));
            // the socket exists, by name or by absolute path: use Wayland
            let name = format!("albumcraft-test-wayland-{}", std::process::id());
            let sock = dir.join(&name);
            std::fs::write(&sock, b"").unwrap();
            let by_name = wayland_unreachable(Some(name.into()), false, run(), true);
            let by_path =
                wayland_unreachable(Some(sock.clone().into_os_string()), false, None, true);
            std::fs::remove_file(&sock).unwrap();
            assert!(!by_name);
            assert!(!by_path);
        }
    }
}
