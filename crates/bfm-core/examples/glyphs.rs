//! Developer helper: prints a font's glyph map as JSON.
fn main() {
    let p = std::env::args().nth(1).unwrap();
    let d = std::fs::read(p).unwrap();
    println!("{}", serde_json::to_string(&bfm_core::reader::glyph_map(&d, 0, 4000).unwrap()).unwrap());
}
