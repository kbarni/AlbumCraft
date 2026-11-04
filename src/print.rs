//! Printing: page layouts (5 x 8 cm … full page), shrink- or
//! crop-to-fit, borders and captions, rendering the pages, and sending them to a Windows printer.

use image::{imageops, Rgba, RgbaImage};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize, Default)]
pub enum PrintLayout {
    Cm5x8,
    Cm9x13,
    #[default]
    Cm10x15,
    Cm13x18,
    Cm20x25,
    FullPage,
}

impl PrintLayout {
    pub const ALL: [PrintLayout; 6] =
        [PrintLayout::Cm5x8, PrintLayout::Cm9x13, PrintLayout::Cm10x15, PrintLayout::Cm13x18, PrintLayout::Cm20x25, PrintLayout::FullPage];
    pub fn label(&self) -> &'static str {
        match self {
            PrintLayout::Cm5x8 => "5 x 8 cm",
            PrintLayout::Cm9x13 => "9 x 13 cm",
            PrintLayout::Cm10x15 => "10 x 15 cm",
            PrintLayout::Cm13x18 => "13 x 18 cm",
            PrintLayout::Cm20x25 => "20 x 25 cm",
            PrintLayout::FullPage => "Full Page",
        }
    }
    /// Print size in millimetres (short side, long side); None = the whole printable area.
    pub fn size_mm(&self) -> Option<(f32, f32)> {
        // stored in cm and turned into inches with CM_TO_INCH
        let cm = |a: f32, b: f32| Some((a * INCH_PER_CM * 25.4, b * INCH_PER_CM * 25.4));
        match self {
            PrintLayout::Cm5x8 => cm(5.0, 8.0),
            PrintLayout::Cm9x13 => cm(9.0, 13.0),
            PrintLayout::Cm10x15 => cm(10.0, 15.0),
            PrintLayout::Cm13x18 => cm(13.0, 18.0),
            PrintLayout::Cm20x25 => cm(20.0, 25.0),
            PrintLayout::FullPage => None,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize, Default)]
pub enum BorderStyle {
    #[default]
    None,
    /// a thin black line around each picture
    ThinBlack,
    /// a white margin inside the print size
    White,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize, Default)]
pub enum CaptionMode {
    #[default]
    None,
    FileName,
    Caption,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct PrintSettings {
    pub layout: PrintLayout,
    /// true = "Crop to Fit" (fill the print size), false = "Shrink to Fit" (whole picture visible)
    pub crop: bool,
    pub border: BorderStyle,
    pub caption: CaptionMode,
    pub copies: u32,
    pub printer: Option<String>,
}

impl Default for PrintSettings {
    fn default() -> Self {
        Self { layout: PrintLayout::Cm10x15, crop: false, border: BorderStyle::None, caption: CaptionMode::None, copies: 1, printer: None }
    }
}

/// The paper as the printer reports it (all in millimetres, printable area relative to the sheet).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Paper {
    pub sheet_w: f32,
    pub sheet_h: f32,
    pub area_x: f32,
    pub area_y: f32,
    pub area_w: f32,
    pub area_h: f32,
    pub dpi: u32,
}

impl Paper {
    /// A4 portrait with 5 mm margins, used when no printer answers.
    pub fn a4() -> Paper {
        Paper { sheet_w: 210.0, sheet_h: 297.0, area_x: 5.0, area_y: 5.0, area_w: 200.0, area_h: 287.0, dpi: 300 }
    }
    pub fn landscape(&self) -> bool {
        self.sheet_w > self.sheet_h
    }
    /// Printer line, e.g. "600 dpi, 8.3 x 11.7 in (Portrait)".
    pub fn describe(&self) -> String {
        format!(
            "{} dpi, {:.1} x {:.1} in ({})",
            self.dpi,
            self.sheet_w / 25.4,
            self.sheet_h / 25.4,
            if self.landscape() { "Landscape" } else { "Portrait" }
        )
    }
}

/// One picture position on a page, in millimetres inside the printable area.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Cell {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

/// Layout constants of the print grid.
pub const FIT_TOLERANCE: f32 = 0.999;
pub const MIN_GAP_MM: f32 = 0.2 * 25.4;
pub const SHRINK_WHEN_TIGHT: f32 = 0.975;
/// cm → inch factor for the metric sizes.
pub const INCH_PER_CM: f32 = 0.3937;

