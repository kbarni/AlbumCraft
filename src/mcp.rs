//! `albumcraft --mcp`: a Model Context Protocol server on stdin/stdout (JSON-RPC 2.0, one message
//! per line) offering the commands of `commands.rs` as tools.
//!
//! When the AlbumCraft window is open, each tool call is sent to it (see `control.rs`), so the
//! window shows the changes and nothing is saved twice; otherwise the library is loaded from
//! library.json and saved after every change. Only protocol messages go to stdout.

use crate::commands::{self, Headless, Reply};
use serde_json::{json, Value};
use std::io::{BufRead, Write};
use std::path::PathBuf;

const PROTOCOL: &str = "2025-06-18";

pub struct Config {
    /// library.json to use when no window is open.
    pub library: PathBuf,
    /// control.json of a window to talk to, if any.
    pub control: Option<PathBuf>,
}

impl Config {
    pub fn default_paths() -> Result<Config, String> {
        Ok(Config { library: crate::library::store_path().ok_or("no folder for application data")?, control: crate::control::control_path() })
    }
}

/// Serve until stdin closes.
pub fn serve(input: impl BufRead, mut output: impl Write, cfg: &Config) -> std::io::Result<()> {
    let mut headless: Option<Headless> = None;
    for line in input.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let msg: Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(e) => {
                send(&mut output, json!({ "jsonrpc": "2.0", "id": null, "error": { "code": -32700, "message": format!("parse error: {e}") } }))?;
                continue;
            }
        };
        let id = msg.get("id").cloned();
        let method = msg.get("method").and_then(|m| m.as_str()).unwrap_or("");
        let params = msg.get("params").cloned().unwrap_or(Value::Null);
        // notifications (no id) get no answer
        let Some(id) = id else { continue };
        let result = match method {
            "initialize" => Ok(json!({
                "protocolVersion": PROTOCOL,
                "capabilities": { "tools": {} },
                "serverInfo": { "name": "albumcraft", "version": env!("CARGO_PKG_VERSION") },
                "instructions": "AlbumCraft photo library: search pictures, pick/reject and rate them, caption, tag and put them in albums, apply non-destructive edits (adjustments, looks, frames, text), preview the result as an image, and export pictures, web galleries or slideshow movies. Pictures are named by their key (file path) from search_photos. Original files are never changed.",
            })),
            "ping" => Ok(json!({})),
            "tools/list" => Ok(json!({ "tools": commands::specs().iter().map(|s| json!({
                "name": s.name,
                "description": s.description,
                "inputSchema": (s.schema)(),
            })).collect::<Vec<_>>() })),
            "tools/call" => {
                let name = params.get("name").and_then(|n| n.as_str()).unwrap_or("").to_string();
                let args = params.get("arguments").cloned().unwrap_or(json!({}));
                Ok(tool_result(call(&name, &args, cfg, &mut headless)))
            }
            _ => Err((-32601, format!("unknown method {method:?}"))),
        };
        let reply = match result {
            Ok(r) => json!({ "jsonrpc": "2.0", "id": id, "result": r }),
            Err((code, message)) => json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } }),
        };
        send(&mut output, reply)?;
    }
    Ok(())
}

fn send(out: &mut impl Write, v: Value) -> std::io::Result<()> {
    writeln!(out, "{v}")?;
    out.flush()
}

/// Run one tool: through the open window when there is one, otherwise on the library itself.
fn call(name: &str, args: &Value, cfg: &Config, headless: &mut Option<Headless>) -> Result<Reply, String> {
    if !commands::specs().iter().any(|s| s.name == name) {
        return Err(format!("unknown tool {name:?}"));
    }
    if let Some(r) = cfg.control.as_deref().and_then(|c| crate::control::call_window(c, name, args)) {
        return r;
    }
    // no window: (re)load the library so changes made meanwhile by the window are seen
    let h = match headless.take() {
        Some(mut h) => match crate::library::Store::load_from(&cfg.library) {
            crate::library::Loaded::Store(s) => {
                let rescan = s.roots != h.store.roots;
                h.store = *s;
                if rescan {
                    commands::Host::rescan(&mut h);
                }
                h
            }
            crate::library::Loaded::Fresh => h,
            crate::library::Loaded::Unreadable { error, .. } => return Err(format!("the library file can't be read ({error})")),
        },
        None => Headless::open(cfg.library.clone())?,
    };
    let h = headless.insert(h);
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| commands::run(h, name, args))).unwrap_or_else(|_| Err("unexpected error".into()))?;
    if !commands::read_only(name) {
        h.save()?;
    }
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| outcome.finish())).unwrap_or_else(|_| Err("unexpected error".into()))
}

