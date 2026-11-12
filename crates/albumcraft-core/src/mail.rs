//! Email: small copies of pictures, attached to a new message in the user's mail program.
//!
//! Nothing is sent from here: the mail program opens a new message with the files attached, and
//! the user writes and sends it. How it is opened depends on the system:
//! - Linux / BSD: `xdg-email --attach` (xdg-utils), which attaches for Thunderbird, Evolution and
//!   KMail; other programs get a message without the files.
//! - Windows: Simple MAPI (`MAPISendMailW`, through `mapi32.dll`), which attaches for classic
//!   Outlook, Thunderbird and other MAPI mail programs. The only `unsafe` code besides printing.
//! - macOS: `open -a Mail` with the files, a new message in Mail.
//!
//! When none of that works (no mail program, a webmail), the window tells the user where the
//! copies are, to attach them by hand.

use std::path::{Path, PathBuf};

/// Longest side of the copies, in pixels, and their JPEG quality.
pub const MAX_DIM: u32 = 1200;
pub const QUALITY: u8 = 85;

/// How the new message was opened.
#[derive(Debug, PartialEq)]
pub enum Opened {
    /// A new message with the files attached is open (or the user closed it).
    Attached,
    /// A mail program was asked to open a message with the files; whether it attached them could
    /// not be checked (xdg-email with a program it does not know attaches nothing).
    Asked,
}

/// A new folder for one message's copies, in the temporary folder. Folders of earlier messages
/// older than a day are removed (the mail program has read their files by then).
pub fn new_folder() -> Result<PathBuf, String> {
    let base = std::env::temp_dir().join("AlbumCraft-email");
    tidy(&base, std::time::Duration::from_secs(24 * 3600));
    let stamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis()).unwrap_or(0);
    let dir = base.join(format!("{stamp}"));
    std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    Ok(dir)
}

fn tidy(base: &Path, age: std::time::Duration) {
    let Ok(rd) = std::fs::read_dir(base) else { return };
    for e in rd.flatten() {
        let old = e.metadata().and_then(|m| m.modified()).ok().and_then(|t| t.elapsed().ok()).is_some_and(|a| a > age);
        if old {
            let _ = std::fs::remove_dir_all(e.path());
        }
    }
}

/// Open a new message in the mail program with these files attached. Err when no mail program
/// could be asked (then the files have to be attached by hand).
pub fn compose(files: &[PathBuf], subject: &str) -> Result<Opened, String> {
    if files.is_empty() {
        return Err(crate::msg!("mail-no-pictures", "no pictures to attach").into());
    }
    platform::compose(files, subject)
}

/// Make the small copies of these pictures in a new folder, then open a new message with them
/// attached. `step(done, total)` says how far it is and answers false when the user stopped it.
/// The report says what happened and, unless the files were attached for sure, where the copies
/// are (to attach by hand).
/// `opts`: the camera information, creator and copyright as for an export; the size, quality and
/// signature are the email's own (`MAX_DIM`, `QUALITY`, none).
pub fn email(jobs: &[crate::commands::ExportJob], subject: &str, with_location: bool, opts: &crate::export::Options, step: &dyn Fn(usize, usize) -> bool) -> crate::library::Report {
    use crate::library::Report;
    let dir = match new_folder() {
        Ok(d) => d,
        Err(e) => return Report::failed(crate::msg!("mail-copies-fail", "Could not make the copies for the email: {error}", error = e)),
    };
    let Some((files, failed)) = copies(jobs, &dir, with_location, opts, step) else {
        let _ = std::fs::remove_dir_all(&dir);
        return Report::done(crate::msg!("mail-stopped", "The email was not prepared: stopped."));
    };
    if files.is_empty() {
        return Report::failed(crate::msg!("mail-none-read", "Could not make the copies for the email: none of the pictures could be read."));
    }
    let n = files.len();
    let left_out = if failed > 0 { format!("\n{}", crate::msg!("mail-left-out", "{n} {n:picture|pictures} could not be read and were left out.", n = failed)) } else { String::new() };
    match compose(&files, subject) {
        Ok(Opened::Attached) => Report::done(crate::msg!("mail-opened", "A new message with {n} {n:picture|pictures} is open in your mail program.{left_out}", n = n, left_out = left_out)).showing(dir),
        Ok(Opened::Asked) => Report::done(crate::msg!(
            "mail-asked",
            "Your mail program was asked for a new message with {n} {n:picture|pictures}. If they are not attached, attach them from:\n{path}{left_out}",
            n = n,
            path = dir.display(),
            left_out = left_out
        ))
        .showing(dir),
        Err(e) => {
            // stays on screen until closed, with Show in folder
            let mut r = Report::failed(crate::msg!(
                "mail-open-fail",
                "Could not open your mail program ({error}).\nThe small copies of {n} {n:picture|pictures} are in:\n{path}\nAttach them to your message.{left_out}",
                error = e,
                n = n,
                path = dir.display(),
                left_out = left_out
            ));
            r.show = Some(dir);
            r
        }
    }
}

