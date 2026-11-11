// SPDX-License-Identifier: GPL-3.0-only
// SPDX-FileCopyrightText: 2026 Noctyros
// Applies to original RetroHatch contributions; third-party provenance is retained below and in THIRD_PARTY_LICENSES.md.
//! Private content-addressed ROM storage, explicit associations and asynchronous UI service.
use super::{
    fingerprints::{self, Assignment},
    storage,
};
use crate::core::{
    profile::DeviceProfile,
    state::{hex, sha256},
};
use std::{
    fs::{File, OpenOptions},
    path::{Path, PathBuf},
    sync::mpsc,
    thread,
    time::{SystemTime, UNIX_EPOCH},
};
pub const DIRECTORY: &str = "rom-library";
const LIMIT: usize = 4 * 1024 * 1024;
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Key {
    pub hash: String,
    pub profile: DeviceProfile,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    pub key: Key,
    pub variant: String,
    pub evidence: String,
    pub filename: String,
    pub source_name: String,
    pub size: usize,
    pub imported: u64,
    pub listed: bool,
    pub problem: Option<String>,
}
impl Entry {
    pub fn label(&self) -> String {
        format!(
            "{} [{}]\n{}",
            self.key.profile.name(),
            &self.key.hash[..8],
            self.variant
        )
    }
    fn assignment(&self) -> Assignment {
        Assignment {
            profile: self.key.profile,
            variant: self.variant.clone(),
            evidence: self.evidence.clone(),
        }
    }
}
#[derive(Clone, Debug)]
pub struct Prepared {
    pub hash: String,
    pub bytes: Vec<u8>,
    pub source_name: String,
    pub candidates: Vec<DeviceProfile>,
}
fn maximum() -> usize {
    DeviceProfile::ALL
        .into_iter()
        .map(|p| p.rom_bytes())
        .max()
        .unwrap()
}
pub fn inspect_bytes(bytes: Vec<u8>, source_name: String) -> Result<Prepared, String> {
    let hash = hex(&sha256(&bytes));
    super::pending_devices::ensure_supported(&hash)?;
    let candidates = fingerprints::tg18_candidates(&bytes, &hash).unwrap_or_else(|| {
        DeviceProfile::ALL
            .into_iter()
            .filter(|p| super::emulator::validate_rom(&bytes, *p).is_ok())
            .collect()
    });
    let p = *candidates
        .first()
        .ok_or("Unsupported ROM format or size. Import a supported classic image, color NOR image or explicitly verified archive.")?;
    super::emulator::validate_rom(&bytes, p)?;
    Ok(Prepared {
        hash,
        bytes,
        source_name: source_name.chars().take(240).collect(),
        candidates,
    })
}
pub fn inspect(path: &Path) -> Result<Prepared, String> {
    let bytes = storage::read_bounded(path, maximum())?
        .ok_or("The ROM file is missing. Reimport it from an available original.")?;
    inspect_bytes(
        bytes,
        path.file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned(),
    )
}
fn valid_hash(s: &str) -> bool {
    s.len() == 64
        && s.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
pub fn hash_bytes(hash: &str) -> Result<[u8; 32], String> {
    if !valid_hash(hash) {
        return Err("Invalid ROM SHA-256".into());
    }
    let mut bytes = [0; 32];
    for (i, b) in bytes.iter_mut().enumerate() {
        *b = u8::from_str_radix(&hash[i * 2..i * 2 + 2], 16).map_err(|e| e.to_string())?;
    }
    Ok(bytes)
}
fn text_hex(s: &str) -> String {
    hex(s.as_bytes())
}
fn decode_text(s: &str) -> Result<String, String> {
    if s.len() > 8192 || s.len() % 2 != 0 {
        return Err("Invalid library metadata".into());
    }
    let b = (0..s.len())
        .step_by(2)
        .map(|i| {
            u8::from_str_radix(s.get(i..i + 2).ok_or("Invalid metadata encoding")?, 16)
                .map_err(|_| "Invalid metadata encoding")
        })
        .collect::<Result<Vec<_>, _>>()?;
    String::from_utf8(b).map_err(|_| "Invalid library metadata text".into())
}
pub struct Library {
    root: PathBuf,
    records: Vec<Entry>,
}
impl Library {
    pub fn open(user_root: &Path) -> Result<Self, String> {
        let root = user_root.join(DIRECTORY);
        check_directory(&root)?;
        check_directory(&root.join("roms"))?;
        let records=match storage::read_bounded(&root.join("index.rhl"),LIMIT)?{Some(b)=>Self::decode(&b).map_err(|e|format!("Library index: {e}. Use Manage ROMs to restore its backup; existing ROMs and saves have been kept."))?,None=>Vec::new()};
        Ok(Self { root, records })
    }
    fn lock(&self) -> Result<File, String> {
        check_directory(&self.root)?;
        std::fs::create_dir_all(&self.root).map_err(|e| e.to_string())?;
        let file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(self.root.join("index.lock"))
            .map_err(|e| e.to_string())?;
        file.try_lock().map_err(|_| {
            "Another RetroHatch operation is updating this library. Please retry.".to_string()
        })?;
        Ok(file)
    }
    fn reload(&mut self) -> Result<(), String> {
        if let Some(b) = storage::read_bounded(&self.root.join("index.rhl"), LIMIT)? {
            self.records = Self::decode(&b)?;
        } else {
            self.records.clear();
        }
        Ok(())
    }
    fn persist(&self) -> Result<(), String> {
        if self.records.len() > 4096 {
            return Err("The library supports up to 4,096 model associations".into());
        }
        let data = Self::encode(&self.records);
        Self::decode(&data)?;
        let path = self.root.join("index.rhl");
        if let Some(old) = storage::read_bounded(&path, LIMIT)? {
            Self::decode(&old)?;
            if old == data {
                return Ok(());
            }
            storage::atomic_write(&self.root.join("index.rhl.bak"), &old)?;
        }
        storage::atomic_write(&path, &data)
    }
    fn encode(entries: &[Entry]) -> Vec<u8> {
        let mut body = "RETROHATCH_LIBRARY=1\n".to_string();
        for e in entries {
            body.push_str(&format!(
                "{}|{}|{}|{}|{}|{}|{}|{}|{}\n",
                e.key.hash,
                e.key.profile.id(),
                text_hex(&e.variant),
                text_hex(&e.evidence),
                e.filename,
                text_hex(&e.source_name),
                e.size,
                e.imported,
                u8::from(e.listed)
            ));
        }
        format!("{body}integrity={}\n", hex(&sha256(body.as_bytes()))).into_bytes()
    }
    fn decode(data: &[u8]) -> Result<Vec<Entry>, String> {
        if data.len() > LIMIT {
            return Err("Library index is too large".into());
        }
        let text = std::str::from_utf8(data).map_err(|_| "Invalid library index encoding")?;
        let (body, check) = text
            .rsplit_once("integrity=")
            .ok_or("Library integrity field is missing")?;
        if hex(&sha256(body.as_bytes())) != check.trim() {
            return Err("Library integrity check failed".into());
        }
        let mut lines = body.lines();
        if lines.next() != Some("RETROHATCH_LIBRARY=1") {
            return Err("Unsupported library format version".into());
        }
        let mut entries = Vec::new();
        let mut keys = std::collections::HashSet::new();
        for line in lines {
            let f: Vec<_> = line.split('|').collect();
            if f.len() != 9 || !valid_hash(f[0]) {
                return Err("Invalid library entry".into());
            }
            let profile = DeviceProfile::parse(f[1]).ok_or("Unsupported library model")?;
            let key = Key {
                hash: f[0].into(),
                profile,
            };
            if f[4] != format!("{}.bin", key.hash) || !keys.insert((key.hash.clone(), profile.id()))
            {
                return Err("Invalid or duplicate library identity".into());
            }
            let size = f[6].parse::<usize>().map_err(|_| "Invalid ROM length")?;
            let valid_size = profile
                .s1c33_key()
                .and_then(crate::core::s1c33::profile::HardwareProfile::find)
                .map(|p| crate::core::s1c33::machine::source_size_matches(p, &key.hash, size))
                .unwrap_or(size == profile.rom_bytes());
            if !valid_size {
                return Err("Library profile and ROM length disagree".into());
            }
            let variant = decode_text(f[2])?;
            if variant.trim().is_empty() {
                return Err("A library variant label is required".into());
            }
            entries.push(Entry {
                key,
                variant,
                evidence: decode_text(f[3])?,
                filename: f[4].into(),
                source_name: decode_text(f[5])?,
                size,
                imported: f[7].parse().map_err(|_| "Invalid import time")?,
                listed: match f[8] {
                    "0" => false,
                    "1" => true,
                    _ => return Err("Invalid library visibility".into()),
                },
                problem: None,
            });
            if entries.len() > 4096 {
                return Err("Too many library associations".into());
            }
        }
        Ok(entries)
    }
    fn blob(&self, hash: &str) -> Result<PathBuf, String> {
        if !valid_hash(hash) {
            return Err("Invalid library identity".into());
        }
        check_directory(&self.root)?;
        check_directory(&self.root.join("roms"))?;
        Ok(self.root.join("roms").join(format!("{hash}.bin")))
    }
    pub fn verified_path(&self, key: &Key) -> Result<PathBuf, String> {
        let entry = self
            .records
            .iter()
            .find(|e| e.listed && e.key == *key)
            .ok_or("This library entry was removed. Import it again to restore it.")?;
        let path = self.blob(&entry.key.hash)?;
        if std::fs::symlink_metadata(&path).is_ok_and(|m| is_link(&m)) {
            return Err(
                "The internal ROM copy is a link. Reimport a regular ROM file to repair it.".into(),
            );
        }
        let data=storage::read_bounded(&path,maximum())?.ok_or("The internal ROM copy is missing. Use Repair to select the same firmware again; original files and saves are untouched.")?;
        if hex(&sha256(&data)) != key.hash {
            return Err("The internal ROM copy is corrupted. Use Repair to reimport matching firmware; saves are untouched.".into());
        }
        super::pending_devices::ensure_supported(&key.hash)?;
        super::emulator::validate_rom(&data, key.profile)?;
        Ok(path)
    }
    pub fn entries(&self) -> Vec<Entry> {
        let mut out: Vec<_> = self.records.iter().filter(|e| e.listed).cloned().collect();
        for e in &mut out {
            e.problem = self.verified_path(&e.key).err();
        }
        out.sort_by_key(|e| e.label());
        out
    }
    pub fn suggestions(&self, p: &Prepared) -> Vec<Assignment> {
        let mut remembered: Vec<_> = self
            .records
            .iter()
            .filter(|e| e.key.hash == p.hash)
            .map(Entry::assignment)
            .collect();
        if !remembered.is_empty() {
            let editions: Vec<_> = fingerprints::matches(&p.hash, fingerprints::CATALOG)
                .into_iter()
                .filter(|a| a.profile.is_meets())
                .collect();
            if editions.len() == 1
                && remembered
                    .iter()
                    .all(|a| a.profile == DeviceProfile::On || a.profile == editions[0].profile)
            {
                return editions;
            }
            // A new verified identity must not silently retain an old guessed model.
            // Keep both choices and all existing saves when evidence conflicts.
            for known in fingerprints::matches(&p.hash, fingerprints::CATALOG) {
                if !remembered.iter().any(|a| a.profile == known.profile) {
                    remembered.push(known);
                }
            }
            remembered
        } else {
            let choices = fingerprints::matches(&p.hash, fingerprints::CATALOG);
            if choices.is_empty() && p.candidates == [DeviceProfile::On] {
                if let Some(known) = tg18::roms::identify(&p.bytes) {
                    return vec![Assignment { profile: DeviceProfile::On,
                        variant: format!("{} {}", if known.region == "en" { "On" } else { "Meets" }, known.display_name()),
                        evidence: "TG18 upstream code fingerprint (FNV-1a); full-dump SHA-256 stored locally".into() }];
                }
            }
            choices
        }
    }
    pub fn automatic(&self, p: &Prepared) -> Option<Assignment> {
        let choices = self.suggestions(p);
        (choices.len() == 1).then(|| choices[0].clone())
    }
    pub fn preferred(&self, p: &Prepared, profile: DeviceProfile) -> Assignment {
        fingerprints::for_profile(&self.suggestions(p), profile)
    }
    pub fn import(&mut self, p: &Prepared, a: Assignment) -> Result<Entry, String> {
        super::pending_devices::ensure_supported(&p.hash)?;
        if !p.candidates.contains(&a.profile) || hex(&sha256(&p.bytes)) != p.hash {
            return Err("ROM contents do not match the selected profile or identity".into());
        }
        let _lock = self.lock()?;
        self.reload()?;
        let before = self.records.clone();
        let path = self.blob(&p.hash)?;
        let valid = storage::read_bounded(&path, maximum())
            .ok()
            .flatten()
            .is_some_and(|b| b == p.bytes)
            && !std::fs::symlink_metadata(&path).is_ok_and(|m| is_link(&m));
        if !valid {
            check_directory(&self.root.join("roms"))?;
            storage::atomic_write(&path, &p.bytes)?;
        }
        let key = Key {
            hash: p.hash.clone(),
            profile: a.profile,
        };
        let entry = if let Some(e) = self.records.iter_mut().find(|e| e.key == key) {
            e.listed = true;
            e.variant = if a.variant.trim().is_empty() {
                "Region not verified".into()
            } else {
                a.variant.trim().chars().take(128).collect()
            };
            e.evidence = a.evidence;
            e.clone()
        } else {
            let e = Entry {
                key,
                variant: if a.variant.trim().is_empty() {
                    "Region not verified".into()
                } else {
                    a.variant.trim().chars().take(128).collect()
                },
                evidence: a.evidence,
                filename: format!("{}.bin", p.hash),
                source_name: p.source_name.clone(),
                size: p.bytes.len(),
                imported: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
                listed: true,
                problem: None,
            };
            self.records.push(e.clone());
            e
        };
        if let Err(e) = self.persist() {
            self.records = before;
            return Err(e);
        }
        Ok(entry)
    }
    pub fn update(&mut self, key: &Key, a: Assignment, keep_original: bool) -> Result<(), String> {
        let _lock = self.lock()?;
        self.reload()?;
        let path = self.verified_path(key)?;
        let p = inspect(&path)?;
        if !p.candidates.contains(&a.profile) {
            return Err("The selected model is incompatible with this ROM structure".into());
        }
        let before = self.records.clone();
        let old = self.records.iter().find(|e| e.key == *key).unwrap().clone();
        if !keep_original {
            self.records.retain(|e| e.key != *key);
        }
        let new_key = Key {
            hash: key.hash.clone(),
            profile: a.profile,
        };
        let mut updated = old;
        updated.key = new_key.clone();
        updated.variant = if a.variant.trim().is_empty() {
            "Region not verified".into()
        } else {
            a.variant.trim().chars().take(128).collect()
        };
        updated.evidence = "User-confirmed model / variant".into();
        self.records.retain(|e| e.key != new_key);
        self.records.push(updated);
        if let Err(e) = self.persist() {
            self.records = before;
            return Err(e);
        }
        Ok(())
    }
    pub fn remove(&mut self, key: &Key, delete_copy: bool) -> Result<(), String> {
        let _lock = self.lock()?;
        self.reload()?;
        if delete_copy
            && self
                .records
                .iter()
                .any(|e| e.listed && e.key.hash == key.hash && e.key != *key)
        {
            return Err("Another model association still uses this ROM copy. Remove the entry without deleting the shared file.".into());
        }
        let before = self.records.clone();
        let entry = self
            .records
            .iter_mut()
            .find(|e| e.listed && e.key == *key)
            .ok_or("Library entry not found")?;
        entry.listed = false;
        if let Err(e) = self.persist() {
            self.records = before;
            return Err(e);
        }
        if delete_copy {
            let path = self.blob(&key.hash)?;
            match std::fs::remove_file(path) {
                Ok(()) => {}
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => {
                    return Err(format!(
                        "Entry removed, but the internal copy could not be deleted: {e}. Saves were kept."
                    ));
                }
            }
        }
        Ok(())
    }
    pub fn restore_backup(root: &Path) -> Result<(), String> {
        let base = root.join(DIRECTORY);
        let lib = Self {
            root: base.clone(),
            records: Vec::new(),
        };
        let _lock = lib.lock()?;
        let backup = storage::read_bounded(&base.join("index.rhl.bak"), LIMIT)?.ok_or(
            "No library index backup is available. Existing ROM copies and saves have been kept.",
        )?;
        Self::decode(&backup)?;
        if let Some(old) = storage::read_bounded(&base.join("index.rhl"), LIMIT)? {
            let stamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos();
            storage::atomic_write(&base.join(format!("index-{stamp}.damaged")), &old)?;
        }
        storage::atomic_write(&base.join("index.rhl"), &backup)
    }
}
fn is_link(m: &std::fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        m.file_attributes() & 0x400 != 0
    }
    #[cfg(not(windows))]
    {
        m.file_type().is_symlink()
    }
}
fn check_directory(path: &Path) -> Result<(), String> {
    match std::fs::symlink_metadata(path) {
        Ok(m) if is_link(&m) || !m.is_dir() => {
            Err("The private ROM library must use regular directories, not links or files.".into())
        }
        Ok(_) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e.to_string()),
    }
}
pub enum Action {
    Refresh,
    Import(PathBuf, Option<DeviceProfile>),
    Migrate(PathBuf, DeviceProfile),
    Confirm(Prepared, Assignment),
    Start(Key),
    Repair(PathBuf, Key),
    Update(Key, Assignment, bool),
    Remove(Key, bool),
    RestoreBackup,
}
pub enum Event {
    View(Vec<Entry>),
    Choice {
        prepared: Prepared,
        suggestions: Vec<Assignment>,
        entries: Vec<Entry>,
    },
    Ready {
        entry: Entry,
        path: PathBuf,
        entries: Vec<Entry>,
    },
    Changed {
        entries: Vec<Entry>,
        message: String,
    },
    Failed {
        message: String,
        repair: Option<Key>,
        entries: Option<Vec<Entry>>,
    },
}
pub struct Service {
    tx: mpsc::Sender<Action>,
    rx: mpsc::Receiver<Event>,
}
impl Service {
    pub fn new(root: PathBuf) -> Self {
        let (tx, actions) = mpsc::channel();
        let (events, rx) = mpsc::channel();
        thread::spawn(move || {
            while let Ok(action) = actions.recv() {
                let repair = match &action {
                    Action::Start(k) | Action::Repair(_, k) => Some(k.clone()),
                    _ => None,
                };
                let result = (|| {
                    if matches!(action, Action::RestoreBackup) {
                        Library::restore_backup(&root)?;
                        return Ok(Event::Changed {
                            entries: Library::open(&root)?.entries(),
                            message: "Library index backup restored. Saves were kept.".into(),
                        });
                    }
                    let mut lib = Library::open(&root)?;
                    match action {
                        Action::Refresh => Ok(Event::View(lib.entries())),
                        Action::Import(path, preferred) => {
                            let p = inspect(&path)?;
                            let a = preferred
                                .map(|profile| lib.preferred(&p, profile))
                                .or_else(|| lib.automatic(&p));
                            if let Some(a) = a {
                                let entry = lib.import(&p, a)?;
                                let path = lib.verified_path(&entry.key)?;
                                Ok(Event::Ready {
                                    entry,
                                    path,
                                    entries: lib.entries(),
                                })
                            } else {
                                let suggestions = lib.suggestions(&p);
                                Ok(Event::Choice {
                                    prepared: p,
                                    suggestions,
                                    entries: lib.entries(),
                                })
                            }
                        }
                        Action::Migrate(path, profile) => {
                            let p = inspect(&path)?;
                            let known = fingerprints::matches(&p.hash, fingerprints::CATALOG);
                            let remembered = lib
                                .records
                                .iter()
                                .any(|e| e.key.hash == p.hash && e.key.profile == profile);
                            if !p.candidates.contains(&profile)
                                || (!remembered
                                    && !known.is_empty()
                                    && !known.iter().any(|a| a.profile == profile))
                            {
                                return Ok(Event::Choice {
                                    suggestions: known,
                                    prepared: p,
                                    entries: lib.entries(),
                                });
                            }
                            let a = lib.preferred(&p, profile);
                            let entry = lib.import(&p, a)?;
                            let path = lib.verified_path(&entry.key)?;
                            Ok(Event::Ready {
                                entry,
                                path,
                                entries: lib.entries(),
                            })
                        }
                        Action::Confirm(p, a) => {
                            let entry = lib.import(&p, a)?;
                            let path = lib.verified_path(&entry.key)?;
                            Ok(Event::Ready {
                                entry,
                                path,
                                entries: lib.entries(),
                            })
                        }
                        Action::Start(key) => {
                            let path = lib.verified_path(&key)?;
                            let entry = lib.records.iter().find(|e| e.key == key).unwrap().clone();
                            Ok(Event::Ready {
                                entry,
                                path,
                                entries: lib.entries(),
                            })
                        }
                        Action::Repair(path, key) => {
                            let p = inspect(&path)?;
                            if p.hash != key.hash {
                                return Err("That file does not match the required ROM SHA-256. The library, originals and saves were not changed.".into());
                            }
                            let a = lib.preferred(&p, key.profile);
                            lib.import(&p, a)?;
                            Ok(Event::Changed {
                                entries: lib.entries(),
                                message:
                                    "Internal ROM copy repaired. Start it from My Tamagotchis."
                                        .into(),
                            })
                        }
                        Action::Update(key, a, keep) => {
                            lib.update(&key, a, keep)?;
                            Ok(Event::Changed {
                                entries: lib.entries(),
                                message:
                                    "Model association saved. Previous-profile saves were kept."
                                        .into(),
                            })
                        }
                        Action::Remove(key, delete) => {
                            lib.remove(&key, delete)?;
                            Ok(Event::Changed {
                                entries: lib.entries(),
                                message: "Library entry removed. Saved games were kept.".into(),
                            })
                        }
                        Action::RestoreBackup => unreachable!(),
                    }
                })();
                let event = result.unwrap_or_else(|message| Event::Failed {
                    message,
                    repair,
                    entries: Library::open(&root).ok().map(|lib| lib.entries()),
                });
                if events.send(event).is_err() {
                    break;
                }
            }
        });
        Self { tx, rx }
    }
    pub fn send(&self, action: Action) -> Result<(), String> {
        self.tx.send(action).map_err(|_| {
            "The ROM library service stopped. Restart RetroHatch; files and saves were kept.".into()
        })
    }
    pub fn poll(&self) -> Option<Event> {
        self.rx.try_recv().ok()
    }
}
#[cfg(test)]
mod tests;
