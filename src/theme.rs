//! Light and dark themes: one set of colour tokens, read with `t()`, and the egui style built from them.
//!
//! Style after PhotoCraft's "Studio" themes: flat surfaces, hairline borders, rounded controls and a
//! single blue accent. Never hard-code a UI colour in a view; add a token here instead.

use crate::icons;
use eframe::egui::{self, Color32, CornerRadius, Stroke};
use std::sync::atomic::{AtomicBool, Ordering};

pub struct Tokens {
    pub dark: bool,
    pub toolbar_bg: Color32,
    /// Left folder list.
    pub left_bg: Color32,
    /// The lightbox / main content area.
    pub lightbox_bg: Color32,
    /// Folder header strips, status bar.
    pub header_bg: Color32,
    pub status_bg: Color32,
    /// The selection bar at the bottom.
    pub tray_bg: Color32,
    /// Cards, inputs, popups.
    pub surface: Color32,
    pub surface_hover: Color32,
    pub border: Color32,
    pub border_strong: Color32,
    pub text: Color32,
    pub text_strong: Color32,
    pub text_weak: Color32,
    pub accent: Color32,
    pub select_bg: Color32,
    pub select_border: Color32,
    /// Light accent wash used for hovered tiles.
    pub hover_wash: Color32,
    /// Behind the picture in the editor.
    pub edit_bg: Color32,
    pub star: Color32,
    pub star_outline: Color32,
    /// Pick flag and reject mark.
    pub pick: Color32,
    pub reject: Color32,
    pub placeholder: Color32,
    pub good: Color32,
    pub bad: Color32,
    /// The built-in map: sea, land, lakes, country borders.
    pub map_water: Color32,
    pub map_land: Color32,
    pub map_border: Color32,
}

const LIGHT: Tokens = Tokens {
    dark: false,
    toolbar_bg: Color32::from_rgb(244, 245, 248),
    left_bg: Color32::from_rgb(246, 247, 250),
    lightbox_bg: Color32::from_rgb(255, 255, 255),
    header_bg: Color32::from_rgb(241, 243, 247),
    status_bg: Color32::from_rgb(244, 245, 248),
    tray_bg: Color32::from_rgb(236, 238, 243),
    surface: Color32::from_rgb(255, 255, 255),
    surface_hover: Color32::from_rgb(236, 240, 248),
    border: Color32::from_rgb(218, 222, 230),
    border_strong: Color32::from_rgb(190, 196, 208),
    text: Color32::from_rgb(38, 42, 52),
    text_strong: Color32::from_rgb(20, 24, 34),
    text_weak: Color32::from_rgb(110, 117, 132),
    accent: Color32::from_rgb(47, 123, 245),
    select_bg: Color32::from_rgb(214, 228, 252),
    select_border: Color32::from_rgb(47, 123, 245),
    hover_wash: Color32::from_rgb(236, 242, 253),
    edit_bg: Color32::from_rgb(226, 229, 235),
    star: Color32::from_rgb(250, 190, 20),
    star_outline: Color32::from_rgb(170, 120, 0),
    pick: Color32::from_rgb(46, 160, 90),
    reject: Color32::from_rgb(214, 64, 64),
    placeholder: Color32::from_rgb(226, 229, 235),
    good: Color32::from_rgb(30, 130, 60),
    bad: Color32::from_rgb(200, 50, 50),
    map_water: Color32::from_rgb(196, 218, 236),
    map_land: Color32::from_rgb(244, 241, 232),
    map_border: Color32::from_rgb(178, 168, 190),
};

