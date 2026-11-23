//! Faces in the editor: boxes with names over the picture (mapped through the picture's turns,
//! crop, straighten and perspective by `edit::geometry`), the info panel's Persons section and
//! "Add a face" (a box drawn by hand, kept as a `FaceState::Added` note).

use crate::app::{App, Tool};
use crate::face_groups::GroupId;
use crate::faces::{self, Correction, FaceState};
use crate::i18n::tr;
use crate::icons;
use crate::theme::t;
use eframe::egui::{self, Color32, Pos2, Rect, RichText, Stroke, StrokeKind, Vec2};

/// One face of the open picture as the editor shows it.
#[derive(Clone, Debug)]
pub struct EditorFace {
    /// Left, top, width, height in the unedited, upright picture.
    pub rect: [f32; 4],
    /// Its place among the picture's faces; None for a face drawn by hand.
    pub index: Option<u16>,
    pub group: Option<GroupId>,
    pub name: Option<String>,
}

impl App {
    /// The faces to show on picture `id`: found ones the user did not turn down, then faces drawn by hand.
    pub fn editor_faces(&self, id: usize) -> Vec<EditorFace> {
        let Some(photo) = self.photos.get(id) else { return vec![] };
        if self.store.faces_enabled != Some(true) {
            return vec![];
        }
        let mut out = vec![];
        if let Some(list) = self.face_groups.index.as_ref().and_then(|i| i.pictures.get(&photo.key)) {
            for f in list.iter().filter(|f| f.counted && !matches!(f.state, Some(FaceState::NotFace))) {
                let group = f.group.map(|g| faces::resolve(&self.store, g));
                let name = group.and_then(|g| faces::name_of(&self.store, g)).map(str::to_string);
                out.push(EditorFace { rect: f.rect, index: Some(f.index), group, name });
            }
        }
        for n in self.store.meta.get(&photo.key).into_iter().flat_map(|m| &m.faces) {
            if let FaceState::Added(g) = n.s {
                let g = faces::resolve(&self.store, g);
                let name = faces::name_of(&self.store, g).map(str::to_string);
                out.push(EditorFace { rect: n.r, index: None, group: Some(g), name });
            }
        }
        out
    }

    /// Draw the face boxes over the picture shown in `img_rect`.
    pub fn paint_face_boxes(&mut self, p: &egui::Painter, img_rect: Rect, id: usize) {
        if !self.edit.face_boxes {
            return;
        }
        let Some((w, h)) = self.real_dims_of(id) else { return };
        let edits = self.edits(id);
        let size = [w as f32, h as f32];
        for f in self.editor_faces(id) {
            let Some(r) = crate::geometry::rect_to_edited(&edits, size, f.rect) else { continue };
            let rect = Rect::from_min_size(
                Pos2::new(img_rect.left() + r[0] * img_rect.width(), img_rect.top() + r[1] * img_rect.height()),
                Vec2::new(r[2] * img_rect.width(), r[3] * img_rect.height()),
            );
            let colour = if f.name.is_some() { Color32::from_rgb(120, 220, 140) } else { Color32::from_rgb(255, 255, 255) };
            p.rect_stroke(rect.expand(1.0), 2, Stroke::new(3.0, Color32::from_black_alpha(120)), StrokeKind::Outside);
            p.rect_stroke(rect, 2, Stroke::new(1.8, colour), StrokeKind::Outside);
            if let Some(name) = &f.name {
                let galley = p.layout_no_wrap(name.clone(), egui::FontId::proportional(13.0), Color32::WHITE);
                let pill = Rect::from_min_size(Pos2::new(rect.left(), rect.bottom() + 3.0), galley.size() + egui::vec2(12.0, 6.0));
                p.rect_filled(pill, 4, Color32::from_rgba_unmultiplied(30, 30, 30, 215));
                p.galley(pill.min + egui::vec2(6.0, 3.0), galley, Color32::WHITE);
            }
        }
    }

