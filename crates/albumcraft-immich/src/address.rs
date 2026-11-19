//! The server's address as the user types it, checked and turned into the API's base URL.
//!
//! Plain `http://` is accepted only for addresses on the local network: a password or an API
//! key must never cross the internet unencrypted.

use crate::Error;

/// A checked server address: `scheme://host[:port][/path]`, without a trailing `/` or `/api`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Address {
    /// What the user sees and what is saved, e.g. `https://photos.example.com`.
    pub web: String,
}

impl Address {
    /// Check what the user typed. Without a scheme, `https://` is assumed, or `http://` for an
    /// address on the local network (Immich's default `http://<nas>:2283`).
    pub fn parse(typed: &str) -> Result<Address, Error> {
        let t = typed.trim();
        if t.is_empty() {
            return Err(Error::Address("empty".into()));
        }
        if t.len() > 2048 || t.chars().any(|c| c.is_whitespace() || c.is_control()) {
            return Err(Error::Address("not an address".into()));
        }
        let (scheme, rest) = match t.split_once("://") {
            Some((s, r)) => (s.to_ascii_lowercase(), r),
            None => (String::new(), t),
        };
        let (authority, path) = match rest.find(['/', '?', '#']) {
            Some(i) => (rest.get(..i).unwrap_or(""), rest.get(i..).unwrap_or("")),
            None => (rest, ""),
        };
        if authority.contains('@') {
            return Err(Error::Address("a name or password in the address is not supported".into()));
        }
        let host = host_of(authority).ok_or_else(|| Error::Address("no server name".into()))?;
        let local = is_local(&host);
        let scheme = match scheme.as_str() {
            "" if local => "http",
            "" | "https" => "https",
            "http" if local => "http",
            "http" => return Err(Error::Address("http only on the local network: use https".into())),
            _ => return Err(Error::Address("only http and https".into())),
        };
        if path.contains(['?', '#']) {
            return Err(Error::Address("no ? or # in the address".into()));
        }
        let mut path = path.trim_end_matches('/').to_string();
        if path.ends_with("/api") {
            path.truncate(path.len() - 4);
        }
        Ok(Address { web: format!("{scheme}://{}{path}", authority.to_ascii_lowercase()) })
    }

    /// The base of every API call, e.g. `https://photos.example.com/api`.
    pub fn api(&self) -> String {
        format!("{}/api", self.web)
    }
}

/// The host name of `host[:port]` or `[v6]:port`, lowercased; None when there isn't one or
/// the port isn't a number.
fn host_of(authority: &str) -> Option<String> {
    let (host, port) = if let Some(v6) = authority.strip_prefix('[') {
        let (h, after) = v6.split_once(']')?;
        (h, after.strip_prefix(':'))
    } else {
        match authority.rsplit_once(':') {
            Some((h, p)) => (h, Some(p)),
            None => (authority, None),
        }
    };
    if let Some(p) = port
        && p.parse::<u16>().is_err()
    {
        return None;
    }
    if host.is_empty() || !host.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | ':' | '_')) {
        return None;
    }
    Some(host.to_ascii_lowercase())
}

/// Whether a host is on this computer or the local network: loopback and private IP ranges,
/// link-local, names without a dot (`nas`) and the `.local`, `.lan`, `.home.arpa` names.
pub fn is_local(host: &str) -> bool {
    if let Ok(ip) = host.parse::<std::net::Ipv4Addr>() {
        return ip.is_loopback() || ip.is_private() || ip.is_link_local();
    }
    if let Ok(ip) = host.parse::<std::net::Ipv6Addr>() {
        let first = ip.segments().first().copied().unwrap_or(0);
        return ip.is_loopback() || (first & 0xfe00) == 0xfc00 || (first & 0xffc0) == 0xfe80;
    }
    host == "localhost" || !host.contains('.') || [".local", ".lan", ".home.arpa", ".localhost"].iter().any(|s| host.ends_with(s))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn web(t: &str) -> String {
        Address::parse(t).map(|a| a.web).unwrap_or_else(|e| format!("error: {e}"))
    }

    #[test]
    fn addresses() {
        assert_eq!(web("demo.immich.app"), "https://demo.immich.app");
        assert_eq!(web("https://Demo.Immich.app/"), "https://demo.immich.app");
        assert_eq!(web("https://demo.immich.app/api"), "https://demo.immich.app");
        assert_eq!(web("https://example.com/photos/api/"), "https://example.com/photos");
        assert_eq!(web("192.168.1.10:2283"), "http://192.168.1.10:2283");
        assert_eq!(web("http://nas:2283"), "http://nas:2283");
        assert_eq!(web("http://[::1]:2283"), "http://[::1]:2283");
        assert_eq!(web("http://photos.home.arpa"), "http://photos.home.arpa");
        assert!(web("http://demo.immich.app").starts_with("error"));
        assert!(web("http://8.8.8.8").starts_with("error"));
        assert!(web("ftp://nas").starts_with("error"));
        assert!(web("").starts_with("error"));
        assert!(web("https://user:pw@example.com").starts_with("error"));
        assert!(web("https://example.com:99999").starts_with("error"));
        assert!(web("https://exa mple.com").starts_with("error"));
        assert!(web("https://example.com/?x=1").starts_with("error"));
        assert_eq!(Address::parse("nas:2283").map(|a| a.api()).ok().as_deref(), Some("http://nas:2283/api"));
    }

    #[test]
    fn local_hosts() {
        for h in ["localhost", "127.0.0.1", "10.0.0.5", "172.20.1.1", "192.168.0.2", "169.254.1.1", "::1", "fd00::1", "fe80::1", "nas", "box.local"] {
            assert!(is_local(h), "{h}");
        }
        for h in ["8.8.8.8", "172.32.0.1", "example.com", "2001:db8::1"] {
            assert!(!is_local(h), "{h}");
        }
    }
}
