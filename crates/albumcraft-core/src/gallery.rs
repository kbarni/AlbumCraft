//! A web gallery: a folder with the pictures (edits applied), thumbnails and one `index.html`
//! (inline CSS and a small lightbox script, no outside files), viewable offline or uploadable to
//! any web host.

use crate::imaging::{self, EditOp};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// The look of the page: colours, frames and type. Shown to the user as small drawn previews
/// (`Theme::look`), so every theme's main colours are plain data.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    #[default]
    Paper,
    Night,
    Museum,
    Darkroom,
    Instant,
    Ocean,
    Vintage,
    Blossom,
}

/// How the thumbnails are laid out.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Layout {
    /// Square thumbnails (cropped) in a regular grid.
    #[default]
    Grid,
    /// Whole pictures in columns.
    Masonry,
    /// Whole pictures in rows of even height.
    Rows,
}

/// A theme's colours (0xRRGGBB) and sizes (CSS pixels), for the page and its previews.
#[derive(Clone, Copy, Debug)]
pub struct Look {
    pub bg: u32,
    pub fg: u32,
    /// Captions and the picture count.
    pub weak: u32,
    /// The tile behind each thumbnail.
    pub card: u32,
    /// Focus rings and links.
    pub accent: u32,
    /// The lightbox's backdrop and its text.
    pub lightbox: u32,
    pub lightbox_fg: u32,
    /// Corner radius of the tile and of the picture in it.
    pub radius: u8,
    pub pic_radius: u8,
    /// Space between tiles.
    pub gap: u8,
    /// Width of the tile's border around the picture (0: the picture is the tile).
    pub frame: u8,
    /// The tile has a drop shadow.
    pub shadow: bool,
}

const SANS: &str = r#"system-ui,-apple-system,"Segoe UI",Roboto,sans-serif"#;
const SERIF: &str = r#"Georgia,"Iowan Old Style","Palatino Linotype","Book Antiqua",serif"#;
const HAND: &str = r#""Bradley Hand","Segoe Print","Chalkboard SE","Comic Sans MS",cursive"#;
const ROUND: &str = r#"Nunito,Quicksand,"Varela Round","Segoe UI",system-ui,sans-serif"#;

impl Theme {
    pub const ALL: [Theme; 8] = [Theme::Paper, Theme::Night, Theme::Museum, Theme::Darkroom, Theme::Instant, Theme::Ocean, Theme::Vintage, Theme::Blossom];

