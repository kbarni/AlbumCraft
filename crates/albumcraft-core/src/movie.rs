//! Movie export: the slideshow (crossfades, pan and zoom, music) rendered frame by frame and
//! encoded by the free FFmpeg program, when it is installed. Frames go to FFmpeg's input as raw
//! RGBA, so nothing but the finished movie is written to disk.

use crate::imaging::{self, EditOp};
use crate::slides::{self, SlideSettings};
use image::RgbaImage;
use rayon::prelude::*;
use std::io::{Read, Write};
use std::path::Path;
use std::process::{Command, Stdio};

pub const FPS: u32 = 30;

/// Whether `ffmpeg` can be run.
pub fn ffmpeg_available() -> bool {
    Command::new("ffmpeg").arg("-version").stdout(Stdio::null()).stderr(Stdio::null()).status().is_ok_and(|s| s.success())
}

fn has_x264() -> bool {
    Command::new("ffmpeg")
        .args(["-hide_banner", "-encoders"])
        .stderr(Stdio::null())
        .output()
        .is_ok_and(|o| String::from_utf8_lossy(&o.stdout).contains("libx264"))
}

/// A picture on screen: (index, seconds since it appeared, opacity).
pub type Shown = (usize, f32, f32);

/// Which pictures show at time `t` (seconds): the current one, and the previous picture still
/// fading out (index, seconds since it appeared), if any.
pub fn at_time(t: f32, n: usize, s: &SlideSettings) -> Option<(Shown, Option<(usize, f32)>)> {
    if n == 0 {
        return None;
    }
    let step = s.step_secs();
    let i = ((t / step).floor() as usize).min(n - 1);
    let since = t - i as f32 * step;
    let alpha = if i == 0 { 1.0 } else { slides::fade_in(since, s) };
    let prev = (i > 0 && alpha < 1.0).then(|| (i - 1, since + step));
    Some(((i, since, alpha), prev))
}

/// Length of the movie, in seconds.
pub fn duration(n: usize, s: &SlideSettings) -> f32 {
    n as f32 * s.step_secs()
}

/// Draw `pic` letterboxed into `out` (w × h, RGBA), showing its part `uv`, blended by `alpha`
/// over what is already there.
pub fn draw(out: &mut [u8], w: u32, h: u32, pic: &RgbaImage, uv: [f32; 4], alpha: f32) {
    let (pw, ph) = (pic.width() as f32, pic.height() as f32);
    if pw < 1.0 || ph < 1.0 || w == 0 || h == 0 {
        return;
    }
    // the visible part's shape decides the letterbox
    let (vw, vh) = ((uv[2] - uv[0]) * pw, (uv[3] - uv[1]) * ph);
    let scale = (w as f32 / vw).min(h as f32 / vh);
    let (rw, rh) = (vw * scale, vh * scale);
    let (rx, ry) = ((w as f32 - rw) / 2.0, (h as f32 - rh) / 2.0);
    let src = pic.as_raw();
    let at = |x: i64, y: i64, c: usize| -> f32 {
        let x = x.clamp(0, pw as i64 - 1) as usize;
        let y = y.clamp(0, ph as i64 - 1) as usize;
        src[(y * pw as usize + x) * 4 + c] as f32
    };
    out.par_chunks_mut(w as usize * 4).enumerate().for_each(|(y, row)| {
        let fy = (y as f32 + 0.5 - ry) / rh;
        if !(0.0..1.0).contains(&fy) {
            return;
        }
        let sy = (uv[1] + fy * (uv[3] - uv[1])) * ph - 0.5;
        let (y0, ty) = (sy.floor() as i64, sy - sy.floor());
        for (x, p) in row.as_chunks_mut::<4>().0.iter_mut().enumerate() {
            let fx = (x as f32 + 0.5 - rx) / rw;
            if !(0.0..1.0).contains(&fx) {
                continue;
            }
            let sx = (uv[0] + fx * (uv[2] - uv[0])) * pw - 0.5;
            let (x0, tx) = (sx.floor() as i64, sx - sx.floor());
            for (c, out) in p.iter_mut().take(3).enumerate() {
                let top = at(x0, y0, c) * (1.0 - tx) + at(x0 + 1, y0, c) * tx;
                let bot = at(x0, y0 + 1, c) * (1.0 - tx) + at(x0 + 1, y0 + 1, c) * tx;
                let v = top * (1.0 - ty) + bot * ty;
                *out = (*out as f32 * (1.0 - alpha) + v * alpha).round().clamp(0.0, 255.0) as u8;
            }
            p[3] = 255;
        }
    });
}

