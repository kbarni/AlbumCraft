//! Videos: what a video file says about itself (size as shown, length, date, place) and single
//! frames from it (thumbnails, the filmstrip), read with FFmpeg's libraries (feature `video`;
//! the playback-only build of `packaging/ffmpeg/build.sh`). Playing is the window's `player`.
//!
//! Without the feature, `is_video` is false for every file: videos are left out of the library
//! as before, and the rest of this module is never reached.

use image::RgbaImage;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{PoisonError, RwLock};

/// What the user set for a video (`Meta.video`), in seconds from its start: where it starts
/// and ends playing (the in and out points: the editor's `[` and `]`, slideshows) and the frame
/// its thumbnail shows. The file itself is never changed.
#[derive(Serialize, Deserialize, Default, Clone, Copy, Debug, PartialEq)]
#[serde(default)]
pub struct VideoMeta {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub poster: Option<f64>,
}

impl VideoMeta {
    pub fn is_empty(&self) -> bool {
        self.start.is_none() && self.end.is_none() && self.poster.is_none()
    }

    /// Drop values a hand-edited file could hold: not numbers, before the start, an out point
    /// before the in point.
    pub fn sanitize(&mut self) {
        for v in [&mut self.start, &mut self.end, &mut self.poster] {
            if v.is_some_and(|s| !s.is_finite() || s < 0.0) {
                *v = None;
            }
        }
        if self.start.is_some_and(|s| s == 0.0) {
            self.start = None;
        }
        if let (Some(s), Some(e)) = (self.start, self.end)
            && e <= s
        {
            self.end = None;
        }
    }

    /// The part played, (from, to) in seconds, for a video of `duration` seconds.
    pub fn range(&self, duration: f64) -> (f64, f64) {
        let d = duration.max(0.0);
        let from = self.start.unwrap_or(0.0).min(d);
        let to = self.end.unwrap_or(d).clamp(from, d);
        if to - from < 0.05 { (0.0, d) } else { (from, to) }
    }
}

/// The thumbnail frame each video shows when the user chose one, by key: picture loading
/// (`imaging::load_source`) is asked for a key only, so the `Store` keeps this in step
/// (`sync_posters` after loading and moving, `set_poster` when one changes).
static POSTERS: RwLock<Option<HashMap<String, f64>>> = RwLock::new(None);

pub fn sync_posters(meta: &HashMap<String, crate::library::Meta>) {
    let map: HashMap<String, f64> = meta.iter().filter_map(|(k, m)| Some((k.clone(), m.video.poster?))).collect();
    *POSTERS.write().unwrap_or_else(PoisonError::into_inner) = Some(map);
}

pub fn set_poster(key: &str, at: Option<f64>) {
    let mut g = POSTERS.write().unwrap_or_else(PoisonError::into_inner);
    let map = g.get_or_insert_with(HashMap::new);
    match at {
        Some(t) => map.insert(key.to_string(), t),
        None => map.remove(key),
    };
}

/// The thumbnail frame the user chose for this video, if any.
pub fn poster_of(key: &str) -> Option<f64> {
    POSTERS.read().unwrap_or_else(PoisonError::into_inner).as_ref()?.get(key).copied()
}

/// The video's thumbnail: the frame the user chose (`poster_of`), else `default_poster_time`.
pub fn poster(path: &str, max_dim: u32) -> Option<RgbaImage> {
    frame(path, poster_of(path), max_dim)
}

/// The frame shown at `secs`, upright, its longest side at most `max_dim` pixels (0 = full size).
pub fn frame_at(path: &str, secs: f64, max_dim: u32) -> Option<RgbaImage> {
    frame(path, Some(secs), max_dim)
}

/// Extensions of the video files shown in the library (lower case).
pub const VIDEO_EXTENSIONS: [&str; 16] =
    ["mp4", "m4v", "mov", "qt", "3gp", "3g2", "avi", "mkv", "webm", "mts", "m2ts", "m2t", "mpg", "mpeg", "dv", "ogv"];

/// Whether a file with this (lower-case) extension is a video the library shows.
pub fn is_video_extension(ext: &str) -> bool {
    cfg!(feature = "video") && VIDEO_EXTENSIONS.contains(&ext)
}

/// Whether the file (by its name) is a video.
pub fn is_video(path: &str) -> bool {
    std::path::Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| is_video_extension(&e.to_ascii_lowercase()))
}