    /// The name used by the commands ("paper", "night"…).
    pub fn id(&self) -> &'static str {
        match self {
            Theme::Paper => "paper",
            Theme::Night => "night",
            Theme::Museum => "museum",
            Theme::Darkroom => "darkroom",
            Theme::Instant => "instant",
            Theme::Ocean => "ocean",
            Theme::Vintage => "vintage",
            Theme::Blossom => "blossom",
        }
    }

    pub fn from_id(id: &str) -> Option<Theme> {
        Theme::ALL.into_iter().find(|t| t.id() == id)
    }

    pub fn look(&self) -> Look {
        let l = Look { bg: 0xf5f5f2, fg: 0x222222, weak: 0x777777, card: 0xffffff, accent: 0x2f6fd6, lightbox: 0x101012, lightbox_fg: 0xeeeeee, radius: 8, pic_radius: 8, gap: 14, frame: 0, shadow: true };
        match self {
            Theme::Paper => l,
            Theme::Night => Look { bg: 0x111214, fg: 0xececec, weak: 0x9a9aa0, card: 0x1d1e22, accent: 0x7aa7ff, lightbox: 0x000000, ..l },
            Theme::Museum => Look { bg: 0xffffff, fg: 0x1a1a1a, weak: 0x8a8a8a, card: 0xffffff, accent: 0x1a1a1a, lightbox: 0xffffff, lightbox_fg: 0x1a1a1a, radius: 0, pic_radius: 0, gap: 40, shadow: false, ..l },
            Theme::Darkroom => Look { bg: 0x000000, fg: 0xe6e6e6, weak: 0x8c8c8c, card: 0x000000, accent: 0xe0b050, lightbox: 0x000000, radius: 0, pic_radius: 0, gap: 3, shadow: false, ..l },
            Theme::Instant => Look { bg: 0xd9cbb4, fg: 0x3b2f22, weak: 0x6d5b47, card: 0xfdfcf8, accent: 0xb5532a, lightbox: 0x281e14, lightbox_fg: 0xf4ede2, radius: 2, pic_radius: 0, gap: 26, frame: 10, ..l },
            Theme::Ocean => Look { bg: 0x0f1f35, fg: 0xe6f0ff, weak: 0x8fa6c4, card: 0x182c47, accent: 0x4fd1c5, lightbox: 0x050c16, lightbox_fg: 0xe6f0ff, radius: 14, pic_radius: 14, gap: 16, ..l },
            Theme::Vintage => Look { bg: 0xf1e7d2, fg: 0x4a3626, weak: 0x8b6f55, card: 0xfbf5e8, accent: 0x9c5b2e, lightbox: 0x241a12, lightbox_fg: 0xf1e7d2, radius: 2, pic_radius: 0, gap: 22, frame: 8, ..l },
            Theme::Blossom => Look { bg: 0xfdf1f4, fg: 0x4a2c3a, weak: 0xa07a8a, card: 0xffffff, accent: 0xe0679a, lightbox: 0x3c1828, lightbox_fg: 0xfdf1f4, radius: 18, pic_radius: 12, gap: 18, frame: 6, ..l },
        }
    }

    /// The fonts of the title and of the text.
    fn fonts(&self) -> (&'static str, &'static str) {
        match self {
            Theme::Museum | Theme::Vintage => (SERIF, SERIF),
            Theme::Instant => (HAND, SANS),
            Theme::Blossom => (ROUND, ROUND),
            _ => (SANS, SANS),
        }
    }

    /// What the theme adds to the shared style sheet.
    fn css(&self) -> &'static str {
        match self {
            Theme::Museum => "header{text-align:center;padding-top:64px}h1{font-size:42px;font-weight:400;letter-spacing:.01em}.count{text-transform:uppercase;letter-spacing:.2em;font-size:12px;margin-top:10px}main{max-width:1240px;padding:32px}figcaption{text-align:center;font-style:italic;padding-top:12px}.tile a:hover img{transform:none;opacity:.9}",
            Theme::Darkroom => "header{max-width:none;padding:28px 20px 10px}h1{font-weight:300;font-size:22px;text-transform:uppercase;letter-spacing:.16em}.count{letter-spacing:.08em}main{max-width:none;padding:3px}figcaption{position:absolute;left:0;right:0;bottom:0;padding:28px 12px 10px;background:linear-gradient(transparent,rgba(0,0,0,.78));color:#eee;opacity:0;transition:opacity .25s;pointer-events:none}.tile:hover figcaption{opacity:1}",
            Theme::Instant => "header{text-align:center}h1{font-size:38px;font-weight:400}.tile{padding:10px 10px 4px;transform:rotate(var(--rot,0deg));transition:transform .3s,box-shadow .3s}.tile:nth-child(3n+1){--rot:-1.6deg}.tile:nth-child(3n+2){--rot:1.1deg}.tile:nth-child(5n){--rot:-.5deg}.tile:hover{transform:rotate(0) scale(1.03);z-index:1;box-shadow:0 10px 24px rgba(0,0,0,.3)}.tile:not(:has(figcaption)){padding-bottom:34px}.tile a:hover img{transform:none}figcaption{font:19px/1.2 var(--head);color:var(--fg);text-align:center;padding:8px 4px 6px;min-height:34px}",
            Theme::Ocean => "body{background:linear-gradient(165deg,#0b1626,#14304f) fixed}h1{color:#bfe9ff}.tile{border:1px solid rgba(143,166,196,.18)}",
            Theme::Vintage => "header{text-align:center;padding-top:48px}h1{font-style:italic;font-weight:400;font-size:38px}.count{font-style:italic}.tile{border:1px solid #d6c3a0}figcaption{text-align:center;font-style:italic;padding-top:10px}",
            Theme::Blossom => "h1{color:var(--accent);font-weight:700}.tile a:hover img{transform:scale(1.05)}figcaption{padding-left:6px}",
            _ => "",
        }
    }
}

impl Layout {
    pub const ALL: [Layout; 3] = [Layout::Grid, Layout::Masonry, Layout::Rows];

    pub fn id(&self) -> &'static str {
        match self {
            Layout::Grid => "grid",
            Layout::Masonry => "masonry",
            Layout::Rows => "rows",
        }
    }

    pub fn from_id(id: &str) -> Option<Layout> {
        Layout::ALL.into_iter().find(|l| l.id() == id)
    }
}

/// The choices of the gallery dialog, kept for next time (`Store.gallery`).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct GallerySettings {
    pub theme: Theme,
    pub layout: Layout,
    /// Longest side of the full pictures.
    pub size: u32,
    pub captions: bool,
}

impl Default for GallerySettings {
    fn default() -> Self {
        Self { theme: Theme::Paper, layout: Layout::Grid, size: 1280, captions: true }
    }
}

#[derive(Clone, Debug)]
pub struct Options {
    pub title: String,
    pub theme: Theme,
    pub layout: Layout,
    /// Longest side of the full pictures.
    pub size: u32,
    pub captions: bool,
}

impl Options {
    pub fn new(title: String, s: &GallerySettings) -> Self {
        Self { title, theme: s.theme, layout: s.layout, size: s.size, captions: s.captions }
    }
}

/// One picture: (source key, file name, edits, caption).
pub type Item = (String, String, Vec<EditOp>, String);

const THUMB: u32 = 480;

