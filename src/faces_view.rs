//! Faces in the window: the question asked once whether to find them, turning them on, the
//! people of a picture (names typed on it and people recognised in it, one list), the People
//! section of the sidebar and the People page, where groups of faces are checked and named.
//! (Corrections of single faces come next.)

use crate::app::{App, Dialog, Source, TraySet, Undo};
use crate::face_groups::GroupId;
use crate::i18n::tr;
use crate::theme::t;
use eframe::egui::{self, Pos2, Rect, RichText, Sense, Vec2};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

/// Face crops kept as textures at most (128 px each); beyond, they are all dropped and asked
/// again as they are drawn.
const MAX_CROPS: usize = 1500;
/// Unnamed groups listed in the sidebar; the others are on the People page.
const SIDEBAR_UNNAMED: usize = 6;
/// The People page: one block per person.
const BLOCK_H: f32 = 150.0;
const COVER: f32 = 56.0;
const FACE: f32 = 64.0;

/// How often the faces are grouped again while the library is being analysed.
const REGROUP_EVERY: f64 = 30.0;

/// The library's faces and their groups, as last grouped.
#[derive(Default)]
pub struct FaceGroups {
    pub index: Option<Arc<crate::faces::FaceIndex>>,
    asked_at: Option<f64>,
    was_active: bool,
    /// The library's pictures at the last grouping (a scan changes them).
    pictures: usize,
    /// What the user decided (names, merges, corrections…) as the lists were made, and the
    /// list revision it was looked at (`decisions_hash`): changed by the People page, a
    /// command, an Undo, the lists and the groups follow.
    decisions: Option<u64>,
    decisions_rev: u64,
    /// Faces selected on a person's faces page.
    pub selected: HashSet<(String, u16)>,
    /// The hidden groups are listed on the People page.
    show_hidden: bool,
    /// Picture -> the names of the people recognised in it.
    pub names: HashMap<String, Vec<String>>,
    /// Names proposed for unnamed groups (from the names typed on their pictures).
    suggestions: HashMap<GroupId, String>,
    crops: HashMap<(String, u16), egui::TextureHandle>,
    asked: HashSet<(String, u16)>,
    /// Name fields of the People page being typed in.
    editing: HashMap<GroupId, String>,
    /// A name typed for a group that is another group's: (group, the other, the name).
    merge_question: Option<(GroupId, GroupId, String)>,
    /// Groups the user took faces out of: their other faces are looked at again with the next grouping.
    recheck: HashSet<GroupId>,
}

/// A person dragged in the sidebar (onto another: one person).
#[derive(Clone, Copy, Debug)]
pub struct DraggedPerson(pub GroupId);

pub use crate::faces::Correction;

/// Faces on a person's faces page.
const GRID_FACE: f32 = 96.0;
const GRID_GAP: f32 = 6.0;