    /// The box drawn with the "Add a face" tool, as `sel` (fractions of the edited picture):
    /// keep it on the picture in its own frame, as a face of a new, unnamed person.
    pub fn add_drawn_face(&mut self, id: usize, sel: [f32; 4]) {
        let Some((w, h)) = self.real_dims_of(id) else { return };
        let Some(key) = self.photos.get(id).map(|p| p.key.clone()) else { return };
        let r = [sel[0], sel[1], sel[2] - sel[0], sel[3] - sel[1]];
        let own = crate::geometry::rect_from_edited(&self.edits(id), [w as f32, h as f32], r);
        if own[2] < 0.005 || own[3] < 0.005 {
            return;
        }
        let g = match &self.face_groups.index {
            Some(index) => faces::new_person_id(&self.store, index),
            None => faces::new_person_id(&self.store, &Default::default()),
        };
        let before = self.store.meta.get(&key).map(|m| m.faces.clone()).unwrap_or_default();
        faces::add_face(self.store.meta.entry(key.clone()).or_default(), own, g);
        self.dirty = true;
        self.people_changed();
        self.undoable(tr!("face-added"), crate::app::Undo::Faces(vec![(key, before)]));
    }

    /// Name a face (found or drawn): the person with that name when there is one, else a new one.
    fn name_face(&mut self, id: usize, f: &EditorFace, name: &str) {
        let Some(key) = self.photos.get(id).map(|p| p.key.clone()) else { return };
        let name = name.trim();
        match (f.group, f.index) {
            (Some(g), _) if name.is_empty() => {
                // only this face's person loses the name when it is the one face; else leave it
                let _ = g;
            }
            (Some(g), None) => {
                // a hand-drawn face is alone in its group: name it, or join the person of that name
                match faces::group_named(&self.store, name, Some(g)) {
                    Some(other) => {
                        faces::add_face(self.store.meta.entry(key).or_default(), f.rect, other);
                        self.dirty = true;
                        self.people_changed();
                    }
                    None => self.name_group(g, name),
                }
            }
            (g, Some(i)) => {
                if let Some(other) = faces::group_named(&self.store, name, g) {
                    self.correct(&[(key, i)], Correction::MoveTo(other));
                } else if let Some(g) = g.filter(|g| self.faces_of(*g).len() <= 1) {
                    // the person's only face: naming the group names it
                    self.name_group(g, name);
                } else {
                    // one face of a group: it becomes a person of its own with that name
                    let index = self.face_groups.index.clone().unwrap_or_default();
                    let new = faces::new_person_id(&self.store, &index);
                    self.correct(&[(key, i)], Correction::MoveTo(new));
                    self.name_group(new, name);
                }
            }
            (None, None) => {}
        }
    }

