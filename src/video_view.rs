//! The editor for a video: the player in the middle, a play bar with the filmstrip under it,
//! and on the left the part played (in and out points, `[` and `]`) and the frame the
//! thumbnail shows. The photo tools don't apply to videos and are not shown. What is set is
//! `Meta.video` (`Store::set_video`); the file is never changed.

use crate::app::App;
use crate::i18n::tr;
use crate::icons;
use crate::player::{Filmstrip, Player};
use crate::theme::t;
use crate::video::VideoMeta;
use eframe::egui::{self, Color32, Pos2, Rect, RichText, Sense, Stroke, Vec2};

/// Largest side of the frames the editor plays (the texture is scaled on screen).
const EDIT_MAX_DIM: u32 = 1920;
/// Height of the filmstrip, points.
const STRIP_H: f32 = 54.0;

/// "0:03.2": minutes, seconds and tenths.
pub fn fmt_time(secs: f64) -> String {
    let tenths = (secs.max(0.0) * 10.0).round() as u64;
    let (m, s, d) = (tenths / 600, tenths / 10 % 60, tenths % 10);
    let sep = crate::locale::decimal("0.5").chars().nth(1).unwrap_or('.');
    format!("{m}:{s:02}{sep}{d}")
}

impl App {
    /// The video's notes (in and out points, thumbnail frame).
    fn video_meta(&self, id: usize) -> VideoMeta {
        self.meta(id).map(|m| m.video).unwrap_or_default()
    }

    fn set_video_meta(&mut self, id: usize, v: VideoMeta) {
        let Some(key) = self.photos.get(id).map(|p| p.key.clone()) else { return };
        if self.store.set_video(&key, v) {
            // the thumbnail shows another frame: made again
            self.lib.forget_thumbs(vec![key]);
        }
        if let Some(p) = self.player.as_mut().filter(|p| p.id == id) {
            p.range = v.range(p.duration());
        }
        self.dirty = true;
    }

    /// The player for the video open in the editor, started at its in point.
    fn editor_player(&mut self, id: usize) {
        let key = self.photos[id].key.clone();
        if self.player.as_ref().is_some_and(|p| p.id == id && p.key == key) {
            return;
        }
        let start = self.video_meta(id).start.unwrap_or(0.0);
        let mut p = Player::open(self.lib.as_ref(), id, &key, start, EDIT_MAX_DIM);
        p.set_muted(self.store.video_muted);
        self.player = Some(p);
        self.strip = None;
    }

    /// Keys of the video editor: Space plays and pauses, `[` and `]` set the in and out
    /// points (typed characters, so they work on every keyboard layout), `,` and `.` step a
    /// frame, M turns the sound off and on, Home goes to the in point.
    pub(crate) fn video_keys(&mut self, ctx: &egui::Context) {
        if self.dialog.is_some() || self.text_focused() {
            return;
        }
        let id = self.edit.id;
        let now = ctx.input(|i| i.time);
        let (space, home, texts) = ctx.input(|i| {
            let texts: Vec<String> = i.events.iter().filter_map(|e| if let egui::Event::Text(t) = e { Some(t.clone()) } else { None }).collect();
            (i.key_pressed(egui::Key::Space), i.key_pressed(egui::Key::Home), texts)
        });
        let Some(p) = self.player.as_mut().filter(|p| p.id == id) else { return };
        if space {
            p.toggle(now);
        }
        if home {
            let from = p.range.0;
            p.seek(from, now);
        }
        let pos = p.position(now);
        for t in texts {
            match t.as_str() {
                "[" => self.set_in_point(id, pos),
                "]" => self.set_out_point(id, pos),
                "," => self.player.as_mut().map_or((), |p| p.step(-1, now)),
                "." => self.player.as_mut().map_or((), |p| p.step(1, now)),
                "m" | "M" => self.toggle_video_sound(),
                _ => {}
            }
        }
    }

    fn set_in_point(&mut self, id: usize, at: f64) {
        let mut v = self.video_meta(id);
        v.start = Some(at);
        if v.end.is_some_and(|e| e <= at) {
            v.end = None;
        }
        self.set_video_meta(id, v);
    }

    fn set_out_point(&mut self, id: usize, at: f64) {
        let mut v = self.video_meta(id);
        v.end = Some(at);
        if v.start.is_some_and(|s| s >= at) {
            v.start = None;
        }
        self.set_video_meta(id, v);
    }

    fn toggle_video_sound(&mut self) {
        self.store.video_muted = !self.store.video_muted;
        self.dirty = true;
        let m = self.store.video_muted;
        if let Some(p) = self.player.as_mut() {
            p.set_muted(m);
        }
    }

