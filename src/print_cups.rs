//! Printing on Linux and macOS through CUPS, the pages sent as a PDF of the chosen paper size.
//!
//! Two ways to reach it, chosen once (`spooler`):
//! - **CUPS's programs** when they are installed: `craft-print` lists the printers (`lpstat`,
//!   with the driverless network printers CUPS has no queue for, which only its programs can
//!   print to) and sends the job (`lp`, the PDF on its standard input); the paper sizes and
//!   resolution come from `lpoptions` (`tools`).
//! - Else **IPP** straight to the scheduler's socket (or `CUPS_SERVER`), in this process: what a
//!   Flatpak does, since it has the socket (`--socket=cups`) but not the CUPS programs.
//!   `craft-ipp` finds the server, lists the printers and the default one, and encodes and reads
//!   the messages; this file adds what it doesn't do yet: Get-Printer-Attributes (the paper sizes,
//!   the default paper and resolution) and a Print-Job that names the paper (`ipp`).

use super::{Paper, PrinterChoice};
use std::sync::OnceLock;

enum Spooler {
    Ipp(craft_ipp::Endpoint),
    Tools,
}

fn spooler() -> Option<&'static Spooler> {
    static SPOOLER: OnceLock<Option<Spooler>> = OnceLock::new();
    SPOOLER
        .get_or_init(|| {
            if tools::running() {
                Some(Spooler::Tools)
            } else {
                craft_ipp::Endpoint::detect().map(Spooler::Ipp)
            }
        })
        .as_ref()
}

pub fn available() -> bool {
    spooler().is_some()
}

pub fn printers() -> Vec<String> {
    match spooler() {
        Some(Spooler::Ipp(ep)) => craft_ipp::printers(ep).map(|list| list.into_iter().map(|p| p.name).collect()).unwrap_or_default(),
        Some(Spooler::Tools) => craft_print::printers().into_iter().map(|p| p.name).collect(),
        None => vec![],
    }
}

pub fn default_printer() -> Option<String> {
    match spooler()? {
        Spooler::Ipp(ep) => craft_ipp::default_printer(ep).ok().flatten(),
        Spooler::Tools => craft_print::printers().into_iter().find(|p| p.default).map(|p| p.name),
    }
}

/// What a printer offers: its paper names, the one it uses when a job names none, its resolution.
#[derive(Debug, Default, PartialEq)]
struct Media {
    sizes: Vec<String>,
    default: Option<String>,
    dpi: Option<u32>,
}

fn media(printer: &str) -> Media {
    if printer.is_empty() {
        return Media::default();
    }
    match spooler() {
        Some(Spooler::Ipp(ep)) => ipp::media(ep, printer).unwrap_or_default(),
        Some(Spooler::Tools) => tools::media(printer),
        None => Media::default(),
    }
}

/// Paper names of the printer that the print screen can lay pages out on.
pub fn paper_sizes(printer: &str) -> Vec<String> {
    let mut sizes: Vec<String> = media(printer).sizes.into_iter().filter(|n| size_mm(n).is_some() && !is_envelope(n)).collect();
    let mut seen = std::collections::HashSet::new();
    sizes.retain(|n| seen.insert(n.clone()));
    sizes
}

pub fn paper(p: &PrinterChoice) -> Option<Paper> {
    if p.name.is_empty() {
        return None;
    }
    let m = media(&p.name);
    let usable = |n: &String| size_mm(n).is_some();
    let name = p.media.clone().or(m.default.filter(usable)).or_else(|| m.sizes.iter().find(|n| usable(n)).cloned()).unwrap_or_else(|| "A4".into());
    let (w, h) = size_mm(&name)?;
    // CUPS doesn't tell the unprintable edge here; 5 mm suits nearly every printer
    let margin = 5.0;
    Some(Paper { sheet_w: w, sheet_h: h, area_x: margin, area_y: margin, area_w: w - 2.0 * margin, area_h: h - 2.0 * margin, dpi: m.dpi.unwrap_or(300).clamp(72, 2400) })
}

