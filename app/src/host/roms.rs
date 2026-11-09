//! User-owned local ROM discovery and bounded loading.
use crate::core::{p1::RomImage, profile::DeviceProfile};
use std::io::Read;
use std::path::{Path, PathBuf};

pub fn default_folder() -> PathBuf {
    if let Ok(cwd) = std::env::current_dir() {
        if cwd.join("app/Cargo.toml").is_file() {
            return cwd.join("roms");
        }
        if cwd.join("Cargo.toml").is_file()
            && cwd.file_name().is_some_and(|n| n == "app")
            && let Some(parent) = cwd.parent()
        {
            return parent.join("roms");
        }
    }
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.join("roms")))
        .unwrap_or_else(|| PathBuf::from("roms"))
}
pub fn list(folder: &Path) -> Result<Vec<PathBuf>, String> {
    let mut paths = Vec::new();
    for entry in std::fs::read_dir(folder).map_err(|e| format!("Cannot read ROM folder: {e}"))? {
        let path = entry.map_err(|e| e.to_string())?.path();
        if path.is_file()
            && path
                .extension()
                .is_some_and(|s| s.eq_ignore_ascii_case("bin") || s.eq_ignore_ascii_case("rom"))
        {
            paths.push(path);
        }
    }
    paths.sort();
    Ok(paths)
}
pub fn load(path: &Path) -> Result<RomImage, String> {
    load_for_profile(path, DeviceProfile::P1)
}
pub fn load_for_profile(path: &Path, profile: DeviceProfile) -> Result<RomImage, String> {
    let file =
        std::fs::File::open(path).map_err(|e| format!("Cannot open {}: {e}", path.display()))?;
    let mut data = Vec::new();
    file.take((profile.rom_bytes() + 1) as u64)
        .read_to_end(&mut data)
        .map_err(|e| format!("Cannot read {}: {e}", path.display()))?;
    super::pending_devices::ensure_supported(&crate::core::state::hex(
        &crate::core::state::sha256(&data),
    ))?;
    RomImage::decode_for_profile(&data, profile).map_err(|e| format!("{}: {e}", profile.name()))
}
