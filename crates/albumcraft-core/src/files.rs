//! Renaming and moving picture files and folders on disk, only when the user asks for it
//! (deleting is `library::move_to_trash`, or `library::delete_permanently` when there is no
//! trash). A move never overwrites what is already there. On the same disk it is one rename;
//! to another disk the file or folder is copied under a temporary name, the copy is compared with
//! the original byte for byte, given its name, and only then is the original removed. The notes
//! follow with `Store::moved`.

use std::path::{Path, PathBuf};

/// A new file or folder name typed by the user, trimmed; an error when it can't be one (empty,
/// a path, characters some systems refuse).
pub fn check_name(name: &str) -> Result<String, String> {
    let name = name.trim();
    if name.is_empty() || name == "." || name == ".." {
        return Err(crate::msg!("name-empty", "Type a name.").into());
    }
    if let Some(c) = name.chars().find(|c| matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|') || c.is_control()) {
        return Err(crate::msg!("name-bad-char", "A name can't contain {char}.", char = c.escape_default()).into());
    }
    // Windows refuses names ending in a dot
    if name.ends_with('.') {
        return Err(crate::msg!("name-dot", "A name can't end with a dot.").into());
    }
    if name.len() > 255 {
        return Err(crate::msg!("name-long", "That name is too long.").into());
    }
    Ok(name.to_string())
}

/// The new path of a picture file renamed to `name`. Its extension is kept when `name` leaves
/// it out (the type of the file doesn't change by renaming it).
pub fn renamed_file(file: &str, name: &str) -> Result<String, String> {
    let name = check_name(name)?;
    let p = Path::new(file);
    let parent = p.parent().ok_or_else(|| String::from(crate::msg!("move-no-parent", "{path} has no folder.", path = file)))?;
    let ext = p.extension().map(|e| e.to_string_lossy().to_string()).unwrap_or_default();
    let has_ext = !ext.is_empty() && Path::new(&name).extension().is_some_and(|e| e.to_string_lossy().eq_ignore_ascii_case(&ext));
    let full = if ext.is_empty() || has_ext { name } else { format!("{name}.{ext}") };
    Ok(parent.join(full).to_string_lossy().into_owned())
}

/// The new path of a folder renamed to `name`.
pub fn renamed_folder(folder: &str, name: &str) -> Result<String, String> {
    let name = check_name(name)?;
    let parent = Path::new(folder).parent().ok_or_else(|| String::from(crate::msg!("move-no-parent", "{path} has no folder.", path = folder)))?;
    Ok(parent.join(name).to_string_lossy().into_owned())
}

/// The path of a file or folder moved into `dir`, under the same name.
pub fn moved_into(path: &str, dir: &str) -> Result<String, String> {
    let name = Path::new(path).file_name().ok_or_else(|| String::from(crate::msg!("move-no-name", "{path} has no name.", path = path)))?;
    Ok(Path::new(dir).join(name).to_string_lossy().into_owned())
}

/// Move (or rename) a file or folder from `from` to `to`: into a folder that exists, never over
/// something already there, never a folder into itself. Across disks it is copied, checked and
/// then removed (see `move_with`).
pub fn move_path(from: &str, to: &str) -> Result<(), String> {
    move_with(from, to, &|_, _| true)
}

/// `move_path`, telling how far a copy to another disk is: `step(bytes done, bytes in all)`,
/// which answers false to stop (the copy is then removed and the original stays).
pub fn move_with(from: &str, to: &str, step: &dyn Fn(u64, u64) -> bool) -> Result<(), String> {
    let (src, dst) = (Path::new(from), PathBuf::from(to));
    if from.starts_with(crate::imaging::SAMPLE_PREFIX) {
        return Err(crate::msg!("move-sample", "Sample pictures can't be moved or renamed.").into());
    }
    let Ok(meta) = std::fs::symlink_metadata(src) else {
        return Err(crate::msg!("move-missing", "{path} is not there any more.", path = from).into());
    };
    if src == dst {
        return Ok(());
    }
    if meta.is_dir() && dst.starts_with(src) {
        return Err(crate::msg!("move-into-itself", "A folder can't be moved into itself.").into());
    }
    if !dst.parent().is_some_and(Path::is_dir) {
        return Err(crate::msg!("move-no-folder", "The folder {path} doesn't exist.", path = dst.parent().unwrap_or(&dst).display()).into());
    }
    // something there already, unless it is the same file under another case of letters (a
    // case-only rename on Windows or macOS)
    if std::fs::symlink_metadata(&dst).is_ok() && !same_file(src, &dst) {
        return Err(taken(&dst));
    }
    match std::fs::rename(src, &dst) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::CrossesDevices => copy_across(src, &dst, step),
        Err(e) => Err(crate::msg!("move-failed", "Could not move {path}: {error}", path = from, error = e).into()),
    }
}

