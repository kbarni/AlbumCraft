//! The window's control connection: while AlbumCraft is open, the MCP server sends commands to
//! it instead of changing library.json behind its back (the window would overwrite such changes
//! with its next save).
//!
//! The window listens on 127.0.0.1 only, on a free port, and writes the port and a random
//! 256-bit token to `control.json` in the data folder (readable by the user only). Each request
//! is one line of JSON `{"token", "command", "args"}` (at most 1 MB) and gets one line back:
//! `{"ok": value, "png": base64?}` or `{"error": message}`. A wrong token closes the connection.

use crate::commands::Reply;
use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{channel, Sender};
use std::sync::Arc;
use std::time::Duration;

const MAX_LINE: u64 = 1 << 20;
const MAX_CONNECTIONS: usize = 4;

pub fn control_path() -> Option<PathBuf> {
    Some(dirs::data_dir()?.join("AlbumCraft").join("control.json"))
}

/// A command for the window to run; the answer goes back through `reply`.
pub struct Request {
    pub command: String,
    pub args: Value,
    pub reply: Sender<Result<Reply, String>>,
}

/// The listening side, in the window. Removes control.json when dropped.
pub struct Server {
    file: PathBuf,
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.file);
    }
}

fn new_token() -> Option<String> {
    let mut b = [0u8; 32];
    getrandom::fill(&mut b).ok()?;
    Some(b.iter().map(|x| format!("{x:02x}")).collect())
}

/// Same length and content, without stopping at the first difference.
fn same(a: &str, b: &str) -> bool {
    a.len() == b.len() && a.bytes().zip(b.bytes()).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

/// Start listening; `wake` is called after a request was queued (to repaint the window).
pub fn start(file: PathBuf, tx: Sender<Request>, wake: impl Fn() + Send + Sync + 'static) -> Result<Server, String> {
    let listener = TcpListener::bind(("127.0.0.1", 0)).map_err(|e| e.to_string())?;
    let port = listener.local_addr().map_err(|e| e.to_string())?.port();
    let token = new_token().ok_or("no random numbers for the control token")?;
    write_private(&file, &json!({ "port": port, "token": token, "pid": std::process::id() }).to_string()).map_err(|e| format!("{}: {e}", file.display()))?;
    let wake = Arc::new(wake);
    let open = Arc::new(AtomicUsize::new(0));
    std::thread::Builder::new()
        .name("control".into())
        .spawn(move || {
            for stream in listener.incoming().flatten() {
                if open.load(Ordering::SeqCst) >= MAX_CONNECTIONS {
                    continue; // dropped: closed
                }
                open.fetch_add(1, Ordering::SeqCst);
                let (tx, token, wake, open) = (tx.clone(), token.clone(), wake.clone(), open.clone());
                let _ = std::thread::Builder::new().name("control-conn".into()).spawn(move || {
                    let _ = serve(stream, &token, &tx, &*wake);
                    open.fetch_sub(1, Ordering::SeqCst);
                });
            }
        })
        .map_err(|e| e.to_string())?;
    Ok(Server { file })
}

fn serve(stream: TcpStream, token: &str, tx: &Sender<Request>, wake: &(dyn Fn() + Send + Sync)) -> std::io::Result<()> {
    let mut out = stream.try_clone()?;
    let mut reader = BufReader::new(stream);
    loop {
        let mut line = String::new();
        let n = (&mut reader).take(MAX_LINE).read_line(&mut line)?;
        if n == 0 {
            return Ok(());
        }
        if !line.ends_with('\n') {
            writeln!(out, "{}", json!({ "error": "request too long" }))?;
            return Ok(());
        }
        let req: Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(e) => {
                writeln!(out, "{}", json!({ "error": format!("not JSON: {e}") }))?;
                continue;
            }
        };
        if !req.get("token").and_then(|t| t.as_str()).is_some_and(|t| same(t, token)) {
            writeln!(out, "{}", json!({ "error": "wrong token" }))?;
            return Ok(());
        }
        let command = req.get("command").and_then(|c| c.as_str()).unwrap_or("").to_string();
        let args = req.get("args").cloned().unwrap_or(Value::Null);
        let (rtx, rrx) = channel();
        if tx.send(Request { command, args, reply: rtx }).is_err() {
            writeln!(out, "{}", json!({ "error": "the window is closing" }))?;
            return Ok(());
        }
        wake();
        let answer = match rrx.recv() {
            Ok(Ok(r)) => json!({ "ok": r.value, "png": r.png.map(|b| base64(&b)) }),
            Ok(Err(e)) => json!({ "error": e }),
            Err(_) => json!({ "error": "the window stopped before answering" }),
        };
        writeln!(out, "{answer}")?;
    }
}

