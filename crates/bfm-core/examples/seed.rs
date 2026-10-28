//! Developer helper: imports the given files into a library folder.
//! Usage: cargo run -p bfm-core --example seed -- <root dir> <origin> <collection> <files...>
use bfm_core::{import::import_paths, store};
use std::path::PathBuf;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let root = PathBuf::from(&args[0]);
    let mut lib = store::load(&root.join("library.json")).unwrap();
    let inputs: Vec<PathBuf> = args[3..].iter().map(PathBuf::from).collect();
    let report = import_paths(&mut lib, &inputs, &root.join("Library"), &args[1], 1);
    lib.add_to_collection(&args[2], &report.font_ids);
    store::save(&root.join("library.json"), &lib).unwrap();
    println!("added {} skipped {} errors {:?}", report.added.len(), report.skipped.len(), report.errors);
}
