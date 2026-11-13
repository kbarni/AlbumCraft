//! The library's search box: one field for everything. Typed text matches file and folder names,
//! captions, tags, people and places anywhere; the dropdown under it suggests tags, places
//! (names set by the user, cities, regions, countries), people, albums and folders that match,
//! with their picture counts. A chosen tag, place or person becomes a chip in the box and matches
//! exactly; several chips combine (all must match). A chosen album or folder opens it.
//! Keys: ↑ / ↓ pick a suggestion, Enter takes it, Esc closes the list, Backspace in an empty
//! field removes the last chip.
#![allow(deprecated)] // egui::Id::new, as used elsewhere in this crate

use crate::i18n::tr;
use crate::app::{App, SearchTerm, Source, TraySet};
use crate::geo::fold;
use crate::icons;
use crate::theme::t;
use eframe::egui::{self, RichText, Sense, Stroke};
use std::collections::HashMap;
use std::sync::Arc;

/// Most suggestions per kind.
const PER_KIND: usize = 4;

/// Tags, places and people of the whole library with their picture counts, most used first.
#[derive(Default)]
pub struct SearchIndex {
    tags: Vec<(String, usize)>,
    places: Vec<(String, usize)>,
    people: Vec<(String, usize)>,
}

#[derive(Clone, Debug, PartialEq)]
enum Suggestion {
    /// Keep the typed text as it is (matches anywhere).
    Text,
    Term(SearchTerm, usize),
    Album(usize, usize),
    Folder(usize, usize),
}

/// Words counted once per picture, case-insensitively (the first spelling seen is kept).
fn counted<'a>(words: impl Iterator<Item = &'a [String]>) -> Vec<(String, usize)> {
    let mut by_fold: HashMap<String, (String, usize)> = HashMap::new();
    for list in words {
        let mut seen: Vec<String> = vec![];
        for w in list {
            let k = w.to_lowercase();
            if seen.contains(&k) {
                continue;
            }
            by_fold.entry(k.clone()).or_insert_with(|| (w.clone(), 0)).1 += 1;
            seen.push(k);
        }
    }
    let mut out: Vec<(String, usize)> = by_fold.into_values().collect();
    out.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.to_lowercase().cmp(&b.0.to_lowercase())));
    out
}

/// Positions of the (name, count) entries that contain `q` (folded: no case, no accents), those
/// with a word starting with it first, then the most used; at most PER_KIND.
fn best<'a>(list: impl Iterator<Item = (&'a str, usize)>, q: &str) -> Vec<usize> {
    let mut hits: Vec<(bool, usize, usize)> = list
        .enumerate()
        .filter_map(|(i, (name, n))| {
            let f = fold(name);
            f.contains(q).then(|| (!(f.starts_with(q) || f.split_whitespace().any(|w| w.starts_with(q))), n, i))
        })
        .collect();
    hits.sort_by(|a, b| a.0.cmp(&b.0).then(b.1.cmp(&a.1)));
    hits.into_iter().take(PER_KIND).map(|(_, _, i)| i).collect()
}

impl App {
    fn search_index(&mut self) -> Arc<SearchIndex> {
        let key = (self.list_rev, self.geo.place_rev, self.photos.len());
        if let Some((k, idx)) = &self.search_index
            && *k == key
        {
            return idx.clone();
        }
        let metas: Vec<&crate::library::Meta> = self.photos.iter().filter_map(|p| self.store.meta.get(&p.key)).collect();
        let idx = Arc::new(SearchIndex {
            tags: counted(metas.iter().map(|m| m.tags.as_slice())),
            people: counted(metas.iter().map(|m| m.people.as_slice())),
            places: counted(self.photos.iter().filter_map(|p| self.geo.place_terms.get(&p.key)).map(|(_, _, w)| w.as_slice())),
        });
        self.search_index = Some((key, idx.clone()));
        idx
    }

    fn suggestions(&mut self) -> Vec<Suggestion> {
        let q = fold(self.search.trim());
        if q.is_empty() {
            return vec![];
        }
        let idx = self.search_index();
        let chosen = |t: &SearchTerm| self.search_terms.contains(t);
        let mut out = vec![Suggestion::Text];
        let add = |list: &[(String, usize)], make: fn(String) -> SearchTerm, out: &mut Vec<Suggestion>| {
            for i in best(list.iter().map(|(s, n)| (s.as_str(), *n)), &q) {
                let Some((name, n)) = list.get(i) else { continue };
                let term = make(name.clone());
                if !chosen(&term) {
                    out.push(Suggestion::Term(term, *n));
                }
            }
        };
        add(&idx.tags, SearchTerm::Tag, &mut out);
        add(&idx.places, SearchTerm::Place, &mut out);
        add(&idx.people, SearchTerm::Person, &mut out);
        for i in best(self.store.albums.iter().map(|a| (a.name.as_str(), a.keys.len())), &q) {
            out.push(Suggestion::Album(i, self.store.albums.get(i).map(|a| a.keys.len()).unwrap_or(0)));
        }
        for i in best(self.folders.iter().map(|f| (f.name.as_str(), f.photos.len())), &q) {
            out.push(Suggestion::Folder(i, self.folders.get(i).map(|f| f.photos.len()).unwrap_or(0)));
        }
        out
    }

