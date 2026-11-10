//! The full-screen slideshow (`View::Slideshow`).

use crate::app::*;
use crate::edit_view::fit_rect;
use crate::icons;
use eframe::egui::{self, Color32, Pos2, Rect, RichText, Vec2};

#[derive(Clone, Copy)]
enum SlideCmd {
    Prev,
    Next,
    PlayPause,
    Stop,
}

/// Ask for a music file; true when one was chosen (music is then switched on).
pub(crate) fn pick_music(s: &mut crate::slides::SlideSettings) -> bool {
    let Some(path) = rfd::FileDialog::new()
        .set_title("Music for the slideshow")
        .add_filter("Music", &["mp3", "ogg", "oga", "flac", "wav", "m4a", "aac", "mp4"])
        .pick_file()
    else {
        return false;
    };
    s.music = Some(path.to_string_lossy().to_string());
    s.music_on = true;
    true
}

impl App {
    pub fn slideshow_ui(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        let now = ctx.input(|i| i.time);
        let settings = self.store.slideshow.clone();
        let step = settings.step_secs() as f64;
        let Some(s) = self.slide.as_mut() else {
            self.view = View::Library;
            return;
        };
        let len = s.list.len();
        if len == 0 {
            self.stop_slideshow();
            return;
        }
        if ctx.input(|i| i.pointer.delta() != Vec2::ZERO) {
            s.last_move = now;
        }
        // keys
        let (mut stop, mut toggle, mut go) = (false, false, 0i32);
        if !ctx.memory(|m| m.focused().is_some()) {
            ctx.input(|i| {
                stop = i.key_pressed(egui::Key::Escape);
                toggle = i.key_pressed(egui::Key::Space);
                if i.key_pressed(egui::Key::ArrowRight) {
                    go = 1;
                }
                if i.key_pressed(egui::Key::ArrowLeft) {
                    go = -1;
                }
            });
        }
        if s.playing && s.paused_at.is_none() && now - s.started >= step {
            go = 1;
        }
        // draw
        let clock = s.paused_at.unwrap_or(now);
        let (id, pos, started, prev) = (s.list[s.pos % len], s.pos, s.started, s.prev);
        let next = s.list[(s.pos + 1) % len];
        let (playing, last_move) = (s.playing, s.last_move);
        self.request_big(next, false);
        ctx.request_repaint();
        let mut clicked: Option<SlideCmd> = None;
        egui::CentralPanel::default().frame(egui::Frame::new().fill(Color32::BLACK)).show(ui, |ui| {
            let full = ui.max_rect();
            let since = (clock - started) as f32;
            let alpha = crate::slides::fade_in(since, &settings);
            if let Some((pid, pindex, pstarted)) = prev
                && alpha < 1.0
            {
                self.paint_slide(ui, full, pid, pindex, (clock - pstarted) as f32, 1.0 - alpha, &settings);
            }
            self.paint_slide(ui, full, id, pos, since, alpha, &settings);
            if let Some(c) = self.meta(id).map(|m| m.caption.clone()).filter(|c| !c.is_empty()) {
                let shade = Color32::from_white_alpha((alpha * 255.0) as u8);
                ui.painter().text(full.center_bottom() - egui::vec2(0.0, 80.0), egui::Align2::CENTER_CENTER, c, egui::FontId::proportional(20.0), shade);
            }
            if now - last_move < 3.0 || !playing {
                clicked = self.slideshow_controls(ui, full, pos, len, playing);
            }
        });
        match clicked {
            Some(SlideCmd::Prev) => go = -1,
            Some(SlideCmd::Next) => go = 1,
            Some(SlideCmd::PlayPause) => toggle = true,
            Some(SlideCmd::Stop) => stop = true,
            None => {}
        }
        if stop {
            self.stop_slideshow();
            return;
        }
        let Some(s) = self.slide.as_mut() else { return };
        if toggle {
            s.playing = !s.playing;
            if s.playing {
                // resume: the clocks skip the time spent paused
                if let Some(p) = s.paused_at.take() {
                    let d = now - p;
                    s.started += d;
                    if let Some(prev) = s.prev.as_mut() {
                        prev.2 += d;
                    }
                }
            } else {
                s.paused_at = Some(now);
            }
            if let Some(m) = &s.music {
                m.set_playing(s.playing);
            }
        }
        if go != 0 {
            let cur = s.list[s.pos % len];
            s.prev = Some((cur, s.pos, s.started));
            s.pos = (s.pos as i64 + go as i64).rem_euclid(len as i64) as usize;
            s.started = s.paused_at.unwrap_or(now);
        }
    }

