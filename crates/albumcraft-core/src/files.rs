//! Renaming and moving picture files and folders on disk, when the user asks for it: with
//! moving to the trash (`library::move_to_trash`), the only changes AlbumCraft makes to the
//! originals. A move is one rename on the same disk: it never copies, never overwrites what is
//! already there, and either happens completely or not at all. The notes follow with
//! `Store::moved`.

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

/// Move (or rename) a file or folder from `from` to `to`: on the same disk, into a folder that
/// exists, never over something already there, never a folder into itself.
pub fn move_path(from: &str, to: &str) -> Result<(), String> {
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
        return Err(crate::msg!("move-taken", "{name} already exists there.", name = dst.file_name().unwrap_or_default().to_string_lossy()).into());
    }
    std::fs::rename(src, &dst).map_err(|e| {
        if e.kind() == std::io::ErrorKind::CrossesDevices {
            crate::msg!("move-other-disk", "{path} is on another disk: AlbumCraft only moves pictures and folders within the same disk.", path = from).into()
        } else {
            crate::msg!("move-failed", "Could not move {path}: {error}", path = from, error = e).into()
        }
    })
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
