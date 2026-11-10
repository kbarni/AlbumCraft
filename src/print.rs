//! Printing: sending pages to a printer through Win32 GDI (Windows) or CUPS (Linux, macOS).
//! The layouts and page rendering are `albumcraft_core::print` (re-exported here).

pub use albumcraft_core::print::*;
use image::RgbaImage;

// ---------------------------------------------------------------------------------------------
// Windows printers

/// Whether printing works here: always on Windows (GDI); on Linux and macOS when the CUPS print
/// service is running. Without it the print commands are hidden.
pub fn available() -> bool {
    #[cfg(windows)]
    {
        true
    }
    #[cfg(not(windows))]
    {
        static UP: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
        *UP.get_or_init(cups::running)
    }
}

/// A chosen printer and its settings: on Windows, after "Printer Setup", the raw DEVMODE bytes;
/// with CUPS, the chosen paper size (None = the printer's default).
#[derive(Clone, Debug, Default)]
pub struct PrinterChoice {
    pub name: String,
    #[cfg_attr(not(windows), allow(dead_code))] // only Windows printing reads it
    pub devmode: Option<Vec<u8>>,
    #[cfg_attr(windows, allow(dead_code))] // only CUPS printing reads it
    pub media: Option<String>,
}

#[cfg(windows)]
mod win {
    use super::*;
    use windows::core::{PCWSTR, PWSTR};
    use windows::Win32::Foundation::{GlobalFree, HGLOBAL};
    use windows::Win32::Graphics::Gdi::*;
    use windows::Win32::Graphics::Printing::*;
    use windows::Win32::Storage::Xps::*;
    use windows::Win32::System::Memory::*;
    use windows::Win32::UI::Controls::Dialogs::*;

    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }

    /// # Safety
    /// `p` is null or points to a NUL-terminated UTF-16 string that stays valid during the call.
    unsafe fn from_wide(p: *const u16) -> String {
        if p.is_null() {
            return String::new();
        }
        let mut n = 0;
        // SAFETY: the caller guarantees a terminator, so every read up to it is in bounds.
        while unsafe { *p.add(n) } != 0 {
            n += 1;
        }
        // SAFETY: the n characters before the terminator were just read.
        String::from_utf16_lossy(unsafe { std::slice::from_raw_parts(p, n) })
    }

    pub fn printers() -> Vec<String> {
        let flags = PRINTER_ENUM_LOCAL | PRINTER_ENUM_CONNECTIONS;
        let (mut needed, mut count) = (0u32, 0u32);
        // SAFETY: a size query: no buffer, only the two out-parameters are written.
        let _ = unsafe { EnumPrintersW(flags, PCWSTR::null(), 4, None, &mut needed, &mut count) };
        if needed == 0 {
            return vec![];
        }
        // u64 words, so the PRINTER_INFO_4W records at the start of the buffer are aligned
        let words = (needed as usize).div_ceil(8);
        let mut buf = vec![0u64; words];
        let len = words * 8;
        // SAFETY: the same allocation viewed as `len` bytes; it lives until the end of this function.
        let bytes = unsafe { std::slice::from_raw_parts_mut(buf.as_mut_ptr().cast::<u8>(), len) };
        // SAFETY: the buffer is at least `needed` bytes, as the size query asked.
        if unsafe { EnumPrintersW(flags, PCWSTR::null(), 4, Some(bytes), &mut needed, &mut count) }.is_err() {
            return vec![];
        }
        let count = (count as usize).min(len / std::mem::size_of::<PRINTER_INFO_4W>());
        // SAFETY: EnumPrintersW wrote `count` records at the start of the 8-byte-aligned buffer (bounded
        // by its size above); their name pointers point to strings inside the same buffer.
        let infos = unsafe { std::slice::from_raw_parts(buf.as_ptr().cast::<PRINTER_INFO_4W>(), count) };
        infos.iter().map(|i| unsafe { from_wide(i.pPrinterName.0) }).filter(|s| !s.is_empty()).collect()
    }

    pub fn default_printer() -> Option<String> {
        // SAFETY: a size query, then a call with a buffer of exactly the size it asked for; the
        // name is read from our own buffer, which Windows NUL-terminates.
        unsafe {
            let mut n = 0u32;
            let _ = GetDefaultPrinterW(None, &mut n);
            if n == 0 {
                return None;
            }
            let mut buf = vec![0u16; n as usize];
            if GetDefaultPrinterW(Some(PWSTR(buf.as_mut_ptr())), &mut n).as_bool() {
                Some(from_wide(buf.as_ptr()))
            } else {
                None
            }
        }
    }

    /// # Safety
    /// `p.devmode`, when set, holds a complete DEVMODEW as returned by the print dialog.
    unsafe fn open_dc(p: &PrinterChoice) -> Option<HDC> {
        let name = wide(&p.name);
        let dm = p.devmode.as_ref().map(|d| d.as_ptr() as *const DEVMODEW);
        // SAFETY: `name` is NUL-terminated and outlives the call; the DEVMODE is complete (see above).
        let dc = unsafe { CreateDCW(PCWSTR::null(), PCWSTR(name.as_ptr()), PCWSTR::null(), dm) };
        (!dc.is_invalid()).then_some(dc)
    }

    pub fn paper(p: &PrinterChoice) -> Option<Paper> {
        // SAFETY: the device context is created, queried and deleted here; the DEVMODE comes from
        // the print dialog only.
        unsafe {
            let dc = open_dc(p)?;
            let cap = |i| GetDeviceCaps(Some(dc), i) as f32;
            let (dx, dy) = (cap(LOGPIXELSX).max(1.0), cap(LOGPIXELSY).max(1.0));
            let mm = |v: f32, d: f32| v / d * 25.4;
            let paper = Paper {
                sheet_w: mm(cap(PHYSICALWIDTH), dx),
                sheet_h: mm(cap(PHYSICALHEIGHT), dy),
                area_x: mm(cap(PHYSICALOFFSETX), dx),
                area_y: mm(cap(PHYSICALOFFSETY), dy),
                area_w: mm(cap(HORZRES), dx),
                area_h: mm(cap(VERTRES), dy),
                dpi: dx as u32,
            };
            let _ = DeleteDC(dc);
            (paper.area_w > 10.0 && paper.area_h > 10.0).then_some(paper)
        }
    }

    /// The Windows "Print Setup" dialog: printer, paper size, orientation.
    pub fn setup_dialog(current: &PrinterChoice) -> Option<PrinterChoice> {
        // SAFETY: the global memory blocks are allocated with room for what is copied into them,
        // locked while used, handed to PrintDlgW and freed once; DEVMODE/DEVNAMES read back are
        // the ones Windows filled in, read within their own sizes (dmSize + dmDriverExtra).
        unsafe {
            let mut pd = PRINTDLGW { lStructSize: std::mem::size_of::<PRINTDLGW>() as u32, Flags: PD_PRINTSETUP, ..Default::default() };
            // start from the current printer and its settings
            if let Some(dm) = &current.devmode {
                if let Ok(h) = GlobalAlloc(GMEM_MOVEABLE, dm.len()) {
                    let ptr = GlobalLock(h) as *mut u8;
                    if !ptr.is_null() {
                        std::ptr::copy_nonoverlapping(dm.as_ptr(), ptr, dm.len());
                        let _ = GlobalUnlock(h);
                        pd.hDevMode = h;
                    }
                }
            } else if !current.name.is_empty() {
                // a DEVNAMES block naming the printer
                let name = wide(&current.name);
                let size = std::mem::size_of::<DEVNAMES>() + name.len() * 2 + 2;
                if let Ok(h) = GlobalAlloc(GMEM_MOVEABLE | GMEM_ZEROINIT, size) {
                    let ptr = GlobalLock(h) as *mut u8;
                    if !ptr.is_null() {
                        let off = (std::mem::size_of::<DEVNAMES>() / 2) as u16;
                        let dn = ptr as *mut DEVNAMES;
                        (*dn).wDriverOffset = off + name.len() as u16; // empty driver string
                        (*dn).wDeviceOffset = off;
                        (*dn).wOutputOffset = off + name.len() as u16;
                        std::ptr::copy_nonoverlapping(name.as_ptr(), ptr.add(off as usize * 2) as *mut u16, name.len());
                        let _ = GlobalUnlock(h);
                        pd.hDevNames = h;
                    }
                }
            }
            let ok = PrintDlgW(&mut pd).as_bool();
            let mut result = None;
            if ok {
                let mut name = current.name.clone();
                if !pd.hDevNames.is_invalid() {
                    let ptr = GlobalLock(pd.hDevNames) as *const u8;
                    if !ptr.is_null() {
                        let dn = &*(ptr as *const DEVNAMES);
                        name = from_wide(ptr.add(dn.wDeviceOffset as usize * 2) as *const u16);
                        let _ = GlobalUnlock(pd.hDevNames);
                    }
                }
                let mut devmode = None;
                if !pd.hDevMode.is_invalid() {
                    let ptr = GlobalLock(pd.hDevMode) as *const u8;
                    if !ptr.is_null() {
                        let dm = &*(ptr as *const DEVMODEW);
                        let len = dm.dmSize as usize + dm.dmDriverExtra as usize;
                        devmode = Some(std::slice::from_raw_parts(ptr, len).to_vec());
                        let _ = GlobalUnlock(pd.hDevMode);
                    }
                }
                result = Some(PrinterChoice { name, devmode, media: None });
            }
            for h in [pd.hDevMode, pd.hDevNames] {
                if !h.is_invalid() {
                    let _ = GlobalFree(Some(HGLOBAL(h.0)));
                }
            }
            result
        }
    }

    /// Send rendered pages (each covering the printable area) to the printer. `to_file` makes the
    /// driver write its output there (for example a PDF with "Microsoft Print to PDF").
    pub fn print_pages(p: &PrinterChoice, doc: &str, pages: &mut dyn Iterator<Item = RgbaImage>, to_file: Option<&str>) -> Result<usize, String> {
        // SAFETY: the strings passed in DOCINFOW live until EndDoc; each page's pixel buffer is
        // exactly width x height x 4 bytes as the BITMAPINFO describes, and lives during StretchDIBits.
        unsafe {
            let dc = open_dc(p).ok_or("The printer could not be opened.")?;
            let doc_w = wide(doc);
            let out_w = to_file.map(wide);
            let info = DOCINFOW {
                cbSize: std::mem::size_of::<DOCINFOW>() as i32,
                lpszDocName: PCWSTR(doc_w.as_ptr()),
                lpszOutput: out_w.as_ref().map(|o| PCWSTR(o.as_ptr())).unwrap_or(PCWSTR::null()),
                ..Default::default()
            };
            if StartDocW(dc, &info) <= 0 {
                let _ = DeleteDC(dc);
                return Err("Printing was cancelled or the printer refused the job.".into());
            }
            // the pages are rendered for the printable area, which is exactly HORZRES x VERTRES
            let (hw, hh) = (GetDeviceCaps(Some(dc), HORZRES), GetDeviceCaps(Some(dc), VERTRES));
            let mut n = 0;
            let mut failed = false;
            for page in pages {
                if StartPage(dc) <= 0 {
                    failed = true;
                    break;
                }
                let (w, h) = (page.width() as i32, page.height() as i32);
                // 32-bit top-down BGRA
                let mut bgra = page.into_raw();
                for px in bgra.as_chunks_mut::<4>().0 {
                    px.swap(0, 2);
                }
                let bmi = BITMAPINFO {
                    bmiHeader: BITMAPINFOHEADER {
                        biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                        biWidth: w,
                        biHeight: -h,
                        biPlanes: 1,
                        biBitCount: 32,
                        biCompression: BI_RGB.0,
                        ..Default::default()
                    },
                    ..Default::default()
                };
                SetStretchBltMode(dc, HALFTONE);
                let lines = StretchDIBits(dc, 0, 0, hw, hh, 0, 0, w, h, Some(bgra.as_ptr() as *const _), &bmi, DIB_RGB_COLORS, SRCCOPY);
                if EndPage(dc) <= 0 || lines == 0 {
                    failed = true;
                    break;
                }
                n += 1;
            }
            let _ = EndDoc(dc);
            let _ = DeleteDC(dc);
            if failed && n == 0 { Err("The printer stopped before the first page.".into()) } else { Ok(n) }
        }
    }
}

