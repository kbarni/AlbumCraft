//! Pictures that live on a server (an Immich server, see `immich.rs`), read through the same
//! functions as files on disk.
//!
//! A server picture's key is `immich://<connection>/<asset id>/<file name>`. The functions
//! that read pictures and what is known about them (`imaging::load_source`,
//! `imaging::oriented_dims`, `catalogue::read_details`, `exifinfo::read`, `geo::file_gps`, the
//! thumbnail cache) first ask the `Source` set here for such keys, so the grid, the editor,
//! export, print, collages and the rest work on server pictures without knowing they are not
//! files. Like `video::set_poster`, it is one table for the whole program: one server at a
//! time.

use std::sync::{Arc, PoisonError, RwLock};

use image::RgbaImage;

/// Keys of server pictures start with this.
pub const PREFIX: &str = "immich://";

pub fn is_remote(key: &str) -> bool {
    key.starts_with(PREFIX)
}

/// What a server can say about one of its pictures. Every answer may be None (unknown
/// picture, no network): callers treat it as an unreadable file.
pub trait Source: Send + Sync {
    /// The picture, upright, its longest side at most `max_dim` (0: full size).
    fn picture(&self, key: &str, max_dim: u32) -> Option<RgbaImage>;
    /// What identifies this version of the picture, for the thumbnail cache (as a file's
    /// modified time and size do).
    fn stamp(&self, key: &str) -> Option<String>;
    /// The size as shown (after the EXIF orientation).
    fn dims(&self, key: &str) -> Option<(u32, u32)>;
    /// Shape and date taken.
    fn details(&self, key: &str) -> Option<crate::catalogue::Details>;
    fn camera(&self, key: &str) -> Option<crate::exifinfo::CameraInfo>;
    fn gps(&self, key: &str) -> Option<crate::geo::LatLon>;
}

static SOURCE: RwLock<Option<Arc<dyn Source>>> = RwLock::new(None);

/// Set (or with None, remove) the server whose pictures are read.
pub fn set(source: Option<Arc<dyn Source>>) {
    *SOURCE.write().unwrap_or_else(PoisonError::into_inner) = source;
}

fn get() -> Option<Arc<dyn Source>> {
    SOURCE.read().unwrap_or_else(PoisonError::into_inner).clone()
}

/// `Some(answer)` for a server key (the answer itself None when the server can't give it),
/// None for any other key: the caller then reads the file as usual.
pub fn ask<T>(key: &str, f: impl FnOnce(&dyn Source) -> Option<T>) -> Option<Option<T>> {
    if !is_remote(key) {
        return None;
    }
    Some(get().and_then(|s| f(&*s)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_keys_are_not_asked() {
        assert!(ask("/home/a.jpg", |_| Some(1)).is_none());
        // a server key with no server set: known to be remote, nothing to read
        let r: Option<Option<u32>> = ask("immich://nowhere/1c27e4ea-29d7-451e-b777-3191930cda3b/a.jpg", |_| Some(1));
        assert!(matches!(r, Some(None) | Some(Some(_))));
    }
}