    fn take_suggestion(&mut self, s: Suggestion) {
        self.search_popup = false;
        self.search_hl = 0;
        self.selection.clear();
        match s {
            Suggestion::Text => {}
            Suggestion::Term(term, _) => {
                self.search.clear();
                if !self.search_terms.contains(&term) {
                    self.search_terms.push(term);
                }
            }
            Suggestion::Album(i, _) => {
                self.search.clear();
                self.source = Source::Album(i);
                self.list_pick = Some(TraySet::Album(i));
            }
            Suggestion::Folder(fi, _) => {
                self.search.clear();
                self.source = Source::Library;
                self.scroll_to_folder = Some(fi);
                self.pick_set(TraySet::Folder(fi));
            }
        }
    }

    /// The search pill of the top bar, `width` wide for the text.
    pub fn library_search(&mut self, ui: &mut egui::Ui, width: f32) {
        let field_id = egui::Id::new("library_search");
        let focused = ui.ctx().memory(|m| m.has_focus(field_id));
        let mut remove: Option<usize> = None;
        let mut clear = false;
        let pill = egui::Frame::new()
            .fill(t().surface)
            .stroke(Stroke::new(1.0, if focused { t().accent } else { t().border }))
            .corner_radius(18)
            .inner_margin(egui::Margin::symmetric(12, 3))
            .show(ui, |ui| {
                ui.allocate_ui_with_layout(egui::vec2(width + 52.0, 26.0), egui::Layout::left_to_right(egui::Align::Center), |ui| {
                    ui.spacing_mut().item_spacing.x = 4.0;
                    ui.label(RichText::new(icons::SEARCH).size(13.0).color(t().text_weak));
                    for (i, term) in self.search_terms.iter().enumerate() {
                        let (icon, name) = match term {
                            SearchTerm::Tag(n) => (icons::TAG, n),
                            SearchTerm::Place(n) => (icons::LOCATION, n),
                            SearchTerm::Person(n) => (icons::PERSON, n),
                            SearchTerm::Like(key) => {
                                let name = self.key_index.get(key).and_then(|&id| self.photos.get(id)).map(|p| p.name.clone()).unwrap_or_else(|| std::path::Path::new(key).file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default());
                                (icons::SEARCH, &tr!("search-like-chip", name = name))
                            }
                        };
                        let chip = egui::Button::new(RichText::new(format!("{icon} {name} {}", icons::CLOSE)).size(12.0)).fill(t().select_bg).corner_radius(10);
                        if ui.add(chip).on_hover_text(tr!("search-remove-hint")).clicked() {
                            remove = Some(i);
                        }
                    }
                    let hint = if self.search_terms.is_empty() { tr!("search-hint") } else { tr!("search-and") };
                    let r = ui.add(
                        egui::TextEdit::singleline(&mut self.search)
                            .id(field_id)
                            .hint_text(hint)
                            .frame(egui::Frame::NONE)
                            .desired_width((ui.available_width() - 22.0).max(60.0)),
                    );
                    if r.changed() {
                        self.selection.clear();
                        self.search_hl = 0;
                        self.search_popup = !self.search.trim().is_empty();
                    }
                    if r.gained_focus() {
                        self.search_popup = !self.search.trim().is_empty();
                    }
                    if self.searching()
                        && ui.add(egui::Label::new(RichText::new(icons::CLOSE).size(14.0).color(t().text_weak)).sense(Sense::click())).on_hover_text(tr!("search-clear-hint")).clicked()
                    {
                        clear = true;
                    }
                    r
                })
                .inner
            });
        let r = pill.inner;
        if let Some(i) = remove {
            self.search_terms.remove(i);
            self.selection.clear();
        }
        if clear {
            self.search.clear();
            self.search_terms.clear();
            self.search_popup = false;
            self.selection.clear();
        }

        let suggestions = if self.search_popup { self.suggestions() } else { vec![] };
        let n = suggestions.len();
        if r.has_focus() {
            let (down, up, esc, back) = ui.input(|i| {
                (i.key_pressed(egui::Key::ArrowDown), i.key_pressed(egui::Key::ArrowUp), i.key_pressed(egui::Key::Escape), i.key_pressed(egui::Key::Backspace))
            });
            if down && n > 0 {
                self.search_hl = (self.search_hl + 1).min(n - 1);
            }
            if up {
                self.search_hl = self.search_hl.saturating_sub(1);
            }
            if esc {
                self.search_popup = false;
            }
            if back && self.search.is_empty() && self.search_terms.pop().is_some() {
                self.selection.clear();
            }
        }
        // Enter takes the highlighted suggestion (the field loses focus on Enter: give it back)
        if r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
            if let Some(s) = suggestions.get(self.search_hl).cloned() {
                let keep_typing = matches!(s, Suggestion::Term(..));
                self.take_suggestion(s);
                if keep_typing {
                    r.request_focus();
                }
            } else {
                self.search_popup = false;
            }
            return;
        }
        if suggestions.len() <= 1 {
            return;
        }
        let pos = pill.response.rect.left_bottom() + egui::vec2(0.0, 4.0);
        let popup_w = pill.response.rect.width().max(320.0);
        let mut take = None;
        let area = egui::Area::new(egui::Id::new("search_suggestions")).order(egui::Order::Foreground).fixed_pos(pos).show(ui.ctx(), |ui| {
            egui::Frame::popup(ui.style()).show(ui, |ui| {
                ui.set_width(popup_w);
                let mut last_kind = String::new();
                for (i, s) in suggestions.iter().enumerate() {
                    let (kind, icon, label, count) = match s {
                        Suggestion::Text => (String::new(), icons::SEARCH, tr!("search-for", text = self.search.trim()), None),
                        Suggestion::Term(SearchTerm::Tag(n), c) => (tr!("search-kind-tags"), icons::TAG, n.clone(), Some(*c)),
                        Suggestion::Term(SearchTerm::Place(n), c) => (tr!("nav-places"), icons::LOCATION, n.clone(), Some(*c)),
                        Suggestion::Term(SearchTerm::Person(n), c) => (tr!("nav-people"), icons::PERSON, n.clone(), Some(*c)),
                        Suggestion::Term(SearchTerm::Like(n), c) => (tr!("search-similar-title"), icons::SEARCH, n.clone(), Some(*c)),
                        Suggestion::Album(a, c) => (tr!("nav-albums"), icons::ALBUM, self.store.albums.get(*a).map(|a| a.name.clone()).unwrap_or_default(), Some(*c)),
                        Suggestion::Folder(f, c) => (tr!("nav-folders"), icons::FOLDER, self.folders.get(*f).map(|f| f.name.clone()).unwrap_or_default(), Some(*c)),
                    };
                    if kind != last_kind {
                        ui.add_space(4.0);
                        ui.label(RichText::new(kind.as_str()).size(11.5).color(t().text_weak));
                        last_kind = kind;
                    }
                    let (rect, resp) = ui.allocate_exact_size(egui::vec2(popup_w, 26.0), Sense::click());
                    let p = ui.painter();
                    if i == self.search_hl || resp.hovered() {
                        p.rect_filled(rect, 5, if i == self.search_hl { t().select_bg } else { t().surface_hover });
                    }
                    p.text(rect.left_center() + egui::vec2(12.0, 0.0), egui::Align2::CENTER_CENTER, icon, egui::FontId::proportional(14.0), t().text_weak);
                    let text = p.layout_no_wrap(label, egui::FontId::proportional(13.5), t().text);
                    p.galley(rect.left_center() + egui::vec2(28.0, -text.size().y / 2.0), text, t().text);
                    if let Some(c) = count {
                        p.text(rect.right_center() - egui::vec2(8.0, 0.0), egui::Align2::RIGHT_CENTER, c.to_string(), egui::FontId::proportional(12.0), t().text_weak);
                    }
                    let resp = match s {
                        Suggestion::Folder(f, _) => resp.on_hover_text(self.folders.get(*f).map(|f| f.path.clone()).unwrap_or_default()),
                        _ => resp,
                    };
                    if resp.clicked() {
                        take = Some(s.clone());
                    }
                }
            });
        });
        if let Some(s) = take {
            let keep_typing = matches!(s, Suggestion::Term(..));
            self.take_suggestion(s);
            if keep_typing {
                r.request_focus();
            }
        } else if !r.has_focus() && !area.response.hovered() && ui.input(|i| i.pointer.any_pressed()) {
            // a click somewhere else
            self.search_popup = false;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_once_per_picture_any_case() {
        let a = vec!["Beach".to_string(), "beach".to_string(), "Sea".to_string()];
        let b = vec!["BEACH".to_string()];
        let c = counted([a.as_slice(), b.as_slice()].into_iter());
        assert_eq!(c, vec![("Beach".to_string(), 2), ("Sea".to_string(), 1)]);
    }

    #[test]
    fn best_matches_starts_first_then_most_used() {
        let list = [("Old town", 3usize), ("Bordeaux", 9), ("Oban", 1), ("Scotland", 40), ("Île-de-France", 2)];
        let names: Vec<&str> = best(list.iter().copied(), "o").iter().map(|&i| list[i].0).collect();
        // words starting with "o" first (most used first), then the others containing it
        assert_eq!(&names[..2], &["Old town", "Oban"]);
        assert_eq!(names.len(), PER_KIND);
        // accents and case don't matter
        let names: Vec<&str> = best(list.iter().copied(), "ile").iter().map(|&i| list[i].0).collect();
        assert_eq!(names, vec!["Île-de-France"]);
        assert!(best(list.iter().copied(), "zzz").is_empty());
    }
}