#[cfg(windows)]
pub use win::{default_printer, paper, print_pages, printers, setup_dialog};

/// Paper sizes to choose from in the print screen (CUPS); empty on Windows, where the printer's
/// own setup dialog does that.
#[cfg(windows)]
pub fn paper_sizes(_: &str) -> Vec<String> {
    vec![]
}

/// Printing on Linux and macOS through CUPS's command-line tools (`lpstat`, `lpoptions`, `lp`):
/// the pages go to the printer as a PDF of the chosen paper size.
#[cfg(not(windows))]
mod cups {
    use super::*;
    use std::process::{Command, Stdio};

    fn run(cmd: &str, args: &[&str]) -> Option<String> {
        let out = Command::new(cmd).args(args).env("LC_ALL", "C").stdin(Stdio::null()).stderr(Stdio::null()).output().ok()?;
        out.status.success().then(|| String::from_utf8_lossy(&out.stdout).to_string())
    }

    pub fn running() -> bool {
        run("lpstat", &["-r"]).is_some_and(|o| o.contains("scheduler is running"))
    }

    pub fn printers() -> Vec<String> {
        let mut names: Vec<String> = run("lpstat", &["-e"]).map(|o| o.lines().map(|l| l.trim().to_string()).filter(|l| !l.is_empty()).collect()).unwrap_or_default();
        if names.is_empty() {
            // older CUPS: "printer NAME is idle. ..."
            names = run("lpstat", &["-p"])
                .map(|o| o.lines().filter_map(|l| l.strip_prefix("printer ")?.split_whitespace().next().map(String::from)).collect())
                .unwrap_or_default();
        }
        names
    }

