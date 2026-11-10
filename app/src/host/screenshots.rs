// SPDX-License-Identifier: GPL-3.0-only
// SPDX-FileCopyrightText: 2026 Noctyros
//! User-generated PNGs. Reuses the attributed TG18 encoder; never overwrites existing files.
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Lcd,
    Full,
}
impl Kind {
    pub fn folder(self) -> &'static str {
        match self {
            Self::Lcd => "LCD",
            Self::Full => "Emulator",
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Image {
    pub width: usize,
    pub height: usize,
    pub rgb: Vec<u8>,
}
impl Image {
    pub fn new(width: usize, height: usize, rgb: Vec<u8>) -> Result<Self, String> {
        if width == 0
            || height == 0
            || width.checked_mul(height).and_then(|v| v.checked_mul(3)) != Some(rgb.len())
            || rgb.len() > 64 * 1024 * 1024
        {
            return Err("Invalid or oversized screenshot image".into());
        }
        Ok(Self { width, height, rgb })
    }
}
pub fn media_root() -> Result<PathBuf, String> {
    if let Some(root) = std::env::var_os("RETROHATCH_CAPTURE_DIR") {
        if root.is_empty() {
            return Err("Capture directory override is empty".into());
        }
        return Ok(root.into());
    }
    Ok(super::platform::pictures_dir()?.join("RetroHatch"))
}
pub fn folder(root: &Path, kind: Kind) -> PathBuf {
    root.join("Screenshots").join(kind.folder())
}
pub fn prepare(root: &Path) -> Result<(), String> {
    for kind in [Kind::Lcd, Kind::Full] {
        fs::create_dir_all(root.join("Screenshots").join(kind.folder()))
            .map_err(|e| format!("Cannot create screenshot folder: {e}"))?;
    }
    for kind in [Kind::Lcd, Kind::Full] {
        fs::create_dir_all(root.join("Recordings").join(kind.folder()))
            .map_err(|e| format!("Cannot create recording folder: {e}"))?;
    }
    Ok(())
}
pub(crate) fn timestamp(time: SystemTime) -> Result<String, String> {
    let t = time.duration_since(UNIX_EPOCH).map_err(|e| e.to_string())?;
    let mut days = t.as_secs() / 86400;
    let mut year = 1970u64;
    let leap = |y: u64| y % 4 == 0 && (y % 100 != 0 || y % 400 == 0);
    while year < 9999 {
        let n = if leap(year) { 366 } else { 365 };
        if days < n {
            break;
        }
        days -= n;
        year += 1;
    }
    let lengths = [
        31,
        if leap(year) { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    let mut month = 0;
    while month < 11 && days >= lengths[month] {
        days -= lengths[month];
        month += 1;
    }
    let seconds = t.as_secs() % 86400;
    Ok(format!(
        "{year:04}-{:02}-{:02}_{:02}-{:02}-{:02}.{:03}Z",
        month + 1,
        days + 1,
        seconds / 3600,
        seconds / 60 % 60,
        seconds % 60,
        t.subsec_millis()
    ))
}
static SERIAL: AtomicU64 = AtomicU64::new(0);
pub fn save(root: &Path, model: &str, kind: Kind, image: Image) -> Result<PathBuf, String> {
    prepare(root)?;
    let image = Image::new(image.width, image.height, image.rgb)?;
    let model: String = model
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-')
        .take(80)
        .collect();
    if model.is_empty() {
        return Err("Invalid screenshot model name".into());
    }
    let stamp = timestamp(SystemTime::now())?;
    let bytes = tg18::png::encode(image.width, image.height, &image.rgb, 1);
    for _ in 0..100 {
        let n = SERIAL.fetch_add(1, Ordering::Relaxed);
        let path = root
            .join("Screenshots")
            .join(kind.folder())
            .join(format!("{model}_{}_{stamp}_{n:04}.png", kind.folder()));
        match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(mut file) => {
                if let Err(e) = file.write_all(&bytes).and_then(|_| file.sync_all()) {
                    drop(file);
                    let _ = fs::remove_file(&path);
                    return Err(format!("Cannot save screenshot: {e}"));
                }
                return Ok(path);
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(format!("Cannot save screenshot: {e}")),
        }
    }
    Err("Cannot choose a unique screenshot filename".into())
}
#[cfg(windows)]
#[path = "screenshot_windows.rs"]
pub mod native;
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn png_preserves_native_pixels_and_never_replaces_captures() {
        let root = std::env::temp_dir().join(format!(
            "retrohatch-shots-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let rgb = vec![255, 0, 0, 0, 255, 0, 0, 0, 255, 12, 34, 56];
        let first = save(
            &root,
            "tamagotchi-ps",
            Kind::Lcd,
            Image::new(2, 2, rgb.clone()).unwrap(),
        )
        .unwrap();
        let bytes = fs::read(&first).unwrap();
        let decoded = eframe::icon_data::from_png_bytes(&bytes).unwrap();
        assert_eq!((decoded.width, decoded.height), (2, 2));
        assert_eq!(
            decoded
                .rgba
                .chunks_exact(4)
                .flat_map(|p| p[..3].to_vec())
                .collect::<Vec<_>>(),
            rgb
        );
        let second = save(
            &root,
            "tamagotchi-ps",
            Kind::Lcd,
            Image::new(2, 2, rgb).unwrap(),
        )
        .unwrap();
        assert_ne!(first, second);
        assert_eq!(fs::read(first).unwrap(), bytes);
        assert!(root.join("Screenshots/Emulator").is_dir());
        let cleanup = root.canonicalize().unwrap();
        assert_eq!(
            cleanup.parent().unwrap(),
            std::env::temp_dir().canonicalize().unwrap()
        );
        fs::remove_dir_all(cleanup).unwrap();
    }
    #[test]
    fn invalid_images_and_destinations_report_errors() {
        assert!(Image::new(0, 1, vec![]).is_err());
        assert!(Image::new(128, 128, vec![0; 4]).is_err());
        assert_eq!(timestamp(UNIX_EPOCH).unwrap(), "1970-01-01_00-00-00.000Z");
        let blocked = std::env::temp_dir().join(format!(
            "retrohatch-screenshot-blocked-{}-{}",
            std::process::id(),
            SERIAL.fetch_add(1, Ordering::Relaxed)
        ));
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&blocked)
            .unwrap();
        file.write_all(b"preserve").unwrap();
        drop(file);
        assert!(prepare(&blocked).is_err());
        assert_eq!(fs::read(&blocked).unwrap(), b"preserve");
        fs::remove_file(blocked).unwrap();
    }
}
