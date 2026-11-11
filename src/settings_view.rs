//! The Settings dialog (`Dialog::Settings`): the sections in a list on the left, the chosen one
//! on the right. Changes apply and are saved at once; Esc or Close shuts it. Settings that also
//! have a place elsewhere (theme button, slideshow bar, Export dialog) change the same saved value.

use crate::app::*;
use crate::icons;
use crate::library::{BarStyle, Cache, fmt_bytes};
use crate::theme::{Mode, t};
use eframe::egui::{self, RichText};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum SettingsSection {
    #[default]
    Interface,
    Folders,
    Editing,
    Slideshow,
    Export,
    Maps,
    Storage,
    Ai,
}

impl SettingsSection {
    pub const ALL: [SettingsSection; 8] = [
        SettingsSection::Interface,
        SettingsSection::Folders,
        SettingsSection::Editing,
        SettingsSection::Slideshow,
        SettingsSection::Export,
        SettingsSection::Maps,
        SettingsSection::Storage,
        SettingsSection::Ai,
    ];

    fn label(self) -> &'static str {
        match self {
            SettingsSection::Interface => "Interface",
            SettingsSection::Folders => "Folders",
            SettingsSection::Editing => "Editing",
            SettingsSection::Slideshow => "Slideshow",
            SettingsSection::Export => "Export",
            SettingsSection::Maps => "Maps",
            SettingsSection::Storage => "Storage",
            SettingsSection::Ai => "AI features",
        }
    }

    fn icon(self) -> &'static str {
        match self {
            SettingsSection::Interface => icons::DESKTOP,
            SettingsSection::Folders => icons::FOLDER,
            SettingsSection::Editing => icons::EDIT,
            SettingsSection::Slideshow => icons::PLAY,
            SettingsSection::Export => icons::EXPORT,
            SettingsSection::Maps => icons::MAP,
            SettingsSection::Storage => icons::DRIVE,
            SettingsSection::Ai => icons::AUTO_FIX,
        }
    }
}

/// A section's sub-heading.
fn group(ui: &mut egui::Ui, text: &str) {
    ui.add_space(10.0);
    ui.label(RichText::new(text).strong());
    ui.add_space(2.0);
}

fn note(ui: &mut egui::Ui, text: &str) {
    ui.label(RichText::new(text).size(12.0).weak());
}

impl App {
    /// Open the Settings dialog (at `section`), and ask how big the caches are.
    pub fn open_settings(&mut self, section: SettingsSection) {
        self.cache_sizes = None;
        self.lib.cache_sizes();
        self.dialog = Some(Dialog::Settings { section });
    }