/// A file name for an attached copy without the characters mail programs take apart: xdg-email
/// gives Thunderbird every attachment in one comma-separated, quoted list
/// (`attachment='a.jpg,b.jpg'`), so a comma cuts a path in two and a quote ends the list.
/// Those, backslashes and spaces become `_` (no doubled or trailing ones).
fn attachment_stem(stem: &str) -> String {
    let mut out = String::with_capacity(stem.len());
    for c in stem.chars() {
        let c = if matches!(c, ',' | '\'' | '"' | '\\' | '%' | ';') || c.is_whitespace() || c.is_control() { '_' } else { c };
        if !(c == '_' && out.ends_with('_')) {
            out.push(c);
        }
    }
    let out = out.trim_matches('_');
    if out.is_empty() { "picture".into() } else { out.to_string() }
}

/// The small copies in `dir`, and how many pictures could not be read; None when stopped.
fn copies(jobs: &[crate::commands::ExportJob], dir: &Path, with_location: bool, opts: &crate::export::Options, step: &dyn Fn(usize, usize) -> bool) -> Option<(Vec<PathBuf>, usize)> {
    let opts = crate::export::Options { max_dim: MAX_DIM, quality: QUALITY, signature: None, ..opts.clone() };
    let mut files = vec![];
    let mut failed = 0;
    for (i, (key, name, edits, place)) in jobs.iter().enumerate() {
        if !step(i, jobs.len()) {
            return None;
        }
        let stem = &attachment_stem(name.rsplit_once('.').map(|(s, _)| s).unwrap_or(name));
        let gps = if with_location { crate::geo::effective(place.as_ref(), crate::geo::file_gps(key)) } else { None };
        let made = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| crate::export::export_one(key, edits, dir, stem, &opts, gps, None)));
        match made {
            Ok(Ok(p)) => files.push(p),
            _ => failed += 1,
        }
    }
    Some((files, failed))
}

#[cfg(windows)]
mod platform {
    use super::Opened;
    use std::os::windows::ffi::OsStrExt;
    use std::path::PathBuf;
    use windows::core::{s, w, PWSTR};
    use windows::Win32::Foundation::FreeLibrary;
    use windows::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW};
    use windows::Win32::System::Mapi::{MapiFileDescW, MapiMessageW, MAPI_DIALOG, MAPI_E_USER_ABORT, MAPI_LOGON_UI, SUCCESS_SUCCESS};

    /// A NUL-terminated UTF-16 copy, kept alive for as long as the call that uses it.
    fn wide(s: &std::ffi::OsStr) -> Vec<u16> {
        s.encode_wide().chain(std::iter::once(0)).collect()
    }

    pub fn compose(files: &[PathBuf], subject: &str) -> Result<Opened, String> {
        let mut paths: Vec<Vec<u16>> = files.iter().map(|f| wide(f.as_os_str())).collect();
        let mut names: Vec<Vec<u16>> = files.iter().map(|f| wide(f.file_name().unwrap_or(f.as_os_str()))).collect();
        let mut subject = wide(std::ffi::OsStr::new(subject));
        let mut descs: Vec<MapiFileDescW> = paths
            .iter_mut()
            .zip(names.iter_mut())
            .map(|(p, n)| MapiFileDescW {
                // u32::MAX: the attachments are not placed inside the message text
                nPosition: u32::MAX,
                lpszPathName: PWSTR(p.as_mut_ptr()),
                lpszFileName: PWSTR(n.as_mut_ptr()),
                ..Default::default()
            })
            .collect();
        let message = MapiMessageW {
            lpszSubject: PWSTR(subject.as_mut_ptr()),
            nFileCount: u32::try_from(descs.len()).map_err(|_| crate::msg!("mail-too-many", "too many pictures"))?,
            lpFiles: descs.as_mut_ptr(),
            ..Default::default()
        };
        // SAFETY: mapi32.dll is the system's Simple MAPI stub, which passes the call to the
        // default mail program. `MAPISendMailW` is looked up by name and called with the
        // signature the windows crate's `LPMAPISENDMAILW` gives it. `message` and everything it points to (`descs`,
        // `paths`, `names`, `subject`: NUL-terminated UTF-16 buffers) live on this stack frame
        // until after the call returns; the function only reads them. The library is freed only
        // after the call.
        unsafe {
            let lib = LoadLibraryW(w!("mapi32.dll")).map_err(|e| crate::msg!("mail-no-mapi", "no mail program (mapi32.dll: {error})", error = e))?;
            let result = match GetProcAddress(lib, s!("MAPISendMailW")) {
                Some(f) => {
                    // the signature of MAPISendMailW (`LPMAPISENDMAILW` in the windows crate)
                    type SendMailW = unsafe extern "system" fn(usize, usize, *const MapiMessageW, u32, u32) -> u32;
                    let send = std::mem::transmute::<unsafe extern "system" fn() -> isize, SendMailW>(f);
                    Ok(send(0, 0, &message, MAPI_DIALOG | MAPI_LOGON_UI, 0))
                }
                None => Err(crate::msg!("mail-no-attach", "the mail program does not take attachments (no MAPISendMailW)").to_string()),
            };
            let _ = FreeLibrary(lib);
            match result? {
                SUCCESS_SUCCESS | MAPI_E_USER_ABORT => Ok(Opened::Attached),
                code => Err(crate::msg!("mail-refused", "the mail program refused the message (MAPI error {code})", code = code).to_string()),
            }
        }
    }
}

