//! The Advanced adjustments (white balance, exposure, tone, colour, detail) as one edit step, and
//! the picture analysis behind the one-click fixes (auto exposure, auto white balance, I'm feeling
//! lucky, auto straighten).
//!
//! Units are the ones shown on the sliders: exposure in EV (−3…+3), sharpen and noise reduction
//! 0…100, everything else −100…+100. Spatial effects (clarity, sharpen, noise reduction) scale their
//! radius with the picture, so the 1400 px preview and a full-size export look alike.

use image::RgbaImage;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Debug, Default)]
#[serde(default)]
pub struct Adjust {
    /// Blue (−) … amber (+).
    pub temp: f32,
    /// Green (−) … magenta (+).
    pub tint: f32,
    /// In EV.
    pub exposure: f32,
    pub highlights: f32,
    pub shadows: f32,
    pub blacks: f32,
    pub contrast: f32,
    pub saturation: f32,
    pub vibrance: f32,
    pub clarity: f32,
    pub sharpen: f32,
    pub noise: f32,
}

/// White-balance gains per unit of temperature (red up, blue down) and tint (green down).
const K_TEMP: f32 = 0.006;
const K_TINT: f32 = 0.003;

impl Adjust {
    pub fn is_neutral(&self) -> bool {
        *self == Adjust::default()
    }

    /// The same settings with every value in its slider range (and NaN treated as 0).
    pub fn clamped(self) -> Self {
        let c = |v: f32, lo: f32, hi: f32| if v.is_finite() { v.clamp(lo, hi) } else { 0.0 };
        Adjust {
            temp: c(self.temp, -100.0, 100.0),
            tint: c(self.tint, -100.0, 100.0),
            exposure: c(self.exposure, -3.0, 3.0),
            highlights: c(self.highlights, -100.0, 100.0),
            shadows: c(self.shadows, -100.0, 100.0),
            blacks: c(self.blacks, -100.0, 100.0),
            contrast: c(self.contrast, -100.0, 100.0),
            saturation: c(self.saturation, -100.0, 100.0),
            vibrance: c(self.vibrance, -100.0, 100.0),
            clarity: c(self.clarity, -100.0, 100.0),
            sharpen: c(self.sharpen, 0.0, 100.0),
            noise: c(self.noise, 0.0, 100.0),
        }
    }

    /// A short description for the edit list ("Adjust: exposure, contrast").
    pub fn label(&self) -> String {
        let d = Adjust::default();
        let mut parts = vec![];
        if self.temp != d.temp || self.tint != d.tint {
            parts.push("white balance");
        }
        for (v, name) in [
            (self.exposure, "exposure"),
            (self.highlights, "highlights"),
            (self.shadows, "shadows"),
            (self.blacks, "blacks"),
            (self.contrast, "contrast"),
            (self.saturation, "saturation"),
            (self.vibrance, "vibrance"),
            (self.clarity, "clarity"),
            (self.sharpen, "sharpen"),
            (self.noise, "noise reduction"),
        ] {
            if v != 0.0 {
                parts.push(name);
            }
        }
        if parts.is_empty() { "Adjust".into() } else { format!("Adjust: {}", parts.join(", ")) }
    }
}

/// White-balance starting points (temperature, tint) relative to the picture as shot.
pub const WB_PRESETS: [(&str, f32, f32); 7] = [
    ("As shot", 0.0, 0.0),
    ("Daylight", 5.0, 0.0),
    ("Cloudy", 20.0, 3.0),
    ("Shade", 35.0, 5.0),
    ("Tungsten", -60.0, 0.0),
    ("Fluorescent", -35.0, 20.0),
    ("Flash", 12.0, 0.0),
];

// ------------------------------------------------------------------ colour helpers

fn srgb_to_linear(v: f32) -> f32 {
    if v <= 0.04045 { v / 12.92 } else { ((v + 0.055) / 1.055).powf(2.4) }
}

