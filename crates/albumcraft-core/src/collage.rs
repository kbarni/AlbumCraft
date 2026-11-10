//! Collages and contact sheets: where each picture goes on the page (`collage`, `contact_sheet`),
//! the background, and drawing a page into pixels (`render`). The window draws the same shapes as
//! its preview, so what is saved or printed is what was on the screen.
//!
//! Everything is measured in millimetres from the page's top left corner; angles are in radians,
//! clockwise on the page.

use crate::imaging::EditOp;
use crate::print::CaptionMode;
use image::{imageops, Rgba, RgbaImage};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::path::Path;

pub type P = [f32; 2];

// ---------------------------------------------------------------------------------------------
// Settings

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize, Default)]
pub enum Style {
    /// Scattered, slightly turned pictures covering the page.
    #[default]
    Pile,
    /// Pictures of different sizes filling the page, without gaps.
    Mosaic,
    /// A mosaic around one big picture in the middle.
    Frame,
    /// Rows and columns of equal cells.
    Grid,
    /// Strips of film with sprocket holes.
    FilmStrip,
    /// Instant pictures hanging on lines with pegs.
    Clothesline,
    /// Fanned out like a hand of cards.
    Fan,
    /// A heart, circle or star made of small square pictures.
    Shape,
    /// Pictures blended on top of each other.
    Blend,
}

impl Style {
    pub const ALL: [Style; 9] =
        [Style::Pile, Style::Mosaic, Style::Frame, Style::Grid, Style::FilmStrip, Style::Clothesline, Style::Fan, Style::Shape, Style::Blend];
    pub fn label(&self) -> &'static str {
        match self {
            Style::Pile => "Photo pile",
            Style::Mosaic => "Mosaic",
            Style::Frame => "Centre stage",
            Style::Grid => "Grid",
            Style::FilmStrip => "Film strip",
            Style::Clothesline => "Clothesline",
            Style::Fan => "Fan",
            Style::Shape => "Shape",
            Style::Blend => "Multiple exposure",
        }
    }
    pub fn describe(&self) -> &'static str {
        match self {
            Style::Pile => "Scattered pictures, like prints thrown on a table",
            Style::Mosaic => "Pictures of different sizes fill the page",
            Style::Frame => "A mosaic around one big picture in the middle",
            Style::Grid => "Regular rows and columns",
            Style::FilmStrip => "Pictures on strips of film",
            Style::Clothesline => "Instant pictures pegged on lines",
            Style::Fan => "Spread out like a hand of cards",
            Style::Shape => "A heart, circle or star made of pictures",
            Style::Blend => "The pictures shown through each other (best with 2 to 4)",
        }
    }
    /// Name used by the command layer.
    pub fn id(&self) -> &'static str {
        match self {
            Style::Pile => "pile",
            Style::Mosaic => "mosaic",
            Style::Frame => "centre",
            Style::Grid => "grid",
            Style::FilmStrip => "film",
            Style::Clothesline => "clothesline",
            Style::Fan => "fan",
            Style::Shape => "shape",
            Style::Blend => "blend",
        }
    }
    pub fn from_id(s: &str) -> Option<Style> {
        Style::ALL.into_iter().find(|st| st.id() == s)
    }
    /// The border that suits the style, chosen when the style is picked.
    pub fn natural_border(&self) -> Border {
        match self {
            Style::Pile | Style::Fan => Border::White,
            Style::Clothesline => Border::Instant,
            _ => Border::None,
        }
    }
    pub fn has_border(&self) -> bool {
        matches!(self, Style::Pile | Style::Clothesline | Style::Fan | Style::Grid)
    }
    pub fn has_gap(&self) -> bool {
        matches!(self, Style::Mosaic | Style::Frame | Style::Grid | Style::Shape)
    }
    pub fn has_shadow(&self) -> bool {
        !matches!(self, Style::Blend)
    }
    /// Pictures can be moved and turned one by one (else dragging swaps two pictures).
    pub fn free(&self) -> bool {
        matches!(self, Style::Pile)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize, Default)]
pub enum Border {
    #[default]
    None,
    White,
    /// White, with a wider strip at the bottom.
    Instant,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize, Default)]
pub enum ShapeKind {
    #[default]
    Heart,
    Circle,
    Star,
}

impl ShapeKind {
    pub const ALL: [ShapeKind; 3] = [ShapeKind::Heart, ShapeKind::Circle, ShapeKind::Star];
    pub fn label(&self) -> &'static str {
        match self {
            ShapeKind::Heart => "Heart",
            ShapeKind::Circle => "Circle",
            ShapeKind::Star => "Star",
        }
    }
}

/// Page sizes (portrait: short side, long side).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize, Default)]
pub enum Format {
    A4,
    A3,
    Letter,
    Cm10x15,
    #[default]
    Cm13x18,
    Cm20x30,
    Square,
    /// 16:9, a screen or a desktop background.
    Wide,
}

impl Format {
    pub const ALL: [Format; 8] =
        [Format::A4, Format::A3, Format::Letter, Format::Cm10x15, Format::Cm13x18, Format::Cm20x30, Format::Square, Format::Wide];
    pub fn label(&self) -> &'static str {
        match self {
            Format::A4 => "A4",
            Format::A3 => "A3",
            Format::Letter => "Letter",
            Format::Cm10x15 => "10 x 15 cm",
            Format::Cm13x18 => "13 x 18 cm",
            Format::Cm20x30 => "20 x 30 cm",
            Format::Square => "Square",
            Format::Wide => "Screen (16:9)",
        }
    }
    fn short_long(&self) -> (f32, f32) {
        match self {
            Format::A4 => (210.0, 297.0),
            Format::A3 => (297.0, 420.0),
            Format::Letter => (215.9, 279.4),
            Format::Cm10x15 => (100.0, 150.0),
            Format::Cm13x18 => (130.0, 180.0),
            Format::Cm20x30 => (200.0, 300.0),
            Format::Square => (200.0, 200.0),
            Format::Wide => (180.0, 320.0),
        }
    }
    /// Width and height in millimetres.
    pub fn size(&self, landscape: bool) -> P {
        let (s, l) = self.short_long();
        if landscape { [l, s] } else { [s, l] }
    }
    pub fn from_id(s: &str) -> Option<Format> {
        let s = s.to_ascii_lowercase().replace(' ', "");
        Format::ALL.into_iter().find(|f| f.label().to_ascii_lowercase().replace(' ', "") == s || format!("{f:?}").to_ascii_lowercase() == s)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize, Default)]
pub enum BgKind {
    #[default]
    Color,
    Gradient,
    Picture,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize, Default)]
pub enum Gradient {
    #[default]
    Vertical,
    Diagonal,
    Radial,
}

