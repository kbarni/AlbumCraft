//! Looking for a newer release: once when the window opens (unless switched off in Settings ▸
//! Interface) and when the user asks (main menu, Settings). A newer release opens a prompt that
//! offers to download and install it and restart, or to open the release page. The check, the
//! prompt and the installation for each way AlbumCraft is installed (AppImage, .deb, Flatpak,
//! .tar.gz, the Windows installer or zip, the macOS app) are `craft_update`'s, shared with the
//! other Craft apps. A copy run from a Cargo build tree never replaces itself; it opens the
//! release page instead.

use crate::app::App;
use crate::i18n::LOADER;
use craft_update::updater::{Action, Status, Updater};
use craft_update::{CheckResultDialog, CheckResultResponse};
use eframe::egui;
use fluent_bundle::FluentValue;
use std::collections::HashMap;

const VERSION: &str = env!("CARGO_PKG_VERSION");

/// AlbumCraft as `craft_update` knows it: where its releases are, how its files are named
/// (`albumcraft-<version>-<os>-<arch>.<kind>`), its icon and accent colour for the prompt.
pub static APP: craft_update::App = craft_update::App {
    icon_png: Some(include_bytes!("../assets/app-icon/hicolor/128x128/apps/albumcraft.png")),
    accent: Some([47, 123, 245]),
    developer: "kbarni",
    bundle_id_override: Some(crate::APP_ID),
    ..craft_update::App::new("albumcraft", "AlbumCraft", "Photo organizer and editor", "kbarni/AlbumCraft")
};

/// When the restart helper of an update started this process: finish the update (put the new
/// copy in place and start it) and return what came of it; the caller then exits. `None` for a
/// normal start.
pub fn finish_update() -> Option<Result<(), String>> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    craft_update::self_update::finish(&APP, &args)
}

pub struct Updates {
    updater: Updater,
    /// The user asked for a check; the next frame starts it.
    requested: bool,
}

impl Default for Updates {
    fn default() -> Self {
        Self { updater: Updater::new(&APP, VERSION).with_translator(translate), requested: false }
    }
}

impl App {
    /// Look for a newer release now and say what came of it (also when there is none).
    pub fn check_for_updates(&mut self) {
        self.updates.requested = true;
    }

    /// Each frame: start a check that is due, take up its result, draw the prompt or the result
    /// window. The check at start waits for a dialog shown at start (the online maps question, a
    /// crash report) to be answered, so the two don't cover each other.
    pub(crate) fn updates_ui(&mut self, ctx: &egui::Context) {
        let u = &mut self.updates;
        if std::mem::take(&mut u.requested) {
            u.updater.check_now(ctx);
        } else if self.persist && self.store.check_updates && self.dialog.is_none() {
            // does something only the first time
            u.updater.check_at_start(ctx);
        }
        u.updater.tick(ctx);
        // Closing for the update waits for background work (export, import, …) to end.
        match u.updater.show_if(ctx, self.tasks.is_empty()) {
            Action::OpenUrl(url) => ctx.open_url(egui::OpenUrl::new_tab(url)),
            // the normal close: the library is written, then the helper installs and restarts
            Action::Close => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
            Action::None => {}
        }
        let status = u.updater.status();
        if !u.updater.prompt_open() && !matches!(status, Status::Idle) {
            match CheckResultDialog::new(&APP, VERSION).with_translator(translate).show(ctx, status) {
                CheckResultResponse::Close => u.updater.dismiss_status(),
                CheckResultResponse::CheckAgain => u.updater.check_now(ctx),
                CheckResultResponse::RestartToUpdate => u.updater.restart_to_update(),
                CheckResultResponse::None => {}
            }
        }
    }

    /// `ALBUMCRAFT_SHOT_VIEW=update`: the prompt for a made-up newer release (no network).
    pub(crate) fn offer_demo_update(&mut self) {
        let mut v = craft_update::parse_version(VERSION).expect("the package version");
        v.minor += 1;
        v.patch = 0;
        let release = craft_update::Release {
            tag: format!("v{v}"),
            name: format!("AlbumCraft {v}"),
            version: Some(v),
            html_url: format!("{}/latest", APP.releases_url()),
            ..Default::default()
        };
        self.updates.updater.offer(release);
    }

    /// At exit: start the helper that installs a downloaded update once AlbumCraft has closed.
    pub(crate) fn start_pending_update(&mut self) {
        self.updates.updater.start_pending_update();
    }
}

/// `craft_update`'s English texts (its lookup keys) and the messages that translate them. Its
/// placeholders (`{app}`) are Fluent variables of the same name (`{ $app }`). A text not listed
/// stays in English.
const MESSAGES: &[(&str, &str)] = &[
    ("{app} {version} is available with the latest features and bug fixes.", "update-available"),
    ("v{from} to {to}", "update-versions"),
    ("by {developer}", "update-by"),
    ("What's new", "update-whats-new"),
    ("Update", "update-badge"),
    ("Update and Restart", "update-and-restart"),
    ("Restart to Update", "update-restart"),
    ("Save and Update", "update-save-and-update"),
    ("Download", "update-download"),
    ("Later", "update-later"),
    ("Dismiss", "update-dismiss"),
    ("Close", "update-close"),
    ("OK", "update-ok"),
    ("Check Again", "update-check-again"),
    ("Saving your work...", "update-saving"),
    ("Downloading and installing update…", "update-downloading"),
    ("Checking for updates…", "update-checking"),
    ("Installing update…", "update-installing"),
    ("Update ready. Close {app} to install it.", "update-ready-windows"),
    ("The update is installed. Close {app} to finish; it restarts by itself.", "update-ready"),
    ("{app} updated to {version}.", "update-updated"),
    ("The update could not be installed: {reason}", "update-install-failed"),
    ("The update failed unexpectedly.", "update-failed"),
    ("The update stopped unexpectedly.", "update-stopped"),
    ("Could not check for updates.", "update-check-failed"),
    ("All releases are listed at {url}", "update-all-releases"),
    ("{app} {version} is the newest version available.", "update-up-to-date"),
    ("{app} {version} follows the version chosen in the launcher, so it does not update itself.", "update-managed"),
    ("Cannot auto-update this installation type. Please download manually.", "update-manual"),
    ("Couldn't start the update ({error})", "update-start-failed"),
    ("Save or close your open work first; the update restarts the app.", "update-wait-for-work"),
];

