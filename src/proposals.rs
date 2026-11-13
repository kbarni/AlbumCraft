//! Tag proposals in the window: the dimmed chips under the tags of the info panel and of the
//! Tags dialog. A click adds the tag; "Not this tag" (right-click) writes it in the picture's
//! `Meta.not_tags`. The proposals are made off the window's thread (`Library::propose_tags` →
//! `Done::TagProposals`), asked again when the pictures, their notes or the search pack
//! change, and every few seconds while the built-in vocabulary is being prepared.

use crate::app::App;
use crate::i18n::tr;
use crate::icons;
use crate::tag_proposals::{self, Examples, Proposal, Proposals};
use crate::theme::t;
use eframe::egui::{self, RichText};
use std::sync::Arc;

/// Seconds before asking again while the vocabulary is being prepared.
const AGAIN: f64 = 3.0;
/// Pictures of a selection sent with the question (the library looks at fewer still).
const MAX_TARGETS: usize = 200;

/// What the proposals were made for: the pictures, the notes' revision, the pack.
type Asked = (Vec<String>, u64, String);

#[derive(Default)]
pub struct TagProposals {
    next: u64,
    /// The question waiting for its answer.
    asked: Option<(u64, Asked)>,
    /// The last answer and what it was for.
    result: Option<(Asked, Proposals)>,
    /// When to ask again for a partial answer.
    again_at: f64,
    examples: Option<(u64, Arc<Examples>)>,
    /// For the screenshots: always these.
    demo: Option<Vec<Proposal>>,
}

impl TagProposals {
    /// For the screenshots: these proposals for any picture, nothing asked of the library.
    pub fn demo(&mut self, tags: Vec<Proposal>) {
        self.demo = Some(tags);
    }
}

impl App {
    /// The tags proposed for the pictures `ids` (none until the library answers; the last
    /// answer for the same pictures meanwhile).
    pub fn tag_proposals(&mut self, ids: &[usize]) -> Vec<Proposal> {
        if let Some(tags) = &self.props.demo {
            return tags.clone();
        }
        let Some(pack) = self.smart.pack.clone() else { return vec![] };
        let keys: Vec<String> = ids.iter().filter_map(|&id| self.photos.get(id)).filter(|p| !p.is_sample()).take(MAX_TARGETS).map(|p| p.key.clone()).collect();
        if keys.is_empty() {
            return vec![];
        }
        let wanted: Asked = (keys, self.list_rev, pack);
        let now = self.ctx.input(|i| i.time);
        let answered = self.props.result.as_ref().filter(|(a, _)| *a == wanted).map(|(_, p)| p.clone());
        let waiting = self.props.asked.as_ref().is_some_and(|(_, a)| *a == wanted);
        let ask = match &answered {
            Some(p) => p.partial && !waiting && now >= self.props.again_at,
            None => !waiting,
        };
        if ask {
            self.ask_proposals(wanted.clone());
        }
        if answered.as_ref().is_some_and(|p| p.partial) {
            self.ctx.request_repaint_after(std::time::Duration::from_secs_f64(AGAIN));
        }
        // while waiting, what was shown for the same pictures stays
        answered
            .or_else(|| self.props.result.as_ref().filter(|(a, _)| a.0 == wanted.0).map(|(_, p)| p.clone()))
            .map(|p| p.tags)
            .unwrap_or_default()
    }

    fn ask_proposals(&mut self, wanted: Asked) {
        let rev = self.list_rev;
        let examples = match &self.props.examples {
            Some((r, e)) if *r == rev => e.clone(),
            _ => {
                let gps = &self.geo.file_gps;
                let e = Arc::new(tag_proposals::examples_of(&self.store, rev, |k, m| crate::geo::effective(m.place.as_ref(), gps.get(k).copied().flatten())));
                self.props.examples = Some((rev, e.clone()));
                e
            }
        };
        let gps = &self.geo.file_gps;
        let targets = wanted.0.iter().map(|k| tag_proposals::picture_of(&self.store, k, |k, m| crate::geo::effective(m.place.as_ref(), gps.get(k).copied().flatten()))).collect();
        self.props.next += 1;
        let n = self.props.next;
        self.lib.propose_tags(wanted.2.clone(), n, examples, targets);
        self.props.asked = Some((n, wanted));
    }

    /// The library's answer to question `n`.
    pub fn on_tag_proposals(&mut self, n: u64, found: Result<Proposals, String>) {
        let Some((asked_n, asked)) = self.props.asked.take() else { return };
        if asked_n != n {
            self.props.asked = Some((asked_n, asked));
            return;
        }
        // a failure shows nothing: proposals are only a help
        let p = found.unwrap_or_default();
        if p.partial {
            self.props.again_at = self.ctx.input(|i| i.time) + AGAIN;
        }
        self.props.result = Some((asked, p));
    }

    /// The pictures `ids` do not show `tag`: it is not proposed for them again, and less for
    /// pictures like them.
    pub fn not_this_tag(&mut self, ids: &[usize], tag: &str) {
        for &id in ids {
            if self.photos.get(id).is_none_or(|p| p.is_sample()) {
                continue;
            }
            let m = self.meta_mut(id);
            if !m.not_tags.iter().any(|t| t.eq_ignore_ascii_case(tag)) {
                m.not_tags.push(tag.to_string());
            }
        }
    }

    /// The proposal chips: returns the tag clicked (to add) and the tag said not to fit.
    pub fn proposal_chips(&mut self, ui: &mut egui::Ui, ids: &[usize]) -> (Option<String>, Option<String>) {
        let proposals = self.tag_proposals(ids);
        let (mut add, mut not) = (None, None);
        if proposals.is_empty() {
            return (add, not);
        }
        ui.add_space(4.0);
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(6.0, 6.0);
            ui.label(RichText::new(tr!("tags-proposed")).size(12.0).color(t().text_weak));
            for p in &proposals {
                let chip = egui::Button::new(RichText::new(format!("{}  {}", icons::ADD, p.tag)).size(12.0).color(t().text_weak))
                    .corner_radius(12)
                    .fill(egui::Color32::TRANSPARENT)
                    .stroke(egui::Stroke::new(1.0, t().border));
                let r = ui.add(chip).on_hover_text(tr!("tags-proposed-hint"));
                if r.clicked() {
                    add = Some(p.tag.clone());
                }
                r.context_menu(|ui| {
                    if ui.button(tr!("tags-not-this", tag = p.tag.clone())).clicked() {
                        not = Some(p.tag.clone());
                        ui.close();
                    }
                });
            }
        });
        (add, not)
    }
}