/// What a video file says about itself.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Info {
    /// The size as shown: turned when the phone recorded it sideways.
    pub width: u32,
    pub height: u32,
    /// Seconds.
    pub duration: f64,
    /// Frames per second (0 when unknown).
    pub fps: f64,
    /// Quarter turns clockwise to show it upright (0–3), from the file's display matrix.
    pub turns: u8,
    /// When it was recorded (Unix seconds), from the file's own date.
    pub taken: Option<i64>,
    /// Where it was recorded, when the camera wrote it (`location`, ISO 6709).
    pub gps: Option<crate::geo::LatLon>,
    pub has_audio: bool,
    /// The video codec's short name (h264, hevc, …).
    pub codec: String,
}

/// The time a thumbnail shows when the user chose none: a second in (the very first frames are
/// often dark or blurred), or a tenth of a short clip.
pub fn default_poster_time(duration: f64) -> f64 {
    if duration <= 0.0 { 0.0 } else { (duration * 0.1).min(1.0) }
}

/// "1:05", "12:00", "1:02:03" for a length in seconds.
pub fn fmt_duration(secs: f64) -> String {
    let s = secs.max(0.0).round() as u64;
    let (h, m, s) = (s / 3600, s / 60 % 60, s % 60);
    if h > 0 { format!("{h}:{m:02}:{s:02}") } else { format!("{m}:{s:02}") }
}

/// The clockwise quarter turns a display matrix (9 values, 16.16 fixed point for the first two
/// columns, as stored in MP4/MOV files) asks for.
pub fn turns_of_matrix(m: &[i32; 9]) -> u8 {
    let (a, b) = (f64::from(m[0]), f64::from(m[1]));
    if a == 0.0 && b == 0.0 {
        return 0;
    }
    // FFmpeg's av_display_rotation_get is -atan2(b, a): counter-clockwise; shown upright = turned back
    let degrees = b.atan2(a).to_degrees();
    ((degrees / 90.0).round() as i64).rem_euclid(4) as u8
}

/// A position "+48.8577+002.2950+035.000/" (ISO 6709, as phones write it).
pub fn parse_iso6709(text: &str) -> Option<crate::geo::LatLon> {
    let t = text.trim().trim_end_matches('/');
    let bytes = t.as_bytes();
    // the numbers start at each sign
    let starts: Vec<usize> = (0..bytes.len()).filter(|&i| bytes[i] == b'+' || bytes[i] == b'-').collect();
    if starts.len() < 2 || starts[0] != 0 {
        return None;
    }
    let end = starts.get(2).copied().unwrap_or(t.len());
    let lat: f64 = t.get(starts[0]..starts[1])?.parse().ok()?;
    let lon: f64 = t.get(starts[1]..end)?.parse().ok()?;
    if lat == 0.0 && lon == 0.0 {
        return None;
    }
    crate::geo::LatLon::new(lat, lon)
}

/// A file's date: Apple's local date with its offset ("2023-07-01T14:22:05+0200") or the
/// container's UTC creation time ("2023-07-01T12:22:05.000000Z"). Cameras that don't know the
/// time write 1904 or 1970: those count as none.
pub fn parse_date(text: &str) -> Option<i64> {
    let t = text.trim();
    let parsed = chrono::DateTime::parse_from_rfc3339(t)
        .or_else(|_| chrono::DateTime::parse_from_str(t, "%Y-%m-%dT%H:%M:%S%z"))
        .map(|d| d.timestamp())
        .ok()
        .or_else(|| {
            let naive = chrono::NaiveDateTime::parse_from_str(t.trim_end_matches('Z'), "%Y-%m-%dT%H:%M:%S%.f")
                .or_else(|_| chrono::NaiveDateTime::parse_from_str(t, "%Y-%m-%d %H:%M:%S"))
                .ok()?;
            Some(naive.and_utc().timestamp())
        })?;
    // before 1980: a camera clock that was never set
    (parsed > 315_532_800).then_some(parsed)
}

/// Sound from `Playback`: interleaved stereo at this rate.
pub const AUDIO_RATE: u32 = 48_000;

/// A frame from `Playback`, upright already when the stored picture is (`turns` says how to
/// turn it: the window turns the texture, which costs nothing).
pub struct VideoFrame {
    /// The seek it belongs to (`Playback::seek`): frames of earlier ones are dropped.
    pub generation: u64,
    /// Seconds from the start of the video.
    pub pts: f64,
    pub width: u32,
    pub height: u32,
    /// RGBA, `width * 4` bytes a row.
    pub rgba: Vec<u8>,
}

/// Sound from `Playback`: `samples` (interleaved stereo, `AUDIO_RATE`) starting at `pts`.
pub struct AudioChunk {
    pub generation: u64,
    pub pts: f64,
    pub samples: Vec<f32>,
}