/// Make the gallery in a new folder inside `parent` (named after the title, never an existing
/// folder). Returns the folder and how many pictures could not be read. `step(done, total)` is
/// called before each picture; when it answers false the work stops, what was made is removed
/// and the error is `library::STOPPED`.
pub fn write(parent: &Path, items: &[Item], opts: &Options, step: &dyn Fn(usize, usize) -> bool) -> Result<(PathBuf, usize), String> {
    let name = folder_name(&opts.title);
    let mut dir = parent.join(&name);
    let mut n = 2;
    while dir.exists() {
        dir = parent.join(format!("{name} ({n})"));
        n += 1;
        if n > 999 {
            return Err(crate::msg!("gallery-too-many", "too many galleries with this name here").into());
        }
    }
    let err = |e: std::io::Error| format!("{}: {e}", dir.display());
    std::fs::create_dir_all(dir.join("images")).map_err(err)?;
    std::fs::create_dir_all(dir.join("thumbs")).map_err(err)?;
    let mut shown: Vec<Shown> = vec![];
    let mut failed = 0;
    for (i, (key, _name, edits, caption)) in items.iter().enumerate() {
        if !step(i, items.len()) {
            let _ = std::fs::remove_dir_all(&dir);
            return Err(crate::library::STOPPED.into());
        }
        let file = format!("{:04}.jpg", i + 1);
        let made = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| -> Option<((u32, u32), (u32, u32))> {
            let src = imaging::load_source(key, opts.size)?;
            let img = imaging::apply_all(&src, edits);
            let big = image::DynamicImage::ImageRgba8(img).to_rgb8();
            save_jpeg(&big, &dir.join("images").join(&file), 88).ok()?;
            let (tw, th) = imaging::fit(big.width(), big.height(), THUMB);
            let small = image::imageops::resize(&big, tw, th, image::imageops::FilterType::Triangle);
            save_jpeg(&small, &dir.join("thumbs").join(&file), 82).ok()?;
            Some(((tw, th), (big.width(), big.height())))
        }));
        match made {
            Ok(Some((thumb, full))) => shown.push(Shown { file, caption: if opts.captions { caption.clone() } else { String::new() }, thumb, full }),
            _ => failed += 1,
        }
    }
    if shown.is_empty() {
        return Err(crate::msg!("gallery-none-read", "none of the pictures could be read").into());
    }
    std::fs::write(dir.join("index.html"), page(opts, &shown)).map_err(err)?;
    Ok((dir, failed))
}

fn save_jpeg(img: &image::RgbImage, path: &Path, quality: u8) -> Result<(), String> {
    let file = std::fs::File::create(path).map_err(|e| e.to_string())?;
    let mut w = std::io::BufWriter::new(file);
    img.write_with_encoder(image::codecs::jpeg::JpegEncoder::new_with_quality(&mut w, quality)).map_err(|e| e.to_string())
}

/// A safe folder name from a title (letters, digits, spaces, - and _ only).
fn folder_name(title: &str) -> String {
    let clean: String = title.chars().map(|c| if c.is_alphanumeric() || c == ' ' || c == '-' || c == '_' { c } else { ' ' }).collect();
    let clean = clean.split_whitespace().collect::<Vec<_>>().join(" ");
    let clean: String = clean.chars().take(60).collect();
    if clean.is_empty() { "Gallery".into() } else { clean }
}

pub fn escape_html(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            c => out.push(c),
        }
    }
    out
}

/// One shown picture: file name, caption, thumbnail size, full size.
struct Shown {
    file: String,
    caption: String,
    thumb: (u32, u32),
    full: (u32, u32),
}

fn hex(c: u32) -> String {
    format!("#{c:06x}")
}

