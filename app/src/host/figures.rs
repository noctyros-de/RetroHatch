// SPDX-License-Identifier: GPL-3.0-only
//! Private content-addressed accessory storage. Source files are never changed.
use super::storage;
use crate::core::{
    state::{hex, require, sha256},
    tamago::figure::Figure,
};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};
pub const DIRECTORY: &str = "tamago-figures";
pub fn import(root: &Path, source: &Path) -> Result<PathBuf, String> {
    let b = storage::read_bounded(source, 4194312)?.ok_or("Figure file is missing")?;
    let figure = Figure::decode(&b)?;
    let path = root
        .join(DIRECTORY)
        .join(format!("{}.bin", figure.identity()));
    if let Some(existing) = storage::read_bounded(&path, 4194312)? {
        if existing != b {
            // Explicitly reimporting the validated original repairs only this corrupt
            // content-addressed internal copy. Source files and saves stay untouched.
            storage::atomic_write(&path, &b)?;
        }
    } else {
        storage::atomic_write(&path, &b)?;
    }
    Ok(path)
}
pub fn load(path: &Path) -> Result<Arc<Figure>, String> {
    let b = storage::read_bounded(path, 4194312)?
        .ok_or("The internal figure is missing; import the original again")?;
    require(
        path.file_stem().and_then(|v| v.to_str()) == Some(&hex(&sha256(&b))),
        "The internal figure is corrupted; reimport the original",
    )?;
    Ok(Arc::new(Figure::decode(&b)?))
}
pub fn list(root: &Path) -> Result<Vec<(PathBuf, String)>, String> {
    let dir = root.join(DIRECTORY);
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    for item in std::fs::read_dir(&dir).map_err(|e| e.to_string())? {
        let p = item.map_err(|e| e.to_string())?.path();
        if p.extension().is_some_and(|s| s == "bin") {
            let f = load(&p)?;
            out.push((p, f.name.clone()));
        }
    }
    out.sort_by(|a, b| a.1.cmp(&b.1));
    Ok(out)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn import_preserves_source_and_reopens_deduplicated_library() {
        let root = std::env::temp_dir().join(format!("retrohatch-figure-{}", std::process::id()));
        let source = root.join("source.bin");
        std::fs::create_dir_all(&root).unwrap();
        let mut b = vec![0; 524288];
        b[1..3].copy_from_slice(&[170, 85]);
        b[17..21].copy_from_slice(&[170, 85, 3, 14]);
        std::fs::write(&source, &b).unwrap();
        let p = import(&root, &source).unwrap();
        assert_eq!(p, import(&root, &source).unwrap());
        assert_eq!(list(&root).unwrap().len(), 1);
        assert_eq!(load(&p).unwrap().name, "Makiko");
        assert_eq!(std::fs::read(&source).unwrap(), b);
        std::fs::write(&p, b"corrupt").unwrap();
        assert!(load(&p).is_err());
        assert_eq!(import(&root, &source).unwrap(), p);
        assert_eq!(load(&p).unwrap().name, "Makiko");
        assert_eq!(std::fs::read(&source).unwrap(), b);
        std::fs::remove_file(&p).unwrap();
        assert!(load(&p).is_err());
        std::fs::remove_dir_all(root).unwrap();
    }
}