/// `craft_update`'s text in the interface language (see [`MESSAGES`]).
fn translate(text: &str, args: &[(&str, &str)]) -> String {
    match MESSAGES.iter().find(|(english, _)| *english == text) {
        Some((_, id)) if LOADER.has(id) => {
            let args: HashMap<&str, FluentValue> = args.iter().map(|(k, v)| (*k, FluentValue::from(*v))).collect();
            LOADER.get_args(id, args)
        }
        _ => craft_update::translate::fill(text, args),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Each English text is the English message, its `{name}`s written `{ $name }`, so the
    /// interface says what `craft_update` means.
    #[test]
    fn messages_are_the_english_texts() {
        for (english, id) in MESSAGES {
            assert!(LOADER.has(id), "{id} is not in i18n/en");
            // AlbumCraft saves by itself: what holds the update back is background work
            if *id == "update-wait-for-work" {
                continue;
            }
            let mut args = vec![];
            let mut rest = *english;
            while let Some((_, after)) = rest.split_once('{') {
                let (name, next) = after.split_once('}').expect("closed placeholder");
                args.push((name, format!("{{{name}}}")));
                rest = next;
            }
            let args: Vec<(&str, &str)> = args.iter().map(|(k, v)| (*k, v.as_str())).collect();
            assert_eq!(translate(english, &args), *english, "{id}");
        }
    }

    /// `cargo test real_release_check -- --ignored --nocapture`: asks GitHub as an older copy
    /// would, and finds the latest release (not the networks' `model-1`).
    #[test]
    #[ignore]
    fn real_release_check() {
        let http = craft_update::Http::new(concat!("AlbumCraft/", env!("CARGO_PKG_VERSION"))).expect("http");
        let found = craft_update::check_for_update(&APP, &http, "0.1.0").expect("check").expect("a newer release");
        println!("found {} with {} files", found.tag, found.assets.len());
        assert!(found.tag.starts_with('v'), "{}", found.tag);
        assert!(found.version.is_some());
    }

    /// The files release.yml publishes are the ones craft-update picks for each kind of install
    /// (packaging/linux/package.sh, packaging/windows/package.ps1, the macOS job).
    #[test]
    fn each_install_finds_its_release_file() {
        use craft_update::{Arch, Asset, Os, PackageKind as K, Platform};
        let names = [
            "albumcraft-0.6.0-linux-x86_64.AppImage",
            "albumcraft-0.6.0-linux-x86_64.AppImage.zsync",
            "albumcraft-0.6.0-linux-x86_64.deb",
            "albumcraft-0.6.0-linux-x86_64.flatpak",
            "albumcraft-0.6.0-linux-x86_64.tar.gz",
            "albumcraft-0.6.0-macos-arm64.dmg",
            "albumcraft-0.6.0-windows-x64.msi",
            "albumcraft-0.6.0-windows-x64-portable.zip",
            "SHA256SUMS.txt",
        ];
        let release = craft_update::Release {
            assets: names.iter().map(|n| Asset { name: (*n).into(), size: 1, url: format!("https://x/{n}"), sha256: None }).collect(),
            ..Default::default()
        };
        let linux = Platform { os: Os::Linux, arch: Arch::X64 };
        let windows = Platform { os: Os::Windows, arch: Arch::X64 };
        let mac = Platform { os: Os::MacOs, arch: Arch::Arm64 };
        for (platform, kind, file) in [
            (linux, K::AppImage, "albumcraft-0.6.0-linux-x86_64.AppImage"),
            (linux, K::Deb, "albumcraft-0.6.0-linux-x86_64.deb"),
            (linux, K::Flatpak, "albumcraft-0.6.0-linux-x86_64.flatpak"),
            (linux, K::TarGz, "albumcraft-0.6.0-linux-x86_64.tar.gz"),
            (windows, K::Msi, "albumcraft-0.6.0-windows-x64.msi"),
            (windows, K::PortableZip, "albumcraft-0.6.0-windows-x64-portable.zip"),
            (mac, K::Dmg, "albumcraft-0.6.0-macos-arm64.dmg"),
        ] {
            let picked = craft_update::pick_asset(&release, &[APP.id], platform, &[kind]).map(|(a, _)| a.name.as_str());
            assert_eq!(picked, Some(file), "{kind:?}");
        }
    }

    #[test]
    fn unknown_texts_stay_english() {
        assert_eq!(translate("Something new from {app}", &[("app", "AlbumCraft")]), "Something new from AlbumCraft");
    }

    #[test]
    fn the_running_version_and_releases_are_albumcraft_s() {
        assert_eq!(APP.releases_url(), "https://github.com/kbarni/AlbumCraft/releases");
        assert_eq!(APP.bundle_id(), crate::APP_ID);
        assert!(craft_update::parse_version(VERSION).is_some());
    }
}
