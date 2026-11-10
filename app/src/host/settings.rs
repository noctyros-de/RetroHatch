// SPDX-License-Identifier: GPL-3.0-only
// SPDX-FileCopyrightText: 2026 Noktyros
// Applies to original RetroHatch contributions; third-party provenance is retained below and in THIRD_PARTY_LICENSES.md.
//! Persistent presentation/audio/save preferences, independently versioned from machine state.
use super::storage;
use crate::core::{
    profile::DeviceProfile,
    state::{hex, sha256},
};
use std::path::{Path, PathBuf};
pub const AUTOSAVES: [u32; 6] = [0, 1, 5, 10, 15, 30];
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Theme {
    Pink,
    Blue,
    Green,
    Yellow,
    Lilac,
}
impl Theme {
    pub const ALL: [Self; 5] = [
        Self::Pink,
        Self::Blue,
        Self::Green,
        Self::Yellow,
        Self::Lilac,
    ];
    pub fn name(self) -> &'static str {
        match self {
            Self::Pink => "Pink",
            Self::Blue => "Blue",
            Self::Green => "Green",
            Self::Yellow => "Yellow",
            Self::Lilac => "Lilac",
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Settings {
    pub theme: Theme,
    pub labels: bool,
    pub muted: bool,
    pub volume: u8,
    pub autosave_minutes: u32,
    pub save_on_exit: bool,
    pub diagnostics: bool,
    pub keep_screen_awake: bool,
    pub last_rom: Option<PathBuf>,
    pub last_profile: DeviceProfile,
    pub rom_folder: Option<PathBuf>,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            theme: Theme::Green,
            labels: true,
            muted: false,
            volume: 100,
            autosave_minutes: 1,
            save_on_exit: true,
            diagnostics: false,
            keep_screen_awake: false,
            last_rom: None,
            last_profile: DeviceProfile::P1,
            rom_folder: None,
        }
    }
}
fn path_text(p: &Option<PathBuf>) -> String {
    p.as_ref()
        .map(|p| hex(p.to_string_lossy().as_bytes()))
        .unwrap_or_default()
}
fn parse_path(v: &str) -> Result<Option<PathBuf>, String> {
    if v.is_empty() {
        return Ok(None);
    }
    if v.len() % 2 != 0 {
        return Err("Invalid settings path".into());
    }
    let bytes = (0..v.len())
        .step_by(2)
        .map(|i| {
            u8::from_str_radix(v.get(i..i + 2).ok_or("Invalid settings path")?, 16)
                .map_err(|e| e.to_string())
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Some(PathBuf::from(
        String::from_utf8(bytes).map_err(|e| e.to_string())?,
    )))
}
impl Settings {
    pub fn encode(&self) -> Vec<u8> {
        let body = format!(
            "RETROHATCH_SETTINGS=1\ntheme={}\nlabels={}\nmuted={}\nvolume={}\nautosave_minutes={}\nsave_on_exit={}\ndiagnostics={}\nkeep_screen_awake={}\nlast_profile={}\nlast_rom={}\nrom_folder={}\n",
            self.theme.name(),
            self.labels,
            self.muted,
            self.volume,
            self.autosave_minutes,
            self.save_on_exit,
            self.diagnostics,
            self.keep_screen_awake,
            self.last_profile.id(),
            path_text(&self.last_rom),
            path_text(&self.rom_folder)
        );
        format!("{body}integrity={}\n", hex(&sha256(body.as_bytes()))).into_bytes()
    }
    pub fn decode(data: &[u8]) -> Result<Self, String> {
        let text = std::str::from_utf8(data).map_err(|e| e.to_string())?;
        let (body, check) = text
            .rsplit_once("integrity=")
            .ok_or("Settings integrity field is missing")?;
        if hex(&sha256(body.as_bytes())) != check.trim() {
            return Err("Settings integrity check failed".into());
        }
        let mut s = Self::default();
        let mut seen = std::collections::HashSet::new();
        for line in body.lines() {
            let (k, v) = line.split_once('=').ok_or("Invalid settings line")?;
            if !seen.insert(k) {
                return Err("Duplicate settings field".into());
            }
            match k {
                "RETROHATCH_SETTINGS" if v == "1" => {}
                "theme" => {
                    s.theme = Theme::ALL
                        .into_iter()
                        .find(|t| t.name() == v)
                        .ok_or("Invalid theme")?
                }
                "labels" => s.labels = v.parse().map_err(|_| "Invalid labels setting")?,
                "muted" => s.muted = v.parse().map_err(|_| "Invalid mute setting")?,
                "volume" => s.volume = v.parse().map_err(|_| "Invalid volume")?,
                "autosave_minutes" => {
                    s.autosave_minutes = v.parse().map_err(|_| "Invalid autosave setting")?
                }
                "save_on_exit" => s.save_on_exit = v.parse().map_err(|_| "Invalid exit setting")?,
                "diagnostics" => {
                    s.diagnostics = v.parse().map_err(|_| "Invalid diagnostics setting")?
                }
                "keep_screen_awake" => {
                    s.keep_screen_awake = v.parse().map_err(|_| "Invalid display-sleep setting")?
                }
                "last_profile" => {
                    s.last_profile = DeviceProfile::parse(v).ok_or("Invalid saved profile")?
                }
                "last_rom" => s.last_rom = parse_path(v)?,
                "rom_folder" => s.rom_folder = parse_path(v)?,
                _ => return Err(format!("Unsupported settings field/version: {k}")),
            }
        }
        if !(seen.len() == 11 && !seen.contains("keep_screen_awake")
            || seen.len() == 12 && seen.contains("keep_screen_awake"))
            || s.volume > 100
            || !AUTOSAVES.contains(&s.autosave_minutes)
        {
            return Err("Invalid settings values".into());
        }
        Ok(s)
    }
    pub fn load(root: &Path) -> Result<Self, String> {
        match storage::read_bounded(&root.join("settings.rhc"), 65536)? {
            Some(data) => Self::decode(&data),
            None => Ok(Self::default()),
        }
    }
    pub fn store(&self, root: &Path) -> Result<(), String> {
        let path = root.join("settings.rhc");
        let data = self.encode();
        Self::decode(&data)?;
        if let Some(old) = storage::read_bounded(&path, 65536)? {
            if !old.starts_with(b"RETROHATCH_SETTINGS=") {
                return Err("Refusing to overwrite a non-settings file".into());
            }
            if old == data {
                return Ok(());
            }
            let backup = root.join("settings.rhc.bak");
            if let Some(previous) = storage::read_bounded(&backup, 65536)?
                && !previous.starts_with(b"RETROHATCH_SETTINGS=")
            {
                return Err("Refusing to overwrite a non-settings backup".into());
            }
            storage::atomic_write(&backup, &old)?;
        }
        storage::atomic_write(&path, &data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn every_device_profile_round_trips_without_changing_user_preferences() {
        for profile in DeviceProfile::ALL {
            let settings = Settings {
                last_profile: profile,
                theme: Theme::Lilac,
                muted: true,
                labels: false,
                ..Settings::default()
            };
            assert_eq!(Settings::decode(&settings.encode()).unwrap(), settings);
        }
    }
    #[test]
    fn legacy_settings_default_to_normal_sleep_and_new_policy_persists() {
        let mut settings = Settings::default();
        assert!(!settings.keep_screen_awake);
        let encoded = String::from_utf8(settings.encode()).unwrap();
        let body = encoded
            .split("integrity=")
            .next()
            .unwrap()
            .replace("keep_screen_awake=false\n", "");
        let old = format!("{body}integrity={}\n", hex(&sha256(body.as_bytes())));
        assert!(!Settings::decode(old.as_bytes()).unwrap().keep_screen_awake);
        settings.keep_screen_awake = true;
        assert_eq!(Settings::decode(&settings.encode()).unwrap(), settings);
    }
    #[test]
    fn new_installations_are_green_but_explicit_pink_preferences_are_preserved() {
        assert_eq!(Settings::default().theme, Theme::Green);
        let old = Settings {
            theme: Theme::Pink,
            ..Settings::default()
        };
        assert_eq!(Settings::decode(&old.encode()).unwrap().theme, Theme::Pink);
    }
    #[test]
    fn all_themes_labels_mute_volume_save_policy_and_unicode_paths_survive_restart_encoding() {
        for theme in Theme::ALL {
            for interval in AUTOSAVES {
                let s = Settings {
                    theme,
                    labels: false,
                    muted: true,
                    volume: 37,
                    autosave_minutes: interval,
                    save_on_exit: false,
                    diagnostics: true,
                    keep_screen_awake: true,
                    last_rom: Some(PathBuf::from("C:/Pets/Angel Omega Ω.bin")),
                    last_profile: DeviceProfile::Angel,
                    rom_folder: Some(PathBuf::from("C:/Pets")),
                };
                assert_eq!(Settings::decode(&s.encode()).unwrap(), s);
            }
        }
        let mut bytes = Settings::default().encode();
        bytes[30] ^= 1;
        assert!(Settings::decode(&bytes).is_err());
        assert!(Settings::default().save_on_exit);
        assert!(!Settings::default().diagnostics);
    }
    #[test]
    fn atomic_settings_update_keeps_previous_preferences_and_unchanged_store_keeps_backup() {
        let root =
            std::env::temp_dir().join(format!("retrohatch-settings-backup-{}", std::process::id()));
        let original = Settings::default();
        original.store(&root).unwrap();
        let next = Settings {
            theme: Theme::Blue,
            labels: false,
            ..original.clone()
        };
        next.store(&root).unwrap();
        assert_eq!(Settings::load(&root).unwrap(), next);
        assert_eq!(
            std::fs::read(root.join("settings.rhc.bak")).unwrap(),
            original.encode()
        );
        next.store(&root).unwrap();
        assert_eq!(
            std::fs::read(root.join("settings.rhc.bak")).unwrap(),
            original.encode()
        );
        assert_eq!(root.parent(), Some(std::env::temp_dir().as_path()));
        std::fs::remove_dir_all(root).unwrap();
    }
}
