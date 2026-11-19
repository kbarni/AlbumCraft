//! An Immich server as AlbumCraft's library (download only in this version).
//!
//! `Engine` keeps a mirror of the account's library (`albumcraft_immich::sync::Mirror`, read
//! from the sync stream and saved in `<data dir>/AlbumCraft/immich/<connection>/`), answers for
//! its pictures through `remote::Source` (thumbnails, previews and originals fetched from the
//! server and cached in `<cache dir>/AlbumCraft/immich/<connection>/`), and turns the
//! server's notes into the window's: folders by month, ratings, favourites (picks), captions,
//! tags, people and albums.
//!
//! What the server says reaches the window as changes (`Changes`): only the fields that
//! changed on the server since they were last sent. A note changed in AlbumCraft stays as it
//! is until the server's own value changes (sending changes to the server is the next step).

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError, RwLock};
use std::time::{Duration, Instant};

use image::RgbaImage;
use serde::{Deserialize, Serialize};

pub use albumcraft_immich as api;
use api::sync::{Asset, Mirror};
use api::{Address, Auth, Client, Size};

use crate::library::{Album, Done, Flag, ScannedFolder};

/// The tag that stands for AlbumCraft's reject mark (Immich has none that keeps the picture).
pub const REJECT_TAG: &str = "albumcraft/rejected";

/// Seconds between two readings of the sync stream while the window is open.
const SYNC_EVERY: u64 = 30;
/// Seconds between two readings of the tags (one request per tag: slow, so seldom).
const TAGS_EVERY: u64 = 24 * 3600;
/// Pause between two tag requests, to stay gentle with the server.
const TAG_PAUSE: Duration = Duration::from_millis(200);
/// Size the cache of downloaded originals is kept under.
const ORIGINALS_MAX: u64 = 2 << 30;
/// Size the cache of previews is kept under.
const PREVIEWS_MAX: u64 = 1 << 30;
/// Lines applied to the mirror at a time (the lock is held only that long).
const BATCH: usize = 2000;

/// The server account the user signed in to, kept in the library's settings (no secret here).
#[derive(Serialize, Deserialize, Default, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct Account {
    /// As shown and typed, e.g. `https://demo.immich.app`.
    pub address: String,
    pub user_id: String,
    pub user_name: String,
    pub user_email: String,
}

impl Account {
    /// The short name of the connection, used in picture keys and folder names:
    /// `demo.immich.app`, `192.168.1.10_2283`.
    pub fn connection(&self) -> String {
        connection_of(&self.address)
    }
}

pub fn connection_of(address: &str) -> String {
    let rest = address.split_once("://").map(|(_, r)| r).unwrap_or(address);
    let mut s: String = rest.chars().map(|c| if c.is_ascii_alphanumeric() || matches!(c, '.' | '-') { c.to_ascii_lowercase() } else { '_' }).collect();
    while s.ends_with('_') {
        s.pop();
    }
    if s.is_empty() { "server".into() } else { s }
}

/// The folder of one account's notes and mirror: `<data dir>/AlbumCraft/immich/<connection>/<user>`
/// (another user of the same server has a library of their own).
fn data_dir(a: &Account) -> Option<PathBuf> {
    Some(dirs::data_dir()?.join("AlbumCraft").join("immich").join(a.connection()).join(user_folder(a)))
}

fn cache_dir(a: &Account) -> Option<PathBuf> {
    Some(dirs::cache_dir()?.join("AlbumCraft").join("immich").join(a.connection()))
}

fn user_folder(a: &Account) -> String {
    let s: String = a.user_id.chars().filter(|c| c.is_ascii_alphanumeric() || *c == '-').take(64).collect();
    if s.is_empty() { "user".into() } else { s }
}

// ------------------------------------------------------------------ secrets

#[derive(Serialize, Deserialize)]
struct Secret {
    kind: String,
    value: String,
}

/// Keep the session token or API key, readable only by the user (on Unix: mode 0600).
pub fn save_secret(account: &Account, auth: &Auth) -> Result<(), String> {
    save_secret_in(&data_dir(account).ok_or("no data folder")?, auth)
}

fn save_secret_in(dir: &Path, auth: &Auth) -> Result<(), String> {
    let s = match auth {
        Auth::ApiKey(k) => Secret { kind: "key".into(), value: k.clone() },
        Auth::Session(t) => Secret { kind: "session".into(), value: t.clone() },
    };
    let bytes = serde_json::to_vec(&s).map_err(|e| e.to_string())?;
    write_private(&dir.join("secret.json"), &bytes)
}

pub fn load_secret(account: &Account) -> Option<Auth> {
    load_secret_in(&data_dir(account)?)
}

fn load_secret_in(dir: &Path) -> Option<Auth> {
    let bytes = std::fs::read(dir.join("secret.json")).ok()?;
    let s: Secret = serde_json::from_slice(&bytes).ok()?;
    match s.kind.as_str() {
        "key" => Some(Auth::ApiKey(s.value)),
        "session" => Some(Auth::Session(s.value)),
        _ => None,
    }
}

pub fn forget_secret(account: &Account) {
    if let Some(d) = data_dir(account) {
        let _ = std::fs::remove_file(d.join("secret.json"));
    }
}

/// Write atomically, and on Unix readable by the user only.
fn write_private(path: &Path, bytes: &[u8]) -> Result<(), String> {
    use std::io::Write;
    let dir = path.parent().ok_or("no folder")?;
    std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let tmp = path.with_extension("tmp");
    let write = || -> std::io::Result<()> {
        let mut opts = std::fs::OpenOptions::new();
        opts.write(true).create(true).truncate(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            opts.mode(0o600);
        }
        let mut f = opts.open(&tmp)?;
        f.write_all(bytes)?;
        f.sync_all()?;
        std::fs::rename(&tmp, path)
    };
    write().map_err(|e| {
        let _ = std::fs::remove_file(&tmp);
        format!("{}: {e}", path.display())
    })
}