const DARK: Tokens = Tokens {
    dark: true,
    toolbar_bg: Color32::from_rgb(40, 41, 46),
    left_bg: Color32::from_rgb(35, 36, 40),
    lightbox_bg: Color32::from_rgb(27, 28, 31),
    header_bg: Color32::from_rgb(38, 39, 44),
    status_bg: Color32::from_rgb(34, 35, 39),
    tray_bg: Color32::from_rgb(40, 41, 46),
    surface: Color32::from_rgb(46, 47, 53),
    surface_hover: Color32::from_rgb(58, 60, 68),
    border: Color32::from_rgb(58, 60, 68),
    border_strong: Color32::from_rgb(82, 85, 96),
    text: Color32::from_rgb(220, 223, 230),
    text_strong: Color32::from_rgb(245, 246, 250),
    text_weak: Color32::from_rgb(145, 150, 165),
    accent: Color32::from_rgb(76, 148, 255),
    select_bg: Color32::from_rgb(36, 62, 104),
    select_border: Color32::from_rgb(76, 148, 255),
    hover_wash: Color32::from_rgb(50, 56, 70),
    edit_bg: Color32::from_rgb(20, 21, 24),
    star: Color32::from_rgb(250, 190, 20),
    star_outline: Color32::from_rgb(230, 170, 20),
    pick: Color32::from_rgb(80, 200, 120),
    reject: Color32::from_rgb(240, 90, 90),
    placeholder: Color32::from_rgb(52, 54, 60),
    good: Color32::from_rgb(90, 200, 120),
    bad: Color32::from_rgb(240, 100, 100),
    map_water: Color32::from_rgb(28, 38, 52),
    map_land: Color32::from_rgb(58, 60, 64),
    map_border: Color32::from_rgb(98, 96, 110),
};

static IS_DARK: AtomicBool = AtomicBool::new(false);

/// The tokens of the theme in use.
pub fn t() -> &'static Tokens {
    if IS_DARK.load(Ordering::Relaxed) { &DARK } else { &LIGHT }
}

/// How the person chose the appearance (saved in the library file).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mode {
    System,
    Light,
    Dark,
}

impl Mode {
    pub fn from_name(s: &str) -> Mode {
        match s {
            "light" => Mode::Light,
            "dark" => Mode::Dark,
            _ => Mode::System,
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Mode::System => "system",
            Mode::Light => "light",
            Mode::Dark => "dark",
        }
    }
    pub fn label(self) -> String {
        match self {
            Mode::System => crate::i18n::tr!("theme-system"),
            Mode::Light => crate::i18n::tr!("theme-light"),
            Mode::Dark => crate::i18n::tr!("theme-dark"),
        }
    }
    pub fn next(self) -> Mode {
        match self {
            Mode::System => Mode::Light,
            Mode::Light => Mode::Dark,
            Mode::Dark => Mode::System,
        }
    }
    pub fn icon(self) -> &'static str {
        match self {
            Mode::System => icons::DESKTOP,
            Mode::Light => icons::LIGHT_MODE,
            Mode::Dark => icons::DARK_MODE,
        }
    }
}

/// Resolves the mode against the system theme, and rebuilds the egui style when the result changed.
pub fn apply(ctx: &egui::Context, mode: Mode, force: bool) {
    let pref = match mode {
        Mode::System => egui::ThemePreference::System,
        Mode::Light => egui::ThemePreference::Light,
        Mode::Dark => egui::ThemePreference::Dark,
    };
    if ctx.options(|o| o.theme_preference) != pref {
        ctx.set_theme(pref);
    }
    let dark = ctx.theme() == egui::Theme::Dark;
    if force || dark != IS_DARK.load(Ordering::Relaxed) {
        IS_DARK.store(dark, Ordering::Relaxed);
        setup_style(ctx);
    }
}

