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

/// Reads a single-font file (OTF/TTF). Collections (.ttc) are refused here.
pub fn read_font(data: &[u8]) -> Result<FontInfo, ReadError> {
    if data.len() >= 4 && &data[0..4] == b"ttcf" {
        return Err(ReadError::Collection);
    }
    read_font_at(data, 0)
}

/// Number of fonts in the file (more than one for .ttc collections).
pub fn face_count(data: &[u8]) -> u32 {
    ttf_parser::fonts_in_collection(data).unwrap_or(1)
}

/// Reads the font at `index` of a file; index 0 for normal OTF/TTF files.
pub fn read_font_at(data: &[u8], index: u32) -> Result<FontInfo, ReadError> {
    let face = Face::parse(data, index).map_err(|_| ReadError::Unreadable)?;

    let name = |ids: &[u16]| -> Option<String> {
        for id in ids {
            // Prefer English Windows records (Windows fonts also carry Catalan,
            // German… names), then any Windows record, then anything readable.
            let mut best: Option<(u8, String)> = None;
            for n in face.names() {
                if n.name_id != *id {
                    continue;
                }
                let Some(s) = n.to_string() else { continue };
                let s = s.trim().to_string();
                if s.is_empty() {
                    continue;
                }
                let rank = match (n.platform_id, n.language_id) {
                    (ttf_parser::PlatformId::Windows, 0x0409) => 0,
                    (ttf_parser::PlatformId::Macintosh, 0) => 1,
                    (ttf_parser::PlatformId::Windows, _) => 2,
                    _ => 3,
                };
                if best.as_ref().map(|(r, _)| rank < *r).unwrap_or(true) {
                    best = Some((rank, s));
                }
            }
            if let Some((_, s)) = best {
                return Some(s);
            }
        }
        None
    };

    let family = name(&[name_id::TYPOGRAPHIC_FAMILY, name_id::FAMILY]).ok_or(ReadError::Unreadable)?;
    let style = name(&[name_id::TYPOGRAPHIC_SUBFAMILY, name_id::SUBFAMILY]).unwrap_or_else(|| "Regular".into());
    let full_name = name(&[name_id::FULL_NAME]).unwrap_or_else(|| format!("{family} {style}"));
    let postscript_name = name(&[name_id::POST_SCRIPT_NAME]).unwrap_or_else(|| full_name.replace(' ', ""));

    let weight = refine_weight(face.weight().to_number(), &style);
    let width = face.width().to_number();
    let style_l = style.to_lowercase();
    let italic = face.is_italic() || face.is_oblique() || ["italic", "oblique", "slanted", "kursiv", "cursiva"].iter().any(|w| style_l.contains(w));
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
    let measured = if classification == "Script" { None } else { spacing_shape(&shape) };
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

/// Some fonts (often the ones Adobe Fonts serves) say 400 in every style;
/// then the style name tells the weight.
pub fn refine_weight(os2: u16, style: &str) -> u16 {
    if os2 != 400 && os2 != 0 {
        return os2;
    }
    let s = style.to_lowercase().replace(['-', '_', ' '], "");
    let table: [(&[&str], u16); 9] = [
        (&["hairline", "thin"], 100),
        (&["extralight", "ultralight", "xlight"], 200),
        (&["semilight", "demilight"], 350),
        (&["light"], 300),
        (&["semibold", "demibold", "demi"], 600),
        (&["extrabold", "ultrabold", "xbold", "heavy"], 800),
        (&["extrablack", "ultrablack", "xblack", "black", "ultra", "fat"], 900),
        (&["bold"], 700),
        (&["medium"], 500),
    ];
    for (words, w) in table {
        if words.iter().any(|x| s.contains(x)) {
            return w;
        }
    }
    if os2 == 0 { 400 } else { os2 }
}

/// Picks one of: Serif, Sans serif, Script, Decorative, Blackletter, Symbols.
/// Order of trust: the font's own name, then clear signs in the letter shapes
/// (leaning, broken or tapered strokes), then PANOSE, the OS/2 family class,
/// and finally the measured serifs.
pub fn classify(family: &str, full_name: &str, family_class: Option<i16>, panose: Option<[u8; 10]>, from_shape: Option<&str>) -> String {
    let n = format!("{family} {full_name}").to_lowercase();
    let has = |words: &[&str]| words.iter().any(|w| n.contains(w));

    if has(&["fraktur", "blackletter", "black letter", "textura", "gothic text", "old english", "rotunda", "schwabacher"])
        || (n.contains("goth") && !n.contains("gothic"))
    {
        return "Blackletter".into();
    }
    if has(&["handwrit", "hand written", "marker", "brush", "script", "calligraph", "signature"]) {
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

    // Shapes that only scripts and blackletter have.
    if let Some(s @ ("Script" | "Blackletter")) = from_shape {
        return s.into();
    }
    // A script claim in the file is trusted unless the letters plainly have serifs.
    if panose.map(|p| p[0] == 3).unwrap_or(false) && from_shape != Some("Serif") {
        return "Script".into();
    }
    if from_shape == Some("Decorative") {
        return "Decorative".into();
    }

    if let Some(p) = panose {
        match p[0] {
            4 => return "Decorative".into(),
            5 => return "Symbols".into(),
            // Text faces: the serif style is the most reliable field fonts carry.
            2 if (2..=10).contains(&p[1]) => return "Serif".into(),
            2 if (11..=15).contains(&p[1]) => return "Sans serif".into(),
            _ => {}
        }
    }

    if let Some(c) = family_class {
        match (c >> 8) as u8 {
            1..=5 | 7 => return "Serif".into(),
            8 => return "Sans serif".into(),
            9 => return "Decorative".into(),
            10 if from_shape != Some("Serif") => return "Script".into(),
            12 => return "Symbols".into(),
            _ => {}
        }
    }

    if let Some(style) = from_shape {
        return style.into();
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

/// Every character a font has, plus the glyphs that have no character of their own
/// (alternates, ligatures, swashes) as SVG outlines.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GlyphMap {
    /// Unicode code points the font maps, sorted.
    pub chars: Vec<u32>,
    pub unmapped: Vec<UnmappedGlyph>,
    /// Glyphs left out of `unmapped` because there were too many.
    pub unmapped_more: usize,
    pub units_per_em: u16,
    pub ascender: i16,
    pub descender: i16,
    pub glyph_count: u16,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UnmappedGlyph {
    pub id: u16,
    pub name: Option<String>,
    /// SVG path in font units, y pointing down (baseline at y = 0).
    pub path: String,
    pub advance: u16,
}

struct SvgPath(String);

impl ttf_parser::OutlineBuilder for SvgPath {
    fn move_to(&mut self, x: f32, y: f32) {
        self.0.push_str(&format!("M{} {}", x.round(), (-y).round()));
    }
    fn line_to(&mut self, x: f32, y: f32) {
        self.0.push_str(&format!("L{} {}", x.round(), (-y).round()));
    }
    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        self.0.push_str(&format!("Q{} {} {} {}", x1.round(), (-y1).round(), x.round(), (-y).round()));
    }
    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        self.0.push_str(&format!("C{} {} {} {} {} {}", x1.round(), (-y1).round(), x2.round(), (-y2).round(), x.round(), (-y).round()));
    }
    fn close(&mut self) {
        self.0.push('Z');
    }
}

pub fn glyph_map(data: &[u8], index: u32, max_unmapped: usize) -> Result<GlyphMap, ReadError> {
    let face = Face::parse(data, index).map_err(|_| ReadError::Unreadable)?;
    let mut map: std::collections::BTreeMap<u32, u16> = std::collections::BTreeMap::new();
    if let Some(cmap) = face.tables().cmap {
        for sub in cmap.subtables {
            if !sub.is_unicode() {
                continue;
            }
            sub.codepoints(|cp| {
                if let Some(g) = sub.glyph_index(cp) {
                    if g.0 != 0 {
                        map.entry(cp).or_insert(g.0);
                    }
                }
            });
        }
    }
    let used: std::collections::HashSet<u16> = map.values().copied().collect();
    let mut unmapped = Vec::new();
    let mut more = 0;
    for id in 1..face.number_of_glyphs() {
        if used.contains(&id) {
            continue;
        }
        let gid = ttf_parser::GlyphId(id);
        let mut p = SvgPath(String::new());
        if face.outline_glyph(gid, &mut p).is_none() || p.0.is_empty() {
            continue;
        }
        if unmapped.len() >= max_unmapped {
            more += 1;
            continue;
        }
        unmapped.push(UnmappedGlyph { id, name: face.glyph_name(gid).map(String::from), path: p.0, advance: face.glyph_hor_advance(gid).unwrap_or(0) });
    }
    Ok(GlyphMap {
        chars: map.into_keys().collect(),
        unmapped,
        unmapped_more: more,
        units_per_em: face.units_per_em(),
        ascender: face.ascender(),
        descender: face.descender(),
        glyph_count: face.number_of_glyphs(),
    })
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
        // Century Gothic says "transitional serif" in OS/2 but PANOSE says sans.
        assert_eq!(classify("Century Gothic", "Century Gothic", Some(0x0204), Some([2, 11, 5, 2, 2, 2, 2, 2, 2, 4]), Some("Sans serif")), "Sans serif");
        assert_eq!(classify("CCGothChic", "CCGothChic Black", None, None, Some("Sans serif")), "Blackletter");
        assert_eq!(classify("Unifraktur", "Unifraktur", None, Some([2, 11, 6, 3, 5, 3, 2, 2, 2, 4]), Some("Blackletter")), "Blackletter");
        assert_eq!(classify("Foo", "Foo", None, None, None), "Sans serif");
        assert_eq!(classify("Foo", "Foo", Some(0), Some([0; 10]), Some("Serif")), "Serif");
    }

    #[test]
    fn weight_from_style_name() {
        assert_eq!(refine_weight(400, "Black"), 900);
        assert_eq!(refine_weight(400, "Light"), 300);
        assert_eq!(refine_weight(400, "Bold Italic"), 700);
        assert_eq!(refine_weight(400, "SemiBold"), 600);
        assert_eq!(refine_weight(400, "Regular Italic"), 400);
        assert_eq!(refine_weight(300, "Book"), 300);
    }

    #[test]
    fn lists_characters_and_alternates() {
        let data = std::fs::read(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/PinyonScript-Regular.ttf")).unwrap();
        let m = glyph_map(&data, 0, 5000).unwrap();
        assert!(m.chars.contains(&('A' as u32)));
        assert!(m.chars.windows(2).all(|w| w[0] < w[1]));
        assert!(m.units_per_em > 0);
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
