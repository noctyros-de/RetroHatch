// SPDX-License-Identifier: GPL-3.0-only
// SPDX-FileCopyrightText: 2026 Noctyros
//! Opt-in local Meets firmware, wall-clock and save/reopen validation; writes only a private validation folder.
use retrohatch::{
    core::{device::Button, mcu::MASTER_HZ, profile::DeviceProfile, state::sha256},
    host::{
        fingerprints::Assignment,
        library::{Library, inspect},
        runner::{Command, Controller, Snapshot},
        saves::{SavePolicy, SaveTarget},
    },
};
use std::{
    env, fs,
    path::PathBuf,
    thread,
    time::{Duration, Instant},
};
fn wait(c: &Controller, condition: impl Fn(&Snapshot) -> bool) -> Result<Snapshot, String> {
    let start = Instant::now();
    loop {
        let s = c.snapshot();
        if let Some(e) = &s.error {
            return Err(e.clone());
        }
        if let Some(e) = &s.save_error {
            return Err(e.clone());
        }
        if condition(&s) {
            return Ok(s);
        }
        if start.elapsed() > Duration::from_secs(25) {
            return Err(format!(
                "Validation worker timed out: ticks={} pc={:?} running={} saves={} restores={}",
                s.ticks, s.arm_pc, s.running, s.save_count, s.restores
            ));
        }
        thread::sleep(Duration::from_millis(10));
    }
}
fn main() -> Result<(), String> {
    let args: Vec<_> = env::args().collect();
    let profile = args
        .windows(2)
        .find(|v| v[0] == "--profile")
        .and_then(|v| DeviceProfile::parse(&v[1]))
        .unwrap_or(DeviceProfile::MeetsFairy);
    if !profile.is_meets() {
        return Err("Select a Meets edition".into());
    }
    let path = PathBuf::from(
        env::args()
            .nth(1)
            .ok_or("Usage: meets_validation ROM [real-time seconds]")?,
    );
    let duration: f64 = env::args()
        .nth(2)
        .and_then(|v| v.parse().ok())
        .unwrap_or(60.0);
    let prepared = inspect(&path)?;
    if !prepared.candidates.contains(&profile) {
        return Err("Firmware is incompatible with the selected Meets profile".into());
    }
    let original_hash = sha256(&fs::read(&path).map_err(|e| e.to_string())?);
    let root = PathBuf::from(
        args.windows(2)
            .find(|v| v[0] == "--root")
            .ok_or("--root PRIVATE_DIRECTORY is required")?[1]
            .clone(),
    );
    if root.exists() {
        return Err("Use a new private output directory".into());
    }
    fs::create_dir_all(&root).map_err(|e| e.to_string())?;
    println!("Private validation root: {}", root.display());
    let mut library = Library::open(&root)?;
    let detected = library
        .automatic(&prepared)
        .ok_or("Firmware was not identified automatically")?;
    if detected.profile != profile {
        return Err("Detected the wrong edition".into());
    }
    let entry = library.import(
        &prepared,
        Assignment {
            profile,
            variant: String::new(),
            evidence: "User-provided Meets firmware; local validation".into(),
        },
    )?;
    drop(library);
    let library = Library::open(&root)?;
    let internal = library.verified_path(&entry.key)?;
    let policy = SavePolicy {
        save_on_exit: true,
        ..SavePolicy::default()
    };
    let resume = PathBuf::from(
        args.windows(2)
            .find(|v| v[0] == "--resume")
            .ok_or("--resume PRIVATE_STATE is required")?[1]
            .clone(),
    );
    let generic =
        retrohatch::host::emulator::Emulator::from_bytes(&prepared.bytes, DeviceProfile::On)?
            .restored(&fs::read(resume).map_err(|e| e.to_string())?)?;
    let legacy = generic.save_state()?;
    let old = root.join("saves").join(generic.identity());
    fs::create_dir_all(&old).map_err(|e| e.to_string())?;
    for name in [
        "autosave.rhstate",
        "slot1.rhstate",
        "slot2.rhstate",
        "slot3.rhstate",
    ] {
        fs::write(old.join(name), &legacy).map_err(|e| e.to_string())?;
    }
    let keep_awake = args.iter().any(|v| v == "--keep-awake");
    let c = Controller::with_storage(true, root.clone(), policy.clone());
    c.send(Command::KeepScreenAwake(keep_awake));
    c.send(Command::LoadProfile(internal.clone(), profile));
    wait(&c, |s| {
        s.loaded && !s.loading && s.ticks > 0 && s.restores > 0
    })?;
    if c.snapshot().slots != [true; 3] {
        return Err("Legacy slots were not discovered".into());
    }
    let start = Instant::now();
    let initial = c.snapshot().ticks;
    let mut pressed = false;
    let mut released = false;
    let mut next = 10.0;
    let mut event = 0;
    while start.elapsed().as_secs_f64() < duration {
        let wall = start.elapsed().as_secs_f64();
        if wall >= 8.0 && !pressed {
            c.send(Command::Button(Button::B, true));
            pressed = true;
        }
        if wall >= 8.12 && !released {
            c.send(Command::Button(Button::B, false));
            released = true;
        }
        let keys = [Button::A, Button::B, Button::C];
        if event < 6 && wall >= 1.0 + (event / 2) as f64 + if event % 2 == 0 { 0.0 } else { 0.15 } {
            c.send(Command::Button(keys[event / 2], event % 2 == 0));
            event += 1;
        }
        let s = c.snapshot();
        if let Some(e) = s.error {
            return Err(e);
        }
        if wall >= next {
            println!(
                "wall={wall:.3} emulated={:.3} speed={:.2}% audio_drops={}",
                (s.ticks - initial) as f64 / MASTER_HZ as f64,
                100.0 * (s.ticks - initial) as f64 / MASTER_HZ as f64 / wall,
                s.audio_drops
            );
            next += 10.0;
        }
        thread::sleep(Duration::from_millis(10));
    }
    c.send(Command::Pause);
    let paused = wait(&c, |s| !s.running)?;
    println!(
        "final wall={:.3} emulated={:.3} audio={} drops={}",
        start.elapsed().as_secs_f64(),
        (paused.ticks - initial) as f64 / MASTER_HZ as f64,
        paused.audio_status,
        paused.audio_drops
    );
    println!("Audio queue: {:?}", paused.audio_stats);
    if keep_awake && paused.sleeping {
        return Err("Keep Screen Awake lost the live display".into());
    }
    if paused.audio_status != "Audio: Windows PCM output active" {
        return Err(format!(
            "Actual Windows playback failed: {}",
            paused.audio_status
        ));
    }
    if paused.audio_drops != 0 || paused.audio_stats.empty_queue_events != 0 {
        return Err("Audio dropped or underrun".into());
    }
    let export = root.join("gameplay.rhstate");
    for slot in 1..=3 {
        let count = c.snapshot().save_count;
        c.send(Command::Save(SaveTarget::Slot(slot)));
        wait(&c, |s| s.save_count > count)?;
        let before = c.snapshot().restores;
        c.send(Command::LoadSave(SaveTarget::Slot(slot)));
        wait(&c, |s| s.restores > before)?;
    }
    let count = c.snapshot().save_count;
    c.send(Command::Export(export.clone()));
    wait(&c, |s| s.save_count > count)?;
    let before = c.snapshot().restores;
    c.send(Command::Import(export));
    wait(&c, |s| s.restores > before)?;
    c.send(Command::PrepareExit);
    wait(&c, |s| s.exit_ready)?;
    drop(c);
    let again = Controller::with_storage(false, root.clone(), policy);
    again.send(Command::KeepScreenAwake(keep_awake));
    again.send(Command::LoadProfile(internal, profile));
    let resumed = wait(&again, |s| s.restores > 0 && !s.loading)?;
    if resumed.ticks < paused.ticks {
        return Err("Restart lost emulated progress".into());
    }
    if resumed.color_display != paused.color_display {
        return Err("Restart lost display state".into());
    }
    println!(
        "Verified all three slots, export/import, Save on Exit and fresh-worker display restoration."
    );
    if args.iter().any(|v| v == "--check-reset") {
        // Only this tool's isolated test device, after verifying its saved continuation.
        again.send(Command::Pause);
        let prior = wait(&again, |s| !s.running)?;
        again.send(Command::Reset);
        again.send(Command::Pause);
        wait(&again, |s| {
            !s.running && s.ticks >= prior.ticks && s.arm_pc == Some(0x20000048)
        })?;
        again.send(Command::Run);
        let running = wait(&again, |s| s.running && s.ticks > prior.ticks + MASTER_HZ)?;
        if running.profile != profile {
            return Err("Reset changed the model".into());
        }
        println!("Verified explicit pause, reset and resume on the isolated test device.");
    }
    again.send(Command::PrepareExit);
    wait(&again, |s| s.exit_ready)?;
    drop(again);
    if sha256(&fs::read(path).map_err(|e| e.to_string())?) != original_hash {
        return Err("Original firmware changed".into());
    }
    for name in [
        "autosave.rhstate",
        "slot1.rhstate",
        "slot2.rhstate",
        "slot3.rhstate",
    ] {
        if fs::read(old.join(name)).map_err(|e| e.to_string())? != legacy {
            return Err("Legacy save was modified".into());
        }
    }
    println!(
        "Verified library persistence, all three slots, export/import, Save on Exit and fresh-worker autosave resume; original firmware unchanged."
    );
    Ok(())
}
