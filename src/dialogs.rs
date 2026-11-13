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
            Dialog::FolderNote { .. } => tr!("dialog-folder-note"),
            Dialog::Movie { .. } => tr!("dialog-movie"),
            Dialog::About => tr!("dialog-about"),
            Dialog::OnlineMaps => tr!("dialog-online-maps"),
            Dialog::PrintReview { .. } => tr!("dialog-print-review"),
            Dialog::TrashRejects { .. } => tr!("dialog-trash"),
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
                Dialog::Gallery { title, style, size, captions } => {
                    ui.label(tr!("gallery-intro", n = targets.len()));
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        ui.label(tr!("gallery-title"));
                        ui.text_edit_singleline(title);
                    });
                    ui.label(tr!("gallery-style"));
                    for st in crate::gallery::Style::ALL {
                        ui.radio_value(style, st, crate::labels::gallery_style(&st));
                    }
                    ui.label(tr!("gallery-size"));
                    ui.radio_value(size, 1280, tr!("gallery-size-medium"));
                    ui.radio_value(size, 2048, tr!("gallery-size-large"));
                    ui.checkbox(captions, tr!("gallery-captions"));
                    ui.horizontal(|ui| {
                        if ui.add_enabled(!targets.is_empty(), egui::Button::new(tr!("gallery-make"))).clicked() {
                            let opts = crate::gallery::Options { title: title.clone(), style: *style, size: *size, captions: *captions };
                            self.make_gallery(targets.clone(), opts);
                            close = true;
                        }
                        if ui.button(tr!("button-cancel")).clicked() {
                            close = true;
                        }
                    });
                }
                Dialog::FolderNote { path, text } => {
                    ui.label(tr!("folder-note-intro"));
                    ui.text_edit_multiline(text);
                    ui.horizontal(|ui| {
                        if ui.button(tr!("button-save")).clicked() {
                            if text.trim().is_empty() {
                                self.store.folder_notes.remove(path.as_str());
                            } else {
                                self.store.folder_notes.insert(path.clone(), text.trim().to_string());
                            }
                            self.dirty = true;
                            close = true;
                        }
                        if ui.button(tr!("button-cancel")).clicked() {
                            close = true;
                        }
                    });
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
                Dialog::TrashRejects { keys, bytes } => {
                    let n = keys.len();
                    let question = tr!("trash-question", n = n, count = crate::app::fmt_count(n), size = crate::library::fmt_bytes(*bytes));
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