pub fn setup_dialog(_: &PrinterChoice) -> Option<PrinterChoice> {
    None
}

pub fn submit(p: &PrinterChoice, title: &str, pdf: &[u8]) -> Result<(), String> {
    match spooler() {
        Some(Spooler::Ipp(ep)) => ipp::print(ep, &p.name, p.media.as_deref(), title, pdf),
        Some(Spooler::Tools) => {
            let job = craft_print::Job {
                printer: Some(p.name.clone()),
                title: title.to_string(),
                // the PDF is already the sheet's size: never scaled to the driver's paper
                scaling: craft_print::Scaling::Exact,
                options: p.media.iter().map(|m| ("media".to_string(), m.clone())).collect(),
                ..Default::default()
            };
            craft_print::submit(pdf, &job).map(|_| ()).map_err(|e| e.to_string())
        }
        None => Err("no print service".into()),
    }
}

/// Envelope sizes, left out of the print screen's paper list (IPP names them all, `lpoptions`
/// names them in words the size table doesn't know).
fn is_envelope(name: &str) -> bool {
    let n = name.to_ascii_lowercase();
    let size = n.split('_').nth(1).unwrap_or(&n);
    n.contains("env")
        || ["number-", "monarch", "personal", "invite", "chou", "kaku", "you"].iter().any(|e| size.starts_with(e))
        || size == "dl"
        || (n.starts_with("iso_c") && size.len() > 1 && size[1..].bytes().all(|b| b.is_ascii_digit()))
}

/// Width and height in millimetres of a CUPS / PPD / PWG paper name; `None` for names that are
/// not one size (custom ranges, envelopes the table doesn't know).
pub(super) fn size_mm(name: &str) -> Option<(f32, f32)> {
    let known: &[(&str, f32, f32)] = &[
        ("A3", 297.0, 420.0),
        ("A4", 210.0, 297.0),
        ("A5", 148.0, 210.0),
        ("A6", 105.0, 148.0),
        ("B5", 182.0, 257.0),
        ("Letter", 215.9, 279.4),
        ("Legal", 215.9, 355.6),
        ("Executive", 184.2, 266.7),
        ("11x17", 279.4, 431.8),
        ("Tabloid", 279.4, 431.8),
        ("4x6", 101.6, 152.4),
        ("5x7", 127.0, 177.8),
        ("8x10", 203.2, 254.0),
    ];
    // IPP's custom size ranges ("custom_min_76.2x127mm") are limits, not a paper
    if name.starts_with("custom_min_") || name.starts_with("custom_max_") {
        return None;
    }
    // PPD variants like "A4.Borderless" have the size before the dot
    let named = name.split('.').next().unwrap_or(name);
    if let Some((_, w, h)) = known.iter().find(|(n, _, _)| n.eq_ignore_ascii_case(named)) {
        return Some((*w, *h));
    }
    // "w288h432": points
    if let Some(rest) = name.strip_prefix('w')
        && let Some((w, h)) = rest.split_once('h')
        && let (Ok(w), Ok(h)) = (w.parse::<f32>(), h.parse::<f32>())
    {
        return Some((w / 72.0 * 25.4, h / 72.0 * 25.4));
    }
    // PWG self-describing names: "iso_a4_210x297mm", "na_letter_8.5x11in"
    let last = name.rsplit('_').next()?;
    let (dims, unit) = if let Some(d) = last.strip_suffix("mm") { (d, 1.0) } else { (last.strip_suffix("in")?, 25.4) };
    let (w, h) = dims.split_once('x')?;
    let (w, h) = (w.parse::<f32>().ok()? * unit, h.parse::<f32>().ok()? * unit);
    (w > 10.0 && h > 10.0 && w < 2000.0 && h < 5000.0).then_some((w, h))
}

/// CUPS's programs, for when no socket answers.
mod tools {
    use super::Media;
    use std::process::{Command, Stdio};