fn linear_to_srgb(v: f32) -> f32 {
    let v = v.clamp(0.0, 1.0);
    if v <= 0.003_130_8 { v * 12.92 } else { 1.055 * v.powf(1.0 / 2.4) - 0.055 }
}

fn to_linear_lut() -> &'static [f32; 256] {
    static T: std::sync::OnceLock<[f32; 256]> = std::sync::OnceLock::new();
    T.get_or_init(|| std::array::from_fn(|i| srgb_to_linear(i as f32 / 255.0)))
}

const OUT_STEPS: usize = 4096;

fn to_srgb_lut() -> &'static [f32; OUT_STEPS + 1] {
    static T: std::sync::OnceLock<[f32; OUT_STEPS + 1]> = std::sync::OnceLock::new();
    T.get_or_init(|| std::array::from_fn(|i| linear_to_srgb(i as f32 / OUT_STEPS as f32)))
}

fn lin_to_srgb_fast(v: f32) -> f32 {
    let i = (v.clamp(0.0, 1.0) * OUT_STEPS as f32) as usize;
    to_srgb_lut()[i.min(OUT_STEPS)]
}

fn luma(r: f32, g: f32, b: f32) -> f32 {
    0.2126 * r + 0.7152 * g + 0.0722 * b
}

/// Gains (r, g, b) for a white balance, keeping brightness about the same.
fn wb_gains(temp: f32, tint: f32) -> [f32; 3] {
    let g = [(K_TEMP * temp).exp(), (-K_TINT * tint).exp(), (-K_TEMP * temp).exp()];
    let n = luma(g[0], g[1], g[2]).max(1e-6);
    [g[0] / n, g[1] / n, g[2] / n]
}

/// The tone curve (blacks, shadows, highlights, contrast) on brightness 0…1, as a table.
fn tone_lut(a: &Adjust) -> Vec<f32> {
    const N: usize = 1024;
    let lift = a.blacks / 100.0 * 0.1;
    let (s, h, c) = (a.shadows / 100.0, a.highlights / 100.0, a.contrast / 100.0 * 0.5);
    (0..=N)
        .map(|i| {
            let y = i as f32 / N as f32;
            // blacks: + lifts the black point, − deepens it
            let y = if lift >= 0.0 { lift + y * (1.0 - lift) } else { ((y + lift) / (1.0 + lift)).max(0.0) };
            let y = y + s * 0.8 * y * (1.0 - y) * (1.0 - y);
            let y = y + h * 0.8 * y * y * (1.0 - y);
            // S-curve around the middle (monotonic for |c| <= 0.5)
            let y = y + c * 2.0 * y * (1.0 - y) * (2.0 * y - 1.0);
            y.clamp(0.0, 1.0)
        })
        .collect()
}

// ------------------------------------------------------------------ applying