#[cfg_attr(not(feature = "video"), allow(dead_code))]
enum Command {
    Seek { generation: u64, at: f64 },
}

/// A video being played: a thread that decodes it from where it was last sent (`seek`), a few
/// frames ahead, and sends upright-sized frames (`frames`) and its sound (`audio`). The window
/// shows each frame when its time comes and plays the sound (the window's `player`). Dropped,
/// the thread stops.
pub struct Playback {
    commands: std::sync::mpsc::Sender<Command>,
    pub frames: std::sync::mpsc::Receiver<VideoFrame>,
    pub audio: Option<std::sync::mpsc::Receiver<AudioChunk>>,
    /// What the file says about itself, once the thread opened it (None: it could not).
    pub info: std::sync::mpsc::Receiver<Option<Info>>,
}

/// Frames waiting to be shown, at most: enough to play smoothly, little to throw away on a seek.
pub const FRAMES_AHEAD: usize = 6;

impl Playback {
    /// Start playing `path` from `at` seconds, frames at most `max_dim` pixels on their longest
    /// side; `wake` is called when something was sent (the window draws again).
    pub fn open(path: &str, at: f64, max_dim: u32, wake: crate::library::Wake) -> Playback {
        let (commands, command_rx) = std::sync::mpsc::channel();
        let (frame_tx, frames) = std::sync::mpsc::sync_channel(FRAMES_AHEAD);
        let (audio_tx, audio) = std::sync::mpsc::channel();
        let (info_tx, info) = std::sync::mpsc::channel();
        let path = path.to_string();
        let _ = commands.send(Command::Seek { generation: 0, at });
        std::thread::Builder::new()
            .name("video".into())
            .spawn(move || {
                #[cfg(feature = "video")]
                with_ffmpeg::play(&path, max_dim, command_rx, frame_tx, audio_tx, info_tx, wake);
                #[cfg(not(feature = "video"))]
                {
                    let _ = (path, max_dim, command_rx, frame_tx, audio_tx);
                    let _ = info_tx.send(None);
                    wake();
                }
            })
            .ok();
        Playback { commands, frames, audio: Some(audio), info }
    }

    /// Go to `at` seconds: the frames and sound sent from now on are tagged `generation`.
    pub fn seek(&self, generation: u64, at: f64) {
        let _ = self.commands.send(Command::Seek { generation, at });
    }
}

/// `count` frames spread over the video (each at the middle of its part), sent one by one as
/// `Done::Filmstrip` as they are read; `max_dim` pixels on their longest side. Stops early
/// when `send` says no one listens any more.
pub fn filmstrip(path: &str, duration: f64, count: usize, max_dim: u32, send: &dyn Fn(crate::library::Done) -> bool) {
    for index in 0..count {
        let at = duration * (index as f64 + 0.5) / count as f64;
        let img = frame_at(path, at, max_dim);
        if !send(crate::library::Done::Filmstrip { key: path.to_string(), index, count, img }) {
            return;
        }
    }
}

#[cfg(feature = "video")]
pub use with_ffmpeg::*;

#[cfg(not(feature = "video"))]
pub fn probe(_path: &str) -> Option<Info> {
    None
}

#[cfg(not(feature = "video"))]
fn frame(_path: &str, _at: Option<f64>, _max_dim: u32) -> Option<RgbaImage> {
    None
}

#[cfg(feature = "video")]
mod with_ffmpeg {
    use super::*;
    use ffmpeg_next as ff;
    use ff::media::Type;

    /// Start the libraries once, quietly (their messages would go to standard error).
    pub fn init() -> bool {
        static OK: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
        *OK.get_or_init(|| {
            let ok = ff::init().is_ok();
            ff::util::log::set_level(if crate::library::tracing() { ff::util::log::Level::Warning } else { ff::util::log::Level::Quiet });
            ok
        })
    }

    /// Open a video file.
    pub fn open(path: &str) -> Option<ff::format::context::Input> {
        if !init() {
            return None;
        }
        ff::format::input(path).ok()
    }

    /// The clockwise quarter turns of a stream (its display matrix), 0 when it has none.
    pub fn stream_turns(stream: &ff::format::stream::Stream) -> u8 {
        for sd in stream.side_data() {
            if sd.kind() == ff::codec::packet::side_data::Type::DisplayMatrix {
                let d = sd.data();
                if d.len() >= 36 {
                    let mut m = [0i32; 9];
                    for (i, v) in m.iter_mut().enumerate() {
                        *v = i32::from_ne_bytes([d[i * 4], d[i * 4 + 1], d[i * 4 + 2], d[i * 4 + 3]]);
                    }
                    return turns_of_matrix(&m);
                }
            }
        }
        0
    }