/// Room kept under each picture for its caption.
pub const CAPTION_MM: f32 = 6.0;
/// White border width.
pub const WHITE_BORDER_MM: f32 = 4.0;

/// The grid of print positions for one page: as many prints of the chosen size as fit, turned
/// whichever way fits more, spread evenly over the printable area.
pub fn page_cells(paper: &Paper, layout: PrintLayout) -> Vec<Cell> {
    let (aw, ah) = (paper.area_w, paper.area_h);
    let Some((s, l)) = layout.size_mm() else {
        return vec![Cell { x: 0.0, y: 0.0, w: aw, h: ah }];
    };
    // A print still counts as fitting when 99.9 % of it fits (size × 0.999).
    let count = |avail: f32, size: f32| ((avail + (1.0 - FIT_TOLERANCE) * size) / size).floor().max(0.0) as usize;
    let fit = |cw: f32, ch: f32| (count(aw, cw), count(ah, ch));
    // Lay the job out both ways and keeps the one needing fewer pages; a tie keeps the
    // first try (the print held upright).
    let (pc, pr) = fit(s, l); // portrait prints
    let (lc, lr) = fit(l, s); // landscape prints
    let (cols, rows, mut cw, mut ch) = if lc * lr > pc * pr { (lc, lr, l, s) } else { (pc, pr, s, l) };
    if cols == 0 || rows == 0 {
        // larger than the paper: one print, as big as fits with the same shape
        let k = (aw / cw).min(ah / ch);
        let (w, h) = (cw * k, ch * k);
        return vec![Cell { x: (aw - w) / 2.0, y: (ah - h) / 2.0, w, h }];
    }
    // The space left over is shared out between the prints; with more than one print in a
    // row/column the gap is at least 0.2 inch, and when there is less room the prints
    // are shrunk to 97.5 %. The leftover is spread evenly.
    let spread = |n: usize, avail: f32, size: &mut f32| -> f32 {
        let mut g = (avail - n as f32 * *size) / (n as f32 + 1.0);
        if n > 1 && g < MIN_GAP_MM {
            *size *= SHRINK_WHEN_TIGHT;
            g = ((avail - n as f32 * *size) / (n as f32 + 1.0)).max(0.0);
        }
        g.max(0.0)
    };
    let gx = spread(cols, aw, &mut cw);
    let gy = spread(rows, ah, &mut ch);
    let mut cells = Vec::new();
    for r in 0..rows {
        for c in 0..cols {
            cells.push(Cell { x: gx + c as f32 * (cw + gx), y: gy + r as f32 * (ch + gy), w: cw, h: ch });
        }
    }
    cells
}

/// Where the picture goes inside a cell, after the border and caption room are taken off:
/// (picture box in mm, rotate the picture a quarter turn clockwise to match the cell's shape).
pub fn picture_box(cell: &Cell, img_w: u32, img_h: u32, settings: &PrintSettings) -> (Cell, bool) {
    let mut b = *cell;
    if settings.border == BorderStyle::White {
        b = Cell { x: b.x + WHITE_BORDER_MM, y: b.y + WHITE_BORDER_MM, w: (b.w - 2.0 * WHITE_BORDER_MM).max(1.0), h: (b.h - 2.0 * WHITE_BORDER_MM).max(1.0) };
    }
    if settings.caption != CaptionMode::None {
        b.h = (b.h - CAPTION_MM).max(1.0);
    }
    // full page keeps the picture's own orientation only when the page has the same shape
    let rotate = (img_w > img_h) != (b.w > b.h) && img_w != img_h;
    (b, rotate)
}