#[cfg(target_os = "macos")]
mod platform {
    use super::Opened;
    use std::path::PathBuf;

    pub fn compose(files: &[PathBuf], _subject: &str) -> Result<Opened, String> {
        let status = std::process::Command::new("open").arg("-a").arg("Mail").args(files).status().map_err(|e| crate::msg!("mail-open-mail", "could not open Mail: {error}", error = e))?;
        if status.success() { Ok(Opened::Attached) } else { Err(crate::msg!("mail-open-mail-short", "could not open Mail").into()) }
    }
}

#[cfg(not(any(windows, target_os = "macos")))]
mod platform {
    use super::Opened;
    use std::path::PathBuf;

    pub fn compose(files: &[PathBuf], subject: &str) -> Result<Opened, String> {
        let mut cmd = std::process::Command::new("xdg-email");
        cmd.arg("--utf8").arg("--subject").arg(subject);
        for f in files {
            cmd.arg("--attach").arg(f);
        }
        // xdg-email may wait for the mail program it starts: not waited for here
        match cmd.stdin(std::process::Stdio::null()).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null()).spawn() {
            Ok(_) => Ok(Opened::Asked),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Err(crate::msg!("mail-no-xdg", "xdg-email (from xdg-utils) is not installed").into()),
            Err(e) => Err(crate::msg!("mail-xdg-fail", "could not start xdg-email: {error}", error = e).into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn attachment_names_survive_the_mail_program() {
        assert_eq!(
            attachment_stem("Sunlight_through_clouds_and_view_of_Ginkaku-ji_Temple_from_above,_Kyoto,_Japan"),
            "Sunlight_through_clouds_and_view_of_Ginkaku-ji_Temple_from_above_Kyoto_Japan"
        );
        assert_eq!(attachment_stem("Urin-in_(Kita-Ku_Kyoto)_Temple"), "Urin-in_(Kita-Ku_Kyoto)_Temple");
        assert_eq!(attachment_stem("Togetsu_Café"), "Togetsu_Café");
        assert_eq!(attachment_stem("John's \"best\" shot; 100%"), "John_s_best_shot_100");
        assert_eq!(attachment_stem(",,,"), "picture");
    }

    #[test]
    fn old_message_folders_are_tidied_new_ones_kept() {
        let base = std::env::temp_dir().join(format!("albumcraft-mail-tidy-{}", std::process::id()));
        let old = base.join("1");
        std::fs::create_dir_all(&old).unwrap();
        std::fs::write(old.join("a.jpg"), b"x").unwrap();
        tidy(&base, std::time::Duration::from_secs(3600));
        assert!(old.exists(), "a folder younger than the age stays");
        tidy(&base, std::time::Duration::ZERO);
        assert!(!old.exists());
        let _ = std::fs::remove_dir_all(&base);
    }

    /// The copies are small JPEGs, edits applied; an unreadable picture is left out, counted.
    #[test]
    fn copies_are_small_edited_jpegs() {
        let dir = std::env::temp_dir().join(format!("albumcraft-mail-copies-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let big = dir.join("big.jpg");
        image::DynamicImage::ImageRgba8(crate::imaging::sample_image(2, 3000, 2000)).to_rgb8().save(&big).unwrap();
        let out = dir.join("out");
        std::fs::create_dir_all(&out).unwrap();
        let jobs: Vec<crate::commands::ExportJob> = vec![
            (big.to_string_lossy().to_string(), "big.jpg".into(), vec![crate::imaging::EditOp::Rotate(1)], None),
            ("/no/such/picture.jpg".into(), "picture.jpg".into(), vec![], None),
        ];
        let (files, failed) = copies(&jobs, &out, false, &Default::default(), &|_, _| true).unwrap();
        assert_eq!((files.len(), failed), (1, 1));
        let made = image::open(&files[0]).unwrap();
        // turned a quarter, longest side MAX_DIM
        assert_eq!((made.width(), made.height()), ((MAX_DIM as f32 * 2.0 / 3.0).round() as u32, MAX_DIM));
        assert!(copies(&jobs, &out, false, &Default::default(), &|_, _| false).is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn nothing_to_attach_is_an_error() {
        assert!(compose(&[], "x").is_err());
    }
}
