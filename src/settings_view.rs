//! The Settings dialog (`Dialog::Settings`): the sections in a list on the left, the chosen one
//! on the right. Changes apply and are saved at once; Esc or Close shuts it. Settings that also
//! have a place elsewhere (theme button, slideshow bar, Export dialog) change the same saved value.

use crate::app::*;
use crate::i18n::{self, tr};
use crate::icons;
use crate::library::{BarStyle, Cache, fmt_bytes};
use crate::theme::Mode;
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

    fn label(self) -> String {
        match self {
            SettingsSection::Interface => tr!("settings-section-interface"),
            SettingsSection::Folders => tr!("settings-section-folders"),
            SettingsSection::Editing => tr!("settings-section-editing"),
            SettingsSection::Slideshow => tr!("settings-section-slideshow"),
            SettingsSection::Export => tr!("settings-section-export"),
            SettingsSection::Maps => tr!("settings-section-maps"),
            SettingsSection::Storage => tr!("settings-section-storage"),
            SettingsSection::Ai => tr!("settings-section-ai"),
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
            ui.heading(tr!("settings-title"));
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
                            SettingsSection::Ai => settings_ai(ui),
                        }
                    });
                });
            });
            ui.separator();
            ui.horizontal(|ui| {
                note(ui, &tr!("settings-saved-note"));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button(tr!("button-close")).clicked() {
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
        group(ui, &tr!("settings-language"));
        let languages = i18n::available();
        let name_of = |code: &str| languages.iter().find(|l| l.0 == code).map(|l| l.1.clone());
        let system = tr!("settings-language-system", language = i18n::system_language().and_then(|c| name_of(&c)).unwrap_or_else(|| name_of("en").unwrap_or_default()));
        let shown = name_of(&self.store.language).unwrap_or_else(|| system.clone());
        let mut chosen = self.store.language.clone();
        egui::ComboBox::from_id_salt("language").selected_text(shown).show_ui(ui, |ui| {
            ui.selectable_value(&mut chosen, String::new(), system);
            for (code, name) in &languages {
                ui.selectable_value(&mut chosen, code.clone(), name);
            }
        });
        if chosen != self.store.language {
            i18n::set_language(&chosen);
            crate::theme::set_fonts(ui.ctx());
            self.store.language = chosen;
            self.dirty = true;
        }
        note(ui, &tr!("settings-language-note"));

        group(ui, &tr!("settings-theme"));
        let mut mode = Mode::from_name(&self.store.theme);
        let before = mode;
        for m in [Mode::System, Mode::Light, Mode::Dark] {
            ui.radio_value(&mut mode, m, m.label());
        }
        if mode != before {
            self.store.theme = mode.name().into();
            self.dirty = true;
        }

        group(ui, &tr!("settings-bar"));
        let b = &mut self.store.bar_style;
        let mut changed = ui.radio_value(b, BarStyle::Compact, tr!("settings-bar-compact")).changed();
        changed |= ui.radio_value(b, BarStyle::Tray, tr!("settings-bar-tray")).changed();
        note(ui, &tr!("settings-bar-note"));

        group(ui, &tr!("settings-thumb-size"));
        changed |= ui.add(egui::Slider::new(&mut self.store.thumb_size, 64.0..=256.0).show_value(false)).changed();
        if changed {
            self.dirty = true;
        }
    }

    fn settings_folders(&mut self, ui: &mut egui::Ui) {
        note(ui, &tr!("settings-folders-intro"));
        ui.add_space(6.0);
        if self.store.roots.is_empty() {
            ui.weak(tr!("settings-folders-none"));
        }
        let mut remove = None;
        egui::Grid::new("settings_roots").num_columns(3).spacing([12.0, 6.0]).show(ui, |ui| {
            for (i, r) in self.store.roots.iter().enumerate() {
                ui.label(format!("{} {r}", icons::FOLDER));
                let n: usize = self.folders.iter().filter(|f| under(&f.path, r)).map(|f| f.photos.len()).sum();
                ui.label(RichText::new(tr!("count-pictures", n = n)).weak());
                if ui.small_button(tr!("button-remove")).on_hover_text(tr!("settings-folders-remove-hint")).clicked() {
                    remove = Some(i);
                }
                ui.end_row();
            }
        });
        if let Some(i) = remove.filter(|&i| i < self.store.roots.len()) {
            let path = self.store.roots.remove(i);
            self.dirty = true;
            self.rescan();
            self.undoable(tr!("removed-root", path = path.as_str()), Undo::Root { at: i, path });
        }
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            if ui.button(format!("{} {}", icons::ADD, tr!("add-folder"))).clicked() {
                self.add_library_folder();
            }
            if ui.button(format!("{} {}", icons::REFRESH, tr!("scan-again"))).on_hover_text(tr!("settings-folders-rescan-hint")).clicked() {
                self.rescan();
            }
        });
    }

    fn settings_editing(&mut self, ui: &mut egui::Ui) {
        group(ui, &tr!("settings-edit-opens"));
        let a = &mut self.store.edit_advanced;
        let mut changed = ui.radio_value(a, false, tr!("settings-edit-basic")).changed();
        changed |= ui.radio_value(a, true, tr!("settings-edit-advanced")).changed();
        group(ui, &tr!("settings-info-panel"));
        changed |= ui.checkbox(&mut self.store.info_open, tr!("settings-info-show")).changed();
        if changed {
            self.dirty = true;
        }
    }

    fn settings_slideshow(&mut self, ui: &mut egui::Ui) {
        let s = &mut self.store.slideshow;
        let mut changed = false;
        group(ui, &tr!("settings-timing"));
        ui.horizontal(|ui| {
            changed |= ui.checkbox(&mut s.auto_advance, tr!("settings-next-after")).changed();
            ui.add_enabled_ui(s.auto_advance, |ui| {
                changed |= ui.add(egui::Slider::new(&mut s.interval, 1.0..=60.0).logarithmic(true).suffix(" s").max_decimals(0)).changed();
            });
        });
        if s.auto_advance {
            note(ui, &tr!("settings-note-paused"));
        } else {
            note(ui, &tr!("settings-note-manual"));
        }

        group(ui, &tr!("settings-effects"));
        changed |= ui.checkbox(&mut s.fade, tr!("settings-fade")).changed();
        changed |= ui.checkbox(&mut s.motion, tr!("settings-motion")).changed();
        ui.add_enabled_ui(s.motion, |ui| {
            ui.horizontal(|ui| {
                ui.add_space(24.0);
                changed |= ui.radio_value(&mut s.strong_motion, false, tr!("settings-motion-subtle")).changed();
                changed |= ui.radio_value(&mut s.strong_motion, true, tr!("settings-motion-medium")).changed();
            });
        });
        changed |= ui.checkbox(&mut s.captions, tr!("settings-captions")).changed();
        note(ui, &tr!("settings-viewer-note"));

        group(ui, &tr!("settings-music"));
        match s.music.clone() {
            Some(path) => {
                changed |= ui.checkbox(&mut s.music_on, tr!("settings-music-play")).changed();
                let name = std::path::Path::new(&path).file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or(path.clone());
                ui.horizontal(|ui| {
                    ui.label(format!("{} {name}", icons::MUSIC)).on_hover_text(&path);
                    if ui.button(tr!("settings-music-another")).clicked() {
                        changed |= crate::slideshow_view::pick_music(s);
                    }
                    if ui.button(tr!("button-remove")).clicked() {
                        s.music = None;
                        s.music_on = false;
                        changed = true;
                    }
                });
            }
            None => {
                if ui.button(format!("{} {}", icons::MUSIC, tr!("settings-music-choose"))).clicked() {
                    changed |= crate::slideshow_view::pick_music(s);
                }
            }
        }
        ui.add_space(8.0);
        note(ui, &tr!("settings-movies-note"));
        if changed {
            self.dirty = true;
        }
    }

    fn settings_export(&mut self, ui: &mut egui::Ui) {
        note(ui, &tr!("settings-export-note"));
        group(ui, &tr!("settings-export-size"));
        let mut changed = false;
        for dim in crate::export_view::SIZES {
            changed |= ui.radio_value(&mut self.store.export_size, dim, crate::export_view::size_label(dim)).changed();
        }
        group(ui, &tr!("settings-export-quality"));
        changed |= ui.add(egui::Slider::new(&mut self.store.export_quality, 50..=100)).changed();
        group(ui, &tr!("settings-export-info"));
        changed |= ui
            .checkbox(&mut self.store.export_location, tr!("settings-export-gps"))
            .on_hover_text(tr!("settings-export-gps-hint"))
            .changed();
        changed |= ui
            .checkbox(&mut self.store.export_exif, tr!("settings-export-exif"))
            .on_hover_text(tr!("settings-export-exif-hint"))
            .changed();
        ui.add_space(6.0);
        egui::Grid::new("settings_credit").num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
            ui.label(tr!("settings-creator"));
            changed |= ui.add(egui::TextEdit::singleline(&mut self.store.export_creator).hint_text(tr!("settings-creator-hint")).desired_width(280.0)).changed();
            ui.end_row();
            ui.label(tr!("settings-copyright"));
            changed |= ui.add(egui::TextEdit::singleline(&mut self.store.export_copyright).hint_text(tr!("settings-copyright-hint")).desired_width(280.0)).changed();
            ui.end_row();
        });
        note(ui, &tr!("settings-credit-note"));
        if changed {
            self.dirty = true;
        }
    }

    fn settings_maps(&mut self, ui: &mut egui::Ui) {
        group(ui, &tr!("settings-maps-online"));
        let mut online = self.store.online_maps == Some(true);
        if ui.checkbox(&mut online, tr!("settings-maps-use")).changed() {
            self.store.online_maps = Some(online);
            self.dirty = true;
        }
        note(ui, &tr!("settings-maps-note"));
        group(ui, &tr!("settings-maps-tiles"));
        let tiles = self.cache_sizes.map(|c| c.1);
        self.cache_row(ui, tiles, Cache::Tiles, &tr!("settings-maps-tiles-after"));
    }

    fn settings_storage(&mut self, ui: &mut egui::Ui) {
        group(ui, &tr!("settings-notes"));
        note(ui, &tr!("settings-notes-note"));
        if let Some(path) = crate::library::store_path() {
            ui.horizontal(|ui| {
                ui.label(RichText::new(path.to_string_lossy()).monospace().size(12.0));
                if ui.small_button(format!("{} {}", icons::FOLDER, tr!("notice-show"))).clicked()
                    && let Err(e) = crate::library_view::reveal_in_file_manager(&path)
                {
                    self.toast(tr!("notice-no-file-manager", error = e.to_string()));
                }
            });
        }
        group(ui, &tr!("settings-thumbs"));
        let thumbs = self.cache_sizes.map(|c| c.0);
        self.cache_row(ui, thumbs, Cache::Thumbs, &tr!("settings-thumbs-after"));
    }

    /// The size of a cache and a button to empty it.
    fn cache_row(&mut self, ui: &mut egui::Ui, bytes: Option<u64>, which: Cache, after: &str) {
        ui.horizontal(|ui| {
            ui.label(match bytes {
                Some(b) => tr!("cache-on-disk", size = fmt_bytes(b)),
                None => tr!("cache-measuring"),
            });
            if ui.add_enabled(bytes.is_some_and(|b| b > 0), egui::Button::new(tr!("cache-empty"))).on_hover_text(after).clicked() {
                self.cache_sizes = None;
                self.lib.clear_cache(which);
            }
        });
    }
}

fn settings_ai(ui: &mut egui::Ui) {
    note(ui, &tr!("settings-ai-soon"));
    ui.add_space(4.0);
    for (title, text) in [
        (tr!("settings-ai-faces"), tr!("settings-ai-faces-text")),
        (tr!("settings-ai-search"), tr!("settings-ai-search-text")),
        (tr!("settings-ai-tags"), tr!("settings-ai-tags-text")),
    ] {
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new(title).strong());
            ui.label(text);
        });
        ui.add_space(2.0);
    }
    ui.add_space(6.0);
    note(ui, &tr!("settings-ai-local"));
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
