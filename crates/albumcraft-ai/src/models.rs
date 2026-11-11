//! The packs of networks AlbumCraft can use, and which of them are on this computer.
//!
//! The list is built into the program (`models.json`, next to this crate's `Cargo.toml`): for
//! each pack its name, version, licence, and the name, size and SHA-256 of each of its files.
//! The files are assets of a release of the project itself, never fetched from anywhere else:
//! a pack only names a release and file names, and the address is made here.
//!
//! Downloaded files are kept in `<data dir>/AlbumCraft/models/`.

use serde::Deserialize;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// Where the project's release assets are downloaded from.
const RELEASES: &str = "https://github.com/kbarni/AlbumCraft/releases/download";

#[derive(Deserialize, Clone, Debug, Default)]
pub struct Manifest {
    #[serde(default)]
    pub packs: Vec<Pack>,
}

/// Networks that are downloaded and removed together, for one feature.
#[derive(Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Pack {
    /// The name it is known by in commands and settings: "faces", "search-light"…
    pub id: String,
    /// What the user reads: "Faces", "Smart search: Light (English)".
    pub title: String,
    /// One sentence on what it is for.
    #[serde(default)]
    pub about: String,
    /// The licence of the networks, shown before downloading.
    pub licence: String,
    pub version: u32,
    /// The release of the project that holds the files (its tag).
    pub release: String,
    pub files: Vec<ModelFile>,
}

#[derive(Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct ModelFile {
    pub name: String,
    pub size: u64,
    /// 64 hexadecimal digits.
    pub sha256: String,
}

/// A name that is safe as a file name and inside an address.
fn plain_name(s: &str) -> bool {
    !s.is_empty() && s.len() <= 100 && !s.starts_with('.') && s.bytes().all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
}

impl ModelFile {
    fn valid(&self) -> bool {
        plain_name(&self.name) && self.size > 0 && self.sha256.len() == 64 && self.sha256.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    }
}

impl Pack {
    pub fn valid(&self) -> bool {
        plain_name(&self.id) && plain_name(&self.release) && !self.title.is_empty() && !self.licence.is_empty() && !self.files.is_empty() && self.files.iter().all(ModelFile::valid)
    }

    /// Bytes to download.
    pub fn bytes(&self) -> u64 {
        self.files.iter().fold(0u64, |n, f| n.saturating_add(f.size))
    }

    /// Where a file of the pack is downloaded from.
    pub fn url(&self, file: &ModelFile) -> String {
        format!("{RELEASES}/{}/{}", self.release, file.name)
    }

    /// Where a file of the pack is kept in the models folder `dir`.
    pub fn path(&self, dir: &Path, file: &ModelFile) -> PathBuf {
        dir.join(&file.name)
    }

    /// Every file is there, with the right size. (The content was checked when it was
    /// downloaded; reading 400 MB again at each start would be slow.)
    pub fn installed(&self, dir: &Path) -> bool {
        self.files.iter().all(|f| std::fs::metadata(self.path(dir, f)).is_ok_and(|m| m.is_file() && m.len() == f.size))
    }

    /// Remove the pack's files (also a download left half done). What was computed with them
    /// is kept.
    pub fn remove(&self, dir: &Path) -> Result<(), String> {
        for f in &self.files {
            let path = self.path(dir, f);
            for p in [crate::download::part_path(&path), path] {
                match std::fs::remove_file(&p) {
                    Err(e) if e.kind() != std::io::ErrorKind::NotFound => return Err(format!("{}: {e}", p.display())),
                    _ => {}
                }
            }
        }
        Ok(())
    }
}

impl Manifest {
    /// Read a manifest; packs that are not well formed are left out.
    pub fn parse(json: &str) -> Result<Manifest, String> {
        let mut m: Manifest = serde_json::from_str(json).map_err(|e| e.to_string())?;
        m.packs.retain(Pack::valid);
        Ok(m)
    }

    pub fn pack(&self, id: &str) -> Option<&Pack> {
        self.packs.iter().find(|p| p.id == id)
    }
}

