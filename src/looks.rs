//! Presets: named looks (film styles, black & white, toning, soft focus…), each applied with an
//! intensity from 0 (untouched) to 1 (the full look) by blending with the picture before it.
//!
//! Most looks are combinations of the Advanced adjustments (`develop::Adjust`) with a few extra
//! steps (black & white channel mix, split toning, vignette); some reuse the one-click effects.

use crate::develop::{self, Adjust};
use crate::imaging::{self, Effect};
use image::RgbaImage;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Look {
    Vivid,
    SoftFilm,
    Polaroid,
    CrossProcess,
    Lomo,
    Matte,
    TealOrange,
    HighKey,
    Cool,
    Warm,
    BlackWhite,
    DramaticBw,
    Sepia,
    SoftFocus,
    Glow,
    FilmGrain,
    FocalBw,
}

impl Look {
    pub const ALL: [Look; 17] = [
        Look::Vivid,
        Look::SoftFilm,
        Look::Polaroid,
        Look::CrossProcess,
        Look::Lomo,
        Look::Matte,
        Look::TealOrange,
        Look::HighKey,
        Look::Cool,
        Look::Warm,
        Look::BlackWhite,
        Look::DramaticBw,
        Look::Sepia,
        Look::SoftFocus,
        Look::Glow,
        Look::FilmGrain,
        Look::FocalBw,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            Look::Vivid => "Vivid",
            Look::SoftFilm => "Soft film",
            Look::Polaroid => "Instant",
            Look::CrossProcess => "Cross process",
            Look::Lomo => "Lomo",
            Look::Matte => "Matte",
            Look::TealOrange => "Teal & orange",
            Look::HighKey => "High key",
            Look::Cool => "Cool",
            Look::Warm => "Warm",
            Look::BlackWhite => "B & W",
            Look::DramaticBw => "Dramatic B & W",
            Look::Sepia => "Sepia",
            Look::SoftFocus => "Soft focus",
            Look::Glow => "Glow",
            Look::FilmGrain => "Film grain",
            Look::FocalBw => "Focal B & W",
        }
    }

    pub fn tip(&self) -> &'static str {
        match self {
            Look::Vivid => "Rich, saturated slide-film colours with deep blacks",
            Look::SoftFilm => "Gentle portrait film: soft contrast, warm and kind to skin",
            Look::Polaroid => "Faded instant-camera print: lifted blacks, warm highlights, cool shadows",
            Look::CrossProcess => "Yellow highlights, blue-green shadows and punchy contrast",
            Look::Lomo => "Saturated toy-camera look with dark corners",
            Look::Matte => "Flat, faded blacks and muted colours",
            Look::TealOrange => "Cinema grade: teal shadows, warm skin and highlights",
            Look::HighKey => "Bright and airy, with open shadows",
            Look::Cool => "A cooler, bluer cast",
            Look::Warm => "A warmer, golden cast",
            Look::BlackWhite => "Classic black and white",
            Look::DramaticBw => "Black and white through a red filter: dark skies, strong local contrast",
            Look::Sepia => "Old-photo brown tone",
            Look::SoftFocus => "Sharp centre, dreamy soft edges",
            Look::Glow => "A soft luminous glow",
            Look::FilmGrain => "Film-like grain",
            Look::FocalBw => "Colour in the middle, black and white around it",
        }
    }

    /// The full-strength look.
    fn render(&self, img: &RgbaImage) -> RgbaImage {
        let adj = |a: Adjust| {
            let mut out = img.clone();
            develop::apply(&mut out, &a);
            out
        };
        match self {
            Look::Vivid => adj(Adjust { saturation: 30.0, vibrance: 20.0, contrast: 25.0, blacks: -10.0, clarity: 10.0, ..Default::default() }),
            Look::SoftFilm => {
                let mut out = adj(Adjust { temp: 10.0, contrast: -12.0, saturation: -12.0, highlights: -25.0, shadows: 15.0, blacks: 12.0, ..Default::default() });
                split_tone(&mut out, [0.35, 0.45, 0.5], [1.0, 0.9, 0.78], 0.12);
                out
            }
            Look::Polaroid => {
                let mut out = adj(Adjust { blacks: 35.0, contrast: -20.0, saturation: -15.0, temp: 12.0, tint: 6.0, highlights: -20.0, ..Default::default() });
                split_tone(&mut out, [0.2, 0.38, 0.45], [1.0, 0.9, 0.7], 0.25);
                vignette(&mut out, 0.25);
                out
            }
            Look::CrossProcess => {
                let mut out = adj(Adjust { contrast: 30.0, saturation: 20.0, ..Default::default() });
                split_tone(&mut out, [0.1, 0.35, 0.6], [1.0, 0.95, 0.45], 0.45);
                out
            }
            Look::Lomo => {
                let mut out = adj(Adjust { saturation: 35.0, contrast: 40.0, blacks: -15.0, ..Default::default() });
                vignette(&mut out, 0.7);
                out
            }
            Look::Matte => adj(Adjust { blacks: 40.0, contrast: -25.0, saturation: -20.0, highlights: -10.0, ..Default::default() }),
            Look::TealOrange => {
                let mut out = adj(Adjust { vibrance: 20.0, contrast: 10.0, ..Default::default() });
                split_tone(&mut out, [0.0, 0.5, 0.55], [1.0, 0.65, 0.3], 0.4);
                out
            }
            Look::HighKey => adj(Adjust { exposure: 0.6, contrast: -20.0, shadows: 40.0, highlights: -30.0, saturation: -10.0, ..Default::default() }),
            Look::Cool => adj(Adjust { temp: -25.0, tint: -3.0, ..Default::default() }),
            Look::Warm => imaging::apply_effect(img.clone(), Effect::Warmify),
            Look::BlackWhite => {
                let mut out = img.clone();
                mono(&mut out, [0.30, 0.59, 0.11]);
                develop::apply(&mut out, &Adjust { contrast: 10.0, ..Default::default() });
                out
            }
            Look::DramaticBw => {
                let mut out = img.clone();
                mono(&mut out, [0.6, 0.35, 0.05]);
                develop::apply(&mut out, &Adjust { contrast: 45.0, clarity: 50.0, blacks: -25.0, highlights: -20.0, ..Default::default() });
                out
            }
            Look::Sepia => imaging::apply_effect(img.clone(), Effect::Sepia),
            Look::SoftFocus => imaging::apply_effect(img.clone(), Effect::SoftFocus),
            Look::Glow => imaging::apply_effect(img.clone(), Effect::Glow),
            Look::FilmGrain => imaging::apply_effect(img.clone(), Effect::FilmGrain),
            Look::FocalBw => imaging::apply_effect(img.clone(), Effect::FocalBw),
        }
    }
}

