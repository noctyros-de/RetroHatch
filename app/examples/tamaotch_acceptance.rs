//! Local-only acceptance for recognized but unavailable TamaOtch hardware.
//! Usage: tamaotch_acceptance ROM_FOLDER NEW_PRIVATE_OUTPUT_ROOT
//! No firmware is embedded; the TamaOtch image is inspected and rejected, never executed.
use retrohatch::{
    core::{
        device::Device,
        mcu::MASTER_HZ,
        profile::DeviceProfile,
        state::{hex, sha256},
        tamagotchi::Tamagotchi,
    },
    host::{
        fingerprints::Assignment,
        library::{self, Library, Prepared},
        pending_devices::{self, TAMAOTCH},
        roms,
        runner::{Command, Controller, Snapshot},
    },
};
use std::{
    path::Path,
    time::{Duration, Instant},
};

fn wait(c: &Controller, predicate: impl Fn(&Snapshot) -> bool) -> Snapshot {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let s = c.snapshot();
        if predicate(&s) {
            return s;
        }
        assert!(Instant::now() < deadline, "Worker timeout: {:?}", s.error);
        std::thread::sleep(Duration::from_millis(2));
    }
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.len() != 2 {
        return Err("Provide a local ROM folder and a new private output root".into());
    }
    let folder = Path::new(&args[0]);
    let root = Path::new(&args[1]);
    if root.exists() {
        return Err("Use a new private output directory to preserve existing data".into());
    }
    std::fs::create_dir_all(root)?;
    let source = folder.join("TamagotchiTamaotch.bin");
    let bytes = std::fs::read(&source)?;
    let hash = hex(&sha256(&bytes));
    assert_eq!(pending_devices::identify(&hash), Some(&TAMAOTCH));
    assert_eq!(bytes.len(), TAMAOTCH.rom_bytes);
    assert!(
        bytes
            .chunks_exact(2)
            .all(|w| u16::from_be_bytes([w[0], w[1]]) <= 0xFFF)
    );
    // Independently corroborated research label, never used for automatic identification.
    let fnv = bytes.iter().fold(2_166_136_261u32, |h, &b| {
        (h ^ u32::from(b)).wrapping_mul(16_777_619)
    });
    assert_eq!(fnv, 0x5E10_91E3);
    assert!(library::inspect(&source).unwrap_err().contains("TamaOtch"));
    assert!(
        library::inspect_bytes(bytes.clone(), "different-name.rom".into())
            .unwrap_err()
            .contains("LCD wiring")
    );
    let mut lib = Library::open(root)?;
    for (profile, name) in [
        (DeviceProfile::P1, "TamagotchiP1Japanese.bin"),
        (DeviceProfile::Genjintch, "TamagotchiGenjintch.bin"),
        (DeviceProfile::Angel, "TamagotchiAngel.bin"),
        (DeviceProfile::Morino, "TamagotchiMorino.bin"),
        (DeviceProfile::Umino, "TamagotchiUmino.bin"),
    ] {
        let prepared = library::inspect(&folder.join(name))?;
        let a = lib
            .automatic(&prepared)
            .ok_or("Expected the existing verified profile")?;
        assert_eq!(a.profile, profile);
        let entry = lib.import(&prepared, a)?;
        let image = roms::load_for_profile(&lib.verified_path(&entry.key)?, profile)?;
        let mut d = Tamagotchi::new(&image)?;
        d.advance_to(2 * MASTER_HZ)?;
        let _ = d.drain_audio();
        let state = d.save_state()?;
        let mut restored = d.restored(&state)?;
        let target = d.time() + MASTER_HZ / 2;
        d.advance_to(target)?;
        restored.advance_to(target)?;
        assert_eq!(d.drain_audio(), restored.drain_audio());
        assert_eq!(d.save_state()?, restored.save_state()?);
        println!(
            "{}: import, execution and deterministic save continuation passed",
            profile.name()
        );
    }
    let index = root.join(library::DIRECTORY).join("index.rhl");
    let before = std::fs::read(&index)?;
    for profile in DeviceProfile::ALL {
        let pending = Prepared {
            hash: hash.clone(),
            bytes: bytes.clone(),
            source_name: "renamed.rom".into(),
            candidates: vec![profile],
        };
        let a = Assignment {
            profile,
            variant: "Unverified".into(),
            evidence: "Attempted explicit override".into(),
        };
        assert!(lib.import(&pending, a).unwrap_err().contains("TamaOtch"));
        assert!(roms::load_for_profile(&source, profile).is_err());
    }
    assert_eq!(std::fs::read(index)?, before);
    assert_eq!(lib.entries().len(), 5);
    let c = Controller::new(false);
    c.send(Command::Load(folder.join("TamagotchiP1Japanese.bin")));
    wait(&c, |s| s.loaded && s.ticks > MASTER_HZ / 4);
    c.send(Command::Pause);
    let paused = wait(&c, |s| !s.running);
    c.send(Command::LoadProfile(source.clone(), DeviceProfile::Angel));
    let rejected = wait(&c, |s| s.error.is_some());
    assert!(rejected.error.as_ref().unwrap().contains("TamaOtch"));
    assert_eq!(rejected.state, paused.state);
    assert_eq!(rejected.display, paused.display);
    assert_eq!(rejected.ticks, paused.ticks);
    assert_eq!(rejected.rom_hash, paused.rom_hash);
    assert_eq!(rejected.profile, paused.profile);
    assert_eq!(rejected.loads, paused.loads);
    assert_eq!(std::fs::read(source)?, bytes);
    println!(
        "TamaOtch: 8192-word structure and independent research fingerprint corroborated; import/override rejected atomically; no firmware execution or playable-hardware claim"
    );
    Ok(())
}
