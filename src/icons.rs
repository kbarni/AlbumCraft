//! The icons of the user interface: Material Symbols (Outlined, weight 400), Apache 2.0, from
//! `assets/fonts/`. The bundled font holds only the icons listed here. To add one, look up its
//! name at <https://fonts.google.com/icons>, add a constant below (name and codepoint from the
//! font's `.codepoints` file), then rebuild the font with `assets/fonts/subset_icons.py`.
//!
//! The icons are ordinary characters of the proportional font family (see `theme::set_fonts`),
//! so they mix with text: `format!("{EXPORT}  Export")`.

/// (constant, Material Symbols name, character): read by `subset_icons.py` and the tests.
macro_rules! icons {
    ($($id:ident = $name:literal $cp:literal;)*) => {
        $(pub const $id: &str = $cp;)*
        #[cfg(test)]
        pub const ALL: &[(&str, &str)] = &[$(($name, $cp)),*];
    };
}

icons! {
    ADD = "add" "\u{e145}";
    ALBUM = "photo_album" "\u{e411}";
    ARROW_BACK = "arrow_back" "\u{e5c4}";
    AUTO_FIX = "auto_awesome" "\u{e65f}";
    BORDER = "border_outer" "\u{e22f}";
    CHECK = "check" "\u{e668}";
    CHEVRON_LEFT = "chevron_left" "\u{e5cb}";
    CHEVRON_RIGHT = "chevron_right" "\u{e5cc}";
    CLOSE = "close" "\u{e5cd}";
    COLLAGE = "auto_awesome_mosaic" "\u{e660}";
    CONTRAST = "contrast" "\u{eb37}";
    CROP = "crop" "\u{e3be}";
    DARK_MODE = "dark_mode" "\u{e51c}";
    DESKTOP = "desktop_windows" "\u{e30c}";
    EDIT = "edit" "\u{f097}";
    EXPAND_MORE = "expand_more" "\u{e5cf}";
    EXPORT = "file_export" "\u{f3b2}";
    EYEDROPPER = "colorize" "\u{e3b8}";
    FLAG = "flag" "\u{f0c6}";
    FOLDER = "folder" "\u{e2c7}";
    FRAME = "filter_frames" "\u{e3de}";
    HOURGLASS = "hourglass_empty" "\u{e88b}";
    LIGHT_MODE = "light_mode" "\u{e518}";
    LOCATION = "location_on" "\u{f1db}";
    MAIL = "mail" "\u{e159}";
    MAP = "map" "\u{e55b}";
    MENU = "menu" "\u{e5d2}";
    MOVIE = "movie" "\u{e404}";
    MUSIC = "music_note" "\u{e405}";
    PALETTE = "palette" "\u{e40a}";
    PAUSE = "pause" "\u{e034}";
    PERSON = "person" "\u{f0d3}";
    PHOTOS = "photo_library" "\u{e413}";
    PIN = "keep" "\u{f027}";
    PLAY = "play_arrow" "\u{e037}";
    PRINT = "print" "\u{e8ad}";
    REDEYE = "remove_red_eye" "\u{e8f4}";
    REDO = "redo" "\u{e15a}";
    REFRESH = "refresh" "\u{e5d5}";
    REMOVE = "remove" "\u{e15b}";
    ROTATE_LEFT = "rotate_left" "\u{e419}";
    ROTATE_RIGHT = "rotate_right" "\u{e41a}";
    ROUNDED = "rounded_corner" "\u{e920}";
    SEARCH = "search" "\u{ef7a}";
    SHADOW = "shadow" "\u{e9df}";
    SKIP_NEXT = "skip_next" "\u{e044}";
    SKIP_PREVIOUS = "skip_previous" "\u{e045}";
    SIDE_BY_SIDE = "vertical_split" "\u{e949}";
    ABOVE_BELOW = "horizontal_split" "\u{e947}";
    STAR = "star" "\u{f09a}";
    STRAIGHTEN = "straighten" "\u{e41c}";
    SWAP = "swap_horiz" "\u{e8d4}";
    TAG = "sell" "\u{f05b}";
    TEXT = "text_fields" "\u{e262}";
    UNDO = "undo" "\u{e166}";
    VIGNETTE = "vignette" "\u{e435}";
    WEB = "public" "\u{e80b}";
    WB_SUNNY = "wb_sunny" "\u{e430}";
    INFO = "info" "\u{e88e}";
}