fn tool_result(r: Result<Reply, String>) -> Value {
    match r {
        Ok(reply) => {
            let text = serde_json::to_string_pretty(&reply.value).unwrap_or_default();
            let mut content = vec![json!({ "type": "text", "text": text })];
            if let Some(png) = reply.png {
                content.push(json!({ "type": "image", "data": crate::control::base64(&png), "mimeType": "image/png" }));
            }
            json!({ "content": content, "structuredContent": structured(&reply.value), "isError": false })
        }
        Err(e) => json!({ "content": [{ "type": "text", "text": e }], "isError": true }),
    }
}

/// Structured content must be an object; lists are wrapped.
fn structured(v: &Value) -> Value {
    if v.is_object() { v.clone() } else { json!({ "result": v }) }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn session(lines: &[Value], cfg: &Config) -> Vec<Value> {
        let input: String = lines.iter().map(|l| format!("{l}\n")).collect();
        let mut out = Vec::new();
        serve(std::io::Cursor::new(input), &mut out, cfg).unwrap();
        String::from_utf8(out).unwrap().lines().map(|l| serde_json::from_str(l).unwrap()).collect()
    }

    #[test]
    fn protocol_round_trip_without_a_window() {
        let dir = std::env::temp_dir().join(format!("albumcraft-mcp-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let pics = dir.join("pics");
        std::fs::create_dir_all(&pics).unwrap();
        let pic = pics.join("a.jpg");
        image::DynamicImage::ImageRgba8(crate::imaging::sample_image(3, 90, 60)).to_rgb8().save(&pic).unwrap();
        let cfg = Config { library: dir.join("library.json"), control: None };
        let key = pic.to_string_lossy().to_string();
        let replies = session(
            &[
                json!({ "jsonrpc": "2.0", "id": 1, "method": "initialize", "params": { "protocolVersion": PROTOCOL, "capabilities": {}, "clientInfo": { "name": "test", "version": "1" } } }),
                json!({ "jsonrpc": "2.0", "method": "notifications/initialized" }),
                json!({ "jsonrpc": "2.0", "id": 2, "method": "tools/list" }),
                json!({ "jsonrpc": "2.0", "id": 3, "method": "tools/call", "params": { "name": "add_folder", "arguments": { "path": pics.to_string_lossy() } } }),
                json!({ "jsonrpc": "2.0", "id": 4, "method": "tools/call", "params": { "name": "set_rating", "arguments": { "keys": [pics.canonicalize().unwrap().join("a.jpg").to_string_lossy()], "rating": 5 } } }),
                json!({ "jsonrpc": "2.0", "id": 5, "method": "tools/call", "params": { "name": "search_photos", "arguments": { "min_rating": 5 } } }),
                json!({ "jsonrpc": "2.0", "id": 6, "method": "tools/call", "params": { "name": "render_preview", "arguments": { "key": pics.canonicalize().unwrap().join("a.jpg").to_string_lossy(), "max_size": 64 } } }),
                json!({ "jsonrpc": "2.0", "id": 7, "method": "tools/call", "params": { "name": "set_rating", "arguments": { "keys": ["/nope.jpg"], "rating": 5 } } }),
                json!({ "jsonrpc": "2.0", "id": 8, "method": "nope" }),
                Value::String("not even an object".into()),
            ],
            &cfg,
        );
        let saved = std::fs::read_to_string(&cfg.library).unwrap_or_default();
        let _ = std::fs::remove_dir_all(&dir);
        let _ = key;
        assert_eq!(replies.len(), 8, "{replies:#?}");
        assert_eq!(replies[0]["result"]["serverInfo"]["name"], "albumcraft");
        assert!(replies[1]["result"]["tools"].as_array().is_some_and(|t| t.len() == commands::specs().len()));
        assert_eq!(replies[2]["result"]["isError"], false, "{}", replies[2]);
        assert_eq!(replies[3]["result"]["isError"], false, "{}", replies[3]);
        assert_eq!(replies[4]["result"]["structuredContent"]["total"], 1);
        assert_eq!(replies[5]["result"]["content"][1]["type"], "image");
        assert_eq!(replies[6]["result"]["isError"], true);
        assert_eq!(replies[7]["error"]["code"], -32601);
        assert!(saved.contains("\"rating\": 5"), "the change was saved");
    }
}