// ------------------------------------------------------------------ signing in

/// Check the server and sign in with a password (blocking: call off the window's thread).
/// The password is used for this one call only.
pub fn sign_in(typed_address: &str, email: &str, password: &str) -> Result<(Account, Auth), api::Error> {
    let address = Address::parse(typed_address)?;
    let client = Client::new(address.clone(), None);
    let info = client.server_info()?;
    if !info.features.password_login {
        return Err(api::Error::NoPasswordLogin);
    }
    let signed = client.login(email.trim(), password)?;
    let account = Account { address: address.web, user_id: signed.user.id, user_name: signed.user.name, user_email: signed.user.email };
    Ok((account, signed.auth))
}

/// Check the server and an API key (blocking).
pub fn sign_in_with_key(typed_address: &str, key: &str) -> Result<(Account, Auth), api::Error> {
    let address = Address::parse(typed_address)?;
    let auth = Auth::ApiKey(key.trim().to_string());
    let client = Client::new(address.clone(), Some(auth.clone()));
    client.server_info()?;
    let me = client.me()?;
    Ok((Account { address: address.web, user_id: me.id, user_name: me.name, user_email: me.email }, auth))
}

/// End the session on the server (a pasted API key is left as it is: the user made it).
pub fn sign_out(account: &Account) {
    if let (Ok(address), Some(auth @ Auth::Session(_))) = (Address::parse(&account.address), load_secret(account)) {
        let _ = Client::new(address, Some(auth)).logout();
    }
    forget_secret(account);
}

// ------------------------------------------------------------------ notes sent to the window

/// The notes the server holds for a picture, as AlbumCraft names them.
#[derive(Serialize, Deserialize, Default, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct ServerNote {
    pub flag: Flag,
    pub rating: u8,
    pub caption: String,
    pub tags: Vec<String>,
    pub people: Vec<String>,
}

/// What changed on the server for one picture: only the fields given.
#[derive(Default, Clone, Debug, PartialEq)]
pub struct NoteChange {
    pub flag: Option<Flag>,
    pub rating: Option<u8>,
    pub caption: Option<String>,
    pub tags: Option<Vec<String>>,
    pub people: Option<Vec<String>>,
}

impl NoteChange {
    fn between(old: Option<&ServerNote>, new: &ServerNote) -> Option<NoteChange> {
        let d = ServerNote::default();
        let old = old.unwrap_or(&d);
        let c = NoteChange {
            flag: (old.flag != new.flag).then_some(new.flag),
            rating: (old.rating != new.rating).then_some(new.rating),
            caption: (old.caption != new.caption).then(|| new.caption.clone()),
            tags: (old.tags != new.tags).then(|| new.tags.clone()),
            people: (old.people != new.people).then(|| new.people.clone()),
        };
        (c != NoteChange::default()).then_some(c)
    }

    /// Put the changed fields into the window's notes of the picture.
    pub fn apply(&self, m: &mut crate::library::Meta) {
        if let Some(f) = self.flag {
            m.flag = f;
        }
        if let Some(r) = self.rating {
            m.rating = r.min(crate::library::MAX_RATING);
        }
        if let Some(c) = &self.caption {
            m.caption = c.clone();
        }
        if let Some(t) = &self.tags {
            m.tags = t.clone();
        }
        if let Some(p) = &self.people {
            m.people = p.clone();
        }
    }
}

/// An album as the server has it now; `was` is the name it had when last sent (to find it in
/// the window when it was renamed).
#[derive(Clone, Debug)]
pub struct AlbumChange {
    pub was: Option<String>,
    pub album: Album,
}

/// The server's changes since they were last sent to the window.
#[derive(Default, Debug)]
pub struct Changes {
    pub notes: Vec<(String, NoteChange)>,
    pub albums: Vec<AlbumChange>,
    /// Names of albums removed on the server.
    pub removed_albums: Vec<String>,
    /// The tag hierarchy (tag, the tag it is under) as the server's tag paths give it.
    pub tag_parents: Vec<(String, String)>,
}

impl Changes {
    pub fn is_empty(&self) -> bool {
        self.notes.is_empty() && self.albums.is_empty() && self.removed_albums.is_empty() && self.tag_parents.is_empty()
    }
}

/// What was sent to the window last (saved, so a restart sends only what changed since).
#[derive(Serialize, Deserialize, Default)]
#[serde(default)]
struct Sent {
    notes: BTreeMap<String, ServerNote>,
    /// album id → (name, keys)
    albums: BTreeMap<String, (String, Vec<String>)>,
    tag_parents: BTreeMap<String, String>,
    /// Unix seconds of the last complete reading of the tags.
    tags_read: i64,
}

// ------------------------------------------------------------------ status

/// How the connection is doing, for the status bar.
#[derive(Clone, Debug, Default)]
pub struct Status {
    pub syncing: bool,
    /// The last sync worked (false also before the first one).
    pub online: bool,
    pub error: Option<String>,
    /// The sign-in is no longer valid: the user must sign in again.
    pub signed_out: bool,
    pub pictures: usize,
    /// Tags read so far / to read, while they are being read.
    pub tags: Option<(usize, usize)>,
}

// ------------------------------------------------------------------ the engine

pub struct Engine {
    pub account: Account,
    conn: String,
    client: Client,
    mirror: RwLock<Mirror>,
    sent: Mutex<Sent>,
    status: Mutex<Status>,
    /// Sync now rather than at the next tick (the window asked for a rescan).
    poke: AtomicBool,
    data: Option<PathBuf>,
    cache: Option<PathBuf>,
    /// One original downloaded at a time per picture.
    downloading: Mutex<BTreeSet<String>>,
}

impl Engine {
    /// The engine for a signed-in account: the mirror as last saved (works offline).
    pub fn open(account: Account) -> Result<Arc<Engine>, String> {
        let (data, cache) = (data_dir(&account), cache_dir(&account));
        Self::open_in(account, data, cache)
    }

