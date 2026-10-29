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
    /// Names Illustrator knows the font by.
    pub postscript: String,
    pub family: String,
    pub style: String,
}

pub struct ArtworkSpec {
    pub title: String,
    pub sample: String,
    pub rows: Vec<ArtworkRow>,
    /// Plain and semibold system font for the labels (Segoe UI on Windows).
    pub label_font: Vec<u8>,
    pub label_bold: Vec<u8>,
    /// PostScript names of the label fonts (for Illustrator).
    pub label_postscript: String,
    pub label_bold_postscript: String,
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

/// The app logo (136.64 units square): amber square with this dark mark.
pub const LOGO_VIEW: f32 = 136.64;
pub const LOGO_PATH: &str = "M48.41,64.99c1.26-1.37,2.4-2.63,3.55-3.66l-1.83-5.15c-13.05-36.17,8.13-47.28,23.01-46.82,6.07.23,11.22,3.43,19.46,3.43,6.3,0,8.13-5.27,8.13-5.27l.8,2.29s-.57,5.38-6.75,6.53c-7.1,1.37-13.28-3.09-17.74-3.89-14.19-2.98-36.97,7.44-24.38,43.5.34,1.26.8,2.4,1.15,3.55,1.14-1.03,2.17-1.72,2.86-2.17,1.26-14.77,8.59-24.61,21.98-34.68,0,0,8.81,14.65,20.03,17.97,0,0-14.31,9.04-21.75,19.92,0,0,5.15-.8,13.74-.8,14.08,0,24.15,5.84,24.15,17.4,0,19.23-41.44,47.85-41.44,47.85,0,0-2.4-16.71-17.4-22.55-2.86,13.05-12.36,21.86-23.81,21.86-10.65,0-14.19-4.58-14.19-10.99,0-9.27,8.93-20.15,30.33-20.83-1.6-5.38-4.35-11.45-8.47-18.2-14.08-22.55-16.94-34-4.69-44.41,4.35-3.66,8.13-4.58,8.13-4.58l.92,1.49c-10.42,6.07-12.82,14.42,2.75,38,5.49,8.47,8.47,16.71,9.5,24.27.34-5.15.46-10.19-1.95-19.23-3.09-2.52-6.07-4.81-6.07-4.81ZM44.63,100.48c-15.57,0-24.04,5.04-24.04,12.93,0,5.04,3.89,8.7,11.68,8.7,10.87,0,18.66-7.67,17.63-21.29-1.72-.23-3.43-.34-5.27-.34ZM82.52,114.33s22.09-16.14,22.09-29.88c0-12.02-5.84-15-18.32-15-11.45,0-16.37,1.26-16.37,1.26l-2.17-1.72s5.61-9.84,18.77-23.01c0,0-6.87-2.63-14.77-13.39,0,0-11.1,6.87-12.94,24.96,2.86,2.29,8.13,6.18,8.13,6.18-4.24,3.09-6.87,5.5-8.24,6.87.23,7.78,0,15-.92,22.67,18.2,3.89,24.73,21.06,24.73,21.06Z";

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

/// Which font a piece of text uses.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Face {
    Label,
    LabelBold,
    Row(usize),
}

/// One drawing step, in design px (y down from the top of the page).
#[derive(Debug, Clone, PartialEq)]
pub enum Op {
    Rect { x: f32, y: f32, w: f32, h: f32, rgb: (u8, u8, u8) },
    Logo { x: f32, y: f32, size: f32 },
    Star { x: f32, y: f32, size: f32 },
    Text { face: Face, size: f32, x: f32, baseline: f32, text: String, rgb: (u8, u8, u8) },
}

/// The laid-out artwork: pages of drawing steps, shared by the PDF and Illustrator outputs.
pub struct Plan {
    pub pages: Vec<Vec<Op>>,
    pub failed: Vec<String>,
    pub outlined: Vec<String>,
    /// Rows actually used (duplicates of the same font removed).
    pub rows: Vec<ArtworkRow>,
    loaded: Vec<Option<Loaded>>,
}

pub fn page_size_pt() -> (f32, f32) {
    (PAGE_W * K, PAGE_H * K)
}

pub const PX_TO_PT: f32 = K;