fn setup_style(ctx: &egui::Context) {
    let k = t();
    let mut v = if k.dark { egui::Visuals::dark() } else { egui::Visuals::light() };
    let r = CornerRadius::same(6);
    v.panel_fill = k.lightbox_bg;
    v.window_fill = k.surface;
    v.window_stroke = Stroke::new(1.0, k.border);
    v.window_corner_radius = CornerRadius::same(10);
    v.menu_corner_radius = CornerRadius::same(8);
    v.extreme_bg_color = if k.dark { Color32::from_rgb(24, 25, 28) } else { Color32::WHITE };
    v.faint_bg_color = k.header_bg;
    v.code_bg_color = k.header_bg;
    v.hyperlink_color = k.accent;
    v.override_text_color = None;
    v.selection.bg_fill = k.select_bg;
    v.selection.stroke = Stroke::new(1.0, k.select_border);

    v.widgets.noninteractive.bg_fill = k.surface;
    v.widgets.noninteractive.weak_bg_fill = k.surface;
    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, k.border);
    v.widgets.noninteractive.fg_stroke = Stroke::new(1.0, k.text);
    v.widgets.noninteractive.corner_radius = r;

    v.widgets.inactive.weak_bg_fill = k.surface;
    v.widgets.inactive.bg_fill = if k.dark { Color32::from_rgb(70, 73, 84) } else { Color32::from_rgb(200, 205, 216) };
    v.widgets.inactive.bg_stroke = Stroke::new(1.0, k.border);
    v.widgets.inactive.fg_stroke = Stroke::new(1.0, k.text);
    v.widgets.inactive.corner_radius = r;

    v.widgets.hovered.weak_bg_fill = k.surface_hover;
    v.widgets.hovered.bg_fill = k.surface_hover;
    v.widgets.hovered.bg_stroke = Stroke::new(1.0, k.accent);
    v.widgets.hovered.fg_stroke = Stroke::new(1.5, k.text_strong);
    v.widgets.hovered.corner_radius = r;

    v.widgets.active.weak_bg_fill = k.select_bg;
    v.widgets.active.bg_fill = k.select_bg;
    v.widgets.active.bg_stroke = Stroke::new(1.0, k.accent);
    v.widgets.active.fg_stroke = Stroke::new(1.5, k.text_strong);
    v.widgets.active.corner_radius = r;

    v.widgets.open.weak_bg_fill = k.surface_hover;
    v.widgets.open.bg_fill = k.surface_hover;
    v.widgets.open.bg_stroke = Stroke::new(1.0, k.border_strong);
    v.widgets.open.fg_stroke = Stroke::new(1.0, k.text_strong);
    v.widgets.open.corner_radius = r;

    v.popup_shadow = egui::Shadow {
        offset: [0, 4],
        blur: 16,
        spread: 0,
        color: Color32::from_black_alpha(if k.dark { 120 } else { 40 }),
    };

    ctx.set_visuals(v);
    ctx.all_styles_mut(|s| {
        s.spacing.item_spacing = egui::vec2(6.0, 5.0);
        s.spacing.button_padding = egui::vec2(10.0, 4.0);
        s.text_styles.insert(egui::TextStyle::Body, egui::FontId::proportional(13.5));
        s.text_styles.insert(egui::TextStyle::Button, egui::FontId::proportional(13.5));
    });
}

/// The Material Symbols subset behind `crate::icons` (see `assets/fonts/`).
pub const ICON_FONT: &[u8] = include_bytes!("../assets/fonts/MaterialSymbolsOutlined-subset.ttf");

/// egui's fonts, with the bundled monospace font (Hack) as a fallback for ordinary text too (it
/// has symbols such as › − × that the main font lacks), and the icon font (`crate::icons`) last.
pub fn set_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    let icons = egui::FontData::from_static(ICON_FONT).tweak(egui::FontTweak {
        scale: 1.15,
        y_offset_factor: 0.12,
        ..Default::default()
    });
    fonts.font_data.insert("icons".to_owned(), std::sync::Arc::new(icons));
    // a language with a script the built-in fonts lack (Chinese, Japanese, Korean) uses a font of
    // the system, after the others, so that Latin text keeps its look
    let script_font = crate::i18n::first_existing(&crate::i18n::script_font_candidates(&crate::i18n::current_language()))
        .and_then(|(path, index)| std::fs::read(path).ok().map(|bytes| (bytes, index)));
    if let Some((bytes, index)) = script_font {
        let mut data = egui::FontData::from_owned(bytes);
        data.index = index;
        fonts.font_data.insert("script".to_owned(), std::sync::Arc::new(data));
    }
    let has_script_font = fonts.font_data.contains_key("script");
    if let Some(family) = fonts.families.get_mut(&egui::FontFamily::Proportional) {
        if !family.iter().any(|f| f == "Hack") {
            family.push("Hack".to_owned());
        }
        if has_script_font {
            family.push("script".to_owned());
        }
        family.push("icons".to_owned());
    }
    if let Some(family) = fonts.families.get_mut(&egui::FontFamily::Monospace) {
        family.push("icons".to_owned());
    }
    ctx.set_fonts(fonts);
}

#[cfg(test)]
mod glyph_tests {
    use ab_glyph::{Font, FontRef};
    use unicode_properties::UnicodeEmoji;

