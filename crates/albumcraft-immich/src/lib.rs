//! AlbumCraft's client for an Immich server (see `immich-integration.md` in the project notes).
//!
//! Immich is the photo server: it keeps the originals, makes thumbnails and previews and runs
//! its own apps. This crate is the HTTP side only: checking an address, signing in, reading
//! the sync stream, fetching thumbnails, previews and originals. It has no window code and no
//! notes of its own; `albumcraft-core` turns what it reads into the library.
//!
//! Rules kept here:
//! - every failure is an `Error` value, never a panic: the server and the network are input;
//! - the password is sent once to sign in and never kept; tokens and keys never appear in
//!   `Debug` output or error texts;
//! - nothing is written to the server's pictures in this version (download only); the only
//!   calls that change anything on the server are signing in and out and the sync stream's
//!   position (`ack`), which belongs to this app's own session.

pub mod address;
pub mod client;
pub mod sync;

pub use address::Address;
pub use client::{Auth, Client, Features, ServerInfo, SignedIn, Size, User, Version};

/// What went wrong talking to the server.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// The typed address can't be used (the reason, in English).
    Address(String),
    /// The server could not be reached (no network, wrong name, refused, timed out, TLS).
    Network(String),
    /// The server answered that the sign-in is wrong or no longer valid (401).
    Unauthorized,
    /// The server refused this call for this account or key (403).
    Forbidden(String),
    /// Something else came back: the status and Immich's message when it gave one.
    Http { status: u16, message: String },
    /// The answer was not what an Immich server sends (not Immich, or a version we can't read).
    Invalid(String),
    /// The server is an Immich version this AlbumCraft doesn't support.
    Unsupported(Version),
    /// The server doesn't allow signing in with a password (OAuth only): use an API key.
    NoPasswordLogin,
    /// Stopped by the caller.
    Stopped,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Address(why) => write!(f, "address can't be used: {why}"),
            Error::Network(why) => write!(f, "server not reachable: {why}"),
            Error::Unauthorized => write!(f, "the sign-in is wrong or has expired"),
            Error::Forbidden(why) => write!(f, "not allowed: {why}"),
            Error::Http { status, message } if message.is_empty() => write!(f, "the server answered {status}"),
            Error::Http { status, message } => write!(f, "the server answered {status}: {message}"),
            Error::Invalid(why) => write!(f, "unexpected answer from the server: {why}"),
            Error::Unsupported(v) => write!(f, "Immich {v} is not supported (needs {} or newer)", client::OLDEST_SUPPORTED),
            Error::NoPasswordLogin => write!(f, "this server signs in through OAuth only: use an API key"),
            Error::Stopped => write!(f, "stopped"),
        }
    }
}

impl std::error::Error for Error {}
