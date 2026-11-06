//! The non-destructive edit pipeline (`EditOp`, `apply_all`) and the built-in sample pictures.
//! Works on pictures already in memory: reading files is `albumcraft_core::imaging`.

use image::{imageops, Rgba, RgbaImage};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Effect {
    Sharpen,
    Sepia,
    BlackWhite,
    Warmify,
    FilmGrain,
    Tint,
    Saturation,
    SoftFocus,
    Glow,
    FilteredBw,
    FocalBw,
    GraduatedTint,
}

impl Effect {
    pub fn label(&self) -> &'static str {
        match self {
            Effect::Sharpen => "Sharpen",
            Effect::Sepia => "Sepia",
            Effect::BlackWhite => "B & W",
            Effect::Warmify => "Warmify",
            Effect::FilmGrain => "Film Grain",
            Effect::Tint => "Tint",
            Effect::Saturation => "Saturation",
            Effect::SoftFocus => "Soft Focus",
            Effect::Glow => "Glow",
            Effect::FilteredBw => "Filtered B & W",
            Effect::FocalBw => "Focal B & W",
            Effect::GraduatedTint => "Graduated Tint",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum EditOp {
    /// Quarter turns clockwise (negative = counter-clockwise).
    Rotate(i32),
    /// Normalized rectangle x0, y0, x1, y1 of the image at this point of the stack.
    Crop([f32; 4]),
    /// Small rotation in degrees, zoomed so no empty corners show.
    Straighten(f32),
    Redeye([f32; 4]),
    Lucky,
    AutoContrast,
    AutoColor,
    Tune {
        fill: f32,
        highlights: f32,
        shadows: f32,
        temp: f32,
    },
    Effect(Effect),
    /// The Advanced adjustments (white balance, exposure, tone, colour, detail) as one step.
    Adjust(crate::develop::Adjust),
    /// A preset look at an intensity (0…1).
    Look { look: crate::looks::Look, amount: f32 },
    /// Vignette, border, shadow, rounded corners or text (the Effects tab).
    Decor(crate::decor::Decor),
    /// Perspective correction, each -1…1: `vertical` > 0 widens the top (buildings that lean
    /// back when shot from below stand up straight), `horizontal` > 0 makes the right side taller.
    /// Zoomed so no empty corners show.
    Perspective { vertical: f32, horizontal: f32 },
}

impl EditOp {
    pub fn label(&self) -> String {
        match self {
            EditOp::Rotate(n) if *n > 0 => "Rotate right".into(),
            EditOp::Rotate(_) => "Rotate left".into(),
            EditOp::Crop(_) => "Crop".into(),
            EditOp::Straighten(a) => format!("Straighten {a:+.1}°"),
            EditOp::Perspective { vertical, horizontal } => match (*vertical != 0.0, *horizontal != 0.0) {
                (true, false) => format!("Perspective, vertical {:+.0}", vertical * 100.0),
                (false, true) => format!("Perspective, horizontal {:+.0}", horizontal * 100.0),
                _ => format!("Perspective {:+.0} / {:+.0}", vertical * 100.0, horizontal * 100.0),
            },
            EditOp::Redeye(_) => "Redeye".into(),
            EditOp::Lucky => "Auto fix (lucky)".into(),
            EditOp::AutoContrast => "Auto Contrast".into(),
            EditOp::AutoColor => "Auto Color".into(),
            EditOp::Tune { .. } => "Tuning".into(),
            EditOp::Effect(e) => e.label().into(),
            EditOp::Adjust(a) => a.label(),
            EditOp::Look { look, amount } => format!("{} {:.0} %", look.label(), amount * 100.0),
            EditOp::Decor(d) => d.label(),
        }
    }
}

// ---------------------------------------------------------------------------
// Loading

pub const SAMPLE_PREFIX: &str = "sample://";

/// A built-in sample picture (key `sample://<seed>`) scaled so its longest side is at most
/// `max_dim` (0 = full size). None for any other key.
pub fn sample_source(key: &str, max_dim: u32) -> Option<RgbaImage> {
    let seed: u32 = key.strip_prefix(SAMPLE_PREFIX)?.parse().ok()?;
    let (w, h) = sample_size(seed);
    let img = sample_image(seed, w, h);
    if max_dim > 0 && (img.width() > max_dim || img.height() > max_dim) {
        let (w, h) = fit(img.width(), img.height(), max_dim);
        Some(imageops::resize(&img, w, h, imageops::FilterType::Triangle))
    } else {
        Some(img)
    }
}

pub fn fit(w: u32, h: u32, max_dim: u32) -> (u32, u32) {
    let s = max_dim as f32 / w.max(h) as f32;
    (((w as f32 * s).round() as u32).max(1), ((h as f32 * s).round() as u32).max(1))
}

pub fn sample_size(seed: u32) -> (u32, u32) {
    if seed % 5 == 3 { (900, 1200) } else { (1200, 900) }
}

struct Rng(u32);
impl Rng {
    fn next(&mut self) -> f32 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.0 = x;
        (x % 10_000) as f32 / 10_000.0
    }
}

fn lerp3(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, a[2] + (b[2] - a[2]) * t]
}

