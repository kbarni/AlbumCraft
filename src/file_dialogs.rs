//! File and folder pickers that never block the event loop (the suite's `craft-dialogs`).
//!
//! A blocking `rfd::FileDialog` runs the platform's modal loop inside winit's event handler; on
//! macOS an event arriving meanwhile (a file dragged over the window, the panel resized) makes
//! winit panic, and the app aborts. Here a picker is only asked for: `App::ask_file` queues it
//! with what to do with the answer (a `FileJob`), `ui` shows it at the end of the frame with the
//! window as its parent, and `logic` runs the `FileJob` on a later frame (`App::file_answers`).

use std::path::PathBuf;
use std::sync::Arc;

use craft_dialogs::{Answer, Dialogs, FileDialog};

use crate::app::App;
use crate::file_actions::Moving;
use crate::i18n::tr;

/// What to do with the file or folder the user picks.
pub enum FileJob {
    /// Export: the folder the copies go to.
    ExportDest,
    /// Export: the picture signed on the copies.
    ExportLogo,
    /// Settings › Storage: keep the library in this folder.
    LibraryLocation,
    /// Settings › Folders, the library's menu: watch one more folder.
    LibraryFolder,
    /// Settings › Folders: leave a folder of the library out.
    ExcludedFolder,
    /// Collage: a picture file as the background.
    CollageBackground,
    /// Collage: save these pages as a PDF, or one picture per page.
    CollageSave { pages: Vec<crate::collage::Page>, pdf: bool },
    /// Import: copy from this folder (a card, a camera).
    ImportSource,
    /// Import: copy into a folder under this one.
    ImportDest,
    /// Web gallery of these pictures, in a new folder inside the one picked.
    Gallery { ids: Vec<usize>, opts: crate::gallery::Options },
    /// Movie of these pictures, at this size.
    Movie { ids: Vec<usize>, size: (u32, u32) },
    /// Slideshow music.
    Music(MusicFor),
    /// The Move to dialog: move into the folder picked (Other folder…).
    MoveTo,
}

/// Whose slideshow music the user picked.
pub enum MusicFor {
    /// The remembered settings (Settings › Slideshow, the movie dialog).
    Saved,
    /// The slideshow playing now; `viewer` as in `slideshow_controls`.
    Playing { viewer: bool },
}

/// The picker queue, with a closure that wakes the window when an answer arrives.
pub fn new_queue(ctx: &eframe::egui::Context) -> Dialogs<FileJob> {
    let ctx = ctx.clone();
    Dialogs::new(Arc::new(move || ctx.request_repaint()))
}

/// The picture types the pickers offer (signature logo, collage background).
pub const PICTURES: &[&str] = &["png", "jpg", "jpeg", "webp", "gif", "bmp", "tif", "tiff"];

impl App {
    /// Ask for a file or folder; `then` runs with the answer on a later frame. While a picker is
    /// open (on macOS Open and Folder panels don't hold the window), another click brings that
    /// one to the front instead of queueing a second.
    pub fn ask_file(&mut self, dialog: FileDialog, then: FileJob) {
        if self.file_dialogs.is_open() || self.file_dialogs.queued() > 0 {
            self.file_dialogs.raise();
            return;
        }
        self.file_dialogs.ask(dialog, then);
    }

    /// Run what waits for the pickers that have answered. In `logic`, before anything else
    /// looks at the state they change.
    pub(crate) fn file_answers(&mut self) {
        for (then, answer) in self.file_dialogs.take_answers() {
            match answer {
                Answer::Paths(paths) => {
                    if let Some(path) = paths.into_iter().next() {
                        self.file_chosen(then, path);
                    }
                }
                Answer::Unavailable(why) => self.error(tr!("file-dialog-failed", error = why)),
                Answer::Cancelled | Answer::Message(_) => {}
            }
        }
    }

    fn file_chosen(&mut self, then: FileJob, path: PathBuf) {
        let text = path.to_string_lossy().into_owned();
        match then {
            FileJob::ExportDest => {
                if let Some(st) = self.export.as_mut() {
                    st.dest = text;
                }
            }
            FileJob::ExportLogo => {
                self.store.export_signature.logo = Some(text);
                self.dirty = true;
            }
            // not while the library must not be saved (Settings shows the button disabled then)
            FileJob::LibraryLocation if self.persist => self.relocate_library(Some(path)),
            FileJob::LibraryLocation => {}
            FileJob::LibraryFolder => self.add_library_folder_at(&text),
            FileJob::ExcludedFolder => self.add_excluded_folder_at(&text),
            FileJob::CollageBackground => self.set_collage_bg(text),
            FileJob::CollageSave { pages, pdf } => self.collage_save_to(pages, pdf, path),
            FileJob::ImportSource => {
                if self.import.is_some() {
                    self.set_import_source(path);
                }
            }
            FileJob::ImportDest => self.set_import_dest(text),
            FileJob::Gallery { ids, opts } => self.make_gallery_in(ids, opts, path),
            FileJob::Movie { ids, size } => self.make_movie_to(ids, size, path),
            FileJob::Music(MusicFor::Saved) => {
                let s = &mut self.store.slideshow;
                s.music = Some(text);
                s.music_on = true;
                self.dirty = true;
            }
            FileJob::Music(MusicFor::Playing { viewer }) => {
                let Some(show) = self.slide.as_mut() else { return };
                show.settings.music = Some(text);
                show.settings.music_on = true;
                self.slide_settings_changed(viewer, true);
            }
            FileJob::MoveTo => {
                let Some(crate::app::Dialog::MoveTo { what, .. }) = &self.dialog else { return };
                let sources = match what {
                    Moving::Pictures(keys) => keys.clone(),
                    Moving::Folder(f) => vec![f.clone()],
                };
                match self.move_into(&sources, &text) {
                    Ok(()) => self.dialog = None,
                    Err(e) => {
                        if let Some(crate::app::Dialog::MoveTo { error, .. }) = &mut self.dialog {
                            *error = e;
                        }
                    }
                }
            }
        }
    }
}
