//! The interface's languages: Fluent files in `i18n/<language>/albumcraft.ftl`, built into the
//! program. English (`i18n/en`) is the reference and the fallback for any missing message.
//!
//! Text in the interface is written `tr!("message-id")`, or `tr!("message-id", n = count)` with
//! arguments. The ids are checked against the English file at compile time.
//!
//! The language is chosen at start from the saved setting (`Store::language`, empty = the
//! system's), the `ALBUMCRAFT_LANG` variable overriding both, and can be changed while running
//! (`set_language`): egui draws every frame again, so the new text appears at once.

use i18n_embed::fluent::{FluentLanguageLoader, fluent_language_loader};
use i18n_embed::unic_langid::LanguageIdentifier;
use i18n_embed::{DesktopLanguageRequester, LanguageLoader};
use rust_embed::RustEmbed;
use std::sync::LazyLock;

#[derive(RustEmbed)]
#[folder = "i18n/"]
struct Localizations;

pub static LOADER: LazyLock<FluentLanguageLoader> = LazyLock::new(|| {
    let loader: FluentLanguageLoader = fluent_language_loader!();
    // tests read English text whatever language the machine is set to
    #[cfg(test)]
    let requested = vec!["en".parse().expect("language code")];
    #[cfg(not(test))]
    let requested = DesktopLanguageRequester::requested_languages();
    load(&loader, &requested);
    loader
});

/// The text of a message in the current language: `tr!("id")` or `tr!("id", name = value, …)`.
macro_rules! tr {
    ($($args:tt)*) => {
        i18n_embed_fl::fl!($crate::i18n::LOADER, $($args)*)
    };
}
pub(crate) use tr;

fn load(loader: &FluentLanguageLoader, requested: &[LanguageIdentifier]) {
    if let Err(e) = i18n_embed::select(loader, &Localizations, requested) {
        eprintln!("Could not load the interface's translations: {e}");
    }
    // Fluent puts invisible direction marks around arguments, which egui's fonts would draw as
    // boxes; loading a language starts over with them on, so this comes after it
    loader.set_use_isolating(false);
}

/// Switch the interface to `language` (a code such as "fr", as saved in `Store::language`), or
/// to the system's language when it is empty or unknown.
pub fn set_language(language: &str) {
    let chosen = std::env::var("ALBUMCRAFT_LANG").ok().filter(|l| !l.is_empty()).unwrap_or_else(|| language.to_owned());
    let mut requested: Vec<LanguageIdentifier> = chosen.parse().into_iter().collect();
    requested.extend(DesktopLanguageRequester::requested_languages());
    load(&LOADER, &requested);
}

/// The languages there are translations for, as (code, the language's own name), English first
/// and the others in the order of their names.
pub fn available() -> Vec<(String, String)> {
    let mut list: Vec<(String, String)> = LOADER
        .available_languages(&Localizations)
        .unwrap_or_default()
        .iter()
        .map(|id| {
            let code = id.to_string();
            let name = own_name(&code).unwrap_or_else(|| code.clone());
            (code, name)
        })
        .collect();
    list.sort_by_key(|(code, name)| (code != "en", name.to_lowercase()));
    list
}

/// The system's language among those there are translations for (its code), if any.
pub fn system_language() -> Option<String> {
    let available = LOADER.available_languages(&Localizations).unwrap_or_default();
    DesktopLanguageRequester::requested_languages()
        .iter()
        .find_map(|want| available.iter().find(|have| have.language == want.language))
        .map(|l| l.to_string())
}

/// The own name of a language (its `language-name` message), read from its file.
pub fn own_name(code: &str) -> Option<String> {
    let file = Localizations::get(&format!("{code}/{}", LOADER.language_file_name()))?;
    let text = std::str::from_utf8(&file.data).ok()?;
    text.lines().find_map(|l| l.strip_prefix("language-name").map(|r| r.trim_start().trim_start_matches('=').trim().to_owned()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    /// The message ids (and `.attribute`s) defined in a language's file (loading it fails the
    /// test when it has a syntax error).
    fn ids(code: &str) -> BTreeSet<String> {
        let id: LanguageIdentifier = code.parse().expect("language code");
        let loader: FluentLanguageLoader = fluent_language_loader!();
        loader.load_languages(&Localizations, std::slice::from_ref(&id)).unwrap_or_else(|e| panic!("{code}: {e}"));
        loader.with_message_iter(&id, |messages| {
            let mut out = BTreeSet::new();
            for m in messages {
                out.insert(m.id.name.to_owned());
                for a in &m.attributes {
                    out.insert(format!("{}.{}", m.id.name, a.id.name));
                }
            }
            out
        })
    }

    /// Every language has exactly the English messages: none missing (it would show English),
    /// none left over (a renamed or removed message).
    #[test]
    fn languages_have_the_english_messages() {
        let en = ids("en");
        for (code, _) in available() {
            let other = ids(&code);
            let missing: Vec<_> = en.difference(&other).collect();
            let extra: Vec<_> = other.difference(&en).collect();
            assert!(missing.is_empty() && extra.is_empty(), "{code}: missing {missing:?}, not in English {extra:?}");
        }
    }

    /// Text with arguments has no invisible direction marks (egui would draw them as boxes).
    #[test]
    fn arguments_are_not_isolated() {
        let text = tr!("op-look", look = "Vivid", amount = "50");
        assert_eq!(text, "Vivid 50 %");
        // a loader of its own: the global one's language is not changed by tests (they run together)
        let fr: FluentLanguageLoader = fluent_language_loader!();
        load(&fr, &["fr".parse().expect("language code")]);
        assert_eq!(i18n_embed_fl::fl!(fr, "op-look", look = "Éclatant", amount = "50"), "Éclatant 50 %");
    }

    #[test]
    fn languages_are_listed_by_their_own_names() {
        let list = available();
        assert_eq!(list.first().map(|l| l.0.as_str()), Some("en"));
        assert!(list.iter().any(|(code, name)| code == "fr" && name == "Français"), "{list:?}");
    }
}