    /// One slideshow picture, faded by `alpha`, with the pan-and-zoom motion at `since` seconds.
    #[allow(clippy::too_many_arguments)]
    fn paint_slide(&mut self, ui: &mut egui::Ui, full: Rect, id: usize, index: usize, since: f32, alpha: f32, settings: &crate::slides::SlideSettings) {
        let tex = self.big_texture(id, &[]).or_else(|| self.thumbs.get(&id).cloned());
        let Some(tex) = tex else {
            self.request_thumb(id);
            return;
        };
        let r = fit_rect(full.shrink(10.0), tex.size());
        let uv = if settings.motion {
            let [u0, v0, u1, v1] = crate::slides::ken_burns_uv(index, since / settings.step_secs().max(0.1));
            Rect::from_min_max(Pos2::new(u0, v0), Pos2::new(u1, v1))
        } else {
            Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0))
        };
        ui.painter().image(tex.id(), r, uv, Color32::from_white_alpha((alpha.clamp(0.0, 1.0) * 255.0) as u8));
    }

    fn slideshow_controls(&mut self, ui: &mut egui::Ui, full: Rect, pos: usize, len: usize, playing: bool) -> Option<SlideCmd> {
        let mut cmd = None;
        let bar = Rect::from_center_size(full.center_bottom() - egui::vec2(0.0, 40.0), egui::vec2(700.0, 50.0));
        ui.painter().rect_filled(bar, 10, Color32::from_rgba_unmultiplied(30, 30, 30, 210));
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(bar.shrink(8.0)).layout(egui::Layout::left_to_right(egui::Align::Center)));
        child.visuals_mut().override_text_color = Some(Color32::WHITE);
        child.visuals_mut().widgets.inactive.weak_bg_fill = Color32::from_rgb(60, 60, 64);
        child.visuals_mut().widgets.hovered.weak_bg_fill = Color32::from_rgb(80, 80, 90);
        let big = |t: &str| egui::Button::new(RichText::new(t).size(18.0).color(Color32::WHITE));
        if child.add(big(icons::SKIP_PREVIOUS)).on_hover_text("Previous (←)").clicked() {
            cmd = Some(SlideCmd::Prev);
        }
        if child.add(big(if playing { icons::PAUSE } else { icons::PLAY })).on_hover_text("Play / pause (Space)").clicked() {
            cmd = Some(SlideCmd::PlayPause);
        }
        if child.add(big(icons::SKIP_NEXT)).on_hover_text("Next (→)").clicked() {
            cmd = Some(SlideCmd::Next);
        }
        child.label(format!("{} / {}", pos + 1, len));
        child.separator();
        let st = &mut self.store.slideshow;
        let mut changed = false;
        child.spacing_mut().slider_width = 90.0;
        changed |= child.add(egui::Slider::new(&mut st.interval, 2.0..=15.0).suffix(" s").max_decimals(0)).on_hover_text("Seconds per picture").changed();
        changed |= child.toggle_value(&mut st.fade, "Fade").on_hover_text("Crossfade between pictures").changed();
        changed |= child.toggle_value(&mut st.motion, "Motion").on_hover_text("Slow pan and zoom").changed();
        let music_label = if st.music.is_some() { format!("{} Music", icons::MUSIC) } else { format!("{} Choose music…", icons::MUSIC) };
        let mut music_changed = false;
        if st.music.is_some() {
            music_changed |= child.toggle_value(&mut st.music_on, music_label).on_hover_text(st.music.clone().unwrap_or_default()).changed();
            if child.small_button("…").on_hover_text("Choose another music file").clicked() {
                music_changed |= pick_music(st);
            }
        } else if child.button(music_label).clicked() {
            music_changed |= pick_music(st);
        }
        if child.add(big(icons::CLOSE)).on_hover_text("Exit (Esc)").clicked() {
            cmd = Some(SlideCmd::Stop);
        }
        if changed || music_changed {
            self.dirty = true;
        }
        if music_changed {
            let music = self.start_music();
            if let Some(s) = self.slide.as_mut() {
                s.music = music;
                if let Some(m) = &s.music {
                    m.set_playing(s.playing);
                }
            }
        }
        cmd
    }
}
