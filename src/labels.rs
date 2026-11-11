//! The names of the edit crate's and core's values (looks, effects, edit steps, white balance
//! presets, gallery styles) in the interface's language. Those crates have no window and no
//! language: their `label()` stays English (it is what logs and the MCP server show), and the
//! window shows these.

use crate::decor::Decor;
use crate::develop::Adjust;
use crate::gallery::Style;
use crate::i18n::tr;
use crate::imaging::{Effect, EditOp};
use crate::looks::Look;

pub fn look(l: &Look) -> String {
    match l {
        Look::Vivid => tr!("look-vivid"),
        Look::SoftFilm => tr!("look-soft-film"),
        Look::Polaroid => tr!("look-polaroid"),
        Look::CrossProcess => tr!("look-cross-process"),
        Look::Lomo => tr!("look-lomo"),
        Look::Matte => tr!("look-matte"),
        Look::TealOrange => tr!("look-teal-orange"),
        Look::HighKey => tr!("look-high-key"),
        Look::Cool => tr!("look-cool"),
        Look::Warm => tr!("look-warm"),
        Look::BlackWhite => tr!("look-bw"),
        Look::DramaticBw => tr!("look-dramatic-bw"),
        Look::Sepia => tr!("look-sepia"),
        Look::SoftFocus => tr!("look-soft-focus"),
        Look::Glow => tr!("look-glow"),
        Look::FilmGrain => tr!("look-film-grain"),
        Look::FocalBw => tr!("look-focal-bw"),
    }
}

pub fn look_tip(l: &Look) -> String {
    match l {
        Look::Vivid => tr!("look-vivid-tip"),
        Look::SoftFilm => tr!("look-soft-film-tip"),
        Look::Polaroid => tr!("look-polaroid-tip"),
        Look::CrossProcess => tr!("look-cross-process-tip"),
        Look::Lomo => tr!("look-lomo-tip"),
        Look::Matte => tr!("look-matte-tip"),
        Look::TealOrange => tr!("look-teal-orange-tip"),
        Look::HighKey => tr!("look-high-key-tip"),
        Look::Cool => tr!("look-cool-tip"),
        Look::Warm => tr!("look-warm-tip"),
        Look::BlackWhite => tr!("look-bw-tip"),
        Look::DramaticBw => tr!("look-dramatic-bw-tip"),
        Look::Sepia => tr!("look-sepia-tip"),
        Look::SoftFocus => tr!("look-soft-focus-tip"),
        Look::Glow => tr!("look-glow-tip"),
        Look::FilmGrain => tr!("look-film-grain-tip"),
        Look::FocalBw => tr!("look-focal-bw-tip"),
    }
}

fn effect(e: &Effect) -> String {
    match e {
        Effect::Sharpen => tr!("effect-sharpen"),
        Effect::Sepia => tr!("effect-sepia"),
        Effect::BlackWhite => tr!("effect-bw"),
        Effect::Warmify => tr!("effect-warmify"),
        Effect::FilmGrain => tr!("effect-film-grain"),
        Effect::Tint => tr!("effect-tint"),
        Effect::Saturation => tr!("effect-saturation"),
        Effect::SoftFocus => tr!("effect-soft-focus"),
        Effect::Glow => tr!("effect-glow"),
        Effect::FilteredBw => tr!("effect-filtered-bw"),
        Effect::FocalBw => tr!("effect-focal-bw"),
        Effect::GraduatedTint => tr!("effect-graduated-tint"),
    }
}

/// What the Advanced adjustments changed, as `Adjust::label` lists it.
fn adjust(a: &Adjust) -> String {
    let d = Adjust::default();
    let mut parts = vec![];
    if a.temp != d.temp || a.tint != d.tint {
        parts.push(tr!("adj-wb"));
    }
    for (v, name) in [
        (a.exposure, tr!("adj-exposure")),
        (a.highlights, tr!("adj-highlights")),
        (a.shadows, tr!("adj-shadows")),
        (a.blacks, tr!("adj-blacks")),
        (a.contrast, tr!("adj-contrast")),
        (a.saturation, tr!("adj-saturation")),
        (a.vibrance, tr!("adj-vibrance")),
        (a.clarity, tr!("adj-clarity")),
        (a.sharpen, tr!("adj-sharpen")),
        (a.noise, tr!("adj-noise")),
    ] {
        if v != 0.0 {
            parts.push(name);
        }
    }
    if parts.is_empty() { tr!("op-adjust") } else { tr!("op-adjust-parts", parts = parts.join(", ")) }
}

pub fn decor(d: &Decor) -> String {
    match d {
        Decor::Vignette { amount, .. } if *amount < 0.0 => tr!("decor-vignette-light"),
        Decor::Vignette { .. } => tr!("decor-vignette"),
        Decor::Border { bottom, .. } if *bottom > 0.0 => tr!("decor-instant"),
        Decor::Border { .. } => tr!("decor-border"),
        Decor::Shadow { .. } => tr!("decor-shadow"),
        Decor::Rounded { .. } => tr!("decor-rounded"),
        Decor::Text(t) => tr!("decor-text", text = t.text.chars().take(20).collect::<String>()),
    }
}

/// An entry of a picture's list of edits (`EditOp::label` in the language of the window).
pub fn op(e: &EditOp) -> String {
    match e {
        EditOp::Rotate(n) if *n > 0 => tr!("op-rotate-right"),
        EditOp::Rotate(_) => tr!("op-rotate-left"),
        EditOp::Crop(_) => tr!("op-crop"),
        EditOp::Straighten(a) => tr!("op-straighten", angle = format!("{a:+.1}")),
        EditOp::Perspective { vertical, horizontal } => {
            let (v, h) = (format!("{:+.0}", vertical * 100.0), format!("{:+.0}", horizontal * 100.0));
            match (*vertical != 0.0, *horizontal != 0.0) {
                (true, false) => tr!("op-perspective-v", v = v),
                (false, true) => tr!("op-perspective-h", h = h),
                _ => tr!("op-perspective-both", v = v, h = h),
            }
        }
        EditOp::Redeye(_) => tr!("op-redeye"),
        EditOp::Lucky => tr!("op-lucky"),
        EditOp::AutoContrast => tr!("op-auto-contrast"),
        EditOp::AutoColor => tr!("op-auto-color"),
        EditOp::Tune { .. } => tr!("op-tuning"),
        EditOp::Effect(e) => effect(e),
        EditOp::Adjust(a) => adjust(a),
        EditOp::Look { look: l, amount } => tr!("op-look", look = look(l), amount = format!("{:.0}", amount * 100.0)),
        EditOp::Decor(d) => decor(d),
    }
}

/// A white balance preset (`develop::WB_PRESETS`, named in English there).
pub fn wb_preset(name: &str) -> String {
    match name {
        "As shot" => tr!("wb-as-shot"),
        "Daylight" => tr!("wb-daylight"),
        "Cloudy" => tr!("wb-cloudy"),
        "Shade" => tr!("wb-shade"),
        "Tungsten" => tr!("wb-tungsten"),
        "Fluorescent" => tr!("wb-fluorescent"),
        "Flash" => tr!("wb-flash"),
        "Custom" => tr!("wb-custom"),
        other => other.to_owned(),
    }
}

pub fn gallery_style(s: &Style) -> String {
    match s {
        Style::Light => tr!("gallery-style-light"),
        Style::Dark => tr!("gallery-style-dark"),
        Style::Masonry => tr!("gallery-style-masonry"),
    }
}
