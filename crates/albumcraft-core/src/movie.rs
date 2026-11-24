//! Movie export: the slideshow (crossfades, pan and zoom, music) rendered frame by frame and
//! encoded in the program with the FFmpeg libraries it plays videos with (MPEG-4 video and AAC
//! sound in an MP4 file, both built into FFmpeg under the LGPL): nothing has to be installed and
//! nothing but the finished movie is written to disk.

use crate::imaging::{self, EditOp};
use crate::slides::{self, SlideSettings};
use image::RgbaImage;
use rayon::prelude::*;
use std::path::Path;

pub const FPS: u32 = 30;

/// Whether this build can make movies (it has the FFmpeg libraries: feature `video`).
pub fn available() -> bool {
    cfg!(feature = "video")
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
        return Err(crate::msg!("movie-no-pictures", "no pictures").into());
    }
    let failed = |e: String| crate::msg!("movie-encode-failed", "The movie could not be made: {error}", error = e).to_string();
    let (w, h) = (w & !1, h & !1); // the video format wants even sizes
    let music = s.music.as_ref().filter(|m| s.music_on && Path::new(m).is_file());
    let mut writer = encode::Writer::new(out, w, h, music.map(String::as_str)).map_err(failed)?;

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
        if let Err(e) = writer.push(&frame) {
            drop(writer);
            let _ = std::fs::remove_file(out);
            return Err(failed(e));
        }
        if f % (FPS as usize / 2).max(1) == 0 && !progress(f as f32 / total as f32) {
            drop(writer);
            let _ = std::fs::remove_file(out);
            return Err(crate::library::STOPPED.into());
        }
    }
    if let Err(e) = writer.finish() {
        let _ = std::fs::remove_file(out);
        return Err(failed(e));
    }
    let _ = progress(1.0);
    Ok(())
}

/// The encoder, with the FFmpeg libraries.
#[cfg(feature = "video")]
mod encode {
    use super::FPS;
    use ff::format::{Pixel, Sample, sample::Type as SampleType};
    use ff::{ChannelLayout, Dictionary, Packet, Rational, codec, encoder, format, frame, media};
    use ffmpeg_next as ff;
    use std::collections::VecDeque;
    use std::path::Path;

    const RATE: u32 = 48_000;
    const PLANAR: Sample = Sample::F32(SampleType::Planar);

    fn err<E: std::fmt::Display>(what: &'static str) -> impl FnOnce(E) -> String {
        move |e| format!("{what}: {e}")
    }

    /// The music as stereo 48 kHz sound, started again when it ends.
    struct Music {
        path: String,
        input: format::context::Input,
        decoder: ff::decoder::Audio,
        index: usize,
        resampler: Option<ff::software::resampling::Context>,
        left: VecDeque<f32>,
        right: VecDeque<f32>,
    }

    impl Music {
        fn open(path: &str) -> Result<Music, String> {
            let input = format::input(path).map_err(err("music"))?;
            let stream = input.streams().best(media::Type::Audio).ok_or("music: no sound in the file")?;
            let index = stream.index();
            let decoder = codec::context::Context::from_parameters(stream.parameters()).and_then(|c| c.decoder().audio()).map_err(err("music"))?;
            Ok(Music { path: path.to_string(), input, decoder, index, resampler: None, left: VecDeque::new(), right: VecDeque::new() })
        }

        fn drain(&mut self) {
            let mut sound = frame::Audio::empty();
            while self.decoder.receive_frame(&mut sound).is_ok() {
                // some files (WAV) give frames without a channel layout
                let layout = ChannelLayout::default(i32::from(sound.channels().max(1)));
                if sound.channel_layout().is_empty() {
                    sound.set_channel_layout(layout);
                }
                if self.resampler.is_none() {
                    self.resampler = ff::software::resampling::Context::get(sound.format(), layout, sound.rate(), PLANAR, ChannelLayout::STEREO, RATE).ok();
                }
                let Some(r) = self.resampler.as_mut() else { continue };
                let mut out = frame::Audio::empty();
                if r.run(&sound, &mut out).is_err() {
                    continue;
                }
                let n = out.samples();
                self.left.extend(&out.plane::<f32>(0)[..n]);
                self.right.extend(&out.plane::<f32>(1)[..n]);
            }
        }