/// One block of the People page.
struct Person {
    id: GroupId,
    name: Option<String>,
    faces: Vec<(String, u16)>,
    pictures: usize,
    cover: (String, u16),
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
        // pictures kept out of face detection (menus, Settings, a command): the analysis leaves
        // them out, and the change is a decision the groups follow
        if self.smart.faces_excluded_told.as_ref() != Some(&self.store.faces_excluded) {
            let first = self.smart.faces_excluded_told.replace(self.store.faces_excluded.clone()).is_none();
            self.lib.exclude_from_faces(self.store.faces_excluded.clone());
            if !first {
                self.list_rev += 1;
            }
        }
        let on = self.store.ai_enabled && crate::ai::runtime::available() && self.store.faces_enabled == Some(true) && crate::faces::pack().is_some();
        if !on {
            self.face_groups = FaceGroups::default();
            return;
        }
        // what the user decided changed since the lists were made (looked at once per change
        // of the notes, which every change of them goes with)
        if self.face_groups.decisions.is_none() || self.face_groups.decisions_rev != self.list_rev {
            self.face_groups.decisions_rev = self.list_rev;
            let h = decisions_hash(&self.store);
            let before = self.face_groups.decisions.replace(h);
            if before.is_some_and(|b| b != h) {
                self.people_changed();
                self.face_groups.decisions_rev = self.list_rev;
                if self.scanned {
                    self.regroup_faces();
                }
            }
        }
        // not before the library's pictures are known; again as soon as they change (a scan)
        if !self.scanned {
            return;
        }
        let active = self.analysis.active() && !self.analysis.paused;
        let pictures = self.photos.len();
        let due = match self.face_groups.asked_at {
            None => true,
            Some(at) => pictures != self.face_groups.pictures || (active && now - at >= REGROUP_EVERY) || (self.face_groups.was_active && !active),
        };
        self.face_groups.was_active = active;
        self.face_groups.pictures = pictures;
        if due {
            self.face_groups.asked_at = Some(now);
            self.regroup_faces();
        }
    }

    /// Group the faces again now, with what the user decided.
    pub fn regroup_faces(&mut self) {
        let named = self.store.person_names.keys().copied().filter(|g| !self.store.person_merged.contains_key(g)).collect();
        let mut decided = crate::faces::decided(&self.store);
        decided.recheck = std::mem::take(&mut self.face_groups.recheck);
        self.lib.group_faces(decided, named);
    }

    /// Faces are being looked for (the menus offer to leave pictures out only then).
    pub fn finding_faces(&self) -> bool {
        self.store.ai_enabled && crate::ai::runtime::available() && self.store.faces_enabled == Some(true) && !self.using_samples
    }

    /// The context menu's item that keeps these pictures or this folder out of face detection
    /// (paintings, screenshots of games), or looks at them again.
    pub fn no_faces_menu(&mut self, ui: &mut egui::Ui, paths: &[String]) {
        if !self.finding_faces() || paths.is_empty() {
            return;
        }
        let by: Vec<Option<String>> = paths.iter().map(|p| self.store.no_faces(p).map(str::to_string)).collect();
        if by.iter().all(Option::is_some) {
            // out by a folder above them: taken back in only there
            let own = paths.iter().zip(&by).any(|(p, b)| b.as_deref() == Some(p.as_str()));
            if !own {
                let folder = by.first().cloned().flatten().unwrap_or_default();
                ui.add_enabled(false, egui::Button::new(tr!("ctx-faces-again"))).on_disabled_hover_text(tr!("ctx-no-faces-by-folder", folder = folder));
            } else if ui.button(tr!("ctx-faces-again")).on_hover_text(tr!("ctx-faces-again-hint")).clicked() {
                self.set_no_faces(paths, false);
                ui.close();
            }
        } else if ui.button(tr!("ctx-no-faces")).on_hover_text(tr!("ctx-no-faces-hint")).clicked() {
            self.set_no_faces(paths, true);
            ui.close();
        }
    }

    /// Keep pictures or folders out of face detection, or look at them again (`faces_tick`
    /// tells the analysis and groups again).
    pub fn set_no_faces(&mut self, paths: &[String], out: bool) {
        if self.store.set_no_faces(paths, out) {
            self.dirty = true;
            self.toast(if out { tr!("notice-no-faces") } else { tr!("notice-faces-again") });
        }
    }

    /// The people of a picture: names typed on it and people recognised in it.
    pub fn people_of(&self, id: usize) -> Vec<String> {
        let Some(p) = self.photos.get(id) else { return vec![] };
        crate::faces::people_of(self.store.meta.get(&p.key), self.face_groups.names.get(&p.key))
    }

    /// A face of the picture is in group `g` (after merges).
    pub fn in_group(&self, id: usize, g: GroupId) -> bool {
        let (Some(p), Some(index)) = (self.photos.get(id), self.face_groups.index.as_ref()) else { return false };
        let g = crate::faces::resolve(&self.store, g);
        index.pictures.get(&p.key).is_some_and(|f| f.iter().any(|f| f.group.is_some_and(|x| crate::faces::resolve(&self.store, x) == g)))
    }

    /// "Delete all face data": what was computed about faces goes (names and corrections stay),
    /// and faces are turned off, or the analysis would make them again.
    pub fn delete_all_face_data(&mut self) -> Result<(), String> {
        self.lib.delete_face_data()?;
        self.store.faces_enabled = Some(false);
        self.dirty = true;
        self.face_groups = FaceGroups::default();
        self.smart.recheck();
        self.people_changed();
        if matches!(self.source, Source::Faces(_) | Source::Group(_) | Source::People) {
            self.source = Source::Library;
        }
        Ok(())
    }

    /// The groups or their names changed: the people of each picture, the proposed names and
    /// every list made from them.
    pub fn people_changed(&mut self) {
        let fg = &mut self.face_groups;
        // faces drawn or imported by hand name their pictures' people with no analysis too
        fg.names = match fg.index.as_ref() {
            Some(i) => crate::faces::names_by_picture(i, &self.store),
            None => crate::faces::names_by_picture(&Default::default(), &self.store),
        };
        fg.suggestions = fg.index.as_ref().map(|i| crate::faces::suggestions(i, &self.store).into_iter().collect()).unwrap_or_default();
        self.list_rev += 1;
    }

    pub fn face_crops_arrived(&mut self, key: String, crops: Vec<(u16, Option<image::RgbaImage>)>) {
        if self.face_groups.crops.len() > MAX_CROPS {
            self.face_groups.crops.clear();
            self.face_groups.asked.clear();
        }
        for (i, img) in crops {
            if let Some(img) = img {
                let tex = self.ctx.load_texture(format!("face{key}#{i}"), crate::app::to_color_image(&img), egui::TextureOptions::LINEAR);
                self.face_groups.crops.insert((key.clone(), i), tex);
            }
        }
    }

    /// The texture of a face, asked for when it is not there yet.
    fn face_texture(&mut self, key: &str, index: u16) -> Option<egui::TextureId> {
        let k = (key.to_string(), index);
        if let Some(t) = self.face_groups.crops.get(&k) {
            return Some(t.id());
        }
        if self.face_groups.asked.insert(k) {
            let rect = self.face_groups.index.as_ref().and_then(|i| i.face(key, index)).map(|f| f.rect)?;
            self.lib.request(crate::library::Job::FaceCrops { key: key.to_string(), faces: vec![(index, rect)] }, true);
        }
        None
    }

    /// Draw a face in `rect` (a grey square with a person until it is there).
    pub(crate) fn paint_face(&mut self, ui: &egui::Ui, rect: Rect, face: &(String, u16), round: f32) {
        if !ui.is_rect_visible(rect) {
            return;
        }
        match self.face_texture(&face.0, face.1) {
            Some(tex) => {
                egui::Image::new((tex, rect.size())).corner_radius(round).paint_at(ui, rect);
            }
            None => {
                ui.painter().rect_filled(rect, round, t().surface_hover);
                ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, crate::icons::PERSON, egui::FontId::proportional(rect.height() * 0.45), t().text_weak);
            }
        }
    }

    /// The people to show, named ones first (by name), then the unnamed groups, largest first.
    fn people_list(&self) -> Vec<Person> {
        self.people_list_with(false)
    }

    /// The people (or, `hidden`, only the groups the user hid).
    fn people_list_with(&self, hidden: bool) -> Vec<Person> {
        let Some(index) = &self.face_groups.index else { return vec![] };
        // a merge made since the last grouping: its groups are shown as one
        let mut by_id: Vec<Person> = vec![];
        for g in &index.groups {
            let id = crate::faces::resolve(&self.store, g.id);
            match by_id.iter_mut().find(|q| q.id == id) {
                Some(q) => {
                    q.pictures += g.pictures;
                    q.faces.extend(g.faces.iter().cloned());
                }
                None => by_id.push(Person { id, name: crate::faces::name_of(&self.store, id).map(str::to_string), faces: g.faces.clone(), pictures: g.pictures, cover: g.cover.clone() }),
            }
        }
        by_id.retain(|p| (p.name.is_some() || p.faces.len() >= crate::faces::MIN_GROUP) && self.store.hidden_groups.contains(&p.id) == hidden);
        let (mut named, mut unnamed): (Vec<Person>, Vec<Person>) = by_id.into_iter().partition(|p| p.name.is_some());
        named.sort_by_key(|p| p.name.as_deref().map(crate::tags::key));
        unnamed.sort_by(|a, b| b.faces.len().cmp(&a.faces.len()).then(a.id.cmp(&b.id)));
        named.extend(unnamed);
        named
    }

    /// People on the People page.
    pub fn people_count(&self) -> usize {
        self.people_list().len()
    }

    /// The People section of the sidebar: everyone with a name (typed or given to a group of
    /// faces), then the largest unnamed groups and the way to the People page.
    pub fn people_sidebar(&mut self, ui: &mut egui::Ui, people: &[(String, usize)]) {
        let faces_on = self.face_groups.index.is_some();
        if faces_on {
            let sel = self.source == Source::People;
            let groups = self.people_list().len();
            if crate::shell::nav_row(ui, crate::icons::PERSON, &tr!("people-all"), Some(groups), sel).clicked() {
                self.source = Source::People;
                self.tray_set = None;
            }
        }
        if people.is_empty() && !faces_on {
            ui.label(RichText::new(tr!("people-hint")).size(12.0).color(t().text_weak));
        }
        for (name, n) in people {
            let sel = self.source == Source::Person(name.clone());
            let group = crate::faces::group_named(&self.store, name, None);
            let cover = group.and_then(|g| self.face_groups.index.as_ref()?.group(g).map(|g| g.cover.clone()));
            let resp = self.person_row(ui, cover.as_ref(), name, Some(*n), sel);
            if resp.clicked() {
                self.source = Source::Person(name.clone());
                self.pick_set(TraySet::Person(name.clone()));
            }
            if let Some(g) = group {
                self.person_drag(ui, &resp, g);
            }
            resp.context_menu(|ui| {
                if ui.button(tr!("person-edit")).clicked() {
                    self.edit_person(Some(name.clone()), group);
                    ui.close();
                }
            });
        }
        if faces_on {
            let unnamed: Vec<Person> = self.people_list().into_iter().filter(|p| p.name.is_none()).collect();
            for p in unnamed.iter().take(SIDEBAR_UNNAMED) {
                let sel = self.source == Source::Group(p.id);
                let resp = self.person_row(ui, Some(&p.cover), &tr!("people-unnamed"), Some(p.pictures), sel);
                if resp.clicked() {
                    self.source = Source::Group(p.id);
                    self.pick_set(TraySet::Group(p.id));
                }
                self.person_drag(ui, &resp, p.id);
                resp.context_menu(|ui| {
                    if ui.button(tr!("person-edit")).clicked() {
                        self.edit_person(None, Some(p.id));
                        ui.close();
                    }
                });
            }
            if unnamed.len() > SIDEBAR_UNNAMED {
                let more = unnamed.len() - SIDEBAR_UNNAMED;
                if crate::shell::nav_row(ui, "", &tr!("people-more", n = more), None, false).clicked() {
                    self.source = Source::People;
                    self.tray_set = None;
                }
            }
        }
    }

    /// The person whose pictures are shown: the group of faces (after merges) and the name
    /// (the group's, or the one typed on pictures).
    pub(crate) fn shown_person(&self) -> Option<(Option<GroupId>, Option<String>)> {
        match &self.source {
            Source::Person(name) => Some((crate::faces::group_named(&self.store, name, None), Some(name.clone()))),
            Source::Group(g) => Some((Some(crate::faces::resolve(&self.store, *g)), None)),
            Source::Faces(g) => {
                let g = crate::faces::resolve(&self.store, *g);
                Some((Some(g), crate::faces::name_of(&self.store, g).map(str::to_string)))
            }
            _ => None,
        }
    }

    /// The faces of group `g` in these pictures (picture, place among its faces).
    fn faces_in(&self, ids: &[usize], g: GroupId) -> Vec<(String, u16)> {
        let Some(index) = &self.face_groups.index else { return vec![] };
        let g = crate::faces::resolve(&self.store, g);
        let mut out = vec![];
        for &id in ids {
            let Some(p) = self.photos.get(id) else { continue };
            for f in index.pictures.get(&p.key).into_iter().flatten() {
                if f.group.map(|x| crate::faces::resolve(&self.store, x)) == Some(g) {
                    out.push((p.key.clone(), f.index));
                }
            }
        }
        out
    }

    /// In a person's pictures, the right-click entries: "Not John" (these pictures are not
    /// of that person: the faces are told apart from them, and a name typed on them goes) and
    /// "Make a new person of these" (the person is two look-alikes: the faces of the selected
    /// pictures become a person of their own, and the others that match them follow).
    pub fn person_menu(&mut self, ui: &mut egui::Ui, tray: &[usize]) {
        let Some((group, name)) = self.shown_person() else { return };
        let label = match &name {
            Some(n) => tr!("ctx-not-person", name = n.clone()),
            None => tr!("ctx-not-this-person"),
        };
        if ui.button(label).on_hover_text(tr!("ctx-not-person-hint")).clicked() {
            let key = name.as_deref().map(crate::tags::key);
            if let Some(k) = key {
                let keys: Vec<String> = tray.iter().filter_map(|&t| self.photos.get(t).map(|p| p.key.clone())).collect();
                for key in keys {
                    if let Some(m) = self.store.meta.get_mut(&key) {
                        m.people.retain(|n| crate::tags::key(n) != k);
                    }
                }
                self.dirty = true;
            }
            if let Some(g) = group {
                let faces = self.faces_in(tray, g);
                self.correct(&faces, Correction::NotThisPerson(g));
            }
            self.people_changed();
            self.selection.clear();
            ui.close();
        }
        if let Some(g) = group {
            let faces = self.faces_in(tray, g);
            if !faces.is_empty() && ui.button(tr!("ctx-split-person")).on_hover_text(tr!("ctx-split-person-hint")).clicked() {
                self.correct(&faces, Correction::NewPerson);
                self.selection.clear();
                ui.close();
            }
        }
    }

    /// Open the dialog for a person's name, caption and description.
    pub fn edit_person(&mut self, old: Option<String>, group: Option<GroupId>) {
        let (caption, text) = old.as_deref().and_then(|n| self.store.person_texts.get(&crate::tags::key(n)).cloned()).unwrap_or_default();
        self.dialog = Some(crate::app::Dialog::Person { name: old.clone().unwrap_or_default(), old, group, caption, text, error: String::new() });
    }

    /// Edit the person shown (the title line's pencil).
    pub fn edit_shown_person(&mut self) {
        if let Some((group, name)) = self.shown_person() {
            self.edit_person(name, group);
        }
    }

    /// The Person dialog's Save: rename (all the pictures, the group), keep the texts.
    pub fn save_person(&mut self, old: Option<&str>, group: Option<GroupId>, name: &str, caption: &str, text: &str) -> Result<(), String> {
        let name = name.trim();
        let renamed = old != Some(name) && !(old.is_none() && name.is_empty());
        if renamed {
            match crate::faces::rename_person(&mut self.store, old, group, name) {
                Ok(()) => {}
                Err(crate::faces::RenameError::Empty) => return Err(tr!("person-name-empty")),
                Err(crate::faces::RenameError::Taken(_)) => return Err(tr!("person-name-taken", name = name)),
            }
            if let Source::Person(n) = &self.source
                && Some(n.as_str()) == old
            {
                self.source = Source::Person(name.to_string());
            }
        }
        if !name.is_empty() {
            let key = crate::tags::key(name);
            if caption.trim().is_empty() && text.trim().is_empty() {
                self.store.person_texts.remove(&key);
            } else {
                self.store.person_texts.insert(key, (caption.trim().to_string(), text.trim().to_string()));
            }
        }
        self.dirty = true;
        self.list_rev += 1;
        self.people_changed();
        Ok(())
    }

    /// A person's caption and description.
    pub fn person_texts(&self, name: &str) -> (String, String) {
        self.store.person_texts.get(&crate::tags::key(name)).cloned().unwrap_or_default()
    }

    /// A person's sidebar row can be dragged onto another: they are one person (with Undo).
    fn person_drag(&mut self, ui: &egui::Ui, resp: &egui::Response, g: GroupId) {
        resp.dnd_set_drag_payload(DraggedPerson(g));
        if resp.dnd_hover_payload::<DraggedPerson>().is_some_and(|d| crate::faces::resolve(&self.store, d.0) != crate::faces::resolve(&self.store, g)) {
            ui.painter().rect_stroke(resp.rect, 9.0, egui::Stroke::new(2.0, t().accent), egui::StrokeKind::Inside);
        }
        if let Some(d) = resp.dnd_release_payload::<DraggedPerson>()
            && crate::faces::resolve(&self.store, d.0) != crate::faces::resolve(&self.store, g)
        {
            let name = crate::faces::name_of(&self.store, g).or_else(|| crate::faces::name_of(&self.store, d.0)).map(str::to_string).unwrap_or_else(|| tr!("people-unnamed"));
            self.merge_into(d.0, g, &name);
        }
        // what is dragged, under the pointer
        if resp.dragged()
            && let Some(pos) = ui.ctx().pointer_interact_pos()
        {
            let label = crate::faces::name_of(&self.store, g).map(str::to_string).unwrap_or_else(|| tr!("people-unnamed"));
            let p = ui.ctx().layer_painter(egui::LayerId::new(egui::Order::Tooltip, egui::Id::unique("dragged-person")));
            let g = p.layout_no_wrap(label, egui::FontId::proportional(13.0), t().text_strong);
            let r = Rect::from_min_size(pos + Vec2::new(14.0, 6.0), g.size() + Vec2::new(16.0, 8.0));
            p.rect_filled(r, 8.0, t().surface);
            p.rect_stroke(r, 8.0, egui::Stroke::new(1.0, t().border), egui::StrokeKind::Inside);
            p.galley(r.min + Vec2::new(8.0, 4.0), g, t().text_strong);
        }
    }

    /// A sidebar row with the person's face in front (or the person icon).
    fn person_row(&mut self, ui: &mut egui::Ui, cover: Option<&(String, u16)>, label: &str, count: Option<usize>, selected: bool) -> egui::Response {
        let Some(cover) = cover else { return crate::shell::nav_row_drag(ui, crate::icons::PERSON, label, count, selected) };
        let resp = crate::shell::nav_row_drag(ui, "", label, count, selected);
        let face = Rect::from_center_size(Pos2::new(resp.rect.left() + 18.0, resp.rect.center().y), Vec2::splat(22.0));
        self.paint_face(ui, face, cover, 11.0);
        resp
    }

    /// The People page (`Source::People`): every person, with a name field and their faces.
    pub fn people_page(&mut self, ui: &mut egui::Ui) {
        let mut list = self.people_list();
        let hidden = self.people_list_with(true);
        if self.face_groups.show_hidden {
            list.extend(hidden.iter().map(|p| Person { id: p.id, name: p.name.clone(), faces: p.faces.clone(), pictures: p.pictures, cover: p.cover.clone() }));
        }
        egui::Frame::new().inner_margin(egui::Margin { left: 24, right: 24, top: 14, bottom: 6 }).show(ui, |ui| {
            if self.face_groups.index.is_none() {
                ui.label(RichText::new(tr!("people-page-off")).color(t().text_weak));
                return;
            }
            let named = list.iter().filter(|p| p.name.is_some()).count();
            let unnamed = list.len() - named;
            ui.label(RichText::new(tr!("people-page-count", named = named, unnamed = unnamed)).color(t().text_weak));
            if self.analysis.active() {
                ui.label(RichText::new(tr!("people-page-analysing")).size(12.0).color(t().text_weak));
            }
            if !hidden.is_empty() {
                ui.checkbox(&mut self.face_groups.show_hidden, tr!("faces-show-hidden", n = hidden.len()));
            }
        });
        if list.is_empty() {
            return;
        }
        let mut open: Option<Source> = None;
        egui::ScrollArea::vertical().auto_shrink([false, false]).show_rows(ui, BLOCK_H, list.len(), |ui, range| {
            for p in &list[range] {
                egui::Frame::new().inner_margin(egui::Margin { left: 24, right: 24, top: 6, bottom: 6 }).show(ui, |ui| {
                    ui.set_height(BLOCK_H - 12.0);
                    if let Some(src) = self.person_block(ui, p) {
                        open = Some(src);
                    }
                });
            }
        });
        if let Some(src) = open {
            self.tray_set = None;
            match &src {
                Source::Person(n) => self.pick_set(TraySet::Person(n.clone())),
                Source::Group(g) => self.pick_set(TraySet::Group(*g)),
                _ => {}
            }
            self.source = src;
        }
    }

    /// One person on the People page; gives the source to open when its pictures are asked for.
    fn person_block(&mut self, ui: &mut egui::Ui, p: &Person) -> Option<Source> {
        let mut open = None;
        ui.horizontal(|ui| {
            let (cover, resp) = ui.allocate_exact_size(Vec2::splat(COVER), Sense::click());
            self.paint_face(ui, cover, &p.cover, 8.0);
            if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                open = Some(self.source_of(p));
            }
            ui.add_space(10.0);
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    // the text being typed is kept only while typing; otherwise the name as it is
                    let current = p.name.clone().unwrap_or_default();
                    let mut text = self.face_groups.editing.get(&p.id).cloned().unwrap_or_else(|| current.clone());
                    let field = ui.add(egui::TextEdit::singleline(&mut text).hint_text(tr!("people-add-name")).desired_width(220.0));
                    let done = field.lost_focus() || (field.has_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)));
                    if field.has_focus() && !done {
                        self.face_groups.editing.insert(p.id, text.clone());
                    } else {
                        self.face_groups.editing.remove(&p.id);
                    }
                    if done && text.trim() != current {
                        self.name_group(p.id, &text);
                    }
                    ui.label(RichText::new(tr!("people-pictures", n = p.pictures)).color(t().text_weak));
                    if ui.button(tr!("people-show-pictures")).clicked() {
                        open = Some(self.source_of(p));
                    }
                    if ui.button(tr!("faces-check")).on_hover_text(tr!("faces-check-hint")).clicked() {
                        open = Some(Source::Faces(p.id));
                    }
                    let hidden = self.store.hidden_groups.contains(&p.id);
                    let label = if hidden { tr!("faces-unhide") } else { tr!("faces-hide") };
                    if ui.button(label).on_hover_text(tr!("faces-hide-hint")).clicked() {
                        if hidden {
                            self.store.hidden_groups.remove(&p.id);
                        } else {
                            self.store.hidden_groups.insert(p.id);
                        }
                        self.dirty = true;
                    }
                });
                // a name typed that is someone else's: one person?
                if let Some((g, other, name)) = self.face_groups.merge_question.clone().filter(|q| q.0 == p.id) {
                    ui.horizontal_wrapped(|ui| {
                        ui.label(tr!("people-merge-question", name = name.as_str()));
                        if ui.button(tr!("people-merge-yes", name = name.as_str())).clicked() {
                            self.merge_into(g, other, &name);
                        }
                        if ui.button(tr!("button-cancel")).clicked() {
                            self.face_groups.merge_question = None;
                        }
                    });
                } else if let Some(other) = self.same_person(p.id) {
                    // two groups that may be one person
                    ui.horizontal(|ui| {
                        let who = crate::faces::name_of(&self.store, other).map(str::to_string);
                        let text = match &who {
                            Some(n) => tr!("faces-same-person", name = n.as_str()),
                            None => tr!("faces-same-person-unnamed"),
                        };
                        ui.label(RichText::new(text).color(t().text_weak));
                        let cover = self.face_groups.index.as_ref().and_then(|i| i.groups.iter().find(|g| crate::faces::resolve(&self.store, g.id) == other).map(|g| g.cover.clone()));
                        if let Some(c) = cover {
                            let (r, _) = ui.allocate_exact_size(Vec2::splat(24.0), Sense::hover());
                            self.paint_face(ui, r, &c, 12.0);
                        }
                        if ui.small_button(tr!("faces-merge")).clicked() {
                            let name = who.or_else(|| p.name.clone()).unwrap_or_else(|| tr!("people-unnamed"));
                            self.merge_into(p.id, other, &name);
                        }
                        if ui.small_button(tr!("people-no")).clicked() {
                            self.store.refused_merges.push((p.id.min(other), p.id.max(other)));
                            self.dirty = true;
                        }
                    });
                } else if p.name.is_none()
                    && let Some(name) = self.face_groups.suggestions.get(&p.id).cloned()
                {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(tr!("people-is-it", name = name.as_str())).color(t().text_weak));
                        if ui.small_button(tr!("people-yes")).clicked() {
                            self.name_group(p.id, &name);
                        }
                        if ui.small_button(tr!("people-no")).clicked() {
                            self.store.refused_names.push((p.id, crate::tags::key(&name)));
                            self.dirty = true;
                            self.people_changed();
                        }
                    });
                }
            });
        });
        ui.add_space(6.0);
        // as many faces as one row holds, then how many more
        let room = ui.available_width();
        let fit = ((room + 4.0) / (FACE + 4.0)).floor().max(1.0) as usize;
        let shown = if p.faces.len() > fit { fit - 1 } else { p.faces.len() };
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
            let step = (p.faces.len() / shown.max(1)).max(1);
            for f in p.faces.iter().step_by(step).take(shown) {
                let (r, resp) = ui.allocate_exact_size(Vec2::splat(FACE), Sense::click());
                self.paint_face(ui, r, f, 6.0);
                let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand);
                if resp.clicked() {
                    open = Some(Source::Faces(p.id));
                }
                resp.context_menu(|ui| {
                    if let Some(c) = self.correction_menu(ui, p.id, 1) {
                        self.correct(std::slice::from_ref(f), c);
                    }
                });
            }
            if p.faces.len() > shown {
                let (r, resp) = ui.allocate_exact_size(Vec2::splat(FACE), Sense::click());
                ui.painter().rect_filled(r, 6.0, t().surface_hover);
                ui.painter().text(r.center(), egui::Align2::CENTER_CENTER, format!("+{}", p.faces.len() - shown), egui::FontId::proportional(15.0), t().text_weak);
                if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                    open = Some(Source::Faces(p.id));
                }
            }
        });
        open
    }

    fn source_of(&self, p: &Person) -> Source {
        match &p.name {
            Some(n) => Source::Person(n.clone()),
            None => Source::Group(p.id),
        }
    }

    fn people_undo(&self) -> Undo {
        Undo::People { names: self.store.person_names.clone(), merged: self.store.person_merged.clone(), refused: self.store.refused_names.clone() }
    }

    /// Give group `g` a name (typed, or a proposed one taken). A name that is another
    /// group's asks first whether they are one person.
    pub fn name_group(&mut self, g: GroupId, name: &str) {
        match crate::faces::set_name(&mut self.store, g, name, false) {
            Ok(_) => {
                // the lists and a new grouping (which keeps a new name's reference faces)
                // follow in `faces_tick`
                self.face_groups.merge_question = None;
                self.dirty = true;
            }
            Err(other) => self.face_groups.merge_question = Some((g, other, name.trim().to_string())),
        }
    }

    /// The user said group `g` is the person of group `into`.
    fn merge_into(&mut self, g: GroupId, into: GroupId, name: &str) {
        let undo = self.people_undo();
        crate::faces::merge_groups(&mut self.store, g, into);
        self.face_groups.merge_question = None;
        self.dirty = true;
        self.undoable(tr!("people-merged", name = name), undo);
    }

    /// The people filter's part of the rating filter menu (only once faces are found).
    pub fn people_filter_menu(&mut self, ui: &mut egui::Ui) {
        if self.face_groups.index.is_none() {
            return;
        }
        use crate::app::PeopleFilter as P;
        ui.separator();
        ui.label(RichText::new(tr!("filter-people")).strong());
        for (f, label) in [(P::Any, tr!("filter-people-any")), (P::WithPeople, tr!("filter-people-with")), (P::Portrait, tr!("filter-people-portrait")), (P::Group, tr!("filter-people-group")), (P::NoPeople, tr!("filter-people-none"))] {
            if ui.radio(self.filter.people == f, label).clicked() {
                self.filter.people = f;
            }
        }
    }

    /// The faces of group `g` (after merges), in the library's order.
    pub fn faces_of(&self, g: GroupId) -> Vec<(String, u16)> {
        let g = crate::faces::resolve(&self.store, g);
        let Some(index) = &self.face_groups.index else { return vec![] };
        index.groups.iter().filter(|x| crate::faces::resolve(&self.store, x.id) == g).flat_map(|x| x.faces.iter().cloned()).collect()
    }

    /// Correct these faces (picture, place among its faces); an Undo notice says what was done.
    pub fn correct(&mut self, faces: &[(String, u16)], c: Correction) {
        let Some(index) = self.face_groups.index.clone() else { return };
        let mut before: Vec<(String, Vec<crate::faces::FaceNote>)> = vec![];
        for (key, _) in faces {
            if !before.iter().any(|(k, _)| k == key) {
                before.push((key.clone(), self.store.meta.get(key).map(|m| m.faces.clone()).unwrap_or_default()));
            }
        }
        // the groups the faces leave: their other faces are looked at again
        let from: Vec<GroupId> = faces.iter().filter_map(|(k, i)| index.face(k, *i)?.group).map(|g| crate::faces::resolve(&self.store, g)).collect();
        let done = crate::faces::correct(&mut self.store, &index, faces, c);
        if done == 0 {
            return;
        }
        if matches!(c, Correction::MoveTo(_) | Correction::NewPerson) {
            self.face_groups.recheck.extend(from);
        }
        self.face_groups.selected.clear();
        self.dirty = true;
        self.undoable(tr!("faces-corrected", n = done), Undo::Faces(before));
    }

    /// The corrections for `faces` of group `from`, as menu entries; what was chosen.
    fn correction_menu(&self, ui: &mut egui::Ui, from: GroupId, n: usize) -> Option<Correction> {
        let mut chosen = None;
        ui.menu_button(tr!("faces-move-to"), |ui| {
            if ui.button(tr!("faces-new-person")).clicked() {
                chosen = Some(Correction::NewPerson);
            }
            ui.separator();
            egui::ScrollArea::vertical().max_height(320.0).show(ui, |ui| {
                let from = crate::faces::resolve(&self.store, from);
                for p in self.people_list().into_iter().filter(|p| p.id != from).take(60) {
                    let label = p.name.clone().unwrap_or_else(|| tr!("faces-unnamed-n", n = p.pictures));
                    if ui.button(label).clicked() {
                        chosen = Some(Correction::MoveTo(p.id));
                    }
                }
            });
        });
        if ui.button(tr!("faces-not-this-person", n = n)).clicked() {
            chosen = Some(Correction::NotThisPerson(from));
        }
        if ui.button(tr!("faces-ignore", n = n)).on_hover_text(tr!("faces-ignore-hint")).clicked() {
            chosen = Some(Correction::Ignore);
        }
        if ui.button(tr!("faces-not-a-face", n = n)).clicked() {
            chosen = Some(Correction::NotFace);
        }
        if chosen.is_some() {
            ui.close();
        }
        chosen
    }

    /// A person's faces (`Source::Faces`): select some (a click), correct them (the bar, or a
    /// right-click).
    pub fn faces_page(&mut self, ui: &mut egui::Ui, g: GroupId) {
        let faces = self.faces_of(g);
        let mut go: Option<Source> = None;
        let mut apply: Option<(Vec<(String, u16)>, Correction)> = None;
        egui::Frame::new().inner_margin(egui::Margin { left: 24, right: 24, top: 12, bottom: 8 }).show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                if ui.button(format!("{}  {}", crate::icons::ARROW_BACK, tr!("faces-back"))).clicked() {
                    go = Some(Source::People);
                }
                if ui.button(tr!("people-show-pictures")).clicked() {
                    go = Some(match crate::faces::name_of(&self.store, g) {
                        Some(n) => Source::Person(n.to_string()),
                        None => Source::Group(g),
                    });
                }
                ui.separator();
                let n = self.face_groups.selected.len();
                if n == 0 {
                    ui.label(RichText::new(tr!("faces-hint")).color(t().text_weak));
                } else {
                    ui.label(RichText::new(tr!("faces-selected", n = n)).strong());
                    let chosen = ui.horizontal(|ui| self.correction_menu(ui, g, n)).inner;
                    if let Some(c) = chosen {
                        apply = Some((self.face_groups.selected.iter().cloned().collect(), c));
                    }
                    if ui.button(tr!("faces-clear")).clicked() {
                        self.face_groups.selected.clear();
                    }
                }
                if ui.button(tr!("faces-select-all")).clicked() {
                    self.face_groups.selected = faces.iter().cloned().collect();
                }
            });
        });
        let cols = (((ui.available_width() - 48.0 + GRID_GAP) / (GRID_FACE + GRID_GAP)).floor() as usize).max(1);
        let rows = faces.len().div_ceil(cols);
        egui::ScrollArea::vertical().auto_shrink([false, false]).show_rows(ui, GRID_FACE + GRID_GAP, rows, |ui, range| {
            for row in range {
                ui.horizontal(|ui| {
                    ui.add_space(24.0);
                    ui.spacing_mut().item_spacing.x = GRID_GAP;
                    for f in faces.iter().skip(row * cols).take(cols) {
                        let (r, resp) = ui.allocate_exact_size(Vec2::splat(GRID_FACE), Sense::click());
                        self.paint_face(ui, r, f, 8.0);
                        let selected = self.face_groups.selected.contains(f);
                        if selected {
                            ui.painter().rect_stroke(r.shrink(1.5), 8.0, egui::Stroke::new(3.0, t().accent), egui::StrokeKind::Inside);
                            ui.painter().circle_filled(r.right_top() + Vec2::new(-14.0, 14.0), 9.0, t().accent);
                            ui.painter().text(r.right_top() + Vec2::new(-14.0, 14.0), egui::Align2::CENTER_CENTER, crate::icons::CHECK, egui::FontId::proportional(12.0), egui::Color32::WHITE);
                        }
                        if resp.clicked() {
                            if selected {
                                self.face_groups.selected.remove(f);
                            } else {
                                self.face_groups.selected.insert(f.clone());
                            }
                        }
                        // right-click: the selection, or this face alone when it is not selected
                        resp.context_menu(|ui| {
                            let these: Vec<(String, u16)> = if selected { self.face_groups.selected.iter().cloned().collect() } else { vec![f.clone()] };
                            if let Some(c) = self.correction_menu(ui, g, these.len()) {
                                apply = Some((these, c));
                            }
                        });
                    }
                });
            }
        });
        if let Some((these, c)) = apply {
            self.correct(&these, c);
        }
        if let Some(src) = go {
            self.face_groups.selected.clear();
            self.source = src;
        }
    }

    /// The closest other person this one may be ("Same person?"), when not answered yet.
    fn same_person(&self, p: GroupId) -> Option<GroupId> {
        let index = self.face_groups.index.as_ref()?;
        let resolve = |g| crate::faces::resolve(&self.store, g);
        let me_named = crate::faces::name_of(&self.store, p).is_some();
        index
            .proposals
            .iter()
            .filter_map(|&(small, large, score)| {
                let (a, b) = (resolve(small), resolve(large));
                let other = if a == p { b } else if b == p { a } else { return None };
                // asked on the smaller group, or on the unnamed one of a named pair
                let other_named = crate::faces::name_of(&self.store, other).is_some();
                let here = if me_named != other_named { !me_named } else { a == p };
                let refused = self.store.refused_merges.contains(&(p.min(other), p.max(other)));
                (here && other != p && !refused && !self.store.hidden_groups.contains(&other)).then_some((other, score))
            })
            .max_by(|x, y| x.1.total_cmp(&y.1))
            .map(|(g, _)| g)
    }
}

/// A number that changes with anything the user decided about faces and people.
fn decisions_hash(store: &crate::library::Store) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    store.person_names.hash(&mut h);
    store.person_merged.hash(&mut h);
    store.refused_names.hash(&mut h);
    store.refused_merges.hash(&mut h);
    store.hidden_groups.hash(&mut h);
    store.faces_excluded.hash(&mut h);
    // the notes on faces, in a fixed order
    let mut notes: Vec<(&String, String)> = store.meta.iter().filter(|(_, m)| !m.faces.is_empty()).map(|(k, m)| (k, serde_json::to_string(&m.faces).unwrap_or_default())).collect();
    notes.sort();
    notes.hash(&mut h);
    h.finish()
}