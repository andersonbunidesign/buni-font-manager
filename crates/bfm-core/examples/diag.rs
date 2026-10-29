//! Developer helper: prints what the style detection sees in font files.
//! Usage: cargo run -p bfm-core --example diag -- <files...>
use ttf_parser::{Face, Tag};
fn main() {
    for p in std::env::args().skip(1) {
        let data = std::fs::read(&p).unwrap();
        let Ok(face) = Face::parse(&data, 0) else { println!("{p}: unreadable"); continue };
        let info = bfm_core::reader::read_font_at(&data, 0).unwrap();
        let os2 = face.raw_face().table(Tag::from_bytes(b"OS/2")).unwrap_or(&[]);
        let (class, pan) = if os2.len() >= 42 { (i16::from_be_bytes([os2[30], os2[31]]), os2[32..42].to_vec()) } else { (0, vec![]) };
        let s = bfm_core::shape::analyse(&face);
        println!(
            "{:22} {:14} w{:4} {:12} {:10} | class {:#06x} pan {:?} | n {:.2?} I {:.2?} l {:.2?} slant {:.2} over {:.2} caps {} uw {:.2?} | {}",
            info.family, info.style, info.weight, info.classification, info.spacing, class, pan, s.n_ratio, s.i_ratio, s.l_ratio, s.slant, s.overhang, s.caps_only, s.upper_width, p.rsplit('/').next().unwrap()
        );
    }
}