/// A simple painted landscape used as a placeholder photo.
pub fn sample_image(seed: u32, w: u32, h: u32) -> RgbaImage {
    let mut rng = Rng(seed.wrapping_mul(2_654_435_761).max(1));
    for _ in 0..4 {
        rng.next();
    }
    let palettes: [([f32; 3], [f32; 3], [f32; 3]); 6] = [
        ([0.25, 0.45, 0.85], [0.85, 0.92, 1.0], [0.20, 0.50, 0.25]), // day
        ([0.35, 0.20, 0.55], [1.0, 0.60, 0.35], [0.25, 0.18, 0.30]), // sunset
        ([0.10, 0.20, 0.45], [0.55, 0.75, 0.95], [0.85, 0.90, 0.95]), // snow
        ([0.45, 0.70, 0.95], [0.95, 0.95, 0.85], [0.80, 0.65, 0.40]), // beach
        ([0.30, 0.50, 0.70], [0.80, 0.85, 0.80], [0.30, 0.42, 0.20]), // hazy
        ([0.95, 0.55, 0.25], [1.0, 0.85, 0.55], [0.45, 0.25, 0.15]), // autumn
    ];
    let (sky_top, sky_bot, ground) = palettes[(seed as usize / 2 + seed as usize) % palettes.len()];
    let horizon = 0.45 + rng.next() * 0.2;
    let sun = (0.15 + rng.next() * 0.7, 0.12 + rng.next() * 0.25, 0.05 + rng.next() * 0.05);
    let water = seed.is_multiple_of(3);
    let layers: Vec<(f32, f32, f32, f32)> = (0..3)
        .map(|_| (rng.next() * 6.0, 2.0 + rng.next() * 5.0, 0.03 + rng.next() * 0.08, rng.next()))
        .collect();
    let mut img = RgbaImage::new(w, h);
    let aspect = w as f32 / h as f32;
    for y in 0..h {
        let fy = y as f32 / h as f32;
        for x in 0..w {
            let fx = x as f32 / w as f32;
            let mut c = lerp3(sky_top, sky_bot, (fy / horizon).min(1.0).powf(1.4));
            // sun
            let dx = (fx - sun.0) * aspect;
            let dy = fy - sun.1;
            let d = (dx * dx + dy * dy).sqrt();
            if d < sun.2 {
                c = lerp3(c, [1.0, 0.97, 0.8], 0.95);
            } else {
                let g = (1.0 - (d - sun.2) * 4.0).max(0.0) * 0.35;
                c = lerp3(c, [1.0, 0.95, 0.75], g);
            }
            // hills
            for (i, &(ph, fr, amp, _)) in layers.iter().enumerate() {
                let base = horizon + i as f32 * 0.08 - 0.05;
                let top = base - amp * ((fx * fr + ph).sin() * 0.6 + (fx * fr * 2.3 + ph * 1.7).sin() * 0.4 + 1.0);
                if fy > top {
                    let shade = 0.55 + i as f32 * 0.22;
                    let mut gc = [ground[0] * shade, ground[1] * shade, ground[2] * shade];
                    let haze = (2 - i as i32) as f32 * 0.18;
                    gc = lerp3(gc, sky_bot, haze);
                    c = gc;
                }
            }
            if water && fy > horizon + 0.18 {
                let ripple = ((fy * 120.0 + (fx * 9.0).sin() * 2.0).sin() * 0.04) + 0.0;
                c = lerp3([sky_top[0] * 0.8, sky_top[1] * 0.85, sky_top[2] * 0.9], sky_bot, (1.0 - fy) * 0.8 + ripple);
            }
            img.put_pixel(x, y, to_px(c[0], c[1], c[2]));
        }
    }
    img
}

// ---------------------------------------------------------------------------
// Edit pipeline

#[inline]
fn to_px(r: f32, g: f32, b: f32) -> Rgba<u8> {
    Rgba([
        (r.clamp(0.0, 1.0) * 255.0 + 0.5) as u8,
        (g.clamp(0.0, 1.0) * 255.0 + 0.5) as u8,
        (b.clamp(0.0, 1.0) * 255.0 + 0.5) as u8,
        255,
    ])
}

#[inline]
fn lum(r: f32, g: f32, b: f32) -> f32 {
    0.299 * r + 0.587 * g + 0.114 * b
}

