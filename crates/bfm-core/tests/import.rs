use bfm_core::import::{import_paths, restore_trashed, trash_fonts};
use bfm_core::reader::{read_font, LANG_PORTUGUESE, LANG_SWEDISH};
use bfm_core::{store, Library};
use std::io::Write;
use std::path::{Path, PathBuf};

fn fixture(name: &str) -> Vec<u8> {
    std::fs::read(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures").join(name)).unwrap()
}

fn make_zip(path: &Path, files: &[(&str, Vec<u8>)]) {
    let f = std::fs::File::create(path).unwrap();
    let mut z = zip::ZipWriter::new(f);
    let opts: zip::write::SimpleFileOptions = zip::write::SimpleFileOptions::default();
    for (name, data) in files {
        z.start_file(*name, opts).unwrap();
        z.write_all(data).unwrap();
    }
    z.finish().unwrap();
}

fn nested_zip_bytes(files: &[(&str, Vec<u8>)]) -> Vec<u8> {
    let mut buf = std::io::Cursor::new(Vec::new());
    {
        let mut z = zip::ZipWriter::new(&mut buf);
        let opts: zip::write::SimpleFileOptions = zip::write::SimpleFileOptions::default();
        for (name, data) in files {
            z.start_file(*name, opts).unwrap();
            z.write_all(data).unwrap();
        }
        z.finish().unwrap();
    }
    buf.into_inner()
}

#[test]
fn reads_real_fonts() {
    let pinyon = read_font(&fixture("PinyonScript-Regular.ttf")).unwrap();
    assert_eq!(pinyon.family, "Pinyon Script");
    assert_eq!(pinyon.classification, "Script");
    assert_eq!(pinyon.weight, 400);

    let mono = read_font(&fixture("SpaceMono-Bold.ttf")).unwrap();
    assert_eq!(mono.family, "Space Mono");
    assert_eq!(mono.weight, 700);
    assert_eq!(mono.spacing, "Monospace");
    assert!(mono.langs.contains(&LANG_SWEDISH.to_string()));
    assert!(mono.langs.contains(&LANG_PORTUGUESE.to_string()));
}

#[test]
fn imports_zip_with_duo_licence_and_duplicates() {
    let tmp = tempfile::tempdir().unwrap();
    let lib_dir = tmp.path().join("Library");
    let zip_path = tmp.path().join("Envato-Duo-Pack.zip");
    let inner = nested_zip_bytes(&[("SpaceMono-Bold.ttf", fixture("SpaceMono-Bold.ttf"))]);
    make_zip(
        &zip_path,
        &[
            ("Duo Pack/PinyonScript-Regular.ttf", fixture("PinyonScript-Regular.ttf")),
            ("Duo Pack/SpaceMono-Regular.ttf", fixture("SpaceMono-Regular.ttf")),
            ("Duo Pack/License.txt", fixture("OFL.txt")),
            ("Duo Pack/preview.jpg", vec![1, 2, 3]),
            ("__MACOSX/Duo Pack/._PinyonScript-Regular.ttf", vec![0, 0]),
            ("Duo Pack/Extras.zip", inner),
        ],
    );

    let mut lib = Library::default();
    let report = import_paths(&mut lib, &[zip_path.clone()], &lib_dir, "Envato", 1000);
    assert_eq!(report.errors, vec![], "unexpected errors");
    assert_eq!(report.added.len(), 3);
    assert_eq!(report.duo_tagged, 3);
    assert_eq!(report.licences_kept, 1);
    assert!(lib.fonts.iter().all(|f| f.duo && f.licence.is_some() && f.origin == "Envato" && f.active));
    assert!(lib.fonts.iter().all(|f| Path::new(&f.path).exists()));
    assert_eq!(lib.origins, vec!["Envato".to_string()]);

    // Importing again skips everything but still returns the ids for the collection.
    let again = import_paths(&mut lib, &[zip_path], &lib_dir, "Envato", 2000);
    assert_eq!(again.added.len(), 0);
    assert_eq!(again.skipped.len(), 3);
    assert_eq!(again.font_ids.len(), 3);
    assert_eq!(lib.fonts.len(), 3);
}

#[test]
fn loose_files_errors_and_trash() {
    let tmp = tempfile::tempdir().unwrap();
    let lib_dir = tmp.path().join("Library");
    let font = tmp.path().join("PinyonScript-Regular.ttf");
    std::fs::write(&font, fixture("PinyonScript-Regular.ttf")).unwrap();
    let bad_zip = tmp.path().join("broken.zip");
    std::fs::write(&bad_zip, b"not a zip").unwrap();
    let bad_font = tmp.path().join("broken.otf");
    std::fs::write(&bad_font, b"nope").unwrap();

    let mut lib = Library::default();
    let inputs: Vec<PathBuf> = vec![font, bad_zip, bad_font];
    let report = import_paths(&mut lib, &inputs, &lib_dir, "Client", 5);
    assert_eq!(report.added.len(), 1);
    assert_eq!(report.errors.len(), 2);
    assert!(!lib.fonts[0].duo);

    let id = lib.fonts[0].id.clone();
    lib.add_to_collection("Brand A", &[id.clone()]);
    let path = PathBuf::from(&lib.fonts[0].path);
    let trash = tmp.path().join(".trash");
    let removed = trash_fonts(&mut lib, &[id.clone()], &trash).unwrap();
    assert!(lib.fonts.is_empty());
    assert!(lib.collections[0].font_ids.is_empty());
    assert!(!path.exists());
    restore_trashed(&removed, &trash).unwrap();
    assert!(path.exists());

    // Round trip through the library file.
    let file = tmp.path().join("library.json");
    lib.fonts = removed;
    store::save(&file, &lib).unwrap();
    assert_eq!(store::load(&file).unwrap(), lib);
}