    pub(crate) fn settings_dialog(&mut self, ctx: &egui::Context, mut section: SettingsSection) {
        let mut close = false;
        let screen = ctx.content_rect().size();
        let size = egui::vec2((screen.x - 80.0).clamp(420.0, 820.0), (screen.y - 140.0).clamp(260.0, 520.0));
        #[allow(deprecated)]
        let modal = egui::Modal::new(egui::Id::new("settings")).show(ctx, |ui| {
            ui.set_width(size.x);
            ui.heading("Settings");
            ui.add_space(6.0);
            // a fixed size: inside a modal, separators and scroll areas would grow without end
            ui.allocate_ui_with_layout(size, egui::Layout::left_to_right(egui::Align::Min), |ui| {
                ui.set_min_size(size);
                ui.vertical(|ui| {
                    ui.set_width(160.0);
                    for s in SettingsSection::ALL {
                        let text = RichText::new(format!("{}  {}", s.icon(), s.label())).size(14.0);
                        let b = egui::Button::new(text).selected(section == s).frame_when_inactive(false).min_size(egui::vec2(160.0, 30.0));
                        if ui.add(b).clicked() {
                            section = s;
                        }
                    }
                });
                ui.separator();
                let body = egui::vec2(size.x - 180.0, size.y);
                ui.allocate_ui_with_layout(body, egui::Layout::top_down(egui::Align::Min), |ui| {
                    ui.set_min_size(body);
                    egui::ScrollArea::vertical().id_salt(("settings", section as u8)).max_height(size.y).auto_shrink([false, true]).show(ui, |ui| {
                        ui.set_max_width(body.x - 16.0);
                        ui.heading(section.label());
                        match section {
                            SettingsSection::Interface => self.settings_interface(ui),
                            SettingsSection::Folders => self.settings_folders(ui),
                            SettingsSection::Editing => self.settings_editing(ui),
                            SettingsSection::Slideshow => self.settings_slideshow(ui),
                            SettingsSection::Export => self.settings_export(ui),
                            SettingsSection::Maps => self.settings_maps(ui),
                            SettingsSection::Storage => self.settings_storage(ui),
                            SettingsSection::Ai => self.settings_ai(ui),
                        }
                    });
                });
            });
            ui.separator();
            ui.horizontal(|ui| {
                note(ui, "Changes are saved as you make them.");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Close").clicked() {
                        close = true;
                    }
                });
            });
        });
        // the folder list may have opened another dialog (none does today); keep it if so
        if self.dialog.is_none() && !close && !modal.should_close() {
            self.dialog = Some(Dialog::Settings { section });
        }
    }

    fn settings_interface(&mut self, ui: &mut egui::Ui) {
        group(ui, "Language");
        ui.add_enabled_ui(false, |ui| {
            egui::ComboBox::from_id_salt("language").selected_text("English").show_ui(ui, |_| {});
        });
        note(ui, "Only English for now.");

        group(ui, "Theme");
        let mut mode = Mode::from_name(&self.store.theme);
        let before = mode;
        for m in [Mode::System, Mode::Light, Mode::Dark] {
            ui.radio_value(&mut mode, m, m.label());
        }
        if mode != before {
            self.store.theme = mode.name().into();
            self.dirty = true;
        }

        group(ui, "Bar at the bottom of the library");
        let b = &mut self.store.bar_style;
        let mut changed = ui.radio_value(b, BarStyle::Compact, "Compact: one line, shown while pictures are selected").changed();
        changed |= ui.radio_value(b, BarStyle::Tray, "Tray: thumbnails of the selected pictures and large buttons").changed();
        note(ui, "In the compact bar, the buttons lose their names when the window is narrow. When the buttons don't fit, the bar scrolls sideways.");

        group(ui, "Picture size in the library");
        changed |= ui.add(egui::Slider::new(&mut self.store.thumb_size, 64.0..=256.0).show_value(false)).changed();
        if changed {
            self.dirty = true;
        }
    }

    fn settings_folders(&mut self, ui: &mut egui::Ui) {
        note(ui, "The library shows the pictures in these folders and their subfolders, where they are: nothing is copied or moved. To copy pictures from a memory card or a camera, use Import.");
        ui.add_space(6.0);
        if self.store.roots.is_empty() {
            ui.weak("No folders yet: sample pictures are shown instead.");
        }
        let mut remove = None;
        egui::Grid::new("settings_roots").num_columns(3).spacing([12.0, 6.0]).show(ui, |ui| {
            for (i, r) in self.store.roots.iter().enumerate() {
                ui.label(format!("{} {r}", icons::FOLDER));
                let n: usize = self.folders.iter().filter(|f| under(&f.path, r)).map(|f| f.photos.len()).sum();
                ui.label(RichText::new(format!("{n} picture{}", if n == 1 { "" } else { "s" })).weak());
                if ui.small_button("Remove").on_hover_text("Stop showing this folder (its pictures and your notes on them are kept)").clicked() {
                    remove = Some(i);
                }
                ui.end_row();
            }
        });
        if let Some(i) = remove.filter(|&i| i < self.store.roots.len()) {
            let path = self.store.roots.remove(i);
            self.dirty = true;
            self.rescan();
            self.undoable(format!("Removed {path} from the library. The pictures and your notes on them are untouched."), Undo::Root { at: i, path });
        }
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            if ui.button(format!("{} Add folder…", icons::ADD)).clicked() {
                self.add_library_folder();
            }
            if ui.button(format!("{} Scan again", icons::REFRESH)).on_hover_text("Look for new, changed and removed pictures now").clicked() {
                self.rescan();
            }
        });
    }

    fn settings_editing(&mut self, ui: &mut egui::Ui) {
        group(ui, "The Edit tab opens on");
        let a = &mut self.store.edit_advanced;
        let mut changed = ui.radio_value(a, false, "Basic: one-click fixes and tools").changed();
        changed |= ui.radio_value(a, true, "Advanced: the sliders").changed();
        group(ui, "Info panel");
        changed |= ui.checkbox(&mut self.store.info_open, "Show the info panel in the editor (I)").changed();
        if changed {
            self.dirty = true;
        }
    }

    fn settings_slideshow(&mut self, ui: &mut egui::Ui) {
        let s = &mut self.store.slideshow;
        let mut changed = false;
        group(ui, "Timing");
        ui.horizontal(|ui| {
            changed |= ui.checkbox(&mut s.auto_advance, "Next picture after").changed();
            ui.add_enabled_ui(s.auto_advance, |ui| {
                changed |= ui.add(egui::Slider::new(&mut s.interval, 1.0..=60.0).logarithmic(true).suffix(" s").max_decimals(0)).changed();
            });
        });
        if s.auto_advance {
            note(ui, "Space pauses a slideshow, ← and → step through it.");
        } else {
            note(ui, "Each picture stays until you move on with → or the buttons. The full-screen viewer (F11) still moves on by itself.");
        }

        group(ui, "Effects");
        changed |= ui.checkbox(&mut s.fade, "Crossfade between pictures").changed();
        changed |= ui.checkbox(&mut s.motion, "Slow pan and zoom").changed();
        ui.add_enabled_ui(s.motion, |ui| {
            ui.horizontal(|ui| {
                ui.add_space(24.0);
                changed |= ui.radio_value(&mut s.strong_motion, false, "Subtle").changed();
                changed |= ui.radio_value(&mut s.strong_motion, true, "Medium").changed();
            });
        });
        changed |= ui.checkbox(&mut s.captions, "Show captions").changed();
        note(ui, "The full-screen viewer (F11 in the editor) always starts without crossfade and pan and zoom.");

        group(ui, "Music");
        match s.music.clone() {
            Some(path) => {
                changed |= ui.checkbox(&mut s.music_on, "Play music during slideshows").changed();
                let name = std::path::Path::new(&path).file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or(path.clone());
                ui.horizontal(|ui| {
                    ui.label(format!("{} {name}", icons::MUSIC)).on_hover_text(&path);
                    if ui.button("Choose another…").clicked() {
                        changed |= crate::slideshow_view::pick_music(s);
                    }
                    if ui.button("Remove").clicked() {
                        s.music = None;
                        s.music_on = false;
                        changed = true;
                    }
                });
            }
            None => {
                if ui.button(format!("{} Choose music…", icons::MUSIC)).clicked() {
                    changed |= crate::slideshow_view::pick_music(s);
                }
            }
        }
        ui.add_space(8.0);
        note(ui, "Movies made from pictures use these settings too.");
        if changed {
            self.dirty = true;
        }
    }

    fn settings_export(&mut self, ui: &mut egui::Ui) {
        note(ui, "What the Export view starts with.");
        group(ui, "Picture size");
        let mut changed = false;
        for (dim, label) in crate::export_view::SIZES {
            changed |= ui.radio_value(&mut self.store.export_size, dim, label).changed();
        }
        group(ui, "JPEG quality");
        changed |= ui.add(egui::Slider::new(&mut self.store.export_quality, 50..=100)).changed();
        group(ui, "Information in the copies");
        changed |= ui
            .checkbox(&mut self.store.export_location, "Include where the pictures were taken (GPS)")
            .on_hover_text("Turn off before sharing pictures if you don't want others to see the location.")
            .changed();
        changed |= ui
            .checkbox(&mut self.store.export_exif, "Include the camera information (EXIF)")
            .on_hover_text("Camera, lens, exposure and the date taken, copied from the original file. The location follows the setting above.")
            .changed();
        ui.add_space(6.0);
        egui::Grid::new("settings_credit").num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
            ui.label("Creator");
            changed |= ui.add(egui::TextEdit::singleline(&mut self.store.export_creator).hint_text("Your name").desired_width(280.0)).changed();
            ui.end_row();
            ui.label("Copyright");
            changed |= ui.add(egui::TextEdit::singleline(&mut self.store.export_copyright).hint_text("© Your Name").desired_width(280.0)).changed();
            ui.end_row();
        });
        note(ui, "Written into each copy (EXIF), in place of the original's. Emailed copies follow these settings too. The signature or logo is chosen in the Export view.");
        if changed {
            self.dirty = true;
        }
    }

    fn settings_maps(&mut self, ui: &mut egui::Ui) {
        group(ui, "Online maps");
        let mut online = self.store.online_maps == Some(true);
        if ui.checkbox(&mut online, "Use online maps (OpenStreetMap)").changed() {
            self.store.online_maps = Some(online);
            self.dirty = true;
        }
        note(ui, "Detailed street maps and address search. Your pictures and their locations are never sent; the map servers see your internet address, the areas you look at and the places you search for. Off: only the built-in world map and city list, and no internet connections.");
        group(ui, "Downloaded map tiles");
        let tiles = self.cache_sizes.map(|c| c.1);
        self.cache_row(ui, tiles, Cache::Tiles, "Map tiles are downloaded again when needed.");
    }

    fn settings_storage(&mut self, ui: &mut egui::Ui) {
        group(ui, "Your notes");
        note(ui, "Ratings, captions, tags, edits and albums are kept in one file; your picture files are never changed.");
        if let Some(path) = crate::library::store_path() {
            ui.horizontal(|ui| {
                ui.label(RichText::new(path.to_string_lossy()).monospace().size(12.0));
                if ui.small_button(format!("{} Show in folder", icons::FOLDER)).clicked()
                    && let Err(e) = crate::library_view::reveal_in_file_manager(&path)
                {
                    self.toast(format!("Could not open the file manager: {e}"));
                }
            });
        }
        group(ui, "Thumbnails");
        let thumbs = self.cache_sizes.map(|c| c.0);
        self.cache_row(ui, thumbs, Cache::Thumbs, "Thumbnails are made again from the pictures when needed (slower for a while).");
    }

    /// The size of a cache and a button to empty it.
    fn cache_row(&mut self, ui: &mut egui::Ui, bytes: Option<u64>, which: Cache, after: &str) {
        ui.horizontal(|ui| {
            ui.label(match bytes {
                Some(b) => format!("{} on disk", fmt_bytes(b)),
                None => "Measuring…".into(),
            });
            if ui.add_enabled(bytes.is_some_and(|b| b > 0), egui::Button::new("Empty")).on_hover_text(after).clicked() {
                self.cache_sizes = None;
                self.lib.clear_cache(which);
            }
        });
    }
}

