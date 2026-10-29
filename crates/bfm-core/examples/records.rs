//! Developer helper: prints font records (JSON) for the files in a folder.
fn main() {
    let dir = std::env::args().nth(1).unwrap();
    let src = bfm_core::external::ScanSource { dir: dir.into(), origin: "System".into(), recursive: false, any_extension: true };
    let mut cache = Default::default();
    let list = bfm_core::external::scan(&[src], &mut cache, bfm_core::DETECTION_VERSION);
    println!("{}", serde_json::to_string(&list).unwrap());
}
