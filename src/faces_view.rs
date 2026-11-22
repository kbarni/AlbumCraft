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
    /// The names and merges the lists were made with (changed by the People page, a command,
    /// an Undo: whichever it is, the lists follow).
    seen: Option<(std::collections::BTreeMap<u32, String>, std::collections::BTreeMap<u32, u32>)>,
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
}

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
        let on = self.store.ai_enabled && crate::ai::runtime::available() && self.store.faces_enabled == Some(true) && crate::faces::pack().is_some();
        if !on {
            self.face_groups = FaceGroups::default();
            return;
        }
        // names or merges changed since the lists were made
        let same = self.face_groups.seen.as_ref().is_some_and(|(n, m)| *n == self.store.person_names && *m == self.store.person_merged);
        if !same {
            let first = self.face_groups.seen.is_none();
            self.face_groups.seen = Some((self.store.person_names.clone(), self.store.person_merged.clone()));
            if !first {
                self.people_changed();
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
        self.lib.group_faces(crate::faces::constraints(&self.store), named);
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

    /// The groups or their names changed: the people of each picture, the proposed names and
    /// every list made from them.
    pub fn people_changed(&mut self) {
        let fg = &mut self.face_groups;
        fg.names = fg.index.as_ref().map(|i| crate::faces::names_by_picture(i, &self.store)).unwrap_or_default();
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
    fn paint_face(&mut self, ui: &egui::Ui, rect: Rect, face: &(String, u16), round: f32) {
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
        let Some(index) = &self.face_groups.index else { return vec![] };
        let mut named = vec![];
        let mut unnamed = vec![];
        for g in &index.groups {
            let id = crate::faces::resolve(&self.store, g.id);
            let name = crate::faces::name_of(&self.store, id).map(str::to_string);
            if name.is_none() && g.faces.len() < crate::faces::MIN_GROUP {
                continue;
            }
            let p = Person { id, name: name.clone(), faces: g.faces.clone(), pictures: g.pictures, cover: g.cover.clone() };
            if name.is_some() { named.push(p) } else { unnamed.push(p) }
        }
        // a merge made since the last grouping: its groups are shown as one
        let mut by_id: Vec<Person> = vec![];
        for p in named {
            match by_id.iter_mut().find(|q| q.id == p.id) {
                Some(q) => {
                    q.pictures += p.pictures;
                    q.faces.extend(p.faces);
                }
                None => by_id.push(p),
            }
        }
        by_id.sort_by_key(|p| p.name.as_deref().map(crate::tags::key));
        by_id.extend(unnamed);
        by_id
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
            let cover = crate::faces::group_named(&self.store, name, None).and_then(|g| self.face_groups.index.as_ref()?.group(g).map(|g| g.cover.clone()));
            if self.person_row(ui, cover.as_ref(), name, Some(*n), sel).clicked() {
                self.source = Source::Person(name.clone());
                self.pick_set(TraySet::Person(name.clone()));
            }
        }
        if faces_on {
            let unnamed: Vec<Person> = self.people_list().into_iter().filter(|p| p.name.is_none()).collect();
            for p in unnamed.iter().take(SIDEBAR_UNNAMED) {
                let sel = self.source == Source::Group(p.id);
                if self.person_row(ui, Some(&p.cover), &tr!("people-unnamed"), Some(p.pictures), sel).clicked() {
                    self.source = Source::Group(p.id);
                    self.pick_set(TraySet::Group(p.id));
                }
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

    /// A sidebar row with the person's face in front (or the person icon).
    fn person_row(&mut self, ui: &mut egui::Ui, cover: Option<&(String, u16)>, label: &str, count: Option<usize>, selected: bool) -> egui::Response {
        let Some(cover) = cover else { return crate::shell::nav_row(ui, crate::icons::PERSON, label, count, selected) };
        let resp = crate::shell::nav_row(ui, "", label, count, selected);
        let face = Rect::from_center_size(Pos2::new(resp.rect.left() + 18.0, resp.rect.center().y), Vec2::splat(22.0));
        self.paint_face(ui, face, cover, 11.0);
        resp
    }

    /// The People page (`Source::People`): every person, with a name field and their faces.
    pub fn people_page(&mut self, ui: &mut egui::Ui) {
        let list = self.people_list();
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
                if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                    open = Some(self.source_of(p));
                }
            }
            if p.faces.len() > shown {
                let (r, resp) = ui.allocate_exact_size(Vec2::splat(FACE), Sense::click());
                ui.painter().rect_filled(r, 6.0, t().surface_hover);
                ui.painter().text(r.center(), egui::Align2::CENTER_CENTER, format!("+{}", p.faces.len() - shown), egui::FontId::proportional(15.0), t().text_weak);
                if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                    open = Some(self.source_of(p));
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
}