/// Lays out the artwork. Fonts that appear twice (same PostScript name) are listed once,
/// so Illustrator never sees two copies of one font.
pub fn plan(spec: &ArtworkSpec) -> Result<Plan, String> {
    let label_face = ttf_parser::Face::parse(&spec.label_font, 0).map_err(|_| "The label font could not be read")?;
    let sample = if spec.sample.trim().is_empty() { spec.title.clone() } else { spec.sample.clone() };
    let caps = sample.to_uppercase();

    let mut seen = std::collections::HashSet::new();
    let rows: Vec<ArtworkRow> = spec.rows.iter().filter(|r| r.postscript.is_empty() || seen.insert(r.postscript.to_lowercase())).cloned().collect();

    let mut loaded: Vec<Option<Loaded>> = Vec::with_capacity(rows.len());
    let mut failed = Vec::new();
    let mut outlined = Vec::new();
    let mut widths = Vec::new();
    for r in &rows {
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
    let count = rows.len().div_ceil(per).max(1);
    const GREY: (u8, u8, u8) = (0x76, 0x76, 0x76);

    let mut pages = Vec::new();
    for p in 0..count {
        let mut ops = Vec::new();
        let top = MARGIN;
        ops.push(Op::Logo { x: MARGIN, y: top, size: 40.0 });
        ops.push(Op::Text { face: Face::LabelBold, size: 18.0, x: MARGIN + 52.0, baseline: top + 26.0, text: "Buni Font Manager".into(), rgb: (0x34, 0x34, 0x34) });
        ops.push(Op::Text {
            face: Face::LabelBold,
            size: 40.0,
            x: MARGIN,
            baseline: top + 40.0 + 28.0 + 40.0,
            text: fit(&label_face, &spec.title, 40.0, PAGE_W - 2.0 * MARGIN),
            rgb: (0, 0, 0),
        });
        let rule_y = top + HEADER_H;
        ops.push(Op::Rect { x: MARGIN, y: rule_y, w: PAGE_W - 2.0 * MARGIN, h: 2.0, rgb: (0, 0, 0) });

        let mut y = rule_y + 2.0;
        for i in p * per..((p + 1) * per).min(rows.len()) {
            let r = &rows[i];
            let name_x = MARGIN + 20.0;
            let text_x = MARGIN + NAME_COL + GAP;
            let (name_base, first_base) = if stacked {
                (y + 14.0 + size * 0.6, y + 14.0 + size * 0.95)
            } else {
                let mid = y + row_h / 2.0;
                (mid + 4.5, mid + size * 0.35)
            };
            if r.favourite {
                ops.push(Op::Star { x: MARGIN, y: name_base - 11.5, size: 14.0 });
            }
            ops.push(Op::Text { face: Face::Label, size: 13.0, x: name_x, baseline: name_base, text: fit(&label_face, &r.name, 13.0, NAME_COL - 24.0), rgb: GREY });
            match &loaded[i] {
                Some(l) => {
                    let face = ttf_parser::Face::parse(&l.data, l.index).unwrap();
                    let a = fit(&face, &sample, size, slot);
                    let b = fit(&face, &caps, size, slot);
                    ops.push(Op::Text { face: Face::Row(i), size, x: text_x, baseline: first_base, text: a, rgb: (0, 0, 0) });
                    let (bx, by) = if stacked { (text_x, first_base + line + 8.0) } else { (text_x + slot + GAP, first_base) };
                    ops.push(Op::Text { face: Face::Row(i), size, x: bx, baseline: by, text: b, rgb: (0, 0, 0) });
                }
                None => ops.push(Op::Text { face: Face::Label, size: 13.0, x: text_x, baseline: name_base, text: "This font file could not be read".into(), rgb: GREY }),
            }
            y += row_h;
        }

        let label_txt = format!("Page {} of {}", p + 1, count);
        let w = text_width(&label_face, &label_txt) * 12.0;
        ops.push(Op::Text { face: Face::Label, size: 12.0, x: PAGE_W - MARGIN - w, baseline: PAGE_H - MARGIN, text: label_txt, rgb: GREY });
        pages.push(ops);
    }
    Ok(Plan { pages, failed, outlined, rows, loaded })
}

fn text(surface: &mut Surface, font: &Font, size_px: f32, x_px: f32, baseline_px: f32, s: &str, outlined: bool, color: (u8, u8, u8)) {
    surface.set_fill(Some(fill(color.0, color.1, color.2)));
    surface.draw_text(Point::from_xy(x_px * K, baseline_px * K), font.clone(), size_px * K, s, outlined, TextDirection::Auto);
}

fn rect(s: &mut Surface, x: f32, y: f32, w: f32, h: f32, rgb: (u8, u8, u8)) {
    s.set_fill(Some(fill(rgb.0, rgb.1, rgb.2)));
    let mut pb = PathBuilder::new();
    if let Some(r) = Rect::from_xywh(x * K, y * K, w * K, h * K) {
        pb.push_rect(r);
    }
    if let Some(path) = pb.finish() {
        s.draw_path(&path);
    }
}

pub fn render_pdf(spec: &ArtworkSpec) -> Result<ArtworkResult, String> {
    let label = Font::new(spec.label_font.clone().into(), 0).ok_or("The label font could not be read")?;
    let label_b = Font::new(spec.label_bold.clone().into(), 0).unwrap_or_else(|| label.clone());
    let plan = plan(spec)?;

    let mut doc = Document::new();
    for ops in &plan.pages {
        let mut page = doc.start_page_with(PageSettings::from_wh(PAGE_W * K, PAGE_H * K).ok_or("page size")?);
        let mut s = page.surface();
        rect(&mut s, 0.0, 0.0, PAGE_W, PAGE_H, (255, 255, 255));
        for op in ops {
            match op {
                Op::Rect { x, y, w, h, rgb } => rect(&mut s, *x, *y, *w, *h, *rgb),
                Op::Logo { x, y, size } => {
                    rect(&mut s, *x, *y, *size, *size, (0xFF, 0xBF, 0x00));
                    s.set_fill(Some(fill(0x34, 0x34, 0x34)));
                    if let Some(path) = svg_path(LOGO_PATH, size / LOGO_VIEW * K, x * K, y * K) {
                        s.draw_path(&path);
                    }
                }
                Op::Star { x, y, size } => {
                    s.set_fill(Some(fill(0xFF, 0xBF, 0x00)));
                    if let Some(star) = svg_path(STAR, size / 24.0 * K, x * K, y * K) {
                        s.draw_path(&star);
                    }
                }
                Op::Text { face, size, x, baseline, text: t, rgb } => {
                    let (font, outlined) = match face {
                        Face::Label => (&label, false),
                        Face::LabelBold => (&label_b, false),
                        Face::Row(i) => match &plan.loaded[*i] {
                            Some(l) => (&l.font, plan.rows[*i].no_embedding),
                            None => continue,
                        },
                    };
                    text(&mut s, font, *size, *x, *baseline, t, outlined, *rgb);
                }
            }
        }
        s.finish();
        page.finish();
    }
    let pdf = doc.finish().map_err(|e| format!("Could not write the PDF: {e:?}"))?;
    Ok(ArtworkResult { pdf, pages: plan.pages.len(), failed: plan.failed, outlined: plan.outlined })
}

/// Escapes text for a JavaScript string literal, keeping the file pure ASCII.
fn js_str(s: &str) -> String {
    let mut o = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            c if (c as u32) < 0x20 || (c as u32) > 0x7e => {
                let mut buf = [0u16; 2];
                for u in c.encode_utf16(&mut buf) {
                    o.push_str(&format!("\\u{:04x}", u));
                }
            }
            c => o.push(c),
        }
    }
    o.push('"');
    o
}