/// `to`'s folder is on the same disk as `from` (a move there is one quick rename). True when it
/// can't be told.
pub fn same_disk(from: &str, to: &str) -> bool {
    let dir = Path::new(to).parent().unwrap_or(Path::new(to));
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        match (std::fs::symlink_metadata(from), std::fs::metadata(dir)) {
            (Ok(a), Ok(b)) => a.dev() == b.dev(),
            _ => true,
        }
    }
    #[cfg(not(unix))]
    {
        // the drive or share at the start of each path
        let prefix = |p: &Path| match p.components().next() {
            Some(std::path::Component::Prefix(x)) => Some(x.as_os_str().to_string_lossy().to_lowercase()),
            _ => None,
        };
        prefix(Path::new(from)) == prefix(dir)
    }
}

fn taken(dst: &Path) -> String {
    crate::msg!("move-taken", "{name} already exists there.", name = dst.file_name().unwrap_or_default().to_string_lossy()).into()
}

/// To another disk: copy under a temporary name next to `dst`, compare, rename, remove the
/// original. Whatever goes wrong before the end leaves the original as it was.
fn copy_across(src: &Path, dst: &Path, step: &dyn Fn(u64, u64) -> bool) -> Result<(), String> {
    let from = src.display().to_string();
    let fail = |e: String| -> String { crate::msg!("move-failed", "Could not move {path}: {error}", path = from.as_str(), error = e).into() };
    let name = dst.file_name().unwrap_or_default().to_string_lossy().into_owned();
    let tmp = dst.with_file_name(format!(".{name}.albumcraft-{}.part", std::process::id()));
    remove_any(&tmp);
    let total = size_of(src).map_err(|e| fail(e.to_string()))?;
    let done = std::cell::Cell::new(0u64);
    let copied = copy_tree(src, &tmp, &|n| {
        done.set(done.get() + n);
        step(done.get(), total)
    })
    .and_then(|()| if same_tree(src, &tmp)? { Ok(()) } else { Err(std::io::Error::other("the copy differs from the original")) });
    if let Err(e) = copied {
        remove_any(&tmp);
        return Err(if e.kind() == std::io::ErrorKind::Interrupted { crate::msg!("move-stopped", "Stopped: {path} was not moved.", path = from.as_str()).into() } else { fail(e.to_string()) });
    }
    // taken meanwhile?
    if std::fs::symlink_metadata(dst).is_ok() {
        remove_any(&tmp);
        return Err(taken(dst));
    }
    if let Err(e) = std::fs::rename(&tmp, dst) {
        remove_any(&tmp);
        return Err(fail(e.to_string()));
    }
    let removed = if src.is_dir() { std::fs::remove_dir_all(src) } else { std::fs::remove_file(src) };
    removed.map_err(|e| {
        crate::msg!("move-original-left", "{to} was copied, but the original {path} could not be removed ({error}): both are there now.", to = dst.display(), path = from.as_str(), error = e).into()
    })
}

/// Bytes in a file, or in every file of a folder.
fn size_of(p: &Path) -> std::io::Result<u64> {
    let m = std::fs::symlink_metadata(p)?;
    if m.is_dir() {
        std::fs::read_dir(p)?.try_fold(0u64, |sum, e| Ok(sum.saturating_add(size_of(&e?.path())?)))
    } else {
        Ok(m.len())
    }
}

