//! Generate Artwork: an A3 portrait PDF specimen of a collection.
//!
//! Layout (from the artwork draft, designed at 1123×1587 px = A3 at 96 dpi):
//! logo and app name, the collection name, one rule, then one line per font file
//! with three text objects: the font name in a grey system font, the sample text,
//! and the sample text in capitals. Favourites come first with a small amber star.
//! The sample size adapts to the text length: side by side up to 48 px; stacked
//! when side by side would be smaller than 32 px; never below 18 px (then "…").

use krilla::color::rgb;
use krilla::geom::{PathBuilder, Point, Rect};
use krilla::num::NormalizedF32;
use krilla::page::PageSettings;
use krilla::paint::{Fill, FillRule};
use krilla::surface::Surface;
use krilla::text::{Font, TextDirection};
use krilla::Document;

/// One line of the artwork.
#[derive(Debug, Clone)]
pub struct ArtworkRow {
    pub path: String,
    pub index: u32,
    /// Shown in the grey column, e.g. "Fraunces Light".
    pub name: String,
    pub favourite: bool,
    /// The font's licence doesn't allow embedding: draw the letters as outlines.
    pub no_embedding: bool,
}

pub struct ArtworkSpec {
    pub title: String,
    pub sample: String,
    pub rows: Vec<ArtworkRow>,
    /// Plain and semibold system font for the labels (Segoe UI on Windows).
    pub label_font: Vec<u8>,
    pub label_bold: Vec<u8>,
}

pub struct ArtworkResult {
    pub pdf: Vec<u8>,
    pub pages: usize,
    /// Fonts that could not be read; they are listed with a note instead.
    pub failed: Vec<String>,
    pub outlined: Vec<String>,
}

// The design is in px at 96 dpi; PDF units are points.
const K: f32 = 0.75;
const PAGE_W: f32 = 1123.0;
const PAGE_H: f32 = 1587.0;
const MARGIN: f32 = 72.0;
const NAME_COL: f32 = 200.0;
const GAP: f32 = 24.0;
const CONTENT: f32 = PAGE_W - 2.0 * MARGIN - NAME_COL - GAP;
const MAX: f32 = 48.0;
const MIN_SIDE: f32 = 32.0;
const MIN: f32 = 18.0;
const HEADER_H: f32 = 40.0 + 28.0 + 54.0 + 28.0; // logo, gap, title, gap
const FOOTER_H: f32 = 40.0;

const LOGO: [&str; 5] = [
    "M23.74,29.79c-.16-.16-.37-.25-.61-.25h-8.72c-.26,0-.38.13-.38.38v19.71c0,.26.13.38.38.38h4.35c.26,0,.38-.13.38-.38v-16h2.83v16c0,.26.13.38.38.38h4.35c.26,0,.38-.13.38-.38v-16.16c0-.24-.09-.44-.26-.61l-3.07-3.07Z",
    "M29.62,29.54h5.11v20.47h-5.11Z",
    "M42.29,44.3h-4.91c-.26,0-.38.14-.38.41v4.88c0,.28.13.41.38.41h4.91c.26,0,.38-.14.38-.41v-4.88c0-.28-.13-.41-.38-.41Z",
    "M14.4,27.15h8.72c.25,0,.45-.08.62-.25l3.07-3.07c.17-.17.26-.38.26-.61v-2.87c0-.23-.09-.46-.26-.62l-2.19-2.19c-.34-.34-.34-.9,0-1.24l2.19-2.19c.16-.16.26-.39.26-.62v-2.87c0-.24-.09-.44-.26-.62l-3.07-3.07c-.16-.16-.37-.25-.62-.25h-8.72c-.26,0-.38.13-.38.38v19.71c0,.21.17.38.38.38ZM19.13,10.76h2.83v3.15l-2.38,2.38c-.34.34-.34.9,0,1.24l2.38,2.38v3.15h-2.83v-12.3Z",
    "M42.29,6.68h-4.35c-.26,0-.38.13-.38.38v16h-2.83V7.06c0-.26-.13-.38-.38-.38h-4.35c-.26,0-.38.13-.38.38v16.16c0,.24.09.44.26.61l3.07,3.07c.16.16.37.25.61.25h8.72c.26,0,.38-.13.38-.38V7.06c0-.26-.13-.38-.38-.38Z",
];

const STAR: &str = "M12 3l2.7 5.6 6.1.9-4.4 4.3 1 6.1L12 17l-5.4 2.9 1-6.1L3.2 9.5l6.1-.9z";

/// Width of `text` at 1 px, from the font's advances.
fn text_width(font: &ttf_parser::Face, text: &str) -> f32 {
    let upem = font.units_per_em() as f32;
    let mut w = 0.0;
    for c in text.chars() {
        let adv = font.glyph_index(c).and_then(|g| font.glyph_hor_advance(g)).unwrap_or((upem * 0.5) as u16);
        w += adv as f32;
    }
    w / upem
}