/// The look at `amount` (0…1): a blend of the picture and the full look.
pub fn apply(img: &mut RgbaImage, look: Look, amount: f32) {
    let amount = if amount.is_finite() { amount.clamp(0.0, 1.0) } else { 0.0 };
    if amount <= 0.0 {
        return;
    }
    let full = look.render(img);
    if full.dimensions() != img.dimensions() {
        return;
    }
    if amount >= 1.0 {
        *img = full;
        return;
    }
    img.as_mut().par_chunks_mut(4096).zip(full.as_raw().par_chunks(4096)).for_each(|(a, b)| {
        for (x, y) in a.iter_mut().zip(b) {
            *x = (*x as f32 + (*y as f32 - *x as f32) * amount).round() as u8;
        }
    });
}

/// Black and white from a channel mix (weights summing to about 1).
fn mono(img: &mut RgbaImage, mix: [f32; 3]) {
    img.as_mut().par_chunks_mut(4096).for_each(|chunk| {
        for p in chunk.as_chunks_mut::<4>().0 {
            let g = (p[0] as f32 * mix[0] + p[1] as f32 * mix[1] + p[2] as f32 * mix[2]).round().clamp(0.0, 255.0) as u8;
            p[0] = g;
            p[1] = g;
            p[2] = g;
        }
    });
}