/// The packs this version of the program knows.
pub fn manifest() -> &'static Manifest {
    static BUILT_IN: OnceLock<Manifest> = OnceLock::new();
    BUILT_IN.get_or_init(|| Manifest::parse(include_str!("../models.json")).unwrap_or_default())
}

/// Where downloaded networks are kept.
pub fn models_dir() -> Option<PathBuf> {
    Some(dirs::data_dir()?.join("AlbumCraft").join("models"))
}

#[cfg(test)]
mod tests {
    use super::*;

    pub(crate) const SAMPLE: &str = r#"{"packs": [
        {"id": "faces", "title": "Faces", "about": "Finds faces.", "licence": "Apache 2.0", "version": 1, "release": "models-1",
         "files": [{"name": "sface-1.onnx", "size": 5, "sha256": "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"}]},
        {"id": "elsewhere", "title": "T", "licence": "MIT", "version": 1, "release": "../../other/project",
         "files": [{"name": "a.onnx", "size": 5, "sha256": "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"}]},
        {"id": "outside", "title": "T", "licence": "MIT", "version": 1, "release": "models-1",
         "files": [{"name": "../../library.json", "size": 5, "sha256": "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"}]},
        {"id": "unchecked", "title": "T", "licence": "MIT", "version": 1, "release": "models-1",
         "files": [{"name": "a.onnx", "size": 5, "sha256": ""}]},
        {"id": "empty", "title": "T", "licence": "MIT", "version": 1, "release": "models-1", "files": []}
    ]}"#;

    /// The list that ships: it must read, with every pack well formed and nothing dropped.
    #[test]
    fn the_built_in_manifest_is_well_formed() {
        let raw: Manifest = serde_json::from_str(include_str!("../models.json")).unwrap();
        assert!(raw.packs.iter().all(Pack::valid));
        assert_eq!(raw.packs.len(), manifest().packs.len());
        let mut ids: Vec<&str> = raw.packs.iter().map(|p| p.id.as_str()).collect();
        let mut files: Vec<&str> = raw.packs.iter().flat_map(|p| &p.files).map(|f| f.name.as_str()).collect();
        let (n_ids, n_files) = (ids.len(), files.len());
        ids.sort_unstable();
        ids.dedup();
        files.sort_unstable();
        files.dedup();
        assert_eq!((ids.len(), files.len()), (n_ids, n_files), "pack and file names are used once");
    }

    #[test]
    fn packs_come_only_from_the_project_and_stay_in_their_folder() {
        let m = Manifest::parse(SAMPLE).unwrap();
        // the other four name another place, a file outside the folder, no checksum, no file
        assert_eq!(m.packs.len(), 1);
        let pack = m.pack("faces").unwrap();
        assert_eq!(pack.url(&pack.files[0]), "https://github.com/kbarni/AlbumCraft/releases/download/models-1/sface-1.onnx");
        assert_eq!(pack.bytes(), 5);
        assert!(Manifest::parse("not json").is_err());
        assert!(Manifest::parse("{}").unwrap().packs.is_empty());
    }

    #[test]
    fn installed_means_every_file_whole() {
        let dir = std::env::temp_dir().join(format!("albumcraft-ai-models-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let m = Manifest::parse(SAMPLE).unwrap();
        let pack = m.pack("faces").unwrap();
        assert!(!pack.installed(&dir));
        std::fs::write(dir.join("sface-1.onnx"), b"hel").unwrap();
        assert!(!pack.installed(&dir), "a file of the wrong size is not the network");
        std::fs::write(dir.join("sface-1.onnx"), b"hello").unwrap();
        std::fs::write(dir.join("sface-1.onnx.part"), b"he").unwrap();
        std::fs::write(dir.join("other.onnx"), b"x").unwrap();
        assert!(pack.installed(&dir));
        pack.remove(&dir).unwrap();
        pack.remove(&dir).unwrap();
        assert!(!pack.installed(&dir) && !dir.join("sface-1.onnx.part").exists() && dir.join("other.onnx").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
