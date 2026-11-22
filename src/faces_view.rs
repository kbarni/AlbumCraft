//! Faces in the window: the question asked once whether to find them, and turning them on.
//! (The People view and the corrections come with the grouping.)

use crate::app::{App, Dialog};
use std::sync::Arc;

/// How often the faces are grouped again while the library is being analysed.
const REGROUP_EVERY: f64 = 30.0;

/// The library's faces and their groups, as last grouped.
#[derive(Default)]
pub struct FaceGroups {
    pub index: Option<Arc<crate::faces::FaceIndex>>,
    asked_at: Option<f64>,
    was_active: bool,
}

impl App {
    /// Ask once whether to find faces: at a start, after the online-maps question, when nothing
    /// else is shown, while the AI features are on and can run. Closed without an answer, it is
    /// asked again at the next start.
    pub fn ask_about_faces(&mut self) {
        let ask = self.persist
            && !self.faces_asked
            && self.dialog.is_none()
            && self.store.faces_enabled.is_none()
            && self.store.online_maps.is_some()
            && self.store.ai_enabled
            && crate::ai::runtime::available()
            && crate::faces::pack().is_some();
        if ask {
            self.faces_asked = true;
            self.dialog = Some(Dialog::Faces);
        }
    }

    /// The user wants faces found: turn them on, and download the Faces pack if it isn't here
    /// (faces are found meanwhile; they get their vectors once it is).
    pub fn turn_faces_on(&mut self) {
        self.store.faces_enabled = Some(true);
        self.dirty = true;
        self.smart.recheck();
        if !crate::faces::pack_installed()
            && let Some(p) = crate::faces::pack()
        {
            let id = p.id.clone();
            if !self.downloading_pack(&id) {
                self.download_pack(&id);
            }
        }
    }

    /// Faces found and grouped: once at the start, every `REGROUP_EVERY` seconds while the
    /// analysis finds more, and when it ends.
    pub fn faces_tick(&mut self, now: f64) {
        let on = self.store.ai_enabled && crate::ai::runtime::available() && self.store.faces_enabled == Some(true) && crate::faces::pack().is_some();
        if !on {
            self.face_groups = FaceGroups::default();
            return;
        }
        let active = self.analysis.active() && !self.analysis.paused;
        let due = match self.face_groups.asked_at {
            None => true,
            Some(at) => (active && now - at >= REGROUP_EVERY) || (self.face_groups.was_active && !active),
        };
        self.face_groups.was_active = active;
        if due {
            self.face_groups.asked_at = Some(now);
            self.lib.group_faces();
        }
    }
}
