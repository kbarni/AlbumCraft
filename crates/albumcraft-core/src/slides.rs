//! Slideshow settings, the slow pan-and-zoom ("Ken Burns") motion and crossfade timing shared by
//! the on-screen slideshow and the movie export.

use serde::{Deserialize, Serialize};

/// Length of a crossfade, in seconds.
pub const FADE: f32 = 1.0;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SlideSettings {
    /// Move to the next picture after `interval` (off: only when asked). Movies always do.
    pub auto_advance: bool,
    /// Seconds each picture is shown (not counting the crossfade).
    pub interval: f32,
    /// Crossfade between pictures.
    pub fade: bool,
    /// Slow pan and zoom on each picture.
    pub motion: bool,
    /// A music file played (looped) during the slideshow.
    pub music: Option<String>,
    pub music_on: bool,
    /// Show each picture's caption under it.
    pub captions: bool,
    /// Pan and zoom further ("medium") than the usual subtle motion.
    pub strong_motion: bool,
}

impl Default for SlideSettings {
    fn default() -> Self {
        SlideSettings { auto_advance: true, interval: 4.0, fade: true, motion: true, music: None, music_on: false, captions: true, strong_motion: false }
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
/// window never leaves the picture. `strong`: a zoom of 25 % instead of 15 %.
pub fn ken_burns_uv(index: usize, t: f32, strong: bool) -> [f32; 4] {
    let zoom = if strong { 0.25 } else { 0.15 };
    let t = smoothstep(t);
    let z = if index.is_multiple_of(2) { 1.0 + zoom * t } else { 1.0 + zoom * (1.0 - t) };
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn motion_stays_inside_the_picture() {
        for strong in [false, true] {
            for i in 0..8 {
                for k in 0..=20 {
                    let [u0, v0, u1, v1] = ken_burns_uv(i, k as f32 / 20.0, strong);
                    assert!(u0 >= -1e-6 && v0 >= -1e-6 && u1 <= 1.0 + 1e-6 && v1 <= 1.0 + 1e-6, "{i} {k}: {u0} {v0} {u1} {v1}");
                    assert!(u1 > u0 && v1 > v0);
                }
            }
        }
        // it moves, further when strong
        assert_ne!(ken_burns_uv(0, 0.0, false), ken_burns_uv(0, 1.0, false));
        let width = |uv: [f32; 4]| uv[2] - uv[0];
        assert!(width(ken_burns_uv(0, 1.0, true)) < width(ken_burns_uv(0, 1.0, false)));
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
}