    /// `open` with its folders given (None: nothing saved or cached; the tests).
    fn open_in(account: Account, data: Option<PathBuf>, cache: Option<PathBuf>) -> Result<Arc<Engine>, String> {
        let conn = account.connection();
        let address = Address::parse(&account.address).map_err(|e| e.to_string())?;
        let auth = data.as_deref().and_then(load_secret_in);
        let mirror = data.as_ref().and_then(|d| read_json::<Mirror>(&d.join("mirror.json"))).unwrap_or_default();
        let sent = data.as_ref().and_then(|d| read_json::<Sent>(&d.join("sent.json"))).unwrap_or_default();
        let status = Status { signed_out: auth.is_none(), pictures: mirror.assets.values().filter(|a| a.shown()).count(), ..Status::default() };
        Ok(Arc::new(Engine {
            account,
            client: Client::new(address, auth),
            cache,
            conn,
            mirror: RwLock::new(mirror),
            sent: Mutex::new(sent),
            status: Mutex::new(status),
            poke: AtomicBool::new(true),
            data,
            downloading: Mutex::new(BTreeSet::new()),
        }))
    }

    pub fn connection(&self) -> &str {
        &self.conn
    }

    /// Where the window's notes for this server are kept.
    pub fn notes_path(&self) -> Option<PathBuf> {
        Some(self.data.as_ref()?.join("library.json"))
    }

    /// The folder that stands for the whole server (the library's one root).
    pub fn root(&self) -> String {
        format!("{}{}", crate::remote::PREFIX, self.conn)
    }

    pub fn status(&self) -> Status {
        self.status.lock().unwrap_or_else(PoisonError::into_inner).clone()
    }

    fn set_status(&self, f: impl FnOnce(&mut Status)) {
        f(&mut self.status.lock().unwrap_or_else(PoisonError::into_inner));
    }

    /// Sync as soon as possible.
    pub fn poke(&self) {
        self.poke.store(true, Ordering::Relaxed);
    }

    /// The key of a picture: `immich://<connection>/<asset id>/<file name>`.
    pub fn key_of(&self, a: &Asset) -> String {
        let name: String = a.original_file_name.chars().map(|c| if matches!(c, '/' | '\\') || c.is_control() { '_' } else { c }).collect();
        let name = if name.is_empty() { "picture".to_string() } else { name };
        format!("{}{}/{}/{name}", crate::remote::PREFIX, self.conn, a.id)
    }