pub fn apply(img: &mut RgbaImage, a: &Adjust) {
    let a = a.clamped();
    if a.is_neutral() {
        return;
    }
    let (w, h) = img.dimensions();
    if w == 0 || h == 0 {
        return;
    }
    let gains = wb_gains(a.temp, a.tint);
    let ev = 2f32.powf(a.exposure);
    let tone = tone_lut(&a);
    let tone_on = a.blacks != 0.0 || a.shadows != 0.0 || a.highlights != 0.0 || a.contrast != 0.0;
    let (sat, vib) = (a.saturation / 100.0, a.vibrance / 100.0);
    let lin = to_linear_lut();
    let last = (tone.len() - 1) as f32;
    let row = w as usize * 4;
    img.as_mut().par_chunks_mut(row).for_each(|line| {
        for p in line.as_chunks_mut::<4>().0 {
            // white balance and exposure in linear light
            let mut c = [0f32; 3];
            for k in 0..3 {
                c[k] = lin_to_srgb_fast(lin[p[k] as usize] * gains[k] * ev);
            }
            if tone_on {
                let y = luma(c[0], c[1], c[2]);
                let y2 = tone[((y * last) as usize).min(tone.len() - 1)];
                if y > 0.02 {
                    let s = (y2 / y).min(4.0);
                    for v in &mut c {
                        *v *= s;
                    }
                } else {
                    for v in &mut c {
                        *v += y2 - y;
                    }
                }
            }
            if sat != 0.0 || vib != 0.0 {
                let y = luma(c[0], c[1], c[2]);
                let colourful = (c[0].max(c[1]).max(c[2]) - c[0].min(c[1]).min(c[2])).clamp(0.0, 1.0);
                // vibrance acts most on the less colourful parts
                let f = (1.0 + sat + vib * (1.0 - colourful) * (1.0 - colourful)).max(0.0);
                for v in &mut c {
                    *v = y + (*v - y) * f;
                }
            }
            for k in 0..3 {
                p[k] = (c[k].clamp(0.0, 1.0) * 255.0 + 0.5) as u8;
            }
        }
    });
    if a.clarity != 0.0 || a.sharpen != 0.0 || a.noise != 0.0 {
        detail(img, &a);
    }
}

/// Box blur of a plane (w × h), `passes` times (two or three passes come close to a Gaussian).
fn blur(plane: &[f32], w: usize, h: usize, r: usize, passes: usize) -> Vec<f32> {
    let mut a = plane.to_vec();
    if r == 0 || w == 0 || h == 0 || a.len() != w * h {
        return a;
    }
    let mut b = vec![0f32; a.len()];
    for _ in 0..passes {
        b.par_chunks_mut(w).zip(a.par_chunks(w)).for_each(|(out, src)| box_line(src, out, r));
        a = vertical_box(&b, w, h, r);
    }
    a
}

fn box_line(src: &[f32], out: &mut [f32], r: usize) {
    let n = src.len();
    if n == 0 {
        return;
    }
    let at = |i: isize| src[i.clamp(0, n as isize - 1) as usize];
    let r = r as isize;
    let mut sum: f32 = (-r..=r).map(at).sum();
    let norm = 1.0 / (2 * r + 1) as f32;
    for (i, o) in out.iter_mut().enumerate() {
        *o = sum * norm;
        let i = i as isize;
        sum += at(i + r + 1) - at(i - r);
    }
}

/// The vertical box pass: strips of columns in parallel, each keeping a running sum per column
/// while walking down the rows (no transposing, cache-friendly).
fn vertical_box(src: &[f32], w: usize, h: usize, r: usize) -> Vec<f32> {
    const STRIP: usize = 64;
    let row = |y: isize| (y.clamp(0, h as isize - 1) as usize) * w;
    let norm = 1.0 / (2 * r + 1) as f32;
    let r = r as isize;
    let strips: Vec<(usize, Vec<f32>)> = (0..w)
        .step_by(STRIP)
        .collect::<Vec<_>>()
        .into_par_iter()
        .map(|x0| {
            let sw = STRIP.min(w - x0);
            let mut out = vec![0f32; sw * h];
            let mut sum = vec![0f32; sw];
            for k in -r..=r {
                let o = row(k) + x0;
                for (s, v) in sum.iter_mut().zip(&src[o..o + sw]) {
                    *s += v;
                }
            }
            for y in 0..h {
                for (d, s) in out[y * sw..(y + 1) * sw].iter_mut().zip(&sum) {
                    *d = s * norm;
                }
                let (add, sub) = (row(y as isize + r + 1) + x0, row(y as isize - r) + x0);
                for ((s, a), b) in sum.iter_mut().zip(&src[add..add + sw]).zip(&src[sub..sub + sw]) {
                    *s += a - b;
                }
            }
            (x0, out)
        })
        .collect();
    let mut out = vec![0f32; w * h];
    for (x0, strip) in strips {
        let sw = strip.len() / h.max(1);
        for y in 0..h {
            out[y * w + x0..y * w + x0 + sw].copy_from_slice(&strip[y * sw..(y + 1) * sw]);
        }
    }
    out
}

