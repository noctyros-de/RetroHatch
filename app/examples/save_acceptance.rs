//! End-to-end local firmware save verification; no firmware is supplied.
//! Usage: save_acceptance ROM_FOLDER PRIVATE_OUTPUT_FOLDER
use retrohatch::core::{
    device::{Button, Device},
    mcu::MASTER_HZ,
    profile::DeviceProfile,
    state::{hex, sha256},
    tamagotchi::Tamagotchi,
};
use retrohatch::host::{
    roms,
    settings::{Settings, Theme},
    storage::Writer,
};
use std::{
    path::{Path, PathBuf},
    process::Command,
};
fn advance(d: &mut Tamagotchi, seconds: f64) -> Result<Vec<i16>, Box<dyn std::error::Error>> {
    let target = d.time() + (seconds * MASTER_HZ as f64) as u64;
    let mut samples = Vec::new();
    while d.time() < target {
        d.advance_to((d.time() + MASTER_HZ / 200).min(target))?;
        samples.extend(d.drain_audio());
    }
    Ok(samples)
}
fn continuation(d: &mut Tamagotchi) -> Result<String, Box<dyn std::error::Error>> {
    let samples = advance(d, 0.5)?;
    let pcm: Vec<u8> = samples.iter().flat_map(|v| v.to_le_bytes()).collect();
    Ok(format!(
        "{} {} {} {}",
        hex(&sha256(&d.save_state()?)),
        hex(&sha256(&pcm)),
        d.time(),
        d.mcu.interrupts
    ))
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.first().is_some_and(|s| s == "--resume") {
        let profile = DeviceProfile::parse(args[1].to_str().ok_or("Invalid profile")?)
            .ok_or("Invalid profile")?;
        let rom = roms::load_for_profile(Path::new(&args[2]), profile)?;
        let fresh = Tamagotchi::new(&rom)?;
        let bytes = std::fs::read(&args[3])?;
        let mut d = fresh.restored(&bytes)?;
        assert_eq!(
            d.save_state()?,
            bytes,
            "Initial full state differs after process restart"
        );
        println!("{}", continuation(&mut d)?);
        return Ok(());
    }
    if args.len() == 3 && args[2] == "--worker-check" {
        return worker_check(Path::new(&args[0]), Path::new(&args[1]));
    }
    if args.len() != 2 {
        return Err("Provide a ROM folder and private output folder".into());
    }
    let folder = PathBuf::from(&args[0]);
    let output = PathBuf::from(&args[1]);
    std::fs::create_dir_all(&output)?;
    for (profile, name) in [
        (DeviceProfile::P1, "TamagotchiP1Japanese.bin"),
        (DeviceProfile::Genjintch, "TamagotchiGenjintch.bin"),
        (DeviceProfile::Angel, "TamagotchiAngel.bin"),
        (DeviceProfile::Morino, "TamagotchiMorino.bin"),
    ] {
        let path = folder.join(name);
        let rom = roms::load_for_profile(&path, profile)?;
        let mut d = Tamagotchi::new(&rom)?;
        advance(&mut d, 5.0)?;
        for b in [Button::B, Button::A, Button::C] {
            d.set_button(b, true);
            advance(&mut d, 1.0)?;
            d.set_button(b, false);
            advance(&mut d, 1.0)?;
        }
        if profile == DeviceProfile::Morino {
            // Confirm the real firmware's egg-selection screen before waiting for hatching.
            d.set_button(Button::B, true);
            advance(&mut d, 1.0)?;
            d.set_button(Button::B, false);
        }
        advance(&mut d, 350.0)?;
        let bytes = d.save_state()?;
        let identity = d.identity();
        let dir = output.join("saves").join(&identity);
        let mut writer = Writer::new();
        for file in [
            "autosave.rhstate",
            "slot1.rhstate",
            "slot2.rhstate",
            "slot3.rhstate",
        ] {
            writer.write(dir.join(file), bytes.clone())?;
        }
        writer.finish();
        for done in writer.poll() {
            done.result?;
        }
        let before = d.time();
        let expected = continuation(&mut d)?;
        let result = Command::new(std::env::current_exe()?)
            .arg("--resume")
            .arg(profile.id())
            .arg(&path)
            .arg(dir.join("slot2.rhstate"))
            .output()?;
        if !result.status.success() {
            return Err(String::from_utf8_lossy(&result.stderr).to_string().into());
        }
        assert_eq!(
            String::from_utf8(result.stdout)?.trim(),
            expected,
            "Future CPU/IRQ/PCM differs after actual process restart"
        );
        println!(
            "{}: gameplay state at {:.6} s, {} bytes, all three slots + autosave written; initial bytes and future CPU/RAM/LCD/timer/IRQ/PCM match across process restart; identity={identity}",
            profile.name(),
            before as f64 / MASTER_HZ as f64,
            bytes.len()
        );
    }
    let config = Settings {
        theme: Theme::Lilac,
        labels: false,
        muted: true,
        volume: 37,
        autosave_minutes: 1,
        save_on_exit: true,
        diagnostics: false,
        last_rom: Some(folder.join("TamagotchiAngel.bin")),
        last_profile: DeviceProfile::Angel,
        rom_folder: Some(folder),
    };
    config.store(&output)?;
    assert_eq!(Settings::load(&output)?, config);
    println!(
        "Persistent Lilac theme, hidden labels, mute, volume 37, autosave 1 minute and Save on Exit seeded and verified for native restart testing."
    );
    Ok(())
}

