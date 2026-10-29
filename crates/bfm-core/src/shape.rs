//! Looks at the actual letter shapes, because most fonts (Envato ones especially)
//! leave the style fields in the file empty.
//!
//! Signals, measured on a few letters:
//! - serifs: how much wider the bottom of `n`, `I` and `l` is than their stems;
//! - slant and overhang of `l`/`n`: scripts lean and their strokes reach past the letter box;
//! - width: average capital width relative to cap height (condensed / expanded).

use ttf_parser::{Face, OutlineBuilder};

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Shape {
    /// Foot width ÷ stem width for `n`, `I`, `l` (1.0 = no serifs).
    pub n_ratio: Option<f32>,
    pub i_ratio: Option<f32>,
    pub l_ratio: Option<f32>,
    /// Horizontal drift of `l` per unit of height (0 = upright).
    pub slant: f32,
    /// How far `n` reaches outside its advance width, as a share of it.
    pub overhang: f32,
    /// Lowercase letters are drawn as capitals (e.g. Bebas Neue).
    pub caps_only: bool,
    /// Average capital advance ÷ cap height.
    pub upper_width: Option<f32>,
    /// Narrow and wide letters have the same advance.
    pub fixed_pitch: bool,
}

#[derive(Default)]
struct Flat {
    contours: Vec<Vec<(f32, f32)>>,
    cur: (f32, f32),
}

impl Flat {
    fn push(&mut self, p: (f32, f32)) {
        if let Some(c) = self.contours.last_mut() {
            c.push(p);
        }
        self.cur = p;
    }
}

impl OutlineBuilder for Flat {
    fn move_to(&mut self, x: f32, y: f32) {
        self.contours.push(vec![(x, y)]);
        self.cur = (x, y);
    }
    fn line_to(&mut self, x: f32, y: f32) {
        self.push((x, y));
    }
    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        let (x0, y0) = self.cur;
        for i in 1..=8 {
            let t = i as f32 / 8.0;
            let (a, b, c) = ((1.0 - t) * (1.0 - t), 2.0 * (1.0 - t) * t, t * t);
            self.push((a * x0 + b * x1 + c * x, a * y0 + b * y1 + c * y));
        }
    }
    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        let (x0, y0) = self.cur;
        for i in 1..=8 {
            let t = i as f32 / 8.0;
            let u = 1.0 - t;
            let px = u * u * u * x0 + 3.0 * u * u * t * x1 + 3.0 * u * t * t * x2 + t * t * t * x;
            let py = u * u * u * y0 + 3.0 * u * u * t * y1 + 3.0 * u * t * t * y2 + t * t * t * y;
            self.push((px, py));
        }
    }
    fn close(&mut self) {}
}

/// Sorted x positions where the outline crosses the horizontal line `y`.
fn crossings(f: &Flat, y: f32) -> Vec<f32> {
    let mut xs = Vec::new();
    for c in &f.contours {
        let n = c.len();
        for i in 0..n {
            let (x0, y0) = c[i];
            let (x1, y1) = c[(i + 1) % n];
            if (y0 <= y && y1 > y) || (y1 <= y && y0 > y) {
                xs.push(x0 + (y - y0) / (y1 - y0) * (x1 - x0));
            }
        }
    }
    xs.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    xs
}

/// Ink width along the horizontal line `y` (even-odd fill).
fn ink_at(f: &Flat, y: f32) -> f32 {
    crossings(f, y).chunks(2).filter(|p| p.len() == 2).map(|p| p[1] - p[0]).sum()
}

struct Glyph {
    flat: Flat,
    y_min: f32,
    y_max: f32,
    x_min: f32,
    x_max: f32,
    advance: f32,
}

fn glyph(face: &Face, ch: char) -> Option<Glyph> {
    let gid = face.glyph_index(ch)?;
    let mut flat = Flat::default();
    let bb = face.outline_glyph(gid, &mut flat)?;
    let advance = face.glyph_hor_advance(gid)? as f32;
    if bb.y_max <= bb.y_min {
        return None;
    }
    Some(Glyph { flat, y_min: bb.y_min as f32, y_max: bb.y_max as f32, x_min: bb.x_min as f32, x_max: bb.x_max as f32, advance })
}