        /// Decode one more packet; false at the end of the file.
        fn step(&mut self) -> bool {
            loop {
                let Some((stream, packet)) = self.input.packets().next() else { return false };
                if stream.index() != self.index || self.decoder.send_packet(&packet).is_err() {
                    continue;
                }
                self.drain();
                return true;
            }
        }

        /// `n` samples of each channel, or None when the file gives no sound.
        fn take(&mut self, n: usize) -> Option<(Vec<f32>, Vec<f32>)> {
            let mut gave = false; // sound since the file was last opened
            while self.left.len() < n {
                let before = self.left.len();
                if self.step() {
                    gave |= self.left.len() > before;
                    continue;
                }
                // the end: what the decoder still holds, then again from the start
                let _ = self.decoder.send_eof();
                self.drain();
                gave |= self.left.len() > before;
                if self.left.len() >= n {
                    break;
                }
                if !gave {
                    return None;
                }
                let again = Music::open(&self.path).ok()?;
                let (left, right) = (std::mem::take(&mut self.left), std::mem::take(&mut self.right));
                *self = again;
                self.left = left;
                self.right = right;
                gave = false;
            }
            Some((self.left.drain(..n).collect(), self.right.drain(..n).collect()))
        }
    }

    struct Sound {
        encoder: encoder::Audio,
        music: Music,
        index: usize,
        time_base: Rational,
        size: usize,
        sent: i64,
    }

    pub struct Writer {
        out: format::context::Output,
        video: encoder::Video,
        index: usize,
        time_base: Rational,
        scaler: ff::software::scaling::Context,
        size: (u32, u32),
        frames: i64,
        sound: Option<Sound>,
    }

    impl Writer {
        pub fn new(path: &Path, w: u32, h: u32, music: Option<&str>) -> Result<Writer, String> {
            crate::video::init_ffmpeg();
            let mut out = format::output_as(path, "mp4").map_err(err("output"))?;
            let global = out.format().flags().contains(format::Flags::GLOBAL_HEADER);

            let codec = encoder::find(codec::Id::MPEG4).ok_or("this FFmpeg has no MPEG-4 encoder")?;
            let mut v = codec::context::Context::new_with_codec(codec).encoder().video().map_err(err("video"))?;
            v.set_width(w);
            v.set_height(h);
            v.set_format(Pixel::YUV420P);
            v.set_time_base(Rational(1, FPS as i32));
            v.set_frame_rate(Some(Rational(FPS as i32, 1)));
            v.set_gop(FPS * 2);
            v.set_bit_rate((w as usize * h as usize * FPS as usize / 6).min(20_000_000));
            if global {
                v.set_flags(codec::Flags::GLOBAL_HEADER);
            }
            let video = v.open().map_err(err("video"))?;
            let index = {
                let mut stream = out.add_stream(codec).map_err(err("video"))?;
                stream.set_parameters(&video);
                stream.index()
            };

            // sound: the music, if any
            let mut sound = None;
            if let Some(path) = music {
                let music = Music::open(path)?;
                let codec = encoder::find(codec::Id::AAC).ok_or("this FFmpeg has no AAC encoder")?;
                let mut a = codec::context::Context::new_with_codec(codec).encoder().audio().map_err(err("sound"))?;
                a.set_rate(RATE as i32);
                a.set_channel_layout(ChannelLayout::STEREO);
                a.set_format(PLANAR);
                a.set_bit_rate(192_000);
                a.set_time_base(Rational(1, RATE as i32));
                if global {
                    a.set_flags(codec::Flags::GLOBAL_HEADER);
                }
                let encoder = a.open().map_err(err("sound"))?;
                let index = {
                    let mut stream = out.add_stream(codec).map_err(err("sound"))?;
                    stream.set_parameters(&encoder);
                    stream.index()
                };
                let size = (encoder.frame_size() as usize).max(1024);
                sound = Some(Sound { encoder, music, index, time_base: Rational(1, RATE as i32), size, sent: 0 });
            }

            let mut options = Dictionary::new();
            options.set("movflags", "+faststart");
            out.write_header_with(options).map_err(err("header"))?;
            let time_base = out.stream(index).map(|s| s.time_base()).unwrap_or(Rational(1, FPS as i32));
            if let Some(s) = sound.as_mut() {
                s.time_base = out.stream(s.index).map(|s| s.time_base()).unwrap_or(s.time_base);
            }
            let scaler = ff::software::scaling::Context::get(Pixel::RGBA, w, h, Pixel::YUV420P, w, h, ff::software::scaling::Flags::BICUBIC).map_err(err("scaling"))?;
            Ok(Writer { out, video, index, time_base, scaler, size: (w, h), frames: 0, sound })
        }

