//! The dialogs (`App.dialog`: one at a time, modal; Esc or a click beside it cancels, Enter
//! presses the main button of the simple ones) and the notices (`App.notices`).

use crate::app::*;
use crate::icons;
use crate::library::Album;
use eframe::egui::{self, Color32};

impl App {
    pub(crate) fn dialogs(&mut self, ctx: &egui::Context) {
        let Some(mut dialog) = self.dialog.take() else { return };
        let mut close = false;
        // Enter presses the main button of the dialogs that only ask for a name or an OK
        let enter = ctx.input(|i| i.key_pressed(egui::Key::Enter));
        let title = match &dialog {
            Dialog::Info { title, .. } => title.clone(),
            Dialog::NewAlbum { .. } => "New album".into(),
            Dialog::RenameAlbum { .. } => "Rename album".into(),
            Dialog::Tag { .. } => "Tags and people".into(),
            Dialog::Export { .. } => "Export pictures".into(),
            Dialog::Gallery { .. } => "Web gallery".into(),
            Dialog::FolderNote { .. } => "Folder description".into(),
            Dialog::FolderManager => "Library folders".into(),
            Dialog::Movie { .. } => "Movie".into(),
            Dialog::About => "About AlbumCraft".into(),
            Dialog::OnlineMaps => "Maps".into(),
            Dialog::PrintReview { .. } => "Review".into(),
            Dialog::TrashRejects { .. } => "Move to the trash".into(),
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
                    if ui.button("OK").clicked() || enter {
                        close = true;
                    }
                }
                Dialog::NewAlbum { name } => {
                    ui.label("Album name:");
                    let r = ui.text_edit_singleline(name);
                    r.request_focus();
                    if !targets.is_empty() {
                        ui.label(format!("The {} selected picture(s) will be added.", targets.len()));
                    }
                    ui.horizontal(|ui| {
                        if (ui.button("Create").clicked() || enter) && !name.trim().is_empty() {
                            self.store.albums.push(Album { name: name.trim().to_string(), keys: vec![] });
                            let idx = self.store.albums.len() - 1;
                            self.dirty = true;
                            if !targets.is_empty() {
                                self.add_to_album(idx, &targets);
                            }
                            self.source = Source::Album(idx);
                            close = true;
                        }
                        if ui.button("Cancel").clicked() {
                            close = true;
                        }
                    });
                }
                Dialog::RenameAlbum { idx, name } => {
                    ui.text_edit_singleline(name);
                    ui.horizontal(|ui| {
                        if (ui.button("Rename").clicked() || enter) && !name.trim().is_empty() {
                            if let Some(a) = self.store.albums.get_mut(*idx) {
                                a.name = name.trim().to_string();
                                self.dirty = true;
                            }
                            close = true;
                        }
                        if ui.button("Cancel").clicked() {
                            close = true;
                        }
                    });
                }
                Dialog::Tag { tags, people } => {
                    ui.label(format!("Add to the {} selected picture(s). Separate words with commas.", targets.len()));
                    egui::Grid::new("tagdlg").num_columns(2).show(ui, |ui| {
                        ui.label("Tags:");
                        ui.text_edit_singleline(tags);
                        ui.end_row();
                        ui.label("People:");
                        ui.text_edit_singleline(people);
                        ui.end_row();
                    });
                    ui.horizontal(|ui| {
                        if ui.button("Add").clicked() || enter {
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
                                }
                                for x in &p {
                                    if !m.people.contains(x) {
                                        m.people.push(x.clone());
                                    }
                                }
                            }
                            close = true;
                        }
                        if ui.button("Cancel").clicked() {
                            close = true;
                        }
                    });
                }
                Dialog::Export { max_dim, quality } => {
                    ui.label(format!("{} picture(s) will be saved as new JPEG files with your edits applied.", targets.len()));
                    ui.label("Your original files are never changed.");
                    ui.add_space(4.0);
                    ui.label("Image size:");
                    ui.radio_value(max_dim, 0, "Use original size");
                    ui.radio_value(max_dim, 1600, "Large (1600 pixels)");
                    ui.radio_value(max_dim, 800, "Small, for email (800 pixels)");
                    ui.add(egui::Slider::new(quality, 50..=100).text("JPEG quality"));
                    if ui
                        .checkbox(&mut self.store.export_location, "Include where the pictures were taken (GPS)")
                        .on_hover_text("Turn off before sharing pictures if you don't want others to see the location.")
                        .changed()
                    {
                        self.dirty = true;
                    }
                    ui.horizontal(|ui| {
                        if ui.button("Choose folder and export…").clicked() {
                            let (m, q) = (*max_dim, *quality);
                            self.export(targets.clone(), m, q);
                            close = true;
                        }
                        if ui.button("Cancel").clicked() {
                            close = true;
                        }
                    });
                }
                Dialog::Gallery { title, style, size, captions } => {
                    ui.label(format!("A web page with {} picture(s) and your edits, viewable in any browser or uploadable to a web host.", targets.len()));
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        ui.label("Title:");
                        ui.text_edit_singleline(title);
                    });
                    ui.label("Style:");
                    for st in crate::gallery::Style::ALL {
                        ui.radio_value(style, st, st.label());
                    }
                    ui.label("Picture size:");
                    ui.radio_value(size, 1280, "Medium (1280 pixels, small files)");
                    ui.radio_value(size, 2048, "Large (2048 pixels, for big screens)");
                    ui.checkbox(captions, "Show captions");
                    ui.horizontal(|ui| {
                        if ui.add_enabled(!targets.is_empty(), egui::Button::new("Choose folder and make…")).clicked() {
                            let opts = crate::gallery::Options { title: title.clone(), style: *style, size: *size, captions: *captions };
                            self.make_gallery(targets.clone(), opts);
                            close = true;
                        }
                        if ui.button("Cancel").clicked() {
                            close = true;
                        }
                    });
                }
                Dialog::FolderNote { path, text } => {
                    ui.label("A short description shown under the folder name:");
                    ui.text_edit_multiline(text);
                    ui.horizontal(|ui| {
                        if ui.button("Save").clicked() {
                            if text.trim().is_empty() {
                                self.store.folder_notes.remove(path.as_str());
                            } else {
                                self.store.folder_notes.insert(path.clone(), text.trim().to_string());
                            }
                            self.dirty = true;
                            close = true;
                        }
                        if ui.button("Cancel").clicked() {
                            close = true;
                        }
                    });
                }
                Dialog::FolderManager => {
                    ui.label("The library shows the pictures in these folders and their subfolders, where they are: nothing is copied or moved.");
                    ui.weak("To copy pictures from a memory card or a camera, use Import.");
                    ui.add_space(4.0);
                    let mut remove = None;
                    if self.store.roots.is_empty() {
                        ui.weak("(none yet — sample pictures are shown instead)");
                    }
                    for (i, r) in self.store.roots.iter().enumerate() {
                        ui.horizontal(|ui| {
                            ui.label(format!("{} {r}", icons::FOLDER));
                            if ui.small_button("Remove").clicked() {
                                remove = Some(i);
                            }
                        });
                    }
                    if let Some(i) = remove.filter(|&i| i < self.store.roots.len()) {
                        let path = self.store.roots.remove(i);
                        self.dirty = true;
                        self.rescan();
                        self.undoable(format!("Removed {path} from the library. The pictures and your notes on them are untouched."), Undo::Root { at: i, path });
                    }
                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        if ui.button(format!("{} Add folder…", icons::ADD)).clicked() {
                            self.add_library_folder();
                        }
                        if ui.button(format!("{} Scan again", icons::REFRESH)).clicked() {
                            self.rescan();
                        }
                        if ui.button("Close").clicked() {
                            close = true;
                        }
                    });
                }
                Dialog::Movie { hd } => {
                    let list = if targets.len() > 1 { targets.clone() } else { self.visible_flat() };
                    let ffmpeg = *self.ffmpeg.get_or_insert_with(crate::movie::ffmpeg_available);
                    let st = self.store.slideshow.clone();
                    ui.label(format!(
                        "A video of {} picture(s) as in the slideshow: {} s each{}{}.",
                        list.len(),
                        st.interval.round(),
                        if st.fade { ", crossfades" } else { "" },
                        if st.motion { ", pan and zoom" } else { "" }
                    ));
                    ui.label(egui::RichText::new(format!("About {:.0} s long. Change the timing and effects in the slideshow's control bar.", crate::movie::duration(list.len(), &st))).weak());
                    ui.add_space(4.0);
                    ui.radio_value(hd, false, "720p (1280 × 720)");
                    ui.radio_value(hd, true, "1080p (1920 × 1080)");
                    let s = &mut self.store.slideshow;
                    ui.horizontal(|ui| {
                        if s.music.is_some() {
                            if ui.checkbox(&mut s.music_on, "Music").changed() {
                                self.dirty = true;
                            }
                            let name = s.music.as_deref().and_then(|m| std::path::Path::new(m).file_name()).map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
                            ui.label(egui::RichText::new(name).weak());
                        }
                        if ui.button(if s.music.is_some() { "Other music…" } else { "Add music…" }).clicked() && crate::slideshow_view::pick_music(s) {
                            self.dirty = true;
                        }
                    });
                    if !ffmpeg {
                        ui.add_space(6.0);
                        ui.label(egui::RichText::new("Making movies needs the free FFmpeg program, which wasn't found.").color(crate::theme::t().reject));
                        ui.label("Install it from ffmpeg.org (or your system's package manager), then open this window again.");
                        ui.hyperlink_to("ffmpeg.org/download", "https://ffmpeg.org/download.html");
                    }
                    ui.horizontal(|ui| {
                        if ui.add_enabled(ffmpeg && !list.is_empty(), egui::Button::new("Choose file and make…")).clicked() {
                            let size = if *hd { (1920, 1080) } else { (1280, 720) };
                            self.make_movie(list.clone(), size);
                            close = true;
                        }
                        if ui.button(format!("{} Preview as slideshow", icons::PLAY)).clicked() {
                            let first = list.first().copied().unwrap_or(0);
                            self.start_slideshow(list.clone(), first);
                            close = true;
                        }
                        if ui.button("Cancel").clicked() {
                            close = true;
                        }
                    });
                }
                Dialog::TrashRejects { keys, bytes } => {
                    let n = keys.len();
                    let what = if n == 1 { "the rejected picture".to_string() } else { format!("the {} rejected pictures", crate::app::fmt_count(n)) };
                    ui.label(format!(
                        "Move {what} ({}) to the trash?\n\nThe files are removed from their folders. You can get them back from the system's trash until it is emptied.",
                        crate::library::fmt_bytes(*bytes)
                    ));
                    ui.add_space(8.0);
                    // no Enter here: this one removes files
                    ui.horizontal(|ui| {
                        if ui.button("Move to the trash").clicked() {
                            let keys = std::mem::take(keys);
                            self.trash_files(keys);
                            close = true;
                        }
                        if ui.button("Cancel").clicked() {
                            close = true;
                        }
                    });
                }
                Dialog::PrintReview { grades, before_print } => {
                    let bad: Vec<usize> = grades.iter().filter(|g| g.3 == 0).map(|g| g.0).collect();
                    ui.label(if bad.is_empty() { crate::print::MSG_READY } else { crate::print::MSG_TOO_SMALL });
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
                                ui.label(egui::RichText::new(crate::print::quality_text(*grade, *ppi)).color(colour));
                                ui.end_row();
                            }
                        });
                    });
                    ui.add_space(6.0);
                    let before = *before_print;
                    ui.horizontal(|ui| {
                        if before && !bad.is_empty() {
                            if ui.button("Remove These Pictures").clicked() {
                                close = true;
                                self.print_remove_bad(&bad);
                            }
                            if ui.button("Print Anyway").clicked() {
                                close = true;
                                self.start_print_job();
                            }
                            if ui.button("Cancel").clicked() {
                                close = true;
                            }
                        } else if ui.button("OK").clicked() {
                            close = true;
                        }
                    });
                }
                Dialog::About => {
                    ui.heading("AlbumCraft");
                    ui.label("A desktop photo organizer: browse your picture folders, make albums, pick, reject and rate pictures, fix and touch up photos, and play slideshows.");
                    ui.label("Edits never change your original files; they are applied when you look at or export a picture.");
                    ui.add_space(6.0);
                    ui.label(egui::RichText::new("Built-in map: Natural Earth (public domain). City list: GeoNames (CC BY 4.0). Online maps: © OpenStreetMap contributors (ODbL).").size(11.5).weak());
                    if ui.button("OK").clicked() || enter {
                        close = true;
                    }
                }
                Dialog::OnlineMaps => {
                    ui.label(egui::RichText::new("Show detailed maps from the internet?").strong().size(15.0));
                    ui.add_space(4.0);
                    ui.label("AlbumCraft can show where your pictures were taken. With online maps, it downloads street maps from OpenStreetMap and can look up addresses when you set a location.");
                    ui.add_space(4.0);
                    ui.label("Your pictures and their locations are never sent. The map servers see your internet address, the areas you look at and the places you search for.");
                    ui.add_space(4.0);
                    ui.label("Without online maps, AlbumCraft uses its built-in world map and list of cities, and makes no internet connections.");
                    ui.add_space(4.0);
                    ui.label(egui::RichText::new("You can change this later on the map, or in the main menu: Online maps.").size(12.0).weak());
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        if ui.add(egui::Button::new(egui::RichText::new("Use online maps").color(Color32::WHITE).strong()).fill(t().accent)).clicked() {
                            self.store.online_maps = Some(true);
                            self.dirty = true;
                            close = true;
                        }
                        if ui.button("Built-in map only").clicked() {
                            self.store.online_maps = Some(false);
                            self.dirty = true;
                            close = true;
                        }
                    });
                }
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
                                Some(NoticeAction::Undo(_)) => Some(format!("{} Undo", icons::UNDO)),
                                Some(NoticeAction::Show(_)) => Some(format!("{} Show in folder", icons::FOLDER)),
                                None => None,
                            };
                            if let Some(label) = label
                                && ui.button(label).clicked()
                            {
                                act = Some(i);
                            }
                            // short plain notes need no button: they are gone in a moment
                            if (n.life.is_none() || n.action.is_some()) && ui.small_button(icons::CLOSE).on_hover_text("Close this message").clicked() {
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
                        self.toast(format!("Could not open the file manager: {e}"));
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