fn smoothstep(a: f32, b: f32, x: f32) -> f32 {
    let t = ((x - a) / (b - a)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Per-pixel map with x, y in normalized coordinates.
fn map_pixels(img: &mut RgbaImage, f: impl Fn(f32, f32, [f32; 3]) -> [f32; 3]) {
    let (w, h) = (img.width() as f32, img.height() as f32);
    for (x, y, p) in img.enumerate_pixels_mut() {
        let c = [p[0] as f32 / 255.0, p[1] as f32 / 255.0, p[2] as f32 / 255.0];
        let o = f(x as f32 / w, y as f32 / h, c);
        let a = p[3];
        *p = to_px(o[0], o[1], o[2]);
        p[3] = a;
    }
}

fn combine(img: &mut RgbaImage, other: &RgbaImage, f: impl Fn(f32, f32, [f32; 3], [f32; 3]) -> [f32; 3]) {
    let (w, h) = (img.width() as f32, img.height() as f32);
    for (x, y, p) in img.enumerate_pixels_mut() {
        let q = other.get_pixel(x, y);
        let c = [p[0] as f32 / 255.0, p[1] as f32 / 255.0, p[2] as f32 / 255.0];
        let d = [q[0] as f32 / 255.0, q[1] as f32 / 255.0, q[2] as f32 / 255.0];
        let o = f(x as f32 / w, y as f32 / h, c, d);
        *p = to_px(o[0], o[1], o[2]);
    }
}

/// Cheap blur: shrink then grow back.
fn cheap_blur(img: &RgbaImage, k: u32) -> RgbaImage {
    let k = k.max(2);
    let sw = (img.width() / k).max(1);
    let sh = (img.height() / k).max(1);
    let small = imageops::resize(img, sw, sh, imageops::FilterType::Triangle);
    imageops::resize(&small, img.width(), img.height(), imageops::FilterType::Triangle)
}

fn rect_px(img: &RgbaImage, r: [f32; 4]) -> (u32, u32, u32, u32) {
    let (w, h) = (img.width() as f32, img.height() as f32);
    let x0 = (r[0].min(r[2]).clamp(0.0, 1.0) * w) as u32;
    let y0 = (r[1].min(r[3]).clamp(0.0, 1.0) * h) as u32;
    let x1 = ((r[0].max(r[2]).clamp(0.0, 1.0) * w) as u32).max(x0 + 1).min(img.width());
    let y1 = ((r[1].max(r[3]).clamp(0.0, 1.0) * h) as u32).max(y0 + 1).min(img.height());
    (x0.min(img.width() - 1), y0.min(img.height() - 1), x1, y1)
}

fn bilinear(img: &RgbaImage, x: f32, y: f32) -> Rgba<u8> {
    let w = img.width() as i32;
    let h = img.height() as i32;
    let x0 = x.floor() as i32;
    let y0 = y.floor() as i32;
    let fx = x - x0 as f32;
    let fy = y - y0 as f32;
    let get = |xx: i32, yy: i32| *img.get_pixel(xx.clamp(0, w - 1) as u32, yy.clamp(0, h - 1) as u32);
    let (a, b, c, d) = (get(x0, y0), get(x0 + 1, y0), get(x0, y0 + 1), get(x0 + 1, y0 + 1));
    let mut out = [0u8; 4];
    for i in 0..4 {
        let top = a[i] as f32 * (1.0 - fx) + b[i] as f32 * fx;
        let bot = c[i] as f32 * (1.0 - fx) + d[i] as f32 * fx;
        out[i] = (top * (1.0 - fy) + bot * fy + 0.5) as u8;
    }
    Rgba(out)
}

/// The edits worth copying to other pictures: everything except red-eye fixes (they mark this
/// picture's eyes).
pub fn copyable_edits(edits: &[EditOp]) -> Vec<EditOp> {
    edits.iter().filter(|op| !matches!(op, EditOp::Redeye(_))).cloned().collect()
}

pub fn apply_all(src: &RgbaImage, ops: &[EditOp]) -> RgbaImage {
    let mut img = src.clone();
    for op in ops {
        img = apply(img, op);
    }
    img
}

pub fn apply(mut img: RgbaImage, op: &EditOp) -> RgbaImage {
    match op {
        EditOp::Rotate(n) => match n.rem_euclid(4) {
            1 => imageops::rotate90(&img),
            2 => imageops::rotate180(&img),
            3 => imageops::rotate270(&img),
            _ => img,
        },
        EditOp::Crop(r) => {
            let (x0, y0, x1, y1) = rect_px(&img, *r);
            imageops::crop_imm(&img, x0, y0, x1 - x0, y1 - y0).to_image()
        }
        EditOp::Straighten(deg) => {
            let a = deg.to_radians();
            let (w, h) = (img.width() as f32, img.height() as f32);
            let ratio = w.max(h) / w.min(h);
            let s = a.cos().abs() + a.sin().abs() * ratio;
            let (cx, cy) = (w / 2.0, h / 2.0);
            let (ca, sa) = (a.cos(), a.sin());
            let mut out = RgbaImage::new(img.width(), img.height());
            for (x, y, p) in out.enumerate_pixels_mut() {
                let dx = (x as f32 - cx) / s;
                let dy = (y as f32 - cy) / s;
                let sx = cx + dx * ca + dy * sa;
                let sy = cy - dx * sa + dy * ca;
                *p = bilinear(&img, sx, sy);
            }
            out
        }
        EditOp::Redeye(r) => {
            let (x0, y0, x1, y1) = rect_px(&img, *r);
            for y in y0..y1 {
                for x in x0..x1 {
                    let p = img.get_pixel_mut(x, y);
                    let (r, g, b) = (p[0] as f32, p[1] as f32, p[2] as f32);
                    if r > 80.0 && r > 1.4 * g.max(b) {
                        p[0] = ((g + b) / 2.0) as u8;
                    }
                }
            }
            img
        }
        EditOp::AutoContrast => {
            auto_contrast(&mut img);
            img
        }
        EditOp::AutoColor => {
            auto_color(&mut img);
            img
        }
        EditOp::Lucky => {
            auto_color(&mut img);
            auto_contrast(&mut img);
            img
        }
        EditOp::Tune { fill, highlights, shadows, temp } => {
            // Fine tuning: fill light, then levels with
            // black point = shadows and white point = 1 - highlights, then colour temperature.
            let (fill, hi, sh, t) = (*fill, *highlights, *shadows, *temp);
            fill_light(&mut img, fill);
            let mut white = 1.0 - hi;
            if white <= 0.001 {
                white = 0.001;
            }
            if white != 1.0 || sh != 0.0 {
                levels(&mut img, sh, white);
            }
            if t != 0.0 {
                temperature(&mut img, t);
            }
            img
        }
        EditOp::Effect(e) => apply_effect(img, *e),
        EditOp::Adjust(a) => {
            crate::develop::apply(&mut img, a);
            img
        }
        EditOp::Look { look, amount } => {
            crate::looks::apply(&mut img, *look, *amount);
            img
        }
        EditOp::Decor(d) => d.apply(img),
        EditOp::Perspective { vertical, horizontal } => perspective(&img, *vertical, *horizontal),
    }
}

/// How strongly the perspective sliders bend the picture: at ±1 one edge is shown 1.8 times
/// wider than the other.
const KEYSTONE: f32 = 0.4;

/// Keystone correction: each row is scaled horizontally by an amount that changes from top to
/// bottom (vertical), each column vertically from left to right (horizontal), then the whole is
/// zoomed so that the picture still fills its frame.
fn perspective(img: &RgbaImage, vertical: f32, horizontal: f32) -> RgbaImage {
    use rayon::prelude::*;
    let v =if vertical.is_finite() { vertical.clamp(-1.0, 1.0) } else { 0.0 };
    let hz = if horizontal.is_finite() { horizontal.clamp(-1.0, 1.0) } else { 0.0 };
    let (w, h) = (img.width() as f32, img.height() as f32);
    if (v == 0.0 && hz == 0.0) || w < 2.0 || h < 2.0 {
        return img.clone();
    }
    let (cx, cy) = (w / 2.0, h / 2.0);
    // the widest sampling factor is at the edge that gets squeezed: zoom by its inverse
    let s = 1.0 / ((1.0 + KEYSTONE * v.abs()) * (1.0 + KEYSTONE * hz.abs()));
    let mut out = RgbaImage::new(img.width(), img.height());
    out.par_enumerate_pixels_mut().for_each(|(x, y, p)| {
        let (dx, dy) = (x as f32 + 0.5 - cx, y as f32 + 0.5 - cy);
        let (nx, ny) = (dx / cx, dy / cy);
        // top (ny = -1) samples a narrower stretch of the source when v > 0, so it shows wider
        let fx = 1.0 + KEYSTONE * v * ny;
        let fy = 1.0 - KEYSTONE * hz * nx;
        *p = bilinear(img, cx + dx * fx * s - 0.5, cy + dy * fy * s - 0.5);
    });
    out
}

fn auto_contrast(img: &mut RgbaImage) {
    let mut hist = [0u32; 256];
    for p in img.pixels() {
        hist[lum(p[0] as f32, p[1] as f32, p[2] as f32).round().clamp(0.0, 255.0) as usize] += 1;
    }
    let total: u32 = hist.iter().sum();
    let cut = (total as f32 * 0.005) as u32;
    let (mut lo, mut hi) = (0usize, 255usize);
    let mut acc = 0;
    for (i, v) in hist.iter().enumerate() {
        acc += v;
        if acc > cut {
            lo = i;
            break;
        }
    }
    acc = 0;
    for (i, v) in hist.iter().enumerate().rev() {
        acc += v;
        if acc > cut {
            hi = i;
            break;
        }
    }
    if hi <= lo + 10 {
        return;
    }
    let (lo, hi) = (lo as f32 / 255.0, hi as f32 / 255.0);
    map_pixels(img, |_, _, c| c.map(|v| (v - lo) / (hi - lo)));
}

fn auto_color(img: &mut RgbaImage) {
    let mut sum = [0f64; 3];
    for p in img.pixels() {
        for i in 0..3 {
            sum[i] += p[i] as f64;
        }
    }
    let avg = (sum[0] + sum[1] + sum[2]) / 3.0;
    if avg <= 0.0 {
        return;
    }
    let gain = sum.map(|s| ((avg / s.max(1.0)) as f32).clamp(0.75, 1.35));
    map_pixels(img, |_, _, c| [c[0] * gain[0], c[1] * gain[1], c[2] * gain[2]]);
}

// ---------------------------------------------------------------------------
// Filters.

/// Gray value used by sepia and desaturation: (77 R + 151 G + 28 B) >> 8.
fn gray(r: u8, g: u8, b: u8) -> u8 {
    ((r as u32 * 77 + g as u32 * 151 + b as u32 * 28) >> 8) as u8
}

/// Sepia: gray, squeezed toward white (x218/256),
/// then an overlay blend with the tone colour R155 G125 B99.
fn sepia(img: &mut RgbaImage) {
    const TONE: [u32; 3] = [155, 125, 99];
    for p in img.pixels_mut() {
        let l = gray(p[0], p[1], p[2]) as u32;
        let v = 255 - (((255 - l) * 218) >> 8);
        for i in 0..3 {
            let c = TONE[i];
            let out = if v < 128 { (2 * v * c) >> 8 } else { 255 - ((2 * (255 - v) * (255 - c)) >> 8) };
            p[i] = out.min(255) as u8;
        }
    }
}

/// Warmify lookup table:
/// 256 entries of (blue, green, red); each channel maps through its own curve.
const WARM_TABLE_HEX: &str = "000000ff010101ff010203ff020304ff030305ff030406ff040508ff050609ff05070aff06080cff07090dff07090eff080a0fff090b11ff090c12ff0a0d13ff0b0e14ff0b0f16ff0c1017ff0d1018ff0d111aff0e121bff0f131cff0f141dff10151fff111620ff111721ff121822ff131924ff131a25ff141b26ff151c27ff151d29ff161e2aff171f2bff17202cff18212eff19212fff1a2230ff1a2331ff1b2432ff1c2534ff1c2635ff1d2736ff1e2837ff1e2938ff1f2a3aff202b3bff202c3cff212d3dff222e3eff232f40ff233041ff243142ff253243ff253344ff263446ff273547ff283648ff283749ff29384aff2a394bff2b3a4cff2b3b4eff2c3c4fff2d3d50ff2d3e51ff2e3f52ff2f4053ff304154ff304155ff314256ff324357ff334459ff33455aff34465bff35475cff36485dff37495eff374a5fff384b60ff394c61ff3a4c62ff3a4d63ff3b4e64ff3c4f65ff3d5066ff3e5167ff3e5268ff3f5369ff40536aff41546bff42556cff42566dff43576eff44586fff455870ff465971ff475a71ff475b72ff485c73ff495c74ff4a5d75ff4b5e76ff4c5f77ff4c5f78ff4d6079ff4e617aff4f627aff50637bff51637cff52647dff52657eff53667fff546680ff556780ff566881ff576982ff586983ff596a84ff5a6b85ff5a6c85ff5b6c86ff5c6d87ff5d6e88ff5e6f89ff5f6f89ff60708aff61718bff62728cff63728dff64738dff65748eff65758fff667590ff677690ff687791ff697892ff6a7993ff6b7994ff6c7a94ff6d7b95ff6e7c96ff6f7c97ff707d97ff717e98ff727f99ff73809aff74809aff75819bff76829cff77839dff78849dff79849eff7a859fff7b86a0ff7c87a0ff7d88a1ff7e89a2ff7f89a3ff808aa3ff818ba4ff828ca5ff838da6ff848ea6ff858fa7ff8690a8ff8790a8ff8891a9ff8992aaff8a93abff8b94abff8c95acff8e96adff8f97aeff9098aeff9199afff9299b0ff939ab1ff949bb1ff959cb2ff969db3ff979eb4ff989fb4ff99a0b5ff9aa1b6ff9ca2b7ff9da3b8ff9ea4b8ff9fa5b9ffa0a6baffa1a7bbffa2a8bbffa3a9bcffa4aabdffa5abbeffa7abbfffa8acbfffa9adc0ffaaaec1ffabafc2ffacb0c3ffadb1c3ffaeb2c4ffafb3c5ffb1b4c6ffb2b5c7ffb3b6c7ffb4b7c8ffb5b8c9ffb6b9caffb7bacbffb8bbccffbabccdffbbbdcdffbcbeceffbdbfcfffbec0d0ffbfc1d1ffc0c3d2ffc2c4d3ffc3c5d3ffc4c6d4ffc5c7d5ffc6c8d6ffc7c9d7ffc8cad8ffcacbd9ffcbccdaffcccddaffcdcedbffcecfdcffcfd0ddffd1d1deffd2d2dfffd3d3e0ffd4d4e1ffd5d5e2ffd6d6e3ffd7d7e3ffd9d8e4ffdad9e5ffdbdae6ffdcdbe7ffdddce8ffdedde9ffe0dfeaffe1e0ebffe2e1ecffe3e2edffe4e3edffe5e4eeffe7e5efffe8e6f0ffe9e7f1ffeae8f2ff";

/// The table as [r, g, b] per input value.
fn warm_table() -> &'static [[u8; 3]; 256] {
    static T: std::sync::OnceLock<[[u8; 3]; 256]> = std::sync::OnceLock::new();
    T.get_or_init(|| {
        let b = WARM_TABLE_HEX.as_bytes();
        let hex = |i: usize| u8::from_str_radix(std::str::from_utf8(&b[i..i + 2]).unwrap(), 16).unwrap();
        let mut t = [[0u8; 3]; 256];
        for (i, e) in t.iter_mut().enumerate() {
            let o = i * 8; // bytes in memory: blue, green, red, alpha
            *e = [hex(o + 4), hex(o + 2), hex(o)];
        }
        t
    })
}

/// Desaturate: each channel moves toward the gray (77R + 151G + 28B) >> 8 by k/256
/// (k = 256 gives black & white; film grain uses 200 on its noise).
fn desaturate(img: &mut RgbaImage, k: i32) {
    for p in img.pixels_mut() {
        let l = gray(p[0], p[1], p[2]) as i32;
        for i in 0..3 {
            let c = p[i] as i32;
            p[i] = (c + (((l - c) * k) >> 8)).clamp(0, 255) as u8;
        }
    }
}

/// One-click Saturation strength (the filter takes an amount; the default used for the
/// one-click effect).
pub const SATURATION_AMOUNT: f32 = 0.3;

/// Saturation (amount p > 0):
/// p' = 3p; brightness L = (2R + 5G + B) / 8; each channel's ratio to L goes through
/// its own curve pow(ratio, e) with e = 1 + 0.3p' (red), 1 + 0.7p' (green), 1 + 0.9p' (blue)
/// as tables of 2048 steps for ratios 0…8;
/// new channel = curve(ratio) × L; if one channel passes 255, all three are scaled down together.
fn saturation(img: &mut RgbaImage, p: f32) {
    let pp = p * 3.0;
    let exps = [1.0 + 0.3 * pp, 1.0 + 0.7 * pp, 1.0 + 0.9 * pp];
    let tables: Vec<Vec<i64>> = exps
        .iter()
        .map(|&e| (0..2048).map(|i| ((i as f32 * 8.0 / 2048.0).powf(e) * 256.0) as i64).collect())
        .collect();
    for px in img.pixels_mut() {
        let (r, g, b) = (px[0] as i64, px[1] as i64, px[2] as i64);
        let l = (b + 5 * g + 2 * r) >> 3;
        if l == 0 {
            continue;
        }
        let s = ((256i64 << 8) - 1) / l;
        let idx = |c: i64| (((s * c) >> 8) as usize).min(2047);
        let mut out = [
            (tables[0][idx(r)] * l) >> 8,
            (tables[1][idx(g)] * l) >> 8,
            (tables[2][idx(b)] * l) >> 8,
        ];
        let max = out[0].max(out[1]).max(out[2]);
        if max > 255 {
            let f = 0xff00 / max;
            for v in out.iter_mut() {
                *v = (*v * f) >> 8;
            }
        }
        for i in 0..3 {
            px[i] = out[i].clamp(0, 255) as u8;
        }
    }
}

/// Small Mersenne Twister (MT19937, 624 words) for the film grain.
struct Mt {
    s: [u32; 624],
    i: usize,
}
impl Mt {
    fn new(seed: u32) -> Self {
        let mut s = [0u32; 624];
        s[0] = seed;
        for i in 1..624 {
            s[i] = 1_812_433_253u32.wrapping_mul(s[i - 1] ^ (s[i - 1] >> 30)).wrapping_add(i as u32);
        }
        Mt { s, i: 624 }
    }
    fn next(&mut self) -> u32 {
        if self.i >= 624 {
            for k in 0..624 {
                let y = (self.s[k] & 0x8000_0000) | (self.s[(k + 1) % 624] & 0x7fff_ffff);
                let mut v = self.s[(k + 397) % 624] ^ (y >> 1);
                if y & 1 != 0 {
                    v ^= 0x9908_b0df;
                }
                self.s[k] = v;
            }
            self.i = 0;
        }
        let mut y = self.s[self.i];
        self.i += 1;
        y ^= y >> 11;
        y ^= (y << 7) & 0x9d2c_5680;
        y ^= (y << 15) & 0xefc6_0000;
        y ^ (y >> 18)
    }
}

/// Film grain (strength 0.5 for the one-click effect):
/// random noise pulled toward 127 by (1 - strength), blurred three times, mixed 210/256 with
/// fresh noise, then added with a midtone weight (160 - |128 - v|) / 256.
fn film_grain(img: &mut RgbaImage, strength: f32) {
    let (w, h) = img.dimensions();
    let mut rng = Mt::new(w.wrapping_mul(31).wrapping_add(h).wrapping_add(5489));
    let keep = (256.0 * (1.0 - strength.min(1.0))) as i32;
    let mut noise = RgbaImage::new(w, h);
    for p in noise.pixels_mut() {
        let r = rng.next();
        for i in 0..3 {
            let n = ((r >> (8 * i)) & 0xff) as i32;
            p[i] = (n + (((127 - n) * keep) >> 8)) as u8;
        }
        p[3] = 255;
    }
    // three small blur passes
    for _ in 0..3 {
        noise = cheap_blur(&noise, 2);
    }
    for p in noise.pixels_mut() {
        let r = rng.next();
        for i in 0..3 {
            let fresh = ((r >> (8 * i)) & 0xff) as i32;
            p[i] = (fresh + (((p[i] as i32 - fresh) * 210) >> 8)).clamp(0, 255) as u8;
        }
    }
    // desaturate(noise, 200): the noise is mostly gray
    desaturate(&mut noise, 200);
    for (p, n) in img.pixels_mut().zip(noise.pixels()) {
        for i in 0..3 {
            let v = p[i] as i32;
            let weight = 160 - (128 - v).abs();
            p[i] = (v + (((n[i] as i32 - 128) * weight) >> 8)).clamp(0, 255) as u8;
        }
    }
}

/// Levels (gamma 1): out = (i/255 - black) / (white - black),
/// computed in 1/65280 steps and clamped, applied to each channel.
fn levels(img: &mut RgbaImage, black: f32, white: f32) {
    let scale = if white != black { 1.0 / (white - black) } else { 1.0 };
    let b0 = black * 65280.0;
    let mut table = [0u8; 256];
    for (i, t) in table.iter_mut().enumerate() {
        let y = ((i as f32 * 0.003_921_569 * 65280.0 - b0) * scale).round() as i32;
        *t = (y.clamp(0, 0xff00) >> 8) as u8;
    }
    for p in img.pixels_mut() {
        for i in 0..3 {
            p[i] = table[p[i] as usize];
        }
    }
}

/// Colour-temperature colours (R, G, B)
/// (first entry R255 G56 B0, getting bluer); stored in memory as B, G, R, 0.
const TEMP_TABLE_HEX: &str = "0038ff000047ff000053ff00005dff000065ff00006dff000073ff000079ff00007eff000083ff001289ff00218eff002c93ff003698ff003f9dff0048a1ff004fa5ff0057a9ff005eadff0065b1ff006bb4ff0072b8ff0078bbff007ebeff0084c1ff0089c4ff008fc7ff0094c9ff0099ccff009fceff00a3d1ff00a8d3ff00add5ff00b1d7ff00b6d9ff00badbff00beddff00c2dfff00c6e1ff00cae3ff00cee4ff00d2e6ff00d5e8ff00d9e9ff00dcebff00e0ecff00e3eeff00e6efff00e9f0ff00ecf2ff00eff3ff00f2f4ff00f5f5ff00f8f6ff00fbf8ff00fdf9ff00fff9fe00fff7fc00fff6f900fff5f700fff3f500fff2f300fff1f000fff0ef00ffefed00ffeeeb00ffede900ffece700ffebe600ffeae400ffe9e300ffe8e100ffe7e000ffe6de00ffe6dd00ffe5dc00ffe4da00ffe3d900ffe3d800ffe2d700ffe1d600ffe1d400ffe0d300ffdfd200ffdfd100ffded000ffddcf00ffddcf00ffdcce00ffdccd00ffdbcc00ffdbcb00ffdaca00ffdac900ffd9c900ffd9c800ffd8c700ffd8c700ffd8c600ffd7c500ffd7c400ffd6c400ffd6c300ffd6c300ffd5c200ffd5c100ffd4c100ffd4c000ffd4c000ffd3bf00ffd3bf00ffd3be00ffd2be00ffd2bd00ffd2bd00ffd2bc00ffd1bc00ffd1bb00ffd1bb00ffd0ba00ffd0ba00ffd0b900ffd0b900ffcfb900ffcfb800ffcfb800ffcfb700ffceb700";

fn temp_color(index: usize) -> [f32; 3] {
    let b = TEMP_TABLE_HEX.as_bytes();
    let n = b.len() / 8;
    let o = index.min(n - 1) * 8;
    let hex = |i: usize| u8::from_str_radix(std::str::from_utf8(&b[i..i + 2]).unwrap(), 16).unwrap() as f32;
    // memory order per entry: byte0 = blue, byte1 = green, byte2 = red
    [hex(o + 4), hex(o + 2), hex(o)]
}

/// Colour temperature: pick the table colour at round(t * 37 + 55)
/// and white-balance the picture so that colour becomes neutral.
fn temperature(img: &mut RgbaImage, t: f32) {
    let idx = (t * 37.0 + 55.0).round().max(0.0) as usize;
    neutral_balance(img, temp_color(idx));
}

/// White balance: with the 3x3 matrix M below, the picked
/// colour c and a gray g of the same brightness ((77R + 151G + 28B) >> 8) are taken into M's
/// space; each pixel p becomes M^-1 * diag(M g / M c) * M * p.
pub fn neutral_balance(img: &mut RgbaImage, c: [f32; 3]) {
    const M: [[f32; 3]; 3] = [[1.9044, 0.4508, -0.3826], [-0.0532, 1.8018, 0.1995], [0.0491, -0.3057, 1.8576]];
    let c = c.map(|v| v.max(1.0));
    let l = ((c[0] as u32 * 77 + c[1] as u32 * 151 + c[2] as u32 * 28) >> 8) as f32;
    let mul = |v: [f32; 3]| [0, 1, 2].map(|r| M[r][0] * v[0] + M[r][1] * v[1] + M[r][2] * v[2]);
    let (mg, mc) = (mul([l, l, l]), mul(c));
    let d = [0, 1, 2].map(|i| mg[i] / mc[i]);
    // inverse of M
    let m = M;
    let det = m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1]) - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
        + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0]);
    let inv = [
        [(m[1][1] * m[2][2] - m[1][2] * m[2][1]) / det, (m[0][2] * m[2][1] - m[0][1] * m[2][2]) / det, (m[0][1] * m[1][2] - m[0][2] * m[1][1]) / det],
        [(m[1][2] * m[2][0] - m[1][0] * m[2][2]) / det, (m[0][0] * m[2][2] - m[0][2] * m[2][0]) / det, (m[0][2] * m[1][0] - m[0][0] * m[1][2]) / det],
        [(m[1][0] * m[2][1] - m[1][1] * m[2][0]) / det, (m[0][1] * m[2][0] - m[0][0] * m[2][1]) / det, (m[0][0] * m[1][1] - m[0][1] * m[1][0]) / det],
    ];
    let mut t = [[0f32; 3]; 3];
    for r in 0..3 {
        for col in 0..3 {
            t[r][col] = (0..3).map(|k| inv[r][k] * d[k] * m[k][col]).sum();
        }
    }
    for p in img.pixels_mut() {
        let v = [p[0] as f32, p[1] as f32, p[2] as f32];
        for r in 0..3 {
            p[r] = (t[r][0] * v[0] + t[r][1] * v[1] + t[r][2] * v[2]).round().clamp(0.0, 255.0) as u8;
        }
    }
}

