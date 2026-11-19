//! The HTTP calls: server information, signing in and out, the sync stream, pictures.

use std::io::{Read, Write};
use std::time::Duration;

use serde::Deserialize;

use crate::{Address, Error};

/// The oldest Immich this version supports. Only the latest Immich is followed: when it changes
/// its API, this client changes with it.
pub const OLDEST_SUPPORTED: Version = Version { major: 3, minor: 4, patch: 0 };

/// Largest JSON answer read whole (lists of albums, tags, people): 64 MB.
const JSON_LIMIT: u64 = 64 << 20;
/// Largest thumbnail or preview: 32 MB (a preview is ~300 KB).
const IMAGE_LIMIT: u64 = 32 << 20;
/// Largest original: 4 GB (some panoramas and raw files are large).
const ORIGINAL_LIMIT: u64 = 4 << 30;
/// Longest line of the sync stream: 4 MB (a line is one asset, about 700 bytes).
const LINE_LIMIT: usize = 4 << 20;

/// How the client proves who it is.
#[derive(Clone, PartialEq, Eq)]
pub enum Auth {
    /// An API key made in Immich (or by AlbumCraft after a password sign-in): header `x-api-key`.
    ApiKey(String),
    /// The session token of a password sign-in: `Authorization: Bearer`.
    Session(String),
}

impl std::fmt::Debug for Auth {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // never print the secret
        match self {
            Auth::ApiKey(_) => write!(f, "ApiKey(…)"),
            Auth::Session(_) => write!(f, "Session(…)"),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
pub struct Version {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl std::fmt::Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// What the server can do (`GET /server/features`), the parts AlbumCraft uses.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Features {
    pub password_login: bool,
    pub oauth: bool,
    pub smart_search: bool,
    pub facial_recognition: bool,
    pub trash: bool,
}

/// The server, as checked before signing in.
#[derive(Clone, Debug)]
pub struct ServerInfo {
    pub version: Version,
    pub features: Features,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct User {
    pub id: String,
    pub email: String,
    pub name: String,
}

/// The answer to a password sign-in.
#[derive(Clone, Debug)]
pub struct SignedIn {
    pub auth: Auth,
    pub user: User,
}

/// The sizes Immich makes of each picture.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Size {
    /// ~250 px WebP, for the grid.
    Thumbnail,
    /// 1440 px JPEG (the server's setting), unedited: what the editor works on.
    Preview,
}

impl Size {
    fn query(self) -> &'static str {
        match self {
            Size::Thumbnail => "thumbnail",
            Size::Preview => "preview",
        }
    }
}

/// A connection to one server. Cheap to clone; usable from several threads (each clone shares
/// the connection pool).
#[derive(Clone)]
pub struct Client {
    address: Address,
    auth: Option<Auth>,
    /// Short calls: 30 s in all.
    quick: ureq::Agent,
    /// Streams and downloads: no limit in all, but each read must come within 60 s.
    long: ureq::Agent,
}

impl std::fmt::Debug for Client {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Client").field("address", &self.address.web).field("auth", &self.auth).finish()
    }
}

fn user_agent() -> String {
    format!("AlbumCraft/{} (+https://github.com/kbarni/AlbumCraft)", env!("CARGO_PKG_VERSION"))
}

impl Client {
    pub fn new(address: Address, auth: Option<Auth>) -> Client {
        let base = || {
            ureq::Agent::config_builder()
                .user_agent(user_agent())
                // statuses are read here, with Immich's message
                .http_status_as_error(false)
                .timeout_connect(Some(Duration::from_secs(10)))
                .max_redirects(3)
        };
        let quick = base().timeout_global(Some(Duration::from_secs(30))).build().into();
        let long = base().timeout_recv_response(Some(Duration::from_secs(60))).timeout_recv_body(None).timeout_global(None).build().into();
        Client { address, auth, quick, long }
    }

    pub fn address(&self) -> &Address {
        &self.address
    }

    pub fn auth(&self) -> Option<&Auth> {
        self.auth.as_ref()
    }

    pub fn with_auth(mut self, auth: Auth) -> Client {
        self.auth = Some(auth);
        self
    }

    fn url(&self, path: &str) -> String {
        format!("{}{path}", self.address.api())
    }

