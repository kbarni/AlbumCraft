//! The fonts installed on this computer, for the editor's text: the families to choose from and
//! the face for a family, bold and italic. The edit crate reads no files, so `install` tells it
//! how to find them. A picture opened on a computer without a font is drawn in the built-in one.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use ab_glyph::{FontArc, FontVec};
use albumcraft_edit::decor::{self, Face};

struct Fonts {
    db: fontdb::Database,
    families: Vec<String>,
    /// Faces already read, by (family in lower case, bold, italic).
    loaded: Mutex<HashMap<(String, bool, bool), Option<Face>>>,
}

static FONTS: OnceLock<Fonts> = OnceLock::new();

fn fonts() -> &'static Fonts {
    FONTS.get_or_init(|| {
        let mut db = fontdb::Database::new();
        db.load_system_fonts();
        let mut families: Vec<String> = db
            .faces()
            .filter(|f| !f.monospaced)
            .filter_map(|f| f.families.first().map(|(name, _)| name.clone()))
            .filter(|n| !n.starts_with('.'))
            .collect();
        families.sort_by_key(|n| n.to_lowercase());
        families.dedup_by_key(|n| n.to_lowercase());
        Fonts { db, families, loaded: Mutex::new(HashMap::new()) }
    })
}

/// Start reading the installed fonts on a thread of their own (it takes a moment with many
/// fonts) and let the editor's text use them.
pub fn install() {
    decor::set_font_resolver(face);
    std::thread::spawn(|| {
        fonts();
    });
}

/// The font families to offer, sorted; empty until the fonts have been read.
pub fn families() -> &'static [String] {
    FONTS.get().map_or(&[], |f| &f.families)
}

/// The face of `family` that comes nearest to the bold and italic asked, and whether it really
/// is bold / italic. None when the computer has no such family.
pub fn face(family: &str, bold: bool, italic: bool) -> Option<Face> {
    let fonts = fonts();
    let key = (family.to_lowercase(), bold, italic);
    if let Some(found) = fonts.loaded.lock().ok()?.get(&key) {
        return found.clone();
    }
    let found = read(fonts, family, bold, italic);
    if let Ok(mut m) = fonts.loaded.lock() {
        m.insert(key, found.clone());
    }
    found
}

fn read(fonts: &Fonts, family: &str, bold: bool, italic: bool) -> Option<Face> {
    let query = fontdb::Query {
        families: &[fontdb::Family::Name(family)],
        weight: if bold { fontdb::Weight::BOLD } else { fontdb::Weight::NORMAL },
        stretch: fontdb::Stretch::Normal,
        style: if italic { fontdb::Style::Italic } else { fontdb::Style::Normal },
    };
    let id = fonts.db.query(&query)?;
    let info = fonts.db.face(id)?;
    let (is_bold, is_italic) = (info.weight.0 >= 600, info.style != fontdb::Style::Normal);
    let font = fonts.db.with_face_data(id, |data, index| FontVec::try_from_vec_and_index(data.to_vec(), index).ok())??;
    Some(Face { font: FontArc::new(font), bold: is_bold, italic: is_italic })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_unknown_family_is_none() {
        assert!(face("No Such Family 123", false, false).is_none());
    }

    #[test]
    fn an_installed_family_is_found_with_its_real_styles() {
        // only checks what this computer has
        let Some(name) = fonts().families.first().cloned() else { return };
        let plain = face(&name, false, false).expect("listed family");
        assert!(!plain.italic);
        let bold = face(&name, true, false).unwrap();
        // what it says about itself matches what was asked when it has such a face
        assert!(bold.bold || !plain.bold);
    }
}