/// Cuts `text` so it fits `max` px at `size`, ending in "…".
fn fit(face: &ttf_parser::Face, text: &str, size: f32, max: f32) -> String {
    if text_width(face, text) * size <= max {
        return text.to_string();
    }
    let chars: Vec<char> = text.chars().collect();
    for n in (0..chars.len()).rev() {
        let t: String = chars[..n].iter().collect::<String>().trim_end().to_string() + "…";
        if text_width(face, &t) * size <= max {
            return t;
        }
    }
    "…".into()
}

fn fill(r: u8, g: u8, b: u8) -> Fill {
    Fill { paint: rgb::Color::new(r, g, b).into(), opacity: NormalizedF32::ONE, rule: FillRule::NonZero }
}

/// Minimal SVG path reader (M L H V C S Z, absolute and relative) for the logo and star.
fn svg_path(d: &str, scale: f32, dx: f32, dy: f32) -> Option<krilla::geom::Path> {
    let mut pb = PathBuilder::new();
    let mut nums: Vec<f32> = Vec::new();
    let mut cmd = 'M';
    let (mut x, mut y) = (0.0f32, 0.0f32);
    let (mut sx, mut sy) = (0.0f32, 0.0f32);
    let (mut cx, mut cy) = (0.0f32, 0.0f32); // last control point for S
    let mut tokens: Vec<String> = Vec::new();
    let mut cur = String::new();
    for ch in d.chars() {
        if ch.is_ascii_alphabetic() {
            if !cur.is_empty() {
                tokens.push(std::mem::take(&mut cur));
            }
            tokens.push(ch.to_string());
        } else if ch == ',' || ch.is_whitespace() {
            if !cur.is_empty() {
                tokens.push(std::mem::take(&mut cur));
            }
        } else if ch == '-' && !cur.is_empty() && !cur.ends_with('e') {
            tokens.push(std::mem::take(&mut cur));
            cur.push(ch);
        } else if ch == '.' && cur.contains('.') {
            tokens.push(std::mem::take(&mut cur));
            cur.push(ch);
        } else {
            cur.push(ch);
        }
    }
    if !cur.is_empty() {
        tokens.push(cur);
    }
    let t = |px: f32, py: f32| (dx + px * scale, dy + py * scale);
    let mut run = |cmd: char, n: &[f32], pb: &mut PathBuilder| {
        let rel = cmd.is_ascii_lowercase();
        let (ox, oy) = if rel { (x, y) } else { (0.0, 0.0) };
        match cmd.to_ascii_uppercase() {
            'M' => {
                x = ox + n[0];
                y = oy + n[1];
                sx = x;
                sy = y;
                let (a, b) = t(x, y);
                pb.move_to(a, b);
                cx = x;
                cy = y;
            }
            'L' => {
                x = ox + n[0];
                y = oy + n[1];
                let (a, b) = t(x, y);
                pb.line_to(a, b);
                cx = x;
                cy = y;
            }
            'H' => {
                x = if rel { x + n[0] } else { n[0] };
                let (a, b) = t(x, y);
                pb.line_to(a, b);
                cx = x;
                cy = y;
            }
            'V' => {
                y = if rel { y + n[0] } else { n[0] };
                let (a, b) = t(x, y);
                pb.line_to(a, b);
                cx = x;
                cy = y;
            }
            'C' => {
                let (x1, y1, x2, y2, ex, ey) = (ox + n[0], oy + n[1], ox + n[2], oy + n[3], ox + n[4], oy + n[5]);
                let (a1, b1) = t(x1, y1);
                let (a2, b2) = t(x2, y2);
                let (a3, b3) = t(ex, ey);
                pb.cubic_to(a1, b1, a2, b2, a3, b3);
                cx = x2;
                cy = y2;
                x = ex;
                y = ey;
            }
            'S' => {
                let (x1, y1) = (2.0 * x - cx, 2.0 * y - cy);
                let (x2, y2, ex, ey) = (ox + n[0], oy + n[1], ox + n[2], oy + n[3]);
                let (a1, b1) = t(x1, y1);
                let (a2, b2) = t(x2, y2);
                let (a3, b3) = t(ex, ey);
                pb.cubic_to(a1, b1, a2, b2, a3, b3);
                cx = x2;
                cy = y2;
                x = ex;
                y = ey;
            }
            'Z' => {
                pb.close();
                x = sx;
                y = sy;
            }
            _ => {}
        }
    };
    let arity = |c: char| match c.to_ascii_uppercase() {
        'M' | 'L' => 2,
        'H' | 'V' => 1,
        'C' => 6,
        'S' => 4,
        _ => 0,
    };
    for tok in tokens {
        let c = tok.chars().next()?;
        if c.is_ascii_alphabetic() {
            cmd = c;
            nums.clear();
            if arity(cmd) == 0 {
                run(cmd, &[], &mut pb);
            }
            continue;
        }
        nums.push(tok.parse().ok()?);
        let a = arity(cmd);
        if a > 0 && nums.len() == a {
            run(cmd, &nums, &mut pb);
            nums.clear();
            // Extra coordinate pairs after M are line-tos.
            if cmd == 'M' {
                cmd = 'L';
            } else if cmd == 'm' {
                cmd = 'l';
            }
        }
    }
    pb.finish()
}