/// Clarity (local contrast in the midtones), sharpening and noise reduction.
fn detail(img: &mut RgbaImage, a: &Adjust) {
    let (w, h) = (img.width() as usize, img.height() as usize);
    let long = w.max(h) as f32;
    let px = img.as_raw();
    let mut lum = vec![0f32; w * h];
    lum.par_chunks_mut(w).zip(px.par_chunks(w * 4)).for_each(|(l, row)| {
        for (v, p) in l.iter_mut().zip(row.chunks_exact(4)) {
            *v = luma(p[0] as f32, p[1] as f32, p[2] as f32) / 255.0;
        }
    });
    // brightness change per pixel from clarity and sharpening
    let mut delta = vec![0f32; w * h];
    if a.clarity != 0.0 {
        let r = ((long * 0.02) as usize).max(2);
        let b = blur(&lum, w, h, r, 2);
        let k = a.clarity / 100.0 * 0.8;
        delta.par_chunks_mut(w).zip(lum.par_chunks(w).zip(b.par_chunks(w))).for_each(|(d, (l, bl))| {
            for ((d, l), bl) in d.iter_mut().zip(l).zip(bl) {
                let mid = 1.0 - (2.0 * l - 1.0).powi(2);
                *d += k * (l - bl) * mid;
            }
        });
    }
    if a.sharpen != 0.0 {
        let r = ((long / 1500.0) as usize).max(1);
        let b = blur(&lum, w, h, r, 1);
        let k = a.sharpen / 100.0 * 1.5;
        delta.par_chunks_mut(w).zip(lum.par_chunks(w).zip(b.par_chunks(w))).for_each(|(d, (l, bl))| {
            for ((d, l), bl) in d.iter_mut().zip(l).zip(bl) {
                *d += k * (l - bl);
            }
        });
    }
    let mut out = px.clone();
    if a.noise > 0.0 {
        // each channel moves toward its local average where the neighbourhood is smooth
        let r = ((long / 1200.0) as usize).max(1);
        let amount = a.noise / 100.0;
        let lum_avg = blur(&lum, w, h, r, 1);
        let weight: Vec<f32> = lum.par_iter().zip(lum_avg.par_iter()).map(|(l, lb)| amount * (1.0 - ((l - lb).abs() * 12.0).min(1.0))).collect();
        for c in 0..3 {
            let plane: Vec<f32> = px.par_chunks(4).map(|p| p[c] as f32).collect();
            let avg = blur(&plane, w, h, r, 1);
            out.par_chunks_mut(w * 4).enumerate().for_each(|(y, row)| {
                for (x, p) in row.as_chunks_mut::<4>().0.iter_mut().enumerate() {
                    let i = y * w + x;
                    let v = plane[i] + (avg[i] - plane[i]) * weight[i];
                    p[c] = v.round().clamp(0.0, 255.0) as u8;
                }
            });
        }
    }
    if a.clarity != 0.0 || a.sharpen != 0.0 {
        out.par_chunks_mut(w * 4).zip(delta.par_chunks(w)).for_each(|(row, d)| {
            for (p, d) in row.as_chunks_mut::<4>().0.iter_mut().zip(d) {
                let add = d * 255.0;
                for v in p.iter_mut().take(3) {
                    *v = (*v as f32 + add).round().clamp(0.0, 255.0) as u8;
                }
            }
        });
    }
    if let Some(done) = RgbaImage::from_raw(w as u32, h as u32, out) {
        *img = done;
    }
}

// ------------------------------------------------------------------ analysis (one-click fixes)

/// Temperature and tint that make this colour (as on screen) a neutral gray.
pub fn neutralize(rgb: [u8; 3]) -> (f32, f32) {
    let lin = to_linear_lut();
    let [r, g, b] = rgb.map(|v| lin[v as usize].max(1e-4).ln());
    wb_for_logs(r, g, b)
}

