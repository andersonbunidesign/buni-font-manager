//! Importing zips, folders and font files into the library.

use crate::model::{FontRecord, ImportError, ImportReport, Library, SkippedFont, DETECTION_VERSION};
use crate::reader::{is_font_file_name, read_font};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};

/// A group of files that arrived together (one zip, one folder, or one loose file).
struct Batch {
    name: String,
    fonts: Vec<(String, Vec<u8>)>,
    licences: Vec<(String, Vec<u8>)>,
    /// True for zips and folders: Duo detection and licence linking apply.
    container: bool,
}

pub fn is_licence_file_name(name: &str) -> bool {
    let n = name.to_ascii_lowercase();
    let file = n.rsplit(['/', '\\']).next().unwrap_or(&n);
    let doc = [".pdf", ".txt", ".md", ".rtf", ".html", ".htm", ".doc", ".docx"].iter().any(|e| file.ends_with(e));
    doc && ["licen", "eula", "terms", "readme"].iter().any(|w| file.contains(w))
}

fn base_name(path: &str) -> String {
    path.rsplit(['/', '\\']).next().unwrap_or(path).to_string()
}

fn skip_entry(name: &str) -> bool {
    let b = base_name(name);
    name.contains("__MACOSX") || b.starts_with("._") || b.starts_with('.')
}

/// Makes a string safe as a single Windows/macOS folder or file name.
pub fn safe_name(s: &str) -> String {
    let cleaned: String = s
        .chars()
        .map(|c| if "<>:\"/\\|?*".contains(c) || c.is_control() { '_' } else { c })
        .collect();
    let trimmed = cleaned.trim().trim_matches('.').trim();
    let mut out: String = trimmed.chars().take(80).collect();
    if out.is_empty() {
        out = "Untitled".into();
    }
    out
}

pub fn font_id(data: &[u8]) -> String {
    let digest = Sha256::digest(data);
    digest.iter().take(8).map(|b| format!("{b:02x}")).collect()
}

fn read_zip(name: &str, data: Vec<u8>, batch: &mut Batch, errors: &mut Vec<ImportError>, depth: u8) {
    let mut archive = match zip::ZipArchive::new(Cursor::new(data)) {
        Ok(a) => a,
        Err(_) => {
            errors.push(ImportError { file: name.to_string(), message: "The zip is damaged and could not be opened".into() });
            return;
        }
    };
    for i in 0..archive.len() {
        let mut entry = match archive.by_index(i) {
            Ok(e) => e,
            Err(_) => {
                errors.push(ImportError { file: name.to_string(), message: "Part of the zip could not be read".into() });
                continue;
            }
        };
        if entry.is_dir() {
            continue;
        }
        let entry_name = entry.name().to_string();
        if skip_entry(&entry_name) {
            continue;
        }
        let lower = entry_name.to_ascii_lowercase();
        let wanted = is_font_file_name(&lower) || is_licence_file_name(&lower) || (lower.ends_with(".zip") && depth < 3);
        if !wanted {
            continue;
        }
        let mut bytes = Vec::with_capacity(entry.size() as usize);
        if entry.read_to_end(&mut bytes).is_err() {
            errors.push(ImportError { file: format!("{name} › {}", base_name(&entry_name)), message: "Could not be read from the zip".into() });
            continue;
        }
        if is_font_file_name(&lower) {
            batch.fonts.push((base_name(&entry_name), bytes));
        } else if is_licence_file_name(&lower) {
            batch.licences.push((base_name(&entry_name), bytes));
        } else {
            read_zip(&format!("{name} › {}", base_name(&entry_name)), bytes, batch, errors, depth + 1);
        }
    }
}

fn read_dir_into(dir: &Path, batch: &mut Batch, errors: &mut Vec<ImportError>) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => {
            errors.push(ImportError { file: dir.display().to_string(), message: "The folder could not be opened".into() });
            return;
        }
    };
    for entry in entries.flatten() {
        let p = entry.path();
        let name = base_name(&p.to_string_lossy());
        if skip_entry(&name) {
            continue;
        }
        if p.is_dir() {
            read_dir_into(&p, batch, errors);
            continue;
        }
        let lower = name.to_ascii_lowercase();
        if !(is_font_file_name(&lower) || is_licence_file_name(&lower) || lower.ends_with(".zip")) {
            continue;
        }
        match std::fs::read(&p) {
            Ok(bytes) => {
                if is_font_file_name(&lower) {
                    batch.fonts.push((name, bytes));
                } else if is_licence_file_name(&lower) {
                    batch.licences.push((name, bytes));
                } else {
                    read_zip(&name, bytes, batch, errors, 1);
                }
            }
            Err(_) => errors.push(ImportError { file: name, message: "Could not be read".into() }),
        }
    }
}