impl Gradient {
    pub const ALL: [Gradient; 3] = [Gradient::Vertical, Gradient::Diagonal, Gradient::Radial];
    pub fn label(&self) -> &'static str {
        match self {
            Gradient::Vertical => "Top to bottom",
            Gradient::Diagonal => "Diagonal",
            Gradient::Radial => "From the centre",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Background {
    pub kind: BgKind,
    pub color: [u8; 3],
    pub color2: [u8; 3],
    pub gradient: Gradient,
    /// The background picture is blurred (soft focus), so the pictures on it stand out.
    pub blur: bool,
}

impl Default for Background {
    fn default() -> Self {
        Background { kind: BgKind::Color, color: [255, 255, 255], color2: [120, 150, 190], gradient: Gradient::Vertical, blur: true }
    }
}

impl Background {
    pub fn white() -> Background {
        Background::default()
    }
    /// The colour at (u, v), fractions of the page (picture backgrounds: the first colour).
    pub fn color_at(&self, u: f32, v: f32) -> [u8; 3] {
        let f = match (self.kind, self.gradient) {
            (BgKind::Gradient, Gradient::Vertical) => v,
            (BgKind::Gradient, Gradient::Diagonal) => (u + v) / 2.0,
            (BgKind::Gradient, Gradient::Radial) => (((u - 0.5).powi(2) + (v - 0.5).powi(2)).sqrt() * std::f32::consts::SQRT_2).min(1.0),
            _ => 0.0,
        };
        let f = f.clamp(0.0, 1.0);
        [0, 1, 2].map(|i| (self.color[i] as f32 * (1.0 - f) + self.color2[i] as f32 * f).round() as u8)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct CollageSettings {
    pub style: Style,
    pub format: Format,
    pub landscape: bool,
    pub border: Border,
    pub shadow: bool,
    /// Text written on instant frames.
    pub captions: CaptionMode,
    /// Space between pictures, 0..1.
    pub gap: f32,
    pub shape: ShapeKind,
    pub background: Background,
    /// Longest side of the saved picture, in pixels.
    pub out_px: u32,
    /// Picks the random parts of the layout (positions, turns, sizes).
    pub seed: u64,
}

impl Default for CollageSettings {
    fn default() -> Self {
        CollageSettings {
            style: Style::Pile,
            format: Format::Cm13x18,
            landscape: true,
            border: Border::White,
            shadow: true,
            captions: CaptionMode::None,
            gap: 0.3,
            shape: ShapeKind::Heart,
            background: Background::default(),
            out_px: 3600,
            seed: 1,
        }
    }
}

pub const OUT_SIZES: [u32; 4] = [1920, 2400, 3600, 4800];

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct SheetSettings {
    pub format: Format,
    pub landscape: bool,
    /// Size of each picture's square, in millimetres.
    pub thumb_mm: f32,
    pub header: bool,
    pub names: bool,
    pub captions: bool,
    pub dates: bool,
    pub shadow: bool,
    pub page_numbers: bool,
}

impl Default for SheetSettings {
    fn default() -> Self {
        SheetSettings {
            format: Format::A4,
            landscape: false,
            thumb_mm: 32.0,
            header: true,
            names: true,
            captions: false,
            dates: false,
            shadow: true,
            page_numbers: true,
        }
    }
}

pub const THUMB_MM_MIN: f32 = 15.0;
pub const THUMB_MM_MAX: f32 = 90.0;

// ---------------------------------------------------------------------------------------------
// Shapes

/// One picture on the page: the picture area `size` centred at `c`, turned by `rot`, inside a
/// frame (`frame`: left, top, right, bottom margins). The picture is cropped to the area's shape.
#[derive(Clone, Debug, PartialEq)]
pub struct PhotoShape {
    /// Index into the pictures.
    pub pic: usize,
    pub c: P,
    pub size: P,
    pub rot: f32,
    pub frame: [f32; 4],
    pub frame_color: [u8; 4],
    pub shadow: bool,
    pub alpha: f32,
}

impl PhotoShape {
    /// Centre and size of the whole card (picture and frame).
    pub fn outer(&self) -> (P, P) {
        let [l, t, r, b] = self.frame;
        let size = [self.size[0] + l + r, self.size[1] + t + b];
        let off = rotate([(r - l) / 2.0, (b - t) / 2.0], self.rot);
        ([self.c[0] + off[0], self.c[1] + off[1]], size)
    }
    /// The page point `p` falls on the card.
    pub fn contains(&self, p: P) -> bool {
        let (c, s) = self.outer();
        let l = rotate([p[0] - c[0], p[1] - c[1]], -self.rot);
        l[0].abs() <= s[0] / 2.0 && l[1].abs() <= s[1] / 2.0
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Shape {
    Photo(PhotoShape),
    Rect { c: P, size: P, rot: f32, color: [u8; 4], radius: f32 },
    Line { pts: Vec<P>, width: f32, color: [u8; 4] },
    /// One line of text centred at `c`, `size` = font size in millimetres.
    Text { c: P, text: String, size: f32, rot: f32, color: [u8; 4] },
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct Page {
    pub size: P,
    pub shapes: Vec<Shape>,
}

impl Page {
    /// The picture card under the page point `p` (the top one), as an index into `shapes`.
    pub fn hit(&self, p: P) -> Option<usize> {
        self.shapes.iter().enumerate().rev().find_map(|(i, s)| match s {
            Shape::Photo(ph) if ph.contains(p) => Some(i),
            _ => None,
        })
    }
}

/// What the layouts need to know about each picture.
#[derive(Clone, Debug, Default)]
pub struct Pic {
    /// Width / height, as edited.
    pub aspect: f32,
    pub name: String,
    pub caption: String,
    pub date: String,
}

/// A picture moved by hand (photo pile): offset, extra turn, and stacking order (higher on top).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Nudge {
    pub dx: f32,
    pub dy: f32,
    pub rot: f32,
    pub z: u32,
}

pub fn rotate(p: P, a: f32) -> P {
    let (s, c) = a.sin_cos();
    [p[0] * c - p[1] * s, p[0] * s + p[1] * c]
}

fn aspect_of(p: &Pic) -> f32 {
    if p.aspect.is_finite() && p.aspect > 0.0 { p.aspect.clamp(0.2, 5.0) } else { 1.5 }
}

/// Width and height of a picture of `aspect` with area `area`.
fn by_area(aspect: f32, area: f32) -> P {
    [(area * aspect).sqrt(), (area / aspect).sqrt()]
}

struct Rng(u64);
impl Rng {
    fn new(seed: u64) -> Rng {
        Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ 0xD1B5_4A32_D192_ED03)
    }
    fn next(&mut self) -> u64 {
        // splitmix64
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    /// 0..1
    fn f(&mut self) -> f32 {
        (self.next() >> 40) as f32 / (1u64 << 24) as f32
    }
    /// -1..1
    fn s(&mut self) -> f32 {
        self.f() * 2.0 - 1.0
    }
}

const WHITE: [u8; 4] = [255, 255, 255, 255];
const CREAM: [u8; 4] = [250, 248, 242, 255];
const NO_FRAME: [f32; 4] = [0.0; 4];

fn frame_for(border: Border, size: P) -> ([f32; 4], [u8; 4]) {
    let m = size[0].min(size[1]);
    match border {
        Border::None => (NO_FRAME, WHITE),
        Border::White => ([m * 0.045; 4].map(|v| v.max(0.8)), WHITE),
        Border::Instant => {
            let s = (m * 0.07).max(1.0);
            ([s, s, s, (m * 0.24).max(3.0)], CREAM)
        }
    }
}

fn photo(pic: usize, c: P, size: P, rot: f32, shadow: bool) -> PhotoShape {
    PhotoShape { pic, c, size, rot, frame: NO_FRAME, frame_color: WHITE, shadow, alpha: 1.0 }
}

/// A caption written on the bottom strip of an instant frame.
fn frame_caption(ph: &PhotoShape, text: &str) -> Option<Shape> {
    let b = ph.frame[3];
    if text.is_empty() || b < 2.0 * ph.frame[0] {
        return None;
    }
    let size = (b * 0.34).min(ph.size[0] * 0.09).max(1.0);
    let text = fit_text(text, size, ph.size[0] * 0.95);
    let off = rotate([0.0, ph.size[1] / 2.0 + b * 0.48], ph.rot);
    Some(Shape::Text { c: [ph.c[0] + off[0], ph.c[1] + off[1]], text, size, rot: ph.rot, color: [50, 50, 60, 255] })
}

fn caption_of(mode: CaptionMode, pic: &Pic) -> &str {
    match mode {
        CaptionMode::None => "",
        CaptionMode::FileName => &pic.name,
        CaptionMode::Caption => &pic.caption,
    }
}

// ---------------------------------------------------------------------------------------------
// Collage layouts

/// The collage page for these pictures (in this order). `nudges` (by picture, may be shorter)
/// are the moves made by hand in a photo pile.
pub fn collage(s: &CollageSettings, pics: &[Pic], nudges: &[Nudge]) -> Page {
    let size = s.format.size(s.landscape);
    let mut page = Page { size, shapes: vec![] };
    if pics.is_empty() {
        return page;
    }
    let mut rng = Rng::new(s.seed);
    let gap = s.gap.clamp(0.0, 1.0) * size[0].min(size[1]) * 0.03;
    let shadow = s.shadow && s.style.has_shadow();
    let mut photos: Vec<PhotoShape> = vec![];
    let mut extra_under: Vec<Shape> = vec![];
    let mut extra_over: Vec<(usize, Shape)> = vec![];
    match s.style {
        Style::Pile => photos = pile(size, pics, &mut rng, shadow),
        Style::Mosaic => {
            let items: Vec<(usize, f32, f32)> = pics
                .iter()
                .enumerate()
                .map(|(i, p)| {
                    let r = rng.f();
                    (i, if r < 0.15 { 3.0 } else if r < 0.4 { 2.0 } else { 1.0 }, aspect_of(p))
                })
                .collect();
            let mut cells = vec![];
            guillotine(Rect { x: gap / 2.0, y: gap / 2.0, w: size[0] - gap, h: size[1] - gap }, &items, &mut cells);
            photos = cells.into_iter().map(|(i, r)| cell_photo(i, r, gap, shadow)).collect();
        }
        Style::Frame => photos = centre_stage(size, pics, gap, shadow),
        Style::Grid => {
            let n = pics.len();
            let mean = pics.iter().map(aspect_of).sum::<f32>() / n as f32;
            let (cols, rows) = best_grid(n, size[0] - gap, size[1] - gap, mean);
            let (cw, ch) = ((size[0] - gap) / cols as f32, (size[1] - gap) / rows as f32);
            for i in 0..n {
                let (row, col) = (i / cols, i % cols);
                let in_row = if row + 1 == rows { n - row * cols } else { cols };
                let x0 = gap / 2.0 + (cols - in_row) as f32 * cw / 2.0;
                let r = Rect { x: x0 + col as f32 * cw, y: gap / 2.0 + row as f32 * ch, w: cw, h: ch };
                photos.push(cell_photo(i, r, gap.max(if s.border == Border::None { 0.0 } else { cw.min(ch) * 0.06 }), shadow));
            }
        }
        Style::FilmStrip => film_strips(size, pics, &mut rng, shadow, &mut photos, &mut extra_under),
        Style::Clothesline => clothesline(size, pics, &mut rng, shadow, &mut photos, &mut extra_under, &mut extra_over),
        Style::Fan => photos = fan(size, pics, shadow),
        Style::Shape => photos = shape_cells(size, pics.len(), s.shape, gap, shadow),
        Style::Blend => {
            for i in 0..pics.len().min(6) {
                let mut ph = photo(i, [size[0] / 2.0, size[1] / 2.0], size, 0.0, false);
                ph.alpha = 1.0 / (i + 1) as f32;
                photos.push(ph);
            }
        }
    }
    // frames (sized from the picture area)
    if s.style.has_border() {
        for ph in &mut photos {
            let (f, col) = frame_for(s.border, ph.size);
            ph.frame = f;
            ph.frame_color = col;
            // a hanging picture keeps its top edge on the line
            if s.style == Style::Clothesline {
                let off = rotate([0.0, f[1]], ph.rot);
                ph.c = [ph.c[0] + off[0], ph.c[1] + off[1]];
            }
        }
    }
    // hand moves and stacking (photo pile)
    if s.style.free() {
        for ph in &mut photos {
            if let Some(n) = nudges.get(ph.pic) {
                ph.c = [ph.c[0] + n.dx, ph.c[1] + n.dy];
                ph.rot += n.rot;
            }
        }
        // a stable shuffle, then the hand-raised ones on top
        let mut order: Vec<(u32, u64, PhotoShape)> = photos.into_iter().map(|p| (nudges.get(p.pic).map(|n| n.z).unwrap_or(0), rng.next(), p)).collect();
        order.sort_by_key(|(z, r, _)| (*z, *r));
        photos = order.into_iter().map(|(_, _, p)| p).collect();
    }
    page.shapes.extend(extra_under);
    for (i, ph) in photos.into_iter().enumerate() {
        let caption = if s.style.has_border() && s.border == Border::Instant {
            pics.get(ph.pic).and_then(|p| frame_caption(&ph, caption_of(s.captions, p)))
        } else {
            None
        };
        page.shapes.push(Shape::Photo(ph));
        page.shapes.extend(caption);
        page.shapes.extend(extra_over.iter().filter(|(j, _)| *j == i).map(|(_, s)| s.clone()));
    }
    page
}

#[derive(Clone, Copy, Debug)]
struct Rect {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
}

fn cell_photo(pic: usize, r: Rect, gap: f32, shadow: bool) -> PhotoShape {
    let (w, h) = ((r.w - gap).max(0.5), (r.h - gap).max(0.5));
    photo(pic, [r.x + r.w / 2.0, r.y + r.h / 2.0], [w, h], 0.0, shadow)
}

fn pile(size: P, pics: &[Pic], rng: &mut Rng, shadow: bool) -> Vec<PhotoShape> {
    let n = pics.len();
    let cols = ((n as f32 * size[0] / size[1]).sqrt().round() as usize).clamp(1, n);
    let rows = n.div_ceil(cols);
    let (cw, ch) = (size[0] / cols as f32, size[1] / rows as f32);
    // fewer pictures: bigger, so the page still looks full
    let area = cw * ch * if n <= 3 { 0.9 } else { 1.45 };
    pics.iter()
        .enumerate()
        .map(|(i, p)| {
            let (row, col) = (i / cols, i % cols);
            let in_row = if row + 1 == rows { n - row * cols } else { cols };
            let x0 = (cols - in_row) as f32 * cw / 2.0;
            let c = [x0 + (col as f32 + 0.5 + rng.s() * 0.22) * cw, (row as f32 + 0.5 + rng.s() * 0.22) * ch];
            let mut sz = by_area(aspect_of(p), area * (0.85 + rng.f() * 0.3));
            // never wider or taller than the page
            let k = (size[0] * 0.92 / sz[0]).min(size[1] * 0.92 / sz[1]).min(1.0);
            sz = [sz[0] * k, sz[1] * k];
            photo(i, c, sz, rng.s() * 0.21, shadow)
        })
        .collect()
}

/// Split `r` between the items (index, weight, aspect) by weight, along its longer side, at the
/// place that gives the best-shaped cells.
fn guillotine(r: Rect, items: &[(usize, f32, f32)], out: &mut Vec<(usize, Rect)>) {
    match items {
        [] => {}
        [one] => out.push((one.0, r)),
        _ => {
            let total: f32 = items.iter().map(|i| i.1).sum::<f32>().max(1e-6);
            let across = r.w >= r.h;
            let mut best = (f32::MAX, 1);
            let mut prefix = 0.0;
            for k in 1..items.len() {
                prefix += items[k - 1].1;
                let f = prefix / total;
                let (a, b) = if across {
                    (Rect { w: r.w * f, ..r }, Rect { x: r.x + r.w * f, w: r.w * (1.0 - f), ..r })
                } else {
                    (Rect { h: r.h * f, ..r }, Rect { y: r.y + r.h * f, h: r.h * (1.0 - f), ..r })
                };
                let score = cell_score(a, &items[..k]) + cell_score(b, &items[k..]) + (k as f32 / items.len() as f32 - 0.5).abs() * 0.3;
                if score < best.0 {
                    best = (score, k);
                }
            }
            let k = best.1;
            let f = items[..k].iter().map(|i| i.1).sum::<f32>() / total;
            let (a, b) = if across {
                (Rect { w: r.w * f, ..r }, Rect { x: r.x + r.w * f, w: r.w * (1.0 - f), ..r })
            } else {
                (Rect { h: r.h * f, ..r }, Rect { y: r.y + r.h * f, h: r.h * (1.0 - f), ..r })
            };
            guillotine(a, &items[..k], out);
            guillotine(b, &items[k..], out);
        }
    }
}

/// How badly a cell fits its pictures: a single picture should keep its shape, a group of
/// pictures should get a roughly square area.
fn cell_score(r: Rect, items: &[(usize, f32, f32)]) -> f32 {
    let a = (r.w / r.h.max(1e-3)).max(1e-3);
    match items {
        [one] => (a / one.2).ln().abs(),
        _ => a.ln().abs() * 0.35,
    }
}

/// Columns and rows for `n` cells of about `aspect` filling w x h.
fn best_grid(n: usize, w: f32, h: f32, aspect: f32) -> (usize, usize) {
    let mut best = (f32::MAX, 1, n);
    for cols in 1..=n.max(1) {
        let rows = n.div_ceil(cols);
        let cell = (w / cols as f32) / (h / rows as f32);
        let empty = (cols * rows - n) as f32;
        let score = (cell / aspect).ln().abs() + empty * 0.15;
        if score < best.0 {
            best = (score, cols, rows);
        }
    }
    (best.1, best.2)
}

fn centre_stage(size: P, pics: &[Pic], gap: f32, shadow: bool) -> Vec<PhotoShape> {
    let [w, h] = size;
    let n = pics.len();
    let a = aspect_of(&pics[0]);
    let rest: Vec<(usize, f32, f32)> = pics.iter().enumerate().skip(1).map(|(i, p)| (i, 1.0, aspect_of(p))).collect();
    if rest.is_empty() {
        return vec![cell_photo(0, Rect { x: 0.0, y: 0.0, w, h }, gap, shadow)];
    }
    // the centre picture: about 30 % of the page, at its own shape
    let mut c = by_area(a, w * h * if n < 5 { 0.4 } else { 0.3 });
    let k = (w * 0.62 / c[0]).min(h * if n < 5 { 1.0 } else { 0.7 } / c[1]).min(1.0);
    c = [c[0] * k, c[1] * k];
    let (x0, x1) = ((w - c[0]) / 2.0, (w + c[0]) / 2.0);
    let (y0, y1) = if n < 5 { (0.0, h) } else { ((h - c[1]) / 2.0, (h + c[1]) / 2.0) };
    let mut regions = vec![Rect { x: 0.0, y: 0.0, w: x0, h }, Rect { x: x1, y: 0.0, w: w - x1, h }];
    if n >= 5 {
        regions.push(Rect { x: x0, y: 0.0, w: x1 - x0, h: y0 });
        regions.push(Rect { x: x0, y: y1, w: x1 - x0, h: h - y1 });
    }
    // pictures per region by area (each at least one)
    let total: f32 = regions.iter().map(|r| r.w * r.h).sum::<f32>().max(1e-3);
    let mut counts: Vec<usize> = regions.iter().map(|r| ((r.w * r.h / total * rest.len() as f32).floor() as usize).max(1)).collect();
    while counts.iter().sum::<usize>() > rest.len() {
        if let Some(m) = counts.iter_mut().filter(|c| **c > 1).max() {
            *m -= 1;
        } else {
            break;
        }
    }
    while counts.iter().sum::<usize>() < rest.len() {
        let (i, _) = regions.iter().enumerate().map(|(i, r)| (i, r.w * r.h / counts[i] as f32)).fold((0, 0.0), |b, x| if x.1 > b.1 { x } else { b });
        counts[i] += 1;
    }
    let mut out = vec![cell_photo(0, Rect { x: x0, y: y0, w: x1 - x0, h: y1 - y0 }, gap, shadow)];
    let mut start = 0;
    for (r, cnt) in regions.iter().zip(counts) {
        let end = (start + cnt).min(rest.len());
        let mut cells = vec![];
        guillotine(*r, &rest[start..end], &mut cells);
        out.extend(cells.into_iter().map(|(i, r)| cell_photo(i, r, gap, shadow)));
        start = end;
    }
    out
}

const FILM: [u8; 4] = [28, 25, 24, 255];
const HOLE: [u8; 4] = [236, 233, 226, 255];

fn film_strips(size: P, pics: &[Pic], rng: &mut Rng, shadow: bool, photos: &mut Vec<PhotoShape>, under: &mut Vec<Shape>) {
    let [w, h] = size;
    let n = pics.len();
    // the number of strips that gives the biggest frames
    let fh_for = |rows: usize| (h / rows as f32 * 0.8 / 1.42).min(w * 0.96 / (n.div_ceil(rows) as f32 * 1.62));
    let rows = (1..=n).max_by(|a, b| fh_for(*a).total_cmp(&fh_for(*b))).unwrap_or(1);
    let per = n.div_ceil(rows);
    let fh = fh_for(rows);
    let fw = fh * 1.5;
    let g = fh * 0.12;
    let band = fh * 0.21;
    let sh = fh + 2.0 * band;
    let pitch = h / rows as f32;
    for row in 0..rows {
        let items: Vec<usize> = (row * per..((row + 1) * per).min(n)).collect();
        let cy = pitch * (row as f32 + 0.5);
        let rot = rng.s() * 0.045;
        let len = w * 1.2;
        let cx = w / 2.0 + rng.s() * w * 0.02;
        under.push(Shape::Rect { c: [cx, cy], size: [len, sh], rot, color: FILM, radius: 0.0 });
        // sprocket holes
        let hole = [fh * 0.085, fh * 0.11];
        let step = hole[0] * 2.4;
        let holes = (len / step) as i32;
        for k in -holes / 2..=holes / 2 {
            for side in [-1.0, 1.0] {
                let p = rotate([k as f32 * step, side * (fh / 2.0 + band / 2.0)], rot);
                under.push(Shape::Rect { c: [cx + p[0], cy + p[1]], size: hole, rot, color: HOLE, radius: hole[0] * 0.25 });
            }
        }
        let used = items.len() as f32 * (fw + g) - g;
        for (j, &i) in items.iter().enumerate() {
            let x = -used / 2.0 + fw / 2.0 + j as f32 * (fw + g);
            let p = rotate([x, 0.0], rot);
            photos.push(photo(i, [cx + p[0], cy + p[1]], [fw, fh], rot, false));
        }
        let _ = shadow;
    }
}

const ROPE: [u8; 4] = [96, 84, 70, 255];
const PEG: [u8; 4] = [205, 160, 108, 255];

fn clothesline(size: P, pics: &[Pic], rng: &mut Rng, _shadow: bool, photos: &mut Vec<PhotoShape>, under: &mut Vec<Shape>, over: &mut Vec<(usize, Shape)>) {
    let [w, h] = size;
    let n = pics.len();
    let mut best = (f32::MAX, 1);
    for rows in 1..=n {
        let per = n.div_ceil(rows) as f32;
        let pw = w / (per + 0.5);
        let score = ((h / rows as f32) / (pw * 1.45)).ln().abs();
        if score < best.0 {
            best = (score, rows);
        }
    }
    let rows = best.1;
    let per = n.div_ceil(rows);
    let pitch = h / rows as f32;
    let slot = w / (per as f32 + 0.5);
    let side = (slot * 0.78).min(pitch * 0.6);
    for row in 0..rows {
        let y0 = pitch * row as f32 + pitch * 0.12;
        let sag = pitch * 0.07;
        let rope_y = |x: f32| y0 + sag * (1.0 - (2.0 * x / w - 1.0).powi(2));
        let pts: Vec<P> = (0..=24).map(|k| {
            let x = -2.0 + (w + 4.0) * k as f32 / 24.0;
            [x, rope_y(x)]
        }).collect();
        under.push(Shape::Line { pts, width: (w * 0.0035).max(0.4), color: ROPE });
        let items: Vec<usize> = (row * per..((row + 1) * per).min(n)).collect();
        let x0 = (w - items.len() as f32 * slot) / 2.0;
        for (j, &i) in items.iter().enumerate() {
            let x = x0 + slot * (j as f32 + 0.5) + rng.s() * slot * 0.05;
            let top = [x, rope_y(x) + side * 0.02];
            let a = aspect_of(&pics[i]);
            let sz = if a >= 1.0 { [side, side / a] } else { [side * a, side] };
            let rot = rng.s() * 0.09;
            // hang from the middle of the top edge (the frame is added later, see `collage`)
            let off = rotate([0.0, sz[1] / 2.0], rot);
            photos.push(photo(i, [top[0] + off[0], top[1] + off[1]], sz, rot, true));
            let peg = [side * 0.07, side * 0.2];
            let po = rotate([0.0, peg[1] * 0.18], rot);
            over.push((photos.len() - 1, Shape::Rect { c: [top[0] + po[0], top[1] + po[1]], size: peg, rot, color: PEG, radius: peg[0] * 0.3 }));
        }
    }
}

fn fan(size: P, pics: &[Pic], shadow: bool) -> Vec<PhotoShape> {
    let [w, h] = size;
    let n = pics.len();
    let card_h = h * if n > 12 { 0.36 } else if n > 6 { 0.42 } else { 0.48 };
    let pivot = [w / 2.0, h * 1.08];
    let dist = h * 0.66;
    // the outer cards stay on the page
    let reach = ((w * 0.47 - card_h * 0.6) / dist).clamp(0.2, 1.0).asin() * 2.0;
    let spread = (n as f32 * 0.22).min(reach).min(2.6);
    pics.iter()
        .enumerate()
        .map(|(i, p)| {
            let a = aspect_of(p).min(1.4);
            let sz = [card_h * a, card_h];
            let t = if n > 1 { i as f32 / (n - 1) as f32 - 0.5 } else { 0.0 };
            let ang = t * spread;
            let off = rotate([0.0, -dist], ang);
            photo(i, [pivot[0] + off[0], pivot[1] + off[1]], sz, ang, shadow)
        })
        .collect()
}

fn inside_shape(kind: ShapeKind, u: f32, v: f32) -> bool {
    match kind {
        ShapeKind::Circle => (u - 0.5).powi(2) + (v - 0.5).powi(2) <= 0.25,
        ShapeKind::Heart => {
            let (x, y) = ((u - 0.5) * 2.5, (0.5 - v) * 2.5 + 0.2);
            (x * x + y * y - 1.0).powi(3) - x * x * y * y * y <= 0.0
        }
        ShapeKind::Star => {
            let pts: Vec<P> = (0..10)
                .map(|k| {
                    let r = if k % 2 == 0 { 0.5 } else { 0.21 };
                    let a = -std::f32::consts::FRAC_PI_2 + k as f32 * std::f32::consts::PI / 5.0;
                    [0.5 + r * a.cos(), 0.54 + r * a.sin()]
                })
                .collect();
            let mut inside = false;
            let mut j = pts.len() - 1;
            for i in 0..pts.len() {
                let (a, b) = (pts[i], pts[j]);
                if (a[1] > v) != (b[1] > v) && u < (b[0] - a[0]) * (v - a[1]) / (b[1] - a[1]) + a[0] {
                    inside = !inside;
                }
                j = i;
            }
            inside
        }
    }
}

fn shape_cells(size: P, n: usize, kind: ShapeKind, gap: f32, shadow: bool) -> Vec<PhotoShape> {
    let side = size[0].min(size[1]) * 0.94;
    let origin = [(size[0] - side) / 2.0, (size[1] - side) / 2.0];
    let mut cells = vec![];
    // at least 8 cells across, so the shape shows (pictures repeat when there are few)
    for k in 8..=40 {
        cells.clear();
        for row in 0..k {
            for col in 0..k {
                let (u, v) = ((col as f32 + 0.5) / k as f32, (row as f32 + 0.5) / k as f32);
                if inside_shape(kind, u, v) {
                    cells.push((col, row));
                }
            }
        }
        if cells.len() >= n {
            let cell = side / k as f32;
            let g = gap.min(cell * 0.2).max(cell * 0.04);
            return cells
                .iter()
                .enumerate()
                .map(|(i, &(col, row))| {
                    let r = Rect { x: origin[0] + col as f32 * cell, y: origin[1] + row as f32 * cell, w: cell, h: cell };
                    // more cells than pictures: start again from the first
                    cell_photo(i % n.max(1), r, g, shadow)
                })
                .collect();
        }
    }
    vec![]
}

// ---------------------------------------------------------------------------------------------
// Contact sheet

const SHEET_MARGIN: f32 = 12.0;
const LINE_MM: f32 = 4.0;
const SMALL_TEXT: f32 = 2.9;

/// The pages of a contact sheet: a header (title, subtitle) on the first page, then rows of
/// pictures with the chosen texts under each one.
pub fn contact_sheet(s: &SheetSettings, pics: &[Pic], title: &str, subtitle: &str) -> Vec<Page> {
    let size = s.format.size(s.landscape);
    let [w, h] = size;
    let m = SHEET_MARGIN.min(w * 0.08);
    let thumb = s.thumb_mm.clamp(THUMB_MM_MIN, THUMB_MM_MAX).min(w - 2.0 * m).min(h * 0.6);
    let gap = (thumb * 0.16).max(3.0);
    let lines = [s.names, s.captions, s.dates].iter().filter(|b| **b).count() as f32;
    let cell_h = thumb + lines * LINE_MM + gap;
    let cols = (((w - 2.0 * m + gap) / (thumb + gap)).floor() as usize).max(1);
    let grid_w = cols as f32 * (thumb + gap) - gap;
    let left = (w - grid_w) / 2.0;
    let has_header = s.header && (!title.trim().is_empty() || !subtitle.trim().is_empty());
    let header_h = if !has_header { 0.0 } else if title.trim().is_empty() { 12.0 } else { 24.0 };
    let footer_h = if s.page_numbers { 8.0 } else { 0.0 };
    let rows_on = |first: bool| (((h - 2.0 * m - footer_h - if first { header_h } else { 0.0 } + gap) / cell_h).floor() as usize).max(1);

    let mut pages: Vec<Page> = vec![];
    let mut i = 0;
    while i < pics.len() || pages.is_empty() {
        let first = pages.is_empty();
        let mut page = Page { size, shapes: vec![] };
        let mut y = m;
        if first && has_header {
            if !title.trim().is_empty() {
                let size = 8.5;
                let t = fit_text(title.trim(), size, w - 2.0 * m);
                let tw = text_width(&t, size);
                page.shapes.push(Shape::Text { c: [left + tw / 2.0, y + size * 0.6], text: t, size, rot: 0.0, color: [30, 30, 34, 255] });
            }
            if !subtitle.trim().is_empty() {
                let size = 4.2;
                let t = fit_text(subtitle.trim(), size, w - 2.0 * m);
                let tw = text_width(&t, size);
                let ty = if title.trim().is_empty() { y + size * 0.6 } else { y + 14.0 };
                page.shapes.push(Shape::Text { c: [left + tw / 2.0, ty], text: t, size, rot: 0.0, color: [95, 95, 100, 255] });
            }
            y += header_h;
        }
        let per = rows_on(first) * cols;
        for k in 0..per {
            let Some(p) = pics.get(i) else { break };
            let (row, col) = (k / cols, k % cols);
            let (cx, top) = (left + col as f32 * (thumb + gap) + thumb / 2.0, y + row as f32 * cell_h);
            let a = aspect_of(p);
            let sz = if a >= 1.0 { [thumb, thumb / a] } else { [thumb * a, thumb] };
            let mut ph = photo(i, [cx, top + thumb / 2.0], sz, 0.0, s.shadow);
            if !s.shadow {
                ph.frame = [0.25; 4];
                ph.frame_color = [190, 190, 190, 255];
            }
            page.shapes.push(Shape::Photo(ph));
            let mut ty = top + thumb + LINE_MM * 0.62;
            for (on, text, color) in [(s.names, &p.name, [55, 55, 60, 255]), (s.captions, &p.caption, [80, 80, 86, 255]), (s.dates, &p.date, [120, 120, 126, 255])] {
                if on {
                    if !text.is_empty() {
                        page.shapes.push(Shape::Text { c: [cx, ty], text: fit_text(text, SMALL_TEXT, thumb + gap * 0.8), size: SMALL_TEXT, rot: 0.0, color });
                    }
                    ty += LINE_MM;
                }
            }
            i += 1;
        }
        pages.push(page);
        if pics.is_empty() {
            break;
        }
    }
    if s.page_numbers && pages.len() > 1 {
        let n = pages.len();
        for (k, p) in pages.iter_mut().enumerate() {
            p.shapes.push(Shape::Text { c: [w / 2.0, h - m * 0.7], text: format!("{} / {n}", k + 1), size: 3.0, rot: 0.0, color: [130, 130, 136, 255] });
        }
    }
    pages
}

/// "37 pictures, April 2006" / "12 pictures, March – May 2006" from the pictures' dates.
pub fn sheet_subtitle(times: &[i64]) -> String {
    const M: [&str; 12] = ["January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December"];
    let n = times.len();
    let count = if n == 1 { "1 picture".to_string() } else { format!("{n} pictures") };
    let (Some(&lo), Some(&hi)) = (times.iter().min(), times.iter().max()) else { return count };
    let ym = |t: i64| {
        use chrono::{Datelike, TimeZone};
        let d = chrono::Local.timestamp_opt(t, 0).single()?;
        Some((d.year(), d.month0() as usize))
    };
    let (Some((y0, m0)), Some((y1, m1))) = (ym(lo), ym(hi)) else { return count };
    let (m0, m1) = (M[m0.min(11)], M[m1.min(11)]);
    let when = if (y0, m0) == (y1, m1) {
        format!("{m0} {y0}")
    } else if y0 == y1 {
        format!("{m0} – {m1} {y0}")
    } else {
        format!("{m0} {y0} – {m1} {y1}")
    };
    format!("{count}, {when}")
}

// ---------------------------------------------------------------------------------------------
// Text

fn font() -> Option<ab_glyph::FontRef<'static>> {
    ab_glyph::FontRef::try_from_slice(epaint_default_fonts::UBUNTU_LIGHT).ok()
}

/// Width of a line of text at font size `size` (any unit; the answer is in the same unit).
pub fn text_width(text: &str, size: f32) -> f32 {
    use ab_glyph::{Font, ScaleFont};
    let Some(f) = font() else { return text.chars().count() as f32 * size * 0.5 };
    let sf = f.as_scaled(size);
    text.chars().map(|c| sf.h_advance(f.glyph_id(c))).sum()
}

/// The text, shortened with "…" so it is at most `max_w` wide.
pub fn fit_text(text: &str, size: f32, max_w: f32) -> String {
    if text_width(text, size) <= max_w {
        return text.to_string();
    }
    let mut chars: Vec<char> = text.chars().collect();
    while chars.len() > 1 {
        chars.pop();
        let s: String = chars.iter().chain(std::iter::once(&'…')).collect();
        if text_width(&s, size) <= max_w {
            return s;
        }
    }
    "…".into()
}

/// One line of text drawn into a transparent picture, `px` pixels high.
fn text_image(text: &str, px: f32, color: [u8; 4]) -> Option<RgbaImage> {
    use ab_glyph::{point, Font, PxScale, ScaleFont};
    let f = font()?;
    let scale = PxScale::from(px.max(1.0));
    let sf = f.as_scaled(scale);
    let w = (text_width(text, px).ceil() as u32 + 2).min(16_000);
    let h = ((sf.ascent() - sf.descent()).ceil() as u32 + 2).min(4000);
    let mut img = RgbaImage::from_pixel(w, h, Rgba([color[0], color[1], color[2], 0]));
    let mut x = 1.0;
    for c in text.chars() {
        let id = f.glyph_id(c);
        if let Some(o) = f.outline_glyph(id.with_scale_and_position(scale, point(x, 1.0 + sf.ascent()))) {
            let bb = o.px_bounds();
            o.draw(|gx, gy, cov| {
                let (px, py) = (bb.min.x as i32 + gx as i32, bb.min.y as i32 + gy as i32);
                if px >= 0 && py >= 0 && (px as u32) < w && (py as u32) < h {
                    let p = img.get_pixel_mut(px as u32, py as u32);
                    p[3] = p[3].max((cov.clamp(0.0, 1.0) * color[3] as f32) as u8);
                }
            });
        }
        x += sf.h_advance(id);
    }
    Some(img)
}

// ---------------------------------------------------------------------------------------------
// Drawing into pixels

/// Largest page side drawn, in pixels.
pub const MAX_PAGE_PX: u32 = 12_000;

fn smoothstep(e0: f32, e1: f32, x: f32) -> f32 {
    let t = ((x - e0) / (e1 - e0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Signed distance from local point `l` to a rounded box of half size `half`.
fn sdf_box(l: P, half: P, radius: f32) -> f32 {
    let r = radius.min(half[0]).min(half[1]).max(0.0);
    let q = [l[0].abs() - half[0] + r, l[1].abs() - half[1] + r];
    let outside = (q[0].max(0.0).powi(2) + q[1].max(0.0).powi(2)).sqrt();
    outside + q[0].max(q[1]).min(0.0) - r
}

fn blend(px: &mut [u8], c: [f32; 3], a: f32) {
    if a <= 0.0 {
        return;
    }
    let a = a.min(1.0);
    for i in 0..3 {
        px[i] = (px[i] as f32 * (1.0 - a) + c[i] * a).round() as u8;
    }
    px[3] = 255;
}

/// Run `f(x, y, pixel)` for every pixel of the box (x0..x1, y0..y1), rows in parallel.
fn each_pixel(img: &mut RgbaImage, b: [f32; 4], f: impl Fn(f32, f32, &mut [u8]) + Sync) {
    let (w, h) = (img.width() as i64, img.height() as i64);
    let x0 = (b[0].floor() as i64).clamp(0, w);
    let x1 = (b[2].ceil() as i64).clamp(0, w);
    let y0 = (b[1].floor() as i64).clamp(0, h);
    let y1 = (b[3].ceil() as i64).clamp(0, h);
    if x0 >= x1 || y0 >= y1 {
        return;
    }
    let stride = w as usize * 4;
    let rows = &mut img.as_mut()[y0 as usize * stride..y1 as usize * stride];
    rows.par_chunks_mut(stride).enumerate().for_each(|(k, row)| {
        let y = y0 as f32 + k as f32 + 0.5;
        for x in x0..x1 {
            let i = x as usize * 4;
            f(x as f32 + 0.5, y, &mut row[i..i + 4]);
        }
    });
}

/// Bounding box of a box of size `size` centred at `c`, turned by `rot`, grown by `grow`.
fn bounds(c: P, size: P, rot: f32, grow: f32) -> [f32; 4] {
    let (s, co) = rot.sin_cos();
    let hw = (size[0] * co.abs() + size[1] * s.abs()) / 2.0 + grow;
    let hh = (size[0] * s.abs() + size[1] * co.abs()) / 2.0 + grow;
    [c[0] - hw, c[1] - hh, c[0] + hw, c[1] + hh]
}

/// Fill a turned, rounded box (all in pixels); `soft` > 0 blurs its edge by that many pixels.
fn fill_box(img: &mut RgbaImage, c: P, size: P, rot: f32, radius: f32, color: [u8; 4], soft: f32) {
    let half = [size[0] / 2.0, size[1] / 2.0];
    let col = [color[0] as f32, color[1] as f32, color[2] as f32];
    let alpha = color[3] as f32 / 255.0;
    each_pixel(img, bounds(c, size, rot, soft + 1.0), |x, y, px| {
        let l = rotate([x - c[0], y - c[1]], -rot);
        let d = sdf_box(l, half, radius);
        let cov = if soft > 0.0 { 1.0 - smoothstep(-soft, soft, d) } else { (0.5 - d).clamp(0.0, 1.0) };
        blend(px, col, cov * alpha);
    });
}

/// Draw `src` (already at its size on the page, in pixels) centred at `c`, turned by `rot`.
fn draw_image(img: &mut RgbaImage, src: &RgbaImage, c: P, rot: f32, alpha: f32) {
    let (sw, sh) = (src.width() as f32, src.height() as f32);
    if sw < 1.0 || sh < 1.0 {
        return;
    }
    let sample = |x: f32, y: f32| -> [f32; 4] {
        // bilinear, clamped to the edge
        let (x, y) = ((x - 0.5).clamp(0.0, sw - 1.0), (y - 0.5).clamp(0.0, sh - 1.0));
        let (x0, y0) = (x.floor() as u32, y.floor() as u32);
        let (x1, y1) = ((x0 + 1).min(src.width() - 1), (y0 + 1).min(src.height() - 1));
        let (fx, fy) = (x - x0 as f32, y - y0 as f32);
        let p = |xx: u32, yy: u32| src.get_pixel(xx, yy).0;
        let (a, b, c, d) = (p(x0, y0), p(x1, y0), p(x0, y1), p(x1, y1));
        [0, 1, 2, 3].map(|i| {
            let top = a[i] as f32 * (1.0 - fx) + b[i] as f32 * fx;
            let bot = c[i] as f32 * (1.0 - fx) + d[i] as f32 * fx;
            top * (1.0 - fy) + bot * fy
        })
    };
    each_pixel(img, bounds(c, [sw, sh], rot, 1.0), |x, y, px| {
        let l = rotate([x - c[0], y - c[1]], -rot);
        let (lx, ly) = (l[0] + sw / 2.0, l[1] + sh / 2.0);
        let cov = (lx + 0.5).min(sw - lx + 0.5).min(ly + 0.5).min(sh - ly + 0.5).clamp(0.0, 1.0);
        if cov <= 0.0 {
            return;
        }
        let s = sample(lx, ly);
        blend(px, [s[0], s[1], s[2]], cov * alpha * s[3] / 255.0);
    });
}

/// The part of `src` with the shape `aspect`, from its middle.
fn crop_to(src: &RgbaImage, aspect: f32) -> RgbaImage {
    let (w, h) = (src.width() as f32, src.height() as f32);
    let (cw, ch) = if w / h > aspect { (h * aspect, h) } else { (w, w / aspect) };
    let (cw, ch) = ((cw.round() as u32).clamp(1, src.width()), (ch.round() as u32).clamp(1, src.height()));
    imageops::crop_imm(src, (src.width() - cw) / 2, (src.height() - ch) / 2, cw, ch).to_image()
}

fn px_size(size: P, k: f32) -> (u32, u32) {
    (((size[0] * k).round() as u32).clamp(1, MAX_PAGE_PX), ((size[1] * k).round() as u32).clamp(1, MAX_PAGE_PX))
}

/// The pixels a picture needs on its longest side to fill `size` at `k` pixels per millimetre.
pub fn needed_px(size: P, k: f32) -> u32 {
    ((size[0].max(size[1]) * k * 1.25).ceil() as u32).clamp(64, 5000)
}

/// A small, blurred copy of a picture, stretched over the page for a soft-focus background.
pub fn soft_picture(src: &RgbaImage) -> RgbaImage {
    let (sw, sh) = crate::imaging::fit(src.width(), src.height(), 96);
    let small = imageops::resize(src, sw, sh, imageops::FilterType::Triangle);
    imageops::blur(&small, 2.2)
}

/// The background: colour, gradient, or the picture (cropped to fill the page, maybe blurred).
pub fn paint_background(img: &mut RgbaImage, bg: &Background, picture: Option<&RgbaImage>) {
    let (w, h) = img.dimensions();
    if let (BgKind::Picture, Some(src)) = (bg.kind, picture) {
        let part = crop_to(src, w as f32 / h.max(1) as f32);
        let filled = if bg.blur {
            imageops::resize(&soft_picture(&part), w, h, imageops::FilterType::CatmullRom)
        } else {
            imageops::resize(&part, w, h, imageops::FilterType::CatmullRom)
        };
        imageops::replace(img, &filled, 0, 0);
        return;
    }
    let (fw, fh) = (w.max(1) as f32, h.max(1) as f32);
    let stride = w as usize * 4;
    img.as_mut().par_chunks_mut(stride.max(4)).enumerate().for_each(|(y, row)| {
        for x in 0..w as usize {
            let c = bg.color_at((x as f32 + 0.5) / fw, (y as f32 + 0.5) / fh);
            row[x * 4..x * 4 + 4].copy_from_slice(&[c[0], c[1], c[2], 255]);
        }
    });
}

/// Draw a page at `k` pixels per millimetre. `load(pic, px)` gives the picture (edited) with
/// about `px` pixels on its longest side; a missing picture leaves a grey box.
pub fn render(page: &Page, k: f32, bg: &Background, bg_picture: Option<&RgbaImage>, load: &mut dyn FnMut(usize, u32) -> Option<RgbaImage>) -> RgbaImage {
    let k = if k.is_finite() && k > 0.0 { k } else { 4.0 };
    let k = k.min(MAX_PAGE_PX as f32 / page.size[0].max(page.size[1]).max(1.0));
    let (w, h) = px_size(page.size, k);
    let mut img = RgbaImage::from_pixel(w, h, Rgba([255, 255, 255, 255]));
    paint_background(&mut img, bg, bg_picture);
    let sc = |p: P| [p[0] * k, p[1] * k];
    for shape in &page.shapes {
        match shape {
            Shape::Photo(ph) => {
                let (oc, os) = ph.outer();
                if ph.shadow {
                    let m = os[0].min(os[1]);
                    let off = (m * 0.012 + 0.35) * k;
                    let soft = (m * 0.025 + 0.6) * k;
                    fill_box(&mut img, [oc[0] * k + off * 0.6, oc[1] * k + off], sc(os), ph.rot, soft * 0.5, [0, 0, 0, 105], soft);
                }
                if ph.frame != NO_FRAME {
                    fill_box(&mut img, sc(oc), sc(os), ph.rot, 0.0, ph.frame_color, 0.0);
                }
                let (tw, th) = px_size(ph.size, k);
                let pic = load(ph.pic, needed_px(ph.size, k));
                let part = match &pic {
                    Some(p) => imageops::resize(&crop_to(p, ph.size[0] / ph.size[1].max(1e-3)), tw, th, imageops::FilterType::CatmullRom),
                    None => RgbaImage::from_pixel(tw, th, Rgba([200, 200, 200, 255])),
                };
                draw_image(&mut img, &part, sc(ph.c), ph.rot, ph.alpha.clamp(0.0, 1.0));
            }
            Shape::Rect { c, size, rot, color, radius } => fill_box(&mut img, sc(*c), sc(*size), *rot, radius * k, *color, 0.0),
            Shape::Line { pts, width, color } => {
                for s in pts.windows(2) {
                    let (a, b) = (sc(s[0]), sc(s[1]));
                    let len = ((b[0] - a[0]).powi(2) + (b[1] - a[1]).powi(2)).sqrt();
                    let rot = (b[1] - a[1]).atan2(b[0] - a[0]);
                    let wpx = (width * k).max(1.0);
                    fill_box(&mut img, [(a[0] + b[0]) / 2.0, (a[1] + b[1]) / 2.0], [len + wpx, wpx], rot, wpx / 2.0, *color, 0.0);
                }
            }
            Shape::Text { c, text, size, rot, color } => {
                if let Some(t) = text_image(text, size * k, *color) {
                    draw_image(&mut img, &t, sc(*c), *rot, 1.0);
                }
            }
        }
    }
    img
}

// ---------------------------------------------------------------------------------------------
// Making the files

/// Everything needed to draw the pages away from the window.
#[derive(Clone, Debug)]
pub struct Job {
    pub pages: Vec<Page>,
    pub background: Background,
    /// The background picture (file and its edits), for `BgKind::Picture`.
    pub bg_picture: Option<(String, Vec<EditOp>)>,
    /// The pictures (file and edits) the pages' `pic` numbers refer to.
    pub pictures: Vec<(String, Vec<EditOp>)>,
}

/// A picture reader: (file, longest side) → picture.
pub type Reader<'a> = &'a (dyn Fn(&str, u32) -> Option<RgbaImage> + Sync);

/// Draw one page of the job at `k` pixels per millimetre.
pub fn render_job_page(job: &Job, page: usize, k: f32, read: Reader) -> Option<RgbaImage> {
    let p = job.pages.get(page)?;
    let bg = if job.background.kind == BgKind::Picture {
        job.bg_picture.as_ref().and_then(|(key, edits)| read(key, 1600).map(|i| crate::imaging::apply_all(&i, edits)))
    } else {
        None
    };
    let mut load = |pic: usize, px: u32| {
        let (key, edits) = job.pictures.get(pic)?;
        read(key, px).map(|i| crate::imaging::apply_all(&i, edits))
    };
    Some(render(p, k, &job.background, bg.as_ref(), &mut load))
}

/// Save the pages: a PDF (one page per page) or, for `.png` / `.jpg`, a picture per page
/// ("name-2.png" for the second). Returns the files written.
pub fn save(job: &Job, path: &Path, px_long: u32, read: Reader) -> Result<Vec<std::path::PathBuf>, String> {
    let Some(first) = job.pages.first() else { return Err("there is nothing to save".into()) };
    let ext = path.extension().map(|e| e.to_string_lossy().to_ascii_lowercase()).unwrap_or_default();
    let write = |target: &Path, bytes: &[u8]| -> Result<(), String> {
        let tmp = target.with_extension(format!("{ext}.part"));
        std::fs::write(&tmp, bytes).map_err(|e| format!("{}: {e}", target.display()))?;
        std::fs::rename(&tmp, target).map_err(|e| {
            let _ = std::fs::remove_file(&tmp);
            format!("{}: {e}", target.display())
        })
    };
    match ext.as_str() {
        "pdf" => {
            // 200 dots per inch
            let k = 200.0 / 25.4;
            let mut jpegs = vec![];
            for i in 0..job.pages.len() {
                let img = render_job_page(job, i, k, read).ok_or("a page could not be drawn")?;
                jpegs.push(jpeg_bytes(&img, 90)?);
            }
            let [w, h] = first.size;
            let paper = crate::print::Paper { sheet_w: w, sheet_h: h, area_x: 0.0, area_y: 0.0, area_w: w, area_h: h, dpi: 200 };
            write(path, &crate::print::pdf_of_pages(&jpegs, &paper))?;
            Ok(vec![path.to_path_buf()])
        }
        "png" | "jpg" | "jpeg" => {
            let mut out = vec![];
            let stem = path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| "Collage".into());
            for (i, p) in job.pages.iter().enumerate() {
                let target = if i == 0 { path.to_path_buf() } else { path.with_file_name(format!("{stem}-{}.{ext}", i + 1)) };
                let k = px_long.clamp(256, MAX_PAGE_PX) as f32 / p.size[0].max(p.size[1]).max(1.0);
                let img = render_job_page(job, i, k, read).ok_or("a page could not be drawn")?;
                let bytes = if ext == "png" {
                    let mut buf = std::io::Cursor::new(vec![]);
                    image::DynamicImage::ImageRgba8(img).to_rgb8().write_to(&mut buf, image::ImageFormat::Png).map_err(|e| e.to_string())?;
                    buf.into_inner()
                } else {
                    jpeg_bytes(&img, 92)?.0
                };
                write(&target, &bytes)?;
                out.push(target);
            }
            Ok(out)
        }
        _ => Err("save as .pdf, .png or .jpg".into()),
    }
}

/// A page as JPEG bytes with its size, for PDF pages.
pub fn jpeg_bytes(img: &RgbaImage, quality: u8) -> Result<(Vec<u8>, u32, u32), String> {
    let rgb = image::DynamicImage::ImageRgba8(img.clone()).to_rgb8();
    let mut buf = vec![];
    rgb.write_with_encoder(image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buf, quality)).map_err(|e| e.to_string())?;
    Ok((buf, rgb.width(), rgb.height()))
}

/// A page placed on a printer's printable area: turned when that fits better, scaled to fit,
/// centred on white.
pub fn on_paper(page: &RgbaImage, paper: &crate::print::Paper, dpi: u32) -> RgbaImage {
    let k = dpi.clamp(72, 600) as f32 / 25.4;
    let (aw, ah) = (((paper.area_w * k).round() as u32).clamp(1, MAX_PAGE_PX), ((paper.area_h * k).round() as u32).clamp(1, MAX_PAGE_PX));
    let page_land = page.width() > page.height();
    let area_land = aw > ah;
    let src = if page_land != area_land && page.width() != page.height() { imageops::rotate90(page) } else { page.clone() };
    let s = (aw as f32 / src.width() as f32).min(ah as f32 / src.height() as f32);
    let (w, h) = (((src.width() as f32 * s) as u32).clamp(1, aw), ((src.height() as f32 * s) as u32).clamp(1, ah));
    let scaled = imageops::resize(&src, w, h, imageops::FilterType::CatmullRom);
    let mut out = RgbaImage::from_pixel(aw, ah, Rgba([255, 255, 255, 255]));
    imageops::overlay(&mut out, &scaled, ((aw - w) / 2) as i64, ((ah - h) / 2) as i64);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pics(n: usize) -> Vec<Pic> {
        (0..n).map(|i| Pic { aspect: if i % 3 == 1 { 0.75 } else { 1.5 }, name: format!("IMG_{i:04}.JPG"), caption: String::new(), date: String::new() }).collect()
    }

    fn photos(p: &Page) -> Vec<&PhotoShape> {
        p.shapes.iter().filter_map(|s| if let Shape::Photo(ph) = s { Some(ph) } else { None }).collect()
    }

    #[test]
    fn every_style_places_every_picture_once() {
        for style in Style::ALL {
            for n in [1, 2, 3, 5, 12, 37] {
                let s = CollageSettings { style, ..Default::default() };
                let page = collage(&s, &pics(n), &[]);
                let ph = photos(&page);
                let mut seen: Vec<usize> = ph.iter().map(|p| p.pic).collect();
                seen.sort();
                seen.dedup();
                let want = match style {
                    Style::Blend => n.min(6),
                    _ => n,
                };
                assert_eq!(seen.len(), want, "{style:?} with {n}");
                for p in &ph {
                    assert!(p.size[0] > 0.0 && p.size[1] > 0.0 && p.c[0].is_finite() && p.c[1].is_finite(), "{style:?} {n}: {p:?}");
                }
            }
        }
    }

    #[test]
    fn mosaic_fills_the_page() {
        let s = CollageSettings { style: Style::Mosaic, gap: 0.0, ..Default::default() };
        let page = collage(&s, &pics(9), &[]);
        let area: f32 = photos(&page).iter().map(|p| p.size[0] * p.size[1]).sum();
        let [w, h] = page.size;
        assert!((area - w * h).abs() < w * h * 0.01, "{area} vs {}", w * h);
        let s = CollageSettings { style: Style::Frame, gap: 0.0, ..Default::default() };
        let page = collage(&s, &pics(9), &[]);
        let area: f32 = photos(&page).iter().map(|p| p.size[0] * p.size[1]).sum();
        assert!((area - w * h).abs() < w * h * 0.01, "centre stage {area} vs {}", w * h);
    }

    #[test]
    fn bad_shapes_and_no_pictures_never_crash() {
        let odd = vec![Pic { aspect: f32::NAN, ..Default::default() }, Pic { aspect: 0.0, ..Default::default() }, Pic { aspect: 1e9, ..Default::default() }];
        for style in Style::ALL {
            let s = CollageSettings { style, ..Default::default() };
            assert!(photos(&collage(&s, &[], &[])).is_empty());
            let page = collage(&s, &odd, &[Nudge { dx: f32::MAX, ..Default::default() }]);
            let img = render(&page, 0.5, &s.background, None, &mut |_, _| None);
            assert!(img.width() > 0);
        }
        assert_eq!(contact_sheet(&SheetSettings::default(), &[], "", "").len(), 1);
    }

    #[test]
    fn pile_moves_and_raises_a_nudged_picture() {
        let s = CollageSettings::default();
        let before = collage(&s, &pics(5), &[]);
        let mut nudges = vec![Nudge::default(); 5];
        nudges[2] = Nudge { dx: 10.0, dy: -5.0, rot: 0.1, z: 1 };
        let after = collage(&s, &pics(5), &nudges);
        let find = |p: &Page| photos(p).into_iter().find(|ph| ph.pic == 2).cloned().unwrap();
        let (a, b) = (find(&before), find(&after));
        assert!((b.c[0] - a.c[0] - 10.0).abs() < 1e-3 && (b.c[1] - a.c[1] + 5.0).abs() < 1e-3);
        assert_eq!(photos(&after).last().unwrap().pic, 2, "a raised picture is drawn last");
        assert_eq!(after.hit(b.c).and_then(|i| if let Shape::Photo(p) = &after.shapes[i] { Some(p.pic) } else { None }), Some(2));
    }

    #[test]
    fn contact_sheet_pages_and_header() {
        let s = SheetSettings::default();
        let pages = contact_sheet(&s, &pics(100), "Holidays", "100 pictures, April 2006");
        assert!(pages.len() > 1);
        let total: usize = pages.iter().map(|p| photos(p).len()).sum();
        assert_eq!(total, 100);
        let has = |p: &Page, t: &str| p.shapes.iter().any(|s| matches!(s, Shape::Text { text, .. } if text == t));
        assert!(has(&pages[0], "Holidays") && !has(&pages[1], "Holidays"));
        assert!(has(&pages[0], "IMG_0000.JPG"));
        assert!(has(&pages[1], &format!("2 / {}", pages.len())));
        // every picture inside the page
        for p in &pages {
            for ph in photos(p) {
                assert!(ph.c[1] + ph.size[1] / 2.0 <= p.size[1] && ph.c[0] + ph.size[0] / 2.0 <= p.size[0]);
            }
        }
        // bigger pictures: more pages
        let big = contact_sheet(&SheetSettings { thumb_mm: 60.0, ..s }, &pics(100), "", "");
        assert!(big.len() > pages.len());
    }

    #[test]
    fn renders_pictures_background_and_text() {
        let s = CollageSettings { style: Style::Grid, border: Border::None, shadow: false, gap: 0.0, background: Background { color: [0, 0, 255], ..Default::default() }, ..Default::default() };
        let page = collage(&s, &pics(1)[..1], &[]);
        let red = RgbaImage::from_pixel(300, 200, Rgba([220, 10, 10, 255]));
        let img = render(&page, 2.0, &s.background, None, &mut |_, _| Some(red.clone()));
        let mid = img.get_pixel(img.width() / 2, img.height() / 2);
        assert_eq!(mid[0], 220);
        // a gradient goes from the first colour to the second
        let bg = Background { kind: BgKind::Gradient, color: [0, 0, 0], color2: [200, 200, 200], ..Default::default() };
        let mut g = RgbaImage::new(10, 100);
        paint_background(&mut g, &bg, None);
        assert!(g.get_pixel(5, 0)[0] < 5 && g.get_pixel(5, 99)[0] > 195);
        // text draws dark pixels
        let t = text_image("Hello", 30.0, [0, 0, 0, 255]).unwrap();
        assert!(t.pixels().any(|p| p[3] > 200));
        assert!(fit_text("A very long file name indeed.jpg", 3.0, 20.0).ends_with('…'));
    }

    #[test]
    fn subtitle_texts() {
        assert_eq!(sheet_subtitle(&[]), "0 pictures");
        let t = 1_146_000_000; // April 2006
        assert!(sheet_subtitle(&[t, t + 3600]).starts_with("2 pictures, April 2006") || sheet_subtitle(&[t]).contains("2006"));
        assert!(sheet_subtitle(&[t, t + 86_400 * 40]).contains(" – "));
    }

    #[test]
    fn saves_pdf_and_pictures() {
        let dir = std::env::temp_dir().join(format!("albumcraft-collage-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let pages = contact_sheet(&SheetSettings { thumb_mm: 80.0, ..Default::default() }, &pics(8), "Test", "");
        assert!(pages.len() >= 2);
        let job = Job { pages, background: Background::white(), bg_picture: None, pictures: (0..8).map(|i| (format!("{}{i}", crate::imaging::SAMPLE_PREFIX), vec![])).collect() };
        let read = |k: &str, d: u32| crate::imaging::load_source(k, d);
        let pdf = dir.join("sheet.pdf");
        save(&job, &pdf, 0, &read).unwrap();
        let bytes = std::fs::read(&pdf).unwrap();
        assert!(bytes.starts_with(b"%PDF") && String::from_utf8_lossy(&bytes).contains(&format!("/Count {}", job.pages.len())));
        let png = dir.join("sheet.png");
        let written = save(&job, &png, 800, &read).unwrap();
        assert_eq!(written.len(), job.pages.len());
        assert!(dir.join("sheet-2.png").exists());
        assert!(save(&job, &dir.join("sheet.txt"), 800, &read).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn page_goes_on_paper_turned_to_fit() {
        let page = RgbaImage::from_pixel(300, 200, Rgba([0, 0, 0, 255]));
        let paper = crate::print::Paper::a4();
        let out = on_paper(&page, &paper, 50);
        assert!(out.height() > out.width());
        // turned: fills the width
        let mid = out.get_pixel(out.width() / 2, out.height() / 2);
        assert_eq!(mid[0], 0);
    }

    /// `ALBUMCRAFT_COLLAGE_OUT=<dir> cargo test -p albumcraft-core demo_collages -- --ignored`
    #[test]
    #[ignore]
    fn demo_collages() {
        let dir = std::path::PathBuf::from(std::env::var("ALBUMCRAFT_COLLAGE_OUT").expect("set ALBUMCRAFT_COLLAGE_OUT"));
        let n = 14;
        let pictures: Vec<(String, Vec<EditOp>)> = (0..n).map(|i| (format!("{}{}", crate::imaging::SAMPLE_PREFIX, i % 8), vec![])).collect();
        let ps: Vec<Pic> = pictures.iter().map(|(k, _)| {
            let img = crate::imaging::load_source(k, 200).unwrap();
            Pic { aspect: img.width() as f32 / img.height() as f32, name: k.clone(), caption: "Caption".into(), date: String::new() }
        }).collect();
        let read = |k: &str, d: u32| crate::imaging::load_source(k, d);
        for style in Style::ALL {
            let s = CollageSettings { style, border: style.natural_border(), captions: CaptionMode::Caption, background: Background { kind: BgKind::Gradient, color: [250, 240, 220], color2: [180, 140, 110], ..Default::default() }, ..Default::default() };
            let job = Job { pages: vec![collage(&s, &ps, &[])], background: s.background, bg_picture: None, pictures: pictures.clone() };
            save(&job, &dir.join(format!("{}.jpg", style.id())), 1600, &read).unwrap();
        }
        let job = Job { pages: contact_sheet(&SheetSettings::default(), &ps, "Holidays", &sheet_subtitle(&[1_146_000_000])), background: Background::white(), bg_picture: None, pictures };
        save(&job, &dir.join("sheet.pdf"), 0, &read).unwrap();
        save(&job, &dir.join("sheet.png"), 1600, &read).unwrap();
    }
}
