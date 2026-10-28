//! Fonts the app doesn't own: the ones installed in Windows and the ones
//! Adobe Fonts syncs. They are listed (and can go into collections) but never
//! moved, deleted or deactivated by the app.
//!
//! Reading hundreds of system fonts takes a few seconds, so results are cached
//! by file size and modification time.

use crate::model::{FontRecord, NameConflict};
use crate::reader::{face_count, read_font_at};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

pub const ORIGIN_SYSTEM: &str = "System";
pub const ORIGIN_ADOBE: &str = "Adobe Fonts";

#[derive(Debug, Clone)]
pub struct ScanSource {
    pub dir: PathBuf,
    pub origin: String,
    /// Look inside sub-folders too (Adobe keeps fonts in several).
    pub recursive: bool,
    /// Try every file, not only .otf/.ttf/.ttc (Adobe's files have no extension).
    pub any_extension: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CacheEntry {
    pub size: u64,
    pub mtime: u64,
    pub records: Vec<FontRecord>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ScanCache {
    #[serde(default)]
    pub version: u32,
    #[serde(default)]
    pub entries: BTreeMap<String, CacheEntry>,
}

pub fn external_id(path: &str, index: u32) -> String {
    let d = Sha256::digest(format!("{path}|{index}").as_bytes());
    let hex: String = d.iter().take(7).map(|b| format!("{b:02x}")).collect();
    format!("x{hex}")
}

fn wanted(path: &Path, any_extension: bool) -> bool {
    let name = path.file_name().map(|n| n.to_string_lossy().to_ascii_lowercase()).unwrap_or_default();
    if name.starts_with('.') || name.ends_with(".xml") || name.ends_with(".json") || name.ends_with(".db") || name.ends_with(".lock") {
        return false;
    }
    if any_extension {
        return true;
    }
    name.ends_with(".otf") || name.ends_with(".ttf") || name.ends_with(".ttc") || name.ends_with(".otc")
}

fn files_in(dir: &Path, recursive: bool, any_extension: bool, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for e in entries.flatten() {
        let p = e.path();
        if p.is_dir() {
            if recursive {
                files_in(&p, recursive, any_extension, out);
            }
        } else if wanted(&p, any_extension) {
            out.push(p);
        }
    }
}

fn read_records(path: &Path, origin: &str, mtime: u64) -> Vec<FontRecord> {
    let Ok(data) = std::fs::read(path) else { return vec![] };
    // Anything too small to be a font is skipped quickly.
    if data.len() < 1024 {
        return vec![];
    }
    let path_str = path.to_string_lossy().to_string();
    let mut out = Vec::new();
    for index in 0..face_count(&data).min(64) {
        let Ok(info) = read_font_at(&data, index) else { continue };
        out.push(FontRecord {
            id: external_id(&path_str, index),
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
            path: path_str.clone(),
            origin: origin.to_string(),
            added_at: mtime,
            licence: None,
            duo: false,
            no_embedding: info.no_embedding,
            active: true,
            ai_tags: vec![],
            hidden_ai_tags: vec![],
            custom_tags: vec![],
            external: true,
            index,
        });
    }
    out
}

/// Lists the fonts in `sources`, reusing `cache` for files that haven't changed.
pub fn scan(sources: &[ScanSource], cache: &mut ScanCache, detection_version: u32) -> Vec<FontRecord> {
    if cache.version != detection_version {
        cache.entries.clear();
        cache.version = detection_version;
    }
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    for src in sources {
        let mut files = Vec::new();
        files_in(&src.dir, src.recursive, src.any_extension, &mut files);
        for f in files {
            let Ok(meta) = std::fs::metadata(&f) else { continue };
            let size = meta.len();
            let mtime = meta.modified().ok().and_then(|t| t.duration_since(UNIX_EPOCH).ok()).map(|d| d.as_secs()).unwrap_or(0);
            let key = f.to_string_lossy().to_string();
            seen.insert(key.clone());
            let hit = cache.entries.get(&key).filter(|e| e.size == size && e.mtime == mtime && e.records.first().map(|r| r.origin == src.origin).unwrap_or(true));
            let records = match hit {
                Some(e) => e.records.clone(),
                None => {
                    let r = read_records(&f, &src.origin, mtime);
                    cache.entries.insert(key, CacheEntry { size, mtime, records: r.clone() });
                    r
                }
            };
            out.extend(records);
        }
    }
    cache.entries.retain(|k, _| seen.contains(k));
    // The same font can be both installed and synced; keep the first one seen.
    let mut ids = BTreeSet::new();
    out.retain(|r| ids.insert(r.id.clone()));
    out
}

/// Imported families whose name matches a font already on the computer.
pub fn find_conflicts(imported: &[FontRecord], external: &[FontRecord]) -> Vec<NameConflict> {
    let mut ext: BTreeMap<String, String> = BTreeMap::new();
    for e in external {
        ext.entry(e.family.to_lowercase()).or_insert_with(|| e.origin.clone());
    }
    let mut by_family: BTreeMap<String, NameConflict> = BTreeMap::new();
    for f in imported {
        if let Some(origin) = ext.get(&f.family.to_lowercase()) {
            by_family
                .entry(f.family.clone())
                .or_insert_with(|| NameConflict { family: f.family.clone(), other_origin: origin.clone(), ids: vec![] })
                .ids
                .push(f.id.clone());
        }
    }
    by_family.into_values().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(name: &str) -> Vec<u8> {
        std::fs::read(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures").join(name)).unwrap()
    }

    #[test]
    fn scans_with_cache_and_finds_conflicts() {
        let tmp = tempfile::tempdir().unwrap();
        let sys = tmp.path().join("Fonts");
        let adobe = tmp.path().join("livetype").join("r");
        std::fs::create_dir_all(&sys).unwrap();
        std::fs::create_dir_all(&adobe).unwrap();
        std::fs::write(sys.join("SpaceMono-Regular.ttf"), fixture("SpaceMono-Regular.ttf")).unwrap();
        std::fs::write(sys.join("notes.txt"), b"hello").unwrap();
        // Adobe stores fonts without an extension.
        std::fs::write(adobe.join("12345"), fixture("PinyonScript-Regular.ttf")).unwrap();
        std::fs::write(adobe.join("entitlements.xml"), b"<x/>").unwrap();

        let sources = vec![
            ScanSource { dir: sys.clone(), origin: ORIGIN_SYSTEM.into(), recursive: false, any_extension: false },
            ScanSource { dir: tmp.path().join("livetype"), origin: ORIGIN_ADOBE.into(), recursive: true, any_extension: true },
        ];
        let mut cache = ScanCache::default();
        let found = scan(&sources, &mut cache, 2);
        assert_eq!(found.len(), 2);
        assert!(found.iter().all(|f| f.external));
        assert_eq!(found.iter().find(|f| f.family == "Pinyon Script").unwrap().origin, ORIGIN_ADOBE);
        assert_eq!(cache.entries.len(), 2); // the .txt and .xml files are not read
        let again = scan(&sources, &mut cache, 2);
        assert_eq!(again, found);

        let mut imported = found[0].clone();
        imported.external = false;
        imported.id = "abc".into();
        let c = find_conflicts(&[imported], &found);
        assert_eq!(c.len(), 1);
        assert_eq!(c[0].ids, vec!["abc".to_string()]);
    }
}
