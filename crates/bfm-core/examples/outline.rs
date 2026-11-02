//! Developer helper: prints one glyph's outline as JSON.
fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let d = std::fs::read(&a[0]).unwrap();
    let cp = a[1].chars().next().unwrap() as u32;
    println!("{}", serde_json::to_string(&bfm_core::reader::glyph_outline(&d, 0, Some(cp), None).unwrap()).unwrap());
}