        fn write_video(&mut self) -> Result<(), String> {
            let mut packet = Packet::empty();
            while self.video.receive_packet(&mut packet).is_ok() {
                packet.set_stream(self.index);
                packet.rescale_ts(Rational(1, FPS as i32), self.time_base);
                packet.write_interleaved(&mut self.out).map_err(err("writing"))?;
            }
            Ok(())
        }

        fn write_sound(&mut self) -> Result<(), String> {
            let Some(s) = self.sound.as_mut() else { return Ok(()) };
            let mut packet = Packet::empty();
            while s.encoder.receive_packet(&mut packet).is_ok() {
                packet.set_stream(s.index);
                packet.rescale_ts(Rational(1, RATE as i32), s.time_base);
                packet.write_interleaved(&mut self.out).map_err(err("writing"))?;
            }
            Ok(())
        }

        /// Add the next picture of the movie (RGBA, the size given to `new`).
        pub fn push(&mut self, rgba: &[u8]) -> Result<(), String> {
            let (w, h) = self.size;
            let mut src = frame::Video::new(Pixel::RGBA, w, h);
            let stride = src.stride(0);
            let row = w as usize * 4;
            for (y, line) in rgba.chunks_exact(row).enumerate().take(h as usize) {
                src.data_mut(0)[y * stride..y * stride + row].copy_from_slice(line);
            }
            let mut yuv = frame::Video::empty();
            self.scaler.run(&src, &mut yuv).map_err(err("scaling"))?;
            yuv.set_pts(Some(self.frames));
            self.video.send_frame(&yuv).map_err(err("video"))?;
            self.write_video()?;
            self.frames += 1;
            // the sound up to the end of this picture
            let target = self.frames * i64::from(RATE) / i64::from(FPS);
            self.feed_sound(target)
        }

        fn feed_sound(&mut self, target: i64) -> Result<(), String> {
            loop {
                let Some(s) = self.sound.as_mut() else { return Ok(()) };
                if s.sent + s.size as i64 > target {
                    return Ok(());
                }
                let Some((left, right)) = s.music.take(s.size) else {
                    // the music can't be read on: the movie goes on without it
                    let _ = s.encoder.send_eof();
                    self.write_sound()?;
                    self.sound = None;
                    return Ok(());
                };
                let mut f = frame::Audio::new(PLANAR, s.size, ChannelLayout::STEREO);
                f.set_rate(RATE);
                f.plane_mut::<f32>(0).copy_from_slice(&left);
                f.plane_mut::<f32>(1).copy_from_slice(&right);
                f.set_pts(Some(s.sent));
                s.sent += s.size as i64;
                s.encoder.send_frame(&f).map_err(err("sound"))?;
                self.write_sound()?;
            }
        }