/// How the (possibly rotated) picture of aspect `img_aspect` (w/h) sits in box `b`:
/// returns the drawn rectangle (mm) and the visible part of the picture as fractions (x0, y0, x1, y1).
pub fn fit_or_crop(b: &Cell, img_aspect: f32, crop: bool) -> (Cell, [f32; 4]) {
    let box_aspect = b.w / b.h;
    if crop {
        if img_aspect > box_aspect {
            let vis = box_aspect / img_aspect;
            (*b, [(1.0 - vis) / 2.0, 0.0, (1.0 + vis) / 2.0, 1.0])
        } else {
            let vis = img_aspect / box_aspect;
            (*b, [0.0, (1.0 - vis) / 2.0, 1.0, (1.0 + vis) / 2.0])
        }
    } else if img_aspect > box_aspect {
        let h = b.w / img_aspect;
        (Cell { x: b.x, y: b.y + (b.h - h) / 2.0, w: b.w, h }, [0.0, 0.0, 1.0, 1.0])
    } else {
        let w = b.h * img_aspect;
        (Cell { x: b.x + (b.w - w) / 2.0, y: b.y, w, h: b.h }, [0.0, 0.0, 1.0, 1.0])
    }
}

/// How many pages a print job needs.
pub fn page_count(pictures: usize, copies: u32, per_page: usize) -> usize {
    let n = pictures * copies.max(1) as usize;
    if n == 0 || per_page == 0 { 0 } else { n.div_ceil(per_page) }
}

/// Quality grades shown in the print review:
/// 2 = "Best quality (%d pixels/inch)", 1 = "Good quality (%d pixels/inch)", 0 = "Bad quality (%d pixels/inch)";
/// any grade 0 makes it ask "Some of your pictures are too small to print well…".
/// The pixels-per-inch limits between the grades were not found in the code — estimated.
pub const BEST_PPI: f32 = 200.0;
pub const GOOD_PPI: f32 = 150.0;

pub fn quality_grade(ppi: f32) -> u8 {
    if ppi >= BEST_PPI {
        2
    } else if ppi >= GOOD_PPI {
        1
    } else {
        0
    }
}

pub fn quality_text(grade: u8, ppi: f32) -> String {
    let p = ppi.round() as i64;
    match grade {
        2 => format!("Best quality ({p} pixels/inch)"),
        1 => format!("Good quality ({p} pixels/inch)"),
        _ => format!("Bad quality ({p} pixels/inch)"),
    }
}

pub const MSG_READY: &str = "All of your pictures are ready to print.";
pub const MSG_TOO_SMALL: &str = "Some of your pictures are too small to print well.  You can remove these pictures, print them anyway, or cancel and change the print size.";
pub const MSG_NONE_LEFT: &str = "There are no pictures left to print.";

// ---------------------------------------------------------------------------------------------
// Rendering a page into pixels

pub struct PageItem {
    /// the edited picture at a generous size
    pub image: RgbaImage,
    pub caption: String,
}

fn draw_text(img: &mut RgbaImage, text: &str, cx: f32, top: f32, px: f32, max_w: f32) {
    use ab_glyph::{point, Font, FontRef, PxScale, ScaleFont};
    let Ok(font) = FontRef::try_from_slice(epaint_default_fonts::UBUNTU_LIGHT) else { return };
    let scale = PxScale::from(px);
    let sf = font.as_scaled(scale);
    // shorten with "…" when too wide
    let mut chars: Vec<char> = text.chars().collect();
    let width = |cs: &[char]| cs.iter().map(|c| sf.h_advance(font.glyph_id(*c))).sum::<f32>();
    while chars.len() > 1 && width(&chars) > max_w {
        chars.pop();
        if let Some(l) = chars.last_mut() {
            *l = '…';
        }
    }
    let mut x = cx - width(&chars) / 2.0;
    let base = top + sf.ascent();
    let (iw, ih) = (img.width() as i32, img.height() as i32);
    for c in chars {
        let id = font.glyph_id(c);
        if let Some(o) = font.outline_glyph(id.with_scale_and_position(scale, point(x, base))) {
            let bb = o.px_bounds();
            o.draw(|gx, gy, cov| {
                let (px, py) = (bb.min.x as i32 + gx as i32, bb.min.y as i32 + gy as i32);
                if px >= 0 && py >= 0 && px < iw && py < ih {
                    let p = img.get_pixel_mut(px as u32, py as u32);
                    for i in 0..3 {
                        p[i] = (p[i] as f32 * (1.0 - cov) + 40.0 * cov) as u8;
                    }
                }
            });
        }
        x += sf.h_advance(id);
    }
}