struct Loaded {
    data: Vec<u8>,
    index: u32,
    font: Font,
}

fn load(path: &str, index: u32) -> Option<Loaded> {
    let data = std::fs::read(path).ok()?;
    ttf_parser::Face::parse(&data, index).ok()?;
    let font = Font::new(data.clone().into(), index)?;
    Some(Loaded { data, index, font })
}

fn text(surface: &mut Surface, font: &Font, size_px: f32, x_px: f32, baseline_px: f32, s: &str, outlined: bool, color: (u8, u8, u8)) {
    surface.set_fill(Some(fill(color.0, color.1, color.2)));
    surface.draw_text(Point::from_xy(x_px * K, baseline_px * K), font.clone(), size_px * K, s, outlined, TextDirection::Auto);
}

pub fn render_pdf(spec: &ArtworkSpec) -> Result<ArtworkResult, String> {
    let label = Font::new(spec.label_font.clone().into(), 0).ok_or("The label font could not be read")?;
    let label_b = Font::new(spec.label_bold.clone().into(), 0).unwrap_or_else(|| label.clone());
    let label_face = ttf_parser::Face::parse(&spec.label_font, 0).map_err(|_| "The label font could not be read")?;

    let sample = if spec.sample.trim().is_empty() { spec.title.clone() } else { spec.sample.clone() };
    let caps = sample.to_uppercase();

    // Load every font once and measure the sample.
    let mut loaded: Vec<Option<Loaded>> = Vec::with_capacity(spec.rows.len());
    let mut failed = Vec::new();
    let mut outlined = Vec::new();
    let mut widths = Vec::new();
    for r in &spec.rows {
        let l = load(&r.path, r.index);
        match &l {
            Some(l) => {
                let face = ttf_parser::Face::parse(&l.data, l.index).unwrap();
                widths.push(text_width(&face, &sample).max(text_width(&face, &caps)));
            }
            None => failed.push(r.name.clone()),
        }
        if r.no_embedding {
            outlined.push(r.name.clone());
        }
        loaded.push(l);
    }
    widths.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let typical = widths.get(widths.len() / 2).copied().unwrap_or(sample.chars().count() as f32 * 0.55).max(0.01);

    // Same rule as the draft: side by side up to 48 px; stacked below 32 px; never below 18 px.
    let side_fit = ((CONTENT - GAP) / 2.0 / typical).floor();
    let stacked = side_fit < MIN_SIDE;
    let size = if stacked { (CONTENT / typical).floor().clamp(MIN, MAX) } else { side_fit.min(MAX) };
    let line = (size * 1.25).ceil();
    let row_h = if stacked { line * 2.0 + 8.0 + 28.0 } else { (line + 30.0).max(64.0) };
    let slot = if stacked { CONTENT } else { (CONTENT - GAP) / 2.0 };

    let avail = PAGE_H - 2.0 * MARGIN - HEADER_H - FOOTER_H;
    let per = ((avail / row_h).floor() as usize).max(1);
    let pages = spec.rows.len().div_ceil(per).max(1);

    let mut doc = Document::new();
    for p in 0..pages {
        let mut page = doc.start_page_with(PageSettings::from_wh(PAGE_W * K, PAGE_H * K).ok_or("page size")?);
        let mut s = page.surface();
        // White page.
        s.set_fill(Some(fill(255, 255, 255)));
        let mut pb = PathBuilder::new();
        pb.push_rect(Rect::from_xywh(0.0, 0.0, PAGE_W * K, PAGE_H * K).unwrap());
        if let Some(bg) = pb.finish() {
            s.draw_path(&bg);
        }

        // Header: logo, app name, collection name, one rule.
        let top = MARGIN;
        s.set_fill(Some(fill(0xFF, 0xBF, 0x00)));
        let mut pb = PathBuilder::new();
        pb.push_rect(Rect::from_xywh(MARGIN * K, top * K, 40.0 * K, 40.0 * K).unwrap());
        if let Some(sq) = pb.finish() {
            s.draw_path(&sq);
        }
        s.set_fill(Some(fill(0x34, 0x34, 0x34)));
        for d in LOGO {
            if let Some(path) = svg_path(d, 40.0 / 56.69 * K, MARGIN * K, top * K) {
                s.draw_path(&path);
            }
        }
        text(&mut s, &label_b, 18.0, MARGIN + 52.0, top + 26.0, "Buni Font Manager", false, (0x34, 0x34, 0x34));
        let title_base = top + 40.0 + 28.0 + 40.0;
        text(&mut s, &label_b, 40.0, MARGIN, title_base, &fit(&label_face, &spec.title, 40.0, PAGE_W - 2.0 * MARGIN), false, (0, 0, 0));
        let rule_y = top + HEADER_H;
        s.set_fill(Some(fill(0, 0, 0)));
        let mut pb = PathBuilder::new();
        pb.push_rect(Rect::from_xywh(MARGIN * K, rule_y * K, (PAGE_W - 2.0 * MARGIN) * K, 2.0 * K).unwrap());
        if let Some(rule) = pb.finish() {
            s.draw_path(&rule);
        }

        // Rows.
        let mut y = rule_y + 2.0;
        for i in p * per..((p + 1) * per).min(spec.rows.len()) {
            let r = &spec.rows[i];
            let name_x = MARGIN + 20.0;
            let text_x = MARGIN + NAME_COL + GAP;
            let (name_base, first_base) = if stacked {
                let b = y + 14.0 + size * 0.95;
                (y + 14.0 + size * 0.6, b)
            } else {
                let mid = y + row_h / 2.0;
                (mid + 4.5, mid + size * 0.35)
            };
            if r.favourite {
                s.set_fill(Some(fill(0xFF, 0xBF, 0x00)));
                if let Some(star) = svg_path(STAR, 14.0 / 24.0 * K, MARGIN * K, (name_base - 11.5) * K) {
                    s.draw_path(&star);
                }
            }
            let name = fit(&label_face, &r.name, 13.0, NAME_COL - 24.0);
            text(&mut s, &label, 13.0, name_x, name_base, &name, false, (0x76, 0x76, 0x76));
            match &loaded[i] {
                Some(l) => {
                    let face = ttf_parser::Face::parse(&l.data, l.index).unwrap();
                    let a = fit(&face, &sample, size, slot);
                    let b = fit(&face, &caps, size, slot);
                    text(&mut s, &l.font, size, text_x, first_base, &a, r.no_embedding, (0, 0, 0));
                    if stacked {
                        text(&mut s, &l.font, size, text_x, first_base + line + 8.0, &b, r.no_embedding, (0, 0, 0));
                    } else {
                        text(&mut s, &l.font, size, text_x + slot + GAP, first_base, &b, r.no_embedding, (0, 0, 0));
                    }
                }
                None => text(&mut s, &label, 13.0, text_x, name_base, "This font file could not be read", false, (0x76, 0x76, 0x76)),
            }
            y += row_h;
        }

        // Page number.
        let label_txt = format!("Page {} of {}", p + 1, pages);
        let w = text_width(&label_face, &label_txt) * 12.0;
        text(&mut s, &label, 12.0, PAGE_W - MARGIN - w, PAGE_H - MARGIN, &label_txt, false, (0x76, 0x76, 0x76));
        s.finish();
        page.finish();
    }
    let pdf = doc.finish().map_err(|e| format!("Could not write the PDF: {e:?}"))?;
    Ok(ArtworkResult { pdf, pages, failed, outlined })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(name: &str) -> String {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures").join(name).to_string_lossy().to_string()
    }

    #[test]
    fn renders_pages() {
        let label = std::fs::read(fixture("SpaceMono-Regular.ttf")).unwrap();
        let mut rows = Vec::new();
        for i in 0..40 {
            rows.push(ArtworkRow {
                path: fixture(if i % 2 == 0 { "PinyonScript-Regular.ttf" } else { "SpaceMono-Bold.ttf" }),
                index: 0,
                name: format!("Font {i}"),
                favourite: i < 3,
                no_embedding: false,
            });
        }
        rows.push(ArtworkRow { path: "/nope.ttf".into(), index: 0, name: "Missing".into(), favourite: false, no_embedding: false });
        let spec = ArtworkSpec { title: "Brand A".into(), sample: "I believe I can fly and touch the sky".into(), rows, label_font: label.clone(), label_bold: label };
        let r = render_pdf(&spec).unwrap();
        assert!(r.pdf.starts_with(b"%PDF"));
        assert!(r.pages >= 2);
        assert_eq!(r.failed, vec!["Missing".to_string()]);
        if let Ok(dir) = std::env::var("BFM_ARTWORK_OUT") {
            std::fs::write(std::path::Path::new(&dir).join("artwork-test.pdf"), &r.pdf).unwrap();
        }
    }
}