    /// The asset id in one of this server's keys.
    pub fn asset_of<'k>(&self, key: &'k str) -> Option<&'k str> {
        let rest = key.strip_prefix(crate::remote::PREFIX)?.strip_prefix(self.conn.as_str())?.strip_prefix('/')?;
        let id = rest.split('/').next()?;
        api::client::check_id(id).ok()?;
        Some(id)
    }

    /// The pictures as folders by month (newest first is the window's business), and what is
    /// known about each (shape, date taken), so the grid is laid out at once.
    pub fn folders(&self) -> (Vec<ScannedFolder>, crate::catalogue::KnownDetails) {
        let m = self.mirror.read().unwrap_or_else(PoisonError::into_inner);
        let mut months: BTreeMap<String, Vec<(String, i64, u64)>> = BTreeMap::new();
        let mut known = vec![];
        for a in m.assets.values().filter(|a| a.shown()) {
            let exif = m.exif.get(&a.id);
            let taken = taken_of(a);
            let month = a.local_date_time.as_deref().or(a.file_created_at.as_deref()).and_then(|t| t.get(..7)).filter(|t| t.len() == 7).unwrap_or("0000-00");
            let key = self.key_of(a);
            let bytes = exif.and_then(|e| e.file_size_in_byte).unwrap_or(0);
            known.push((key.clone(), crate::catalogue::Details { dims: dims_of(a, exif), taken, secs: None }));
            months.entry(format!("{}/{month}", self.root())).or_default().push((key, taken.unwrap_or(0), bytes));
        }
        (months.into_iter().collect(), known)
    }

    /// The server's notes of every shown picture, as AlbumCraft names them.
    fn server_notes(&self, m: &Mirror) -> BTreeMap<String, ServerNote> {
        // tags by picture, leaving out a tag when a tag under it is there too (a search by a
        // tag may also find the pictures of the tags under it)
        let mut by_asset: HashMap<&str, Vec<&str>> = HashMap::new();
        for (tag, ids) in &m.tagged {
            let Some(t) = m.tags.get(tag) else { continue };
            for id in ids {
                by_asset.entry(id.as_str()).or_default().push(t.value.as_str());
            }
        }
        // the named people on each picture, from its visible faces (one pass over the faces)
        let mut people: HashMap<&str, Vec<String>> = HashMap::new();
        for f in m.faces.values().filter(|f| f.deleted_at.is_none() && f.is_visible) {
            if let Some(p) = f.person_id.as_ref().and_then(|p| m.people.get(p)).filter(|p| !p.name.is_empty() && !p.is_hidden) {
                people.entry(f.asset_id.as_str()).or_default().push(p.name.clone());
            }
        }
        for names in people.values_mut() {
            names.sort();
            names.dedup();
        }
        m.assets
            .values()
            .filter(|a| a.shown())
            .map(|a| {
                let exif = m.exif.get(&a.id);
                let paths = by_asset.get(a.id.as_str()).cloned().unwrap_or_default();
                let rejected = paths.iter().any(|p| p.eq_ignore_ascii_case(REJECT_TAG));
                let mut tags: Vec<String> = paths
                    .iter()
                    .filter(|p| !p.eq_ignore_ascii_case(REJECT_TAG))
                    .filter(|p| !paths.iter().any(|q| q.len() > p.len() && q.starts_with(*p) && q.as_bytes().get(p.len()) == Some(&b'/')))
                    .map(|p| leaf(p).to_string())
                    .filter(|t| !t.is_empty())
                    .collect();
                tags.sort();
                tags.dedup();
                let note = ServerNote {
                    flag: if rejected { Flag::Reject } else if a.is_favorite { Flag::Pick } else { Flag::None },
                    rating: exif.and_then(|e| e.rating).map(|r| r.clamp(0, 5) as u8).unwrap_or(0),
                    caption: exif.and_then(|e| e.description.clone()).unwrap_or_default(),
                    tags,
                    people: people.get(a.id.as_str()).cloned().unwrap_or_default(),
                };
                (self.key_of(a), note)
            })
            .collect()
    }

    fn server_albums(&self, m: &Mirror) -> BTreeMap<String, (String, Vec<String>)> {
        let mut keys: BTreeMap<&str, Vec<String>> = BTreeMap::new();
        for (album, asset) in &m.album_assets {
            if let Some(a) = m.assets.get(asset).filter(|a| a.shown()) {
                keys.entry(album.as_str()).or_default().push(self.key_of(a));
            }
        }
        m.albums.values().map(|al| (al.id.clone(), (al.name.clone(), keys.remove(al.id.as_str()).unwrap_or_default()))).collect()
    }

    fn server_tag_parents(m: &Mirror) -> BTreeMap<String, String> {
        let mut out = BTreeMap::new();
        for t in m.tags.values() {
            let parts: Vec<&str> = t.value.split('/').filter(|p| !p.is_empty()).collect();
            if t.value.eq_ignore_ascii_case(REJECT_TAG) || parts.first().is_some_and(|p| p.eq_ignore_ascii_case("albumcraft")) {
                continue;
            }
            for w in parts.windows(2) {
                if let [parent, child] = w {
                    out.insert(child.to_string(), parent.to_string());
                }
            }
        }
        out
    }

    /// What changed on the server since the last call (everything, the first time). Saved as
    /// sent once taken.
    pub fn take_changes(&self) -> Changes {
        let m = self.mirror.read().unwrap_or_else(PoisonError::into_inner);
        let notes = self.server_notes(&m);
        let albums = self.server_albums(&m);
        let parents = Self::server_tag_parents(&m);
        drop(m);
        let mut sent = self.sent.lock().unwrap_or_else(PoisonError::into_inner);
        let mut ch = Changes::default();
        for (key, n) in &notes {
            if let Some(c) = NoteChange::between(sent.notes.get(key), n) {
                ch.notes.push((key.clone(), c));
            }
        }
        for (id, (name, keys)) in &albums {
            let old = sent.albums.get(id);
            if old != Some(&(name.clone(), keys.clone())) {
                ch.albums.push(AlbumChange { was: old.map(|o| o.0.clone()), album: Album { name: name.clone(), keys: keys.clone() } });
            }
        }
        for (id, (name, _)) in &sent.albums {
            if !albums.contains_key(id) {
                ch.removed_albums.push(name.clone());
            }
        }
        for (child, parent) in &parents {
            if sent.tag_parents.get(child) != Some(parent) {
                ch.tag_parents.push((child.clone(), parent.clone()));
            }
        }
        if !ch.is_empty() {
            sent.notes = notes;
            sent.albums = albums;
            sent.tag_parents = parents;
            self.save_sent(&sent);
        }
        ch
    }

    fn save_sent(&self, sent: &Sent) {
        if let (Some(d), Ok(bytes)) = (&self.data, serde_json::to_vec(sent)) {
            let _ = crate::library::write_library(&d.join("sent.json"), &bytes);
        }
    }

    fn save_mirror(&self) -> Result<(), String> {
        let d = self.data.as_ref().ok_or("no data folder")?;
        let bytes = {
            let m = self.mirror.read().unwrap_or_else(PoisonError::into_inner);
            serde_json::to_vec(&*m).map_err(|e| e.to_string())?
        };
        crate::library::write_library(&d.join("mirror.json"), &bytes)
    }

    /// Read the sync stream to its end, save the mirror, then acknowledge. Returns whether
    /// anything changed.
    pub fn sync(&self, stop: &AtomicBool) -> Result<bool, api::Error> {
        if self.client.auth().is_none() {
            return Err(api::Error::Unauthorized);
        }
        let mut changed = false;
        for _ in 0..2 {
            let reset = self.mirror.read().unwrap_or_else(PoisonError::into_inner).reset_asked;
            if reset {
                self.mirror.write().unwrap_or_else(PoisonError::into_inner).clear();
                changed = true;
            }
            let mut batch = Vec::with_capacity(BATCH);
            let mut apply = |batch: &mut Vec<api::sync::Line>| {
                let mut m = self.mirror.write().unwrap_or_else(PoisonError::into_inner);
                for line in batch.drain(..) {
                    if line.kind != "SyncCompleteV1" {
                        changed = true;
                    }
                    m.apply(line);
                }
            };
            self.client.sync_stream(api::sync::TYPES, reset, &mut |line| {
                batch.push(line);
                if batch.len() >= BATCH {
                    apply(&mut batch);
                }
                !stop.load(Ordering::Relaxed)
            })?;
            apply(&mut batch);
            if !self.mirror.read().unwrap_or_else(PoisonError::into_inner).reset_asked {
                break;
            }
        }
        if changed {
            self.save_mirror().map_err(api::Error::Invalid)?;
        }
        let acks = self.mirror.read().unwrap_or_else(PoisonError::into_inner).pending_acks();
        self.client.sync_ack(&acks)?;
        Ok(changed)
    }

    /// Read the tags and their pictures again (one request per tag and page). Returns whether
    /// anything changed.
    pub fn read_tags(&self, stop: &AtomicBool) -> Result<bool, api::Error> {
        let tags = self.client.tags()?;
        let total = tags.len();
        let mut tagged: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        for (i, t) in tags.iter().enumerate() {
            self.set_status(|s| s.tags = Some((i, total)));
            let mut page = 1;
            loop {
                if stop.load(Ordering::Relaxed) {
                    self.set_status(|s| s.tags = None);
                    return Err(api::Error::Stopped);
                }
                let (ids, more) = self.client.tagged(&t.id, page)?;
                if !ids.is_empty() {
                    tagged.entry(t.id.clone()).or_default().extend(ids);
                }
                std::thread::sleep(TAG_PAUSE);
                if !more || page >= 1000 {
                    break;
                }
                page += 1;
            }
        }
        self.set_status(|s| s.tags = None);
        let changed = {
            let mut m = self.mirror.write().unwrap_or_else(PoisonError::into_inner);
            let new_tags: BTreeMap<String, api::sync::Tag> = tags.iter().map(|t| (t.id.clone(), t.clone())).collect();
            let changed = m.tagged != tagged || m.tags != new_tags;
            m.set_tags(tags, tagged);
            changed
        };
        if changed {
            self.save_mirror().map_err(api::Error::Invalid)?;
        }
        let mut sent = self.sent.lock().unwrap_or_else(PoisonError::into_inner);
        sent.tags_read = now_secs();
        self.save_sent(&sent);
        Ok(changed)
    }

    fn tags_due(&self) -> bool {
        let read = self.sent.lock().unwrap_or_else(PoisonError::into_inner).tags_read;
        now_secs().saturating_sub(read) >= TAGS_EVERY as i64
    }

    /// The background loop: sync now and every `SYNC_EVERY` seconds (or when poked), read the
    /// tags when due, and send the window what changed. Ends when `stop` is set.
    pub fn run(self: Arc<Self>, send: Arc<dyn Fn(Done) + Send + Sync>, stop: Arc<AtomicBool>) {
        std::thread::spawn(move || {
            let mut last = Instant::now() - Duration::from_secs(SYNC_EVERY);
            while !stop.load(Ordering::Relaxed) {
                let poked = self.poke.swap(false, Ordering::Relaxed);
                if poked || last.elapsed() >= Duration::from_secs(SYNC_EVERY) {
                    last = Instant::now();
                    self.set_status(|s| s.syncing = true);
                    let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| self.sync(&stop))).unwrap_or_else(|_| Err(api::Error::Invalid("the sync stopped unexpectedly".into())));
                    let ok = r.is_ok();
                    let changed = matches!(r, Ok(true));
                    self.set_status(|s| {
                        s.syncing = false;
                        s.online = ok;
                        s.signed_out = matches!(r, Err(api::Error::Unauthorized));
                        s.error = r.as_ref().err().filter(|e| **e != api::Error::Stopped).map(|e| e.to_string());
                    });
                    if changed || poked {
                        self.send_state(&*send);
                    }
                    if ok && self.tags_due() {
                        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| self.read_tags(&stop))).unwrap_or(Ok(false));
                        if matches!(r, Ok(true)) {
                            self.send_state(&*send);
                        }
                    }
                    (send)(Done::RemoteStatus);
                }
                std::thread::sleep(Duration::from_millis(500));
            }
        });
    }

    /// Send the window the pictures and what changed in their notes.
    fn send_state(&self, send: &(dyn Fn(Done) + Send + Sync)) {
        let (folders, _) = self.folders();
        let pictures = folders.iter().map(|(_, f)| f.len()).sum();
        self.set_status(|s| s.pictures = pictures);
        send(Done::Scan(folders, false, vec![]));
        let ch = self.take_changes();
        if !ch.is_empty() {
            send(Done::ServerNotes(Box::new(ch)));
        }
    }

    fn asset(&self, key: &str) -> Option<(Asset, Option<api::sync::Exif>)> {
        let id = self.asset_of(key)?;
        let m = self.mirror.read().unwrap_or_else(PoisonError::into_inner);
        let a = m.assets.get(id)?.clone();
        let e = m.exif.get(id).cloned();
        Some((a, e))
    }

    /// The original file, downloaded into the cache once.
    pub fn original(&self, key: &str) -> Option<PathBuf> {
        let (a, _) = self.asset(key)?;
        let dir = self.cache.as_ref()?.join("originals").join(&a.id);
        let name = key.rsplit('/').next().filter(|n| !n.is_empty() && *n != "." && *n != "..")?;
        let path = dir.join(name);
        if path.is_file() {
            touch(&path);
            return Some(path);
        }
        // one download per picture: others wait for it
        loop {
            let mut d = self.downloading.lock().unwrap_or_else(PoisonError::into_inner);
            if !d.contains(&a.id) {
                d.insert(a.id.clone());
                break;
            }
            drop(d);
            std::thread::sleep(Duration::from_millis(100));
            if path.is_file() {
                return Some(path);
            }
        }
        let r = (|| {
            std::fs::create_dir_all(&dir).ok()?;
            let part = dir.join(format!("{name}.part"));
            let mut f = std::fs::File::create(&part).ok()?;
            let ok = self.client.original(&a.id, &mut f, &mut |_, _| true).is_ok() && f.sync_all().is_ok();
            drop(f);
            if ok && std::fs::rename(&part, &path).is_ok() {
                Some(path.clone())
            } else {
                let _ = std::fs::remove_file(&part);
                None
            }
        })();
        self.downloading.lock().unwrap_or_else(PoisonError::into_inner).remove(&a.id);
        if r.is_some()
            && let Some(c) = &self.cache
        {
            prune(&c.join("originals"), ORIGINALS_MAX);
        }
        r
    }

    fn preview_bytes(&self, a: &Asset) -> Option<Vec<u8>> {
        let path = self.cache.as_ref().map(|c| c.join("previews").join(format!("{}-{}.jpg", a.id, short_hash(&a.checksum))));
        if let Some(p) = &path
            && let Ok(b) = std::fs::read(p)
        {
            touch(p);
            return Some(b);
        }
        let bytes = self.client.picture(&a.id, Size::Preview).ok()?;
        if let Some(p) = &path
            && let Some(dir) = p.parent()
            && std::fs::create_dir_all(dir).is_ok()
        {
            let _ = crate::library::write_library(p, &bytes);
            if let Some(c) = &self.cache {
                prune(&c.join("previews"), PREVIEWS_MAX);
            }
        }
        Some(bytes)
    }
}