fn collect_batches(inputs: &[PathBuf], errors: &mut Vec<ImportError>) -> Vec<Batch> {
    let mut batches = Vec::new();
    for input in inputs {
        let name = base_name(&input.to_string_lossy());
        if input.is_dir() {
            let mut b = Batch { name: name.clone(), fonts: vec![], licences: vec![], container: true };
            read_dir_into(input, &mut b, errors);
            batches.push(b);
            continue;
        }
        let lower = name.to_ascii_lowercase();
        if lower.ends_with(".zip") {
            match std::fs::read(input) {
                Ok(bytes) => {
                    let mut b = Batch { name: name.clone(), fonts: vec![], licences: vec![], container: true };
                    read_zip(&name, bytes, &mut b, errors, 0);
                    batches.push(b);
                }
                Err(_) => errors.push(ImportError { file: name, message: "Could not be read".into() }),
            }
        } else if is_font_file_name(&lower) {
            match std::fs::read(input) {
                Ok(bytes) => batches.push(Batch { name: name.clone(), fonts: vec![(name, bytes)], licences: vec![], container: false }),
                Err(_) => errors.push(ImportError { file: name, message: "Could not be read".into() }),
            }
        } else if lower.ends_with(".ttc") || lower.ends_with(".otc") {
            errors.push(ImportError { file: name, message: "Font collection files (.ttc) are not supported yet".into() });
        } else if lower.ends_with(".woff") || lower.ends_with(".woff2") {
            errors.push(ImportError { file: name, message: "Web fonts (.woff) can't be installed on Windows; use the OTF or TTF version".into() });
        } else {
            errors.push(ImportError { file: name, message: "Not a font, zip or folder".into() });
        }
    }
    batches
}

/// Writes `data` to `dir/file_name`, adding " (2)" etc. if a different file already has that name.
fn write_unique(dir: &Path, file_name: &str, data: &[u8]) -> std::io::Result<PathBuf> {
    std::fs::create_dir_all(dir)?;
    let file_name = safe_name(file_name);
    let (stem, ext) = match file_name.rfind('.') {
        Some(i) if i > 0 => (file_name[..i].to_string(), file_name[i..].to_string()),
        _ => (file_name.clone(), String::new()),
    };
    let mut n = 1;
    loop {
        let candidate = if n == 1 { dir.join(&file_name) } else { dir.join(format!("{stem} ({n}){ext}")) };
        if !candidate.exists() {
            std::fs::write(&candidate, data)?;
            return Ok(candidate);
        }
        if std::fs::read(&candidate).map(|b| b == data).unwrap_or(false) {
            return Ok(candidate);
        }
        n += 1;
    }
}

/// Imports files/folders/zips into `library_dir` and records them in `lib`.
/// Fonts already in the library are skipped (but still listed in `font_ids`
/// so they can be added to the import's collection).
pub fn import_paths(lib: &mut Library, inputs: &[PathBuf], library_dir: &Path, origin: &str, now: u64) -> ImportReport {
    let mut report = ImportReport::default();
    let origin = if origin.trim().is_empty() { "Other" } else { origin.trim() };
    let batches = collect_batches(inputs, &mut report.errors);

    for batch in batches {
        let mut parsed = Vec::new();
        for (file, data) in batch.fonts {
            match read_font(&data) {
                Ok(info) => parsed.push((file, data, info)),
                Err(e) => report.errors.push(ImportError { file: format!("{} › {file}", batch.name).trim_start_matches(" › ").to_string(), message: e.to_string() }),
            }
        }
        let families: BTreeSet<String> = parsed.iter().map(|(_, _, i)| i.family.clone()).collect();
        let duo = batch.container && families.len() == 2;

        let mut added_here: Vec<usize> = Vec::new();
        for (file, data, info) in parsed {
            let id = font_id(&data);
            let existing = lib
                .fonts
                .iter()
                .find(|f| f.id == id || (f.postscript_name == info.postscript_name && f.family == info.family && f.style == info.style))
                .map(|f| f.id.clone());
            if let Some(existing_id) = existing {
                report.skipped.push(SkippedFont { name: info.full_name.clone(), existing_id: existing_id.clone() });
                if !report.font_ids.contains(&existing_id) {
                    report.font_ids.push(existing_id);
                }
                continue;
            }
            let dir = library_dir.join(safe_name(origin)).join(safe_name(&info.family));
            let path = match write_unique(&dir, &file, &data) {
                Ok(p) => p,
                Err(e) => {
                    report.errors.push(ImportError { file, message: format!("Could not be saved to the library: {e}") });
                    continue;
                }
            };
            if duo {
                report.duo_tagged += 1;
            }
            lib.fonts.push(FontRecord {
                id: id.clone(),
                family: info.family,
                style: info.style,
                full_name: info.full_name,
                postscript_name: info.postscript_name,
                weight: info.weight,
                width: info.width,
                italic: info.italic,
                monospace: info.monospace,
                variable: info.variable,
                axes: info.axes,
                classification: info.classification,
                spacing: info.spacing,
                langs: info.langs,
                path: path.to_string_lossy().to_string(),
                origin: origin.to_string(),
                added_at: now,
                licence: None,
                duo,
                no_embedding: info.no_embedding,
                active: true,
                ai_tags: vec![],
                hidden_ai_tags: vec![],
                custom_tags: vec![],
                external: false,
                index: 0,
            });
            added_here.push(lib.fonts.len() - 1);
            report.added.push(id.clone());
            if !report.font_ids.contains(&id) {
                report.font_ids.push(id);
            }
        }

        if batch.container && !added_here.is_empty() && !batch.licences.is_empty() {
            let dir = library_dir.join(safe_name(origin)).join("_Licences").join(safe_name(batch.name.trim_end_matches(".zip").trim_end_matches(".ZIP")));
            let mut first: Option<String> = None;
            for (name, data) in &batch.licences {
                if let Ok(p) = write_unique(&dir, name, data) {
                    report.licences_kept += 1;
                    let p = p.to_string_lossy().to_string();
                    // Prefer a file that is clearly the licence over a readme.
                    let is_licence = name.to_ascii_lowercase().contains("licen") || name.to_ascii_lowercase().contains("eula");
                    if first.is_none() || is_licence && !first.as_deref().unwrap_or("").to_ascii_lowercase().contains("licen") {
                        first = Some(p);
                    }
                }
            }
            for i in added_here {
                lib.fonts[i].licence = first.clone();
            }
        }
    }

    if !report.added.is_empty() {
        lib.remember_origin(origin);
    }
    lib.detection = DETECTION_VERSION.max(lib.detection);
    report
}

