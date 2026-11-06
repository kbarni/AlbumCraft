//! A web gallery: a folder with the pictures (edits applied), thumbnails and one `index.html`
//! (inline CSS and a small lightbox script, no outside files), viewable offline or uploadable to
//! any web host.

use crate::imaging::{self, EditOp};
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Style {
    Light,
    Dark,
    Masonry,
}

impl Style {
    pub const ALL: [Style; 3] = [Style::Light, Style::Dark, Style::Masonry];

    pub fn label(&self) -> &'static str {
        match self {
            Style::Light => "Light grid",
            Style::Dark => "Dark grid",
            Style::Masonry => "Masonry (whole pictures)",
        }
    }
}

#[derive(Clone, Debug)]
pub struct Options {
    pub title: String,
    pub style: Style,
    /// Longest side of the full pictures.
    pub size: u32,
    pub captions: bool,
}

/// One picture: (source key, file name, edits, caption).
pub type Item = (String, String, Vec<EditOp>, String);

const THUMB: u32 = 480;

/// Make the gallery in a new folder inside `parent` (named after the title, never an existing
/// folder). Returns the folder and how many pictures could not be read.
pub fn write(parent: &Path, items: &[Item], opts: &Options) -> Result<(PathBuf, usize), String> {
    let name = folder_name(&opts.title);
    let mut dir = parent.join(&name);
    let mut n = 2;
    while dir.exists() {
        dir = parent.join(format!("{name} ({n})"));
        n += 1;
        if n > 999 {
            return Err("too many galleries with this name here".into());
        }
    }
    let err = |e: std::io::Error| format!("{}: {e}", dir.display());
    std::fs::create_dir_all(dir.join("images")).map_err(err)?;
    std::fs::create_dir_all(dir.join("thumbs")).map_err(err)?;
    let mut shown: Vec<(String, String, u32, u32)> = vec![];
    let mut failed = 0;
    for (i, (key, _name, edits, caption)) in items.iter().enumerate() {
        let file = format!("{:04}.jpg", i + 1);
        let made = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| -> Option<(u32, u32)> {
            let src = imaging::load_source(key, opts.size)?;
            let img = imaging::apply_all(&src, edits);
            let big = image::DynamicImage::ImageRgba8(img).to_rgb8();
            save_jpeg(&big, &dir.join("images").join(&file), 88).ok()?;
            let (tw, th) = imaging::fit(big.width(), big.height(), THUMB);
            let small = image::imageops::resize(&big, tw, th, image::imageops::FilterType::Triangle);
            save_jpeg(&small, &dir.join("thumbs").join(&file), 82).ok()?;
            Some((tw, th))
        }));
        match made {
            Ok(Some((w, h))) => shown.push((file, if opts.captions { caption.clone() } else { String::new() }, w, h)),
            _ => failed += 1,
        }
    }
    if shown.is_empty() {
        return Err("none of the pictures could be read".into());
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

fn page(opts: &Options, items: &[(String, String, u32, u32)]) -> String {
    let title = escape_html(if opts.title.trim().is_empty() { "Gallery" } else { opts.title.trim() });
    let (bg, fg, card, weak) = match opts.style {
        Style::Dark => ("#111214", "#ececec", "#1d1e22", "#9a9aa0"),
        _ => ("#f5f5f2", "#222", "#ffffff", "#777"),
    };
    let layout = match opts.style {
        Style::Masonry => ".grid{column-width:260px;column-gap:14px}.tile{break-inside:avoid;margin:0 0 14px}.tile img{width:100%;height:auto;display:block}",
        _ => ".grid{display:grid;grid-template-columns:repeat(auto-fill,minmax(220px,1fr));gap:14px}.tile img{width:100%;height:auto;aspect-ratio:1;object-fit:cover;display:block}",
    };
    let mut tiles = String::new();
    for (i, (file, caption, w, h)) in items.iter().enumerate() {
        let cap = escape_html(caption);
        let figcap = if cap.is_empty() { String::new() } else { format!("<figcaption>{cap}</figcaption>") };
        // the first screenful loads at once, the rest as they scroll into view
        let lazy = if i < 12 { "eager" } else { "lazy" };
        tiles.push_str(&format!(
            "<figure class=\"tile\"><a href=\"images/{file}\" data-i=\"{i}\"><img src=\"thumbs/{file}\" width=\"{w}\" height=\"{h}\" loading=\"{lazy}\" alt=\"{cap}\"></a>{figcap}</figure>\n"
        ));
    }
    // captions for the lightbox, as JSON with '<' escaped so nothing can close the script
    let list: Vec<(String, String)> = items.iter().map(|(f, c, _, _)| (format!("images/{f}"), c.clone())).collect();
    let data = serde_json::to_string(&list).unwrap_or_else(|_| "[]".into()).replace('<', "\\u003c");
    format!(
        r#"<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>{title}</title>
<style>
*{{box-sizing:border-box}}
body{{margin:0;background:{bg};color:{fg};font:16px/1.4 system-ui,-apple-system,"Segoe UI",sans-serif}}
header{{padding:28px 16px 8px;max-width:1400px;margin:0 auto}}
h1{{margin:0;font-weight:600;font-size:28px}}
.count{{color:{weak};font-size:14px}}
main{{padding:16px;max-width:1400px;margin:0 auto}}
.tile{{margin:0;background:{card};border-radius:8px;overflow:hidden;box-shadow:0 1px 3px rgba(0,0,0,.15)}}
.tile a{{display:block;cursor:zoom-in}}
figcaption{{padding:8px 10px;font-size:14px;color:{weak}}}
{layout}
#box{{position:fixed;inset:0;background:rgba(0,0,0,.92);display:none;align-items:center;justify-content:center;flex-direction:column;z-index:10}}
#box.on{{display:flex}}
#box img{{max-width:94vw;max-height:84vh;box-shadow:0 4px 30px rgba(0,0,0,.6)}}
#cap{{color:#ddd;margin-top:12px;min-height:1.4em;text-align:center;padding:0 16px}}
.nav{{position:fixed;top:50%;transform:translateY(-50%);background:rgba(255,255,255,.12);color:#fff;border:0;font-size:32px;width:56px;height:72px;cursor:pointer;border-radius:8px}}
.nav:hover{{background:rgba(255,255,255,.25)}}
#prev{{left:12px}} #next{{right:12px}}
#close{{position:fixed;top:12px;right:16px;background:none;border:0;color:#fff;font-size:34px;cursor:pointer}}
</style>
</head>
<body>
<header><h1>{title}</h1><div class="count">{count} pictures</div></header>
<main class="grid">
{tiles}</main>
<div id="box" role="dialog" aria-modal="true"><button id="close" aria-label="Close">&times;</button><button class="nav" id="prev" aria-label="Previous">&#8249;</button><img id="big" alt=""><div id="cap"></div><button class="nav" id="next" aria-label="Next">&#8250;</button></div>
<script>
const items={data};let cur=0;const box=document.getElementById('box'),big=document.getElementById('big'),cap=document.getElementById('cap');
function show(i){{cur=(i+items.length)%items.length;big.src=items[cur][0];cap.textContent=items[cur][1];box.classList.add('on');}}
function hide(){{box.classList.remove('on');}}
document.querySelectorAll('.tile a').forEach(a=>a.addEventListener('click',e=>{{e.preventDefault();show(+a.dataset.i);}}));
document.getElementById('prev').onclick=e=>{{e.stopPropagation();show(cur-1);}};
document.getElementById('next').onclick=e=>{{e.stopPropagation();show(cur+1);}};
document.getElementById('close').onclick=hide;box.onclick=e=>{{if(e.target===box)hide();}};
document.addEventListener('keydown',e=>{{if(!box.classList.contains('on'))return;if(e.key==='Escape')hide();if(e.key==='ArrowLeft')show(cur-1);if(e.key==='ArrowRight')show(cur+1);}});
let x0=null;box.addEventListener('touchstart',e=>{{x0=e.touches[0].clientX;}});box.addEventListener('touchend',e=>{{if(x0===null)return;const d=e.changedTouches[0].clientX-x0;if(Math.abs(d)>40)show(cur+(d<0?1:-1));x0=null;}});
</script>
</body>
</html>
"#,
        count = items.len()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `ALBUMCRAFT_GALLERY_OUT=/some/dir cargo test demo_gallery -- --ignored`: a sample gallery
    /// in each style, to look at in a browser.
    #[test]
    #[ignore]
    fn demo_gallery() {
        let out = PathBuf::from(std::env::var("ALBUMCRAFT_GALLERY_OUT").expect("set ALBUMCRAFT_GALLERY_OUT"));
        let items: Vec<Item> = (1..=12).map(|n| (format!("{}{n}", imaging::SAMPLE_PREFIX), format!("{n}.jpg"), vec![], format!("Picture {n}"))).collect();
        for style in Style::ALL {
            let opts = Options { title: format!("Demo {}", style.label()), style, size: 1280, captions: true };
            let (dir, _) = write(&out, &items, &opts).unwrap();
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
        let opts = Options { title: "Trip / \"2009\"".into(), style: Style::Masonry, size: 300, captions: true };
        let (dir, failed) = write(&parent, &items, &opts).unwrap();
        let (dir2, _) = write(&parent, &items, &opts).unwrap();
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
    }
}
