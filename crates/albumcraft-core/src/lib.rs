//! AlbumCraft without a window: the library (`library::Store`, folder scan, the background
//! picture loader), the command layer agents and the UI use (`commands`), the MCP server, and
//! everything that reads or writes picture files (loading, EXIF, raw previews, GPS, thumbnails,
//! export, web gallery, movie).
//!
//! The edit pipeline is `albumcraft-edit`; its modules are re-exported here, and `imaging` adds
//! reading pictures from files to it.

pub use albumcraft_edit::{decor, develop, looks, steps};

pub use albumcraft_ai as ai;

pub mod analysis;
pub mod catalogue;
pub mod commands;
pub mod control;
pub mod exifinfo;
pub mod export;
pub mod gallery;
pub mod geo;
pub mod imaging;
pub mod import;
#[cfg(test)]
mod perf_tests;
pub mod library;
pub mod locale;
pub mod mail;
pub mod mcp;
pub mod movie;
pub mod msg;
pub mod online;
pub mod collage;
pub mod print;
pub mod raw;
pub mod slides;
pub mod smart_search;
pub mod tag_proposals;
pub mod thumbcache;
pub mod vocabulary;
