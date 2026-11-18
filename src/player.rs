//! Videos played in the window (the editor and slideshows): the frames as a texture shown when
//! their time comes, the clock, and the sound through rodio. Decoding is core's
//! `video::Playback`, which `Library::open_video` starts; the filmstrip's frames come as
//! `Done::Filmstrip`.
//!
//! The clock is the window's own time (`ui.input(|i| i.time)`), started when the first frame
//! of a seek is shown; the sound starts with it. Over a clip of some minutes the two drift by
//! less than a frame.

use crate::backend::Library;
use crate::video::{AudioChunk, Info, Playback, VideoFrame};
use eframe::egui::{self, Color32, ColorImage, Pos2, Rect, TextureHandle, TextureOptions};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::Receiver;

/// A video being played.
pub struct Player {
    /// The photo id and key it plays.
    pub id: usize,
    pub key: String,
    /// What the file says about itself, once the decoding thread opened it.
    pub info: Option<Info>,
    /// The file could not be opened or has no picture.
    pub failed: bool,
    playback: Playback,
    /// Of the last seek: frames of earlier ones are dropped.
    generation: u64,
    /// The next frame, waiting for its time.
    next: Option<VideoFrame>,
    tex: Option<TextureHandle>,
    /// The time of the frame shown.
    pub shown_pts: f64,
    /// Where it is, while paused (and where playing started from).
    pos: f64,
    /// (window time, position) when playing started.
    clock: Option<(f64, f64)>,
    /// Playing, or about to once the seek's first frame is there.
    playing: bool,
    /// A seek's first frame is not shown yet.
    waiting: bool,
    /// The part played (in and out points), seconds.
    pub range: (f64, f64),
    sound: Option<Sound>,
    /// Sound for `sound`, until it is opened (at the first Play).
    audio_rx: Option<Receiver<AudioChunk>>,
    audio_generation: Arc<AtomicU64>,
    pub muted: bool,
}

/// What `Player::poll` saw happen.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Polled {
    /// Played up to the out point (and paused there).
    pub ended: bool,
}

impl Player {
    /// Start reading `key` at `at` seconds, paused; frames at most `max_dim` pixels.
    pub fn open(lib: &dyn Library, id: usize, key: &str, at: f64, max_dim: u32) -> Player {
        let mut playback = lib.open_video(key, at, max_dim);
        let audio_rx = playback.audio.take();
        Player {
            id,
            key: key.to_string(),
            info: None,
            failed: false,
            playback,
            generation: 0,
            next: None,
            tex: None,
            shown_pts: at,
            pos: at,
            clock: None,
            playing: false,
            waiting: true,
            range: (0.0, f64::MAX),
            sound: None,
            audio_rx,
            audio_generation: Arc::new(AtomicU64::new(0)),
            muted: false,
        }
    }

    pub fn duration(&self) -> f64 {
        self.info.as_ref().map(|i| i.duration).unwrap_or(0.0)
    }

    /// Seconds of one frame (1/30 when the file does not say).
    pub fn frame_secs(&self) -> f64 {
        let fps = self.info.as_ref().map(|i| i.fps).unwrap_or(0.0);
        if fps > 1.0 { 1.0 / fps } else { 1.0 / 30.0 }
    }

    /// Where it is now, seconds.
    pub fn position(&self, now: f64) -> f64 {
        match self.clock {
            Some((t0, p0)) if self.playing && !self.waiting => p0 + (now - t0).max(0.0),
            _ => self.pos,
        }
    }

    pub fn is_playing(&self) -> bool {
        self.playing
    }

    pub fn play(&mut self, now: f64) {
        if self.playing || self.failed {
            return;
        }
        // at the out point: from the in point again
        if self.pos >= self.range.1 - self.frame_secs() * 0.5 {
            self.seek(self.range.0, now);
        }
        self.playing = true;
        self.clock = Some((now, self.pos));
        self.open_sound();
        if !self.waiting {
            self.set_sound(true);
        }
    }

    pub fn pause(&mut self, now: f64) {
        if !self.playing {
            return;
        }
        self.pos = self.position(now).min(self.range.1);
        self.playing = false;
        self.clock = None;
        self.set_sound(false);
        // the frames sent ahead are from where it was: go back to the one for this place
        self.seek(self.pos, now);
    }

    pub fn toggle(&mut self, now: f64) {
        if self.playing { self.pause(now) } else { self.play(now) }
    }

    /// Go to `at` seconds (within the video); keeps playing if it was.
    pub fn seek(&mut self, at: f64, now: f64) {
        let d = self.duration();
        let at = if d > 0.0 { at.clamp(0.0, d) } else { at.max(0.0) };
        let _ = now;
        self.generation += 1;
        self.audio_generation.store(self.generation, Ordering::Relaxed);
        self.playback.seek(self.generation, at);
        self.next = None;
        self.pos = at;
        self.clock = None;
        self.waiting = true;
        self.set_sound(false);
    }