/// Copy a file (keeping its date) or a folder with everything in it; `progress(bytes)` answers
/// false to stop (an `Interrupted` error). Links are refused: what they point to is not the
/// user's to move.
fn copy_tree(src: &Path, dst: &Path, progress: &dyn Fn(u64) -> bool) -> std::io::Result<()> {
    use std::io::{Read, Write};
    let m = std::fs::symlink_metadata(src)?;
    if m.file_type().is_symlink() {
        return Err(std::io::Error::other(format!("{} is a link", src.display())));
    }
    if m.is_dir() {
        std::fs::create_dir(dst)?;
        for e in std::fs::read_dir(src)? {
            let e = e?;
            copy_tree(&e.path(), &dst.join(e.file_name()), progress)?;
        }
        return Ok(());
    }
    let mut input = std::fs::File::open(src)?;
    let mut out = std::fs::OpenOptions::new().write(true).create_new(true).open(dst)?;
    let mut buf = vec![0u8; 1 << 20];
    loop {
        let n = input.read(&mut buf)?;
        if n == 0 {
            break;
        }
        out.write_all(&buf[..n])?;
        if !progress(n as u64) {
            return Err(std::io::Error::from(std::io::ErrorKind::Interrupted));
        }
    }
    if let Ok(t) = m.modified() {
        let _ = out.set_modified(t);
    }
    out.sync_all()
}

/// The two files hold the same bytes, or the two folders the same names and contents.
fn same_tree(a: &Path, b: &Path) -> std::io::Result<bool> {
    use std::io::Read;
    let (ma, mb) = (std::fs::symlink_metadata(a)?, std::fs::symlink_metadata(b)?);
    if ma.is_dir() != mb.is_dir() {
        return Ok(false);
    }
    if ma.is_dir() {
        let names = |p: &Path| -> std::io::Result<Vec<std::ffi::OsString>> {
            let mut v: Vec<_> = std::fs::read_dir(p)?.map(|e| e.map(|e| e.file_name())).collect::<Result<_, _>>()?;
            v.sort();
            Ok(v)
        };
        let na = names(a)?;
        if na != names(b)? {
            return Ok(false);
        }
        for n in na {
            if !same_tree(&a.join(&n), &b.join(&n))? {
                return Ok(false);
            }
        }
        return Ok(true);
    }
    if ma.len() != mb.len() {
        return Ok(false);
    }
    let (mut fa, mut fb) = (std::fs::File::open(a)?, std::fs::File::open(b)?);
    let (mut ba, mut bb) = (vec![0u8; 1 << 20], vec![0u8; 1 << 20]);
    loop {
        let n = fa.read(&mut ba)?;
        if n == 0 {
            return Ok(true);
        }
        fb.read_exact(&mut bb[..n])?;
        if ba[..n] != bb[..n] {
            return Ok(false);
        }
    }
}

fn remove_any(p: &Path) {
    if p.is_dir() {
        let _ = std::fs::remove_dir_all(p);
    } else {
        let _ = std::fs::remove_file(p);
    }
}