        pub fn finish(mut self) -> Result<(), String> {
            self.video.send_eof().map_err(err("video"))?;
            self.write_video()?;
            if let Some(s) = self.sound.as_mut() {
                let _ = s.encoder.send_eof();
            }
            self.write_sound()?;
            self.out.write_trailer().map_err(err("trailer"))
        }
    }
}

/// Without the FFmpeg libraries there is no encoder.
#[cfg(not(feature = "video"))]
mod encode {
    use std::path::Path;

    pub struct Writer;

    impl Writer {
        pub fn new(_: &Path, _: u32, _: u32, _: Option<&str>) -> Result<Writer, String> {
            Err("this version of AlbumCraft was built without the FFmpeg libraries".into())
        }
        pub fn push(&mut self, _: &[u8]) -> Result<(), String> {
            Err(String::new())
        }
        pub fn finish(self) -> Result<(), String> {
            Ok(())
        }
    }
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

    /// One second of a tone as a WAV file (16-bit mono), to be the music.
    fn wav_tone(path: &Path) {
        let samples: Vec<u8> = (0..RATE_TEST).flat_map(|i| (((i as f32 * 440.0 * std::f32::consts::TAU / RATE_TEST as f32).sin() * 8000.0) as i16).to_le_bytes()).collect();
        let mut f = Vec::new();
        f.extend(b"RIFF");
        f.extend((36 + samples.len() as u32).to_le_bytes());
        f.extend(b"WAVEfmt ");
        f.extend(16u32.to_le_bytes());
        f.extend(1u16.to_le_bytes());
        f.extend(1u16.to_le_bytes());
        f.extend(RATE_TEST.to_le_bytes());
        f.extend((RATE_TEST * 2).to_le_bytes());
        f.extend(2u16.to_le_bytes());
        f.extend(16u16.to_le_bytes());
        f.extend(b"data");
        f.extend((samples.len() as u32).to_le_bytes());
        f.extend(samples);
        std::fs::write(path, f).unwrap();
    }
    const RATE_TEST: u32 = 22_050;

    /// A real short movie with looped music, read back with the video code.
    #[cfg(feature = "video")]
    #[test]
    fn makes_a_short_movie_with_music() {
        let id = std::process::id();
        let out = std::env::temp_dir().join(format!("albumcraft-movie-{id}.mp4"));
        let song = std::env::temp_dir().join(format!("albumcraft-movie-{id}.wav"));
        wav_tone(&song);
        let pics = vec![(format!("{}1", imaging::SAMPLE_PREFIX), vec![]), (format!("{}2", imaging::SAMPLE_PREFIX), vec![])];
        // more movie than music: the music must start again
        let s = SlideSettings { interval: 1.0, fade: true, motion: true, music: Some(song.to_string_lossy().to_string()), music_on: true, ..Default::default() };
        let r = make(&out, &pics, (160, 90), &s, &|_| true);
        let size = std::fs::metadata(&out).map(|m| m.len()).unwrap_or(0);
        let info = crate::video::probe(&out.to_string_lossy());
        let _ = std::fs::remove_file(&out);
        let _ = std::fs::remove_file(&song);
        r.unwrap();
        assert!(size > 1000, "movie of {size} bytes");
        let info = info.expect("the movie can be read");
        assert!(info.has_audio);
        assert_eq!((info.width, info.height), (160, 90));
        assert!((info.duration - f64::from(duration(2, &s))).abs() < 0.2, "{} s", info.duration);
    }

    #[cfg(feature = "video")]
    #[test]
    fn stopping_removes_the_file() {
        let out = std::env::temp_dir().join(format!("albumcraft-movie-stop-{}.mp4", std::process::id()));
        let pics = vec![(format!("{}1", imaging::SAMPLE_PREFIX), vec![])];
        let s = SlideSettings { interval: 2.0, music_on: false, ..Default::default() };
        let r = make(&out, &pics, (160, 90), &s, &|_| false);
        assert_eq!(r, Err(crate::library::STOPPED.to_string()));
        assert!(!out.exists());
    }
}
