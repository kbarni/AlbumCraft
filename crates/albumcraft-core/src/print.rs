//! Print layouts: page layouts (5 x 8 cm … full page), shrink- or crop-to-fit, borders and
//! captions, and rendering the pages. Sending them to a printer is the app's `print` module.

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
}