    /// One frame on (`frames` > 0) or back, paused.
    pub fn step(&mut self, frames: i32, now: f64) {
        self.pause(now);
        let at = self.shown_pts + f64::from(frames) * self.frame_secs();
        self.seek(at, now);
    }

    pub fn set_muted(&mut self, muted: bool) {
        self.muted = muted;
        if let Some(s) = &self.sound {
            s.player.set_volume(if muted { 0.0 } else { 1.0 });
        }
    }

    fn open_sound(&mut self) {
        if self.sound.is_some() || !self.info.as_ref().is_some_and(|i| i.has_audio) {
            return;
        }
        let Some(rx) = self.audio_rx.take() else { return };
        self.sound = Sound::open(rx, self.audio_generation.clone());
        self.set_muted(self.muted);
    }

    fn set_sound(&self, on: bool) {
        if let Some(s) = &self.sound {
            if on { s.player.play() } else { s.player.pause() }
        }
    }

    /// Take what the decoding thread sent and show the frame due now. Call every frame the
    /// player is visible; asks for the next repaint while playing.
    pub fn poll(&mut self, ctx: &egui::Context, now: f64) -> Polled {
        let mut out = Polled::default();
        if self.info.is_none()
            && !self.failed
            && let Ok(info) = self.playback.info.try_recv()
        {
            self.failed = info.is_none();
            if let Some(i) = &info {
                self.range.1 = self.range.1.min(i.duration);
            }
            self.info = info;
            if self.playing {
                self.open_sound();
            }
        }
        let mut due: Option<VideoFrame> = None;
        loop {
            if self.next.is_none() {
                match self.playback.frames.try_recv() {
                    Ok(f) => self.next = Some(f),
                    Err(_) => break,
                }
            }
            let Some(f) = self.next.as_ref() else { break };
            if f.generation != self.generation {
                self.next = None;
                continue;
            }
            if self.waiting {
                // the seek's first frame: shown now, and the clock starts from it
                let f = self.next.take();
                self.waiting = false;
                if self.playing {
                    self.clock = Some((now, self.pos));
                    self.set_sound(true);
                }
                due = f;
                break;
            }
            if self.playing && f.pts <= self.position(now) + 0.004 {
                due = self.next.take();
                continue;
            }
            break;
        }
        if let Some(f) = due {
            self.shown_pts = f.pts;
            let img = ColorImage::from_rgba_unmultiplied([f.width as usize, f.height as usize], &f.rgba);
            match &mut self.tex {
                Some(t) => t.set(img, TextureOptions::LINEAR),
                None => self.tex = Some(ctx.load_texture(format!("video-{}", self.key), img, TextureOptions::LINEAR)),
            }
        }
        if self.playing && !self.waiting && self.position(now) >= self.range.1 {
            self.pos = self.range.1;
            self.playing = false;
            self.clock = None;
            self.set_sound(false);
            out.ended = true;
        }
        if self.playing {
            ctx.request_repaint();
        }
        out
    }

    /// The size the video is shown at (turned when the phone recorded it sideways).
    pub fn shown_size(&self) -> Option<[usize; 2]> {
        let i = self.info.as_ref()?;
        Some([i.width as usize, i.height as usize])
    }

    pub fn has_frame(&self) -> bool {
        self.tex.is_some()
    }

    /// Draw the frame shown into `rect` (the whole of it: fit it first), upright.
    pub fn paint(&self, p: &egui::Painter, rect: Rect, tint: Color32) {
        let Some(tex) = &self.tex else { return };
        let turns = self.info.as_ref().map(|i| i.turns).unwrap_or(0);
        paint_turned(p, tex.id(), rect, turns, tint);
    }
}

/// A texture drawn turned `turns` quarters clockwise, filling `rect`.
pub fn paint_turned(p: &egui::Painter, tex: egui::TextureId, rect: Rect, turns: u8, tint: Color32) {
    // the corners of the stored picture, top left first, clockwise
    let uv = [Pos2::new(0.0, 0.0), Pos2::new(1.0, 0.0), Pos2::new(1.0, 1.0), Pos2::new(0.0, 1.0)];
    let corners = [rect.left_top(), rect.right_top(), rect.right_bottom(), rect.left_bottom()];
    let mut mesh = egui::Mesh::with_texture(tex);
    for (i, c) in corners.iter().enumerate() {
        mesh.vertices.push(egui::epaint::Vertex { pos: *c, uv: uv[(i + 4 - usize::from(turns % 4)) % 4], color: tint });
    }
    mesh.indices.extend_from_slice(&[0, 1, 2, 0, 2, 3]);
    p.add(egui::Shape::mesh(mesh));
}