fn page(opts: &Options, items: &[Shown]) -> String {
    let title = escape_html(if opts.title.trim().is_empty() { "Gallery" } else { opts.title.trim() });
    let look = opts.theme.look();
    let (head, body) = opts.theme.fonts();
    let shadow = if look.shadow { "0 1px 3px rgba(0,0,0,.14),0 4px 14px rgba(0,0,0,.08)" } else { "none" };
    let vars = format!(
        ":root{{--bg:{};--fg:{};--weak:{};--card:{};--accent:{};--lb:{};--lbfg:{};--radius:{}px;--pic-radius:{}px;--gap:{}px;--frame:{}px;--shadow:{shadow};--head:{head};--body:{body}}}",
        hex(look.bg),
        hex(look.fg),
        hex(look.weak),
        hex(look.card),
        hex(look.accent),
        hex(look.lightbox),
        hex(look.lightbox_fg),
        look.radius,
        look.pic_radius,
        look.gap,
        look.frame,
    );
    let mut tiles = String::new();
    for (i, it) in items.iter().enumerate() {
        let cap = escape_html(&it.caption);
        let figcap = if cap.is_empty() { String::new() } else { format!("<figcaption>{cap}</figcaption>") };
        // the first screenful loads at once, the rest as they scroll into view
        let lazy = if i < 12 { "eager" } else { "lazy" };
        let (w, h) = it.thumb;
        // in rows, a tile grows with its picture's width for an even height
        let flex = match opts.layout {
            Layout::Rows => {
                let r = w as f32 / h.max(1) as f32;
                format!(" style=\"flex:{:.3} 1 {:.0}px\"", r, r * 230.0)
            }
            _ => String::new(),
        };
        let file = &it.file;
        tiles.push_str(&format!(
            "<figure class=\"tile\"{flex}><a href=\"images/{file}\" data-i=\"{i}\"><img src=\"thumbs/{file}\" width=\"{w}\" height=\"{h}\" loading=\"{lazy}\" alt=\"{cap}\" draggable=\"false\"></a>{figcap}</figure>\n"
        ));
    }
    // the lightbox's list, as JSON with '<' escaped so nothing can close the script
    let list: Vec<serde_json::Value> = items
        .iter()
        .map(|it| serde_json::json!({ "s": format!("images/{}", it.file), "t": format!("thumbs/{}", it.file), "c": it.caption, "w": it.full.0, "h": it.full.1 }))
        .collect();
    let data = serde_json::to_string(&list).unwrap_or_else(|_| "[]".into()).replace('<', "\\u003c");
    let tr = |s: crate::msg::Msg| escape_html(&s.to_string());
    format!(
        r#"<!doctype html>
<html lang="{lang}">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<meta name="color-scheme" content="{scheme}">
<title>{title}</title>
<style>
{vars}
{base}
{layout}
{extra}
</style>
</head>
<body class="t-{theme}">
<header><h1>{title}</h1><div class="count">{count}</div></header>
<main class="{layout_id}">
{tiles}</main>
<div id="lb" role="dialog" aria-modal="true" aria-label="{title}" hidden>
<div id="lb-bg"></div>
<div class="lb-slide"><img class="lb-img" alt="" draggable="false"></div><div class="lb-slide"><img class="lb-img" alt="" draggable="false"></div><div class="lb-slide"><img class="lb-img" alt="" draggable="false"></div>
<div id="lb-top" class="lb-ui"><span id="lb-count"></span><span><button class="lb-btn" id="lb-zoom" aria-label="{zoom}" title="{zoom}"><svg viewBox="0 0 24 24"><circle cx="10.5" cy="10.5" r="6.5"/><path d="M15.5 15.5 21 21M10.5 7.5v6M7.5 10.5h6"/></svg></button><button class="lb-btn" id="lb-close" aria-label="{close}" title="{close}"><svg viewBox="0 0 24 24"><path d="M6 6l12 12M18 6 6 18"/></svg></button></span></div>
<button class="lb-btn lb-nav lb-ui" id="lb-prev" aria-label="{prev}" title="{prev}"><svg viewBox="0 0 24 24"><path d="M15 5l-7 7 7 7"/></svg></button>
<button class="lb-btn lb-nav lb-ui" id="lb-next" aria-label="{next}" title="{next}"><svg viewBox="0 0 24 24"><path d="M9 5l7 7-7 7"/></svg></button>
<div id="lb-cap" class="lb-ui" aria-live="polite"></div>
</div>
<script type="application/json" id="lb-data">{data}</script>
<script>
{script}
</script>
</body>
</html>
"#,
        lang = tr(crate::msg!("gallery-html-lang", "en")),
        scheme = if luma(look.bg) < 0.5 { "dark" } else { "light" },
        base = BASE_CSS,
        layout = match opts.layout {
            Layout::Grid => GRID_CSS,
            Layout::Masonry => MASONRY_CSS,
            Layout::Rows => ROWS_CSS,
        },
        extra = opts.theme.css(),
        theme = opts.theme.id(),
        layout_id = opts.layout.id(),
        count = tr(crate::msg!("gallery-count", "{n} {n:picture|pictures}", n = items.len())),
        close = tr(crate::msg!("gallery-close", "Close")),
        prev = tr(crate::msg!("gallery-prev", "Previous")),
        next = tr(crate::msg!("gallery-next", "Next")),
        zoom = tr(crate::msg!("gallery-zoom", "Zoom")),
        script = LIGHTBOX_JS,
    )
}

/// Relative lightness of a colour, 0 to 1.
pub fn luma(c: u32) -> f32 {
    let (r, g, b) = ((c >> 16) & 255, (c >> 8) & 255, c & 255);
    (0.299 * r as f32 + 0.587 * g as f32 + 0.114 * b as f32) / 255.0
}

