//! Slideshow settings, the slow pan-and-zoom ("Ken Burns") motion and crossfade timing shared by
//! the on-screen slideshow and the movie export, and the background music player.

use serde::{Deserialize, Serialize};

/// Length of a crossfade, in seconds.
pub const FADE: f32 = 1.0;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SlideSettings {
    /// Seconds each picture is shown (not counting the crossfade).
    pub interval: f32,
    /// Crossfade between pictures.
    pub fade: bool,
    /// Slow pan and zoom on each picture.
    pub motion: bool,
    /// A music file played (looped) during the slideshow.
    pub music: Option<String>,
    pub music_on: bool,
}

impl Default for SlideSettings {
    fn default() -> Self {
        SlideSettings { interval: 4.0, fade: true, motion: true, music: None, music_on: false }
    }
}

impl SlideSettings {
    pub fn fade_secs(&self) -> f32 {
        if self.fade { FADE } else { 0.0 }
    }

    /// Time from one picture appearing to the next one appearing.
    pub fn step_secs(&self) -> f32 {
        self.interval.clamp(1.0, 60.0) + self.fade_secs()
    }
}

fn smoothstep(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// The part of the picture (u0, v0, u1, v1 in 0…1) shown at `t` (0…1 over the time the picture
/// is on screen). Even pictures zoom in, odd ones zoom out, each panning a different way; the
/// window never leaves the picture.
pub fn ken_burns_uv(index: usize, t: f32) -> [f32; 4] {
    const ZOOM: f32 = 0.15;
    let t = smoothstep(t);
    let z = if index.is_multiple_of(2) { 1.0 + ZOOM * t } else { 1.0 + ZOOM * (1.0 - t) };
    let s = 1.0 / z;
    let dirs = [(1.0, 0.6), (-1.0, 0.4), (0.7, -1.0), (-0.6, -0.8)];
    let (dx, dy) = dirs[(index * 7 + 3) % dirs.len()];
    let room = (1.0 - s) / 2.0;
    let p = (t - 0.5) * 2.0 * 0.8;
    let (cx, cy) = (0.5 + dx * p * room, 0.5 + dy * p * room);
    [cx - s / 2.0, cy - s / 2.0, cx + s / 2.0, cy + s / 2.0]
}

/// How much of the new picture shows `since` seconds after it appeared (1 = fully).
pub fn fade_in(since: f32, settings: &SlideSettings) -> f32 {
    let f = settings.fade_secs();
    if f <= 0.0 { 1.0 } else { (since / f).clamp(0.0, 1.0) }
}

/// The music file, decoded and looping.
fn open_music(path: &str) -> Result<rodio::decoder::LoopedDecoder<std::io::BufReader<std::fs::File>>, String> {
    let file = std::fs::File::open(path).map_err(|e| format!("{path}: {e}"))?;
    rodio::Decoder::new_looped(std::io::BufReader::new(file)).map_err(|e| format!("this music file can't be played ({e})"))
}

/// Background music, looping until dropped.
pub struct Music {
    _sink: rodio::MixerDeviceSink,
    player: rodio::Player,
}

impl Music {
    pub fn start(path: &str) -> Result<Music, String> {
        let source = open_music(path)?;
        let mut sink = rodio::DeviceSinkBuilder::open_default_sink().map_err(|e| format!("no sound output ({e})"))?;
        sink.log_on_drop(false);
        let player = rodio::Player::connect_new(sink.mixer());
        player.set_volume(0.8);
        player.append(source);
        Ok(Music { _sink: sink, player })
    }

    pub fn set_playing(&self, on: bool) {
        if on {
            self.player.play();
        } else {
            self.player.pause();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn motion_stays_inside_the_picture() {
        for i in 0..8 {
            for k in 0..=20 {
                let [u0, v0, u1, v1] = ken_burns_uv(i, k as f32 / 20.0);
                assert!(u0 >= -1e-6 && v0 >= -1e-6 && u1 <= 1.0 + 1e-6 && v1 <= 1.0 + 1e-6, "{i} {k}: {u0} {v0} {u1} {v1}");
                assert!(u1 > u0 && v1 > v0);
            }
        }
        // it moves
        assert_ne!(ken_burns_uv(0, 0.0), ken_burns_uv(0, 1.0));
    }

    #[test]
    fn fade_timing() {
        let s = SlideSettings::default();
        assert_eq!(fade_in(0.0, &s), 0.0);
        assert_eq!(fade_in(5.0, &s), 1.0);
        let cut = SlideSettings { fade: false, ..s };
        assert_eq!(fade_in(0.0, &cut), 1.0);
        assert_eq!(cut.step_secs(), 4.0);
    }

    #[test]
    fn a_wav_file_decodes() {
        // 0.1 s of 8 kHz mono 16-bit silence
        let samples = 800u32;
        let mut wav = b"RIFF".to_vec();
        wav.extend((36 + samples * 2).to_le_bytes());
        wav.extend(b"WAVEfmt ");
        wav.extend(16u32.to_le_bytes());
        wav.extend(1u16.to_le_bytes()); // PCM
        wav.extend(1u16.to_le_bytes()); // mono
        wav.extend(8000u32.to_le_bytes());
        wav.extend(16000u32.to_le_bytes());
        wav.extend(2u16.to_le_bytes());
        wav.extend(16u16.to_le_bytes());
        wav.extend(b"data");
        wav.extend((samples * 2).to_le_bytes());
        wav.extend(vec![0u8; samples as usize * 2]);
        let path = std::env::temp_dir().join(format!("albumcraft-music-{}.wav", std::process::id()));
        std::fs::write(&path, wav).unwrap();
        let ok = open_music(&path.to_string_lossy()).is_ok();
        let _ = std::fs::remove_file(&path);
        assert!(ok);
    }

    #[test]
    fn missing_music_is_an_error_not_a_crash() {
        assert!(Music::start("/no/such/song.mp3").is_err());
    }
}