impl App {
    fn settings_ai(&mut self, ui: &mut egui::Ui) {
        note(ui, "Finding faces, searching by describing a picture, proposing tags. Everything is computed on this computer: no picture, face or search text is sent anywhere. The networks these features need are downloaded only when you ask, from AlbumCraft's own releases.");
        if !crate::ai::runtime::available() {
            ui.add_space(4.0);
            ui.label(RichText::new("This build of AlbumCraft cannot run networks: these features are not available in it.").color(t().bad));
        }

        group(ui, "Networks");
        let now = ui.input(|i| i.time);
        if now - self.packs_at > 1.0 {
            self.packs_at = now;
            self.packs = self.lib.model_packs();
        }
        if self.packs.is_empty() {
            ui.label("No network can be downloaded with this version yet. Planned:");
            ui.add_space(2.0);
            for (title, text) in [
                ("Faces", "find faces and group them by person; you name the groups. A download of about 10 to 40 MB."),
                ("Smart search", "find pictures by describing them (\"snow\", \"birthday cake\"). Light (English, about 100 MB) or Multilingual (about 400 MB)."),
                ("Tag suggestions", "tags proposed from what is in the picture."),
            ] {
                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new(title).strong());
                    ui.label(text);
                });
                ui.add_space(2.0);
            }
        }
        let downloading = |id: &str, tasks: &[RunningTask]| tasks.iter().any(|t| t.label == download_label(id));
        let (mut download, mut remove) = (None, None);
        for p in &self.packs {
            let pack = &p.pack;
            ui.horizontal(|ui| {
                ui.label(RichText::new(&pack.title).strong());
                ui.label(RichText::new(format!("{}  ·  licence: {}", fmt_bytes(pack.bytes()), pack.licence)).size(12.0).weak());
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if p.installed {
                        if ui.button("Remove").on_hover_text("Remove the networks from this computer. What was found in your pictures is kept.").clicked() {
                            remove = Some(pack.id.clone());
                        }
                        ui.label(RichText::new(format!("{} Downloaded", icons::CHECK)).size(12.0).color(t().good));
                    } else if downloading(&pack.id, &self.tasks) {
                        ui.spinner();
                        ui.label(RichText::new("Downloading… (see the bar at the bottom)").size(12.0).weak());
                    } else if ui.add_enabled(crate::ai::runtime::available(), egui::Button::new("Download")).clicked() {
                        download = Some(pack.id.clone());
                    }
                });
            });
            if !pack.about.is_empty() {
                note(ui, &pack.about);
            }
            ui.add_space(6.0);
        }
        if let Some(id) = download {
            self.download_pack(&id);
        }
        if let Some(id) = remove {
            if let Err(e) = self.lib.remove_pack(&id) {
                self.error(format!("The networks could not be removed: {e}"));
            }
            self.packs_at = f64::NEG_INFINITY;
        }

        group(ui, "Analysis of the library");
        note(ui, "Each picture is analysed once, in the background, while AlbumCraft has nothing else to do. It stops when AlbumCraft closes and carries on at the next start.");
        ui.add_space(2.0);
        let a = self.analysis;
        ui.label(if a.active() {
            format!("{} of {} pictures analysed{}", fmt_count(a.done), fmt_count(a.total), if a.paused { " (paused)" } else { "" })
        } else if self.packs.iter().any(|p| p.installed) {
            "Every picture is analysed.".to_string()
        } else {
            "Nothing to analyse: no network is downloaded.".to_string()
        });
        if ui.checkbox(&mut self.store.analysis_paused, "Pause the analysis").changed() {
            self.dirty = true;
        }
    }

    /// Download a pack of networks, as a task in the status bar.
    pub fn download_pack(&mut self, id: &str) {
        let task = self.start_task(download_label(id));
        self.lib.download_pack(task, id.to_string());
    }
}

/// The status bar's label of a pack's download (also how a running download is recognised).
fn download_label(id: &str) -> String {
    let title = crate::ai::models::manifest().pack(id).map(|p| p.title.as_str()).unwrap_or(id);
    format!("Downloading {title}")
}

/// `path` is the folder `root` or inside it.
fn under(path: &str, root: &str) -> bool {
    path.strip_prefix(root).is_some_and(|rest| rest.is_empty() || rest.starts_with(['/', '\\']) || root.ends_with(['/', '\\']))
}

#[cfg(test)]
mod tests {
    #[test]
    fn folders_under_a_root() {
        assert!(super::under("/p/a", "/p/a"));
        assert!(super::under("/p/a/b", "/p/a"));
        assert!(super::under("/p/a/b", "/p/a/"));
        assert!(!super::under("/p/ab", "/p/a"));
        assert!(super::under("C:\\p\\a", "C:\\p"));
    }
}