/// Immich's preview's longest side (its default setting): asked for anything up to this size.
const PREVIEW_DIM: u32 = 1440;

impl crate::remote::Source for Engine {
    fn picture(&self, key: &str, max_dim: u32) -> Option<RgbaImage> {
        let (a, _) = self.asset(key)?;
        let decoded = if max_dim > 0 && max_dim <= crate::library::THUMB_DIM {
            self.client.picture(&a.id, Size::Thumbnail).ok().and_then(|b| image::load_from_memory(&b).ok())
        } else {
            None
        };
        let decoded = match decoded {
            Some(d) => Some(d),
            None if max_dim > 0 && max_dim <= PREVIEW_DIM => self.preview_bytes(&a).and_then(|b| image::load_from_memory(&b).ok()),
            None => {
                // full size or a zoom: the original (any format AlbumCraft reads, raw included)
                if let Some(path) = self.original(key)
                    && let Some(img) = crate::imaging::load_source(&path.to_string_lossy(), max_dim)
                {
                    return Some(img);
                }
                self.preview_bytes(&a).and_then(|b| image::load_from_memory(&b).ok())
            }
        }?;
        let img = if max_dim > 0 && (decoded.width() > max_dim || decoded.height() > max_dim) { decoded.thumbnail(max_dim, max_dim) } else { decoded };
        Some(img.to_rgba8())
    }

