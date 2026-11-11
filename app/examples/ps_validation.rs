// SPDX-License-Identifier: GPL-3.0-only
// SPDX-FileCopyrightText: 2026 Noctyros
//! Opt-in local S1C33 firmware, wall-clock and save/reopen validation; writes only a private validation folder.
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
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
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
            return Err("Validation worker timed out".into());
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
        .unwrap_or(DeviceProfile::Ps);
    if profile.s1c33_key().is_none() {
        return Err("Select a supported S1C33 profile".into());
    }
    let path = PathBuf::from(
        env::args()
            .nth(1)
            .ok_or("Usage: ps_validation ROM [real-time seconds]")?,
    );
    let duration: f64 = env::args()
        .nth(2)
        .and_then(|v| v.parse().ok())
        .unwrap_or(60.0);
    let prepared = inspect(&path)?;
    if !prepared.candidates.contains(&profile) {
        return Err("Firmware is incompatible with the selected S1C33 profile".into());
    }
    let original_hash = sha256(&fs::read(&path).map_err(|e| e.to_string())?);
    let root = env::temp_dir().join(format!(
        "retrohatch-{}-validation-{}",
        profile.id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&root).map_err(|e| e.to_string())?;
    println!("Private validation root: {}", root.display());
    let mut library = Library::open(&root)?;
    let entry = library.import(
        &prepared,
        Assignment {
            profile,
            variant: String::new(),
            evidence: "User-provided S1C33 firmware; local validation".into(),
        },
    )?;
    drop(library);
    let library = Library::open(&root)?;
    let internal = library.verified_path(&entry.key)?;
    let policy = SavePolicy {
        save_on_exit: true,
        ..SavePolicy::default()
    };
    let c = Controller::with_storage(true, root.clone(), policy.clone());
    c.send(Command::LoadProfile(internal.clone(), profile));
    wait(&c, |s| s.loaded && !s.loading && s.ticks > 0)?;
    let start = Instant::now();
    let initial = c.snapshot().ticks;
    let mut pressed = false;
    let mut released = false;
    let mut next = 10.0;
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
    let export = root.join("gameplay.rhstate");
    c.send(Command::Save(SaveTarget::Slot(1)));
    wait(&c, |s| s.slots[0])?;
    c.send(Command::LoadSave(SaveTarget::Slot(1)));
    wait(&c, |s| s.restores > 0)?;
    let count = c.snapshot().save_count;
    c.send(Command::Export(export.clone()));
    wait(&c, |s| s.save_count > count)?;
    let before = c.snapshot().restores;
    c.send(Command::Import(export));
    wait(&c, |s| s.restores > before)?;
    c.send(Command::PrepareExit);
    wait(&c, |s| s.exit_ready)?;
    drop(c);
    let again = Controller::with_storage(false, root, policy);
    again.send(Command::LoadProfile(internal, profile));
    let resumed = wait(&again, |s| s.restores > 0 && !s.loading)?;
    if resumed.ticks < paused.ticks {
        return Err("Restart lost emulated progress".into());
    }
    if resumed.color_display != paused.color_display {
        return Err("Restart lost display state".into());
    }
    again.send(Command::PrepareExit);
    wait(&again, |s| s.exit_ready)?;
    drop(again);
    if sha256(&fs::read(path).map_err(|e| e.to_string())?) != original_hash {
        return Err("Original firmware changed".into());
    }
    println!(
        "Verified library persistence, Slot 1, export/import, Save on Exit and fresh-worker autosave resume; original firmware unchanged."
    );
    Ok(())
}