    pub fn default_printer() -> Option<String> {
        let o = run("lpstat", &["-d"])?;
        let name = o.split_once(':')?.1.trim();
        (!name.is_empty() && !o.contains("no system default")).then(|| name.to_string())
    }

    /// The printer's options from `lpoptions -l`: (choices, default) for `key`.
    fn option(printer: &str, key: &str) -> (Vec<String>, Option<String>) {
        let Some(o) = run("lpoptions", &["-p", printer, "-l"]) else { return (vec![], None) };
        parse_option(&o, key)
    }

    pub(super) fn parse_option(listing: &str, key: &str) -> (Vec<String>, Option<String>) {
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

    /// Width and height in millimetres of a CUPS / PPD / PWG paper name.
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
        // PPD variants like "A4.Borderless" have the size before the dot
        let named = name.split('.').next().unwrap_or(name);
        if let Some((_, w, h)) = known.iter().find(|(n, _, _)| n.eq_ignore_ascii_case(named)) {
            return Some((*w, *h));
        }
        let base = name;
        // "w288h432": points
        if let Some(rest) = base.strip_prefix('w')
            && let Some((w, h)) = rest.split_once('h')
            && let (Ok(w), Ok(h)) = (w.parse::<f32>(), h.parse::<f32>())
        {
            return Some((w / 72.0 * 25.4, h / 72.0 * 25.4));
        }
        // PWG self-describing names: "iso_a4_210x297mm", "na_letter_8.5x11in"
        let last = base.rsplit('_').next()?;
        let (dims, unit) = if let Some(d) = last.strip_suffix("mm") { (d, 1.0) } else { (last.strip_suffix("in")?, 25.4) };
        let (w, h) = dims.split_once('x')?;
        Some((w.parse::<f32>().ok()? * unit, h.parse::<f32>().ok()? * unit))
    }