    /// The Persons section of the info panel.
    pub fn info_persons(&mut self, ui: &mut egui::Ui, id: usize) {
        if !self.info_section(ui, "persons", &tr!("info-persons"), true) {
            return;
        }
        let faces_on = self.store.faces_enabled == Some(true);
        let list = self.editor_faces(id);
        let key = self.photos.get(id).map(|p| p.key.clone()).unwrap_or_default();
        if faces_on {
            ui.horizontal_wrapped(|ui| {
                ui.checkbox(&mut self.edit.face_boxes, tr!("persons-show-boxes"));
            });
            if list.is_empty() {
                ui.label(RichText::new(tr!("persons-no-faces")).size(12.5).color(t().text_weak));
            }
            let mut rename: Option<(EditorFace, String)> = None;
            let mut remove: Option<[f32; 4]> = None;
            // the library's names, for the drop-down under a name being typed
            let known: Vec<String> = self.sidebar_counts().people.iter().map(|(n, _)| n.clone()).collect();
            for (n, f) in list.iter().enumerate() {
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        let (rect, _) = ui.allocate_exact_size(Vec2::splat(36.0), egui::Sense::hover());
                        match f.index {
                            Some(i) => self.paint_face(ui, rect, &(key.clone(), i), 4.0),
                            None => {
                                ui.painter().rect_filled(rect, 4, t().surface_hover);
                                ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, icons::PERSON, egui::FontId::proportional(16.0), t().text_weak);
                            }
                        }
                        let slot = ui.scope_id().with(("face-name", n, f.index));
                        let current = f.name.clone().unwrap_or_default();
                        let mut text: String = ui.data_mut(|d| d.get_temp(slot)).unwrap_or_else(|| current.clone());
                        let w = (ui.available_width() - if f.index.is_none() { 30.0 } else { 0.0 }).max(60.0);
                        let r = ui.add(egui::TextEdit::singleline(&mut text).hint_text(tr!("persons-unnamed")).desired_width(w));
                        // where the drop-down was last frame: a click on it takes the focus off the
                        // field first, and must not count as the end of typing
                        let list_at: Option<Rect> = ui.data(|d| d.get_temp(slot.with("list")));
                        let on_list = list_at.is_some_and(|l| ui.input(|i| i.pointer.interact_pos()).is_some_and(|p| l.contains(p)));
                        if r.has_focus() || text != current {
                            ui.data_mut(|d| d.insert_temp(slot, text.clone()));
                        }
                        if r.lost_focus() && !on_list {
                            if text.trim() != current && !text.trim().is_empty() {
                                rename = Some((f.clone(), text.trim().to_string()));
                            }
                            ui.data_mut(|d| d.remove::<String>(slot));
                        }
                        if f.index.is_none() && ui.small_button(icons::CLOSE).on_hover_text(tr!("persons-remove-face")).clicked() {
                            remove = Some(f.rect);
                        }
                        // existing names that fit what is typed: a click gives the face to that person
                        let typed = crate::tags::key(&text);
                        let matches: Vec<&String> = if r.has_focus() || on_list {
                            known.iter().filter(|k| crate::tags::key(k) != crate::tags::key(&current) && crate::tags::key(k).contains(&typed)).take(6).collect()
                        } else {
                            vec![]
                        };
                        if matches.is_empty() {
                            ui.data_mut(|d| d.remove::<Rect>(slot.with("list")));
                        } else {
                            let at = egui::Area::new(slot.with("area")).order(egui::Order::Foreground).fixed_pos(r.rect.left_bottom() + egui::vec2(0.0, 2.0)).show(ui.ctx(), |ui| {
                                egui::Frame::popup(ui.style()).show(ui, |ui| {
                                    ui.set_min_width(r.rect.width());
                                    for name in &matches {
                                        if ui.add(egui::Button::new(format!("{} {name}", icons::PERSON)).frame(false).min_size(egui::vec2(r.rect.width() - 16.0, 0.0))).on_hover_text(tr!("persons-use-name")).clicked() {
                                            rename = Some((f.clone(), (*name).clone()));
                                            ui.data_mut(|d| d.remove::<String>(slot));
                                        }
                                    }
                                });
                            });
                            ui.data_mut(|d| d.insert_temp(slot.with("list"), at.response.rect));
                        }
                    });
                });
            }
            if let Some((f, name)) = rename {
                self.name_face(id, &f, &name);
            }
            if let Some(rect) = remove {
                if let Some(m) = self.store.meta.get_mut(&key) {
                    faces::note_face(m, rect, None);
                }
                self.dirty = true;
                self.people_changed();
            }
            let adding = self.edit.tool == Tool::AddFace;
            if ui.selectable_label(adding, format!("{}  {}", icons::PERSON, tr!("persons-add-face"))).on_hover_text(tr!("persons-add-face-hint")).clicked() {
                self.edit.tool = if adding { Tool::None } else { Tool::AddFace };
                self.edit.sel = None;
                self.edit.face_boxes = true;
            }
            ui.add_space(4.0);
        }
        // names typed on the picture, without a box
        let typed = self.meta(id).map(|m| m.people.clone()).unwrap_or_default();
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(6.0, 6.0);
            for name in &typed {
                let chip = egui::Button::new(RichText::new(format!("{} {name}  {}", icons::PERSON, icons::CLOSE)).size(12.5)).corner_radius(12).fill(t().select_bg);
                if ui.add(chip).on_hover_text(tr!("edit-person-remove")).clicked() {
                    self.meta_mut(id).people.retain(|x| x != name);
                }
            }
        });
        let r = ui.add(egui::TextEdit::singleline(&mut self.edit.person_buf).hint_text(tr!("edit-person-add")).desired_width(ui.available_width()));
        if r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) && !self.edit.person_buf.trim().is_empty() {
            let name = self.edit.person_buf.trim().to_string();
            self.edit.person_buf.clear();
            let m = self.meta_mut(id);
            if !m.people.contains(&name) {
                m.people.push(name);
            }
            r.request_focus();
        }
    }
}
