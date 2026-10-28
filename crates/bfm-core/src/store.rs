//! Loading and saving the library file (library.json).

use crate::model::Library;
use std::path::Path;

pub fn load(path: &Path) -> std::io::Result<Library> {
    match std::fs::read(path) {
        Ok(bytes) => serde_json::from_slice(&bytes).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Library { version: 1, ..Default::default() }),
        Err(e) => Err(e),
    }
}

/// Writes atomically: a temporary file first, then a rename, so a crash never leaves half a file.
pub fn save(path: &Path, lib: &Library) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension("json.tmp");
    let bytes = serde_json::to_vec_pretty(lib).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    std::fs::write(&tmp, bytes)?;
    std::fs::rename(&tmp, path)
}
