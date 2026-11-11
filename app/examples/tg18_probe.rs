// SPDX-License-Identifier: GPL-3.0-only
// SPDX-FileCopyrightText: 2026 Noctyros
//! Opt-in real TG18 firmware boot/input/state probe. Outputs remain private.
//! Usage: tg18_probe ROM NEW_PRIVATE_OUTPUT SECONDS [PROFILE] [INPUT_SCRIPT] [RESUME_STATE]
use retrohatch::{
    core::{
        device::Button,
        mcu::MASTER_HZ,
        profile::DeviceProfile,
        state::{hex, sha256},
    },
    host::emulator::Emulator,
};
use std::{fs, path::PathBuf, time::Instant};
// Compare every saved emulated field, excluding only the upstream wall-clock
// capture timestamp; unordered sparse zero pages are sorted by page address.
fn semantic_state(bytes: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    use std::io::Read;
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
            assert_eq!(data.len() % 65540, 0, "Malformed sparse pages");
            let mut pages: Vec<_> = data.chunks_exact(65540).map(|v| v.to_vec()).collect();
            pages.sort();
            data = pages.concat();
        }
        chunks.insert(name, data);
    }
    let mut out = bytes[..45].to_vec();
    out.extend_from_slice(&bytes[49..81]);
    for (name, data) in chunks {
        out.extend_from_slice(name.as_bytes());
        out.extend_from_slice(&(data.len() as u64).to_le_bytes());
        out.extend(data);
    }
    Ok(out)
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let a: Vec<_> = std::env::args_os().skip(1).collect();
    if a.len() < 3 {
        return Err("Provide ROM, new private output folder and duration".into());
    }
    let path = PathBuf::from(&a[0]);
    let out = PathBuf::from(&a[1]);
    if out.exists() {
        return Err("Output folder exists; use a new private directory".into());
    }
    fs::create_dir_all(&out)?;
    let duration: f64 = a[2].to_str().ok_or("Invalid duration")?.parse()?;
    if !duration.is_finite() || duration <= 0.0 {
        return Err("Invalid duration".into());
    }
    let profile = a
        .get(3)
        .and_then(|s| s.to_str())
        .and_then(DeviceProfile::parse)
        .unwrap_or(DeviceProfile::On);
    let bytes = fs::read(&path)?;
    let hash = sha256(&bytes);
    let known = tg18::roms::identify(&bytes).ok_or("No upstream TG18 code identity")?;
    println!(
        "model={} region={} version={} bytes={} sha256={} code_hash={:016X}",
        known.name,
        known.region,
        known.version,
        bytes.len(),
        hex(&hash),
        tg18::code_hash(&bytes)
    );
    let mut d = Emulator::from_bytes(&bytes, profile)?;
    if let Some(state) = a.get(5) {
        d = d.restored(&fs::read(state)?)?;
    }
    let initial = d.time();
    let mut events = Vec::new();
    if let Some(script) = a.get(4) {
        for line in fs::read_to_string(script)?
            .lines()
            .filter(|l| !l.trim().is_empty() && !l.starts_with('#'))
        {
            let f: Vec<_> = line.split_whitespace().collect();
            if f.len() != 3 {
                return Err("Expected seconds A|B|C 0|1".into());
            }
            let t: f64 = f[0].parse()?;
            let b = match f[1] {
                "A" => Button::A,
                "B" => Button::B,
                "C" => Button::C,
                _ => return Err("Invalid input".into()),
            };
            events.push(((t * MASTER_HZ as f64) as u64, b, f[2] == "1"));
        }
    }
    let start = Instant::now();
    let target = initial + (duration * MASTER_HZ as f64) as u64;
    let mut event = 0;
    let mut next = MASTER_HZ;
    let mut total = 0;
    let mut audible = 0;
    let mut peak = 0i32;
    let mut frames = std::collections::HashSet::new();
    let mut report = String::new();
    while d.time() < target {
        let elapsed = d.time() - initial;
        while event < events.len() && events[event].0 <= elapsed {
            d.set_button(events[event].1, events[event].2);
            event += 1;
        }
        let mut end = (d.time() + MASTER_HZ / 200).min(target);
        if event < events.len() {
            end = end.min(initial + events[event].0);
        }
        d.advance_to(end)?;
        let pcm = d.drain_audio();
        total += pcm.len();
        audible += pcm.iter().filter(|&&v| v != 0).count();
        peak = peak.max(pcm.iter().map(|&v| i32::from(v).abs()).max().unwrap_or(0));
        if d.time() - initial >= next || d.time() >= target {
            let sec = (d.time() - initial) as f64 / MASTER_HZ as f64;
            let rgb = d.color_display().ok_or("No color framebuffer")?;
            frames.insert(sha256(&rgb));
            if sec <= 10.0
                || d.time() >= target
                || events
                    .iter()
                    .any(|(t, _, _)| ((sec * MASTER_HZ as f64) as u64).abs_diff(*t) < MASTER_HZ)
            {
                fs::write(
                    out.join(format!("lcd-{sec:.3}.png")),
                    tg18::png::encode(128, 128, &rgb, 1),
                )?;
            }
            report.push_str(&format!(
                "t={sec:.6} pc={:?} instructions={} irqs={} sleeping={} lcd={}\n",
                d.arm_pc(),
                d.instructions(),
                d.interrupts(),
                d.sleeping(),
                hex(&sha256(&rgb))
            ));
            fs::write(out.join("progress.txt"), &report)?;
            next += MASTER_HZ;
        }
    }
    let wall = start.elapsed().as_secs_f64();
    let saved = d.save_state()?;
    fs::write(out.join("final.rhstate"), &saved)?;
    let mut restored = d.restored(&saved)?;
    let roundtrip = restored.save_state()?;
    fs::write(out.join("roundtrip.rhstate"), &roundtrip)?;
    assert!(
        semantic_state(&saved)? == semantic_state(&roundtrip)?,
        "Roundtrip changed emulated fields"
    );
    let future = d.time() + MASTER_HZ / 2;
    d.advance_to(future)?;
    restored.advance_to(future)?;
    assert!(
        d.drain_audio() == restored.drain_audio(),
        "PCM continuation differs"
    );
    assert!(
        semantic_state(&d.save_state()?)? == semantic_state(&restored.save_state()?)?,
        "Future emulated fields differ"
    );
    assert_eq!(sha256(&fs::read(path)?), hash);
    report.push_str(&format!("wall={wall:.6} seconds={duration} samples={total} audible={audible} peak={peak} distinct_frames={} input_transitions={event} save_bytes={} full_state_and_pcm_continuation=true\n",frames.len(),saved.len()));
    fs::write(out.join("report.txt"), &report)?;
    println!("{}", report.lines().last().unwrap());
    Ok(())
}