    pub fn paper_sizes(printer: &str) -> Vec<String> {
        option(printer, "PageSize").0.into_iter().filter(|n| size_mm(n).is_some()).collect()
    }

    pub fn paper(p: &PrinterChoice) -> Option<Paper> {
        if p.name.is_empty() {
            return None;
        }
        let (sizes, default) = option(&p.name, "PageSize");
        let name = p.media.clone().or(default).or_else(|| sizes.first().cloned()).unwrap_or_else(|| "A4".into());
        let (w, h) = size_mm(&name)?;
        let dpi = option(&p.name, "Resolution").1.and_then(|r| r.split(['x', 'd']).next()?.parse::<u32>().ok()).unwrap_or(300);
        // CUPS doesn't tell the unprintable edge here; 5 mm suits nearly every printer
        let m = 5.0;
        Some(Paper { sheet_w: w, sheet_h: h, area_x: m, area_y: m, area_w: w - 2.0 * m, area_h: h - 2.0 * m, dpi: dpi.clamp(72, 2400) })
    }

    pub fn setup_dialog(_: &PrinterChoice) -> Option<PrinterChoice> {
        None
    }

    /// Print the pages (each covering the printable area), or write the PDF to `to_file`.
    pub fn print_pages(p: &PrinterChoice, doc: &str, pages: &mut dyn Iterator<Item = RgbaImage>, to_file: Option<&str>) -> Result<usize, String> {
        let paper = paper(p).unwrap_or_else(Paper::a4);
        let mut jpegs = vec![];
        for page in pages {
            let rgb = image::DynamicImage::ImageRgba8(page).to_rgb8();
            let mut buf = vec![];
            rgb.write_with_encoder(image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buf, 92)).map_err(|e| e.to_string())?;
            jpegs.push((buf, rgb.width(), rgb.height()));
        }
        if jpegs.is_empty() {
            return Err("nothing to print".into());
        }
        let pdf = pdf_of_pages(&jpegs, &paper);
        let n = jpegs.len();
        if let Some(path) = to_file {
            std::fs::write(path, pdf).map_err(|e| format!("{path}: {e}"))?;
            return Ok(n);
        }
        if p.name.is_empty() {
            return Err("no printer chosen".into());
        }
        let file = std::env::temp_dir().join(format!("albumcraft-print-{}-{}.pdf", std::process::id(), std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis()).unwrap_or(0)));
        std::fs::write(&file, pdf).map_err(|e| format!("{}: {e}", file.display()))?;
        let title: String = doc.chars().filter(|c| !c.is_control()).take(100).collect();
        let mut cmd = Command::new("lp");
        cmd.env("LC_ALL", "C").args(["-d", &p.name, "-t", &title]);
        if let Some(m) = &p.media {
            cmd.args(["-o", &format!("media={m}")]);
        }
        // "--" so a file name can never be read as an option
        let out = cmd.arg("--").arg(&file).stdin(Stdio::null()).output();
        let _ = std::fs::remove_file(&file);
        let out = out.map_err(|e| format!("lp could not be started: {e}"))?;
        if !out.status.success() {
            let err = String::from_utf8_lossy(&out.stderr).trim().to_string();
            return Err(if err.is_empty() { "the print service refused the job".into() } else { err });
        }
        Ok(n)
    }
}
#[cfg(not(windows))]
pub use cups::{default_printer, paper, paper_sizes, print_pages, printers, setup_dialog};

