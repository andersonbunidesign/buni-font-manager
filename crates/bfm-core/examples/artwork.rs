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
            let info = bfm_core::reader::read_font_at(&d, 0).ok();
            let name = info.as_ref().map(|f| f.full_name.clone()).unwrap_or_else(|| p.clone());
            bfm_core::artwork::ArtworkRow {
                path: p.clone(),
                index: 0,
                name,
                favourite: i < 2,
                no_embedding: false,
                postscript: info.as_ref().map(|f| f.postscript_name.clone()).unwrap_or_default(),
                family: info.as_ref().map(|f| f.family.clone()).unwrap_or_default(),
                style: info.as_ref().map(|f| f.style.clone()).unwrap_or_default(),
            }
        })
        .collect();
    let spec = bfm_core::artwork::ArtworkSpec { title: "Brand A".into(), sample: a[2].clone(), rows, label_font: label.clone(), label_bold: label, label_postscript: "SegoeUI".into(), label_bold_postscript: "SegoeUI-Semibold".into() };
    let r = bfm_core::artwork::render_pdf(&spec).unwrap();
    std::fs::write(&a[0], r.pdf).unwrap();
    let (js, _) = bfm_core::artwork::illustrator_script(&spec, "C:/out/a.ai", "C:/tmp/logo.svg").unwrap();
    std::fs::write(format!("{}.jsx", a[0]), js).unwrap();
    println!("{} pages", r.pages);
}