/// Send a command to the open window. `None` when no window is listening (then the caller
/// works on the library itself).
pub fn call_window(file: &Path, command: &str, args: &Value) -> Option<Result<Reply, String>> {
    let info: Value = serde_json::from_str(&std::fs::read_to_string(file).ok()?).ok()?;
    let port = u16::try_from(info.get("port")?.as_u64()?).ok()?;
    let token = info.get("token")?.as_str()?;
    let addr = std::net::SocketAddr::from(([127, 0, 0, 1], port));
    let stream = TcpStream::connect_timeout(&addr, Duration::from_millis(500)).ok()?;
    let request = json!({ "token": token, "command": command, "args": args });
    let mut w = stream.try_clone().ok()?;
    if writeln!(w, "{request}").is_err() {
        return None;
    }
    let mut line = String::new();
    if BufReader::new(stream).read_line(&mut line).is_err() || line.is_empty() {
        return Some(Err("the AlbumCraft window didn't answer".into()));
    }
    let v: Value = match serde_json::from_str(&line) {
        Ok(v) => v,
        Err(e) => return Some(Err(format!("bad answer from the window: {e}"))),
    };
    if let Some(e) = v.get("error").and_then(|e| e.as_str()) {
        return Some(Err(e.to_string()));
    }
    let png = v.get("png").and_then(|p| p.as_str()).and_then(unbase64);
    Some(Ok(Reply { value: v.get("ok").cloned().unwrap_or(Value::Null), png }))
}

/// Write a file only the user can read (the token in it is a key to the library).
fn write_private(path: &Path, text: &str) -> std::io::Result<()> {
    if let Some(d) = path.parent() {
        std::fs::create_dir_all(d)?;
    }
    let _ = std::fs::remove_file(path);
    let mut o = std::fs::OpenOptions::new();
    o.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        o.mode(0o600);
    }
    o.open(path)?.write_all(text.as_bytes())
}

const B64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

pub fn base64(data: &[u8]) -> String {
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for c in data.chunks(3) {
        let n = (c[0] as u32) << 16 | (*c.get(1).unwrap_or(&0) as u32) << 8 | *c.get(2).unwrap_or(&0) as u32;
        for i in 0..4 {
            if i <= c.len() {
                out.push(B64[(n >> (18 - 6 * i) & 63) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

pub fn unbase64(s: &str) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(s.len() / 4 * 3);
    let (mut acc, mut bits) = (0u32, 0);
    for ch in s.bytes().filter(|b| !b.is_ascii_whitespace()) {
        if ch == b'=' {
            break;
        }
        let v = B64.iter().position(|&x| x == ch)? as u32;
        acc = acc << 6 | v;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits & 0xff) as u8);
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_round_trip() {
        for data in [&b""[..], b"f", b"fo", b"foo", b"foob", b"fooba", b"foobar", &[0, 255, 7, 128]] {
            assert_eq!(unbase64(&base64(data)).as_deref(), Some(data));
        }
        assert_eq!(base64(b"foobar"), "Zm9vYmFy");
        assert_eq!(base64(b"fo"), "Zm8=");
    }

    #[test]
    fn window_answers_with_the_right_token_only() {
        let file = std::env::temp_dir().join(format!("albumcraft-control-{}.json", std::process::id()));
        let (tx, rx) = channel::<Request>();
        let server = start(file.clone(), tx, || {}).unwrap();
        // a stand-in for the window: answers every request
        std::thread::spawn(move || {
            for r in rx {
                let answer = if r.command == "echo" { Ok(Reply { value: r.args, png: Some(vec![1, 2, 3]) }) } else { Err("unknown".into()) };
                let _ = r.reply.send(answer);
            }
        });
        let ok = call_window(&file, "echo", &json!({ "x": 1 })).unwrap().unwrap();
        assert_eq!(ok.value, json!({ "x": 1 }));
        assert_eq!(ok.png, Some(vec![1, 2, 3]));
        assert_eq!(call_window(&file, "nope", &json!({})).unwrap().unwrap_err(), "unknown");
        // a forged token is refused
        let info: Value = serde_json::from_str(&std::fs::read_to_string(&file).unwrap()).unwrap();
        let forged = file.with_extension("forged.json");
        std::fs::write(&forged, json!({ "port": info["port"], "token": "00" }).to_string()).unwrap();
        assert_eq!(call_window(&forged, "echo", &json!({})).unwrap().unwrap_err(), "wrong token");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(std::fs::metadata(&file).unwrap().permissions().mode() & 0o077, 0, "control.json readable by others");
        }
        drop(server);
        let _ = std::fs::remove_file(&forged);
        assert!(!file.exists(), "control.json removed when the window closes");
        // no window: None, so the caller works on the library itself
        assert!(call_window(&file, "echo", &json!({})).is_none());
    }
}