    fn run(cmd: &str, args: &[&str]) -> Option<String> {
        let out = Command::new(cmd).args(args).env("LC_ALL", "C").stdin(Stdio::null()).stderr(Stdio::null()).output().ok()?;
        out.status.success().then(|| String::from_utf8_lossy(&out.stdout).to_string())
    }

    pub fn running() -> bool {
        run("lpstat", &["-r"]).is_some_and(|o| o.contains("scheduler is running"))
    }

    /// The paper sizes and resolution from `lpoptions -l` (the queue's PPD, through the server).
    pub fn media(printer: &str) -> Media {
        let Some(listing) = run("lpoptions", &["-p", printer, "-l"]) else { return Media::default() };
        let (sizes, default) = parse_option(&listing, "PageSize");
        let dpi = parse_option(&listing, "Resolution").1.and_then(|r| r.split(['x', 'd']).next()?.parse::<u32>().ok());
        Media { sizes, default, dpi }
    }

    /// The choices of option `key` in an `lpoptions -l` listing, and the current one (`*`).
    pub fn parse_option(listing: &str, key: &str) -> (Vec<String>, Option<String>) {
        for line in listing.lines() {
            let Some((name, values)) = line.split_once(':') else { continue };
            if name.split('/').next() != Some(key) {
                continue;
            }
            let mut all = vec![];
            let mut default = None;
            for v in values.split_whitespace() {
                let clean = v.trim_start_matches('*').to_string();
                if v.starts_with('*') {
                    default = Some(clean.clone());
                }
                all.push(clean);
            }
            return (all, default);
        }
        (vec![], None)
    }
}

/// The IPP requests `craft-ipp` doesn't make yet, built with its encoder and read with its
/// parser, over one HTTP/1.1 exchange of our own.
mod ipp {
    use super::Media;
    use craft_ipp::{Attribute, Endpoint, Reply, Request};
    use std::io::{Read, Write};
    use std::time::{Duration, Instant};

    const OP_PRINT_JOB: u16 = 0x0002;
    const OP_GET_PRINTER_ATTRIBUTES: u16 = 0x000B;
    const TAG_RESOLUTION: u8 = 0x32;
    /// How long a question may take, and a job (which carries the whole PDF).
    const ASK: Duration = Duration::from_secs(5);
    const JOB: Duration = Duration::from_secs(120);
    /// Largest reply read.
    const MAX_REPLY: usize = 8 << 20;

    fn request_id() -> u32 {
        static NEXT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(1);
        NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    }

    /// The user CUPS shows in its queue (on its socket it knows who we are anyway).
    fn user() -> String {
        std::env::var("USER").ok().filter(|u| !u.is_empty() && u.len() < 256 && !u.chars().any(char::is_control)).unwrap_or_else(|| "anonymous".into())
    }

    /// The server as the HTTP request and the printer URIs name it.
    fn authority(ep: &Endpoint) -> String {
        match ep {
            Endpoint::Unix(_) => "localhost".into(),
            Endpoint::Tcp(..) => ep.to_string(),
        }
    }