const BASE_CSS: &str = r#"*{box-sizing:border-box}
html{scrollbar-gutter:stable}
html.lb-open{overflow:hidden}
body{margin:0;background:var(--bg);color:var(--fg);font:16px/1.45 var(--body)}
header{padding:32px 16px 8px;max-width:1400px;margin:0 auto}
h1{margin:0;font:600 30px/1.2 var(--head)}
.count{color:var(--weak);font-size:14px;margin-top:4px}
main{padding:16px;max-width:1400px;margin:0 auto}
.tile{margin:0;position:relative;background:var(--card);padding:var(--frame);border-radius:var(--radius);box-shadow:var(--shadow)}
.tile a{display:block;cursor:zoom-in;border-radius:var(--pic-radius);overflow:hidden;outline-offset:3px}
.tile a:focus-visible{outline:3px solid var(--accent)}
.tile img{display:block;width:100%;height:auto;transition:transform .45s cubic-bezier(.2,.8,.2,1),opacity .3s}
.tile a:hover img{transform:scale(1.04)}
figcaption{padding:8px 4px 4px;font-size:14px;color:var(--weak);overflow-wrap:anywhere}
#lb{position:fixed;inset:0;z-index:10;overflow:hidden;touch-action:none;user-select:none;-webkit-user-select:none;color:var(--lbfg);font-family:var(--body)}
#lb[hidden]{display:none}
#lb-bg{position:absolute;inset:0;background:var(--lb);opacity:.96}
.lb-slide{position:absolute;inset:0;will-change:transform}
.lb-img{position:absolute;max-width:none;transform-origin:50% 50%;will-change:transform;cursor:zoom-in;box-shadow:0 8px 40px rgba(0,0,0,.35);-webkit-user-drag:none}
#lb.zoomed .lb-img{cursor:grab}
#lb.dragging .lb-img{cursor:grabbing}
.lb-ui{transition:opacity .25s}
#lb.zoomed #lb-cap,#lb.zoomed .lb-nav{opacity:0;pointer-events:none}
#lb-top{position:absolute;top:0;left:0;right:0;height:56px;background:linear-gradient(color-mix(in srgb,var(--lb) 60%,transparent),transparent);display:flex;align-items:center;justify-content:space-between;padding:0 4px 0 18px;font-size:14px;font-variant-numeric:tabular-nums}
#lb-top>span:last-child{display:flex}
.lb-btn{background:none;border:0;color:inherit;width:48px;height:48px;padding:0;border-radius:50%;cursor:pointer;display:grid;place-items:center;opacity:.85}
.lb-btn:hover{opacity:1;background:color-mix(in srgb,currentColor 14%,transparent)}
.lb-btn:focus-visible{outline:2px solid currentColor}
.lb-btn svg{width:26px;height:26px;fill:none;stroke:currentColor;stroke-width:2;stroke-linecap:round;stroke-linejoin:round}
.lb-nav{position:absolute;top:50%;margin-top:-28px;width:56px;height:56px;background:color-mix(in srgb,var(--lb) 50%,transparent)}
#lb-prev{left:10px}#lb-next{right:10px}
#lb.single .lb-nav{display:none}
#lb-cap{position:absolute;left:0;right:0;bottom:0;min-height:60px;padding:14px 24px 20px;text-align:center;font-size:15px;line-height:1.4}
#lb-cap:empty{display:none}
@media (hover:none){.lb-nav{display:none}}
@media (prefers-reduced-motion:reduce){*{transition:none!important}}"#;

const GRID_CSS: &str = "main{display:grid;grid-template-columns:repeat(auto-fill,minmax(220px,1fr));gap:var(--gap)}.tile img{aspect-ratio:1;object-fit:cover}@media (max-width:520px){main{grid-template-columns:repeat(2,1fr)}}";
const MASONRY_CSS: &str = "main{column-width:260px;column-gap:var(--gap)}.tile{break-inside:avoid;margin-bottom:var(--gap)}";
const ROWS_CSS: &str = r#"main{display:flex;flex-wrap:wrap;gap:var(--gap)}main::after{content:"";flex-grow:1e4}"#;

/// The lightbox: opens from the thumbnail and closes back into it, slides between pictures
/// (arrow keys, buttons, swipe or drag), zooms on a click, a pinch or the wheel, pans when
/// dragged while zoomed, closes with Esc, a click beside the picture or a swipe down.
const LIGHTBOX_JS: &str = r#"(()=>{
const D=JSON.parse(document.getElementById('lb-data').textContent),n=D.length;
const $=id=>document.getElementById(id),lb=$('lb'),bg=$('lb-bg'),cap=$('lb-cap'),cnt=$('lb-count');
const slides=[...lb.querySelectorAll('.lb-slide')],links=[...document.querySelectorAll('.tile a')];
const ui=[...lb.querySelectorAll('.lb-ui')];
const reduce=matchMedia('(prefers-reduced-motion: reduce)').matches;
const DUR=reduce?0:360,EASE='cubic-bezier(.22,.8,.24,1)',GAP=24;
const hasCap=D.some(d=>d.c);
let open=false,busy=false,v=0,s=1,tx=0,ty=0,dragX=0,W=innerWidth,H=innerHeight,g=null;const pts=new Map();
const mod=(a,b)=>((a%b)+b)%b,pic=i=>D[mod(i,n)],slot=i=>slides[mod(i,3)],img=i=>slot(i).firstChild;
// where a picture fits, left free of the top bar and the caption
function fit(d){const side=W<600?0:72,top=W<600?48:56,bot=hasCap?64:(W<600?48:56);
 const aw=W-2*side,ah=H-top-bot,k=Math.min(aw/d.w,ah/d.h,1.5),w=Math.round(d.w*k),h=Math.round(d.h*k);
 return{x:Math.round((W-w)/2),y:Math.round(top+(ah-h)/2),w,h};}