fn same_file(a: &Path, b: &Path) -> bool {
    matches!((a.canonicalize(), b.canonicalize()), (Ok(x), Ok(y)) if x == y)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("albumcraft-files-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn names_are_checked() {
        assert_eq!(check_name("  Summer 2024 ").unwrap(), "Summer 2024");
        for bad in ["", "  ", ".", "..", "a/b", "a\\b", "a:b", "x?", "end.", "tab\there"] {
            assert!(check_name(bad).is_err(), "{bad:?}");
        }
        assert!(check_name(&"é".repeat(200)).is_err());
    }

    #[test]
    fn renaming_keeps_the_extension() {
        let f = Path::new("/p/IMG_1.JPG").to_string_lossy().to_string();
        let at = |n: &str| Path::new("/p").join(n).to_string_lossy().to_string();
        assert_eq!(renamed_file(&f, "beach").unwrap(), at("beach.JPG"));
        assert_eq!(renamed_file(&f, "beach.jpg").unwrap(), at("beach.jpg"));
        assert_eq!(renamed_file(&f, "beach.v2").unwrap(), at("beach.v2.JPG"));
        assert!(renamed_file(&f, "a/b").is_err());
    }

    #[test]
    fn moves_never_overwrite() {
        let d = temp("overwrite");
        let (a, b) = (d.join("a.jpg"), d.join("b.jpg"));
        std::fs::write(&a, b"a").unwrap();
        std::fs::write(&b, b"b").unwrap();
        let (sa, sb) = (a.to_string_lossy().to_string(), b.to_string_lossy().to_string());
        assert!(move_path(&sa, &sb).is_err());
        assert_eq!(std::fs::read(&b).unwrap(), b"b");
        assert_eq!(std::fs::read(&a).unwrap(), b"a");
        // into a folder that isn't there, or a missing file
        assert!(move_path(&sa, &d.join("no/a.jpg").to_string_lossy()).is_err());
        assert!(move_path(&d.join("gone.jpg").to_string_lossy(), &sb).is_err());
        assert!(move_path(&format!("{}1", crate::imaging::SAMPLE_PREFIX), &sb).is_err());
        let c = d.join("c.jpg");
        move_path(&sa, &c.to_string_lossy()).unwrap();
        assert!(!a.exists() && std::fs::read(&c).unwrap() == b"a");
        let _ = std::fs::remove_dir_all(&d);
    }

    /// What a move to another disk does, called directly (a test can't pick two disks).
    #[test]
    fn copies_to_another_disk_are_checked_before_the_original_goes() {
        let d = temp("across");
        let trip = d.join("trip");
        std::fs::create_dir_all(trip.join("inner")).unwrap();
        std::fs::write(trip.join("a.jpg"), vec![7u8; 3 << 20]).unwrap();
        std::fs::write(trip.join("inner/b.jpg"), b"b").unwrap();
        let old = std::fs::metadata(trip.join("a.jpg")).unwrap().modified().unwrap();
        let to = d.join("2024-trip");
        // stopped half way: nothing left behind, the original whole
        let steps = std::cell::Cell::new(0);
        let stopped = copy_across(&trip, &to, &|_, _| {
            steps.set(steps.get() + 1);
            steps.get() < 2
        });
        assert!(stopped.is_err() && !to.exists() && trip.join("a.jpg").is_file());
        assert_eq!(std::fs::read_dir(&d).unwrap().count(), 1, "no temporary copy left");
        copy_across(&trip, &to, &|_, _| true).unwrap();
        assert!(!trip.exists());
        assert_eq!(std::fs::read(to.join("a.jpg")).unwrap(), vec![7u8; 3 << 20]);
        assert_eq!(std::fs::read(to.join("inner/b.jpg")).unwrap(), b"b");
        assert_eq!(std::fs::metadata(to.join("a.jpg")).unwrap().modified().unwrap(), old);
        // a file, and one in the way
        let f = d.join("c.jpg");
        std::fs::write(&f, b"c").unwrap();
        std::fs::write(d.join("taken.jpg"), b"x").unwrap();
        assert!(copy_across(&f, &d.join("taken.jpg"), &|_, _| true).is_err());
        assert!(f.is_file() && std::fs::read(d.join("taken.jpg")).unwrap() == b"x");
        copy_across(&f, &to.join("c.jpg"), &|_, _| true).unwrap();
        assert!(!f.exists() && to.join("c.jpg").is_file());
        assert!(same_disk(&to.join("c.jpg").to_string_lossy(), &d.join("x.jpg").to_string_lossy()));
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn folders_move_with_their_pictures() {
        let d = temp("folders");
        let (trip, into) = (d.join("trip"), d.join("2024"));
        std::fs::create_dir_all(&trip).unwrap();
        std::fs::create_dir_all(&into).unwrap();
        std::fs::write(trip.join("a.jpg"), b"a").unwrap();
        let st = trip.to_string_lossy().to_string();
        // not into itself
        assert!(move_path(&st, &trip.join("inner").to_string_lossy()).is_err());
        let to = moved_into(&st, &into.to_string_lossy()).unwrap();
        move_path(&st, &to).unwrap();
        assert!(Path::new(&to).join("a.jpg").is_file() && !trip.exists());
        // an empty folder with the same name is not replaced
        std::fs::create_dir_all(&trip).unwrap();
        assert!(move_path(&to, &trip.to_string_lossy()).is_err());
        let _ = std::fs::remove_dir_all(&d);
    }
}