fn worker_check(folder: &Path, root: &Path) -> Result<(), Box<dyn std::error::Error>> {
    use retrohatch::host::{
        runner::{Command as Op, Controller, Snapshot},
        saves::{SavePolicy, SaveTarget},
    };
    use std::time::{Duration, Instant};
    fn wait(c: &Controller, f: impl Fn(&Snapshot) -> bool) -> Snapshot {
        let end = Instant::now() + Duration::from_secs(8);
        loop {
            let s = c.snapshot();
            assert!(s.save_error.is_none(), "{:?}", s.save_error);
            assert!(s.error.is_none(), "{:?}", s.error);
            if f(&s) {
                return s;
            }
            assert!(Instant::now() < end, "Worker save operation timed out");
            std::thread::sleep(Duration::from_millis(5));
        }
    }
    for (profile, name) in [
        (DeviceProfile::P1, "TamagotchiP1Japanese.bin"),
        (DeviceProfile::Genjintch, "TamagotchiGenjintch.bin"),
        (DeviceProfile::Angel, "TamagotchiAngel.bin"),
        (DeviceProfile::Morino, "TamagotchiMorino.bin"),
    ] {
        let rom = roms::load_for_profile(&folder.join(name), profile)?;
        let fresh = Tamagotchi::new(&rom)?;
        let auto = root
            .join("saves")
            .join(fresh.identity())
            .join("autosave.rhstate");
        let auto_before = std::fs::read(&auto)?;
        let c = Controller::with_storage(
            false,
            root.to_path_buf(),
            SavePolicy {
                autosave_minutes: 0,
                save_on_exit: false,
                volume: 37,
            },
        );
        c.send(Op::LoadProfile(folder.join(name), profile));
        wait(&c, |s| s.restores >= 1 && !s.loading);
        c.send(Op::Pause);
        let paused = wait(&c, |s| !s.running);
        for slot in 1..=3 {
            c.send(Op::Save(SaveTarget::Slot(slot)));
            wait(&c, |s| s.save_count >= u64::from(slot));
        }
        let export = root.join(format!("{}-export.rhstate", profile.id()));
        c.send(Op::Export(export.clone()));
        wait(&c, |s| s.save_count >= 4);
        let exact = std::fs::read(&export)?;
        c.send(Op::Reset);
        wait(&c, |s| s.running && s.ticks > paused.ticks);
        c.send(Op::Pause);
        wait(&c, |s| !s.running);
        c.send(Op::LoadSave(SaveTarget::Slot(3)));
        let loaded = wait(&c, |s| s.restores >= 2);
        assert_eq!(loaded.ticks, paused.ticks);
        assert_eq!(loaded.state, paused.state);
        let after = root.join(format!("{}-loaded.rhstate", profile.id()));
        c.send(Op::Export(after.clone()));
        wait(&c, |s| s.save_count >= 5);
        assert_eq!(std::fs::read(after)?, exact);
        c.send(Op::Import(export));
        wait(&c, |s| s.restores >= 3);
        c.send(Op::PrepareExit);
        wait(&c, |s| s.exit_ready);
        drop(c);
        assert_eq!(
            std::fs::read(auto)?,
            auto_before,
            "Save on Exit=Off changed autosave"
        );
        println!(
            "{}: real gameplay auto-resume, all three worker slots, full export/import, manual restore and Save on Exit=Off passed.",
            profile.name()
        );
    }
    Ok(())
}
