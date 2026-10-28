//! Reads what we need from a font file: names, weight, width, style flags,
//! a classification (Serif, Script…) and which languages it covers.

use crate::shape::{analyse, classify_shape, spacing_shape};
use ttf_parser::{name_id, Face, Tag};

#[derive(Debug, Clone, PartialEq)]
pub struct FontInfo {
    pub family: String,
    pub style: String,
    pub full_name: String,
    pub postscript_name: String,
    pub weight: u16,
    pub width: u16,
    pub italic: bool,
    pub monospace: bool,
    pub variable: bool,
    pub axes: Vec<String>,
    pub classification: String,
    pub spacing: String,
    pub langs: Vec<String>,
    pub no_embedding: bool,
}

pub const LANG_SWEDISH: &str = "Swedish (å ä ö)";
pub const LANG_PORTUGUESE: &str = "Portuguese (ã ç)";
pub const LANG_CYRILLIC: &str = "Cyrillic";
pub const LANG_GREEK: &str = "Greek";

#[derive(Debug, thiserror::Error)]
pub enum ReadError {
    #[error("not a font file the app can read")]
    Unreadable,
    #[error("font collection files (.ttc) are not supported yet")]
    Collection,
}

pub fn is_font_file_name(name: &str) -> bool {
    let n = name.to_ascii_lowercase();
    n.ends_with(".otf") || n.ends_with(".ttf")
}

pub fn read_font(data: &[u8]) -> Result<FontInfo, ReadError> {
    if data.len() >= 4 && &data[0..4] == b"ttcf" {
        return Err(ReadError::Collection);
    }
    let face = Face::parse(data, 0).map_err(|_| ReadError::Unreadable)?;

    let name = |ids: &[u16]| -> Option<String> {
        for id in ids {
            // Prefer Windows/Unicode records, then anything readable.
            let mut fallback = None;
            for n in face.names() {
                if n.name_id != *id {
                    continue;
                }
                if let Some(s) = n.to_string() {
                    let s = s.trim().to_string();
                    if s.is_empty() {
                        continue;
                    }
                    if n.platform_id == ttf_parser::PlatformId::Windows {
                        return Some(s);
                    }
                    fallback.get_or_insert(s);
                }
            }
            if fallback.is_some() {
                return fallback;
            }
        }
        None
    };

    let family = name(&[name_id::TYPOGRAPHIC_FAMILY, name_id::FAMILY]).ok_or(ReadError::Unreadable)?;
    let style = name(&[name_id::TYPOGRAPHIC_SUBFAMILY, name_id::SUBFAMILY]).unwrap_or_else(|| "Regular".into());
    let full_name = name(&[name_id::FULL_NAME]).unwrap_or_else(|| format!("{family} {style}"));
    let postscript_name = name(&[name_id::POST_SCRIPT_NAME]).unwrap_or_else(|| full_name.replace(' ', ""));

    let weight = face.weight().to_number();
    let width = face.width().to_number();
    let italic = face.is_italic() || face.is_oblique();
    let monospace = face.is_monospaced();
    let variable = face.is_variable();
    let axes = face.variation_axes().into_iter().map(|a| a.tag.to_string()).collect();

    let os2 = face.raw_face().table(Tag::from_bytes(b"OS/2"));
    let (family_class, panose, fs_type) = match os2 {
        Some(t) if t.len() >= 42 => {
            let class = i16::from_be_bytes([t[30], t[31]]);
            let mut p = [0u8; 10];
            p.copy_from_slice(&t[32..42]);
            let fs = u16::from_be_bytes([t[8], t[9]]);
            (Some(class), Some(p), fs)
        }
        _ => (None, None, 0),
    };

    let shape = analyse(&face);
    let classification = classify(&family, &full_name, family_class, panose, classify_shape(&shape, italic));
    // Swash capitals make scripts look wide, so only measure width for regular text styles.
    let measured = if matches!(classification.as_str(), "Script" | "Handwritten") { None } else { spacing_shape(&shape) };
    let spacing = spacing_of(monospace || shape.fixed_pitch, width, &format!("{family} {style}"), measured);

    let has_all = |chars: &str| chars.chars().all(|c| face.glyph_index(c).is_some());
    let mut langs = Vec::new();
    if has_all("åäöÅÄÖ") {
        langs.push(LANG_SWEDISH.to_string());
    }
    if has_all("ãõçÃÕÇáéíóúâêô") {
        langs.push(LANG_PORTUGUESE.to_string());
    }
    if has_all("АБВДЖЗИЯабвджзия") {
        langs.push(LANG_CYRILLIC.to_string());
    }
    if has_all("ΑΒΓΔΩαβγδλω") {
        langs.push(LANG_GREEK.to_string());
    }

    // fsType bit 1 (0x0002) = restricted licence embedding: may not be embedded.
    let no_embedding = fs_type & 0x000F == 0x0002;

    Ok(FontInfo {
        family,
        style,
        full_name,
        postscript_name,
        weight,
        width,
        italic,
        monospace,
        variable,
        axes,
        classification,
        spacing,
        langs,
        no_embedding,
    })
}

