//! Smart search in the window: the "Looks like" section under the search results (pictures
//! ranked by how well they match the typed text) and "More like this" (a chip in the search
//! box that lists the pictures like one).
//!
//! The ranking runs off the window's thread (`Library::looks_like`), 250 ms after the last key;
//! answers come back as `Done::LooksLike` and are dropped when the query changed meanwhile.
//! The pack used is the user's choice or the one for the interface's language
//! (`smart_search::active`), looked at every few seconds since a download or a removal can
//! change it.

use crate::app::{App, Head, SearchTerm};
use crate::i18n::tr;
use crate::icons;
use crate::smart_search::{self, Hit, Query};
use crate::theme::t;
use eframe::egui::{self, RichText};
use std::cell::Cell;
use std::rc::Rc;

/// Seconds after the last key before the typed text is searched.
const TYPING_PAUSE: f64 = 0.25;
/// How often the active pack is looked for again (a few file sizes).
const PACK_CHECK: f64 = 2.0;
/// Pictures added by each "Show more".
const MORE: usize = 60;

#[derive(Default)]
pub struct SmartSearch {
    /// The search pack in use (None: none is downloaded, or this build can't run networks).
    pub pack: Option<String>,
    /// What the library was last told to use.
    told: Option<Option<String>>,
    checked_at: Option<f64>,
    /// The typed text as last seen, and when it changed.
    text: String,
    changed_at: f64,
    /// Asked of the library and not answered yet.
    asked: Option<Query>,
    /// The last answer.
    pub result: Option<(Query, Rc<Vec<Hit>>)>,
    /// How many of its pictures are shown (the good matches, then "Show more").
    pub shown: usize,
    /// Changes whenever what is shown changes (part of the list's cache key).
    pub rev: u64,
    /// Pictures the last list could have shown in all (set while the list is made).
    pub available: Cell<usize>,
    last_error: Option<String>,
}

impl SmartSearch {
    /// Look for the pack to use at the next frame (the user chose another one).
    pub fn recheck(&mut self) {
        self.checked_at = None;
    }

    /// For the screenshots (sample pictures are never analysed): `pack` is in use and `query`
    /// was answered with `hits`; nothing is asked of the library.
    pub fn demo(&mut self, pack: &str, query: Query, hits: Vec<Hit>, shown: usize) {
        self.pack = Some(pack.to_string());
        self.told = Some(self.pack.clone());
        self.checked_at = Some(f64::INFINITY);
        self.result = Some((query, Rc::new(hits)));
        self.shown = shown;
        self.rev += 1;
    }
}

impl App {
    /// What smart search looks for now: the pictures like a chosen one, or the typed text.
    pub fn smart_query(&self) -> Option<Query> {
        if let Some(key) = self.search_terms.iter().find_map(|t| if let SearchTerm::Like(k) = t { Some(k.clone()) } else { None }) {
            return Some(Query::Like(key));
        }
        let text = self.search.trim();
        (self.smart.pack.is_some() && !text.is_empty()).then(|| Query::Text(text.to_string()))
    }

    /// Once per frame: tell the library which pack to use, and ask for a ranking when the query
    /// changed (typed text after a short pause).
    pub fn smart_search_tick(&mut self, ctx: &egui::Context, now: f64) {
        if self.smart.checked_at.is_none_or(|at| now - at >= PACK_CHECK) {
            self.smart.checked_at = Some(now);
            let pack = if crate::ai::runtime::available() { smart_search::active(&self.store.search_pack, &crate::i18n::current_language()).map(|p| p.id.clone()) } else { None };
            if self.smart.told.as_ref() != Some(&pack) {
                self.lib.use_search_pack(pack.clone());
                self.smart.told = Some(pack.clone());
                if self.smart.pack != pack {
                    // vectors of another network: what was found does not count any more
                    self.smart.result = None;
                    self.smart.asked = None;
                    self.smart.rev += 1;
                }
            }
            self.smart.pack = pack;
        }
        if self.search != self.smart.text {
            self.smart.text = self.search.clone();
            self.smart.changed_at = now;
        }
        let wanted = self.smart_query();
        let Some(query) = wanted else {
            if self.smart.result.take().is_some() {
                self.smart.rev += 1;
            }
            self.smart.asked = None;
            return;
        };
        let answered = self.smart.result.as_ref().is_some_and(|(q, _)| *q == query);
        if answered || self.smart.asked.as_ref() == Some(&query) {
            return;
        }
        let Some(pack) = self.smart.pack.clone() else {
            if matches!(query, Query::Like(_)) && self.smart.last_error.is_none() {
                self.smart.last_error = Some(String::new());
                self.toast(tr!("search-like-no-pack"));
            }
            return;
        };
        let wait = TYPING_PAUSE - (now - self.smart.changed_at);
        if matches!(query, Query::Text(_)) && wait > 0.0 {
            ctx.request_repaint_after(std::time::Duration::from_secs_f64(wait));
            return;
        }
        self.smart.asked = Some(query.clone());
        self.lib.looks_like(pack, query);
    }