    fn authorize<B>(&self, req: ureq::RequestBuilder<B>) -> ureq::RequestBuilder<B> {
        match &self.auth {
            Some(Auth::ApiKey(k)) => req.header("x-api-key", k.as_str()),
            Some(Auth::Session(t)) => req.header("Authorization", format!("Bearer {t}")),
            None => req,
        }
    }

    fn get(&self, agent: &ureq::Agent, path: &str) -> Result<ureq::http::Response<ureq::Body>, Error> {
        let req = self.authorize(agent.get(self.url(path)).header("Accept", "application/json"));
        checked(req.call())
    }

    fn post(&self, agent: &ureq::Agent, path: &str, body: &serde_json::Value) -> Result<ureq::http::Response<ureq::Body>, Error> {
        let bytes = serde_json::to_vec(body).map_err(|e| Error::Invalid(e.to_string()))?;
        let req = self.authorize(agent.post(self.url(path)).header("Accept", "application/json").content_type("application/json"));
        checked(req.send(&bytes[..]))
    }

    fn get_json<T: serde::de::DeserializeOwned>(&self, path: &str) -> Result<T, Error> {
        let mut resp = self.get(&self.quick, path)?;
        json_of(&mut resp)
    }

    /// Check the server before asking for a password: is it Immich, a supported version, and
    /// how can one sign in. Needs no sign-in.
    pub fn server_info(&self) -> Result<ServerInfo, Error> {
        let version: Version = self.get_json("/server/version")?;
        if version < OLDEST_SUPPORTED {
            return Err(Error::Unsupported(version));
        }
        let features: Features = self.get_json("/server/features")?;
        Ok(ServerInfo { version, features })
    }

    /// Sign in with email and password. The password is sent once and not kept.
    pub fn login(&self, email: &str, password: &str) -> Result<SignedIn, Error> {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Answer {
            access_token: String,
            #[serde(default)]
            user_id: String,
            #[serde(default)]
            user_email: String,
            #[serde(default)]
            name: String,
        }
        let body = serde_json::json!({ "email": email, "password": password });
        // a wrong email or password is a 401: Error::Unauthorized
        let mut resp = self.post(&self.quick, "/auth/login", &body)?;
        let a: Answer = json_of(&mut resp)?;
        if a.access_token.is_empty() {
            return Err(Error::Invalid("no access token".into()));
        }
        Ok(SignedIn { auth: Auth::Session(a.access_token), user: User { id: a.user_id, email: a.user_email, name: a.name } })
    }

    /// End a password session on the server (the token stops working).
    pub fn logout(&self) -> Result<(), Error> {
        self.post(&self.quick, "/auth/logout", &serde_json::json!({})).map(|_| ())
    }

    /// Who is signed in; also the check that a key or token still works.
    pub fn me(&self) -> Result<User, Error> {
        self.get_json("/users/me")
    }

    /// A thumbnail or the unedited preview, as the encoded file (WebP or JPEG).
    pub fn picture(&self, asset: &str, size: Size) -> Result<Vec<u8>, Error> {
        check_id(asset)?;
        let mut resp = self.get(&self.quick_or_long(size), &format!("/assets/{asset}/thumbnail?size={}", size.query()))?;
        resp.body_mut().with_config().limit(IMAGE_LIMIT).read_to_vec().map_err(network)
    }

    fn quick_or_long(&self, size: Size) -> ureq::Agent {
        match size {
            Size::Thumbnail => self.quick.clone(),
            Size::Preview => self.long.clone(),
        }
    }

    /// Download the original file into `out`, calling `progress(done, total)` (total 0 when
    /// unknown); `progress` answering false stops it. Returns the number of bytes written.
    pub fn original(&self, asset: &str, out: &mut dyn Write, progress: &mut dyn FnMut(u64, u64) -> bool) -> Result<u64, Error> {
        check_id(asset)?;
        let mut resp = self.get(&self.long, &format!("/assets/{asset}/original"))?;
        let total = resp.headers().get("content-length").and_then(|v| v.to_str().ok()).and_then(|v| v.parse::<u64>().ok()).unwrap_or(0);
        let mut reader = resp.body_mut().with_config().limit(ORIGINAL_LIMIT).reader();
        let mut buf = vec![0u8; 256 << 10];
        let mut done = 0u64;
        loop {
            let n = reader.read(&mut buf).map_err(|e| Error::Network(e.to_string()))?;
            if n == 0 {
                break;
            }
            out.write_all(buf.get(..n).unwrap_or(&[])).map_err(|e| Error::Network(format!("writing: {e}")))?;
            done = done.saturating_add(n as u64);
            if !progress(done, total) {
                return Err(Error::Stopped);
            }
        }
        Ok(done)
    }