/// Picks one of: Serif, Sans serif, Script, Decorative, Handwritten, Blackletter, Symbols.
/// Uses the font's own name first (designers name these reliably), then the
/// OS/2 family class, then PANOSE.
pub fn classify(family: &str, full_name: &str, family_class: Option<i16>, panose: Option<[u8; 10]>, from_shape: Option<&str>) -> String {
    let n = format!("{family} {full_name}").to_lowercase();
    let has = |words: &[&str]| words.iter().any(|w| n.contains(w));

    if has(&["fraktur", "blackletter", "black letter", "textura", "gothic text", "old english", "rotunda", "schwabacher"]) {
        return "Blackletter".into();
    }
    if has(&["handwrit", "hand written", "marker", "brush"]) {
        return "Handwritten".into();
    }
    if has(&["script", "calligraph", "signature"]) {
        return "Script".into();
    }
    if has(&["symbol", "icon", "dingbat", "ornament", "emoji"]) {
        return "Symbols".into();
    }
    if has(&["sans"]) {
        return "Sans serif".into();
    }
    if has(&["serif", "garamond", "didone", "slab"]) {
        return "Serif".into();
    }

    if let Some(c) = family_class {
        match (c >> 8) as u8 {
            1..=5 | 7 => return "Serif".into(),
            8 => return "Sans serif".into(),
            9 => return "Decorative".into(),
            10 => return "Script".into(),
            12 => return "Symbols".into(),
            _ => {}
        }
    }

    if let Some(p) = panose {
        match p[0] {
            3 => return "Script".into(),
            4 => return "Decorative".into(),
            5 => return "Symbols".into(),
            _ => {}
        }
    }

    // Most fonts leave the fields above empty, so the letter shapes decide.
    if let Some(style) = from_shape {
        return style.into();
    }

    if let Some(p) = panose {
        if p[0] == 2 && p[1] >= 2 {
            // bSerifStyle 11–15 are sans styles.
            return if (11..=15).contains(&p[1]) { "Sans serif".into() } else { "Serif".into() };
        }
    }

    "Sans serif".into()
}

pub fn spacing_of(monospace: bool, width: u16, name: &str, from_shape: Option<&str>) -> String {
    let n = name.to_lowercase();
    if monospace || n.contains(" mono") {
        return "Monospace".into();
    }
    if width <= 4 || n.contains("condensed") || n.contains("narrow") || n.contains("compressed") || n.contains("compact") {
        return "Condensed".into();
    }
    if width >= 6 || n.contains("extended") || n.contains("expanded") || n.contains("wide") {
        return "Expanded".into();
    }
    // Most fonts say width 5 (normal) whatever they look like; measure instead.
    from_shape.unwrap_or("Normal").into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_by_name_first() {
        assert_eq!(classify("Pinyon Script", "Pinyon Script", Some(0x0800), None, None), "Script");
        assert_eq!(classify("UnifrakturMaguntia", "UnifrakturMaguntia", None, None, Some("Blackletter")), "Blackletter");
        assert_eq!(classify("Open Sans", "Open Sans", Some(0x0100), None, None), "Sans serif");
    }

    #[test]
    fn classify_falls_back_to_os2_and_panose() {
        assert_eq!(classify("Fraunces", "Fraunces", Some(0x0102), None, None), "Serif");
        assert_eq!(classify("Inter", "Inter", Some(0x0800), None, None), "Sans serif");
        assert_eq!(classify("Foo", "Foo", Some(0), Some([3, 0, 0, 0, 0, 0, 0, 0, 0, 0]), None), "Script");
        assert_eq!(classify("Foo", "Foo", None, Some([2, 11, 0, 0, 0, 0, 0, 0, 0, 0]), None), "Sans serif");
        assert_eq!(classify("Foo", "Foo", None, None, None), "Sans serif");
        assert_eq!(classify("Foo", "Foo", Some(0), Some([0; 10]), Some("Serif")), "Serif");
    }

    #[test]
    fn spacing_rules() {
        assert_eq!(spacing_of(true, 5, "X", None), "Monospace");
        assert_eq!(spacing_of(false, 3, "X", None), "Condensed");
        assert_eq!(spacing_of(false, 5, "Archivo Narrow", None), "Condensed");
        assert_eq!(spacing_of(false, 7, "X", None), "Expanded");
        assert_eq!(spacing_of(false, 5, "X", None), "Normal");
        assert_eq!(spacing_of(false, 5, "X", Some("Condensed")), "Condensed");
    }
}