    /// `/printers/<name>` or `/classes/<name>`, the name percent-encoded.
    fn queue_path(ep: &Endpoint, name: &str) -> Result<String, String> {
        if name.is_empty() || name.len() > 127 || name.chars().any(|c| c.is_control() || c == '/' || c == '\\') || matches!(name, "." | "..") {
            return Err(format!("not a printer name: {name}"));
        }
        let is_class = craft_ipp::printers(ep).ok().and_then(|list| list.into_iter().find(|p| p.name == name)).is_some_and(|p| p.is_class);
        let mut path = String::from(if is_class { "/classes/" } else { "/printers/" });
        for b in name.bytes() {
            if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'.' | b'_' | b'~') {
                path.push(char::from(b));
            } else {
                path.push_str(&format!("%{b:02X}"));
            }
        }
        Ok(path)
    }

    /// A printer's paper sizes, its default paper and resolution (Get-Printer-Attributes).
    pub fn media(ep: &Endpoint, printer: &str) -> Result<Media, String> {
        let path = queue_path(ep, printer)?;
        let mut r = Request::new(OP_GET_PRINTER_ATTRIBUTES, request_id());
        r.attr(craft_ipp::TAG_URI, "printer-uri", format!("ipp://{}{path}", authority(ep)).as_bytes());
        r.attr(craft_ipp::TAG_NAME, "requesting-user-name", user().as_bytes());
        r.attr_many(craft_ipp::TAG_KEYWORD, "requested-attributes", &["media-supported", "media-default", "printer-resolution-default"]);
        let reply = exchange(ep, &path, &r.end(), ASK)?;
        Ok(media_of(&reply))
    }

    pub(super) fn media_of(reply: &Reply) -> Media {
        let named = |a: &&Attribute| a.group == craft_ipp::TAG_PRINTER_GROUP && matches!(a.tag, craft_ipp::TAG_KEYWORD | craft_ipp::TAG_NAME);
        let text = |v: &Vec<u8>| String::from_utf8_lossy(v).trim().to_string();
        let sizes = reply.attributes.iter().filter(named).filter(|a| a.name == "media-supported").flat_map(|a| a.values.iter().map(text)).filter(|s| !s.is_empty()).collect();
        let default = reply.attributes.iter().filter(named).find(|a| a.name == "media-default").map(Attribute::text).filter(|s| !s.is_empty());
        // resolution: x, y (signed 32-bit) and the unit (3 dots per inch, 4 per centimetre)
        let dpi = reply.attributes.iter().find(|a| a.name == "printer-resolution-default" && a.tag == TAG_RESOLUTION).and_then(|a| {
            let v = a.values.first()?;
            let x = i32::from_be_bytes(v.get(0..4)?.try_into().ok()?);
            let per_inch = match v.get(8)? {
                3 => x as f32,
                4 => x as f32 * 2.54,
                _ => return None,
            };
            (per_inch >= 1.0).then(|| per_inch.round() as u32)
        });
        Media { sizes, default, dpi }
    }

    /// The Print-Job request: the PDF on the paper named (else the printer's default), never
    /// scaled (it already has the sheet's size).
    pub(super) fn job_request(ep: &Endpoint, path: &str, media: Option<&str>, title: &str, pdf: &[u8]) -> Vec<u8> {
        let mut r = Request::new(OP_PRINT_JOB, request_id());
        r.attr(craft_ipp::TAG_URI, "printer-uri", format!("ipp://{}{path}", authority(ep)).as_bytes());
        r.attr(craft_ipp::TAG_NAME, "requesting-user-name", user().as_bytes());
        r.attr(craft_ipp::TAG_NAME, "job-name", title.as_bytes());
        r.attr(craft_ipp::TAG_MIME, "document-format", b"application/pdf");
        r.group(craft_ipp::TAG_JOB_GROUP);
        if let Some(m) = media.filter(|m| !m.is_empty() && m.len() < 256 && m.bytes().all(|b| b.is_ascii_graphic())) {
            r.attr(craft_ipp::TAG_KEYWORD, "media", m.as_bytes());
        }
        r.attr(craft_ipp::TAG_KEYWORD, "print-scaling", b"none");
        let mut body = r.end();
        body.extend_from_slice(pdf);
        body
    }

    pub fn print(ep: &Endpoint, printer: &str, media: Option<&str>, title: &str, pdf: &[u8]) -> Result<(), String> {
        let path = queue_path(ep, printer)?;
        // sent once: a job the server may already have accepted is never sent again
        exchange(ep, &path, &job_request(ep, &path, media, title, pdf), JOB).map(|_| ())
    }

    enum Stream {
        Unix(std::os::unix::net::UnixStream),
        Tcp(std::net::TcpStream),
    }

    impl Stream {
        fn open(ep: &Endpoint, timeout: Duration) -> Result<Stream, String> {
            let s = match ep {
                Endpoint::Unix(path) => {
                    let s = std::os::unix::net::UnixStream::connect(path).map_err(|e| format!("{}: {e}", path.display()))?;
                    s.set_read_timeout(Some(timeout)).and(s.set_write_timeout(Some(timeout))).map_err(|e| e.to_string())?;
                    Stream::Unix(s)
                }
                Endpoint::Tcp(host, port) => {
                    use std::net::ToSocketAddrs;
                    let addr = (host.as_str(), *port).to_socket_addrs().map_err(|e| format!("{ep}: {e}"))?.next().ok_or_else(|| format!("{ep}: no address"))?;
                    let s = std::net::TcpStream::connect_timeout(&addr, Duration::from_secs(3)).map_err(|e| format!("{ep}: {e}"))?;
                    s.set_read_timeout(Some(timeout)).and(s.set_write_timeout(Some(timeout))).map_err(|e| e.to_string())?;
                    Stream::Tcp(s)
                }
            };
            Ok(s)
        }

        fn io(&mut self) -> &mut dyn ReadWrite {
            match self {
                Stream::Unix(s) => s,
                Stream::Tcp(s) => s,
            }
        }
    }

    trait ReadWrite: Read + Write {}
    impl<T: Read + Write> ReadWrite for T {}

    /// One request posted to `path` and its IPP reply, which must be a success.
    fn exchange(ep: &Endpoint, path: &str, body: &[u8], timeout: Duration) -> Result<Reply, String> {
        let mut stream = Stream::open(ep, timeout)?;
        let io = stream.io();
        let head = format!(
            "POST {path} HTTP/1.1\r\nHost: {}\r\nContent-Type: application/ipp\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            authority(ep),
            body.len()
        );
        io.write_all(head.as_bytes()).and_then(|()| io.write_all(body)).and_then(|()| io.flush()).map_err(|e| format!("the print service: {e}"))?;
        let deadline = Instant::now() + timeout;
        let mut raw = vec![];
        let mut buf = [0u8; 16 << 10];
        loop {
            if Instant::now() > deadline {
                return Err("the print service did not answer in time".into());
            }
            match io.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    raw.extend_from_slice(&buf[..n]);
                    if raw.len() > MAX_REPLY {
                        return Err("the print service's answer is too long".into());
                    }
                    if http_body(&raw).is_some_and(|r| r.is_ok()) {
                        break;
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {}
                Err(e) => return Err(format!("the print service: {e}")),
            }
        }
        let body = http_body(&raw).unwrap_or_else(|| Err("the print service's answer was cut short".into()))?;
        let reply = Reply::parse(&body).map_err(|e| e.to_string())?;
        // 0x0000-0x00ff: successful (some attributes may have been ignored or substituted)
        if reply.status > 0x00ff {
            let message = reply.attributes.iter().find(|a| a.name == "status-message").map(Attribute::text).filter(|m| !m.is_empty());
            return Err(message.unwrap_or_else(|| format!("the print service refused (IPP status {:#06x})", reply.status)));
        }
        Ok(reply)
    }

    /// The body of a complete HTTP response in `raw`: `None` while more is to come, an error
    /// when the server answered with an HTTP error.
    pub(super) fn http_body(raw: &[u8]) -> Option<Result<Vec<u8>, String>> {
        let mut rest = raw;
        loop {
            let end = rest.windows(4).position(|w| w == b"\r\n\r\n")?;
            let head = String::from_utf8_lossy(&rest[..end]).to_string();
            let body = &rest[end + 4..];
            let mut lines = head.split("\r\n");
            let status: u16 = lines.next().and_then(|l| l.split_whitespace().nth(1)).and_then(|c| c.parse().ok()).unwrap_or(0);
            // "100 Continue" comes before the real answer
            if (100..200).contains(&status) {
                rest = body;
                continue;
            }
            if status != 200 {
                return Some(Err(format!("the print service answered HTTP {status}")));
            }
            let header = |name: &str| {
                head.split("\r\n").skip(1).find_map(|l| l.split_once(':').filter(|(k, _)| k.trim().eq_ignore_ascii_case(name)).map(|(_, v)| v.trim().to_ascii_lowercase()))
            };
            if header("transfer-encoding").is_some_and(|v| v.contains("chunked")) {
                return dechunk(body).map(Ok);
            }
            if let Some(n) = header("content-length").and_then(|v| v.parse::<usize>().ok()) {
                return (body.len() >= n).then(|| Ok(body[..n].to_vec()));
            }
            // neither: the body ends where the connection does (the caller reads to the end)
            return None;
        }
    }

    /// A chunked body put together; `None` until its last chunk has arrived.
    pub(super) fn dechunk(mut b: &[u8]) -> Option<Vec<u8>> {
        let mut out = vec![];
        loop {
            let line_end = b.windows(2).position(|w| w == b"\r\n")?;
            let size_text = String::from_utf8_lossy(&b[..line_end]);
            let size = usize::from_str_radix(size_text.split(';').next()?.trim(), 16).ok()?;
            b = &b[line_end + 2..];
            if size == 0 {
                return Some(out);
            }
            if b.len() < size + 2 {
                return None;
            }
            out.extend_from_slice(&b[..size]);
            b = &b[size + 2..];
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use craft_ipp::{Endpoint, Request};
    use std::io::{Read, Write};

    #[test]
    fn option_and_paper_names() {
        let listing = "PageSize/Page Size: Custom.WIDTHxHEIGHT Letter *A4 11x17 A3 EnvDL\nResolution/Resolution: 300x300dpi *600x600dpi\n";
        assert_eq!(tools::parse_option(listing, "PageSize").1.as_deref(), Some("A4"));
        assert_eq!(tools::parse_option(listing, "Resolution").1.as_deref(), Some("600x600dpi"));
        assert_eq!(tools::parse_option(listing, "Duplex"), (vec![], None));
        assert_eq!(size_mm("A4"), Some((210.0, 297.0)));
        assert_eq!(size_mm("a4"), Some((210.0, 297.0)));
        assert_eq!(size_mm("w288h432"), Some((101.6, 152.4)));
        assert_eq!(size_mm("iso_a5_148x210mm"), Some((148.0, 210.0)));
        let l = size_mm("na_letter_8.5x11in").unwrap();
        assert!((l.0 - 215.9).abs() < 0.01 && (l.1 - 279.4).abs() < 0.01);
        assert_eq!(size_mm("EnvDL"), None);
        assert_eq!(size_mm("Custom.WIDTHxHEIGHT"), None);
        assert_eq!(size_mm("custom_min_76.2x127mm"), None);
        // a PPD's A5 as CUPS names it over IPP
        assert_eq!(size_mm("custom_148.52x209.9mm_148.52x209.9mm"), Some((148.52, 209.9)));
        for e in ["iso_c5_162x229mm", "iso_dl_110x220mm", "na_number-10_4.125x9.5in", "na_monarch_3.875x7.5in", "EnvDL"] {
            assert!(is_envelope(e), "{e}");
        }
        for p in ["iso_a4_210x297mm", "na_letter_8.5x11in", "na_index-4x6_4x6in", "iso_b5_176x250mm", "A4"] {
            assert!(!is_envelope(p), "{p}");
        }
        assert_eq!(size_mm("custom_max_215.9x355.6mm"), None);
    }

    /// A reply as CUPS sends it: printer attributes, several values for one, a resolution.
    fn printer_reply() -> Vec<u8> {
        let mut r = Request::new(0x0000, 7);
        r.group(craft_ipp::TAG_PRINTER_GROUP);
        r.attr_many(craft_ipp::TAG_KEYWORD, "media-supported", &["iso_a4_210x297mm", "na_letter_8.5x11in", "na_index-4x6_4x6in", "custom_min_76.2x127mm"]);
        r.attr(craft_ipp::TAG_KEYWORD, "media-default", b"iso_a4_210x297mm");
        let mut res = vec![];
        res.extend_from_slice(&600i32.to_be_bytes());
        res.extend_from_slice(&600i32.to_be_bytes());
        res.push(3);
        r.attr(0x32, "printer-resolution-default", &res);
        r.end()
    }

    #[test]
    fn reads_the_printers_paper() {
        let reply = craft_ipp::Reply::parse(&printer_reply()).unwrap();
        let m = ipp::media_of(&reply);
        assert_eq!(m.sizes, ["iso_a4_210x297mm", "na_letter_8.5x11in", "na_index-4x6_4x6in", "custom_min_76.2x127mm"]);
        assert_eq!(m.default.as_deref(), Some("iso_a4_210x297mm"));
        assert_eq!(m.dpi, Some(600));
    }

    #[test]
    fn http_answers() {
        let ok = b"HTTP/1.1 200 OK\r\nContent-Type: application/ipp\r\nContent-Length: 3\r\n\r\nabc";
        assert_eq!(ipp::http_body(ok), Some(Ok(b"abc".to_vec())));
        // not all there yet
        assert_eq!(ipp::http_body(&ok[..ok.len() - 1]), None);
        assert_eq!(ipp::http_body(b"HTTP/1.1 200 OK\r\nContent-Len"), None);
        let chunked = b"HTTP/1.1 100 Continue\r\n\r\nHTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n3\r\nabc\r\n2;x=y\r\nde\r\n0\r\n\r\n";
        assert_eq!(ipp::http_body(chunked), Some(Ok(b"abcde".to_vec())));
        assert_eq!(ipp::dechunk(b"3\r\nabc\r\n"), None);
        assert!(matches!(ipp::http_body(b"HTTP/1.1 401 Unauthorized\r\nContent-Length: 0\r\n\r\n"), Some(Err(_))));
    }

    /// A one-request print server on a socket in a temporary folder: answers `reply` to every
    /// request and hands back what it was sent.
    fn fake_server(replies: Vec<Vec<u8>>) -> (Endpoint, std::thread::JoinHandle<Vec<Vec<u8>>>, std::path::PathBuf) {
        let dir = std::env::temp_dir().join(format!("albumcraft-ipp-{}-{}", std::process::id(), ipp_test_id()));
        std::fs::create_dir_all(&dir).unwrap();
        let sock = dir.join("cups.sock");
        let listener = std::os::unix::net::UnixListener::bind(&sock).unwrap();
        let handle = std::thread::spawn(move || {
            let mut got = vec![];
            for reply in replies {
                let (mut s, _) = listener.accept().unwrap();
                let mut raw = vec![];
                let mut buf = [0u8; 4096];
                // read the head, then as much body as it says
                loop {
                    let n = s.read(&mut buf).unwrap();
                    raw.extend_from_slice(&buf[..n]);
                    if let Some(end) = raw.windows(4).position(|w| w == b"\r\n\r\n") {
                        let head = String::from_utf8_lossy(&raw[..end]).to_string();
                        let len: usize = head.lines().find_map(|l| l.strip_prefix("Content-Length: ")).unwrap().trim().parse().unwrap();
                        if raw.len() >= end + 4 + len {
                            got.push(raw[end + 4..end + 4 + len].to_vec());
                            break;
                        }
                    }
                    assert!(n > 0, "request cut short");
                }
                let mut answer = format!("HTTP/1.1 200 OK\r\nContent-Type: application/ipp\r\nTransfer-Encoding: chunked\r\n\r\n{:x}\r\n", reply.len()).into_bytes();
                answer.extend_from_slice(&reply);
                answer.extend_from_slice(b"\r\n0\r\n\r\n");
                s.write_all(&answer).unwrap();
            }
            got
        });
        (Endpoint::Unix(sock), handle, dir)
    }

    fn ipp_test_id() -> u32 {
        static N: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
        N.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    }

    /// CUPS-Get-Printers' answer: one printer (the class lookup) and an empty classes list.
    fn printers_reply(name: &str) -> Vec<u8> {
        let mut r = Request::new(0x0000, 1);
        r.group(craft_ipp::TAG_PRINTER_GROUP);
        r.attr(craft_ipp::TAG_NAME, "printer-name", name.as_bytes());
        r.end()
    }

    fn not_found() -> Vec<u8> {
        Request::new(0x0406, 1).end()
    }

    #[test]
    fn asks_a_server_for_paper_sizes() {
        // the class lookup (printers, classes, default), then Get-Printer-Attributes
        let (ep, server, dir) = fake_server(vec![printers_reply("Office"), not_found(), not_found(), printer_reply()]);
        let m = ipp::media(&ep, "Office").unwrap();
        let sent = server.join().unwrap();
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(m.default.as_deref(), Some("iso_a4_210x297mm"));
        assert_eq!(m.dpi, Some(600));
        let (op, attrs, _) = craft_ipp::parse_request(&sent[3]).unwrap();
        assert_eq!(op, 0x000B);
        let uri = attrs.iter().find(|a| a.name == "printer-uri").unwrap().text();
        assert_eq!(uri, "ipp://localhost/printers/Office");
    }

    #[test]
    fn sends_a_job_with_its_paper() {
        let mut accepted = Request::new(0x0000, 2);
        accepted.group(craft_ipp::TAG_JOB_GROUP);
        accepted.integer("job-id", 42);
        let (ep, server, dir) = fake_server(vec![printers_reply("Office Jet"), not_found(), not_found(), accepted.end()]);
        let pdf = b"%PDF-1.4 test";
        ipp::print(&ep, "Office Jet", Some("na_index-4x6_4x6in"), "AlbumCraft pictures", pdf).unwrap();
        let sent = server.join().unwrap();
        let _ = std::fs::remove_dir_all(&dir);
        let (op, attrs, document) = craft_ipp::parse_request(&sent[3]).unwrap();
        assert_eq!(op, 0x0002);
        let get = |n: &str| attrs.iter().find(|a| a.name == n).map(|a| a.text());
        assert_eq!(get("printer-uri").as_deref(), Some("ipp://localhost/printers/Office%20Jet"));
        assert_eq!(get("media").as_deref(), Some("na_index-4x6_4x6in"));
        assert_eq!(get("print-scaling").as_deref(), Some("none"));
        assert_eq!(get("document-format").as_deref(), Some("application/pdf"));
        assert_eq!(get("job-name").as_deref(), Some("AlbumCraft pictures"));
        assert_eq!(document, pdf);
    }

    #[test]
    fn a_refused_job_says_why() {
        let mut refused = Request::new(0x0506, 2);
        refused.attr(craft_ipp::TAG_TEXT, "status-message", b"Printer is not accepting jobs.");
        let (ep, server, dir) = fake_server(vec![printers_reply("Office"), not_found(), not_found(), refused.end()]);
        let err = ipp::print(&ep, "Office", None, "t", b"%PDF").unwrap_err();
        server.join().unwrap();
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(err, "Printer is not accepting jobs.");
        assert!(ipp::print(&ep, "../x", None, "t", b"%PDF").is_err());
    }

    /// The printers, default printer and paper of this machine's CUPS, through IPP (nothing is
    /// printed). `cargo test real_cups -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn real_cups() {
        let ep = Endpoint::detect().expect("no CUPS server");
        println!("server: {ep}");
        let printers = craft_ipp::printers(&ep).unwrap();
        println!("printers: {printers:?}\ndefault: {:?}", craft_ipp::default_printer(&ep));
        for p in printers {
            println!("{}: {:?}", p.name, ipp::media(&ep, &p.name));
        }
    }
}