function place(sl,i){const d=pic(i),im=sl.firstChild,f=fit(d);sl.dataset.i=i;
 Object.assign(im.style,{left:f.x+'px',top:f.y+'px',width:f.w+'px',height:f.h+'px',transition:'none',transform:'none'});
 im.alt=d.c||'';if(im.dataset.src!==d.s){im.dataset.src=d.s;im.src=d.t;const big=new Image();big.src=d.s;
  big.decode().then(()=>{if(im.dataset.src===d.s)im.src=d.s;}).catch(()=>{});}}
// the three slides (previous, current, next) side by side; dragX follows a swipe
function layout(anim){for(let k=-1;k<=1;k++){const i=v+k,sl=slot(i);
  if(n<2&&k){sl.style.visibility='hidden';continue;}sl.style.visibility='';
  const fresh=sl.dataset.i!==String(i);if(fresh)place(sl,i);
  sl.style.transition=anim&&!fresh?`transform ${DUR}ms ${EASE}`:'none';
  sl.style.transform=`translate3d(${k*(W+GAP)+dragX}px,0,0)`;}}
function info(){const d=pic(v);cap.textContent=d.c||'';cnt.textContent=n>1?`${mod(v,n)+1} / ${n}`:'';}
function clamp1(t,size,c,view){return size<=view?0:Math.min(size/2-c,Math.max(view-c-size/2,t));}
function zmax(){const f=fit(pic(v));return Math.max(4,2*pic(v).w/f.w);}
function setZ(ns,x,y,anim,free){const f=fit(pic(v)),im=img(v);s=ns;
 if(free){tx=x;ty=y;}else{tx=clamp1(x,f.w*s,f.x+f.w/2,W);ty=clamp1(y,f.h*s,f.y+f.h/2,H);}
 im.style.transition=anim?`transform ${DUR}ms ${EASE}`:'none';
 im.style.transform=`translate3d(${tx}px,${ty}px,0) scale(${s})`;lb.classList.toggle('zoomed',s>1.01);}
// zoom to ns keeping the screen point (px,py) where it is
function zoomAt(ns,px,py,anim){const f=fit(pic(v));ns=Math.min(zmax(),Math.max(1,ns));
 const qx=px-(f.x+f.w/2),qy=py-(f.y+f.h/2);setZ(ns,qx-ns*(qx-tx)/s,qy-ns*(qy-ty)/s,anim);}
function toggleZoom(px,py){if(s>1.01)setZ(1,0,0,true);else{const f=fit(pic(v));zoomAt(Math.max(2,pic(v).w/f.w),px,py,true);}}
function go(d){if(n<2||busy)return;if(s>1.01||tx||ty)setZ(1,0,0,true);v+=d;dragX=0;s=1;tx=ty=0;layout(true);info();}
// the keyframes from a thumbnail's box to the picture's place: scaled to cover it, clipped to it
function from(r,d,radius){const f=fit(d),k=Math.max(r.width/f.w,r.height/f.h);
 const dx=r.left+r.width/2-(f.x+f.w/2),dy=r.top+r.height/2-(f.y+f.h/2);
 const ix=Math.max(0,(f.w-r.width/k)/2),iy=Math.max(0,(f.h-r.height/k)/2);
 return{transform:`translate3d(${dx}px,${dy}px,0) scale(${k})`,clipPath:`inset(${iy}px ${ix}px round ${radius/k}px)`};}
function thumbBox(i){const a=links[i];if(!a)return null;const r=a.getBoundingClientRect();
 return r.width&&r.bottom>0&&r.top<innerHeight?{r,radius:parseFloat(getComputedStyle(a).borderTopLeftRadius)||0,a}:null;}
// keep: hold the end (closing); otherwise the style sheet takes over when it ends
function fade(el,a,b,dur,keep){return el.animate([{opacity:a},{opacity:b}],{duration:dur,easing:EASE,fill:keep?'forwards':'none'});}
function show(i){if(open||busy)return;open=true;busy=true;W=innerWidth;H=innerHeight;v=i;s=1;tx=ty=0;dragX=0;
 slides.forEach(sl=>{sl.dataset.i='';sl.firstChild.dataset.src='';});
 lb.classList.toggle('single',n<2);lb.classList.remove('zoomed');lb.hidden=false;document.documentElement.classList.add('lb-open');
 layout(false);info();const im=img(v),t=thumbBox(i),done=()=>{busy=false;if(t)t.a.style.visibility='';};
 fade(bg,0,.96,DUR);ui.forEach(u=>fade(u,0,1,DUR));
 if(t&&DUR){t.a.style.visibility='hidden';im.animate([from(t.r,pic(v),t.radius),{transform:'translate3d(0,0,0) scale(1)',clipPath:'inset(0px 0px round 0px)'}],{duration:DUR,easing:EASE}).finished.then(done,done);}
 else{fade(im,0,1,DUR).finished.then(done,done);}
 $('lb-close').focus({preventScroll:true});}
