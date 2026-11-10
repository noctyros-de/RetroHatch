// SPDX-License-Identifier: GPL-3.0-only
// SPDX-FileCopyrightText: 2026 Noktyros
//! Opt-in local PCM/WinMM diagnostics. No firmware or recordings are embedded.
use retrohatch::{
    core::{device::Device, mcu::MASTER_HZ, profile::DeviceProfile, tamagotchi::Tamagotchi},
    host::{
        roms,
        runner::{Command, Controller},
    },
};
use std::{
    env, fs,
    path::Path,
    thread,
    time::{Duration, Instant},
};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = env::args().collect();
    let path = args
        .get(1)
        .ok_or("Usage: audio_diagnostics ROM PROFILE PCM_OUTPUT [seconds]")?;
    let profile = DeviceProfile::parse(args.get(2).ok_or("Missing profile")?)
        .ok_or("Invalid classic profile")?;
    let output = args.get(3).ok_or("Missing PCM output path")?;
    if Path::new(output).extension().and_then(|v| v.to_str()) != Some("pcm") {
        return Err("Diagnostic output must use .pcm; ROM files are never overwritten".into());
    }
    if Path::new(output).exists() {
        return Err("Choose a new PCM output file; existing files are never overwritten".into());
    }
    let duration: f64 = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(15.0);
    let rom = roms::load_for_profile(Path::new(path), profile)?;
    let mut d = Tamagotchi::new(&rom)?;
    let target = (duration * MASTER_HZ as f64) as u64;
    let mut pcm = vec![];
    while d.time() < target {
        d.advance_to((d.time() + MASTER_HZ / 200).min(target))?;
        pcm.extend(d.drain_audio());
    }
    let audible = pcm.iter().filter(|&&v| v != 0).count();
    let clipped = pcm.iter().filter(|&&v| v.abs() == 655).count();
    let bytes: Vec<u8> = pcm.iter().flat_map(|v| v.to_le_bytes()).collect();
    fs::write(output, bytes)?;
    println!(
        "PCM samples={} audible={} at_source_ceiling={} max={}",
        pcm.len(),
        audible,
        clipped,
        pcm.iter().map(|v| v.abs()).max().unwrap_or(0)
    );
    let c = Controller::new(true);
    c.send(Command::LoadProfile(path.into(), profile));
    let start = Instant::now();
    while start.elapsed().as_secs_f64() < duration {
        let s = c.snapshot();
        if let Some(e) = s.error {
            return Err(e.into());
        }
        thread::sleep(Duration::from_millis(25));
    }
    let s = c.snapshot();
    println!(
        "wall={:.3} emulated={:.3} empty_queue_events={} writes={} accepted={} submitted={} pending={} max_gap_ms={:.3} drops={}",
        start.elapsed().as_secs_f64(),
        s.performance.emulated_seconds,
        s.audio_stats.empty_queue_events,
        s.audio_stats.writes,
        s.audio_stats.accepted,
        s.audio_stats.submitted,
        s.audio_stats.pending,
        s.audio_stats.max_write_gap_us as f64 / 1000.0,
        s.audio_drops
    );
    c.send(Command::Pause);
    Ok(())
}
