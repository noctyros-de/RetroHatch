// SPDX-License-Identifier: GPL-3.0-only
// SPDX-FileCopyrightText: 2026 Noctyros
//! Opt-in real-firmware timeout validation. Usage: ROM NEW_PRIVATE_OUTPUT RESUME_STATE.
use retrohatch::{
    core::{
        device::Button,
        mcu::MASTER_HZ,
        profile::DeviceProfile,
        state::{hex, sha256},
    },
    host::emulator::Emulator,
};
use std::{fs, io::Read, path::PathBuf, sync::Arc, time::Instant};

fn machine(d: &Emulator, image: &[u8]) -> Result<tg18::Machine, Box<dyn std::error::Error>> {
    let state = d.save_state()?;
    let mut m = tg18::Machine::new(Arc::new(image.to_vec()), image, 0.0);
    tg18::snapshot::Snapshot::from_bytes(&state[81..state.len() - 32])
        .map_err(|e| e.to_string())?
        .restore(&mut m)
        .map_err(|e| e.to_string())?;
    Ok(m)
}
fn semantic_state(d: &Emulator) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let bytes = d.save_state()?;
    let mut raw = Vec::new();
    flate2::read::ZlibDecoder::new(&bytes[89..bytes.len() - 32]).read_to_end(&mut raw)?;
    let mut chunks = std::collections::BTreeMap::new();
    let mut at = 0;
    while at < raw.len() {
        let n = raw[at] as usize;
        at += 1;
        let name = std::str::from_utf8(&raw[at..at + n])?.to_owned();
        at += n;
        let n = u32::from_le_bytes(raw[at..at + 4].try_into()?) as usize;
        at += 4;
        let mut data = raw[at..at + n].to_vec();
        at += n;
        if name == "info" {
            data.truncate(8);
        }
        if name == "extra" {
            let mut pages: Vec<_> = data.chunks_exact(65540).collect();
            pages.sort();
            data = pages.concat();
        }
        chunks.insert(name, data);
    }
    let mut out = bytes[..45].to_vec();
    out.extend_from_slice(&bytes[49..81]);
    for (name, data) in chunks {
        out.extend(name.as_bytes());
        out.extend(data);
    }
    Ok(out)
}
fn advance(d: &mut Emulator, seconds: f64) -> Result<(), String> {
    let end = d.time() + (seconds * MASTER_HZ as f64) as u64;
    while d.time() < end {
        d.advance_to((d.time() + MASTER_HZ / 100).min(end))?;
        d.drain_audio();
    }
    Ok(())
}
fn sleep_after(d: &mut Emulator) -> Result<usize, String> {
    for second in 1..=120 {
        advance(d, 1.0)?;
        if d.sleeping() {
            return Ok(second);
        }
    }
    Err("Firmware did not enter its normal screen timeout within 120 seconds".into())
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.len() != 3 {
        return Err("Provide ROM, new private output directory and resume state".into());
    }
    let rom = PathBuf::from(&args[0]);
    let out = PathBuf::from(&args[1]);
    let public = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .canonicalize()?;
    if out.exists() {
        return Err("Output exists; preserve previous evidence".into());
    }
    fs::create_dir_all(&out)?;
    if out.canonicalize()?.starts_with(public) {
        return Err("Use a private output outside the repository".into());
    }
    let image = fs::read(&rom)?;
    let hash = sha256(&image);
    let known = tg18::roms::identify(&image).ok_or("Unknown TG18 firmware")?;
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
    let base = Emulator::from_bytes(&image, profile)?;
    let mut seed = base.restored(&fs::read(&args[2])?)?;
    seed.set_keep_screen_awake(false);
    seed.set_button(Button::B, true);
    advance(&mut seed, 0.08)?;
    seed.set_button(Button::B, false);
    advance(&mut seed, 3.0)?;
    assert!(!seed.sleeping(), "User wake did not display the LCD");
    let idle = machine(&seed, &image)?
        .idle_counter
        .ok_or("Reference idle-counter signature unavailable")?;
    let initial = seed.save_state()?;
    fs::write(out.join("initial.rhstate"), &initial)?;
    let mut normal = base.restored(&initial)?;
    let timeout = sleep_after(&mut normal)?;
    fs::write(
        out.join("normal.png"),
        tg18::png::encode(128, 128, &normal.color_display().unwrap(), 1),
    )?;
    let mut awake = base.restored(&initial)?;
    awake.set_keep_screen_awake(true);
    let before = machine(&awake, &image)?;
    let start = Instant::now();
    let mut frames = std::collections::HashSet::new();
    for _ in 0..180 {
        advance(&mut awake, 1.0)?;
        assert!(
            !awake.sleeping(),
            "Screen slept with Keep Screen Awake enabled"
        );
        let rgb = awake.color_display().unwrap();
        assert!(rgb.iter().any(|v| *v != 0), "Blank active framebuffer");
        frames.insert(sha256(&rgb));
    }
    let after = machine(&awake, &image)?;
    assert!((after.sys.game_time - before.sys.game_time - 180.0).abs() < 0.001);
    assert!(
        (after.sys.rtc.ticks(after.sys.game_time)
            - before.sys.rtc.ticks(before.sys.game_time)
            - 180 * 32768)
            .abs()
            < 40
    );
    assert!(after.irqs > before.irqs && after.executed > before.executed && frames.len() > 1);
    assert_eq!(
        after.sys.key_script, before.sys.key_script,
        "Keep awake injected input"
    );
    assert_eq!(after.sys.live_keys, [false; 3]);
    fs::write(
        out.join("awake.png"),
        tg18::png::encode(128, 128, &awake.color_display().unwrap(), 1),
    )?;
    let saved = awake.save_state()?;
    fs::write(out.join("awake.rhstate"), &saved)?;
    let mut restored = base.restored(&saved)?;
    advance(&mut awake, 0.37)?;
    advance(&mut restored, 0.37)?;
    if semantic_state(&awake)? != semantic_state(&restored)? {
        fs::write(out.join("continued.rhstate"), awake.save_state()?)?;
        fs::write(
            out.join("restored-continued.rhstate"),
            restored.save_state()?,
        )?;
        return Err(
            "Save continuation changed emulated fields; private checkpoints retained".into(),
        );
    }
    let before_input = awake.color_display().unwrap();
    awake.set_button(Button::A, true);
    advance(&mut awake, 0.08)?;
    awake.set_button(Button::A, false);
    advance(&mut awake, 0.5)?;
    assert_ne!(
        awake.color_display().unwrap(),
        before_input,
        "A did not change the live display"
    );
    awake.set_keep_screen_awake(false);
    let restored_timeout = sleep_after(&mut awake)?;
    let wake_keys = machine(&awake, &image)?.sys.key_script;
    awake.set_keep_screen_awake(true);
    let mut first_frame = None;
    for second in 1..=180 {
        advance(&mut awake, 1.0)?;
        assert!(
            !awake.sleeping(),
            "Enabling while asleep did not remain awake"
        );
        let visible = awake.color_display().unwrap().iter().any(|&v| v != 0);
        if visible && first_frame.is_none() {
            first_frame = Some(second);
        }
        // A real deep wake cold-boots the controller. Allow bounded firmware
        // initialization, then require continuously live LCD data, not a cached frame.
        if second >= 5 {
            assert!(visible, "LCD stayed blank after cold-wake initialization");
        }
    }
    assert_eq!(
        machine(&awake, &image)?.sys.key_script,
        wake_keys,
        "Wake injected input"
    );
    assert_eq!(sha256(&fs::read(&rom)?), hash);
    let result = format!(
        "{} code={:016X} sha256={} idle_counter={idle:08X} normal_timeout={timeout}s awake=180s distinct_frames={} irq_delta={} rtc_delta={} disabled_timeout={restored_timeout}s save_continuation=exact buttons=live wake_without_keys=true sleep_to_awake=180s first_frame={first_frame:?}s wall={:.3}s\n",
        known.display_name(),
        tg18::code_hash(&image),
        hex(&hash),
        frames.len(),
        after.irqs - before.irqs,
        after.sys.rtc.ticks(after.sys.game_time) - before.sys.rtc.ticks(before.sys.game_time),
        start.elapsed().as_secs_f64()
    );
    fs::write(out.join("result.txt"), &result)?;
    print!("{result}");
    Ok(())
}