/// Render one page of the printable area at `dpi`: white paper with the pictures in their cells.
pub fn render_page(paper: &Paper, dpi: u32, cells: &[Cell], items: &[PageItem], settings: &PrintSettings) -> RgbaImage {
    let k = dpi as f32 / 25.4; // pixels per mm
    let (w, h) = (((paper.area_w * k).round() as u32).max(1), ((paper.area_h * k).round() as u32).max(1));
    let mut page = RgbaImage::from_pixel(w, h, Rgba([255, 255, 255, 255]));
    for (cell, item) in cells.iter().zip(items) {
        let (b, rotate) = picture_box(cell, item.image.width(), item.image.height(), settings);
        let src = if rotate { imageops::rotate90(&item.image) } else { item.image.clone() };
        let aspect = src.width() as f32 / src.height() as f32;
        let (r, vis) = fit_or_crop(&b, aspect, settings.crop);
        let (sx0, sy0) = ((vis[0] * src.width() as f32) as u32, (vis[1] * src.height() as f32) as u32);
        let (sw, sh) = (
            (((vis[2] - vis[0]) * src.width() as f32) as u32).max(1).min(src.width() - sx0),
            (((vis[3] - vis[1]) * src.height() as f32) as u32).max(1).min(src.height() - sy0),
        );
        let part = imageops::crop_imm(&src, sx0, sy0, sw, sh).to_image();
        let (dw, dh) = (((r.w * k).round() as u32).max(1), ((r.h * k).round() as u32).max(1));
        let scaled = imageops::resize(&part, dw, dh, imageops::FilterType::CatmullRom);
        let (dx, dy) = ((r.x * k).round() as i64, (r.y * k).round() as i64);
        imageops::overlay(&mut page, &scaled, dx, dy);
        if settings.border == BorderStyle::ThinBlack {
            let t = ((0.3 * k).round() as i64).max(1); // 0.3 mm line
            for yy in dy - t..dy + dh as i64 + t {
                for xx in dx - t..dx + dw as i64 + t {
                    let inside = xx >= dx && xx < dx + dw as i64 && yy >= dy && yy < dy + dh as i64;
                    if !inside && xx >= 0 && yy >= 0 && (xx as u32) < w && (yy as u32) < h {
                        page.put_pixel(xx as u32, yy as u32, Rgba([0, 0, 0, 255]));
                    }
                }
            }
        }
        if settings.caption != CaptionMode::None && !item.caption.is_empty() {
            let px = 3.2 * k; // about 9 pt
            draw_text(&mut page, &item.caption, (b.x + b.w / 2.0) * k, (b.y + b.h + 1.2) * k, px, b.w * k);
        }
    }
    page
}

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

    /// A PDF with one page per JPEG, each placed on the printable area of `paper`.
    pub(super) fn pdf_of_pages(pages: &[(Vec<u8>, u32, u32)], paper: &Paper) -> Vec<u8> {
        let pt = |mm: f32| mm / 25.4 * 72.0;
        let (sw, sh) = (pt(paper.sheet_w), pt(paper.sheet_h));
        let (ax, aw, ah) = (pt(paper.area_x), pt(paper.area_w), pt(paper.area_h));
        // PDF puts the origin at the bottom left
        let ay = sh - pt(paper.area_y) - ah;
        let mut out: Vec<u8> = b"%PDF-1.4\n%\xe2\xe3\xcf\xd3\n".to_vec();
        let mut offsets: Vec<usize> = vec![];
        let n = pages.len();
        // objects: 1 catalog, 2 pages, then per page: page, content, image
        let page_obj = |i: usize| 3 + i * 3;
        let obj = |out: &mut Vec<u8>, offsets: &mut Vec<usize>, body: &[u8]| {
            offsets.push(out.len());
            out.extend_from_slice(format!("{} 0 obj\n", offsets.len()).as_bytes());
            out.extend_from_slice(body);
            out.extend_from_slice(b"\nendobj\n");
        };
        obj(&mut out, &mut offsets, b"<< /Type /Catalog /Pages 2 0 R >>");
        let kids: Vec<String> = (0..n).map(|i| format!("{} 0 R", page_obj(i))).collect();
        obj(&mut out, &mut offsets, format!("<< /Type /Pages /Kids [{}] /Count {n} >>", kids.join(" ")).as_bytes());
        for (i, (jpeg, w, h)) in pages.iter().enumerate() {
            let (content, image) = (page_obj(i) + 1, page_obj(i) + 2);
            obj(
                &mut out,
                &mut offsets,
                format!("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {sw:.2} {sh:.2}] /Resources << /XObject << /Im0 {image} 0 R >> >> /Contents {content} 0 R >>").as_bytes(),
            );
            let draw = format!("q {aw:.2} 0 0 {ah:.2} {ax:.2} {ay:.2} cm /Im0 Do Q");
            obj(&mut out, &mut offsets, format!("<< /Length {} >>\nstream\n{draw}\nendstream", draw.len()).as_bytes());
            let mut body = format!("<< /Type /XObject /Subtype /Image /Width {w} /Height {h} /ColorSpace /DeviceRGB /BitsPerComponent 8 /Filter /DCTDecode /Length {} >>\nstream\n", jpeg.len()).into_bytes();
            body.extend_from_slice(jpeg);
            body.extend_from_slice(b"\nendstream");
            obj(&mut out, &mut offsets, &body);
        }
        let xref = out.len();
        out.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f \n", offsets.len() + 1).as_bytes());
        for o in &offsets {
            out.extend_from_slice(format!("{o:010} 00000 n \n").as_bytes());
        }
        out.extend_from_slice(format!("trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n", offsets.len() + 1).as_bytes());
        out
    }
}
#[cfg(not(windows))]
pub use cups::{default_printer, paper, paper_sizes, print_pages, printers, setup_dialog};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layouts_fill_a4() {
        let a4 = Paper::a4();
        // 10 x 15 on A4 portrait (200 x 287 printable): 2 x 1 landscape-held prints? portrait 1x1 vs landscape 1x2
        let c = page_cells(&a4, PrintLayout::Cm10x15);
        assert_eq!(c.len(), 2);
        assert_eq!(page_cells(&a4, PrintLayout::Cm5x8).len(), 4 * 3);
        assert_eq!(page_cells(&a4, PrintLayout::FullPage).len(), 1);
        // A4 landscape with 9 x 13: four prints
        let land = Paper { sheet_w: 297.0, sheet_h: 210.0, area_x: 5.0, area_y: 5.0, area_w: 287.0, area_h: 200.0, dpi: 600 };
        assert_eq!(page_cells(&land, PrintLayout::Cm9x13).len(), 4);
        assert_eq!(page_count(5, 1, 4), 2);
        assert_eq!(page_count(3, 2, 4), 2);
    }

    #[test]
    fn review_texts() {
        assert_eq!(quality_text(quality_grade(312.0), 312.0), "Best quality (312 pixels/inch)");
        assert_eq!(quality_text(quality_grade(160.0), 160.0), "Good quality (160 pixels/inch)");
        assert_eq!(quality_text(quality_grade(90.4), 90.4), "Bad quality (90 pixels/inch)");
    }

    #[test]
    fn renders_a_page_with_a_rotated_crop() {
        let paper = Paper::a4();
        let cells = page_cells(&paper, PrintLayout::Cm10x15);
        let settings = PrintSettings { crop: true, caption: CaptionMode::FileName, border: BorderStyle::ThinBlack, ..Default::default() };
        let img = RgbaImage::from_pixel(400, 300, Rgba([200, 30, 30, 255]));
        let items = vec![PageItem { image: img.clone(), caption: "IMG_0001.JPG".into() }, PageItem { image: img, caption: String::new() }];
        let page = render_page(&paper, 50, &cells, &items, &settings);
        assert_eq!(page.dimensions(), ((200.0 * 50.0 / 25.4f32).round() as u32, (287.0 * 50.0 / 25.4f32).round() as u32));
        // the middle of the first print is red, the paper corner stays white
        let k = 50.0 / 25.4;
        let mid = page.get_pixel(((cells[0].x + cells[0].w / 2.0) * k) as u32, ((cells[0].y + cells[0].h / 3.0) * k) as u32);
        assert_eq!(mid[0], 200);
        assert_eq!(page.get_pixel(0, 0).0, [255, 255, 255, 255]);
    }

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
