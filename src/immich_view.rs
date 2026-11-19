//! An Immich server in the window: the sign-in dialog, the switch between this computer's
//! folders and the server in the top bar, the connection's status line, and the server's
//! notes arriving (`Done::ServerNotes`). The library side is `immich_library.rs`.

use crate::app::*;
use crate::i18n::tr;
use crate::icons;
use crate::immich::{self, api, Account, Changes};
use crate::library::Album;
use crate::theme::t;
use eframe::egui::{self, RichText};
use std::sync::mpsc::Receiver;

/// The sign-in dialog's fields, and the sign-in running in the background.
pub struct ImmichForm {
    pub address: String,
    pub email: String,
    pub password: String,
    pub key: String,
    /// Sign in with an API key rather than email and password.
    pub with_key: bool,
    pub busy: Option<Receiver<Result<(Account, api::Auth), String>>>,
    pub error: String,
}

/// What to tell the user about a failed sign-in.
fn explain(e: &api::Error) -> String {
    match e {
        api::Error::Address(why) => tr!("immich-bad-address", why = why.as_str()),
        api::Error::Network(why) => tr!("immich-unreachable", error = why.as_str()),
        api::Error::Unauthorized => tr!("immich-wrong-login"),
        api::Error::NoPasswordLogin => tr!("immich-no-password"),
        api::Error::Unsupported(v) => tr!("immich-too-old", version = v.to_string()),
        other => tr!("immich-failed", error = other.to_string()),
    }
}

impl App {
    /// The window shows an Immich server's pictures.
    pub fn showing_server(&self) -> bool {
        self.lib.remote_status().is_some()
    }

    pub fn open_immich_dialog(&mut self) {
        let a = self.store.immich.clone().unwrap_or_default();
        self.dialog = Some(Dialog::Immich(Box::new(ImmichForm {
            address: a.address,
            email: a.user_email,
            password: String::new(),
            key: String::new(),
            with_key: false,
            busy: None,
            error: String::new(),
        })));
    }