    /// Every character in the user interface's text must be in the text fonts egui ships with or
    /// in the icon font (see `set_fonts`). Anything else shows as an empty box on some systems,
    /// as ☰ and ＋ did on Windows. Emoji are not used either: they come from each system's own
    /// colour emoji font and look different everywhere; icons come from `crate::icons`. Scans
    /// the string literals of the sources, up to each file's tests.
    #[test]
    fn every_ui_character_is_in_the_fonts() {
        let text_fonts: Vec<FontRef> = [epaint_default_fonts::UBUNTU_LIGHT, epaint_default_fonts::HACK_REGULAR]
            .iter()
            .filter_map(|b| FontRef::try_from_slice(b).ok())
            .collect();
        assert_eq!(text_fonts.len(), 2);
        let icons = FontRef::try_from_slice(super::ICON_FONT).unwrap();
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut missing: Vec<(String, char)> = vec![];
        for entry in std::fs::read_dir(&dir).unwrap().flatten() {
            let path = entry.path();
            if path.extension().is_none_or(|e| e != "rs") {
                continue;
            }
            let text = std::fs::read_to_string(&path).unwrap();
            let code = text.split("#[cfg(test)]").next().unwrap_or("");
            let name = path.file_name().unwrap().to_string_lossy().to_string();
            for line in code.lines() {
                let line = line.trim_start();
                if line.starts_with("//") {
                    continue;
                }
                // characters inside "…" on this line
                let mut inside = false;
                let mut prev = ' ';
                for c in line.chars() {
                    if c == '"' && prev != '\\' {
                        inside = !inside;
                    } else if inside && !c.is_ascii() && !c.is_whitespace() {
                        let in_text = text_fonts.iter().any(|f| f.glyph_id(c).0 != 0);
                        // emoji are allowed only where the main font draws them itself (©)
                        let emoji_ok = !c.is_emoji_char() || text_fonts[0].glyph_id(c).0 != 0;
                        let found = (in_text && emoji_ok) || icons.glyph_id(c).0 != 0;
                        if !found && !missing.iter().any(|(_, m)| *m == c) {
                            missing.push((name.clone(), c));
                        }
                    }
                    prev = c;
                }
            }
        }
        // the translations (i18n/<language>/albumcraft.ftl): every character but in # comments
        let i18n = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("i18n");
        for entry in std::fs::read_dir(&i18n).unwrap().flatten() {
            let path = entry.path().join("albumcraft.ftl");
            // Chinese, Japanese and Korean are drawn with a font of the system, not these
            if !crate::i18n::script_font_candidates(entry.file_name().to_string_lossy().split('-').next().unwrap_or("")).is_empty() {
                continue;
            }
            let Ok(text) = std::fs::read_to_string(&path) else { continue };
            let name = format!("i18n/{}", entry.file_name().to_string_lossy());
            for line in text.lines().filter(|l| !l.trim_start().starts_with('#')) {
                for c in line.chars().filter(|c| !c.is_ascii() && !c.is_whitespace()) {
                    let emoji_ok = !c.is_emoji_char() || text_fonts[0].glyph_id(c).0 != 0;
                    let found = (text_fonts.iter().any(|f| f.glyph_id(c).0 != 0) && emoji_ok) || icons.glyph_id(c).0 != 0;
                    if !found && !missing.iter().any(|(_, m)| *m == c) {
                        missing.push((name.clone(), c));
                    }
                }
            }
        }
        let list: Vec<String> = missing.iter().map(|(f, c)| format!("{c} (U+{:04X}) in {f}", *c as u32)).collect();
        assert!(missing.is_empty(), "characters the fonts don't have, or emoji (use crate::icons):\n{}", list.join("\n"));
    }

    /// Every constant of `crate::icons` is in the bundled icon font.
    #[test]
    fn every_icon_is_in_the_icon_font() {
        let icons = FontRef::try_from_slice(super::ICON_FONT).unwrap();
        for (name, s) in crate::icons::ALL {
            let mut chars = s.chars();
            let (Some(c), None) = (chars.next(), chars.next()) else { panic!("{name} is not one character") };
            assert_ne!(icons.glyph_id(c).0, 0, "{name} (U+{:04X}) is missing: run assets/fonts/subset_icons.py", c as u32);
        }
    }
}