/// The video's sound: `AudioChunk`s played as they come, silence when none is there (so the
/// output never ends), chunks of an earlier seek dropped.
struct Sound {
    _sink: rodio::MixerDeviceSink,
    player: rodio::Player,
}

impl Sound {
    fn open(rx: Receiver<AudioChunk>, generation: Arc<AtomicU64>) -> Option<Sound> {
        let mut sink = rodio::DeviceSinkBuilder::open_default_sink().ok()?;
        sink.log_on_drop(false);
        let player = rodio::Player::connect_new(sink.mixer());
        player.pause();
        player.append(Feed { rx, generation, buf: vec![], at: 0, buf_generation: u64::MAX, right: false });
        Some(Sound { _sink: sink, player })
    }
}

struct Feed {
    rx: Receiver<AudioChunk>,
    generation: Arc<AtomicU64>,
    buf: Vec<f32>,
    at: usize,
    buf_generation: u64,
    /// The next sample is a right one: a new chunk starts only on a left one.
    right: bool,
}

impl Iterator for Feed {
    type Item = f32;

    fn next(&mut self) -> Option<f32> {
        let g = self.generation.load(Ordering::Relaxed);
        if self.buf_generation != g {
            self.buf.clear();
            self.at = 0;
            self.buf_generation = g;
        }
        if self.at >= self.buf.len() && !self.right {
            while let Ok(c) = self.rx.try_recv() {
                if c.generation == g {
                    self.buf = c.samples;
                    self.at = 0;
                    break;
                }
            }
        }
        let s = if self.at < self.buf.len() {
            self.at += 1;
            self.buf[self.at - 1]
        } else {
            0.0
        };
        self.right = !self.right;
        Some(s)
    }
}

impl rodio::Source for Feed {
    fn current_span_len(&self) -> Option<usize> {
        None
    }

    fn channels(&self) -> rodio::ChannelCount {
        rodio::ChannelCount::new(2).unwrap_or(rodio::ChannelCount::MIN)
    }

    fn sample_rate(&self) -> rodio::SampleRate {
        rodio::SampleRate::new(crate::video::AUDIO_RATE).unwrap_or(rodio::SampleRate::MIN)
    }

    fn total_duration(&self) -> Option<std::time::Duration> {
        None
    }
}

/// The frames of a video's filmstrip, as they arrive (`Done::Filmstrip`).
pub struct Filmstrip {
    pub key: String,
    pub frames: Vec<Option<TextureHandle>>,
    stop: Arc<AtomicBool>,
}

impl Filmstrip {
    pub fn start(lib: &dyn Library, key: &str, duration: f64, count: usize, max_dim: u32) -> Filmstrip {
        let stop = lib.filmstrip(key, duration, count, max_dim);
        Filmstrip { key: key.to_string(), frames: vec![None; count], stop }
    }

    pub fn arrived(&mut self, ctx: &egui::Context, index: usize, img: &image::RgbaImage) {
        if let Some(slot) = self.frames.get_mut(index) {
            let c = ColorImage::from_rgba_unmultiplied([img.width() as usize, img.height() as usize], img.as_raw());
            *slot = Some(ctx.load_texture(format!("strip-{index}"), c, TextureOptions::LINEAR));
        }
    }
}

impl Drop for Filmstrip {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sound_keeps_left_and_right_apart() {
        let (tx, rx) = std::sync::mpsc::channel();
        let g = Arc::new(AtomicU64::new(0));
        let mut feed = Feed { rx, generation: g.clone(), buf: vec![], at: 0, buf_generation: u64::MAX, right: false };
        // nothing yet: silence; a chunk arriving after a left sample starts on the next left one
        assert_eq!(feed.next(), Some(0.0));
        tx.send(AudioChunk { generation: 0, pts: 0.0, samples: vec![1.0, -1.0, 1.0, -1.0] }).unwrap();
        assert_eq!(feed.next(), Some(0.0));
        let got: Vec<f32> = (0..4).filter_map(|_| feed.next()).collect();
        assert_eq!(got, vec![1.0, -1.0, 1.0, -1.0]);
        // a seek: what is left of the old chunk and chunks of the old generation are dropped
        tx.send(AudioChunk { generation: 0, pts: 0.0, samples: vec![5.0, 5.0] }).unwrap();
        tx.send(AudioChunk { generation: 1, pts: 1.0, samples: vec![2.0, -2.0] }).unwrap();
        g.store(1, Ordering::Relaxed);
        let got: Vec<f32> = (0..4).filter_map(|_| feed.next()).collect();
        assert_eq!(got, vec![2.0, -2.0, 0.0, 0.0]);
    }
}