/// Re-reads every font file with the current style detection, if the library was
/// filled in by an older version. Tags the user added are kept. Returns true if anything ran.
pub fn refresh_detection(lib: &mut Library) -> bool {
    if lib.detection >= DETECTION_VERSION {
        return false;
    }
    for f in lib.fonts.iter_mut() {
        if let Ok(info) = std::fs::read(&f.path).map_err(|_| ()).and_then(|d| read_font(&d).map_err(|_| ())) {
            f.classification = info.classification;
            f.spacing = info.spacing;
            f.langs = info.langs;
            f.no_embedding = info.no_embedding;
            f.monospace = info.monospace;
        }
    }
    lib.detection = DETECTION_VERSION;
    true
}

/// Moves the files of `ids` into `trash_dir` and removes them from the library and collections.
pub fn trash_fonts(lib: &mut Library, ids: &[String], trash_dir: &Path) -> std::io::Result<Vec<FontRecord>> {
    std::fs::create_dir_all(trash_dir)?;
    let mut removed = Vec::new();
    for id in ids {
        if let Some(pos) = lib.fonts.iter().position(|f| &f.id == id) {
            let rec = lib.fonts.remove(pos);
            let src = PathBuf::from(&rec.path);
            if src.exists() {
                let dst = trash_dir.join(format!("{}-{}", rec.id, base_name(&rec.path)));
                if std::fs::rename(&src, &dst).is_err() {
                    std::fs::copy(&src, &dst)?;
                    std::fs::remove_file(&src)?;
                }
            }
            removed.push(rec);
        }
    }
    for c in lib.collections.iter_mut() {
        c.font_ids.retain(|i| !ids.contains(i));
        c.favourites.retain(|i| !ids.contains(i));
    }
    Ok(removed)
}

/// Moves trashed files back to where they were.
pub fn restore_trashed(records: &[FontRecord], trash_dir: &Path) -> std::io::Result<()> {
    for rec in records {
        let src = trash_dir.join(format!("{}-{}", rec.id, base_name(&rec.path)));
        if src.exists() {
            let dst = PathBuf::from(&rec.path);
            if let Some(parent) = dst.parent() {
                std::fs::create_dir_all(parent)?;
            }
            if std::fs::rename(&src, &dst).is_err() {
                std::fs::copy(&src, &dst)?;
                std::fs::remove_file(&src)?;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn licence_names() {
        assert!(is_licence_file_name("Fonts/License.pdf"));
        assert!(is_licence_file_name("EULA.txt"));
        assert!(!is_licence_file_name("preview.jpg"));
        assert!(!is_licence_file_name("License.otf"));
    }

    #[test]
    fn safe_names() {
        assert_eq!(safe_name("Brand: A/B"), "Brand_ A_B");
        assert_eq!(safe_name("  ..  "), "Untitled");
    }
}
