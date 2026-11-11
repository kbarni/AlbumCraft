//! Text the core crate shows to people (errors, reports): the English wording stays at the place
//! that makes it, with a key for the translation. `msg!("key", "English with {name}", name = value)`
//! gives a `Msg`; turning it into a `String` (`.into()`, `?`, `to_string()`) writes it in the
//! language the window installed with `set_translator`, and in English when there is none
//! (tests, `--mcp` before it starts the window's language). Messages for agents (`commands`,
//! `mcp`, `control`) and the system's own error texts stay as they are.

use std::fmt;
use std::sync::OnceLock;

/// Writes the message `key` with its arguments in the current language; None when it has no
/// translation (the English text is used then).
pub type Translator = fn(&str, &[(&'static str, String)]) -> Option<String>;

static TRANSLATOR: OnceLock<Translator> = OnceLock::new();

/// Install the translator (the first call counts).
pub fn set_translator(t: Translator) {
    let _ = TRANSLATOR.set(t);
}

#[derive(Clone, Debug, PartialEq)]
pub struct Msg {
    key: &'static str,
    english: &'static str,
    args: Vec<(&'static str, String)>,
}

impl Msg {
    pub fn new(key: &'static str, english: &'static str) -> Msg {
        Msg { key, english, args: vec![] }
    }

    pub fn arg(mut self, name: &'static str, value: impl ToString) -> Msg {
        self.args.push((name, value.to_string()));
        self
    }

    pub fn key(&self) -> &'static str {
        self.key
    }

    /// The English text: `{name}` is replaced by the argument, and `{name:one|other}` by one of
    /// two words, by whether the argument is 1.
    pub fn english(&self) -> String {
        let value = |name: &str| self.args.iter().find(|(n, _)| *n == name).map(|(_, v)| v.as_str());
        let mut out = String::new();
        let mut rest = self.english;
        while let Some(start) = rest.find('{') {
            out.push_str(&rest[..start]);
            let Some(len) = rest[start..].find('}') else { break };
            let inner = &rest[start + 1..start + len];
            match inner.split_once(':') {
                Some((name, forms)) => {
                    let (one, other) = forms.split_once('|').unwrap_or((forms, forms));
                    out.push_str(if value(name) == Some("1") { one } else { other });
                }
                None => out.push_str(value(inner).unwrap_or("")),
            }
            rest = &rest[start + len + 1..];
        }
        out.push_str(rest);
        out
    }
}

impl fmt::Display for Msg {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match TRANSLATOR.get().and_then(|t| t(self.key, &self.args)) {
            Some(text) => f.write_str(&text),
            None => f.write_str(&self.english()),
        }
    }
}

impl From<Msg> for String {
    fn from(m: Msg) -> String {
        m.to_string()
    }
}

/// `msg!("key", "English text with {name}", name = value, …)`.
#[macro_export]
macro_rules! msg {
    ($key:literal, $english:literal $(, $name:ident = $value:expr)* $(,)?) => {
        $crate::msg::Msg::new($key, $english)$(.arg(stringify!($name), $value))*
    };
}

#[cfg(test)]
mod tests {
    #[test]
    fn english_without_a_translator() {
        let m = crate::msg!("test-key", "Could not read {path} ({error}).", path = "a.jpg", error = "denied");
        assert_eq!(m.key(), "test-key");
        assert_eq!(String::from(m.clone()), "Could not read a.jpg (denied).");
        assert_eq!(m.to_string(), m.english());
        let n = |n: usize| crate::msg!("test-plural", "{n} {n:picture|pictures} left out.", n = n).to_string();
        assert_eq!(n(1), "1 picture left out.");
        assert_eq!(n(3), "3 pictures left out.");
    }
}
