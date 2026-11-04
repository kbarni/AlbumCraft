//! Light and dark themes: one set of colour tokens, read with `t()`, and the egui style built from them.
//!
//! Style after PhotoCraft's "Studio" themes: flat surfaces, hairline borders, rounded controls and a
//! single blue accent. Never hard-code a UI colour in a view; add a token here instead.

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
    /// The photo tray at the bottom and the dark well that holds its thumbnails.
    pub tray_bg: Color32,
    pub tray_well: Color32,
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
    pub placeholder: Color32,
    pub good: Color32,
    pub bad: Color32,
}

const LIGHT: Tokens = Tokens {
    dark: false,
    toolbar_bg: Color32::from_rgb(244, 245, 248),
    left_bg: Color32::from_rgb(246, 247, 250),
    lightbox_bg: Color32::from_rgb(255, 255, 255),
    header_bg: Color32::from_rgb(241, 243, 247),
    status_bg: Color32::from_rgb(244, 245, 248),
    tray_bg: Color32::from_rgb(236, 238, 243),
    tray_well: Color32::from_rgb(58, 62, 72),
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
    placeholder: Color32::from_rgb(226, 229, 235),
    good: Color32::from_rgb(30, 130, 60),
    bad: Color32::from_rgb(200, 50, 50),
};

const DARK: Tokens = Tokens {
    dark: true,
    toolbar_bg: Color32::from_rgb(40, 41, 46),
    left_bg: Color32::from_rgb(35, 36, 40),
    lightbox_bg: Color32::from_rgb(27, 28, 31),
    header_bg: Color32::from_rgb(38, 39, 44),
    status_bg: Color32::from_rgb(34, 35, 39),
    tray_bg: Color32::from_rgb(40, 41, 46),
    tray_well: Color32::from_rgb(22, 23, 26),
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
    placeholder: Color32::from_rgb(52, 54, 60),
    good: Color32::from_rgb(90, 200, 120),
    bad: Color32::from_rgb(240, 100, 100),
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
    pub fn label(self) -> &'static str {
        match self {
            Mode::System => "Same as system",
            Mode::Light => "Light",
            Mode::Dark => "Dark",
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
            Mode::System => "🖥",
            Mode::Light => "☀",
            Mode::Dark => "🌙",
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