#[cfg(test)]
mod tests {
    use super::*;
    use image::Rgba;

    #[cfg(not(windows))]
    #[test]
    fn cups_option_and_paper_names() {
        let listing = "PageSize/Page Size: Custom.WIDTHxHEIGHT Letter *A4 11x17 A3 EnvDL\nResolution/Resolution: 300x300dpi *600x600dpi\n";
        assert_eq!(cups::parse_option(listing, "PageSize").1.as_deref(), Some("A4"));
        assert_eq!(cups::parse_option(listing, "Resolution").1.as_deref(), Some("600x600dpi"));
        assert_eq!(cups::parse_option(listing, "Duplex"), (vec![], None));
        assert_eq!(cups::size_mm("A4"), Some((210.0, 297.0)));
        assert_eq!(cups::size_mm("a4"), Some((210.0, 297.0)));
        assert_eq!(cups::size_mm("w288h432"), Some((101.6, 152.4)));
        assert_eq!(cups::size_mm("iso_a5_148x210mm"), Some((148.0, 210.0)));
        let l = cups::size_mm("na_letter_8.5x11in").unwrap();
        assert!((l.0 - 215.9).abs() < 0.01 && (l.1 - 279.4).abs() < 0.01);
        assert_eq!(cups::size_mm("EnvDL"), None);
        assert_eq!(cups::size_mm("Custom.WIDTHxHEIGHT"), None);
    }