    pub fn probe(path: &str) -> Option<Info> {
        let input = open(path)?;
        let stream = input.streams().best(Type::Video)?;
        let params = stream.parameters();
        let ctx = ff::codec::context::Context::from_parameters(params).ok()?;
        let video = ctx.decoder().video().ok()?;
        let (w, h) = (video.width(), video.height());
        if w == 0 || h == 0 {
            return None;
        }
        let turns = stream_turns(&stream);
        let (width, height) = if turns % 2 == 1 { (h, w) } else { (w, h) };
        let duration = if input.duration() > 0 {
            input.duration() as f64 / f64::from(ff::ffi::AV_TIME_BASE)
        } else {
            let tb = f64::from(stream.time_base());
            (stream.duration().max(0) as f64) * tb
        };
        let rate = stream.avg_frame_rate();
        let fps = if rate.denominator() != 0 { f64::from(rate) } else { 0.0 };
        let meta = input.metadata();
        let tag = |k: &str| meta.get(k).map(str::to_string).or_else(|| stream.metadata().get(k).map(str::to_string));
        let taken = tag("com.apple.quicktime.creationdate").and_then(|t| parse_date(&t)).or_else(|| tag("creation_time").and_then(|t| parse_date(&t)));
        let gps = tag("com.apple.quicktime.location.ISO6709").or_else(|| tag("location")).and_then(|t| parse_iso6709(&t));
        let has_audio = input.streams().best(Type::Audio).is_some();
        let codec = video.id().name().to_string();
        Some(Info { width, height, duration, fps, turns, taken, gps, has_audio, codec })
    }

    /// A decoder for the stream, using several threads.
    pub fn video_decoder(stream: &ff::format::stream::Stream) -> Option<ff::decoder::Video> {
        let mut ctx = ff::codec::context::Context::from_parameters(stream.parameters()).ok()?;
        ctx.set_threading(ff::threading::Config { kind: ff::threading::Type::Frame, count: 0 });
        ctx.decoder().video().ok()
    }

    /// Fit (w, h) within `max_dim` (0 = as it is), even sizes, at least 2.
    pub fn fit(w: u32, h: u32, max_dim: u32) -> (u32, u32) {
        let s = if max_dim > 0 && w.max(h) > max_dim { f64::from(max_dim) / f64::from(w.max(h)) } else { 1.0 };
        let even = |v: f64| ((v.round() as u32) & !1).max(2);
        (even(f64::from(w) * s), even(f64::from(h) * s))
    }

    /// Turns a converter's RGBA frame into a picture, upright.
    pub fn to_image(frame: &ff::frame::Video, turns: u8) -> Option<RgbaImage> {
        let (w, h) = (frame.width(), frame.height());
        let stride = frame.stride(0);
        let data = frame.data(0);
        let row = w as usize * 4;
        let mut buf = Vec::with_capacity(row * h as usize);
        for y in 0..h as usize {
            buf.extend_from_slice(data.get(y * stride..y * stride + row)?);
        }
        let img = RgbaImage::from_raw(w, h, buf)?;
        Some(match turns % 4 {
            1 => image::imageops::rotate90(&img),
            2 => image::imageops::rotate180(&img),
            3 => image::imageops::rotate270(&img),
            _ => img,
        })
    }