    /// Read the sync stream for these types, calling `each` with every line in order. `each`
    /// answering false stops reading (the server keeps the position of the last `ack`).
    pub fn sync_stream(&self, types: &[&str], reset: bool, each: &mut dyn FnMut(crate::sync::Line) -> bool) -> Result<(), Error> {
        let body = serde_json::json!({ "types": types, "reset": reset });
        let mut resp = self.post(&self.long, "/sync/stream", &body)?;
        let reader = resp.body_mut().with_config().limit(u64::MAX).reader();
        crate::sync::read_lines(reader, LINE_LIMIT, each)
    }

    /// Tell the server these lines are saved: the next stream starts after them. Send the last
    /// `ack` of each type.
    pub fn sync_ack(&self, acks: &[String]) -> Result<(), Error> {
        if acks.is_empty() {
            return Ok(());
        }
        self.post(&self.quick, "/sync/ack", &serde_json::json!({ "acks": acks })).map(|_| ())
    }

    /// The tags of the account (`GET /tags`).
    pub fn tags(&self) -> Result<Vec<crate::sync::Tag>, Error> {
        self.get_json("/tags")
    }

    /// The ids of the pictures with this tag, a page at a time (`POST /search/metadata`).
    /// Returns the ids and whether there is a next page.
    pub fn tagged(&self, tag: &str, page: u32) -> Result<(Vec<String>, bool), Error> {
        check_id(tag)?;
        #[derive(Deserialize)]
        struct Answer {
            assets: Page,
        }
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Page {
            #[serde(default)]
            items: Vec<Id>,
            #[serde(default)]
            next_page: Option<serde_json::Value>,
        }
        #[derive(Deserialize)]
        struct Id {
            id: String,
        }
        let body = serde_json::json!({ "tagIds": [tag], "page": page.max(1), "size": 1000, "withDeleted": false });
        let mut resp = self.post(&self.quick, "/search/metadata", &body)?;
        let a: Answer = json_of(&mut resp)?;
        let more = a.assets.next_page.is_some_and(|v| !v.is_null());
        Ok((a.assets.items.into_iter().map(|i| i.id).collect(), more))
    }
}

/// A UUID as Immich writes them; anything else is refused before it goes into a URL.
pub fn check_id(id: &str) -> Result<(), Error> {
    let ok = id.len() == 36
        && id.char_indices().all(|(i, c)| if matches!(i, 8 | 13 | 18 | 23) { c == '-' } else { c.is_ascii_hexdigit() });
    if ok { Ok(()) } else { Err(Error::Invalid(format!("not an id: {:.40}", id))) }
}

fn network(e: ureq::Error) -> Error {
    Error::Network(e.to_string())
}

/// Turn a status that isn't 2xx into an `Error`, with Immich's `message` when it gave one.
fn checked(r: Result<ureq::http::Response<ureq::Body>, ureq::Error>) -> Result<ureq::http::Response<ureq::Body>, Error> {
    let mut resp = r.map_err(network)?;
    let status = resp.status().as_u16();
    if (200..300).contains(&status) {
        return Ok(resp);
    }
    let text = resp.body_mut().with_config().limit(64 << 10).read_to_string().unwrap_or_default();
    let message = immich_message(&text);
    Err(match status {
        401 => Error::Unauthorized,
        403 => Error::Forbidden(message),
        _ => Error::Http { status, message },
    })
}

/// Immich's error answers are `{"message": "…" | ["…", …], …}`.
pub(crate) fn immich_message(text: &str) -> String {
    let Ok(v) = serde_json::from_str::<serde_json::Value>(text) else { return String::new() };
    let m = match v.get("message") {
        Some(serde_json::Value::String(s)) => s.clone(),
        Some(serde_json::Value::Array(a)) => a.iter().filter_map(|x| x.as_str()).collect::<Vec<_>>().join("; "),
        _ => String::new(),
    };
    let mut m: String = m.chars().filter(|c| !c.is_control()).collect();
    if let Some((i, _)) = m.char_indices().nth(300) {
        m.truncate(i);
    }
    m
}