    fn stamp(&self, key: &str) -> Option<String> {
        let (a, _) = self.asset(key)?;
        Some(a.checksum)
    }

    fn dims(&self, key: &str) -> Option<(u32, u32)> {
        let (a, e) = self.asset(key)?;
        dims_of(&a, e.as_ref())
    }

    fn details(&self, key: &str) -> Option<crate::catalogue::Details> {
        let (a, e) = self.asset(key)?;
        Some(crate::catalogue::Details { dims: dims_of(&a, e.as_ref()), taken: taken_of(&a), secs: None })
    }

    fn camera(&self, key: &str) -> Option<crate::exifinfo::CameraInfo> {
        let (a, e) = self.asset(key)?;
        let e = e?;
        let date = e.date_time_original.as_deref().or(a.local_date_time.as_deref()).and_then(exif_style_date);
        let mut all = vec![];
        let mut add = |name: &str, v: Option<String>| {
            if let Some(v) = v.filter(|v| !v.is_empty()) {
                all.push((name.to_string(), v));
            }
        };
        add("Make", e.make.clone());
        add("Model", e.model.clone());
        add("LensModel", e.lens_model.clone());
        add("DateTimeOriginal", date.clone());
        add("TimeZone", e.time_zone.clone());
        add("City", e.city.clone());
        add("State", e.state.clone());
        add("Country", e.country.clone());
        Some(crate::exifinfo::CameraInfo {
            make: e.make.clone(),
            model: e.model.clone(),
            lens: e.lens_model.clone(),
            date_taken: date,
            focal_mm: e.focal_length.filter(|f| f.is_finite() && *f > 0.0),
            exposure_s: e.exposure_time.as_deref().and_then(parse_exposure),
            fnumber: e.f_number.filter(|f| f.is_finite() && *f > 0.0),
            iso: e.iso.filter(|i| i.is_finite() && *i > 0.0 && *i < 1e7).map(|i| i as u32),
            orientation: e.orientation.as_deref().and_then(|o| o.parse().ok()),
            all,
            ..Default::default()
        })
    }

    fn gps(&self, key: &str) -> Option<crate::geo::LatLon> {
        let (_, e) = self.asset(key)?;
        let e = e?;
        crate::geo::LatLon::new(e.latitude?, e.longitude?)
    }
}

/// The last part of a tag path: `Places/France/Paris` → `Paris`.
fn leaf(path: &str) -> &str {
    path.rsplit('/').find(|p| !p.is_empty()).unwrap_or(path)
}

/// The size as shown: the file's size with its EXIF orientation (Immich's own `width` /
/// `height` are the edited size when it cropped the picture).
fn dims_of(a: &Asset, e: Option<&api::sync::Exif>) -> Option<(u32, u32)> {
    let from_exif = e.and_then(|e| {
        let (w, h) = (e.exif_image_width?, e.exif_image_height?);
        let turned = e.orientation.as_deref().and_then(|o| o.parse::<u32>().ok()).is_some_and(|o| (5..=8).contains(&o));
        Some(if turned { (h, w) } else { (w, h) })
    });
    let d = if a.is_edited { from_exif } else { from_exif.or(a.width.zip(a.height)) };
    d.filter(|(w, h)| *w > 0 && *h > 0)
}

/// When it was taken, as the library counts it (the place's local time read as this
/// computer's local time, like EXIF dates).
fn taken_of(a: &Asset) -> Option<i64> {
    let t = a.local_date_time.as_deref().or(a.file_created_at.as_deref())?;
    let naive = naive_of(t)?;
    use chrono::TimeZone;
    chrono::Local.from_local_datetime(&naive).earliest().map(|d| d.timestamp())
}

fn naive_of(t: &str) -> Option<chrono::NaiveDateTime> {
    let head = t.get(..19)?;
    chrono::NaiveDateTime::parse_from_str(head, "%Y-%m-%dT%H:%M:%S").ok()
}

/// `2024-10-16T12:04:35.976Z` → `2024:10:16 12:04:35`, as EXIF writes dates.
fn exif_style_date(t: &str) -> Option<String> {
    Some(naive_of(t)?.format("%Y:%m:%d %H:%M:%S").to_string())
}

