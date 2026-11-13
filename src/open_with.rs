//! "Open with": the programs of this computer that open a kind of picture, to offer in the
//! context menu. On Linux and the BSDs they are the desktop entries (`*.desktop`) listed for the
//! file's type in the `mimeinfo.cache` files of the XDG data folders; on Windows the system's
//! own "Open with" window is used instead; macOS offers only the default program.

use std::path::{Path, PathBuf};

/// A program that can open a picture.
#[derive(Clone, Debug, PartialEq)]
pub struct Program {
    pub name: String,
    /// The desktop entry's command line, with its field codes (`%f`, `%U`…).
    exec: String,
}

/// The file type of a picture, from its extension (for looking up programs).
pub fn mime_of(path: &str) -> Option<&'static str> {
    let ext = Path::new(path).extension()?.to_string_lossy().to_ascii_lowercase();
    Some(match ext.as_str() {
        "jpg" | "jpeg" | "jpe" => "image/jpeg",
        "png" => "image/png",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "bmp" => "image/bmp",
        "tif" | "tiff" => "image/tiff",
        "heic" | "heif" => "image/heif",
        "avif" => "image/avif",
        "cr2" => "image/x-canon-cr2",
        "cr3" => "image/x-canon-cr3",
        "nef" => "image/x-nikon-nef",
        "arw" => "image/x-sony-arw",
        "dng" => "image/x-adobe-dng",
        "orf" => "image/x-olympus-orf",
        "rw2" => "image/x-panasonic-rw2",
        "raf" => "image/x-fuji-raf",
        "pef" => "image/x-pentax-pef",
        _ => return None,
    })
}

/// The programs listed for this file type, by name (none where that is not known).
pub fn programs_for(mime: &str) -> Vec<Program> {
    if !cfg!(all(unix, not(target_os = "macos"))) {
        return vec![];
    }
    let dirs = data_dirs();
    let mut ids: Vec<String> = vec![];
    for d in &dirs {
        let Ok(text) = std::fs::read_to_string(d.join("applications/mimeinfo.cache")) else { continue };
        for id in listed_for(&text, mime) {
            if !ids.contains(&id) {
                ids.push(id);
            }
        }
    }
    let mut out: Vec<Program> = vec![];
    for id in ids.iter().filter(|id| !id.starts_with("io.github.kbarni.albumcraft")) {
        // the first data folder that has the entry wins (the user's own before the system's)
        let Some(text) = dirs.iter().find_map(|d| std::fs::read_to_string(d.join("applications").join(id)).ok()) else { continue };
        if let Some(p) = parse_entry(&text)
            && !out.iter().any(|o| o.name == p.name)
        {
            out.push(p);
        }
    }
    out.sort_by_key(|p| p.name.to_lowercase());
    out.truncate(15);
    out
}

/// Start `program` on the file at `path`.
pub fn open(program: &Program, path: &str) -> std::io::Result<()> {
    let args = command_line(&program.exec, path);
    let Some((cmd, rest)) = args.split_first() else {
        return Err(std::io::Error::other("empty command"));
    };
    std::process::Command::new(cmd).args(rest).spawn().map(drop)
}

/// The system's own "Open with" window exists (Windows).
pub fn has_system_chooser() -> bool {
    cfg!(target_os = "windows")
}

/// Show the system's "Open with" window for the file (Windows).
pub fn system_chooser(path: &str) -> std::io::Result<()> {
    std::process::Command::new("rundll32.exe").arg("shell32.dll,OpenAs_RunDLL").arg(path).spawn().map(drop)
}

fn data_dirs() -> Vec<PathBuf> {
    let mut v = vec![];
    match std::env::var_os("XDG_DATA_HOME") {
        Some(h) if !h.is_empty() => v.push(PathBuf::from(h)),
        _ => v.extend(dirs::home_dir().map(|h| h.join(".local/share"))),
    }
    let sys = std::env::var("XDG_DATA_DIRS").ok().filter(|s| !s.is_empty()).unwrap_or_else(|| "/usr/local/share:/usr/share".into());
    v.extend(sys.split(':').filter(|s| !s.is_empty()).map(PathBuf::from));
    v
}

/// The desktop entry ids a `mimeinfo.cache` lists for `mime`.
fn listed_for(cache: &str, mime: &str) -> Vec<String> {
    cache
        .lines()
        .filter_map(|l| l.split_once('='))
        .filter(|(k, _)| k.trim() == mime)
        .flat_map(|(_, v)| v.split(';').map(str::trim).filter(|s| s.ends_with(".desktop")).map(str::to_string).collect::<Vec<_>>())
        .collect()
}

