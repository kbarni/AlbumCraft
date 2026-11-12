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
use fluent_bundle::FluentValue;
use std::collections::HashMap;
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
    apply_locale(&loader);
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

/// Make the core crate's messages (`msg!`) come out in the interface's language. Called once,
/// before the window starts; `--mcp` does not, and stays English.
pub fn init() {
    crate::msg::set_translator(translate);
}

/// The message `key` of the core crate, with its arguments; None when there is no such message.
/// Arguments that are whole numbers are numbers for Fluent (plurals).
fn translate(key: &str, args: &[(&'static str, String)]) -> Option<String> {
    if !LOADER.has(key) {
        return None;
    }
    let args: HashMap<&str, FluentValue> = args
        .iter()
        .map(|(name, value)| (*name, value.parse::<i64>().map(FluentValue::from).unwrap_or_else(|_| value.clone().into())))
        .collect();
    Some(LOADER.get_args(key, args))
}

/// Switch the interface to `language` (a code such as "fr", as saved in `Store::language`), or
/// to the system's language when it is empty or unknown.
pub fn set_language(language: &str) {
    let chosen = std::env::var("ALBUMCRAFT_LANG").ok().filter(|l| !l.is_empty()).unwrap_or_else(|| language.to_owned());
    let mut requested: Vec<LanguageIdentifier> = chosen.parse().into_iter().collect();
    requested.extend(DesktopLanguageRequester::requested_languages());
    load(&LOADER, &requested);
    apply_locale(&LOADER);
}

/// The language the interface is in now: its code without the region ("en", "fr", "ja").
pub fn current_language() -> String {
    LOADER.current_languages().first().map(|l| l.language.as_str().to_string()).unwrap_or_else(|| "en".into())
}

/// System fonts (path, index in a font collection) that have the characters of `language`'s
/// script, for those the fonts built into the program lack: Chinese, Japanese and Korean. They
/// are not bundled (each is several MB); the first one found on the machine is used. Empty for
/// the languages the built-in fonts cover. Right-to-left languages (Arabic, Hebrew) are not
/// supported at all: egui does not lay such text out.
pub fn script_font_candidates(language: &str) -> Vec<(&'static str, u32)> {
    // the Noto CJK collection holds Japanese, Korean, Simplified and Traditional Chinese in turn
    let noto = |index| {
        [
            "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
            "/usr/share/fonts/noto-cjk/NotoSansCJK-Regular.ttc",
            "/usr/share/fonts/truetype/noto/NotoSansCJK-Regular.ttc",
            "/usr/share/fonts/google-noto-cjk/NotoSansCJK-Regular.ttc",
            "/usr/share/fonts/OTF/NotoSansCJK-Regular.ttc",
        ]
        .map(|p| (p, index))
    };
    match language {
        "ja" => [&noto(0)[..], &[("C:\\Windows\\Fonts\\YuGothM.ttc", 0), ("C:\\Windows\\Fonts\\meiryo.ttc", 0), ("/System/Library/Fonts/Hiragino Sans GB.ttc", 0)]].concat(),
        "ko" => [&noto(1)[..], &[("C:\\Windows\\Fonts\\malgun.ttf", 0), ("/System/Library/Fonts/AppleSDGothicNeo.ttc", 0)]].concat(),
        "zh" => [&noto(2)[..], &[("C:\\Windows\\Fonts\\msyh.ttc", 0), ("/System/Library/Fonts/PingFang.ttc", 0), ("/usr/share/fonts/truetype/wqy/wqy-microhei.ttc", 0)]].concat(),
        _ => vec![],
    }
}

/// The first of these fonts that exists on this machine.
pub fn first_existing(candidates: &[(&'static str, u32)]) -> Option<(std::path::PathBuf, u32)> {
    candidates.iter().map(|(p, i)| (std::path::PathBuf::from(p), *i)).find(|(p, _)| p.is_file())
}

/// Give the core crate the way this language writes dates, numbers and sizes.
fn apply_locale(loader: &FluentLanguageLoader) {
    let list = |id: &str| loader.get(id).split(',').map(|w| w.trim().to_string()).collect::<Vec<_>>();
    crate::locale::set(crate::locale::Locale {
        months_short: list("locale-months-short"),
        months_long: list("locale-months-long"),
        date: i18n_embed_fl::fl!(loader, "locale-date", month = "{month}", day = "{day}", year = "{year}"),
        decimal: loader.get("locale-decimal"),
        group: loader.get("locale-group"),
        units: [loader.get("locale-unit-kb"), loader.get("locale-unit-mb"), loader.get("locale-unit-gb")],
    });
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

    /// The `msg!("key", "English", …)` calls of the core crate's sources: (key, English).
    fn core_messages() -> Vec<(String, String)> {
        // reads a string literal at the start of `s` (after spaces): its text and what follows it
        fn literal(s: &str) -> Option<(String, &str)> {
            let s = s.trim_start().strip_prefix('"')?;
            let (mut out, mut chars) = (String::new(), s.char_indices());
            while let Some((i, c)) = chars.next() {
                match c {
                    '"' => return Some((out, &s[i + 1..])),
                    '\\' => match chars.next()?.1 {
                        'n' => out.push('\n'),
                        other => out.push(other),
                    },
                    c => out.push(c),
                }
            }
            None
        }
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("crates/albumcraft-core/src");
        let mut found = vec![];
        for entry in std::fs::read_dir(dir).expect("core sources").flatten() {
            let path = entry.path();
            if path.extension().is_none_or(|e| e != "rs") || path.file_name().is_some_and(|n| n == "msg.rs") {
                continue;
            }
            let text = std::fs::read_to_string(&path).expect("utf-8");
            for part in text.split("msg!(").skip(1) {
                if let Some((key, rest)) = literal(part)
                    && let Some((english, _)) = rest.trim_start().strip_prefix(',').and_then(literal)
                {
                    found.push((key, english));
                }
            }
        }
        found
    }

    /// The English written where the core crate makes a message is the English of the message in
    /// `en.ftl` (one is the fallback when there is no window, the other what the window shows).
    #[test]
    fn core_messages_match_the_english_file() {
        let messages = core_messages();
        assert!(messages.len() > 30, "found only {} messages in the core sources", messages.len());
        for (key, english) in messages {
            let (key, english): (&'static str, &'static str) = (Box::leak(key.into_boxed_str()), Box::leak(english.into_boxed_str()));
            // every {name} or {name:one|other} of the English text, as an argument
            let names: Vec<&'static str> = english
                .split('{')
                .skip(1)
                .filter_map(|p| p.split('}').next())
                .map(|inner| inner.split(':').next().unwrap_or(inner))
                .map(|n| &*Box::leak(n.to_string().into_boxed_str()))
                .collect();
            let mut msg = crate::msg::Msg::new(key, english);
            let mut args = vec![];
            for name in names {
                let value = if name == "n" { "3".to_string() } else { format!("<{name}>") };
                msg = msg.arg(name, &value);
                args.push((name, value));
            }
            let shown = translate(key, &args).unwrap_or_else(|| panic!("{key}: no such message in en.ftl"));
            assert_eq!(shown, msg.english(), "{key}: en.ftl and the English in the core crate differ");
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

    /// The `$arguments` each message uses, from the raw text of a language's file.
    fn arguments(code: &str) -> std::collections::BTreeMap<String, BTreeSet<String>> {
        let file = Localizations::get(&format!("{code}/{}", LOADER.language_file_name())).expect("file");
        let text = std::str::from_utf8(&file.data).expect("utf-8").to_owned();
        let mut out = std::collections::BTreeMap::new();
        let mut current = None;
        for line in text.lines() {
            let starts = line.chars().next().is_some_and(|c| c.is_ascii_lowercase()) && line.contains(" =");
            if starts {
                let id = line.split(" =").next().unwrap_or("").to_string();
                out.insert(id.clone(), BTreeSet::new());
                current = Some(id);
            } else if !line.starts_with(' ') && !line.trim().is_empty() {
                current = None;
            }
            if let Some(vars) = current.as_ref().and_then(|id| out.get_mut(id)) {
                for part in line.split('$').skip(1) {
                    let name: String = part.chars().take_while(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '-').collect();
                    vars.insert(name);
                }
            }
        }
        out
    }

    /// A translation uses the arguments of the English message, no more and no fewer.
    #[test]
    fn languages_use_the_english_arguments() {
        let en = arguments("en");
        assert!(en.len() > 500);
        for (code, _) in available() {
            for (id, vars) in arguments(&code) {
                if let Some(english) = en.get(&id) {
                    assert_eq!(&vars, english, "{code}: the arguments of {id} are not those of the English message");
                }
            }
        }
    }

    #[test]
    fn only_some_scripts_need_a_system_font() {
        assert!(script_font_candidates("en").is_empty());
        assert!(script_font_candidates("fr").is_empty());
        for lang in ["ja", "ko", "zh"] {
            assert!(!script_font_candidates(lang).is_empty(), "{lang}");
        }
        // the Noto collection holds the languages one after the other
        let index = |lang: &str| script_font_candidates(lang)[0].1;
        assert_eq!((index("ja"), index("ko"), index("zh")), (0, 1, 2));
    }

    #[test]
    fn the_first_font_that_exists_is_used() {
        let here = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
        let here: &'static str = Box::leak(here.to_string_lossy().into_owned().into_boxed_str());
        assert_eq!(first_existing(&[("/no/such/font.ttc", 0), (here, 3)]).map(|(_, i)| i), Some(3));
        assert!(first_existing(&[("/no/such/font.ttc", 0)]).is_none());
    }

    #[test]
    fn languages_are_listed_by_their_own_names() {
        let list = available();
        assert_eq!(list.first().map(|l| l.0.as_str()), Some("en"));
        assert!(list.iter().any(|(code, name)| code == "fr" && name == "Français"), "{list:?}");
        assert!(list.iter().any(|(code, name)| code == "de" && name == "Deutsch"), "{list:?}");
    }
}
