//! AlbumCraft's optional AI features, the part every one of them needs:
//!
//! - `runtime`: running a network file (ONNX Runtime, behind the `onnx` feature);
//! - `models`: the packs of networks the app can use, listed in a manifest built into it;
//! - `download`: fetching a pack, only when the user asked for it;
//! - `store`: what the networks found in each picture, in files that are only ever added to.
//!
//! Everything is computed on this computer. The only connection made is the download of a pack
//! from the project's own releases, and no picture, face or search text is ever sent.

pub mod download;
pub mod models;
pub mod runtime;
pub mod store;

/// The error of work the user stopped (not a failure to report as one).
pub const STOPPED: &str = "stopped";

/// One kind of analysis of a picture by one network: what the background analysis runs on every
/// picture of the library, once.
pub trait Analyser: Send + Sync {
    /// The file its results are kept in.
    fn table(&self) -> store::Spec;
    /// The longest side, in pixels, of the picture it wants to be given.
    fn input_dim(&self) -> u32;
    /// What it found in this picture (unedited, turned the right way up): one record per thing
    /// found (one vector for the whole picture, one record per face…), each `table().record_len`
    /// bytes long. No record means "looked at, nothing there".
    fn analyse(&self, picture: &image::RgbaImage) -> Result<Vec<Vec<u8>>, String>;
}
