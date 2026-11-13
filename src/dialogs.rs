//! The dialogs (`App.dialog`: one at a time, modal; Esc or a click beside it cancels, Enter
//! presses the main button of the simple ones) and the notices (`App.notices`).

use crate::app::*;
use crate::i18n::tr;
use crate::icons;
use crate::library::Album;
use eframe::egui::{self, Color32};

impl App {
    pub(crate) fn dialogs(&mut self, ctx: &egui::Context) {
        // Settings is a larger window of its own
        if let Some(Dialog::Settings { section }) = self.dialog {
            self.dialog = None;
            self.settings_dialog(ctx, section);
            return;
        }
        let Some(mut dialog) = self.dialog.take() else { return };
        let mut close = false;
        // Enter presses the main button of the dialogs that only ask for a name or an OK
        let enter = ctx.input(|i| i.key_pressed(egui::Key::Enter));
        let title = match &dialog {
            Dialog::Info { title, .. } => title.clone(),
            Dialog::NewAlbum { .. } => tr!("dialog-new-album"),
            Dialog::RenameAlbum { .. } => tr!("dialog-rename-album"),
            Dialog::Tag { .. } => tr!("dialog-tags"),
            Dialog::Gallery { .. } => tr!("dialog-gallery"),
            Dialog::FolderNote { .. } => tr!("dialog-folder-texts"),
            Dialog::Rename { folder: true, .. } => tr!("dialog-rename-folder"),
            Dialog::Rename { .. } => tr!("dialog-rename-picture"),
            Dialog::MoveTo { .. } => tr!("dialog-move"),
            Dialog::Movie { .. } => tr!("dialog-movie"),
            Dialog::About => tr!("dialog-about"),
            Dialog::OnlineMaps => tr!("dialog-online-maps"),
            Dialog::PrintReview { .. } => tr!("dialog-print-review"),
            Dialog::Trash { .. } | Dialog::TrashFolder { .. } => tr!("dialog-trash"),
            Dialog::DeleteForever { .. } => tr!("dialog-delete-forever"),
            Dialog::Settings { .. } => tr!("settings-title"),
        };
        let targets = self.targets();
        #[allow(deprecated)]
        let modal_id = egui::Id::new("dialog");
        let modal = egui::Modal::new(modal_id).show(ctx, |ui| {
            ui.set_width(400.0);
            ui.heading(title);
            ui.add_space(6.0);
            match &mut dialog {
                Dialog::Info { text, .. } => {
                    ui.label(text.as_str());
                    ui.add_space(8.0);
                    if ui.button(tr!("button-ok")).clicked() || enter {
                        close = true;
                    }
                }
                Dialog::NewAlbum { name } => {
                    ui.label(tr!("new-album-name"));
                    let r = ui.text_edit_singleline(name);
                    r.request_focus();
                    if !targets.is_empty() {
                        ui.label(tr!("new-album-adds", n = targets.len()));
                    }
                    ui.horizontal(|ui| {
                        if (ui.button(tr!("new-album-create")).clicked() || enter) && !name.trim().is_empty() {
                            self.store.albums.push(Album { name: name.trim().to_string(), keys: vec![] });
                            let idx = self.store.albums.len() - 1;
                            self.dirty = true;
                            if !targets.is_empty() {
                                self.add_to_album(idx, &targets);
                            }
                            self.source = Source::Album(idx);
                            close = true;
                        }
                        if ui.button(tr!("button-cancel")).clicked() {
                            close = true;
                        }
                    });
                }
                Dialog::RenameAlbum { idx, name } => {
                    ui.text_edit_singleline(name);
                    ui.horizontal(|ui| {
                        if (ui.button(tr!("rename-album-rename")).clicked() || enter) && !name.trim().is_empty() {
                            if let Some(a) = self.store.albums.get_mut(*idx) {
                                a.name = name.trim().to_string();
                                self.dirty = true;
                            }
                            close = true;
                        }
                        if ui.button(tr!("button-cancel")).clicked() {
                            close = true;
                        }
                    });
                }
                Dialog::Tag { tags, people } => {
                    ui.label(tr!("tags-intro", n = targets.len()));
                    egui::Grid::new("tagdlg").num_columns(2).show(ui, |ui| {
                        ui.label(tr!("tags-tags"));
                        ui.text_edit_singleline(tags);
                        ui.end_row();
                        ui.label(tr!("tags-people"));
                        ui.text_edit_singleline(people);
                        ui.end_row();
                    });
                    // proposed tags: a click puts one in the field
                    let (proposed, not) = self.proposal_chips(ui, &targets);
                    if let Some(tag) = proposed {
                        let have = tags.split(',').any(|t| t.trim().eq_ignore_ascii_case(&tag));
                        if !have {
                            if !tags.trim().is_empty() {
                                tags.push_str(", ");
                            }
                            tags.push_str(&tag);
                        }
                    }
                    if let Some(tag) = not {
                        self.not_this_tag(&targets, &tag);
                    }
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        if ui.button(tr!("tags-add")).clicked() || enter {
                            let split = |s: &str| -> Vec<String> {
                                s.split(',').map(|t| t.trim().to_string()).filter(|t| !t.is_empty()).collect()
                            };
                            let (t, p) = (split(tags), split(people));
                            for &id in &targets {
                                let m = self.meta_mut(id);
                                for x in &t {
                                    if !m.tags.contains(x) {
                                        m.tags.push(x.clone());
                                    }
                                    m.not_tags.retain(|n| !n.eq_ignore_ascii_case(x));
                                }
                                for x in &p {
                                    if !m.people.contains(x) {
                                        m.people.push(x.clone());
                                    }
                                }
                            }
                            close = true;
                        }
                        if ui.button(tr!("button-cancel")).clicked() {
                            close = true;
                        }
                    });
                }
                Dialog::Gallery { title } => {
                    ui.set_width(CARD_W * 4.0 + 3.0 * 8.0);
                    ui.label(tr!("gallery-intro", n = targets.len()));
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        ui.label(tr!("gallery-title"));
                        ui.text_edit_singleline(title);
                    });
                    // the themes as small pages made of the tray's own thumbnails
                    let pics: Vec<egui::TextureHandle> = targets.iter().filter_map(|id| self.thumbs.get(id).cloned()).take(12).collect();
                    let before = self.store.gallery;
                    let g = &mut self.store.gallery;
                    ui.add_space(4.0);
                    ui.label(tr!("gallery-theme"));
                    ui.spacing_mut().item_spacing = egui::vec2(8.0, 8.0);
                    for row in crate::gallery::Theme::ALL.chunks(4) {
                        ui.horizontal(|ui| {
                            for &th in row {
                                if theme_card(ui, th, g.layout, &pics, g.theme == th).clicked() {
                                    g.theme = th;
                                }
                            }
                        });
                    }
                    ui.spacing_mut().item_spacing = egui::vec2(8.0, 4.0);
                    ui.horizontal(|ui| {
                        ui.label(tr!("gallery-layout"));
                        for l in crate::gallery::Layout::ALL {
                            ui.radio_value(&mut g.layout, l, crate::labels::gallery_layout(&l));
                        }
                    });
                    ui.label(tr!("gallery-size"));
                    ui.radio_value(&mut g.size, 1280, tr!("gallery-size-medium"));
                    ui.radio_value(&mut g.size, 2048, tr!("gallery-size-large"));
                    ui.checkbox(&mut g.captions, tr!("gallery-captions"));
                    if *g != before {
                        self.dirty = true;
                    }
                    ui.horizontal(|ui| {
                        if ui.add_enabled(!targets.is_empty(), egui::Button::new(tr!("gallery-make"))).clicked() {
                            let opts = crate::gallery::Options::new(title.clone(), &self.store.gallery);
                            self.make_gallery(targets.clone(), opts);
                            close = true;
                        }
                        if ui.button(tr!("button-cancel")).clicked() {
                            close = true;
                        }
                    });
                }
                Dialog::FolderNote { path, caption, text } => {
                    ui.label(tr!("folder-caption-label"));
                    ui.add(egui::TextEdit::singleline(caption).desired_width(f32::INFINITY));
                    ui.add_space(4.0);
                    ui.label(tr!("folder-description-label"));
                    ui.add(egui::TextEdit::multiline(text).desired_width(f32::INFINITY));
                    ui.horizontal(|ui| {
                        if ui.button(tr!("button-save")).clicked() {
                            crate::commands::set_folder_text(&mut self.store, path, Some(caption.as_str()), Some(text.as_str()));
                            self.dirty = true;
                            self.list_rev += 1;
                            close = true;
                        }
                        if ui.button(tr!("button-cancel")).clicked() {
                            close = true;
                        }
                    });
                }
                Dialog::Rename { path, folder, name, error } => {
                    let path = path.clone();
                    close = self.rename_dialog(ui, &path, *folder, name, error, enter);
                }
                Dialog::MoveTo { what, filter, error } => {
                    let what = what.clone();
                    close = self.move_dialog(ui, &what, filter, error);
                }
                Dialog::Movie { hd } => {
                    let list = if targets.len() > 1 { targets.clone() } else { self.visible_flat() };
                    let ffmpeg = *self.ffmpeg.get_or_insert_with(crate::movie::ffmpeg_available);
                    let st = self.store.slideshow.clone();
                    let yes_no = |b: bool| if b { "yes" } else { "no" };
                    ui.label(tr!("movie-intro", n = list.len(), seconds = st.interval.round(), fade = yes_no(st.fade), motion = yes_no(st.motion)));
                    ui.label(egui::RichText::new(tr!("movie-length", seconds = crate::movie::duration(list.len(), &st).round())).weak());
                    ui.add_space(4.0);
                    ui.radio_value(hd, false, "720p (1280 × 720)");
                    ui.radio_value(hd, true, "1080p (1920 × 1080)");
                    let s = &mut self.store.slideshow;
                    ui.horizontal(|ui| {
                        if s.music.is_some() {
                            if ui.checkbox(&mut s.music_on, tr!("movie-music")).changed() {
                                self.dirty = true;
                            }
                            let name = s.music.as_deref().and_then(|m| std::path::Path::new(m).file_name()).map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
                            ui.label(egui::RichText::new(name).weak());
                        }
                        if ui.button(if s.music.is_some() { tr!("movie-other-music") } else { tr!("movie-add-music") }).clicked() && crate::slideshow_view::pick_music(s) {
                            self.dirty = true;
                        }
                    });
                    if !ffmpeg {
                        ui.add_space(6.0);
                        ui.label(egui::RichText::new(tr!("movie-no-ffmpeg")).color(crate::theme::t().reject));
                        ui.label(tr!("movie-install-ffmpeg"));
                        ui.hyperlink_to("ffmpeg.org/download", "https://ffmpeg.org/download.html");
                    }
                    ui.horizontal(|ui| {
                        if ui.add_enabled(ffmpeg && !list.is_empty(), egui::Button::new(tr!("movie-make"))).clicked() {
                            let size = if *hd { (1920, 1080) } else { (1280, 720) };
                            self.make_movie(list.clone(), size);
                            close = true;
                        }
                        if ui.button(format!("{} {}", icons::PLAY, tr!("movie-preview"))).clicked() {
                            let first = list.first().copied().unwrap_or(0);
                            self.start_slideshow(list.clone(), first);
                            close = true;
                        }
                        if ui.button(tr!("button-cancel")).clicked() {
                            close = true;
                        }
                    });
                }
                Dialog::Trash { keys, bytes, rejects } => {
                    let n = keys.len();
                    let (count, size) = (crate::app::fmt_count(n), crate::library::fmt_bytes(*bytes));
                    let question = if *rejects { tr!("trash-question", n = n, count = count, size = size) } else { tr!("trash-question-chosen", n = n, count = count, size = size) };
                    ui.label(format!("{question}\n\n{}", tr!("trash-explain")));
                    ui.add_space(8.0);
                    // no Enter here: this one removes files
                    ui.horizontal(|ui| {
                        if ui.button(tr!("trash-move")).clicked() {
                            let keys = std::mem::take(keys);
                            self.trash_files(keys);
                            close = true;
                        }
                        if ui.button(tr!("button-cancel")).clicked() {
                            close = true;
                        }
                    });
                }
                Dialog::TrashFolder { path, pictures, bytes } => {
                    let name = std::path::Path::new(path.as_str()).file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| path.clone());
                    let (n, count, size) = (*pictures, crate::app::fmt_count(*pictures), crate::library::fmt_bytes(*bytes));
                    ui.label(format!("{}\n\n{}", tr!("trash-folder-question", name = name, n = n, count = count, size = size), tr!("trash-explain")));
                    ui.add_space(8.0);
                    // no Enter here: this one removes files
                    ui.horizontal(|ui| {
                        if ui.button(tr!("trash-move")).clicked() {
                            let path = std::mem::take(path);
                            self.trash_folder(path);
                            close = true;
                        }
                        if ui.button(tr!("button-cancel")).clicked() {
                            close = true;
                        }
                    });
                }
                Dialog::DeleteForever { paths } => {
                    let n = paths.len();
                    ui.label(tr!("delete-forever-question", n = n));
                    ui.add_space(4.0);
                    for p in paths.iter().take(6) {
                        ui.label(egui::RichText::new(format!("• {p}")).weak());
                    }
                    if n > 6 {
                        let more = n - 6;
                        ui.label(egui::RichText::new(tr!("files-more-problems", n = more)).weak());
                    }
                    ui.add_space(8.0);
                    // no Enter here: nothing brings these back
                    ui.horizontal(|ui| {
                        if ui.button(egui::RichText::new(tr!("delete-forever-button")).color(crate::theme::t().reject)).clicked() {
                            let paths = std::mem::take(paths);
                            self.delete_forever(paths);
                            close = true;
                        }
                        if ui.button(tr!("delete-forever-keep")).clicked() {
                            close = true;
                        }
                    });
                }
                Dialog::PrintReview { grades, before_print } => {
                    let bad: Vec<usize> = grades.iter().filter(|g| g.3 == 0).map(|g| g.0).collect();
                    ui.label(if bad.is_empty() { tr!("print-ready") } else { tr!("print-too-small") });
                    ui.add_space(6.0);
                    egui::ScrollArea::vertical().max_height(260.0).show(ui, |ui| {
                        egui::Grid::new("print_review").num_columns(2).striped(true).spacing([12.0, 3.0]).show(ui, |ui| {
                            for (_, name, ppi, grade) in grades.iter() {
                                ui.label(name.as_str());
                                let colour = match grade {
                                    2 => t().good,
                                    1 => t().text_weak,
                                    _ => t().bad,
                                };
                                ui.label(egui::RichText::new(crate::labels::print_quality(*grade, *ppi)).color(colour));
                                ui.end_row();
                            }
                        });
                    });
                    ui.add_space(6.0);
                    let before = *before_print;
                    ui.horizontal(|ui| {
                        if before && !bad.is_empty() {
                            if ui.button(tr!("print-review-remove")).clicked() {
                                close = true;
                                self.print_remove_bad(&bad);
                            }
                            if ui.button(tr!("print-review-print")).clicked() {
                                close = true;
                                self.start_print_job();
                            }
                            if ui.button(tr!("button-cancel")).clicked() {
                                close = true;
                            }
                        } else if ui.button(tr!("button-ok")).clicked() {
                            close = true;
                        }
                    });
                }
                Dialog::About => {
                    ui.heading("AlbumCraft");
                    ui.label(tr!("about-text"));
                    ui.label(tr!("about-edits"));
                    ui.add_space(6.0);
                    ui.label(egui::RichText::new(tr!("about-credits")).size(11.5).weak());
                    if ui.button(tr!("button-ok")).clicked() || enter {
                        close = true;
                    }
                }
                Dialog::OnlineMaps => {
                    ui.label(egui::RichText::new(tr!("online-maps-question")).strong().size(15.0));
                    ui.add_space(4.0);
                    ui.label(tr!("online-maps-what"));
                    ui.add_space(4.0);
                    ui.label(tr!("online-maps-privacy"));
                    ui.add_space(4.0);
                    ui.label(tr!("online-maps-offline"));
                    ui.add_space(4.0);
                    ui.label(egui::RichText::new(tr!("online-maps-later")).size(12.0).weak());
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        if ui.add(egui::Button::new(egui::RichText::new(tr!("online-maps-use")).color(Color32::WHITE).strong()).fill(t().accent)).clicked() {
                            self.store.online_maps = Some(true);
                            self.dirty = true;
                            close = true;
                        }
                        if ui.button(tr!("online-maps-builtin")).clicked() {
                            self.store.online_maps = Some(false);
                            self.dirty = true;
                            close = true;
                        }
                    });
                }
                // drawn by `settings_dialog`, before this one
                Dialog::Settings { .. } => close = true,
            }
        });
        // Esc, or a click beside the dialog: the same as Cancel
        if !close && !modal.should_close() {
            self.dialog = Some(dialog);
        }
    }

    /// The notices, stacked over the bottom of the window, newest lowest: short notes fade by
    /// themselves, errors stay until closed, some offer "Undo" or "Show in folder".
    pub(crate) fn toast_ui(&mut self, ctx: &egui::Context) {
        if self.notices.is_empty() {
            return;
        }
        let now = ctx.input(|i| i.time);
        let mut close: Option<usize> = None;
        let mut act: Option<usize> = None;
        let mut hovered: Option<usize> = None;
        #[allow(deprecated)]
        let id = egui::Id::new("notices");
        egui::Area::new(id).order(egui::Order::Foreground).anchor(egui::Align2::CENTER_BOTTOM, [0.0, -150.0]).show(ctx, |ui| {
            ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                for (i, n) in self.notices.iter().enumerate() {
                    let fill = if n.error { Color32::from_rgba_unmultiplied(92, 34, 34, 245) } else { Color32::from_rgba_unmultiplied(40, 44, 52, 240) };
                    let frame = egui::Frame::new().fill(fill).corner_radius(6).inner_margin(egui::Margin::symmetric(14, 8)).show(ui, |ui| {
                        ui.set_max_width((ctx.content_rect().width() - 60.0).clamp(200.0, 560.0));
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new(n.text.as_str()).color(Color32::WHITE));
                            let label = match &n.action {
                                Some(NoticeAction::Undo(_)) => Some(format!("{} {}", icons::UNDO, tr!("notice-undo"))),
                                Some(NoticeAction::Show(_)) => Some(format!("{} {}", icons::FOLDER, tr!("notice-show"))),
                                None => None,
                            };
                            if let Some(label) = label
                                && ui.button(label).clicked()
                            {
                                act = Some(i);
                            }
                            // short plain notes need no button: they are gone in a moment
                            if (n.life.is_none() || n.action.is_some()) && ui.small_button(icons::CLOSE).on_hover_text(tr!("notice-close")).clicked() {
                                close = Some(i);
                            }
                        });
                    });
                    if frame.response.contains_pointer() {
                        hovered = Some(i);
                    }
                    ui.add_space(4.0);
                }
            });
        });
        if let Some(i) = act {
            match self.notices.remove(i).action {
                Some(NoticeAction::Undo(undo)) => self.undo_action(undo),
                Some(NoticeAction::Show(path)) => {
                    if let Err(e) = crate::library_view::reveal_in_file_manager(&path) {
                        self.toast(tr!("notice-no-file-manager", error = e.to_string()));
                    }
                }
                None => {}
            }
        } else if let Some(i) = close {
            self.notices.remove(i);
        } else {
            // a notice under the mouse pointer waits
            let mut i = 0;
            self.notices.retain(|n| {
                let keep = hovered == Some(i) || n.life.is_none_or(|life| now - n.born <= life);
                i += 1;
                keep
            });
        }
        if self.notices.iter().any(|n| n.life.is_some()) {
            ctx.request_repaint_after(std::time::Duration::from_millis(250));
        }
    }
}