/// Fill Light: a power curve built from the amount f, blended in by how
/// dark each pixel is (L = (B + 2G + R) / 4, weight (255 - L) / 256).
pub fn fill_light(img: &mut RgbaImage, f: f32) {
    if f <= 0.0 {
        return;
    }
    let a = 1.0 / ((1.0 - f) * 0.7 + 0.3);
    let e = 1.0 / (a * 0.7 + 0.3);
    let mut table = [0i32; 256];
    for (i, t) in table.iter_mut().enumerate() {
        let y = (i as f32 * a / 255.0).max(0.0).powf(e).min(256.0);
        *t = (y * 255.0) as i32;
    }
    for p in img.pixels_mut() {
        let (r, g, b) = (p[0] as i32, p[1] as i32, p[2] as i32);
        let l = (b + 2 * g + r) >> 2;
        let w = 0xff00 - l * 256;
        for i in 0..3 {
            let c = p[i] as i32;
            p[i] = (c + (((table[c as usize] - c) * w) >> 16)).clamp(0, 255) as u8;
        }
    }
}

pub fn apply_effect(mut img: RgbaImage, e: Effect) -> RgbaImage {
    let aspect = img.width() as f32 / img.height() as f32;
    let centre_dist = move |x: f32, y: f32| (((x - 0.5) * aspect).powi(2) + (y - 0.5).powi(2)).sqrt() / (0.5 * aspect.max(1.0));
    match e {
        Effect::Sharpen => {
            let blur = cheap_blur(&img, 2);
            combine(&mut img, &blur, |_, _, c, b| [0, 1, 2].map(|i| c[i] + 0.9 * (c[i] - b[i])));
        }
        Effect::Sepia => sepia(&mut img),
        // "bw" filter: desaturate fully (256)
        Effect::BlackWhite => desaturate(&mut img, 256),
        Effect::Warmify => {
            let t = warm_table();
            for p in img.pixels_mut() {
                p[0] = t[p[0] as usize][0];
                p[1] = t[p[1] as usize][1];
                p[2] = t[p[2] as usize][2];
            }
        }
        Effect::FilmGrain => film_grain(&mut img, 0.5),
        Effect::Tint => map_pixels(&mut img, |_, _, c| {
            let l = lum(c[0], c[1], c[2]);
            let t = [l * 0.82, l * 0.92, l * 1.15];
            [0, 1, 2].map(|i| c[i] * 0.35 + t[i] * 0.65)
        }),
        Effect::Saturation => saturation(&mut img, SATURATION_AMOUNT),
        Effect::SoftFocus => {
            let k = (img.width().max(img.height()) / 90).max(3);
            let blur = cheap_blur(&img, k);
            combine(&mut img, &blur, |x, y, c, b| {
                let m = smoothstep(0.25, 0.8, centre_dist(x, y));
                [0, 1, 2].map(|i| c[i] * (1.0 - m) + b[i] * m)
            });
        }
        Effect::Glow => {
            let k = (img.width().max(img.height()) / 120).max(3);
            let blur = cheap_blur(&img, k);
            combine(&mut img, &blur, |_, _, c, b| [0, 1, 2].map(|i| 1.0 - (1.0 - c[i]) * (1.0 - 0.45 * b[i])));
        }
        Effect::FilteredBw => map_pixels(&mut img, |_, _, c| {
            let l = 0.6 * c[0] + 0.32 * c[1] + 0.08 * c[2];
            [((l - 0.5) * 1.15 + 0.5); 3]
        }),
        Effect::FocalBw => map_pixels(&mut img, |x, y, c| {
            let m = smoothstep(0.3, 0.55, centre_dist(x, y));
            let l = lum(c[0], c[1], c[2]);
            c.map(|v| v * (1.0 - m) + l * m)
        }),
        Effect::GraduatedTint => map_pixels(&mut img, |_, y, c| {
            let m = (1.0 - y / 0.55).max(0.0) * 0.55;
            let tint = [1.0, 0.62, 0.32];
            [0, 1, 2].map(|i| c[i] * (1.0 - m) + c[i] * tint[i] * m)
        }),
    }
    img
}