/// Points of the star outline (it has only straight edges), in 24-unit space.
fn star_points() -> Vec<(f32, f32)> {
    let rel = [(2.7, 5.6), (6.1, 0.9), (-4.4, 4.3), (1.0, 6.1)];
    let mut pts = vec![(12.0, 3.0)];
    let (mut x, mut y) = (12.0f32, 3.0f32);
    for (dx, dy) in rel {
        x += dx;
        y += dy;
        pts.push((x, y));
    }
    pts.push((12.0, 17.0));
    let (mut x, mut y) = (12.0f32, 17.0f32);
    for (dx, dy) in [(-5.4, 2.9), (1.0, -6.1)] {
        x += dx;
        y += dy;
        pts.push((x, y));
    }
    pts.push((3.2, 9.5));
    pts.push((3.2 + 6.1, 9.5 - 0.9));
    pts
}

/// An Illustrator script (ExtendScript) that builds the artwork as live text with the
/// real fonts, one A3 artboard per page, and saves it as an .ai file at `ai_path`.
/// `logo_svg` is the path of the logo SVG, placed as vector artwork.
pub fn illustrator_script(spec: &ArtworkSpec, ai_path: &str, logo_svg: &str) -> Result<(String, Plan), String> {
    let plan = plan(spec)?;
    let (w, h) = page_size_pt();
    let mut js = String::new();
    js.push_str("#target illustrator\n// Made by Buni Font Manager\n");
    js.push_str(&format!("var W={w:.3},H={h:.3},K={K},GAP=40,AI_PATH={},LOGO={};\n", js_str(ai_path), js_str(logo_svg)));
    js.push_str(&format!("var LABEL={},LABEL_BOLD={};\n", js_str(&spec.label_postscript), js_str(&spec.label_bold_postscript)));
    js.push_str("var FONTS=[");
    for (i, r) in plan.rows.iter().enumerate() {
        if i > 0 {
            js.push(',');
        }
        js.push_str(&format!("{{ps:{},family:{},style:{}}}", js_str(&r.postscript), js_str(&r.family), js_str(&r.style)));
    }
    js.push_str("];\n");
    let star = star_points().iter().map(|(x, y)| format!("[{x:.2},{y:.2}]")).collect::<Vec<_>>().join(",");
    js.push_str(&format!("var STAR=[{star}];\n"));
    js.push_str("var PAGES=[\n");
    for ops in &plan.pages {
        js.push('[');
        for op in ops {
            match op {
                Op::Rect { x, y, w, h, rgb } => js.push_str(&format!("['r',{x},{y},{w},{h},{},{},{}],", rgb.0, rgb.1, rgb.2)),
                Op::Logo { x, y, size } => js.push_str(&format!("['l',{x},{y},{size}],")),
                Op::Star { x, y, size } => js.push_str(&format!("['s',{x},{y},{size}],")),
                Op::Text { face, size, x, baseline, text, rgb } => {
                    let f = match face {
                        Face::Label => -1,
                        Face::LabelBold => -2,
                        Face::Row(i) => *i as i64,
                    };
                    js.push_str(&format!("['t',{f},{size},{x},{baseline},{},{},{},{}],", js_str(text), rgb.0, rgb.1, rgb.2));
                }
            }
        }
        js.push_str("],\n");
    }
    js.push_str("];\n");
    js.push_str(ILLUSTRATOR_RUNTIME);
    Ok((js, plan))
}