    /// The frame shown at `at` seconds (None: `default_poster_time`). Seeks to the keyframe
    /// before and decodes up to it.
    pub(super) fn frame(path: &str, at: Option<f64>, max_dim: u32) -> Option<RgbaImage> {
        let mut input = open(path)?;
        let secs = at.unwrap_or_else(|| default_poster_time(input.duration().max(0) as f64 / f64::from(ff::ffi::AV_TIME_BASE)));
        let stream = input.streams().best(Type::Video)?;
        let index = stream.index();
        let turns = stream_turns(&stream);
        let tb = f64::from(stream.time_base());
        let start = if stream.start_time() == ff::ffi::AV_NOPTS_VALUE { 0 } else { stream.start_time() };
        let mut decoder = video_decoder(&stream)?;
        let target = start + (secs.max(0.0) / tb) as i64;
        if secs > 0.0 {
            let ts = (secs * f64::from(ff::ffi::AV_TIME_BASE)) as i64;
            // to the keyframe at or before; files that can't seek are decoded from the start
            let _ = input.seek(ts, ..ts);
        }
        // turned sideways: the shorter stored side is the longer one shown, same limit either way
        let (w, h) = fit(decoder.width(), decoder.height(), max_dim);
        let mut scaler: Option<ff::software::scaling::Context> = None;
        let mut decoded = ff::frame::Video::empty();
        let mut last: Option<ff::frame::Video> = None;
        let take = |decoded: &ff::frame::Video, scaler: &mut Option<ff::software::scaling::Context>| -> Option<ff::frame::Video> {
            if scaler.is_none() {
                *scaler = ff::software::scaling::Context::get(decoded.format(), decoded.width(), decoded.height(), ff::format::Pixel::RGBA, w, h, ff::software::scaling::Flags::AREA).ok();
            }
            let mut rgba = ff::frame::Video::empty();
            scaler.as_mut()?.run(decoded, &mut rgba).ok()?;
            Some(rgba)
        };
        let mut packets_after = 0;
        for (s, packet) in input.packets() {
            if s.index() != index {
                continue;
            }
            if decoder.send_packet(&packet).is_err() {
                continue;
            }
            while decoder.receive_frame(&mut decoded).is_ok() {
                let pts = decoded.timestamp().unwrap_or(target);
                if pts + 1 >= target || last.is_none() && pts >= target {
                    return to_image(&take(&decoded, &mut scaler)?, turns);
                }
                last = take(&decoded, &mut scaler);
            }
            packets_after += 1;
            // a broken index: give what is there rather than decode the whole file
            if packets_after > 2000 {
                break;
            }
        }
        let _ = decoder.send_eof();
        while decoder.receive_frame(&mut decoded).is_ok() {
            last = take(&decoded, &mut scaler);
        }
        to_image(&last?, turns)
    }

