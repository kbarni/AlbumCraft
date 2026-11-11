//! Downloading a pack of networks: only when the user asked for it, only from the project's
//! releases (`models::Pack::url`), and nothing about the user or the pictures is sent.
//!
//! Each file goes to `<name>.part` first. A download that was stopped or cut continues from
//! there the next time. The finished file must have the size and the SHA-256 the built-in list
//! gives, and only then gets its real name: a file under its real name is always whole.

use crate::models::{ModelFile, Pack};
use sha2::{Digest, Sha256};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

/// The unfinished download of `file`.
pub fn part_path(file: &Path) -> PathBuf {
    let mut name = file.file_name().unwrap_or_default().to_os_string();
    name.push(".part");
    file.with_file_name(name)
}

fn agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        // no limit on the whole download (hundreds of MB on a slow line), but none of its
        // steps may hang
        .timeout_resolve(Some(Duration::from_secs(20)))
        .timeout_connect(Some(Duration::from_secs(20)))
        .timeout_recv_response(Some(Duration::from_secs(30)))
        .timeout_per_read(Some(Duration::from_secs(30)))
        .http_status_as_error(false)
        .user_agent(format!("AlbumCraft/{}", env!("CARGO_PKG_VERSION")))
        .build()
        .into()
}

/// Download the files of `pack` that are missing into the models folder `dir`.
/// `progress(done, total)` is told the bytes so far and answers false to stop; stopping gives
/// `Err(STOPPED)` and keeps what was downloaded for the next time.
pub fn download_pack(pack: &Pack, dir: &Path, progress: &dyn Fn(u64, u64) -> bool) -> Result<(), String> {
    let urls: Vec<String> = pack.files.iter().map(|f| pack.url(f)).collect();
    download_files(pack, &urls, dir, progress)
}

/// `download_pack` with the address of each file given (the tests use a server of their own).
pub(crate) fn download_files(pack: &Pack, urls: &[String], dir: &Path, progress: &dyn Fn(u64, u64) -> bool) -> Result<(), String> {
    if !pack.valid() {
        return Err("this pack is not well formed".into());
    }
    std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let agent = agent();
    let total = pack.bytes();
    let mut before = 0u64;
    for (file, url) in pack.files.iter().zip(urls) {
        let dest = pack.path(dir, file);
        let have = std::fs::metadata(&dest).is_ok_and(|m| m.is_file() && m.len() == file.size);
        if !have {
            let step = |done: u64| progress(before.saturating_add(done).min(total), total);
            fetch(&agent, url, &dest, file, &step)?;
        }
        before = before.saturating_add(file.size);
        if !progress(before.min(total), total) && before < total {
            return Err(crate::STOPPED.into());
        }
    }
    Ok(())
}

/// One file: continue or start its `.part`, check it, give it its name.
fn fetch(agent: &ureq::Agent, url: &str, dest: &Path, file: &ModelFile, progress: &dyn Fn(u64) -> bool) -> Result<(), String> {
    let part = part_path(dest);
    let disk = |e: std::io::Error| format!("{}: {e}", part.display());
    let mut out = std::fs::OpenOptions::new().read(true).write(true).create(true).truncate(false).open(&part).map_err(disk)?;
    let mut have = out.metadata().map_err(disk)?.len();
    if have > file.size {
        // not a part of this file
        out.set_len(0).map_err(disk)?;
        have = 0;
    }
    if have < file.size {
        let asked_from = have;
        let mut request = agent.get(url);
        if have > 0 {
            request = request.header("Range", format!("bytes={have}-"));
        }
        let mut resp = request.call().map_err(|e| format!("The download failed ({e})."))?;
        match resp.status().as_u16() {
            206 => {}
            // the whole file again (the server does not continue downloads)
            200 => {
                out.set_len(0).map_err(disk)?;
                have = 0;
            }
            404 => return Err(format!("{} is not available for download (yet).", file.name)),
            code => return Err(format!("The download failed (the server answered {code}).")),
        }
        if have != 0 && !content_range_starts_at(&resp, asked_from) {
            return Err("The download failed (the server sent another part of the file than asked).".into());
        }
        out.seek(SeekFrom::Start(have)).map_err(disk)?;
        let mut body = resp.body_mut().as_reader();
        let mut buf = vec![0u8; 64 << 10];
        loop {
            if !progress(have) {
                return Err(crate::STOPPED.into());
            }
            let n = body.read(&mut buf).map_err(|e| format!("The download was cut ({e}); starting it again continues from here."))?;
            if n == 0 {
                break;
            }
            have += n as u64;
            if have > file.size {
                drop(out);
                let _ = std::fs::remove_file(&part);
                return Err(format!("The download failed ({} is longer than expected).", file.name));
            }
            out.write_all(buf.get(..n).unwrap_or(&[])).map_err(disk)?;
        }
        if have < file.size {
            return Err("The download was cut; starting it again continues from here.".into());
        }
    }
    out.flush().map_err(disk)?;
    out.seek(SeekFrom::Start(0)).map_err(disk)?;
    if sha256_of(&mut out).map_err(disk)? != file.sha256 {
        drop(out);
        let _ = std::fs::remove_file(&part);
        return Err(format!("The download failed ({} is not the expected file); it was removed.", file.name));
    }
    drop(out);
    std::fs::rename(&part, dest).map_err(|e| format!("{}: {e}", dest.display()))
}