const ILLUSTRATOR_RUNTIME: &str = r#"
function rgb(r, g, b) { var c = new RGBColor(); c.red = r; c.green = g; c.blue = b; return c; }
var cache = {}, missing = [];
function fontByName(ps) {
  if (!ps) return null;
  if (cache.hasOwnProperty(ps)) return cache[ps];
  var f = null;
  try { f = app.textFonts.getByName(ps); } catch (e) { f = null; }
  cache[ps] = f;
  return f;
}
function rowFont(i) {
  var d = FONTS[i], f = fontByName(d.ps);
  if (!f && d.family) {
    for (var k = 0; k < app.textFonts.length; k++) {
      var t = app.textFonts[k];
      if (t.family == d.family && t.style == d.style) { f = t; break; }
    }
  }
  if (!f && !d.reported) { missing.push(d.family ? d.family + " " + d.style : d.ps); d.reported = true; }
  return f;
}
function labelFont(bold) {
  return fontByName(bold ? LABEL_BOLD : LABEL) || fontByName(LABEL) || fontByName("ArialMT");
}
function main() {
  app.userInteractionLevel = UserInteractionLevel.DONTDISPLAYALERTS;
  var doc = app.documents.add(DocumentColorSpace.RGB, W, H);
  app.coordinateSystem = CoordinateSystem.DOCUMENTCOORDINATESYSTEM;
  var base = doc.artboards[0].artboardRect; // [left, top, right, bottom], y up
  var L0 = base[0], T0 = base[1];
  // Artboards in rows of 10, so long collections stay inside Illustrator's canvas.
  var origin = function (p) { return [L0 + (p % 10) * (W + GAP), T0 - Math.floor(p / 10) * (H + GAP)]; };
  for (var p = 1; p < PAGES.length; p++) {
    var o0 = origin(p);
    doc.artboards.add([o0[0], o0[1], o0[0] + W, o0[1] - H]);
  }
  for (var p = 0; p < PAGES.length; p++) {
    doc.artboards[p].name = "Page " + (p + 1);
    var org = origin(p), left = org[0], top = org[1];
    var X = function (v) { return left + v * K; };
    var Y = function (v) { return top - v * K; };
    var layer = doc.layers[0];
    var ops = PAGES[p];
    for (var i = 0; i < ops.length; i++) {
      var o = ops[i];
      if (o[0] == 'r') {
        var r = layer.pathItems.rectangle(Y(o[2]), X(o[1]), o[3] * K, o[4] * K);
        r.stroked = false; r.filled = true; r.fillColor = rgb(o[5], o[6], o[7]);
      } else if (o[0] == 'l') {
        var sq = layer.pathItems.rectangle(Y(o[2]), X(o[1]), o[3] * K, o[3] * K);
        sq.stroked = false; sq.fillColor = rgb(255, 191, 0);
        try {
          var g = layer.groupItems.createFromFile(new File(LOGO));
          g.width = o[3] * K; g.height = o[3] * K; g.left = X(o[1]); g.top = Y(o[2]);
          sq.remove();
        } catch (e) {}
      } else if (o[0] == 's') {
        var s = o[3] / 24 * K, pts = [];
        for (var k = 0; k < STAR.length; k++) pts.push([X(o[1]) + STAR[k][0] * s, Y(o[2]) - STAR[k][1] * s]);
        var st = layer.pathItems.add();
        st.setEntirePath(pts); st.closed = true; st.stroked = false; st.filled = true; st.fillColor = rgb(255, 191, 0);
      } else if (o[0] == 't') {
        var tf = layer.textFrames.pointText([X(o[3]), Y(o[4])]);
        tf.contents = o[5];
        var a = tf.textRange.characterAttributes;
        var f = o[1] >= 0 ? rowFont(o[1]) : labelFont(o[1] == -2);
        if (f) a.textFont = f;
        a.size = o[2] * K;
        a.fillColor = rgb(o[6], o[7], o[8]);
        if (o[1] >= 0) tf.name = FONTS[o[1]].ps;
      }
    }
  }
  doc.artboards.setActiveArtboardIndex(0);
  var opts = new IllustratorSaveOptions();
  opts.pdfCompatible = true;
  doc.saveAs(new File(AI_PATH), opts);
  return "OK" + (missing.length ? "\nMISSING:" + missing.join("|") : "");
}
var __result;
try { __result = main(); } catch (e) { __result = "ERROR:" + e; }
__result;
"#;

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
                postscript: format!("Font{i}"),
                family: "F".into(),
                style: "Regular".into(),
            });
        }
        rows.push(ArtworkRow { path: "/nope.ttf".into(), index: 0, name: "Missing".into(), favourite: false, no_embedding: false, postscript: "Nope".into(), family: "Nope".into(), style: "Regular".into() });
        // A duplicate of the first font is listed once.
        let mut dup = rows[0].clone();
        dup.name = "Duplicate".into();
        rows.push(dup);
        let spec = ArtworkSpec {
            title: "Brand \"A\" é".into(),
            sample: "I believe I can fly and touch the sky".into(),
            rows,
            label_font: label.clone(),
            label_bold: label,
            label_postscript: "SegoeUI".into(),
            label_bold_postscript: "SegoeUI-Semibold".into(),
        };
        let r = render_pdf(&spec).unwrap();
        assert!(r.pdf.starts_with(b"%PDF"));
        assert!(r.pages >= 2);
        assert_eq!(r.failed, vec!["Missing".to_string()]);
        let (js, plan) = illustrator_script(&spec, "C:\\out\\a.ai", "C:\\tmp\\logo.svg").unwrap();
        assert_eq!(plan.rows.len(), 41);
        assert!(js.is_ascii());
        assert!(js.contains("Brand \\\"A\\\" \\u00e9"));
        assert!(js.contains("\"C:\\\\out\\\\a.ai\""));
        if let Ok(dir) = std::env::var("BFM_ARTWORK_OUT") {
            std::fs::write(std::path::Path::new(&dir).join("artwork-test.pdf"), &r.pdf).unwrap();
        }
    }
}