fn json_of<T: serde::de::DeserializeOwned>(resp: &mut ureq::http::Response<ureq::Body>) -> Result<T, Error> {
    let bytes = resp.body_mut().with_config().limit(JSON_LIMIT).read_to_vec().map_err(network)?;
    serde_json::from_slice(&bytes).map_err(|e| Error::Invalid(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids() {
        assert!(check_id("1c27e4ea-29d7-451e-b777-3191930cda3b").is_ok());
        assert!(check_id("1c27e4ea-29d7-451e-b777-3191930cda3").is_err());
        assert!(check_id("../../server/config-aaaaaaaaaaaaaaaaa").is_err());
        assert!(check_id("1c27e4ea-29d7-451e-b777-3191930cda3g").is_err());
        assert!(check_id("").is_err());
    }

    #[test]
    fn messages() {
        assert_eq!(immich_message(r#"{"message":"Incorrect email or password","statusCode":401}"#), "Incorrect email or password");
        assert_eq!(immich_message(r#"{"message":["a","b"]}"#), "a; b");
        assert_eq!(immich_message("<html>"), "");
        assert_eq!(immich_message(&format!(r#"{{"message":"{}"}}"#, "é".repeat(500))).chars().count(), 300);
    }

    #[test]
    fn secrets_are_not_printed() {
        let c = Client::new(Address::parse("nas").unwrap_or(Address { web: "http://nas".into() }), Some(Auth::ApiKey("SECRET".into())));
        assert!(!format!("{c:?}").contains("SECRET"));
        assert!(!format!("{:?}", Auth::Session("SECRET".into())).contains("SECRET"));
    }

    #[test]
    fn versions_compare() {
        assert!(Version { major: 3, minor: 3, patch: 9 } < OLDEST_SUPPORTED);
        assert!(Version { major: 3, minor: 4, patch: 2 } >= OLDEST_SUPPORTED);
        assert!(Version { major: 4, minor: 0, patch: 0 } >= OLDEST_SUPPORTED);
    }

    /// Talks to the public demo server (read only). `cargo test -p albumcraft-immich immich_demo -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn immich_demo() {
        let Ok(address) = Address::parse("https://demo.immich.app") else { panic!("address") };
        let c = Client::new(address, None);
        let info = c.server_info().unwrap_or_else(|e| panic!("server info: {e}"));
        println!("Immich {} {:?}", info.version, info.features);
        assert!(info.features.password_login);
        let signed = c.login("demo@immich.app", "demo").unwrap_or_else(|e| panic!("login: {e}"));
        let c = c.with_auth(signed.auth.clone());
        let me = c.me().unwrap_or_else(|e| panic!("me: {e}"));
        println!("signed in as {}", me.name);
        let mut lib = crate::sync::Mirror::default();
        let mut n = 0usize;
        c.sync_stream(crate::sync::TYPES, false, &mut |line| {
            n += 1;
            lib.apply(line);
            true
        })
        .unwrap_or_else(|e| panic!("sync: {e}"));
        println!("{n} lines: {} pictures, {} albums, {} people", lib.assets.len(), lib.albums.len(), lib.people.len());
        assert!(lib.assets.len() > 100);
        let id = lib.assets.keys().next().cloned().unwrap_or_default();
        let thumb = c.picture(&id, Size::Thumbnail).unwrap_or_else(|e| panic!("thumbnail: {e}"));
        assert!(thumb.len() > 100);
        let preview = c.picture(&id, Size::Preview).unwrap_or_else(|e| panic!("preview: {e}"));
        assert!(preview.starts_with(&[0xff, 0xd8]));
        let tags = c.tags().unwrap_or_else(|e| panic!("tags: {e}"));
        println!("{} tags", tags.len());
        if let Some(t) = tags.first() {
            let (ids, _) = c.tagged(&t.id, 1).unwrap_or_else(|e| panic!("tagged: {e}"));
            println!("tag {}: {} pictures on the first page", t.value, ids.len());
        }
        let _ = c.logout();
    }
}