/// Make the movie. `pictures` are (source key, edits); `progress` gets 0…1 twice a second of
/// movie; when it answers false the work stops, the unfinished file is removed and the error is
/// `library::STOPPED`.
pub fn make(out: &Path, pictures: &[(String, Vec<EditOp>)], (w, h): (u32, u32), s: &SlideSettings, progress: &dyn Fn(f32) -> bool) -> Result<(), String> {
    if pictures.is_empty() {
        return Err("no pictures".into());
    }
    let (w, h) = (w & !1, h & !1); // the video format wants even sizes
    let mut args: Vec<String> = ["-y", "-hide_banner", "-loglevel", "error", "-f", "rawvideo", "-pix_fmt", "rgba"].map(String::from).to_vec();
    args.extend(["-s".into(), format!("{w}x{h}"), "-r".into(), FPS.to_string(), "-i".into(), "-".into()]);
    let music = s.music.as_ref().filter(|m| s.music_on && Path::new(m).is_file());
    if let Some(m) = music {
        args.extend(["-stream_loop", "-1", "-i"].map(String::from));
        args.push(m.clone());
        args.extend(["-shortest", "-c:a", "aac", "-b:a", "192k"].map(String::from));
    }
    if has_x264() {
        args.extend(["-c:v", "libx264", "-preset", "medium", "-crf", "20"].map(String::from));
    } else {
        args.extend(["-c:v", "mpeg4", "-q:v", "3"].map(String::from));
    }
    args.extend(["-pix_fmt", "yuv420p", "-movflags", "+faststart"].map(String::from));
    args.push(out.to_string_lossy().to_string());
    let mut child = Command::new("ffmpeg")
        .args(&args)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("FFmpeg could not be started: {e}"))?;
    let mut stderr = child.stderr.take();
    let errors = std::thread::spawn(move || {
        let mut text = String::new();
        if let Some(e) = stderr.as_mut() {
            let _ = e.read_to_string(&mut text);
        }
        text
    });
    let mut stdin = child.stdin.take().ok_or("FFmpeg has no input")?;

    // pictures are loaded one ahead, a bit larger than the movie so zooming in stays sharp
    let long = (w.max(h) as f32 * 1.2) as u32;
    let load = |i: usize| -> Option<RgbaImage> {
        let (key, edits) = pictures.get(i)?;
        let src = imaging::load_source(key, long)?;
        Some(imaging::apply_all(&src, edits))
    };
    let mut cache: Vec<(usize, Option<RgbaImage>)> = vec![];
    let get = |i: usize, cache: &mut Vec<(usize, Option<RgbaImage>)>| -> Option<RgbaImage> {
        if let Some((_, p)) = cache.iter().find(|(k, _)| *k == i) {
            return p.clone();
        }
        let p = load(i);
        cache.push((i, p.clone()));
        if cache.len() > 3 {
            cache.remove(0);
        }
        p
    };
    let total = (duration(pictures.len(), s) * FPS as f32).ceil() as usize;
    let mut frame = vec![0u8; w as usize * h as usize * 4];
    let mut result = Ok(());
    for f in 0..total {
        let t = f as f32 / FPS as f32;
        let Some(((i, since, alpha), prev)) = at_time(t, pictures.len(), s) else { break };
        frame.par_chunks_mut(4).for_each(|p| p.copy_from_slice(&[0, 0, 0, 255]));
        let uv_of = |index: usize, since: f32| if s.motion { slides::ken_burns_uv(index, since / s.step_secs(), s.strong_motion) } else { [0.0, 0.0, 1.0, 1.0] };
        if let Some((pi, psince)) = prev
            && let Some(p) = get(pi, &mut cache)
        {
            draw(&mut frame, w, h, &p, uv_of(pi, psince), 1.0);
        }
        if let Some(p) = get(i, &mut cache) {
            draw(&mut frame, w, h, &p, uv_of(i, since), alpha);
        }
        if let Err(e) = stdin.write_all(&frame) {
            result = Err(format!("FFmpeg stopped ({e})"));
            break;
        }
        if f % (FPS as usize / 2).max(1) == 0 && !progress(f as f32 / total as f32) {
            drop(stdin);
            let _ = child.kill();
            let _ = child.wait();
            let _ = errors.join();
            let _ = std::fs::remove_file(out);
            return Err(crate::library::STOPPED.into());
        }
    }
    drop(stdin);
    let status = child.wait().map_err(|e| e.to_string())?;
    let log = errors.join().unwrap_or_default();
    if !status.success() || result.is_err() {
        let tail: String = log.lines().rev().take(4).collect::<Vec<_>>().into_iter().rev().collect::<Vec<_>>().join("\n");
        return Err(match result {
            Err(e) => format!("{e}\n{tail}"),
            Ok(()) => format!("FFmpeg failed:\n{tail}"),
        });
    }
    let _ = progress(1.0);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timeline() {
        let s = SlideSettings { interval: 3.0, fade: true, motion: true, music: None, music_on: false, ..Default::default() };
        assert_eq!(duration(3, &s), 12.0);
        // the first picture has no fade-in
        assert_eq!(at_time(0.0, 3, &s), Some(((0, 0.0, 1.0), None)));
        // half a second into the second one: half faded in over the first
        let ((i, since, a), prev) = at_time(4.5, 3, &s).unwrap();
        assert_eq!((i, since), (1, 0.5));
        assert!((a - 0.5).abs() < 1e-6);
        assert_eq!(prev, Some((0, 4.5)));
        // past the end: the last picture
        assert_eq!(at_time(100.0, 3, &s).unwrap().0.0, 2);
    }

    #[test]
    fn letterboxed_drawing() {
        let pic = RgbaImage::from_pixel(100, 100, image::Rgba([200, 100, 50, 255]));
        let (w, h) = (160u32, 90u32);
        let mut out = vec![0u8; (w * h * 4) as usize];
        draw(&mut out, w, h, &pic, [0.0, 0.0, 1.0, 1.0], 1.0);
        let px = |x: u32, y: u32| &out[((y * w + x) * 4) as usize..((y * w + x) * 4 + 3) as usize];
        assert_eq!(px(80, 45), &[200, 100, 50]);
        assert_eq!(px(5, 45), &[0, 0, 0], "black bars at the sides of a square picture");
    }

    /// Runs only where FFmpeg is installed.
    #[test]
    fn makes_a_short_movie_when_ffmpeg_is_there() {
        if !ffmpeg_available() {
            eprintln!("ffmpeg not found: skipped");
            return;
        }
        let out = std::env::temp_dir().join(format!("albumcraft-movie-{}.mp4", std::process::id()));
        let pics = vec![(format!("{}1", imaging::SAMPLE_PREFIX), vec![]), (format!("{}2", imaging::SAMPLE_PREFIX), vec![])];
        // a second of tone as the music
        let song = std::env::temp_dir().join(format!("albumcraft-movie-{}.wav", std::process::id()));
        let made_song = Command::new("ffmpeg")
            .args(["-y", "-loglevel", "error", "-f", "lavfi", "-i", "sine=frequency=440:duration=1"])
            .arg(&song)
            .status()
            .is_ok_and(|s| s.success());
        let s = SlideSettings { interval: 1.0, fade: true, motion: true, music: Some(song.to_string_lossy().to_string()), music_on: made_song, ..Default::default() };
        let r = make(&out, &pics, (160, 90), &s, &|_| true);
        let size = std::fs::metadata(&out).map(|m| m.len()).unwrap_or(0);
        let streams = Command::new("ffprobe")
            .args(["-v", "error", "-show_entries", "stream=codec_type", "-of", "csv=p=0"])
            .arg(&out)
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
            .unwrap_or_default();
        let _ = std::fs::remove_file(&out);
        let _ = std::fs::remove_file(&song);
        r.unwrap();
        assert!(size > 1000, "movie of {size} bytes");
        assert!(streams.contains("video"), "{streams}");
        if made_song {
            assert!(streams.contains("audio"), "{streams}");
        }
    }
}
