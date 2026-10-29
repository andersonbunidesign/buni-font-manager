//! Developer helper: renders an artwork PDF from font files.
//! Usage: cargo run -p bfm-core --example artwork -- <out.pdf> <label.ttf> <sample> <fonts...>
fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let label = std::fs::read(&a[1]).unwrap();
    let rows = a[3..]
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let d = std::fs::read(p).unwrap();
            let name = bfm_core::reader::read_font_at(&d, 0).map(|f| f.full_name).unwrap_or_else(|_| p.clone());
            bfm_core::artwork::ArtworkRow { path: p.clone(), index: 0, name, favourite: i < 2, no_embedding: false }
        })
        .collect();
    let spec = bfm_core::artwork::ArtworkSpec { title: "Brand A".into(), sample: a[2].clone(), rows, label_font: label.clone(), label_bold: label };
    let r = bfm_core::artwork::render_pdf(&spec).unwrap();
    std::fs::write(&a[0], r.pdf).unwrap();
    println!("{} pages", r.pages);
}
