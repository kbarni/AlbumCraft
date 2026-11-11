//! The Effects tab: things added on or around the picture — vignette, border / frame, drop
//! shadow, rounded corners and a line of text. Sizes are fractions of the picture's longer side,
//! so the preview and a full-size export look the same.

use image::{Rgba, RgbaImage};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Decor {
    /// Darken (amount > 0) or lighten (< 0) the corners; `size` 0…1 = how far in it reaches.
    Vignette { amount: f32, size: f32 },
    /// A plain border around the picture; `bottom` is extra width at the bottom (instant-print look).
    Border { width: f32, bottom: f32, color: [u8; 3] },
    /// A soft shadow below and to the right, on a margin of `background`.
    Shadow { size: f32, opacity: f32, background: [u8; 3] },
    /// Rounded corners filled with `background`.
    Rounded { radius: f32, background: [u8; 3] },
    /// A line of text: `anchor` 0…8 = top-left … bottom-right in reading order, `size` = letter
    /// height as a fraction of the picture's height.
    Text(TextOverlay),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct TextOverlay {
    pub text: String,
    pub anchor: u8,
    pub size: f32,
    pub color: [u8; 3],
    /// A soft dark (or light, for dark text) halo so the text reads on any picture.
    pub halo: bool,
}

impl Default for TextOverlay {
    fn default() -> Self {
        TextOverlay { text: String::new(), anchor: 7, size: 0.06, color: [255, 255, 255], halo: true }
    }
}

impl Decor {
    pub fn label(&self) -> String {
        match self {
            Decor::Vignette { amount, .. } if *amount < 0.0 => "Light vignette".into(),
            Decor::Vignette { .. } => "Vignette".into(),
            Decor::Border { bottom, .. } if *bottom > 0.0 => "Instant-print frame".into(),
            Decor::Border { .. } => "Border".into(),
            Decor::Shadow { .. } => "Drop shadow".into(),
            Decor::Rounded { .. } => "Rounded corners".into(),
            Decor::Text(t) => format!("Text \"{}\"", t.text.chars().take(20).collect::<String>()),
        }
    }

    /// Same kind of effect (the Effects tab edits the last step when it is of the same kind).
    pub fn same_kind(&self, other: &Decor) -> bool {
        std::mem::discriminant(self) == std::mem::discriminant(other)
    }

    pub fn apply(&self, img: RgbaImage) -> RgbaImage {
        let long = img.width().max(img.height()) as f32;
        let px = |f: f32| (f.clamp(0.0, 0.5) * long).round() as u32;
        match self {
            Decor::Vignette { amount, size } => {
                let mut img = img;
                vignette(&mut img, amount.clamp(-1.0, 1.0), size.clamp(0.0, 1.0));
                img
            }
            Decor::Border { width, bottom, color } => {
                let (b, extra) = (px(*width), px(*bottom));
                let mut out = canvas(img.width() + 2 * b, img.height() + 2 * b + extra, *color);
                image::imageops::overlay(&mut out, &img, i64::from(b), i64::from(b));
                out
            }
            Decor::Shadow { size, opacity, background } => shadow(&img, px(*size).max(1), opacity.clamp(0.0, 1.0), *background),
            Decor::Rounded { radius, background } => {
                let mut img = img;
                round_corners(&mut img, px(*radius) as f32, *background);
                img
            }
            Decor::Text(t) => {
                let mut img = img;
                draw_text(&mut img, t);
                img
            }
        }
    }
}

fn canvas(w: u32, h: u32, c: [u8; 3]) -> RgbaImage {
    RgbaImage::from_pixel(w.max(1), h.max(1), Rgba([c[0], c[1], c[2], 255]))
}

fn vignette(img: &mut RgbaImage, amount: f32, size: f32) {
    let (w, h) = (img.width() as usize, img.height() as usize);
    if w == 0 || h == 0 || amount == 0.0 {
        return;
    }
    let (cx, cy) = (w as f32 / 2.0, h as f32 / 2.0);
    let r = (cx * cx + cy * cy).sqrt();
    // where the falloff starts: size 1 = from the middle, 0 = only the very corners
    let start = 0.85 - 0.6 * size;
    img.as_mut().par_chunks_mut(w * 4).enumerate().for_each(|(y, row)| {
        let dy = (y as f32 + 0.5 - cy) / r;
        for (x, p) in row.as_chunks_mut::<4>().0.iter_mut().enumerate() {
            let dx = (x as f32 + 0.5 - cx) / r;
            let t = (((dx * dx + dy * dy).sqrt() - start) / (1.0 - start)).clamp(0.0, 1.0);
            let k = amount * t * t * (3.0 - 2.0 * t);
            for v in p.iter_mut().take(3) {
                let f = *v as f32;
                // darken toward black, or lighten toward white
                let out = if k >= 0.0 { f * (1.0 - k) } else { f + (255.0 - f) * -k };
                *v = out.round().clamp(0.0, 255.0) as u8;
            }
        }
    });
}

/// The picture on a margin of `bg`, with a blurred shadow offset down and right.
fn shadow(img: &RgbaImage, size: u32, opacity: f32, bg: [u8; 3]) -> RgbaImage {
    let (w, h) = (img.width(), img.height());
    let (ow, oh) = (w + 3 * size, h + 3 * size);
    // shadow mask: the picture's rectangle, shifted, then blurred
    let (mw, mh) = (ow as usize, oh as usize);
    let mut mask = vec![0f32; mw * mh];
    let off = size as usize;
    let shift = size as usize / 2 + 1;
    for y in 0..h as usize {
        let row = (y + off + shift) * mw;
        for x in 0..w as usize {
            if let Some(m) = mask.get_mut(row + x + off + shift) {
                *m = 1.0;
            }
        }
    }
    let r = (size as usize / 2).max(1);
    for _ in 0..3 {
        mask = box_blur(&mask, mw, mh, r);
    }
    let mut out = canvas(ow, oh, bg);
    out.as_mut().par_chunks_mut(mw * 4).enumerate().for_each(|(y, row)| {
        for (x, p) in row.as_chunks_mut::<4>().0.iter_mut().enumerate() {
            let a = mask[y * mw + x] * opacity;
            for v in p.iter_mut().take(3) {
                *v = (*v as f32 * (1.0 - a)).round() as u8;
            }
        }
    });
    image::imageops::overlay(&mut out, img, i64::from(size), i64::from(size));
    out
}

fn box_blur(src: &[f32], w: usize, h: usize, r: usize) -> Vec<f32> {
    let mut tmp = vec![0f32; src.len()];
    let norm = 1.0 / (2 * r + 1) as f32;
    let ri = r as isize;
    tmp.par_chunks_mut(w).zip(src.par_chunks(w)).for_each(|(out, line)| {
        let at = |i: isize| line[i.clamp(0, w as isize - 1) as usize];
        let mut sum: f32 = (-ri..=ri).map(at).sum();
        for (i, o) in out.iter_mut().enumerate() {
            *o = sum * norm;
            sum += at(i as isize + ri + 1) - at(i as isize - ri);
        }
    });
    let mut out = vec![0f32; src.len()];
    // columns: simple per-column pass (the shadow mask is small next to the picture work)
    let cols: Vec<Vec<f32>> = (0..w)
        .into_par_iter()
        .map(|x| {
            let at = |y: isize| tmp[(y.clamp(0, h as isize - 1) as usize) * w + x];
            let mut sum: f32 = (-ri..=ri).map(at).sum();
            (0..h)
                .map(|y| {
                    let v = sum * norm;
                    sum += at(y as isize + ri + 1) - at(y as isize - ri);
                    v
                })
                .collect()
        })
        .collect();
    for (x, col) in cols.iter().enumerate() {
        for (y, v) in col.iter().enumerate() {
            out[y * w + x] = *v;
        }
    }
    out
}

fn round_corners(img: &mut RgbaImage, r: f32, bg: [u8; 3]) {
    let (w, h) = (img.width() as f32, img.height() as f32);
    let r = r.min(w / 2.0).min(h / 2.0);
    if r < 1.0 {
        return;
    }
    let width = img.width() as usize;
    img.as_mut().par_chunks_mut(width * 4).enumerate().for_each(|(y, row)| {
        let py = y as f32 + 0.5;
        for (x, p) in row.as_chunks_mut::<4>().0.iter_mut().enumerate() {
            let px = x as f32 + 0.5;
            // distance outside the rounded rectangle, anti-aliased over one pixel
            let cx = px.clamp(r, w - r);
            let cy = py.clamp(r, h - r);
            let d = ((px - cx).powi(2) + (py - cy).powi(2)).sqrt() - r;
            let cover = (0.5 - d).clamp(0.0, 1.0);
            if cover < 1.0 {
                for k in 0..3 {
                    p[k] = (p[k] as f32 * cover + bg[k] as f32 * (1.0 - cover)).round() as u8;
                }
            }
        }
    });
}

fn draw_text(img: &mut RgbaImage, t: &TextOverlay) {
    use ab_glyph::{point, Font, FontRef, PxScale, ScaleFont};
    let text = t.text.trim();
    if text.is_empty() {
        return;
    }
    let Ok(font) = FontRef::try_from_slice(epaint_default_fonts::UBUNTU_LIGHT) else { return };
    let (iw, ih) = (img.width() as f32, img.height() as f32);
    let px = (t.size.clamp(0.01, 0.4) * ih).max(6.0);
    let scale = PxScale::from(px);
    let sf = font.as_scaled(scale);
    let width: f32 = text.chars().map(|c| sf.h_advance(font.glyph_id(c))).sum();
    let height = sf.ascent() - sf.descent();
    let margin = px * 0.6;
    let (x0, top) = anchored(t.anchor, (iw, ih), (width, height), margin);
    // coverage of the text, so the halo can be a blurred copy of it
    let (w, h) = (img.width() as usize, img.height() as usize);
    let mut cov = vec![0f32; w * h];
    let mut x = x0;
    let base = top + sf.ascent();
    for c in text.chars() {
        let id = font.glyph_id(c);
        if let Some(o) = font.outline_glyph(id.with_scale_and_position(scale, point(x, base))) {
            let bb = o.px_bounds();
            o.draw(|gx, gy, v| {
                let (px, py) = (bb.min.x as i64 + gx as i64, bb.min.y as i64 + gy as i64);
                if px >= 0 && py >= 0 && (px as usize) < w && (py as usize) < h {
                    let c = &mut cov[py as usize * w + px as usize];
                    *c = c.max(v);
                }
            });
        }
        x += sf.h_advance(id);
    }
    let light = t.color.iter().map(|&v| v as u32).sum::<u32>() > 382;
    let halo = if t.halo {
        let r = ((px * 0.08) as usize).max(1);
        let mut b = box_blur(&cov, w, h, r);
        b = box_blur(&b, w, h, r);
        Some(b)
    } else {
        None
    };
    let halo_c = if light { 0.0 } else { 255.0 };
    img.as_mut().par_chunks_mut(w * 4).enumerate().for_each(|(y, row)| {
        for (x, p) in row.as_chunks_mut::<4>().0.iter_mut().enumerate() {
            let i = y * w + x;
            if let Some(hb) = &halo {
                let a = (hb[i] * 1.6).min(1.0) * 0.6;
                for v in p.iter_mut().take(3) {
                    *v = (*v as f32 * (1.0 - a) + halo_c * a).round() as u8;
                }
            }
            let a = cov[i];
            if a > 0.0 {
                for (v, c) in p.iter_mut().zip(t.color) {
                    *v = (*v as f32 * (1.0 - a) + c as f32 * a).round() as u8;
                }
            }
        }
    });
}

/// Where something of `size` (w, h) goes on a picture of (iw, ih) at `anchor` (0…8 = top-left …
/// bottom-right in reading order), `margin` pixels from the edges: its top-left corner.
pub fn anchored(anchor: u8, (iw, ih): (f32, f32), (w, h): (f32, f32), margin: f32) -> (f32, f32) {
    let (col, row) = (anchor.min(8) % 3, anchor.min(8) / 3);
    let x = match col {
        0 => margin,
        1 => (iw - w) / 2.0,
        _ => iw - w - margin,
    };
    let y = match row {
        0 => margin,
        1 => (ih - h) / 2.0,
        _ => ih - h - margin,
    };
    (x, y)
}

/// Lay `logo` on `img` (a signature or watermark): its longer side `size` × the picture's
/// shorter side, at `anchor` with a margin of 3 % of the shorter side, its own transparency
/// times `opacity`.
pub fn stamp(img: &mut RgbaImage, logo: &RgbaImage, anchor: u8, size: f32, opacity: f32) {
    if logo.width() == 0 || logo.height() == 0 {
        return;
    }
    let short = img.width().min(img.height()) as f32;
    let long = (size.clamp(0.01, 1.0) * short).max(4.0);
    let s = long / logo.width().max(logo.height()) as f32;
    let (w, h) = (((logo.width() as f32 * s).round() as u32).max(1), ((logo.height() as f32 * s).round() as u32).max(1));
    let scaled = image::imageops::resize(logo, w, h, image::imageops::FilterType::Lanczos3);
    let (x, y) = anchored(anchor, (img.width() as f32, img.height() as f32), (w as f32, h as f32), short * 0.03);
    let (x0, y0) = (x.round() as i64, y.round() as i64);
    let opacity = opacity.clamp(0.0, 1.0);
    for (lx, ly, px) in scaled.enumerate_pixels() {
        let (tx, ty) = (x0 + lx as i64, y0 + ly as i64);
        if tx < 0 || ty < 0 || tx >= img.width() as i64 || ty >= img.height() as i64 {
            continue;
        }
        let a = px.0[3] as f32 / 255.0 * opacity;
        if a <= 0.0 {
            continue;
        }
        let dst = img.get_pixel_mut(tx as u32, ty as u32);
        for k in 0..3 {
            dst.0[k] = (px.0[k] as f32 * a + dst.0[k] as f32 * (1.0 - a)).round() as u8;
        }
    }
}

/// `over` laid on `under` at `opacity` (the same size; used to fade a drawn signature).
pub fn blend(under: &RgbaImage, over: &RgbaImage, opacity: f32) -> RgbaImage {
    let a = opacity.clamp(0.0, 1.0);
    let mut out = under.clone();
    out.pixels_mut().zip(over.pixels()).for_each(|(o, v)| {
        for k in 0..3 {
            o.0[k] = (v.0[k] as f32 * a + o.0[k] as f32 * (1.0 - a)).round() as u8;
        }
    });
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_logo_is_stamped_in_its_corner() {
        let mut img = RgbaImage::from_pixel(200, 100, Rgba([0, 0, 0, 255]));
        let logo = RgbaImage::from_pixel(10, 10, Rgba([255, 255, 255, 255]));
        // bottom-right, 20 % of the shorter side (20 px), margin 3 px
        stamp(&mut img, &logo, 8, 0.2, 1.0);
        assert_eq!(img.get_pixel(190, 90).0[0], 255);
        assert_eq!(img.get_pixel(198, 98).0[0], 0, "inside the margin");
        assert_eq!(img.get_pixel(170, 90).0[0], 0, "left of the logo");
        assert_eq!(img.get_pixel(10, 10).0[0], 0);
        // half opacity
        let mut faint = RgbaImage::from_pixel(200, 100, Rgba([0, 0, 0, 255]));
        stamp(&mut faint, &logo, 0, 0.2, 0.5);
        let v = faint.get_pixel(10, 10).0[0];
        assert!((120..=135).contains(&v), "{v}");
    }

    fn pic() -> RgbaImage {
        RgbaImage::from_pixel(200, 100, Rgba([120, 140, 160, 255]))
    }

    #[test]
    fn border_and_shadow_grow_the_canvas() {
        let b = Decor::Border { width: 0.05, bottom: 0.1, color: [255, 255, 255] }.apply(pic());
        assert_eq!(b.dimensions(), (220, 140));
        assert_eq!(*b.get_pixel(1, 1), Rgba([255, 255, 255, 255]));
        let s = Decor::Shadow { size: 0.04, opacity: 0.6, background: [255, 255, 255] }.apply(pic());
        assert_eq!(s.dimensions(), (224, 124));
        // the shadow side (bottom right) is darker than the far corner of the margin
        assert!(s.get_pixel(212, 112)[0] < s.get_pixel(2, 2)[0]);
    }

    #[test]
    fn vignette_darkens_corners_and_keeps_the_middle() {
        let v = Decor::Vignette { amount: 0.8, size: 0.5 }.apply(pic());
        assert!(v.get_pixel(0, 0)[0] < 60);
        assert_eq!(v.get_pixel(100, 50)[0], 120);
    }

    #[test]
    fn rounded_corners_fill_with_the_background() {
        let r = Decor::Rounded { radius: 0.1, background: [0, 0, 0] }.apply(pic());
        assert_eq!(r.get_pixel(0, 0)[0], 0);
        assert_eq!(r.get_pixel(100, 50)[0], 120);
    }

    #[test]
    fn text_is_drawn_where_anchored() {
        let t = TextOverlay { text: "Hello".into(), anchor: 8, size: 0.2, color: [255, 0, 0], halo: false };
        let img = Decor::Text(t).apply(pic());
        let red = |x0: u32, x1: u32| (x0..x1).any(|x| (0..100).any(|y| img.get_pixel(x, y)[0] > 200));
        assert!(red(120, 200), "text in the right half");
        assert!(!red(0, 80), "nothing in the left part");
    }

    #[test]
    fn empty_text_changes_nothing() {
        let img = Decor::Text(TextOverlay::default()).apply(pic());
        assert_eq!(img, pic());
    }
}