    #[cfg(not(windows))]
    #[test]
    fn cups_writes_a_pdf_to_a_file_without_printing() {
        // `to_file` writes the PDF instead of sending it, so this never reaches a real printer
        let paper = Paper::a4();
        let page = RgbaImage::from_pixel(200, 287, Rgba([200, 30, 30, 255]));
        let out = std::env::temp_dir().join(format!("albumcraft-cups-{}.pdf", std::process::id()));
        let choice = PrinterChoice { name: String::new(), ..Default::default() };
        let n = print_pages(&choice, "test", &mut vec![page.clone(), page].into_iter(), Some(&out.to_string_lossy())).unwrap();
        let pdf = std::fs::read(&out).unwrap();
        let _ = std::fs::remove_file(&out);
        assert_eq!(n, 2);
        let text = String::from_utf8_lossy(&pdf);
        assert!(pdf.starts_with(b"%PDF-1.4") && text.contains("/Count 2") && text.trim_end().ends_with("%%EOF"));
        assert!(text.contains("/MediaBox [0 0 595.28 841.89]"), "A4 in points");
        // the cross-reference table points at the objects
        let xref: usize = text.rsplit("startxref").next().unwrap().split_whitespace().next().unwrap().parse().unwrap();
        assert!(pdf[xref..].starts_with(b"xref"));
        let _ = paper;
    }

    /// `ALBUMCRAFT_PDF_OUT=out.pdf cargo test cups_pdf_demo -- --ignored`: a two-page sample PDF
    /// (A4, one 10 x 15 layout page) to open in a viewer; nothing is printed.
    #[cfg(not(windows))]
    #[test]
    #[ignore]
    fn cups_pdf_demo() {
        let out = std::env::var("ALBUMCRAFT_PDF_OUT").expect("set ALBUMCRAFT_PDF_OUT");
        let paper = Paper::a4();
        let cells = page_cells(&paper, PrintLayout::Cm10x15);
        let img = crate::imaging::sample_image(4, 1200, 900);
        let items: Vec<PageItem> = (0..cells.len()).map(|i| PageItem { image: img.clone(), caption: format!("Sample {i}") }).collect();
        let settings = PrintSettings { caption: CaptionMode::FileName, border: BorderStyle::ThinBlack, ..Default::default() };
        let page = render_page(&paper, 150, &cells, &items, &settings);
        let choice = PrinterChoice::default();
        assert_eq!(print_pages(&choice, "demo", &mut vec![page.clone(), page].into_iter(), Some(&out)).unwrap(), 2);
    }

    /// Real print through "Microsoft Print to PDF" into a file (run with `cargo test -- --ignored`).
    #[cfg(windows)]
    #[test]
    #[ignore]
    fn prints_to_pdf() {
        let choice = PrinterChoice { name: "Microsoft Print to PDF".into(), ..Default::default() };
        let paper = paper(&choice).expect("printer");
        let cells = page_cells(&paper, PrintLayout::Cm10x15);
        let img = crate::imaging::sample_image(4, 1200, 900);
        let items: Vec<PageItem> = (0..cells.len()).map(|i| PageItem { image: img.clone(), caption: format!("Sample {i}") }).collect();
        let settings = PrintSettings { caption: CaptionMode::FileName, ..Default::default() };
        let page = render_page(&paper, 150, &cells, &items, &settings);
        let out = std::env::temp_dir().join("albumcraft-print-test.pdf");
        let _ = std::fs::remove_file(&out);
        let n = print_pages(&choice, "AlbumCraft test", &mut std::iter::once(page), Some(out.to_str().unwrap())).unwrap();
        assert_eq!(n, 1);
        std::thread::sleep(std::time::Duration::from_secs(3));
        assert!(out.exists(), "no PDF written");
    }
}