    pub(crate) fn immich_dialog(&mut self, ctx: &egui::Context, mut f: Box<ImmichForm>) {
        let mut close = false;
        // the sign-in's answer
        if let Some(rx) = &f.busy {
            match rx.try_recv() {
                Ok(Ok((account, auth))) => {
                    f.busy = None;
                    match immich::save_secret(&account, &auth) {
                        Ok(()) => {
                            self.store.immich = Some(account);
                            self.dirty = true;
                            close = true;
                            self.show_immich(true);
                        }
                        Err(e) => f.error = tr!("immich-failed", error = e),
                    }
                }
                Ok(Err(e)) => {
                    f.busy = None;
                    f.error = e;
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => ctx.request_repaint_after(std::time::Duration::from_millis(100)),
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    f.busy = None;
                    f.error = tr!("immich-failed", error = "stopped");
                }
            }
        }
        let enter = ctx.input(|i| i.key_pressed(egui::Key::Enter));
        let busy = f.busy.is_some();
        #[allow(deprecated)]
        let modal = egui::Modal::new(egui::Id::new("immich-sign-in")).show(ctx, |ui| {
            ui.set_width(420.0);
            ui.heading(tr!("immich-title"));
            ui.add_space(4.0);
            ui.label(RichText::new(tr!("immich-intro")).size(12.5).color(t().text_weak));
            ui.add_space(8.0);
            ui.add_enabled_ui(!busy, |ui| {
                ui.label(tr!("immich-address"));
                ui.add(egui::TextEdit::singleline(&mut f.address).hint_text("https://photos.example.com").desired_width(f32::INFINITY));
                ui.add_space(4.0);
                if f.with_key {
                    ui.label(tr!("immich-key"));
                    ui.add(egui::TextEdit::singleline(&mut f.key).password(true).desired_width(f32::INFINITY));
                } else {
                    ui.label(tr!("immich-email"));
                    ui.add(egui::TextEdit::singleline(&mut f.email).desired_width(f32::INFINITY));
                    ui.label(tr!("immich-password"));
                    ui.add(egui::TextEdit::singleline(&mut f.password).password(true).desired_width(f32::INFINITY));
                }
                let other = if f.with_key { tr!("immich-use-password") } else { tr!("immich-use-key") };
                if ui.link(other).clicked() {
                    f.with_key = !f.with_key;
                    f.error.clear();
                }
            });
            if !f.error.is_empty() {
                ui.add_space(4.0);
                ui.label(RichText::new(f.error.as_str()).color(t().bad));
            }
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                let ready = !f.address.trim().is_empty() && if f.with_key { !f.key.trim().is_empty() } else { !f.email.trim().is_empty() && !f.password.is_empty() };
                if ui.add_enabled(ready && !busy, egui::Button::new(tr!("immich-sign-in"))).clicked() || (enter && ready && !busy) {
                    f.error.clear();
                    let (tx, rx) = std::sync::mpsc::channel();
                    let (address, email, key, with_key) = (f.address.clone(), f.email.clone(), f.key.clone(), f.with_key);
                    // the password leaves the form here and is dropped after the one call
                    let password = std::mem::take(&mut f.password);
                    let wake = ui.ctx().clone();
                    std::thread::spawn(move || {
                        let r = std::panic::catch_unwind(|| if with_key { immich::sign_in_with_key(&address, &key) } else { immich::sign_in(&address, &email, &password) });
                        let r = match r {
                            Ok(r) => r.map_err(|e| explain(&e)),
                            Err(_) => Err(tr!("immich-failed", error = "unexpected")),
                        };
                        let _ = tx.send(r);
                        wake.request_repaint();
                    });
                    f.busy = Some(rx);
                }
                if busy {
                    ui.spinner();
                    ui.label(tr!("immich-signing-in"));
                }
                if ui.button(tr!("button-cancel")).clicked() {
                    close = true;
                }
            });
        });
        if self.dialog.is_none() && !close && !modal.should_close() {
            self.dialog = Some(Dialog::Immich(f));
        }
    }

    /// Show the server's pictures (true) or this computer's folders (false).
    pub fn show_immich(&mut self, on: bool) {
        if on == self.showing_server() {
            return;
        }
        let account = if on { self.store.immich.clone() } else { None };
        if on && account.is_none() {
            self.open_immich_dialog();
            return;
        }
        // nothing of the other library stays open
        self.view = View::Library;
        self.source = Source::Library;
        // the notes as they are now go with the library they belong to
        if self.persist {
            self.lib.save(&mut self.store, true);
            if let Some(r) = self.lib.take_saved(true) {
                self.save_finished(r);
            }
            self.dirty = false;
            self.unsaved = false;
        }
        match self.lib.show_server(&mut self.store, account) {
            Ok(()) => {
                self.store.use_immich = on;
                self.dirty = true;
                self.scanned = false;
                self.set_library(vec![], false);
                if !self.store.roots.is_empty() {
                    self.lib.cached(self.store.roots.clone(), self.store.excluded.clone());
                }
                self.rescan();
            }
            Err(e) => self.error(tr!("immich-failed", error = e)),
        }
    }

    /// Forget the server's sign-in (the pictures' notes kept on this computer stay).
    pub fn sign_out_immich(&mut self) {
        self.show_immich(false);
        if let Some(a) = self.store.immich.take() {
            std::thread::spawn(move || immich::sign_out(&a));
        }
        self.store.use_immich = false;
        self.dirty = true;
    }

    /// The server's notes changed: put the changed fields into the window's notes.
    pub fn on_server_notes(&mut self, ch: Changes) {
        if !self.showing_server() {
            return;
        }
        for (key, c) in &ch.notes {
            c.apply(self.store.meta.entry(key.clone()).or_default());
        }
        for name in &ch.removed_albums {
            if let Some(i) = self.store.albums.iter().position(|a| a.name == *name) {
                self.store.albums.remove(i);
            }
        }
        for c in ch.albums {
            let at = c.was.as_ref().and_then(|w| self.store.albums.iter().position(|a| a.name == *w)).or_else(|| self.store.albums.iter().position(|a| a.name == c.album.name));
            match at {
                Some(i) => self.store.albums[i] = c.album,
                None => self.store.albums.push(Album { name: c.album.name, keys: c.album.keys }),
            }
        }
        if let Source::Album(i) = self.source
            && i >= self.store.albums.len()
        {
            self.source = Source::Library;
        }
        for (child, parent) in ch.tag_parents {
            self.store.tag_parents.entry(child).or_insert(parent);
        }
        self.dirty = true;
        self.list_rev += 1;
    }

    /// The top bar's switch between this computer's folders and the server, shown once a server
    /// was signed in to.
    pub fn library_switch(&mut self, ui: &mut egui::Ui) {
        let Some(a) = &self.store.immich else { return };
        let server = self.showing_server();
        let name = if a.user_name.is_empty() { a.connection() } else { a.user_name.clone() };
        let hint = tr!("immich-switch-hint", server = a.address.as_str());
        // right to left: the server's side first
        if ui.selectable_label(server, format!("{}  {}", icons::WEB, tr!("immich-switch-server"))).on_hover_text(format!("{name}\n{hint}")).clicked() {
            self.show_immich(true);
        }
        if ui.selectable_label(!server, format!("{}  {}", icons::DESKTOP, tr!("immich-switch-local"))).on_hover_text(tr!("immich-switch-local-hint")).clicked() {
            self.show_immich(false);
        }
    }

    /// The connection's state, in the status bar while the server is shown.
    pub fn immich_status(&mut self, ui: &mut egui::Ui) {
        let Some(s) = self.lib.remote_status() else { return };
        ui.separator();
        if s.syncing {
            ui.spinner();
            ui.label(RichText::new(tr!("immich-syncing")).size(12.0));
        } else if s.signed_out {
            ui.label(RichText::new(tr!("immich-signed-out")).size(12.0).color(t().bad));
            if ui.small_button(tr!("immich-sign-in")).clicked() {
                self.open_immich_dialog();
            }
        } else if let Some(e) = &s.error {
            ui.label(RichText::new(tr!("immich-offline")).size(12.0).color(t().bad)).on_hover_text(e.as_str());
        } else if s.online {
            ui.label(RichText::new(tr!("immich-up-to-date")).size(12.0).weak());
        }
        if let Some((done, total)) = s.tags {
            ui.label(RichText::new(tr!("immich-reading-tags", done = done, total = total)).size(12.0).weak());
        }
    }
}