/// `1/100` or `0.5` → seconds.
fn parse_exposure(t: &str) -> Option<f64> {
    let v = match t.split_once('/') {
        Some((a, b)) => a.trim().parse::<f64>().ok()? / b.trim().parse::<f64>().ok().filter(|b| *b != 0.0)?,
        None => t.trim().parse().ok()?,
    };
    (v.is_finite() && v > 0.0).then_some(v)
}

fn short_hash(s: &str) -> String {
    format!("{:08x}", s.bytes().fold(0x811c9dc5u32, |h, b| (h ^ b as u32).wrapping_mul(0x01000193)))
}

fn now_secs() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0)
}

fn read_json<T: serde::de::DeserializeOwned>(path: &Path) -> Option<T> {
    serde_json::from_slice(&std::fs::read(path).ok()?).ok()
}

/// Mark a cached file as just used (for pruning, least recently used first).
fn touch(path: &Path) {
    if let Ok(f) = std::fs::File::options().append(true).open(path) {
        let _ = f.set_modified(std::time::SystemTime::now());
    }
}

/// Remove the least recently used files under `dir` until it is under 80 % of `max`.
fn prune(dir: &Path, max: u64) {
    let mut files = vec![];
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&d) else { continue };
        for e in rd.flatten() {
            let Ok(md) = e.metadata() else { continue };
            if md.is_dir() {
                stack.push(e.path());
            } else {
                files.push((md.modified().ok(), md.len(), e.path()));
            }
        }
    }
    let total: u64 = files.iter().map(|f| f.1).sum();
    if total <= max {
        return;
    }
    files.sort_by_key(|f| f.0);
    let mut left = total;
    for (_, len, path) in files {
        if left <= max / 10 * 8 {
            break;
        }
        if std::fs::remove_file(&path).is_ok() {
            left = left.saturating_sub(len);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn engine() -> Arc<Engine> {
        let account = Account { address: "https://photos.example.com".into(), ..Default::default() };
        Engine::open_in(account, None, None).unwrap_or_else(|e| panic!("{e}"))
    }

    fn line(kind: &str, data: serde_json::Value) -> api::sync::Line {
        api::sync::Line { kind: kind.into(), data, ack: format!("{kind}|1") }
    }

    const ID: &str = "1c27e4ea-29d7-451e-b777-3191930cda3b";
    const ID2: &str = "2c27e4ea-29d7-451e-b777-3191930cda3b";

    fn filled() -> Arc<Engine> {
        let e = engine();
        {
            let mut m = e.mirror.write().unwrap_or_else(PoisonError::into_inner);
            m.clear();
            m.apply(line("AssetV2", serde_json::json!({"id": ID, "type":"IMAGE","visibility":"timeline","originalFileName":"IMG/1.jpg","localDateTime":"2025-03-23T12:20:05.000Z","isFavorite":true,"width":4000,"height":3000,"checksum":"abc"})));
            m.apply(line("AssetExifV1", serde_json::json!({"assetId": ID, "description":"Beach","rating":4,"exifImageWidth":4000,"exifImageHeight":3000,"orientation":"6","latitude":45.0,"longitude":7.5,"exposureTime":"1/100","make":"Google"})));
            m.apply(line("AssetV2", serde_json::json!({"id": ID2, "type":"IMAGE","visibility":"timeline","originalFileName":"b.jpg","localDateTime":"2024-01-02T00:00:00.000Z","checksum":"def"})));
            m.apply(line("AlbumV2", serde_json::json!({"id":"al","name":"March"})));
            m.apply(line("AlbumToAssetV1", serde_json::json!({"albumId":"al","assetId": ID})));
            let tags = vec![
                api::sync::Tag { id: "t1".into(), name: "Places".into(), value: "Places".into(), parent_id: None },
                api::sync::Tag { id: "t2".into(), name: "Paris".into(), value: "Places/Paris".into(), parent_id: Some("t1".into()) },
                api::sync::Tag { id: "t3".into(), name: "rejected".into(), value: REJECT_TAG.into(), parent_id: None },
            ];
            m.set_tags(tags, [("t1".to_string(), [ID.to_string()].into()), ("t2".to_string(), [ID.to_string()].into()), ("t3".to_string(), [ID2.to_string()].into())].into());
        }
        *e.sent.lock().unwrap_or_else(PoisonError::into_inner) = Sent::default();
        e
    }

    #[test]
    fn keys() {
        let e = engine();
        assert_eq!(e.connection(), "photos.example.com");
        assert_eq!(connection_of("http://192.168.1.10:2283"), "192.168.1.10_2283");
        let a = Asset { id: ID.into(), original_file_name: "a/b\\c.jpg".into(), ..Default::default() };
        let key = e.key_of(&a);
        assert_eq!(key, format!("immich://photos.example.com/{ID}/a_b_c.jpg"));
        assert_eq!(e.asset_of(&key), Some(ID));
        assert_eq!(e.asset_of("immich://other.example.com/x/y.jpg"), None);
        assert_eq!(e.asset_of("immich://photos.example.com/../../etc/passwd"), None);
        assert_eq!(e.asset_of("/home/a.jpg"), None);
    }

    #[test]
    fn folders_and_details() {
        let e = filled();
        let (folders, known) = e.folders();
        let paths: Vec<&str> = folders.iter().map(|f| f.0.as_str()).collect();
        assert_eq!(paths, vec!["immich://photos.example.com/2024-01", "immich://photos.example.com/2025-03"]);
        let key = &folders[1].1[0].0;
        let d = known.iter().find(|(k, _)| k == key).map(|(_, d)| *d).unwrap_or_default();
        assert_eq!(d.dims, Some((3000, 4000)), "turned by the EXIF orientation");
        assert!(d.taken.is_some());
        use crate::remote::Source;
        assert_eq!(e.gps(key).map(|g| g.lat), Some(45.0));
        let cam = e.camera(key).unwrap_or_default();
        assert_eq!(cam.exposure_s, Some(0.01));
        assert_eq!(cam.date_taken.as_deref(), Some("2025:03:23 12:20:05"));
        assert_eq!(cam.make.as_deref(), Some("Google"));
    }

    #[test]
    fn changes_are_sent_once() {
        let e = filled();
        let ch = e.take_changes();
        let key1 = format!("immich://photos.example.com/{ID}/IMG_1.jpg");
        let key2 = format!("immich://photos.example.com/{ID2}/b.jpg");
        let n1 = ch.notes.iter().find(|(k, _)| *k == key1).map(|(_, c)| c.clone()).unwrap_or_default();
        assert_eq!(n1.flag, Some(Flag::Pick));
        assert_eq!(n1.rating, Some(4));
        assert_eq!(n1.caption.as_deref(), Some("Beach"));
        assert_eq!(n1.tags, Some(vec!["Paris".to_string()]), "a tag under another leaves the upper one out");
        let n2 = ch.notes.iter().find(|(k, _)| *k == key2).map(|(_, c)| c.clone()).unwrap_or_default();
        assert_eq!(n2.flag, Some(Flag::Reject), "the reject tag is the reject mark");
        assert_eq!(n2.tags, None, "and not a tag");
        assert_eq!(ch.albums.len(), 1);
        assert_eq!(ch.albums[0].album.keys, vec![key1.clone()]);
        assert_eq!(ch.tag_parents, vec![("Paris".to_string(), "Places".to_string())]);
        // nothing new: nothing sent
        assert!(e.take_changes().is_empty());
        // only the field that changed on the server
        e.mirror.write().unwrap_or_else(PoisonError::into_inner).apply(line("AssetExifV1", serde_json::json!({"assetId": ID, "description":"Beach","rating":2,"exifImageWidth":4000,"exifImageHeight":3000})));
        let ch = e.take_changes();
        assert_eq!(ch.notes, vec![(key1.clone(), NoteChange { rating: Some(2), ..Default::default() })]);
        let mut m = crate::library::Meta { rating: 5, caption: "mine".into(), ..Default::default() };
        ch.notes[0].1.apply(&mut m);
        assert_eq!((m.rating, m.caption.as_str()), (2, "mine"), "local caption kept");
        // album removed on the server
        e.mirror.write().unwrap_or_else(PoisonError::into_inner).apply(line("AlbumDeleteV1", serde_json::json!({"albumId":"al"})));
        assert_eq!(e.take_changes().removed_albums, vec!["March".to_string()]);
    }

    /// The engine against the public demo server, in a temporary folder (read only on the
    /// server). `cargo test -p albumcraft-core immich_engine_demo -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn immich_engine_demo() {
        use crate::remote::Source;
        let (account, auth) = sign_in("https://demo.immich.app", "demo@immich.app", "demo").unwrap_or_else(|e| panic!("sign in: {e}"));
        let tmp = std::env::temp_dir().join(format!("albumcraft-immich-test-{}", std::process::id()));
        let data = tmp.join("data");
        save_secret_in(&data, &auth).unwrap_or_else(|e| panic!("secret: {e}"));
        let e = Engine::open_in(account, Some(data.clone()), Some(tmp.join("cache"))).unwrap_or_else(|e| panic!("open: {e}"));
        let stop = AtomicBool::new(false);
        let t0 = Instant::now();
        let changed = e.sync(&stop).unwrap_or_else(|e| panic!("sync: {e}"));
        println!("sync: {:?}, changed {changed}", t0.elapsed());
        assert!(changed);
        let (folders, known) = e.folders();
        let pictures: usize = folders.iter().map(|f| f.1.len()).sum();
        println!("{} months, {pictures} pictures, {} with a shape", folders.len(), known.iter().filter(|k| k.1.dims.is_some()).count());
        assert!(pictures > 1000);
        let ch = e.take_changes();
        println!("{} notes, {} albums", ch.notes.len(), ch.albums.len());
        assert!(e.take_changes().is_empty());
        // again: nothing new (the acks were taken)
        let t0 = Instant::now();
        let again = e.sync(&stop).unwrap_or_else(|e| panic!("sync again: {e}"));
        println!("second sync: {:?}, changed {again}", t0.elapsed());
        // the mirror is saved and read back
        let e2 = Engine::open_in(e.account.clone(), Some(data.clone()), None).unwrap_or_else(|e| panic!("reopen: {e}"));
        assert_eq!(e2.folders().1.len(), known.len());
        // pictures: thumbnail, preview, original, through the remote source
        let key = folders.last().and_then(|f| f.1.first()).map(|f| f.0.clone()).unwrap_or_default();
        let t0 = Instant::now();
        let thumb = e.picture(&key, crate::library::THUMB_DIM).unwrap_or_else(|| panic!("thumbnail of {key}"));
        println!("thumbnail {}x{} in {:?}", thumb.width(), thumb.height(), t0.elapsed());
        let t0 = Instant::now();
        let big = e.picture(&key, crate::library::BIG_DIM).unwrap_or_else(|| panic!("preview"));
        println!("preview {}x{} in {:?}", big.width(), big.height(), t0.elapsed());
        assert!(big.width().max(big.height()) <= crate::library::BIG_DIM);
        let t0 = Instant::now();
        let full = e.picture(&key, 0).unwrap_or_else(|| panic!("original"));
        println!("original {}x{} in {:?}, dims {:?}", full.width(), full.height(), t0.elapsed(), e.dims(&key));
        let d = e.dims(&key).unwrap_or_default();
        assert_eq!((full.width(), full.height()), d, "the size as shown matches the original, upright");
        println!("camera: {:?}", e.camera(&key).map(|c| (c.make, c.model, c.date_taken)));
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn helpers() {
        assert_eq!(parse_exposure("1/0"), None);
        assert_eq!(parse_exposure("0.5"), Some(0.5));
        assert_eq!(parse_exposure("x"), None);
        assert_eq!(exif_style_date("bad"), None);
        assert_eq!(leaf("a/b/c"), "c");
        assert_eq!(leaf("a/"), "a");
    }
}
