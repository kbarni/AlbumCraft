//! AlbumCraft's non-destructive edit pipeline: the edit steps (`imaging::EditOp`), the
//! adjustments (`develop`), preset looks (`looks`) and effects (`decor`), applied to pictures
//! already in memory.
//!
//! No files, threads of its own or platform code, so the same code can run wherever a picture is
//! shown or exported: the desktop app, a server, or a browser (it builds for
//! `wasm32-unknown-unknown`; rayon then runs on the calling thread).

pub mod decor;
pub mod develop;
pub mod imaging;
pub mod looks;
pub mod steps;