/// Tint the shadows toward one colour and the highlights toward another (colours 0…1).
fn split_tone(img: &mut RgbaImage, shadow: [f32; 3], highlight: [f32; 3], strength: f32) {
    img.as_mut().par_chunks_mut(4096).for_each(|chunk| {
        for p in chunk.as_chunks_mut::<4>().0 {
            let c = [p[0] as f32 / 255.0, p[1] as f32 / 255.0, p[2] as f32 / 255.0];
            let y = 0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2];
            let (ws, wh) = ((1.0 - y).powi(2) * strength, y * y * strength);
            for k in 0..3 {
                // shift each channel by how far the tone colour is from neutral
                let v = c[k] + (shadow[k] - 0.5) * ws + (highlight[k] - 0.5) * wh;
                p[k] = (v.clamp(0.0, 1.0) * 255.0).round() as u8;
            }
        }
    });
}

/// Darken (amount > 0) the corners, with a soft round falloff.
pub fn vignette(img: &mut RgbaImage, amount: f32) {
    let (w, h) = (img.width() as usize, img.height() as usize);
    if w == 0 || h == 0 {
        return;
    }
    let (cx, cy) = (w as f32 / 2.0, h as f32 / 2.0);
    let r = (cx * cx + cy * cy).sqrt();
    img.as_mut().par_chunks_mut(w * 4).enumerate().for_each(|(y, row)| {
        let dy = (y as f32 + 0.5 - cy) / r;
        for (x, p) in row.as_chunks_mut::<4>().0.iter_mut().enumerate() {
            let dx = (x as f32 + 0.5 - cx) / r;
            let d = (dx * dx + dy * dy).sqrt();
            let t = ((d - 0.45) / 0.55).clamp(0.0, 1.0);
            let f = 1.0 - amount * t * t * (3.0 - 2.0 * t);
            for v in p.iter_mut().take(3) {
                *v = (*v as f32 * f).round().clamp(0.0, 255.0) as u8;
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_look_changes_the_picture_and_keeps_its_size() {
        let base = crate::imaging::sample_image(9, 120, 80);
        for look in Look::ALL {
            let mut img = base.clone();
            apply(&mut img, look, 1.0);
            assert_eq!(img.dimensions(), base.dimensions(), "{look:?}");
            assert_ne!(img, base, "{look:?} did nothing");
        }
    }

    #[test]
    fn intensity_blends_between_untouched_and_full() {
        let base = crate::imaging::sample_image(9, 60, 40);
        let mut none = base.clone();
        apply(&mut none, Look::Lomo, 0.0);
        assert_eq!(none, base);
        let mut full = base.clone();
        apply(&mut full, Look::Lomo, 1.0);
        let mut half = base.clone();
        apply(&mut half, Look::Lomo, 0.5);
        let (a, b, c) = (base.get_pixel(30, 20)[0] as i32, half.get_pixel(30, 20)[0] as i32, full.get_pixel(30, 20)[0] as i32);
        assert!((b - (a + c) / 2).abs() <= 1, "{a} {b} {c}");
    }

    #[test]
    fn black_and_white_is_gray() {
        let mut img = crate::imaging::sample_image(9, 40, 30);
        apply(&mut img, Look::DramaticBw, 1.0);
        assert!(img.pixels().all(|p| p[0] == p[1] && p[1] == p[2]));
    }

    #[test]
    fn looks_are_saved_by_name() {
        assert_eq!(serde_json::to_string(&Look::TealOrange).unwrap(), "\"teal_orange\"");
    }
}