    /// The left panel: the part played and the thumbnail's frame.
    pub(crate) fn video_panel(&mut self, ui: &mut egui::Ui) {
        let id = self.edit.id;
        self.editor_player(id);
        let now = ui.input(|i| i.time);
        egui::Panel::left("edit_tools")
            .exact_size(260.0)
            .frame(egui::Frame::new().fill(t().left_bg).inner_margin(egui::Margin::same(8)).stroke(Stroke::new(1.0, t().border)))
            .show(ui, |ui| {
                crate::theme::panel_gradient(ui, egui::Margin::same(8), t().side_top, t().side_bottom);
                ui.label(RichText::new(format!("{}  {}", icons::MOVIE, tr!("video-panel"))).strong().size(15.0));
                let (info, pos) = match self.player.as_ref() {
                    Some(p) => (p.info.clone(), p.position(now)),
                    None => (None, 0.0),
                };
                if let Some(i) = &info {
                    let (width, height, codec) = (i.width, i.height, i.codec.to_uppercase());
                    let fps = crate::locale::decimal(&format!("{:.0}", i.fps));
                    ui.label(RichText::new(tr!("video-details", width = width, height = height, fps = fps, codec = codec)).color(t().text_weak).size(12.0));
                    if !i.has_audio {
                        ui.label(RichText::new(tr!("video-no-sound")).color(t().text_weak).size(12.0));
                    }
                }
                ui.add_space(10.0);
                let v = self.video_meta(id);
                let duration = info.as_ref().map(|i| i.duration).unwrap_or(0.0);
                ui.label(RichText::new(tr!("video-part")).strong());
                let (from, to) = v.range(duration);
                let (from, to) = (fmt_time(from), fmt_time(to));
                ui.label(tr!("video-plays", from = from, to = to));
                ui.horizontal(|ui| {
                    if ui.button(format!("[  {}", tr!("video-set-start"))).on_hover_text(tr!("video-set-start-hint")).clicked() {
                        self.set_in_point(id, pos);
                    }
                    if ui.button(format!("{}  ]", tr!("video-set-end"))).on_hover_text(tr!("video-set-end-hint")).clicked() {
                        self.set_out_point(id, pos);
                    }
                });
                if ui.add_enabled(v.start.is_some() || v.end.is_some(), egui::Button::new(tr!("video-whole"))).clicked() {
                    self.set_video_meta(id, VideoMeta { start: None, end: None, ..v });
                }
                ui.add_space(14.0);
                ui.label(RichText::new(tr!("video-thumbnail")).strong());
                let shown = self.player.as_ref().map(|p| p.shown_pts).unwrap_or(0.0);
                if ui.button(tr!("video-use-frame")).on_hover_text(tr!("video-use-frame-hint")).clicked() {
                    self.set_video_meta(id, VideoMeta { poster: Some(shown), ..v });
                }
                if ui.add_enabled(v.poster.is_some(), egui::Button::new(tr!("video-default-frame"))).clicked() {
                    self.set_video_meta(id, VideoMeta { poster: None, ..v });
                }
                ui.add_space(14.0);
                ui.label(RichText::new(tr!("video-keys")).color(t().text_weak).size(12.0));
            });
    }

    /// Above the editor's bottom bar: play / pause, the time, the sound, and the filmstrip.
    pub(crate) fn video_bar(&mut self, ui: &mut egui::Ui) {
        let id = self.edit.id;
        let now = ui.input(|i| i.time);
        egui::Panel::bottom("video_bar")
            .frame(egui::Frame::new().fill(t().tray_bg).inner_margin(egui::Margin::symmetric(8, 6)).stroke(Stroke::new(1.0, t().border)))
            .show(ui, |ui| {
                let Some(p) = self.player.as_ref().filter(|p| p.id == id) else { return };
                let (playing, pos, duration, has_audio) = (p.is_playing(), p.position(now), p.duration(), p.info.as_ref().is_some_and(|i| i.has_audio));
                ui.horizontal(|ui| {
                    let (icon, hint) = if playing { (icons::PAUSE, tr!("video-pause")) } else { (icons::PLAY, tr!("video-play")) };
                    if ui.add_sized([40.0, 30.0], egui::Button::new(RichText::new(icon).size(20.0))).on_hover_text(hint).clicked()
                        && let Some(p) = self.player.as_mut()
                    {
                        p.toggle(now);
                    }
                    ui.label(RichText::new(format!("{}  /  {}", fmt_time(pos), fmt_time(duration))).monospace());
                    if has_audio {
                        let muted = self.store.video_muted;
                        let text = if muted { tr!("video-sound-off") } else { tr!("video-sound-on") };
                        if ui.add(egui::Button::new(format!("{}  {text}", icons::MUSIC)).selected(!muted)).on_hover_text(tr!("video-sound-hint")).clicked() {
                            self.toggle_video_sound();
                        }
                    }
                });
                ui.add_space(4.0);
                self.filmstrip(ui, id, now);
            });
    }

