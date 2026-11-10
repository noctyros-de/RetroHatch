// SPDX-License-Identifier: GPL-3.0-only
// SPDX-FileCopyrightText: 2026 Noctyros
// Applies to original RetroHatch contributions; third-party provenance is retained below and in THIRD_PARTY_LICENSES.md.
//! Local-only real-ROM library and previous-save migration acceptance.
//! Usage: library_acceptance ROM_FOLDER PREVIOUS_QA_SAVE_ROOT NEW_PRIVATE_ROOT
use retrohatch::{
    core::{device::Device, profile::DeviceProfile, state::hex, tamagotchi::Tamagotchi},
    host::{
        library::{self, Library},
        roms,
        runner::{Command, Controller, Snapshot},
        saves::SavePolicy,
    },
};
use std::{
    path::Path,
    time::{Duration, Instant},
};
fn wait(c: &Controller, f: impl Fn(&Snapshot) -> bool) -> Snapshot {
    let end = Instant::now() + Duration::from_secs(5);
    loop {
        let s = c.snapshot();
        assert!(s.save_error.is_none(), "{:?}", s.save_error);
        if f(&s) {
            return s;
        }
        assert!(Instant::now() < end, "Worker timeout: {:?}", s.error);
        std::thread::sleep(Duration::from_millis(2));
    }
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.len() != 3 {
        return Err(
            "Provide ROM folder, prior private save root and new private output root".into(),
        );
    }
    let folder = Path::new(&args[0]);
    let prior = Path::new(&args[1]);
    let root = Path::new(&args[2]);
    std::fs::create_dir_all(root)?;
    let mut lib = Library::open(root)?;
    for (i, (profile, name)) in [
        (DeviceProfile::P1, "TamagotchiP1Japanese.bin"),
        (DeviceProfile::P2, "TamagotchiP2.bin"),
        (DeviceProfile::Genjintch, "TamagotchiGenjintch.bin"),
        (DeviceProfile::Angel, "TamagotchiAngel.bin"),
        (DeviceProfile::Morino, "TamagotchiMorino.bin"),
        (DeviceProfile::Umino, "TamagotchiUmino.bin"),
    ]
    .into_iter()
    .enumerate()
    {
        let original = std::fs::read(folder.join(name))?;
        let staged = root.join(format!("unidentified-{i}.rom"));
        std::fs::write(&staged, &original)?;
        let prepared = library::inspect(&staged)?;
        let a = lib
            .automatic(&prepared)
            .ok_or("Expected a verified exact fingerprint")?;
        assert_eq!(a.profile, profile);
        let entry = lib.import(&prepared, a)?;
        let internal = lib.verified_path(&entry.key)?;
        let duplicate = lib.import(&prepared, lib.automatic(&prepared).unwrap())?;
        assert_eq!(duplicate.key, entry.key);
        std::fs::remove_file(staged)?;
        let image = roms::load_for_profile(&internal, profile)?;
        let machine = Tamagotchi::new(&image)?;
        let identity = machine.identity();
        assert_eq!(identity, format!("{}-{}", profile.id(), prepared.hash));
        let dir = root.join("saves").join(&identity);
        std::fs::create_dir_all(&dir)?;
        for slot in [
            "autosave.rhstate",
            "slot1.rhstate",
            "slot2.rhstate",
            "slot3.rhstate",
        ] {
            let bytes = std::fs::read(prior.join("saves").join(&identity).join(slot))?;
            std::fs::write(dir.join(slot), bytes)?;
        }
        let before = std::fs::read(dir.join("slot2.rhstate"))?;
        let mut imported = machine.restored(&before)?;
        let mut old = Tamagotchi::new(&roms::load_for_profile(&folder.join(name), profile)?)?
            .restored(&before)?;
        assert_eq!(imported.save_state()?, before);
        let target = old.time() + 102_400_000 / 2;
        imported.advance_to(target)?;
        old.advance_to(target)?;
        assert_eq!(imported.drain_audio(), old.drain_audio());
        assert_eq!(imported.save_state()?, old.save_state()?);
        let c = Controller::with_storage(
            false,
            root.into(),
            SavePolicy {
                autosave_minutes: 0,
                save_on_exit: false,
                volume: 37,
            },
        );
        c.send(Command::LoadVerified(
            internal.clone(),
            profile,
            library::hash_bytes(&prepared.hash)?,
        ));
        let resumed = wait(&c, |s| s.restores >= 1 && !s.loading);
        assert_eq!(resumed.profile, profile);
        assert_eq!(hex(&resumed.rom_hash.unwrap()), prepared.hash);
        c.send(Command::Pause);
        let paused = wait(&c, |s| !s.running);
        std::fs::write(&internal, vec![0; profile.rom_bytes()])?;
        c.send(Command::LoadVerified(
            internal.clone(),
            profile,
            library::hash_bytes(&prepared.hash)?,
        ));
        let failed = wait(&c, |s| s.error.is_some());
        assert_eq!(failed.state, paused.state);
        assert_eq!(failed.ticks, paused.ticks);
        assert!(failed.error.unwrap().contains("changed"));
        lib.import(&prepared, lib.automatic(&prepared).unwrap())?;
        c.send(Command::PrepareExit);
        wait(&c, |s| s.exit_ready);
        drop(c);
        assert_eq!(std::fs::read(dir.join("slot2.rhstate"))?, before);
        assert_eq!(std::fs::read(folder.join(name))?, original);
        println!(
            "{}: exact fingerprint, deduplicated import, source independence, previous saves, future PCM/state continuation, worker auto-resume and changed-ROM rejection passed.",
            profile.name()
        );
    }
    let reopened = Library::open(root)?;
    assert_eq!(reopened.entries().len(), 6);
    assert!(reopened.entries().iter().all(|e| e.problem.is_none()));
    assert_eq!(
        std::fs::read_dir(root.join(library::DIRECTORY).join("roms"))?.count(),
        6
    );
    println!(
        "Six persistent, verified entries; six internal ROM copies; no changes to original ROMs or previous saves."
    );
    Ok(())
}