function hide(){if(!open||busy)return;busy=true;const i=mod(v,n),im=img(v);
 if(links[i])links[i].scrollIntoView({block:'nearest'});const t=thumbBox(i);
 const end=()=>{lb.hidden=true;open=false;busy=false;document.documentElement.classList.remove('lb-open');
  lb.getAnimations({subtree:true}).forEach(a=>a.cancel());slides.forEach(sl=>{sl.firstChild.style.transform='none';});bg.style.opacity='';
  s=1;tx=ty=0;lb.classList.remove('zoomed','dragging');if(t)t.a.style.visibility='';if(links[i])links[i].focus({preventScroll:true});};
 fade(bg,getComputedStyle(bg).opacity,0,DUR,1);ui.forEach(u=>fade(u,getComputedStyle(u).opacity,0,DUR*.6,1));
 if(t&&DUR){t.a.style.visibility='hidden';
  im.animate([{transform:getComputedStyle(im).transform,clipPath:'inset(0px 0px round 0px)'},from(t.r,pic(v),t.radius)],{duration:DUR,easing:EASE,fill:'forwards'}).finished.then(end,end);}
 else fade(im,1,0,DUR,1).finished.then(end,end);}
links.forEach(a=>a.addEventListener('click',e=>{e.preventDefault();show(+a.dataset.i);}));
$('lb-close').onclick=hide;$('lb-prev').onclick=()=>go(-1);$('lb-next').onclick=()=>go(1);
$('lb-zoom').onclick=()=>{const f=fit(pic(v));toggleZoom(f.x+f.w/2,f.y+f.h/2);};
document.addEventListener('keydown',e=>{if(!open||e.altKey||e.ctrlKey||e.metaKey)return;const f=fit(pic(v)),cx=f.x+f.w/2,cy=f.y+f.h/2;
 const k={Escape:hide,ArrowLeft:()=>go(-1),ArrowRight:()=>go(1),Home:()=>go(-mod(v,n)),End:()=>go(n-1-mod(v,n)),
  '+':()=>zoomAt(s*1.5,cx,cy,true),'=':()=>zoomAt(s*1.5,cx,cy,true),'-':()=>zoomAt(s/1.5,cx,cy,true),'0':()=>setZ(1,0,0,true)}[e.key];
 if(k){e.preventDefault();k();}});
addEventListener('resize',()=>{if(!open)return;W=innerWidth;H=innerHeight;slides.forEach(sl=>sl.dataset.i='');s=1;tx=ty=0;lb.classList.remove('zoomed');layout(false);});
lb.addEventListener('wheel',e=>{if(!open||busy)return;e.preventDefault();const k=Math.exp(-e.deltaY*(e.deltaMode?.05:.0022));zoomAt(s*k,e.clientX,e.clientY,false);},{passive:false});
// pointers: a click or tap zooms the picture (or closes, beside it); a drag swipes, pans when
// zoomed or, downwards, closes; two fingers pinch
const dist=(a,b)=>Math.hypot(a.x-b.x,a.y-b.y),mid=(a,b)=>({x:(a.x+b.x)/2,y:(a.y+b.y)/2});
lb.addEventListener('pointerdown',e=>{if(!open||busy||e.target.closest('button')||(e.pointerType==='mouse'&&e.button))return;
 lb.setPointerCapture(e.pointerId);pts.set(e.pointerId,{x:e.clientX,y:e.clientY});const now=performance.now();
 if(pts.size===1)g={mode:'',x0:e.clientX,y0:e.clientY,tx0:tx,ty0:ty,onImg:e.target.classList.contains('lb-img'),lx:e.clientX,ly:e.clientY,lt:now,vx:0,vy:0};
 else if(pts.size===2){const[a,b]=[...pts.values()],f=fit(pic(v)),m=mid(a,b);dragX=0;layout(true);
  g={mode:'pinch',d0:dist(a,b)||1,s0:s,lx0:(m.x-(f.x+f.w/2)-tx)/s,ly0:(m.y-(f.y+f.h/2)-ty)/s};lb.classList.add('dragging');}});
lb.addEventListener('pointermove',e=>{if(!pts.has(e.pointerId)||!g)return;pts.set(e.pointerId,{x:e.clientX,y:e.clientY});
 if(g.mode==='pinch'){if(pts.size<2)return;const[a,b]=[...pts.values()],f=fit(pic(v)),m=mid(a,b);
  const ns=Math.min(zmax(),Math.max(.75,g.s0*dist(a,b)/g.d0));setZ(ns,m.x-(f.x+f.w/2)-ns*g.lx0,m.y-(f.y+f.h/2)-ns*g.ly0,false,true);return;}
 const dx=e.clientX-g.x0,dy=e.clientY-g.y0,now=performance.now(),dt=Math.max(1,now-g.lt);
 g.vx=.6*g.vx+.4*(e.clientX-g.lx)/dt;g.vy=.6*g.vy+.4*(e.clientY-g.ly)/dt;g.lx=e.clientX;g.ly=e.clientY;g.lt=now;
 if(!g.mode){if(Math.hypot(dx,dy)<6)return;g.mode=s>1.01?'pan':Math.abs(dx)>=Math.abs(dy)?'swipe':dy>0?'down':'swipe';lb.classList.add('dragging');}
 if(g.mode==='pan')setZ(s,g.tx0+dx,g.ty0+dy,false);
 else if(g.mode==='swipe'){dragX=n<2?dx*.25:dx;layout(false);}
 else{const p=Math.max(0,dy);img(v).style.transition='none';img(v).style.transform=`translate3d(${dx*.5}px,${p}px,0) scale(${1-Math.min(p/H,.25)})`;
  bg.getAnimations().forEach(a=>a.cancel());bg.style.opacity=.96*(1-Math.min(p/(H*.6),.8));}});
