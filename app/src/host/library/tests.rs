// SPDX-License-Identifier: GPL-3.0-only
// SPDX-FileCopyrightText: 2026 Noktyros
// Applies to original RetroHatch contributions; third-party provenance is retained below and in THIRD_PARTY_LICENSES.md.
use super::*;
struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        let n = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let p = std::env::temp_dir().join(format!("retrohatch-library-{}-{n}", std::process::id()));
        assert!(!p.exists());
        std::fs::create_dir(&p).unwrap();
        Self(p)
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        assert_eq!(self.0.parent(), Some(std::env::temp_dir().as_path()));
        std::fs::remove_dir_all(&self.0).unwrap();
    }
}
fn program(profile: DeviceProfile) -> Vec<u8> {
    (0..profile.rom_bytes() / 2)
        .flat_map(|_| 0xFF8u16.to_be_bytes())
        .collect()
}
fn assignment(profile: DeviceProfile) -> Assignment {
    Assignment {
        profile,
        variant: "Test variant".into(),
        evidence: "Synthetic test confirmation".into(),
    }
}
#[test]
fn detection_uses_verified_hashes_and_never_promotes_filenames_size_or_crc_to_identity() {
    let root = Temp::new();
    let lib = Library::open(&root.0).unwrap();
    let p = inspect_bytes(
        program(DeviceProfile::Umino),
        "TamagotchiP1Japanese.bin".into(),
    )
    .unwrap();
    assert_eq!(p.candidates.len(), 4);
    assert!(!p.candidates.contains(&DeviceProfile::P1));
    assert!(lib.automatic(&p).is_none());
    let small = inspect_bytes(program(DeviceProfile::P1), "Ocean.bin".into()).unwrap();
    assert_eq!(small.candidates, vec![DeviceProfile::P1, DeviceProfile::P2]);
    assert!(lib.automatic(&small).is_none());
    assert!(inspect_bytes(vec![0; 12], "x".into()).is_err());
    let mut bad = program(DeviceProfile::P1);
    bad[0] = 16;
    assert!(inspect_bytes(bad, "x".into()).is_err());
    for profile in [
        DeviceProfile::P1,
        DeviceProfile::P2,
        DeviceProfile::Genjintch,
        DeviceProfile::Angel,
        DeviceProfile::Morino,
        DeviceProfile::Umino,
    ] {
        assert!(fingerprints::CATALOG.iter().any(|f| f.profile == profile));
    }
    for f in fingerprints::CATALOG {
        assert!(valid_hash(f.sha256));
        assert!(
            fingerprints::matches(f.sha256, fingerprints::CATALOG)
                .iter()
                .any(|a| a.profile == f.profile && a.variant == f.variant)
        );
    }
    let shared = [
        fingerprints::Fingerprint {
            sha256: &p.hash,
            profile: DeviceProfile::Angel,
            variant: "Shared Japanese",
        },
        fingerprints::Fingerprint {
            sha256: &p.hash,
            profile: DeviceProfile::Morino,
            variant: "Shared regional edition",
        },
    ];
    assert_eq!(fingerprints::matches(&p.hash, &shared).len(), 2);
    let regional = [
        fingerprints::Fingerprint {
            sha256: &p.hash,
            profile: DeviceProfile::Angel,
            variant: "Synthetic edition A",
        },
        fingerprints::Fingerprint {
            sha256: &p.hash,
            profile: DeviceProfile::Angel,
            variant: "Synthetic edition B",
        },
    ];
    assert_eq!(
        fingerprints::for_profile(
            &fingerprints::matches(&p.hash, &regional),
            DeviceProfile::Angel
        )
        .variant,
        "Region not verified"
    );
}
#[test]
fn import_deduplicates_persists_after_source_removal_and_remembers_confirmation() {
    let root = Temp::new();
    let source = root.0.join("unrelated-name.rom");
    let bytes = program(DeviceProfile::Umino);
    std::fs::write(&source, &bytes).unwrap();
    let p = inspect(&source).unwrap();
    let mut lib = Library::open(&root.0).unwrap();
    let entry = lib.import(&p, assignment(DeviceProfile::Umino)).unwrap();
    assert_eq!(std::fs::read(&source).unwrap(), bytes);
    let duplicate = lib.import(&p, assignment(DeviceProfile::Umino)).unwrap();
    assert_eq!(entry, duplicate);
    assert_eq!(std::fs::read_dir(lib.root.join("roms")).unwrap().count(), 1);
    std::fs::remove_file(&source).unwrap();
    let mut lib = Library::open(&root.0).unwrap();
    assert_eq!(lib.automatic(&p).unwrap().profile, DeviceProfile::Umino);
    assert_eq!(
        std::fs::read(lib.verified_path(&entry.key).unwrap()).unwrap(),
        bytes
    );
    lib.remove(&entry.key, false).unwrap();
    assert!(lib.entries().is_empty());
    let mut lib = Library::open(&root.0).unwrap();
    assert_eq!(lib.automatic(&p).unwrap().profile, DeviceProfile::Umino);
    lib.import(&p, lib.automatic(&p).unwrap()).unwrap();
    assert_eq!(lib.entries().len(), 1);
}
#[test]
fn identical_firmware_can_have_explicit_distinct_profiles_without_duplicate_files_or_saves() {
    let root = Temp::new();
    let p = inspect_bytes(program(DeviceProfile::Angel), "shared.rom".into()).unwrap();
    let mut lib = Library::open(&root.0).unwrap();
    let a = lib.import(&p, assignment(DeviceProfile::Angel)).unwrap();
    lib.update(&a.key, assignment(DeviceProfile::Morino), true)
        .unwrap();
    let lib2 = Library::open(&root.0).unwrap();
    assert!(lib2.automatic(&p).is_none());
    assert_eq!(lib2.entries().len(), 2);
    assert_eq!(std::fs::read_dir(lib.root.join("roms")).unwrap().count(), 1);
    let path = root
        .0
        .join("saves")
        .join(format!("{}-{}", a.key.profile.id(), p.hash))
        .join("slot1.rhstate");
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, b"existing-save-sentinel").unwrap();
    assert!(lib.remove(&a.key, true).is_err());
    assert_eq!(lib.entries().len(), 2);
    lib.remove(&a.key, false).unwrap();
    assert_eq!(std::fs::read(path).unwrap(), b"existing-save-sentinel");
    assert_eq!(lib.entries().len(), 1);
}
#[test]
fn missing_corrupt_and_repaired_internal_images_are_verified_before_start() {
    let root = Temp::new();
    let p = inspect_bytes(program(DeviceProfile::Genjintch), "source".into()).unwrap();
    let mut lib = Library::open(&root.0).unwrap();
    let entry = lib
        .import(&p, assignment(DeviceProfile::Genjintch))
        .unwrap();
    let path = lib.verified_path(&entry.key).unwrap();
    std::fs::remove_file(&path).unwrap();
    assert!(
        lib.entries()[0]
            .problem
            .as_ref()
            .unwrap()
            .contains("missing")
    );
    lib.import(&p, assignment(DeviceProfile::Genjintch))
        .unwrap();
    assert!(lib.verified_path(&entry.key).is_ok());
    std::fs::write(&path, vec![0; 16384]).unwrap();
    assert!(
        lib.entries()[0]
            .problem
            .as_ref()
            .unwrap()
            .contains("corrupted")
    );
    lib.import(&p, assignment(DeviceProfile::Genjintch))
        .unwrap();
    assert_eq!(std::fs::read(path).unwrap(), p.bytes);
}
#[test]
fn index_integrity_version_backup_and_writer_lock_prevent_silent_metadata_loss() {
    let root = Temp::new();
    let p = inspect_bytes(program(DeviceProfile::Genjintch), "source".into()).unwrap();
    let mut lib = Library::open(&root.0).unwrap();
    let entry = lib
        .import(&p, assignment(DeviceProfile::Genjintch))
        .unwrap();
    lib.update(&entry.key, assignment(DeviceProfile::Morino), true)
        .unwrap();
    let index = lib.root.join("index.rhl");
    let before = std::fs::read(&index).unwrap();
    let lock = lib.lock().unwrap();
    assert!(lib.remove(&entry.key, false).is_err());
    assert_eq!(std::fs::read(&index).unwrap(), before);
    drop(lock);
    std::fs::write(&index, b"damaged").unwrap();
    assert!(Library::open(&root.0).is_err());
    Library::restore_backup(&root.0).unwrap();
    assert_eq!(Library::open(&root.0).unwrap().entries().len(), 1);
    let unsupported = "RETROHATCH_LIBRARY=2\n";
    let data = format!(
        "{unsupported}integrity={}\n",
        hex(&sha256(unsupported.as_bytes()))
    );
    assert!(
        Library::decode(data.as_bytes())
            .unwrap_err()
            .contains("version")
    );
}
#[test]
fn model_correction_changes_only_association_and_preserves_original_copy() {
    let root = Temp::new();
    let source = root.0.join("original.bin");
    let bytes = program(DeviceProfile::Umino);
    std::fs::write(&source, &bytes).unwrap();
    let p = inspect(&source).unwrap();
    let mut lib = Library::open(&root.0).unwrap();
    let entry = lib.import(&p, assignment(DeviceProfile::Angel)).unwrap();
    lib.update(&entry.key, assignment(DeviceProfile::Umino), false)
        .unwrap();
    assert_eq!(lib.automatic(&p).unwrap().profile, DeviceProfile::Umino);
    assert_eq!(lib.entries().len(), 1);
    assert!(
        lib.update(&lib.entries()[0].key, assignment(DeviceProfile::P1), false)
            .is_err()
    );
    assert_eq!(std::fs::read(source).unwrap(), bytes);
    let key = lib.entries()[0].key.clone();
    lib.remove(&key, true).unwrap();
    assert!(!lib.blob(&p.hash).unwrap().exists());
    assert_eq!(
        Library::open(&root.0)
            .unwrap()
            .automatic(&p)
            .unwrap()
            .profile,
        DeviceProfile::Umino
    );
}