/// Lay out pictures in a grid on a light background.
pub fn make_collage(pics: &[RgbaImage], cell: u32) -> RgbaImage {
    let n = pics.len().max(1) as u32;
    let cols = (n as f32).sqrt().ceil() as u32;
    let rows = n.div_ceil(cols);
    let gap = cell / 20;
    let mut out = RgbaImage::from_pixel(cols * (cell + gap) + gap, rows * (cell + gap) + gap, Rgba([245, 245, 240, 255]));
    for (i, p) in pics.iter().enumerate() {
        let (w, h) = fit(p.width(), p.height(), cell);
        let small = imageops::resize(p, w, h, imageops::FilterType::Triangle);
        let cx = (i as u32 % cols) * (cell + gap) + gap + (cell - w) / 2;
        let cy = (i as u32 / cols) * (cell + gap) + gap + (cell - h) / 2;
        imageops::overlay(&mut out, &small, cx as i64, cy as i64);
    }
    out
}

#[cfg(test)]
mod filter_tests {
    use super::*;

    #[test]
    fn redeye_fixes_are_not_copied() {
        let edits = [EditOp::Rotate(1), EditOp::Redeye([0.1, 0.1, 0.2, 0.2]), EditOp::AutoColor];
        let copied = copyable_edits(&edits);
        assert_eq!(copied.len(), 2);
        assert!(copied.iter().all(|op| !matches!(op, EditOp::Redeye(_))));
    }