    /// The filmstrip: frames over the whole video, the part outside the in and out points
    /// dimmed, the play head. A click or a drag moves the play head; dragging an in or out
    /// mark moves it.
    fn filmstrip(&mut self, ui: &mut egui::Ui, id: usize, now: f64) {
        let width = ui.available_width();
        let (rect, resp) = ui.allocate_exact_size(Vec2::new(width, STRIP_H), Sense::click_and_drag());
        let Some(p) = self.player.as_ref().filter(|p| p.id == id) else { return };
        let (duration, Some(size)) = (p.duration(), p.shown_size()) else {
            ui.painter().rect_filled(rect, 4.0, t().placeholder);
            return;
        };
        if duration <= 0.0 {
            return;
        }
        let aspect = size[0] as f32 / size[1].max(1) as f32;
        let cell = (STRIP_H * aspect).clamp(24.0, 160.0);
        let count = ((rect.width() / cell).ceil() as usize).clamp(2, 60);
        if self.strip.as_ref().is_none_or(|s| s.key != p.key || s.frames.len() != count) {
            let ppp = ui.ctx().pixels_per_point();
            let long = (STRIP_H * ppp * aspect.max(1.0 / aspect.max(0.01))).ceil() as u32;
            self.strip = Some(Filmstrip::start(self.lib.as_ref(), &p.key, duration, count, long.max(16)));
        }
        let x_of = |secs: f64| rect.left() + (secs / duration).clamp(0.0, 1.0) as f32 * rect.width();
        let secs_of = |x: f32| f64::from(((x - rect.left()) / rect.width()).clamp(0.0, 1.0)) * duration;
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 4.0, Color32::BLACK);
        if let Some(strip) = &self.strip {
            let w = rect.width() / count as f32;
            for (i, f) in strip.frames.iter().enumerate() {
                let cell_rect = Rect::from_min_size(Pos2::new(rect.left() + i as f32 * w, rect.top()), Vec2::new(w, STRIP_H));
                match f {
                    Some(tex) => {
                        // cut to the cell's shape, centred
                        let ts = tex.size_vec2();
                        let (tw, th) = (ts.x.max(1.0), ts.y.max(1.0));
                        let (cw, ch) = (cell_rect.width(), cell_rect.height());
                        let (u, v) = if tw / th > cw / ch { ((cw / ch) / (tw / th), 1.0) } else { (1.0, (tw / th) / (cw / ch)) };
                        let uv = Rect::from_center_size(Pos2::new(0.5, 0.5), Vec2::new(u, v));
                        painter.image(tex.id(), cell_rect.shrink2(Vec2::new(0.5, 0.0)), uv, Color32::WHITE);
                    }
                    None => {
                        painter.rect_filled(cell_rect.shrink(1.0), 2.0, Color32::from_gray(40));
                    }
                }
            }
        }
        let (from, to) = self.video_meta(id).range(duration);
        let (xa, xb) = (x_of(from), x_of(to));
        let dim = Color32::from_black_alpha(170);
        painter.rect_filled(Rect::from_min_max(rect.left_top(), Pos2::new(xa, rect.bottom())), 0.0, dim);
        painter.rect_filled(Rect::from_min_max(Pos2::new(xb, rect.top()), rect.right_bottom()), 0.0, dim);
        // the in and out marks: yellow brackets with a dark edge, seen on any frame (drawn on
        // the pictures, so the same in both themes)
        for (x, dir) in [(xa, 1.0), (xb, -1.0)] {
            let bracket = |s: Stroke| {
                let (top, bottom) = (rect.top() + 2.0, rect.bottom() - 2.0);
                painter.add(egui::Shape::line(vec![Pos2::new(x + 8.0 * dir, top), Pos2::new(x, top), Pos2::new(x, bottom), Pos2::new(x + 8.0 * dir, bottom)], s));
            };
            bracket(Stroke::new(5.0, Color32::from_black_alpha(160)));
            bracket(Stroke::new(3.0, Color32::from_rgb(255, 204, 0)));
        }
        // the thumbnail's frame: a small mark at the top
        if let Some(poster) = self.video_meta(id).poster {
            let x = x_of(poster);
            painter.add(egui::Shape::convex_polygon(
                vec![Pos2::new(x - 5.0, rect.top()), Pos2::new(x + 5.0, rect.top()), Pos2::new(x, rect.top() + 6.0)],
                Color32::WHITE,
                Stroke::NONE,
            ));
        }
        let head = x_of(p.position(now));
        painter.line_segment([Pos2::new(head, rect.top()), Pos2::new(head, rect.bottom())], Stroke::new(2.0, Color32::WHITE));
        painter.circle_filled(Pos2::new(head, rect.top() + 3.0), 4.0, Color32::WHITE);
        painter.rect_stroke(rect, 4.0, Stroke::new(1.0, t().border), egui::StrokeKind::Inside);

