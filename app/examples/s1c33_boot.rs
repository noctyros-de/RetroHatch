// SPDX-License-Identifier: GPL-3.0-only
// SPDX-FileCopyrightText: 2026 Noctyros
//! Local, opt-in firmware diagnostic. Never embeds or overwrites firmware.
use retrohatch::core::s1c33::{machine::Machine, profile::HardwareProfile};
use std::{env, fs, time::Instant};
fn hash(bytes: &[u8]) -> u32 {
    bytes
        .iter()
        .fold(2166136261u32, |h, b| (h ^ *b as u32).wrapping_mul(16777619))
}
fn main() -> Result<(), String> {
    let args: Vec<_> = env::args().collect();
    let image = fs::read(
        args.get(1)
            .ok_or("Usage: s1c33_boot ROM [seconds] [PPM output]")?,
    )
    .map_err(|e| e.to_string())?;
    let end = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(10.0);
    let mut m = Machine::new(&image, HardwareProfile::find("ps").unwrap())?;
    m.keep_screen_awake = args.iter().any(|v| v == "--keep-screen-awake");
    let wall = Instant::now();
    let mut next = 1.0;
    let mut edges: Vec<(u64, u8)> = args
        .get(4)
        .filter(|v| !v.is_empty() && !v.starts_with("--"))
        .map(|v| {
            v.split(',')
                .map(|edge| {
                    let (t, mask) = edge.split_once(':').expect("time:button-mask");
                    (
                        (t.parse::<f64>().unwrap() * 18432000.0) as u64,
                        mask.parse().unwrap(),
                    )
                })
                .collect()
        })
        .unwrap_or_default();
    edges.reverse();
    let mut pcm = Vec::<i16>::new();

    let elapsed_time = args.iter().any(|v| v == "--emulated-time");
    while if elapsed_time {
        m.seconds() < end
    } else {
        m.cpu.cycles < (end * 18432000.0) as u64
    } {
        if let Some(&(at, mask)) = edges.last() {
            if m.cpu.cycles >= at {
                m.buttons(mask);
                edges.pop();
            }
        }

        if let Err(e) = m.step() {
            eprintln!(
                "FAIL pc={:08x} cycles={} seconds={:.6} last_io={:08x} registers={:08x?}",
                m.cpu.pc,
                m.cpu.cycles,
                m.seconds(),
                m.memory.last_io_read,
                m.cpu.r
            );
            return Err(e.to_string());
        }
        if if elapsed_time {
            m.seconds() >= next
        } else {
            m.cpu.cycles >= (next * 18432000.0) as u64
        } {
            let p = m.memory.peripherals.as_ref().unwrap();
            println!(
                "t={:.3} wall={:.3} pc={:08x} cycles={} hz={:.0} irq={} lcd={} pixels={} colors={} tone_events={} flash={}/{}",
                m.seconds(),
                wall.elapsed().as_secs_f64(),
                m.cpu.pc,
                m.cpu.cycles,
                p.mclk,
                m.interrupts,
                m.memory.lcd.commands,
                m.memory.lcd.ramwr_bytes,
                m.rgb()
                    .chunks_exact(3)
                    .filter(|c| c[0] != 0 || c[1] != 0 || c[2] != 0)
                    .count(),
                p.tone_events,
                m.memory.flash_state.programs,
                m.memory.flash_state.erases
            );
            println!(
                "display sleep_out={} display_on={} awake_policy={}",
                m.memory.lcd.sleep_out, m.memory.lcd.display_on, m.keep_screen_awake
            );
            let gram: Vec<u8> = m
                .memory
                .lcd
                .gram
                .iter()
                .flat_map(|v| v.to_le_bytes())
                .collect();
            println!(
                "state ram={:08x} io={:08x} gram={:08x}",
                hash(&m.memory.a0ram),
                hash(&p.io),
                hash(&gram)
            );
            pcm.extend(m.drain_audio());
            next += 1.0;
        }
    }
    pcm.extend(m.drain_audio());
    if let Some(output) = args
        .get(5)
        .filter(|v| !v.is_empty() && !v.starts_with("--"))
    {
        let data: Vec<u8> = pcm.iter().flat_map(|v| v.to_le_bytes()).collect();
        fs::write(output, data).map_err(|e| e.to_string())?;
        println!(
            "audio samples={} audible={}",
            pcm.len(),
            pcm.iter().filter(|&&v| v != 0).count()
        );
    }
    // Capture the requested checkpoint before the continuation test presses A.
    let checkpoint_display = m.rgb();
    let hash = retrohatch::core::state::sha256(&image);
    let saved = m.save_state(hash)?;
    let mut restored = Machine::restored(&saved, hash)?;
    restored.keep_screen_awake = m.keep_screen_awake;
    if saved != restored.save_state(hash)? {
        return Err("Full snapshot did not round-trip".into());
    }
    for n in 0..10000 {
        if n == 100 {
            m.buttons(1);
            restored.buttons(1);
        }
        if n == 1000 {
            m.buttons(0);
            restored.buttons(0);
        }
        m.step().map_err(|e| e.to_string())?;
        restored.step().map_err(|e| e.to_string())?;
    }
    if m.save_state(hash)? != restored.save_state(hash)? {
        return Err("Restored execution diverged".into());
    }
    println!(
        "Full-state and 10000-step continuation match, including flash, LCD, peripherals and PCM."
    );
    if let Some(path) = args.get(3) {
        let mut ppm = b"P6\n128 128\n255\n".to_vec();
        ppm.extend(checkpoint_display);
        fs::write(path, ppm).map_err(|e| e.to_string())?;
    }
    Ok(())
}
