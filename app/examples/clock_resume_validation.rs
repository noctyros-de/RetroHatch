// SPDX-License-Identifier: GPL-3.0-only
// SPDX-FileCopyrightText: 2026 Noctyros
//! Opt-in real TG18 firmware/save/UTC validation. Outputs must remain private.
//! Usage: clock_resume_validation ROM CHECKPOINT NEW_PRIVATE_OUTPUT
use retrohatch::{
    core::{device::Button, mcu::MASTER_HZ, profile::DeviceProfile, state::sha256},
    host::{
        emulator::Emulator,
        resume_clock::unix_millis,
        runner::Snapshot,
        saves::{SavePolicy, SaveTarget, Saves},
    },
};
use std::{
    fs,
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};
type Error = Box<dyn std::error::Error>;
fn machine(d: &Emulator, image: &[u8]) -> Result<tg18::Machine, Error> {
    let data = d.save_state()?;
    let mut m = tg18::Machine::new(Arc::new(image.to_vec()), image, 0.0);
    tg18::snapshot::Snapshot::from_bytes(&data[81..data.len() - 32])
        .map_err(|e| e.to_string())?
        .restore(&mut m)
        .map_err(|e| e.to_string())?;
    Ok(m)
}
fn flush(
    s: &mut Saves,
    d: &mut Option<Emulator>,
    state: &mut Snapshot,
    running: &mut bool,
) -> Result<(), Error> {
    let end = Instant::now() + Duration::from_secs(20);
    while s.busy() {
        s.poll(d, state, running);
        if Instant::now() > end {
            return Err("Save worker timed out".into());
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    if let Some(e) = state.save_error.take() {
        return Err(e.into());
    }
    Ok(())
}
fn show_clock(d: &mut Emulator, out: &std::path::Path) -> Result<(), Error> {
    d.set_keep_screen_awake(false);
    // Let any deferred refresh reach the reference core's normal idle boundary.
    let end = d.time() + MASTER_HZ;
    while d.time() < end {
        d.advance_to((d.time() + MASTER_HZ / 100).min(end))?;
        d.drain_audio();
    }
    for attempt in 0..3 {
        d.set_button(Button::B, true);
        d.advance_to(d.time() + MASTER_HZ / 5)?;
        d.drain_audio();
        d.set_button(Button::B, false);
        let end = d.time() + 3 * MASTER_HZ;
        while d.time() < end {
            d.advance_to((d.time() + MASTER_HZ / 100).min(end))?;
            d.drain_audio();
        }
        let file = if attempt == 0 {
            out.to_path_buf()
        } else {
            out.with_file_name(format!(
                "{}-attempt{}.png",
                out.file_stem().unwrap().to_string_lossy(),
                attempt + 1
            ))
        };
        fs::write(file.with_extension("rhstate"), d.save_state()?)?;
        fs::write(
            file,
            tg18::png::encode(128, 128, &d.color_display().ok_or("No LCD")?, 1),
        )?;
    }
    Ok(())
}
fn main() -> Result<(), Error> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.len() < 3 || args.len() > 4 {
        return Err("Provide ROM, diagnostic checkpoint and new private output".into());
    }
    let out = PathBuf::from(&args[2]);
    if out.exists() {
        return Err("Output exists; use a new disposable directory".into());
    }
    if std::env::var_os("RETROHATCH_DATA_DIR").map(PathBuf::from) != Some(out.clone()) {
        return Err("Set RETROHATCH_DATA_DIR to the new isolated output directory".into());
    }
    let parent = out
        .parent()
        .ok_or("Missing output parent")?
        .canonicalize()?;
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .canonicalize()?;
    if parent.starts_with(repo) || parent.ancestors().any(|p| p.join(".git").exists()) {
        return Err("Keep firmware diagnostics outside Git repositories".into());
    }
    if let Some(local) = std::env::var_os("LOCALAPPDATA") {
        let protected = PathBuf::from(local).join("RetroHatch");
        let normalized = parent.to_string_lossy().replace('\\', "/").to_lowercase();
        let normalized = normalized.strip_prefix("//?/").unwrap_or(&normalized);
        let protected = protected
            .to_string_lossy()
            .replace('\\', "/")
            .to_lowercase();
        if normalized == protected || normalized.starts_with(&(protected + "/")) {
            return Err("Personal application data is never a test directory".into());
        }
    }
    fs::create_dir(&out)?;
    let image = fs::read(&args[0])?;
    let checkpoint = fs::read(&args[1])?;
    let rom_hash = sha256(&image);
    let checkpoint_hash = sha256(&checkpoint);
    let known = tg18::roms::identify(&image).ok_or("Unknown firmware")?;
    let profile = if known.region == "en" {
        DeviceProfile::On
    } else {
        DeviceProfile::parse(&format!(
            "meets-{}",
            if known.slug == "magic" {
                "magical"
            } else {
                known.slug
            }
        ))
        .ok_or("Unknown edition")?
    };
    let seed = Emulator::from_bytes(&image, profile)?.restored(&checkpoint)?;
    if !seed.supports_clock_resume() {
        return Err(
            "Clock sync is not enabled for this firmware; use clock_resume_probe for research"
                .into(),
        );
    }
    let before = machine(&seed, &image)?;
    let initial_ticks = before.sys.rtc.ticks(before.sys.game_time);
    let initial_tod = before.sys.rtc_irq.tod(before.sys.game_time);
    let base = unix_millis();
    let wall = Arc::new(AtomicU64::new(base));
    let make = || {
        let wall = wall.clone();
        Saves::with_clock(
            out.clone(),
            SavePolicy::default(),
            Arc::new(move || wall.load(Ordering::SeqCst)),
        )
    };
    let mut state = Snapshot::default();
    let mut running = false;
    let mut device = Some(seed);
    let mut saves = make();
    saves.save(device.as_ref().unwrap(), SaveTarget::Autosave)?;
    for slot in 1..=3 {
        saves.save(device.as_ref().unwrap(), SaveTarget::Slot(slot))?;
    }
    saves.export(device.as_ref().unwrap(), out.join("export.rhstate"))?;
    flush(&mut saves, &mut device, &mut state, &mut running)?;
    saves.finish();
    drop(saves);
    drop(device);
    // A full restart with a controlled UTC jump; Windows time is unchanged.
    let elapsed: u64 = args
        .get(3)
        .map(|v| {
            v.to_str()
                .ok_or("Invalid duration")?
                .parse::<u64>()
                .map_err(|_| "Invalid duration")
        })
        .transpose()?
        .unwrap_or(25 * 3_600_000 + 125);
    wall.store(base + elapsed, Ordering::SeqCst);
    let mut device = Some(Emulator::from_bytes(&image, profile)?);
    let mut saves = make();
    saves.start(device.as_ref().unwrap(), &mut state)?;
    flush(&mut saves, &mut device, &mut state, &mut running)?;
    let corrected = machine(device.as_ref().unwrap(), &image)?;
    let expected = initial_ticks + (elapsed * 32768 / 1000) as i64;
    assert_eq!(corrected.sys.rtc.ticks(corrected.sys.game_time), expected);
    assert_eq!(
        corrected.sys.rtc_irq.tod(corrected.sys.game_time),
        ((before.sys.rtc_irq.tod_base as f64 + before.sys.game_time - before.sys.rtc_irq.tod_set_at
            + elapsed as f64 / 1000.0) as i64)
            .rem_euclid(86400)
    );
    assert_eq!(corrected.executed, before.executed);
    assert_eq!(corrected.cpu.state(), before.cpu.state());
    assert_eq!(corrected.irqs, before.irqs);
    assert_eq!(corrected.sys.game_time, before.sys.game_time);
    assert_eq!(corrected.sys.ram, before.sys.ram);
    assert_eq!(corrected.sys.sram, before.sys.sram);
    fs::write(
        out.join("resumed.rhstate"),
        device.as_ref().unwrap().save_state()?,
    )?;
    let mut visible = device
        .as_ref()
        .unwrap()
        .restored(&device.as_ref().unwrap().save_state()?)?;
    show_clock(&mut visible, &out.join("resumed-clock.png"))?;
    let mut control = Emulator::from_bytes(&image, profile)?.restored(&checkpoint)?;
    show_clock(&mut control, &out.join("disabled-clock.png"))?;
    // Reopening the same immutable save twice must not add elapsed time twice.
    saves.finish();
    drop(saves);
    let mut saves = make();
    saves.start(device.as_ref().unwrap(), &mut state)?;
    flush(&mut saves, &mut device, &mut state, &mut running)?;
    assert_eq!(
        machine(device.as_ref().unwrap(), &image)?
            .sys
            .rtc
            .ticks(before.sys.game_time),
        expected
    );
    // Save-on-exit anchors corrected time at the new UTC timestamp.
    saves.exit(device.as_ref())?;
    flush(&mut saves, &mut device, &mut state, &mut running)?;
    saves.finish();
    drop(saves);
    let mut saves = make();
    saves.start(device.as_ref().unwrap(), &mut state)?;
    flush(&mut saves, &mut device, &mut state, &mut running)?;
    assert_eq!(
        machine(device.as_ref().unwrap(), &image)?
            .sys
            .rtc
            .ticks(before.sys.game_time),
        expected
    );
    // Manual slots/import are exact even with synchronization enabled.
    for slot in 1..=3 {
        let path = saves
            .directory(device.as_ref().unwrap())
            .join(format!("slot{slot}.rhstate"));
        saves.load(device.as_ref().unwrap(), path, false)?;
        flush(&mut saves, &mut device, &mut state, &mut running)?;
        assert_eq!(
            machine(device.as_ref().unwrap(), &image)?
                .sys
                .rtc
                .ticks(before.sys.game_time),
            initial_ticks
        );
    }
    saves.load(device.as_ref().unwrap(), out.join("export.rhstate"), false)?;
    flush(&mut saves, &mut device, &mut state, &mut running)?;
    assert_eq!(
        machine(device.as_ref().unwrap(), &image)?
            .sys
            .rtc
            .ticks(before.sys.game_time),
        initial_ticks
    );
    // Disabled automatic restore loads the original exported anchor without correction.
    saves.sync_clock_on_resume = false;
    saves.load(device.as_ref().unwrap(), out.join("export.rhstate"), true)?;
    flush(&mut saves, &mut device, &mut state, &mut running)?;
    assert_eq!(
        machine(device.as_ref().unwrap(), &image)?
            .sys
            .rtc
            .ticks(before.sys.game_time),
        initial_ticks
    );
    // Legacy checkpoints remain loadable and never infer a timestamp.
    saves.sync_clock_on_resume = true;
    fs::write(out.join("legacy.rhstate"), &checkpoint)?;
    saves.load(device.as_ref().unwrap(), out.join("legacy.rhstate"), true)?;
    flush(&mut saves, &mut device, &mut state, &mut running)?;
    assert_eq!(
        machine(device.as_ref().unwrap(), &image)?
            .sys
            .rtc
            .ticks(before.sys.game_time),
        initial_ticks
    );
    saves.finish();
    assert_eq!(rom_hash, sha256(&fs::read(&args[0])?));
    assert_eq!(checkpoint_hash, sha256(&fs::read(&args[1])?));
    let report = format!(
        "{} UTC_delta_ms={elapsed} RTC_delta_ticks={} TOD_before={initial_tod} TOD_after={} immediate_CPU_RAM_time_IRQ_unchanged=true repeated_resume=true exit_restart=true slots=3 import_export=exact disabled=frozen legacy=frozen original_files_unchanged=true\n",
        known.display_name(),
        expected - initial_ticks,
        corrected.sys.rtc_irq.tod(corrected.sys.game_time)
    );
    fs::write(out.join("result.txt"), &report)?;
    println!("{report}");
    Ok(())
}