function up(e){if(!pts.delete(e.pointerId)||!g)return;const m=g.mode;
 if(m==='pinch'){if(pts.size)return;g=null;lb.classList.remove('dragging');setZ(Math.max(1,s),tx,ty,true);return;}
 const G=g;g=null;lb.classList.remove('dragging');
 if(!m){if(e.type==='pointercancel')return;if(G.onImg)toggleZoom(e.clientX,e.clientY);else if(s<=1.01)hide();else setZ(1,0,0,true);return;}
 if(m==='swipe'){const dx=dragX,fling=Math.abs(G.vx)>.45;
  if(n>1&&(dx<-W*.2||(fling&&G.vx<0&&dx<-20)))go(1);else if(n>1&&(dx>W*.2||(fling&&G.vx>0&&dx>20)))go(-1);else{dragX=0;layout(true);}}
 else if(m==='down'){const dy=e.clientY-G.y0;
  if(dy>H*.18||(G.vy>.5&&dy>30))hide();else{fade(bg,getComputedStyle(bg).opacity,.96,DUR);bg.style.opacity='';setZ(1,0,0,true);}}}
lb.addEventListener('pointerup',up);lb.addEventListener('pointercancel',up);
})();"#;

#[cfg(test)]
mod tests {
    use super::*;

    /// `ALBUMCRAFT_GALLERY_OUT=/some/dir cargo test demo_gallery -- --ignored`: a sample gallery
    /// in each theme (layouts in turn), to look at in a browser.
    #[test]
    #[ignore]
    fn demo_gallery() {
        let out = PathBuf::from(std::env::var("ALBUMCRAFT_GALLERY_OUT").expect("set ALBUMCRAFT_GALLERY_OUT"));
        let items: Vec<Item> = (1..=12).map(|n| (format!("{}{n}", imaging::SAMPLE_PREFIX), format!("{n}.jpg"), vec![], format!("Picture {n}"))).collect();
        for (n, theme) in Theme::ALL.into_iter().enumerate() {
            let layout = Layout::ALL[n % Layout::ALL.len()];
            let opts = Options { title: format!("Demo {} {}", theme.id(), layout.id()), theme, layout, size: 1280, captions: true };
            let (dir, _) = write(&out, &items, &opts, &|_, _| true).unwrap();
            println!("{}", dir.join("index.html").display());
        }
    }

    #[test]
    fn makes_a_gallery_and_escapes_text() {
        let parent = std::env::temp_dir().join(format!("albumcraft-gallery-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&parent);
        std::fs::create_dir_all(&parent).unwrap();
        let sample = |n: u32| format!("{}{n}", imaging::SAMPLE_PREFIX);
        let items: Vec<Item> = vec![
            (sample(1), "a.jpg".into(), vec![], "Sea & <b>sky</b>".into()),
            (sample(2), "b.jpg".into(), vec![EditOp::Rotate(1)], "</script><script>alert(1)</script>".into()),
            ("/no/such.jpg".into(), "c.jpg".into(), vec![], String::new()),
        ];
        let opts = Options { title: "Trip / \"2009\"".into(), theme: Theme::Instant, layout: Layout::Rows, size: 300, captions: true };
        let (dir, failed) = write(&parent, &items, &opts, &|_, _| true).unwrap();
        let (dir2, _) = write(&parent, &items, &opts, &|_, _| true).unwrap();
        let html = std::fs::read_to_string(dir.join("index.html")).unwrap();
        let big = image::image_dimensions(dir.join("images/0001.jpg")).unwrap();
        let thumb = dir.join("thumbs/0002.jpg").exists();
        let _ = std::fs::remove_dir_all(&parent);
        assert_eq!(failed, 1);
        assert_ne!(dir, dir2, "a second gallery gets its own folder");
        assert!(dir.starts_with(&parent) && dir.file_name().unwrap() == "Trip 2009");
        assert_eq!(big.0.max(big.1), 300);
        assert!(thumb);
        assert!(html.contains("Sea &amp; &lt;b&gt;sky&lt;/b&gt;"));
        assert!(!html.contains("</script><script>alert"));
        assert!(html.contains("2 pictures"));
        assert!(html.contains("class=\"t-instant\"") && html.contains("<main class=\"rows\">"));
        // the lightbox knows each picture's full size
        assert!(html.contains("\"w\":300") || html.contains("\"h\":300"));
    }
}