fn wb_for_logs(lr: f32, lg: f32, lb: f32) -> (f32, f32) {
    let temp = (lb - lr) / (2.0 * K_TEMP);
    let tint = (lg - (lr + lb) / 2.0) / K_TINT;
    (temp.clamp(-100.0, 100.0), tint.clamp(-100.0, 100.0))
}

/// Gray-world white balance: the average of the picture's mid-tones becomes neutral.
pub fn auto_white_balance(img: &RgbaImage) -> (f32, f32) {
    let lin = to_linear_lut();
    let (mut sum, mut n) = ([0f64; 3], 0u64);
    for p in sample(img) {
        let c = [lin[p[0] as usize], lin[p[1] as usize], lin[p[2] as usize]];
        let y = luma(c[0], c[1], c[2]);
        // leave out the darkest and clipped pixels
        if (0.01..0.9).contains(&y) && p[0] < 250 && p[1] < 250 && p[2] < 250 {
            for k in 0..3 {
                sum[k] += c[k] as f64;
            }
            n += 1;
        }
    }
    if n < 50 {
        return (0.0, 0.0);
    }
    let m = sum.map(|s| ((s / n as f64) as f32).max(1e-4).ln());
    let (t, ti) = wb_for_logs(m[0], m[1], m[2]);
    // a little short of full gray-world, which over-corrects sunsets and the like
    ((t * 0.8).round(), (ti * 0.8).round())
}

/// Exposure (EV) that brings the picture's average brightness to a mid gray, without pushing
/// its brightest parts much past white.
pub fn auto_exposure(img: &RgbaImage) -> f32 {
    let lin = to_linear_lut();
    let mut ys: Vec<f32> = sample(img).map(|p| luma(lin[p[0] as usize], lin[p[1] as usize], lin[p[2] as usize])).collect();
    if ys.is_empty() {
        return 0.0;
    }
    let log_mean = ys.iter().map(|y| (y + 1e-3).ln()).sum::<f32>() / ys.len() as f32;
    let mut ev = (0.18f32.ln() - log_mean) / std::f32::consts::LN_2;
    ys.sort_by(f32::total_cmp);
    let p99 = ys[(ys.len() * 99 / 100).min(ys.len() - 1)].max(1e-3);
    if ev > 0.0 {
        ev = ev.min((1.0 / p99).log2().max(0.0));
    }
    ((ev * 0.8).clamp(-2.0, 2.0) * 10.0).round() / 10.0
}

/// "I'm feeling lucky": white balance, exposure, a touch of black point and vibrance.
pub fn lucky(img: &RgbaImage) -> Adjust {
    let (temp, tint) = auto_white_balance(img);
    let exposure = auto_exposure(img);
    let mut ys: Vec<u8> = sample(img).map(|p| luma(p[0] as f32, p[1] as f32, p[2] as f32) as u8).collect();
    ys.sort_unstable();
    let dark = ys.get(ys.len() / 200).copied().unwrap_or(0) as f32 / 255.0;
    // a washed-out black point gets deepened a little
    let blacks = -((dark - 0.02) / 0.1 * 60.0).clamp(0.0, 60.0).round();
    Adjust { temp, tint, exposure, blacks, contrast: 10.0, vibrance: 20.0, ..Default::default() }
}

/// Every pixel of a picture up to ~250,000 pixels, otherwise an even sample of about that many.
fn sample(img: &RgbaImage) -> impl Iterator<Item = &image::Rgba<u8>> {
    let n = img.width() as usize * img.height() as usize;
    let step = (n / 250_000).max(1);
    img.pixels().step_by(step)
}