fn serif_ratio(g: &Glyph) -> Option<f32> {
    let h = g.y_max - g.y_min;
    let stem = ink_at(&g.flat, g.y_min + 0.5 * h);
    if stem <= 0.0 {
        return None;
    }
    let foot = [0.01f32, 0.02, 0.035, 0.05].iter().map(|k| ink_at(&g.flat, g.y_min + k * h)).fold(0.0f32, f32::max);
    Some(foot / stem)
}

fn centre_at(g: &Glyph, y: f32) -> Option<f32> {
    let xs = crossings(&g.flat, y);
    Some((xs.first()? + xs.last()?) / 2.0)
}

pub fn analyse(face: &Face) -> Shape {
    let mut s = Shape::default();
    let cap = face.capital_height().map(|c| c as f32).filter(|c| *c > 0.0).or_else(|| glyph(face, 'H').map(|g| g.y_max));

    let n = glyph(face, 'n');
    let i = glyph(face, 'I');
    let l = glyph(face, 'l');
    s.n_ratio = n.as_ref().and_then(serif_ratio);
    s.i_ratio = i.as_ref().and_then(serif_ratio);
    s.l_ratio = l.as_ref().and_then(serif_ratio);

    if let (Some(n), Some(cap)) = (&n, cap) {
        s.caps_only = n.y_max - n.y_min >= 0.9 * cap;
        s.overhang = ((-n.x_min).max(0.0) + (n.x_max - n.advance).max(0.0)) / n.advance.max(1.0);
    }
    if let Some(l) = &l {
        let h = l.y_max - l.y_min;
        if let (Some(a), Some(b)) = (centre_at(l, l.y_min + 0.25 * h), centre_at(l, l.y_min + 0.75 * h)) {
            s.slant = (b - a) / (0.5 * h);
        }
    }
    if let Some(cap) = cap {
        let adv: Vec<f32> = "NOEHSR".chars().filter_map(|c| face.glyph_index(c).and_then(|g| face.glyph_hor_advance(g))).map(|a| a as f32).collect();
        if !adv.is_empty() {
            s.upper_width = Some(adv.iter().sum::<f32>() / adv.len() as f32 / cap);
        }
    }
    let adv = |c: char| face.glyph_index(c).and_then(|g| face.glyph_hor_advance(g));
    if let (Some(a), Some(b)) = (adv('i'), adv('m')) {
        s.fixed_pitch = a == b && a > 0;
    }
    s
}

/// Style from the letter shapes. `None` when the shapes don't settle it.
/// Scripts and handwriting are one style ("Script").
pub fn classify_shape(s: &Shape, italic: bool) -> Option<&'static str> {
    // Scripts and handwriting lean even in their "regular" style.
    if !italic && s.slant >= 0.12 {
        return Some("Script");
    }
    // For all-caps fonts the lowercase is a capital, so judge by the I.
    let main = if s.caps_only { s.i_ratio } else { s.n_ratio };
    let main = main?;
    let min_il = s.i_ratio.unwrap_or(1.0).min(s.l_ratio.unwrap_or(1.0));
    // Broken, pointed feet: blackletter (rounded sans terminals stay above 0.8).
    if !italic && !s.caps_only && main < 0.8 && min_il < 0.8 {
        return Some("Blackletter");
    }
    // A capital I that tapers to a point at the foot: display lettering.
    if !italic && !s.caps_only && s.i_ratio.map(|r| r < 0.7).unwrap_or(false) {
        return Some("Decorative");
    }
    if main >= 1.3 && (s.l_ratio.unwrap_or(main) >= 1.2 || s.i_ratio.unwrap_or(main) >= 1.2) {
        return Some("Serif");
    }
    if (0.75..=1.15).contains(&main) {
        return Some("Sans serif");
    }
    if !italic && main < 0.75 {
        return Some("Decorative");
    }
    None
}

/// Spacing from the letter shapes. `None` when it looks normal.
pub fn spacing_shape(s: &Shape) -> Option<&'static str> {
    if s.fixed_pitch {
        return Some("Monospace");
    }
    match s.upper_width {
        Some(w) if w < 0.72 => Some("Condensed"),
        Some(w) if w > 1.25 => Some("Expanded"),
        _ => None,
    }
}