    #[test]
    fn perspective_widens_the_top() {
        // white picture with a black vertical line at 3/4 of the width
        let (w, h) = (200u32, 100u32);
        let img = RgbaImage::from_fn(w, h, |x, _| if x == 150 { Rgba([0, 0, 0, 255]) } else { Rgba([255, 255, 255, 255]) });
        let line_at = |out: &RgbaImage, y: u32| (0..w).min_by_key(|&x| out.get_pixel(x, y)[0]).unwrap_or(0);
        assert_eq!(perspective(&img, 0.0, 0.0), img, "zero changes nothing");
        let out = perspective(&img, 0.8, 0.0);
        assert_eq!(out.dimensions(), img.dimensions());
        // the top shows a narrower part of the picture spread wider: the line moves outwards
        assert!(line_at(&out, 2) > line_at(&out, h - 3), "top {} bottom {}", line_at(&out, 2), line_at(&out, h - 3));
        // no empty (black, transparent) corners
        for (x, y) in [(0, 0), (w - 1, 0), (0, h - 1), (w - 1, h - 1)] {
            assert_eq!(out.get_pixel(x, y)[3], 255);
        }
        // out-of-range values are bounded, never a crash
        for (v, hz) in [(f32::NAN, 0.3), (5.0, -5.0), (-1.0, 1.0)] {
            assert_eq!(perspective(&img, v, hz).dimensions(), img.dimensions());
        }
        assert_eq!(perspective(&RgbaImage::new(1, 1), 1.0, 1.0).dimensions(), (1, 1));
    }