#[test]
fn service_keeps_unknown_import_pending_and_repairs_only_matching_content() {
    let root = Temp::new();
    let source = root.0.join("source.bin");
    std::fs::write(&source, program(DeviceProfile::Umino)).unwrap();
    let service = Service::new(root.0.clone());
    let next = || {
        let end = std::time::Instant::now() + std::time::Duration::from_secs(3);
        loop {
            if let Some(e) = service.poll() {
                return e;
            }
            assert!(std::time::Instant::now() < end);
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
    };
    service.send(Action::Import(source.clone(), None)).unwrap();
    let Event::Choice { prepared, .. } = next() else {
        panic!("Unknown ROM was guessed")
    };
    assert!(Library::open(&root.0).unwrap().entries().is_empty());
    service
        .send(Action::Confirm(
            prepared.clone(),
            assignment(DeviceProfile::Umino),
        ))
        .unwrap();
    let Event::Ready { entry, path, .. } = next() else {
        panic!("Confirmed import failed")
    };
    std::fs::remove_file(&path).unwrap();
    service.send(Action::Start(entry.key.clone())).unwrap();
    assert!(matches!(
        next(),
        Event::Failed {
            repair: Some(_),
            ..
        }
    ));
    let wrong = root.0.join("wrong.bin");
    std::fs::write(&wrong, vec![0; 16384]).unwrap();
    service
        .send(Action::Repair(wrong, entry.key.clone()))
        .unwrap();
    assert!(matches!(next(), Event::Failed { .. }));
    assert!(!path.exists());
    service
        .send(Action::Repair(source, entry.key.clone()))
        .unwrap();
    assert!(matches!(next(), Event::Changed { .. }));
    service.send(Action::Start(entry.key)).unwrap();
    assert!(matches!(next(), Event::Ready { .. }));
    drop(service);
}

#[test]
fn pending_device_cannot_be_forced_through_confirmation_or_modify_the_library() {
    let root = Temp::new();
    let mut lib = Library::open(&root.0).unwrap();
    let p1 = inspect_bytes(program(DeviceProfile::P1), "existing.rom".into()).unwrap();
    let existing = lib.import(&p1, assignment(DeviceProfile::P1)).unwrap();
    let index = std::fs::read(lib.root.join("index.rhl")).unwrap();
    // Public Prepared values must not bypass the unavailable-device guard.
    // This is synthetic metadata, not an embedded or generated firmware image.
    let pending = Prepared {
        hash: crate::host::pending_devices::TAMAOTCH.sha256.into(),
        bytes: program(DeviceProfile::Angel),
        source_name: "renamed.rom".into(),
        candidates: DeviceProfile::ALL.to_vec(),
    };
    for profile in DeviceProfile::ALL {
        assert!(
            lib.import(&pending, assignment(profile))
                .unwrap_err()
                .contains("TamaOtch")
        );
    }
    assert_eq!(std::fs::read(lib.root.join("index.rhl")).unwrap(), index);
    assert_eq!(lib.entries(), vec![existing]);
    assert_eq!(std::fs::read_dir(lib.root.join("roms")).unwrap().count(), 1);
    assert!(!lib.blob(&pending.hash).unwrap().exists());
}

#[test]
fn unknown_p1_sized_content_requires_confirmation_and_remembers_p2_without_save_mixing() {
    let root = Temp::new();
    let p = inspect_bytes(program(DeviceProfile::P2), "P1-misleading-name.bin".into()).unwrap();
    let mut lib = Library::open(&root.0).unwrap();
    assert_eq!(p.candidates, vec![DeviceProfile::P1, DeviceProfile::P2]);
    assert!(lib.automatic(&p).is_none());
    let p2 = lib.import(&p, assignment(DeviceProfile::P2)).unwrap();
    let mut again = Library::open(&root.0).unwrap();
    assert_eq!(again.automatic(&p).unwrap().profile, DeviceProfile::P2);
    again
        .update(&p2.key, assignment(DeviceProfile::P1), true)
        .unwrap();
    assert!(again.automatic(&p).is_none());
    assert_eq!(again.entries().len(), 2);
    assert_eq!(
        std::fs::read_dir(again.root.join("roms")).unwrap().count(),
        1
    );
    let p2_path = root
        .0
        .join("saves")
        .join(format!("{}-{}", DeviceProfile::P2.id(), p.hash));
    let p1_path = root
        .0
        .join("saves")
        .join(format!("{}-{}", DeviceProfile::P1.id(), p.hash));
    assert_ne!(p1_path, p2_path);
}