    /// The thread behind `Playback`.
    pub(super) fn play(
        path: &str,
        max_dim: u32,
        commands: std::sync::mpsc::Receiver<Command>,
        frames: std::sync::mpsc::SyncSender<VideoFrame>,
        audio: std::sync::mpsc::Sender<AudioChunk>,
        info_tx: std::sync::mpsc::Sender<Option<Info>>,
        wake: crate::library::Wake,
    ) {
        use std::collections::VecDeque;
        use std::sync::mpsc::{TryRecvError, TrySendError};
        let info = probe(path);
        let opened = open(path);
        let ok = info.is_some() && opened.is_some();
        let _ = info_tx.send(info);
        wake();
        let Some(mut input) = opened.filter(|_| ok) else { return };
        let Some(vstream) = input.streams().best(Type::Video) else { return };
        let vindex = vstream.index();
        let vtb = f64::from(vstream.time_base());
        let vstart = if vstream.start_time() == ff::ffi::AV_NOPTS_VALUE { 0 } else { vstream.start_time() };
        let Some(mut vdec) = video_decoder(&vstream) else { return };
        let (w, h) = fit(vdec.width(), vdec.height(), max_dim);
        let mut adec = None;
        let mut aindex = usize::MAX;
        let (mut atb, mut astart) = (1.0, 0);
        if let Some(s) = input.streams().best(Type::Audio)
            && let Ok(ctx) = ff::codec::context::Context::from_parameters(s.parameters())
            && let Ok(d) = ctx.decoder().audio()
        {
            aindex = s.index();
            atb = f64::from(s.time_base());
            astart = if s.start_time() == ff::ffi::AV_NOPTS_VALUE { 0 } else { s.start_time() };
            adec = Some(d);
        }
        let stereo = ff::ChannelLayout::STEREO;
        let packed = ff::format::Sample::F32(ff::format::sample::Type::Packed);
        let mut resampler: Option<ff::software::resampling::Context> = None;
        let mut scaler: Option<(ff::software::scaling::Context, ff::format::Pixel, u32, u32)> = None;
        let mut generation = 0;
        let mut skip_before = 0.0;
        let mut pending: VecDeque<VideoFrame> = VecDeque::new();
        let mut eof = false;
        let mut decoded = ff::frame::Video::empty();
        let mut sound = ff::frame::Audio::empty();
        loop {
            // the latest seek asked for wins
            let mut seek = None;
            loop {
                match commands.try_recv() {
                    Ok(Command::Seek { generation, at }) => seek = Some((generation, at)),
                    Err(TryRecvError::Empty) => break,
                    Err(TryRecvError::Disconnected) => return,
                }
            }
            if let Some((g, at)) = seek {
                generation = g;
                skip_before = at.max(0.0);
                let ts = (skip_before * f64::from(ff::ffi::AV_TIME_BASE)) as i64;
                let _ = input.seek(ts, ..ts);
                vdec.flush();
                if let Some(a) = adec.as_mut() {
                    a.flush();
                }
                resampler = None;
                pending.clear();
                eof = false;
            }
            // hand over what was decoded, as far as there is room
            while let Some(f) = pending.pop_front() {
                match frames.try_send(f) {
                    Ok(()) => wake(),
                    Err(TrySendError::Full(f)) => {
                        pending.push_front(f);
                        break;
                    }
                    Err(TrySendError::Disconnected(_)) => return,
                }
            }
            if !pending.is_empty() || eof {
                std::thread::sleep(std::time::Duration::from_millis(if eof { 15 } else { 4 }));
                continue;
            }
            let next = input.packets().next();
            match next {
                // the end: what the decoder still holds
                None => {
                    let _ = vdec.send_eof();
                    eof = true;
                }
                Some((stream, packet)) if stream.index() == aindex => {
                    let Some(a) = adec.as_mut() else { continue };
                    if a.send_packet(&packet).is_err() {
                        continue;
                    }
                    while a.receive_frame(&mut sound).is_ok() {
                        let pts = sound.timestamp().map(|t| (t - astart) as f64 * atb).unwrap_or(skip_before);
                        if resampler.is_none() {
                            let layout = ff::ChannelLayout::default(i32::from(sound.channels().max(1)));
                            resampler = ff::software::resampling::Context::get(sound.format(), layout, sound.rate(), packed, stereo, AUDIO_RATE).ok();
                        }
                        let Some(r) = resampler.as_mut() else { continue };
                        let mut out = ff::frame::Audio::empty();
                        if r.run(&sound, &mut out).is_err() {
                            continue;
                        }
                        let n = out.samples() * 2;
                        let mut samples: Vec<f32> = out.data(0).as_chunks::<4>().0.iter().take(n).map(|b| f32::from_ne_bytes(*b)).collect();
                        let mut start = pts;
                        if pts < skip_before {
                            // the part before the seek is dropped
                            let drop = (((skip_before - pts) * f64::from(AUDIO_RATE)) as usize * 2).min(samples.len());
                            samples.drain(..drop);
                            start = skip_before;
                        }
                        // no one listens (muted for good): stop decoding the sound
                        if !samples.is_empty() && audio.send(AudioChunk { generation, pts: start, samples }).is_err() {
                            adec = None;
                            break;
                        }
                    }
                    continue;
                }
                Some((stream, packet)) => {
                    if stream.index() != vindex || vdec.send_packet(&packet).is_err() {
                        continue;
                    }
                }
            }
            let half_frame = 0.5 / 60.0;
            while vdec.receive_frame(&mut decoded).is_ok() {
                let pts = decoded.timestamp().map(|t| (t - vstart) as f64 * vtb).unwrap_or(skip_before);
                if pts + half_frame < skip_before {
                    continue;
                }
                let (fmt, fw, fh) = (decoded.format(), decoded.width(), decoded.height());
                if scaler.as_ref().is_none_or(|s| (s.1, s.2, s.3) != (fmt, fw, fh)) {
                    scaler = ff::software::scaling::Context::get(fmt, fw, fh, ff::format::Pixel::RGBA, w, h, ff::software::scaling::Flags::BILINEAR).ok().map(|c| (c, fmt, fw, fh));
                }
                let Some((s, ..)) = scaler.as_mut() else { continue };
                let mut rgba = ff::frame::Video::empty();
                if s.run(&decoded, &mut rgba).is_err() {
                    continue;
                }
                let (stride, row) = (rgba.stride(0), w as usize * 4);
                let data = rgba.data(0);
                let mut buf = Vec::with_capacity(row * h as usize);
                for y in 0..h as usize {
                    if let Some(r) = data.get(y * stride..y * stride + row) {
                        buf.extend_from_slice(r);
                    }
                }
                if buf.len() == row * h as usize {
                    pending.push_back(VideoFrame { generation, pts, width: w, height: h, rgba: buf });
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_matrix_turns() {
        let one = 1 << 16;
        assert_eq!(turns_of_matrix(&[one, 0, 0, 0, one, 0, 0, 0, 1 << 30]), 0);
        // what iPhones write for a portrait video
        assert_eq!(turns_of_matrix(&[0, one, 0, -one, 0, 0, 0, 0, 1 << 30]), 1);
        assert_eq!(turns_of_matrix(&[-one, 0, 0, 0, -one, 0, 0, 0, 1 << 30]), 2);
        assert_eq!(turns_of_matrix(&[0, -one, 0, one, 0, 0, 0, 0, 1 << 30]), 3);
    }

    #[test]
    fn places_and_dates() {
        let p = parse_iso6709("+48.8577+002.2950+035.000/").unwrap();
        assert!((p.lat - 48.8577).abs() < 1e-6 && (p.lon - 2.295).abs() < 1e-6);
        let p = parse_iso6709("-33.8688+151.2093/").unwrap();
        assert!((p.lat + 33.8688).abs() < 1e-6 && (p.lon - 151.2093).abs() < 1e-6);
        assert!(parse_iso6709("+00.0000+000.0000/").is_none());
        assert!(parse_iso6709("nonsense").is_none());
        assert_eq!(parse_date("2023-07-01T12:22:05.000000Z"), Some(1_688_214_125));
        assert_eq!(parse_date("2023-07-01T14:22:05+0200"), Some(1_688_214_125));
        assert_eq!(parse_date("1970-01-01T00:00:00.000000Z"), None);
        assert_eq!(fmt_duration(65.4), "1:05");
        assert_eq!(fmt_duration(3723.0), "1:02:03");
        assert_eq!(default_poster_time(4.0), 0.4);
        assert_eq!(default_poster_time(60.0), 1.0);
    }

    /// A short test clip made by the `ffmpeg` program (None when it isn't installed): `secs`
    /// seconds red, then as long blue, 64 × 48, with a tone, H.264 in MP4, plus `extra`
    /// options for the output (e.g. a date). `turn` is a display rotation (counter-clockwise
    /// degrees, as FFmpeg's option counts them).
    #[cfg(feature = "video")]
    pub(crate) fn test_clip(name: &str, secs: f64, turn: i32, extra: &[&str]) -> Option<std::path::PathBuf> {
        use std::process::{Command, Stdio};
        if !crate::movie::ffmpeg_available() {
            eprintln!("skipped: no ffmpeg program to make test videos with");
            return None;
        }
        let dir = std::env::temp_dir().join(format!("albumcraft-video-{}", std::process::id()));
        std::fs::create_dir_all(&dir).ok()?;
        let plain = dir.join(format!("{name}-plain.mp4"));
        let out = dir.join(format!("{name}.mp4"));
        let src = format!(
            "color=c=red:s=64x48:r=25:d={secs}[r];color=c=blue:s=64x48:r=25:d={secs}[b];[r][b]concat=n=2:v=1:a=0,format=yuv420p[v]"
        );
        let tone = format!("sine=frequency=440:duration={}", secs * 2.0);
        let ok = Command::new("ffmpeg")
            .args(["-y", "-v", "error", "-filter_complex", &src, "-f", "lavfi", "-i", &tone, "-map", "[v]", "-map", "0:a", "-c:v", "libx264", "-g", "10", "-c:a", "aac", "-shortest"])
            .args(extra)
            .arg(&plain)
            .stdout(Stdio::null())
            .status()
            .is_ok_and(|s| s.success());
        if !ok {
            return None;
        }
        let ok = Command::new("ffmpeg")
            .args(["-y", "-v", "error", "-display_rotation", &turn.to_string(), "-i"])
            .arg(&plain)
            .args(["-c", "copy"])
            .args(extra)
            .arg(&out)
            .status()
            .is_ok_and(|s| s.success());
        ok.then_some(out)
    }

    #[cfg(feature = "video")]
    #[test]
    fn reads_a_real_clip() {
        let Some(path) = test_clip("plain", 1.0, 0, &["-metadata", "creation_time=2023-07-01T12:22:05.000000Z", "-metadata", "location=+48.8577+002.2950/"]) else { return };
        let key = path.to_string_lossy().to_string();
        assert!(is_video(&key));
        let info = probe(&key).unwrap();
        assert_eq!((info.width, info.height, info.turns), (64, 48, 0));
        assert!((info.duration - 2.0).abs() < 0.15, "{}", info.duration);
        assert!((info.fps - 25.0).abs() < 0.01);
        assert!(info.has_audio);
        assert_eq!(info.codec, "h264");
        assert_eq!(info.taken, Some(1_688_214_125));
        assert!(info.gps.is_some_and(|p| (p.lat - 48.8577).abs() < 1e-4));
        let red = frame_at(&key, 0.4, 0).unwrap();
        let blue = frame_at(&key, 1.5, 32).unwrap();
        assert_eq!((red.width(), red.height()), (64, 48));
        assert_eq!((blue.width(), blue.height()), (32, 24));
        let px = red.get_pixel(32, 24);
        assert!(px[0] > 200 && px[2] < 60, "{px:?}");
        let px = blue.get_pixel(16, 12);
        assert!(px[2] > 200 && px[0] < 60, "{px:?}");
        // past the end: the last frame
        assert!(frame_at(&key, 10.0, 0).is_some_and(|f| f.get_pixel(5, 5)[2] > 200));
    }

    #[cfg(feature = "video")]
    #[test]
    fn a_turned_clip_is_shown_upright() {
        // a phone held upright: the stored picture is landscape, shown turned a quarter clockwise
        // (FFmpeg's -display_rotation counts counter-clockwise)
        let Some(path) = test_clip("turned", 0.5, -90, &[]) else { return };
        let key = path.to_string_lossy().to_string();
        let info = probe(&key).unwrap();
        assert_eq!((info.width, info.height, info.turns), (48, 64, 1));
        let f = frame_at(&key, 0.2, 0).unwrap();
        assert_eq!((f.width(), f.height()), (48, 64));
    }

    /// `ALBUMCRAFT_VIDEO_DIR=<folder> cargo test -p albumcraft-core real_videos -- --ignored --nocapture`:
    /// reads every video there and its thumbnail, and says how long each took.
    #[cfg(feature = "video")]
    #[test]
    #[ignore]
    fn real_videos() {
        let dir = std::env::var("ALBUMCRAFT_VIDEO_DIR").expect("ALBUMCRAFT_VIDEO_DIR");
        let mut files: Vec<_> = std::fs::read_dir(dir).unwrap().flatten().map(|e| e.path().to_string_lossy().to_string()).filter(|p| is_video(p)).collect();
        files.sort();
        assert!(!files.is_empty());
        for f in files {
            let t = std::time::Instant::now();
            let info = probe(&f);
            let probed = t.elapsed().as_millis();
            let thumb = poster(&f, 256);
            println!("{f}: {info:?} in {probed} ms; thumbnail {:?} in {} ms", thumb.as_ref().map(|i| i.dimensions()), t.elapsed().as_millis());
            assert!(info.is_some() && thumb.is_some(), "{f}");
        }
    }

    #[cfg(feature = "video")]
    #[test]
    fn playback_seeks_and_sends_sound() {
        use std::time::Duration;
        let Some(path) = test_clip("play", 1.0, 0, &[]) else { return };
        let key = path.to_string_lossy().to_string();
        let p = Playback::open(&key, 0.2, 0, std::sync::Arc::new(|| {}));
        let info = p.info.recv_timeout(Duration::from_secs(5)).unwrap().unwrap();
        assert!(info.has_audio);
        let first = p.frames.recv_timeout(Duration::from_secs(5)).unwrap();
        assert_eq!(first.generation, 0);
        assert!((first.pts - 0.2).abs() < 0.05, "{}", first.pts);
        assert_eq!(first.rgba.len(), 64 * 48 * 4);
        assert!(first.rgba[0] > 200 && first.rgba[2] < 60, "red first");
        p.seek(1, 1.4);
        // frames of the first generation may still come; the seek's own follow
        let blue = loop {
            let f = p.frames.recv_timeout(Duration::from_secs(5)).unwrap();
            if f.generation == 1 {
                break f;
            }
        };
        assert!((blue.pts - 1.4).abs() < 0.05, "{}", blue.pts);
        assert!(blue.rgba[2] > 200 && blue.rgba[0] < 60, "blue after the seek");
        let audio = p.audio.as_ref().unwrap();
        let chunk = loop {
            let c = audio.recv_timeout(Duration::from_secs(5)).unwrap();
            if c.generation == 1 {
                break c;
            }
        };
        assert!(chunk.pts >= 1.4 - 1e-6 && chunk.pts < 1.5, "{}", chunk.pts);
        assert!(chunk.samples.iter().any(|s| s.abs() > 0.05), "the tone");
    }

    #[cfg(feature = "video")]
    #[test]
    fn filmstrip_frames_arrive_in_order() {
        let Some(path) = test_clip("strip", 1.0, 0, &[]) else { return };
        let key = path.to_string_lossy().to_string();
        let got = std::sync::Mutex::new(vec![]);
        filmstrip(&key, 2.0, 4, 32, &|d| {
            if let crate::library::Done::Filmstrip { index, count, img, .. } = d {
                got.lock().unwrap().push((index, count, img.map(|i| i.get_pixel(4, 4)[2] > 200)));
            }
            true
        });
        // the first half red, the second blue
        assert_eq!(got.into_inner().unwrap(), vec![(0, 4, Some(false)), (1, 4, Some(false)), (2, 4, Some(true)), (3, 4, Some(true))]);
    }
}