    #[test]
    fn neutral_balance_makes_picked_colour_gray() {
        let mut img = RgbaImage::from_pixel(2, 2, Rgba([200, 150, 100, 255]));
        neutral_balance(&mut img, [200.0, 150.0, 100.0]);
        let p = img.get_pixel(0, 0);
        assert!((p[0] as i32 - p[1] as i32).abs() <= 2 && (p[1] as i32 - p[2] as i32).abs() <= 2, "{p:?}");
    }

    #[test]
    fn temperature_table_starts_red_and_levels_identity() {
        assert_eq!(temp_color(0), [255.0, 56.0, 0.0]);
        let mut img = RgbaImage::from_pixel(1, 1, Rgba([10, 128, 250, 255]));
        levels(&mut img, 0.0, 1.0);
        assert_eq!(img.get_pixel(0, 0).0, [10, 128, 250, 255]);
    }

    #[test]
    fn saturation_and_bw() {
        // gray stays gray, colour gets stronger
        let mut gray = RgbaImage::from_pixel(1, 1, Rgba([120, 120, 120, 255]));
        saturation(&mut gray, 0.5);
        let g = gray.get_pixel(0, 0);
        assert!((g[0] as i32 - g[2] as i32).abs() <= 2, "{g:?}");
        let mut col = RgbaImage::from_pixel(1, 1, Rgba([180, 120, 90, 255]));
        saturation(&mut col, 0.5);
        let c = col.get_pixel(0, 0);
        assert!(c[0] as i32 - c[2] as i32 > 90, "{c:?}");
        let mut bw = RgbaImage::from_pixel(1, 1, Rgba([200, 100, 50, 255]));
        desaturate(&mut bw, 256);
        let b = bw.get_pixel(0, 0);
        assert!(b[0] == b[1] && b[1] == b[2]);
    }

    #[test]
    fn sepia_and_fill_are_stable() {
        let mut img = RgbaImage::from_pixel(1, 1, Rgba([255, 255, 255, 255]));
        sepia(&mut img);
        assert!(img.get_pixel(0, 0)[0] > 240);
        let mut dark = RgbaImage::from_pixel(1, 1, Rgba([20, 20, 20, 255]));
        fill_light(&mut dark, 0.5);
        assert!(dark.get_pixel(0, 0)[0] > 20);
    }
}