        // what the mouse does: a drag that starts on a mark moves it, any other moves the play head
        let grab = egui::Id::unique("video_strip_grab");
        if resp.drag_started()
            && let Some(pt) = resp.interact_pointer_pos()
        {
            let which = if (pt.x - xa).abs() < 7.0 { 1u8 } else if (pt.x - xb).abs() < 7.0 { 2 } else { 0 };
            ui.ctx().data_mut(|d| d.insert_temp(grab, which));
        }
        let which: u8 = ui.ctx().data(|d| d.get_temp(grab)).unwrap_or(0);
        if (resp.dragged() || resp.clicked())
            && let Some(pt) = resp.interact_pointer_pos()
        {
            let at = secs_of(pt.x);
            match which {
                1 if resp.dragged() => self.set_in_point(id, at),
                2 if resp.dragged() => self.set_out_point(id, at),
                _ => {}
            }
            if let Some(p) = self.player.as_mut() {
                // a drag shows the frames as it goes; several seeks between two frames are one
                if (p.shown_pts - at).abs() > 0.02 || resp.clicked() {
                    p.seek(at, now);
                }
            }
        }
        if resp.drag_stopped() {
            ui.ctx().data_mut(|d| d.remove::<u8>(grab));
        }
        if resp.hovered() {
            let near = resp.hover_pos().is_some_and(|pt| (pt.x - xa).abs() < 7.0 || (pt.x - xb).abs() < 7.0);
            ui.ctx().set_cursor_icon(if near { egui::CursorIcon::ResizeHorizontal } else { egui::CursorIcon::PointingHand });
        }
    }

    /// The video itself, fitted in the middle; a click plays or pauses.
    pub(crate) fn video_canvas(&mut self, ui: &mut egui::Ui) {
        let id = self.edit.id;
        let now = ui.input(|i| i.time);
        egui::CentralPanel::default().frame(egui::Frame::new().fill(t().edit_bg)).show(ui, |ui| {
            let area = ui.available_rect_before_wrap().shrink(18.0);
            let ctx = ui.ctx().clone();
            let Some(p) = self.player.as_mut().filter(|p| p.id == id) else { return };
            p.poll(&ctx, now);
            if p.failed {
                ui.painter().text(area.center(), egui::Align2::CENTER_CENTER, tr!("video-unreadable"), egui::FontId::proportional(15.0), Color32::WHITE);
                return;
            }
            let (Some(size), true) = (p.shown_size(), p.has_frame()) else {
                ui.put(Rect::from_center_size(area.center(), Vec2::splat(40.0)), egui::Spinner::new().size(32.0));
                return;
            };
            let r = crate::edit_view::fit_rect(area, size);
            p.paint(ui.painter(), r, Color32::WHITE);
            let resp = ui.interact(r, ui.scope_id().with("video_canvas"), Sense::click());
            if resp.clicked() {
                p.toggle(now);
            }
            if !p.is_playing() {
                // paused: a play mark in the middle
                let c = r.center();
                ui.painter().circle_filled(c, 30.0, Color32::from_black_alpha(110));
                ui.painter().add(egui::Shape::convex_polygon(
                    vec![Pos2::new(c.x - 9.0, c.y - 14.0), Pos2::new(c.x + 15.0, c.y), Pos2::new(c.x - 9.0, c.y + 14.0)],
                    Color32::WHITE,
                    Stroke::NONE,
                ));
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn times_with_tenths() {
        assert_eq!(fmt_time(0.0), "0:00.0");
        assert_eq!(fmt_time(3.24), "0:03.2");
        assert_eq!(fmt_time(65.96), "1:06.0");
    }
}