/// The tilt of the picture's dominant near-horizontal or near-vertical lines, as the
/// `Straighten` angle (degrees) that levels them; `None` when no clear lines are found.
pub fn auto_straighten(img: &RgbaImage) -> Option<f32> {
    const MAX: f32 = 12.0;
    const BIN: f32 = 0.1;
    let small = {
        let (w, h) = crate::imaging::fit(img.width(), img.height(), 600);
        image::imageops::resize(img, w, h, image::imageops::FilterType::Triangle)
    };
    let (w, h) = (small.width() as usize, small.height() as usize);
    if w < 16 || h < 16 {
        return None;
    }
    let g: Vec<f32> = small.pixels().map(|p| luma(p[0] as f32, p[1] as f32, p[2] as f32)).collect();
    let at = |x: usize, y: usize| g[y * w + x];
    let nbins = (2.0 * MAX / BIN) as usize + 1;
    let mut hist = vec![0f32; nbins];
    let mut total = 0f32;
    for y in 1..h - 1 {
        for x in 1..w - 1 {
            let gx = (at(x + 1, y - 1) + 2.0 * at(x + 1, y) + at(x + 1, y + 1)) - (at(x - 1, y - 1) + 2.0 * at(x - 1, y) + at(x - 1, y + 1));
            let gy = (at(x - 1, y + 1) + 2.0 * at(x, y + 1) + at(x + 1, y + 1)) - (at(x - 1, y - 1) + 2.0 * at(x, y - 1) + at(x + 1, y - 1));
            let m = (gx * gx + gy * gy).sqrt();
            if m < 40.0 {
                continue;
            }
            // direction of the edge line, folded so horizontal and vertical lines both give ~0
            let line = gy.atan2(gx).to_degrees() + 90.0;
            let a = (line + 45.0).rem_euclid(90.0) - 45.0;
            total += m;
            if a.abs() <= MAX {
                let i = ((a + MAX) / BIN).round() as usize;
                if let Some(b) = hist.get_mut(i) {
                    *b += m;
                }
            }
        }
    }
    if total <= 0.0 {
        return None;
    }
    // smooth over ±0.5°
    let win = (0.5 / BIN) as isize;
    let smooth: Vec<f32> = (0..nbins as isize)
        .map(|i| (-win..=win).filter_map(|d| hist.get((i + d) as usize).copied()).sum())
        .collect();
    let (best, &peak) = smooth.iter().enumerate().max_by(|a, b| a.1.total_cmp(b.1))?;
    let angle = best as f32 * BIN - MAX;
    // weak or flat: no clear lines
    if peak < total * 0.04 {
        return None;
    }
    Some(-angle)
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Rgba;

    fn flat(rgb: [u8; 3]) -> RgbaImage {
        RgbaImage::from_pixel(64, 48, Rgba([rgb[0], rgb[1], rgb[2], 255]))
    }

    #[test]
    fn neutral_settings_change_nothing() {
        let mut img = crate::imaging::sample_image(4, 80, 60);
        let before = img.clone();
        apply(&mut img, &Adjust::default());
        assert_eq!(img, before);
    }

    #[test]
    fn exposure_and_temperature_move_the_right_way() {
        let mut brighter = flat([100, 100, 100]);
        apply(&mut brighter, &Adjust { exposure: 1.0, ..Default::default() });
        assert!(brighter.get_pixel(0, 0)[1] > 120);
        let mut warmer = flat([128, 128, 128]);
        apply(&mut warmer, &Adjust { temp: 50.0, ..Default::default() });
        let p = warmer.get_pixel(0, 0);
        assert!(p[0] > p[2] + 10, "{p:?}");
    }

    #[test]
    fn picking_a_colour_makes_it_gray() {
        let colour = [180, 150, 110];
        let (temp, tint) = neutralize(colour);
        let mut img = flat(colour);
        apply(&mut img, &Adjust { temp, tint, ..Default::default() });
        let p = img.get_pixel(0, 0);
        let spread = p[0].max(p[1]).max(p[2]) - p[0].min(p[1]).min(p[2]);
        assert!(spread <= 4, "{p:?}");
    }

    #[test]
    fn auto_white_balance_removes_a_cast() {
        let mut img = crate::imaging::sample_image(7, 160, 120);
        for p in img.pixels_mut() {
            p[2] = (p[2] as f32 * 0.7) as u8; // a yellow cast
        }
        let (temp, _) = auto_white_balance(&img);
        assert!(temp < -10.0, "temp {temp}");
    }

    #[test]
    fn auto_exposure_brightens_a_dark_picture() {
        let mut img = crate::imaging::sample_image(3, 160, 120);
        for p in img.pixels_mut() {
            for c in 0..3 {
                p[c] /= 4;
            }
        }
        assert!(auto_exposure(&img) > 0.5);
    }

    #[test]
    fn tone_curve_is_monotonic_at_the_extremes() {
        for v in [-100.0, 100.0] {
            let a = Adjust { blacks: v, shadows: v, highlights: v, contrast: v, ..Default::default() };
            let lut = tone_lut(&a);
            assert!(lut.windows(2).all(|w| w[1] >= w[0] - 1e-6), "not monotonic for {v}");
        }
    }

    #[test]
    fn detail_sliders_keep_the_size_and_change_pixels() {
        let base = crate::imaging::sample_image(5, 120, 90);
        for a in [Adjust { clarity: 60.0, ..Default::default() }, Adjust { sharpen: 80.0, ..Default::default() }, Adjust { noise: 80.0, ..Default::default() }] {
            let mut img = base.clone();
            apply(&mut img, &a);
            assert_eq!(img.dimensions(), base.dimensions());
            assert_ne!(img, base, "{a:?} did nothing");
        }
    }

    /// A picture with strong horizontal stripes, turned by `deg`.
    fn stripes(deg: f32) -> RgbaImage {
        let img = RgbaImage::from_fn(400, 300, |_, y| if (y / 15) % 2 == 0 { Rgba([230, 230, 230, 255]) } else { Rgba([30, 30, 30, 255]) });
        crate::imaging::apply(img, &crate::imaging::EditOp::Straighten(deg))
    }

    #[test]
    fn auto_straighten_levels_a_tilted_picture() {
        let tilted = stripes(4.0);
        let fix = auto_straighten(&tilted).expect("lines found");
        let fixed = crate::imaging::apply(tilted, &crate::imaging::EditOp::Straighten(fix));
        let left = auto_straighten(&fixed).unwrap_or(0.0);
        assert!(fix.abs() > 2.5, "fix {fix}");
        assert!(left.abs() < 0.6, "fix {fix}, left {left}");
    }

    /// `cargo test --release develop_speed -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn develop_speed() {
        for (w, h) in [(1400, 933), (6000, 4000)] {
            let base = crate::imaging::sample_image(2, w, h);
            let all = Adjust { temp: 10.0, exposure: 0.3, shadows: 20.0, contrast: 10.0, vibrance: 20.0, clarity: 30.0, sharpen: 30.0, noise: 30.0, ..Default::default() };
            let tone = Adjust { clarity: 0.0, sharpen: 0.0, noise: 0.0, ..all };
            let only = |f: fn(&mut Adjust)| {
                let mut a = Adjust::default();
                f(&mut a);
                a
            };
            for (name, a) in [
                ("tone+colour", tone),
                ("everything", all),
                ("clarity", only(|a| a.clarity = 30.0)),
                ("sharpen", only(|a| a.sharpen = 30.0)),
                ("noise", only(|a| a.noise = 30.0)),
            ] {
                let mut img = base.clone();
                let t = std::time::Instant::now();
                apply(&mut img, &a);
                println!("{w}x{h} {name}: {:?}", t.elapsed());
            }
        }
    }

    #[test]
    fn auto_straighten_finds_nothing_in_a_flat_picture() {
        assert_eq!(auto_straighten(&flat([90, 120, 150])), None);
    }
}