/// Name and command of a desktop entry that is a program to show.
fn parse_entry(text: &str) -> Option<Program> {
    let (mut name, mut exec, mut in_entry, mut hidden) = (None, None, false, false);
    for line in text.lines().map(str::trim) {
        if line.starts_with('[') {
            in_entry = line == "[Desktop Entry]";
            continue;
        }
        if !in_entry {
            continue;
        }
        let Some((k, v)) = line.split_once('=') else { continue };
        match k.trim() {
            "Name" => name = Some(v.trim().to_string()),
            "Exec" => exec = Some(v.trim().to_string()),
            "NoDisplay" | "Hidden" if v.trim() == "true" => hidden = true,
            "Type" if v.trim() != "Application" => hidden = true,
            _ => {}
        }
    }
    let (name, exec) = (name?, exec?);
    (!hidden && !name.is_empty() && !exec.is_empty()).then_some(Program { name, exec })
}

/// The command line of a desktop entry's `Exec` for one file: quoted words kept together, the
/// file in place of `%f %F %u %U` (added at the end when there is none), other codes dropped.
fn command_line(exec: &str, path: &str) -> Vec<String> {
    let mut words: Vec<String> = vec![];
    let (mut cur, mut quoted, mut any, mut chars) = (String::new(), false, false, exec.chars().peekable());
    while let Some(c) = chars.next() {
        match c {
            '"' => {
                quoted = !quoted;
                any = true;
            }
            '\\' if quoted => {
                if let Some(n) = chars.next() {
                    cur.push(n);
                }
            }
            c if c.is_whitespace() && !quoted => {
                if any || !cur.is_empty() {
                    words.push(std::mem::take(&mut cur));
                }
                any = false;
            }
            c => cur.push(c),
        }
    }
    if any || !cur.is_empty() {
        words.push(cur);
    }
    let mut placed = false;
    let mut out = vec![];
    for w in words {
        match w.as_str() {
            "%f" | "%F" | "%u" | "%U" => {
                out.push(path.to_string());
                placed = true;
            }
            "%i" | "%c" | "%k" => {}
            _ => {
                let w = w.replace("%%", "\u{0}");
                if w.contains('%') {
                    // a code inside a word (rare): leave the word out
                    let cleaned: String = w.split('%').next().unwrap_or("").to_string();
                    if !cleaned.is_empty() {
                        out.push(cleaned.replace('\u{0}', "%"));
                    }
                } else {
                    out.push(w.replace('\u{0}', "%"));
                }
            }
        }
    }
    if !placed {
        out.push(path.to_string());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn desktop_entries_are_read() {
        let cache = "[MIME Cache]\nimage/jpeg=gimp.desktop;org.gnome.eog.desktop;\nimage/png=gimp.desktop;\n";
        assert_eq!(listed_for(cache, "image/jpeg"), vec!["gimp.desktop", "org.gnome.eog.desktop"]);
        assert!(listed_for(cache, "image/gif").is_empty());
        let entry = "[Desktop Entry]\nType=Application\nName=GNU Image Manipulation Program\nName[fr]=GIMP\nExec=gimp-2.10 %U\n[Desktop Action new]\nName=New\nExec=gimp --new";
        let p = parse_entry(entry).unwrap();
        assert_eq!(p.name, "GNU Image Manipulation Program");
        assert_eq!(command_line(&p.exec, "/a b/c.jpg"), vec!["gimp-2.10", "/a b/c.jpg"]);
        assert!(parse_entry("[Desktop Entry]\nName=X\nExec=x\nNoDisplay=true").is_none());
        assert!(parse_entry("[Desktop Entry]\nName=X").is_none());
    }

    #[test]
    fn exec_lines_take_the_file() {
        assert_eq!(command_line("\"/opt/my app/run\" --open %f --icon %i", "/p.jpg"), vec!["/opt/my app/run", "--open", "/p.jpg", "--icon"]);
        assert_eq!(command_line("viewer", "/p.jpg"), vec!["viewer", "/p.jpg"]);
        assert_eq!(command_line("x --pct=100%% %F", "/p.jpg"), vec!["x", "--pct=100%", "/p.jpg"]);
        assert_eq!(mime_of("/a/B.JPG"), Some("image/jpeg"));
        assert_eq!(mime_of("/a/b"), None);
    }
}