/// The answer to a request for the bytes from `from` on really starts there.
fn content_range_starts_at(resp: &ureq::http::Response<ureq::Body>, from: u64) -> bool {
    let Some(range) = resp.headers().get("content-range").and_then(|v| v.to_str().ok()) else { return false };
    range.trim().strip_prefix("bytes").map(|r| r.trim_start_matches([' ', '='])).and_then(|r| r.split('-').next()).and_then(|start| start.trim().parse::<u64>().ok()) == Some(from)
}

fn sha256_of(file: &mut std::fs::File) -> std::io::Result<String> {
    let mut hash = Sha256::new();
    let mut buf = vec![0u8; 256 << 10];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hash.update(buf.get(..n).unwrap_or(&[]));
    }
    Ok(hash.finalize().iter().map(|b| format!("{b:02x}")).collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    use std::io::{BufRead, BufReader};
    use std::net::TcpListener;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    /// How the test server behaves.
    #[derive(Clone, Copy, PartialEq)]
    enum Server {
        Good,
        /// Answers requests for a part with the whole file.
        NoRanges,
        /// Sends more than the file.
        TooLong,
        Missing,
    }

    /// A small web server on this computer holding one file; returns its address and the
    /// counter of bytes it was asked to skip (the last `Range` start).
    fn serve(content: Vec<u8>, kind: Server) -> (String, Arc<AtomicUsize>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}/file.onnx", listener.local_addr().unwrap());
        let skipped = Arc::new(AtomicUsize::new(0));
        let seen = skipped.clone();
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(mut stream) = stream else { return };
                let mut from = 0usize;
                let mut lines = BufReader::new(stream.try_clone().unwrap()).lines();
                while let Some(Ok(line)) = lines.next() {
                    if let Some(r) = line.to_ascii_lowercase().strip_prefix("range: bytes=") {
                        from = r.trim_end_matches('-').parse().unwrap();
                    }
                    if line.is_empty() {
                        break;
                    }
                }
                seen.store(from, Ordering::SeqCst);
                let answer = match kind {
                    Server::Missing => b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".to_vec(),
                    Server::Good if from > 0 => {
                        let body = &content[from.min(content.len())..];
                        let mut a = format!("HTTP/1.1 206 Partial Content\r\nContent-Range: bytes {from}-{}/{}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", content.len() - 1, content.len(), body.len()).into_bytes();
                        a.extend_from_slice(body);
                        a
                    }
                    _ => {
                        let mut body = content.clone();
                        if kind == Server::TooLong {
                            body.extend_from_slice(&[7; 1000]);
                        }
                        let mut a = format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len()).into_bytes();
                        a.extend_from_slice(&body);
                        a
                    }
                };
                let _ = stream.write_all(&answer);
            }
        });
        (url, skipped)
    }

    fn content() -> Vec<u8> {
        (0..300_000u32).map(|i| (i * 7 + i / 255) as u8).collect()
    }

    fn pack_of(content: &[u8]) -> Pack {
        let sha256 = Sha256::digest(content).iter().map(|b| format!("{b:02x}")).collect();
        Pack {
            id: "test".into(),
            title: "Test".into(),
            about: String::new(),
            licence: "MIT".into(),
            version: 1,
            release: "models-1".into(),
            files: vec![ModelFile { name: "file.onnx".into(), size: content.len() as u64, sha256 }],
        }
    }

    fn temp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("albumcraft-ai-download-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn a_pack_is_downloaded_checked_and_named() {
        let dir = temp("whole");
        let data = content();
        let pack = pack_of(&data);
        let (url, _) = serve(data.clone(), Server::Good);
        let last = Cell::new((0, 0));
        download_files(&pack, &[url], &dir, &|done, total| {
            assert!(done >= last.get().0 && done <= total);
            last.set((done, total));
            true
        })
        .unwrap();
        assert_eq!(last.get(), (data.len() as u64, data.len() as u64));
        assert_eq!(std::fs::read(dir.join("file.onnx")).unwrap(), data);
        assert!(!dir.join("file.onnx.part").exists() && pack.installed(&dir));
        // nothing is asked of the server when the pack is there already
        download_files(&pack, &["http://127.0.0.1:1/none".into()], &dir, &|_, _| true).unwrap();
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_stopped_download_continues_where_it_was() {
        let dir = temp("resume");
        let data = content();
        let pack = pack_of(&data);
        let (url, skipped) = serve(data.clone(), Server::Good);
        let stopped = download_files(&pack, std::slice::from_ref(&url), &dir, &|done, _| done < 100_000);
        assert_eq!(stopped, Err(crate::STOPPED.to_string()));
        let part = std::fs::metadata(dir.join("file.onnx.part")).unwrap().len();
        assert!(part >= 100_000 && part < data.len() as u64 && !pack.installed(&dir));
        let first = Cell::new(None);
        download_files(&pack, std::slice::from_ref(&url), &dir, &|done, _| {
            first.set(first.get().or(Some(done)));
            true
        })
        .unwrap();
        assert_eq!(skipped.load(Ordering::SeqCst) as u64, part, "only the rest was asked for");
        assert_eq!(first.get(), Some(part), "progress starts from what was there");
        assert_eq!(std::fs::read(dir.join("file.onnx")).unwrap(), data);
        // a server that cannot continue: the file is downloaded whole again, and is right
        std::fs::remove_file(dir.join("file.onnx")).unwrap();
        std::fs::write(dir.join("file.onnx.part"), &data[..1234]).unwrap();
        let (url, _) = serve(data.clone(), Server::NoRanges);
        download_files(&pack, &[url], &dir, &|_, _| true).unwrap();
        assert_eq!(std::fs::read(dir.join("file.onnx")).unwrap(), data);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_wrong_file_is_never_kept() {
        let dir = temp("wrong");
        let data = content();
        let pack = pack_of(&data);
        // other content of the same size
        let mut other = data.clone();
        other[1000] ^= 1;
        let (url, _) = serve(other, Server::Good);
        let e = download_files(&pack, &[url], &dir, &|_, _| true).unwrap_err();
        assert!(e.contains("not the expected file"), "{e}");
        assert!(!dir.join("file.onnx").exists() && !dir.join("file.onnx.part").exists());
        // a part left by something else, of the right size: checked, removed
        std::fs::write(dir.join("file.onnx.part"), vec![1u8; data.len()]).unwrap();
        assert!(download_files(&pack, &["http://127.0.0.1:1/none".into()], &dir, &|_, _| true).is_err());
        assert!(!dir.join("file.onnx.part").exists());
        // longer than announced: stopped at once, nothing kept
        let (url, _) = serve(data.clone(), Server::TooLong);
        let e = download_files(&pack, &[url], &dir, &|_, _| true).unwrap_err();
        assert!(e.contains("longer than expected"), "{e}");
        assert!(!dir.join("file.onnx").exists() && !dir.join("file.onnx.part").exists());
        // not published, and no server at all: messages, no file
        let (url, _) = serve(data, Server::Missing);
        let e = download_files(&pack, &[url], &dir, &|_, _| true).unwrap_err();
        assert!(e.contains("not available"), "{e}");
        assert!(download_files(&pack, &["http://127.0.0.1:1/none".into()], &dir, &|_, _| true).is_err());
        assert!(!pack.installed(&dir));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