    /// The library ranked the pictures for `query`.
    pub fn on_looks_like(&mut self, query: Query, found: Result<Vec<Hit>, String>) {
        if self.smart.asked.as_ref() != Some(&query) {
            return;
        }
        self.smart.asked = None;
        match found {
            Ok(hits) => {
                let pack = self.smart.pack.as_deref().and_then(|id| crate::ai::models::manifest().pack(id));
                self.smart.shown = pack.map_or(0, |p| smart_search::cut(p, &hits, &query));
                self.smart.result = Some((query, Rc::new(hits)));
                self.smart.last_error = None;
            }
            Err(e) => {
                self.smart.result = Some((query, Rc::new(vec![])));
                // once, not at every key
                if self.smart.last_error.as_ref() != Some(&e) {
                    self.toast(tr!("search-failed-because", error = e.clone()));
                    self.smart.last_error = Some(e);
                }
            }
        }
        self.smart.rev += 1;
    }

    /// The ranked pictures of the current query that `keep` lets through, as ids, best first,
    /// at most as many as are shown. None when there is no answer for the current query.
    pub fn looks_like_ids(&self, keep: impl Fn(usize) -> bool) -> Option<Vec<usize>> {
        let query = self.smart_query()?;
        let (q, hits) = self.smart.result.as_ref()?;
        if *q != query {
            return None;
        }
        let all: Vec<usize> = hits.iter().filter_map(|h| self.key_index.get(&h.key).copied()).filter(|&id| keep(id)).collect();
        self.smart.available.set(all.len());
        Some(all.into_iter().take(self.smart.shown).collect())
    }

    /// The list shows the pictures like this one.
    pub fn more_like_this(&mut self, id: usize) {
        let Some(key) = self.photos.get(id).map(|p| p.key.clone()) else { return };
        self.search_terms.retain(|t| !matches!(t, SearchTerm::Like(_)));
        self.search_terms.push(SearchTerm::Like(key));
        self.search.clear();
        self.selection.clear();
        self.smart.last_error = None;
    }

    /// The header of the "Looks like" section.
    pub fn looks_like_header(&mut self, ui: &mut egui::Ui, ids: &[usize]) -> egui::Response {
        let like = matches!(self.smart_query(), Some(Query::Like(_)));
        let more = self.smart.available.get().saturating_sub(ids.len());
        let analysing = self.analysis.active();
        let inner = ui.vertical(|ui| {
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                let title = if like { tr!("search-similar-title") } else { tr!("search-looks-like-title") };
                ui.label(RichText::new(format!("{} {title}", icons::SEARCH)).size(19.0).strong().color(t().text_strong));
                ui.label(RichText::new(crate::app::fmt_count(ids.len())).size(12.5).color(t().text_weak));
                if analysing {
                    ui.label(RichText::new(tr!("search-still-analysing")).size(12.5).color(t().text_weak));
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if more > 0 && ui.button(tr!("search-show-more")).on_hover_text(tr!("search-show-more-hint", n = more, count = crate::app::fmt_count(more))).clicked() {
                        self.smart.shown = ids.len() + MORE;
                        self.smart.rev += 1;
                    }
                });
            });
            ui.add_space(4.0);
        });
        inner.response
    }
}

/// Is this a "Looks like" group (so its order is the ranking, not the chosen sort)?
pub fn ranked(head: Head) -> bool {
    head == Head::LooksLike
}