const CARD_W: f32 = 108.0;

/// A gallery theme drawn as a small page (title, count and tiles in the chosen layout, with the
/// given thumbnails or plain shapes), its name below; a click picks it.
fn theme_card(ui: &mut egui::Ui, theme: crate::gallery::Theme, layout: crate::gallery::Layout, pics: &[egui::TextureHandle], selected: bool) -> egui::Response {
    use crate::gallery::Layout;
    use egui::{pos2, vec2, Rect, Stroke};
    let rgb = |c: u32| Color32::from_rgb((c >> 16) as u8, (c >> 8) as u8, c as u8);
    let look = theme.look();
    let name = crate::labels::gallery_theme(&theme);
    let (rect, resp) = ui.allocate_exact_size(vec2(CARD_W, 96.0), egui::Sense::click());
    let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand);
    if !ui.is_rect_visible(rect) {
        return resp;
    }
    let vis = ui.visuals();
    let page = Rect::from_min_size(rect.min, vec2(CARD_W, 76.0));
    let p = ui.painter();
    p.rect_filled(page, 5, rgb(look.bg));
    let pp = p.with_clip_rect(page.shrink(1.0));
    // title and count, centred in the themes that centre them
    let centred = matches!(theme, crate::gallery::Theme::Museum | crate::gallery::Theme::Instant | crate::gallery::Theme::Vintage);
    let bar = |y: f32, w: f32, h: f32, c: u32| {
        let x = if centred { page.center().x - w / 2.0 } else { page.left() + 7.0 };
        pp.rect_filled(Rect::from_min_size(pos2(x, page.top() + y), vec2(w, h)), 1, rgb(c));
    };
    bar(6.0, 38.0, 4.0, if theme == crate::gallery::Theme::Blossom { look.accent } else { look.fg });
    bar(12.5, 20.0, 2.0, look.weak);
    let gap = (look.gap as f32 * 0.14).clamp(1.0, 5.0);
    let frame = look.frame as f32 * 0.3;
    let radius = look.radius as f32 * 0.3;
    let area = Rect::from_min_max(pos2(page.left() + 6.0, page.top() + 18.0), pos2(page.right() - 6.0, page.bottom() + 20.0));
    // aspect ratios of the pictures (or of made-up ones), cycled
    const MADE_UP: [f32; 6] = [1.5, 0.75, 1.33, 1.0, 1.5, 0.67];
    let aspect = |i: usize| match pics.get(i % pics.len().max(1)) {
        Some(t) => t.size()[0] as f32 / t.size()[1].max(1) as f32,
        None => MADE_UP[i % MADE_UP.len()],
    };
    let mut tiles: Vec<Rect> = vec![];
    match layout {
        Layout::Grid => {
            let c = (area.width() - 2.0 * gap) / 3.0;
            for i in 0..9 {
                let (col, row) = ((i % 3) as f32, (i / 3) as f32);
                tiles.push(Rect::from_min_size(area.min + vec2(col * (c + gap), row * (c + gap)), vec2(c, c)));
            }
        }
        Layout::Masonry => {
            let c = (area.width() - 2.0 * gap) / 3.0;
            let mut y = [0.0f32; 3];
            for i in 0..12 {
                let col = (0..3).min_by(|&a, &b| y[a].total_cmp(&y[b])).unwrap_or(0);
                let h = c / aspect(i) + 2.0 * frame;
                tiles.push(Rect::from_min_size(area.min + vec2(col as f32 * (c + gap), y[col]), vec2(c, h)));
                y[col] += h + gap;
            }
        }
        Layout::Rows => {
            let (h, mut i, mut y) = (17.0, 0, area.top());
            while y < area.bottom() && i < 30 {
                // as many as fit at this height, then stretched to the full width
                let mut row = vec![];
                let mut w = 0.0;
                while row.is_empty() || w + aspect(i) * h <= area.width() * 1.15 {
                    w += aspect(i) * h + if row.is_empty() { 0.0 } else { gap };
                    row.push(aspect(i));
                    i += 1;
                }
                let k = (area.width() - gap * (row.len() - 1) as f32) / (w - gap * (row.len() - 1) as f32);
                let mut x = area.left();
                for a in row {
                    tiles.push(Rect::from_min_size(pos2(x, y), vec2(a * h * k, h * k)));
                    x += a * h * k + gap;
                }
                y += h * k + gap;
            }
        }
    }
    let card = rgb(look.card);
    for (i, t) in tiles.into_iter().enumerate() {
        if look.shadow {
            pp.rect_filled(t.translate(vec2(0.0, 1.0)).expand(0.5), radius, Color32::from_black_alpha(28));
        }
        if look.frame > 0 || look.card != look.bg {
            pp.rect_filled(t, radius, card);
        }
        let bottom = if theme == crate::gallery::Theme::Instant { frame * 2.5 } else { frame };
        let r = Rect::from_min_max(t.min + vec2(frame, frame), t.max - vec2(frame, bottom));
        let pr = (look.pic_radius as f32 * 0.3).min(r.width() / 2.0);
        match pics.get(i % pics.len().max(1)) {
            Some(tex) => {
                // cover: crop the thumbnail to the tile's shape
                let [w, h] = tex.size();
                let (ta, ra) = (w as f32 / h.max(1) as f32, r.width() / r.height().max(1.0));
                let uv = if ta > ra {
                    let u = ra / ta;
                    Rect::from_min_max(pos2((1.0 - u) / 2.0, 0.0), pos2((1.0 + u) / 2.0, 1.0))
                } else {
                    let v = ta / ra;
                    Rect::from_min_max(pos2(0.0, (1.0 - v) / 2.0), pos2(1.0, (1.0 + v) / 2.0))
                };
                pp.add(egui::epaint::RectShape::filled(r, pr, Color32::WHITE).with_texture(tex.id(), uv));
            }
            None => {
                let shade = if crate::gallery::luma(look.bg) < 0.5 { Color32::from_white_alpha(40) } else { Color32::from_black_alpha(30) };
                pp.rect_filled(r, pr, rgb(look.accent).linear_multiply(0.45));
                pp.rect_filled(Rect::from_min_max(pos2(r.left(), r.center().y), r.max), pr, shade);
            }
        }
    }
    let stroke = if selected {
        Stroke::new(3.0, crate::theme::t().accent)
    } else if resp.hovered() {
        Stroke::new(1.5, vis.widgets.hovered.fg_stroke.color)
    } else {
        Stroke::new(1.0, vis.widgets.noninteractive.bg_stroke.color)
    };
    p.rect_stroke(page, 5, stroke, egui::StrokeKind::Outside);
    let text = if selected { egui::RichText::new(&name).strong() } else { egui::RichText::new(&name) };
    let galley = egui::WidgetText::from(text).into_galley(ui, Some(egui::TextWrapMode::Truncate), CARD_W, egui::TextStyle::Body);
    let color = if selected { vis.strong_text_color() } else { vis.text_color() };
    p.galley(pos2(rect.center().x - galley.size().x / 2.0, page.bottom() + 3.0), galley, color);
    resp.widget_info(|| egui::WidgetInfo::selected(egui::Role::RadioButton, true, selected, &name));
    resp
}
