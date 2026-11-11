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
    let hardware = args
        .windows(2)
        .find(|v| v[0] == "--hardware")
        .map(|v| v[1].as_str())
        .unwrap_or("ps");
    let profile = HardwareProfile::find(hardware).ok_or("Unknown hardware profile")?;
    let mut m = if let Some(path) = args.windows(2).find(|v| v[0] == "--resume").map(|v| &v[1]) {
        Machine::restored(
            &fs::read(path).map_err(|e| e.to_string())?,
            retrohatch::core::state::sha256(&image),
        )?
    } else {
        Machine::new(&image, profile)?
    };
    if m.memory.profile != profile {
        return Err("Checkpoint uses a different hardware profile".into());
    }
    m.keep_screen_awake = profile.has_sleep_flag && args.iter().any(|v| v == "--keep-screen-awake");
    // Cycle-mode comparisons use the selected board oscillator; elapsed-time mode
    // follows CMU changes through Machine::seconds(). Neither changes emulation speed.
    let cycle_hz = profile.osc3_hz as f64;
    let wall = Instant::now();
    let elapsed_time = args.iter().any(|v| v == "--emulated-time");
    let mut next = if elapsed_time {
        m.seconds().floor() + 1.0
    } else {
        (m.cpu.cycles / profile.osc3_hz as u64) as f64 + 1.0
    };
    let mut edges: Vec<(f64, u8)> = args
        .get(4)
        .filter(|v| !v.is_empty() && !v.starts_with("--"))
        .map(|v| {
            v.split(',')
                .map(|edge| {
                    let (t, mask) = edge.split_once(':').expect("time:button-mask");
                    (t.parse::<f64>().unwrap(), mask.parse().unwrap())
                })
                .collect()
        })
        .unwrap_or_default();
    edges.reverse();
    let mut pcm = Vec::<i16>::new();

    while if elapsed_time {
        m.seconds() < end
    } else {
        m.cpu.cycles < (end * cycle_hz) as u64
    } {
        if let Some(&(at, mask)) = edges.last() {
            if if elapsed_time {
                m.seconds() >= at
            } else {
                m.cpu.cycles >= (at * cycle_hz) as u64
            } {
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
            m.cpu.cycles >= (next * cycle_hz) as u64
        } {
            let p = m.memory.peripherals.as_ref().unwrap();
            println!(
                "t={:.3} checkpoint={} wall={:.3} pc={:08x} cycles={} hz={:.0} irq={} lcd={} pixels={} colors={} tone_events={} flash={}/{}",
                m.seconds(),
                next as u64,
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
            if args.iter().any(|v| v == "--power-trace") {
                println!(
                    "power asleep={} sleep_flag={} fd8={} efc={} melody={} rtc={}",
                    !m.memory.lcd.sleep_out || !m.memory.lcd.display_on,
                    m.memory.a0ram[(profile.sleep_flag - profile.a0ram_base) as usize],
                    m.memory.a0ram[0xfd8],
                    m.memory.a0ram[0xefc],
                    m.memory.a0ram[0x11e0],
                    p.rtc_seconds
                );
            }
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
    if let Some(path) = args.windows(2).find(|v| v[0] == "--state").map(|v| &v[1]) {
        fs::write(path, &saved).map_err(|e| e.to_string())?;
    }
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
