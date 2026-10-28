//! Developer helper: prints the detected style and spacing of font files.
//! Usage: cargo run -p bfm-core --example classify -- <files...>
fn main() {
    for p in std::env::args().skip(1) {
        let data = std::fs::read(&p).unwrap();
        match bfm_core::reader::read_font(&data) {
            Ok(i) => println!("{:28} {:12} {:10} {}", i.family, i.classification, i.spacing, i.weight),
            Err(e) => println!("{p}: {e}"),
        }
    }
}
