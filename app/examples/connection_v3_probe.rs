// SPDX-License-Identifier: GPL-3.0-only
// SPDX-FileCopyrightText: 2026 Noctyros
//! Opt-in real firmware probe. Outputs belong in a private directory.
use retrohatch::{
    core::{
        splb32::{HZ, Machine},
        state::{hex, sha256},
    },
    host::connection_v3_lcd::Lcd,
};
use std::{fs, path::PathBuf, time::Instant};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let a: Vec<_> = std::env::args().collect();
    let rom = fs::read(a.get(1).ok_or("Provide a local ROM")?)?;
    let out = PathBuf::from(a.get(2).ok_or("Provide a private output directory")?);
    fs::create_dir_all(&out)?;
    let seconds: u64 = a.get(3).map(|s| s.parse()).transpose()?.unwrap_or(5);
    let mut m = Machine::new(&rom)?;
    if let Some(state) = a.get(5) {
        m = m.restored(&fs::read(state)?)?;
    }
    let mut events = Vec::new();
    if let Some(path) = a.get(4).filter(|v| v.as_str() != "-") {
        for line in fs::read_to_string(path)?.lines() {
            let fields: Vec<_> = line.split_whitespace().collect();
            if fields.len() == 2 {
                events.push((fields[0].parse::<u64>()?, fields[1].parse::<u8>()?));
            }
        }
    }
    let lcd = Lcd::load()?;
    let start = Instant::now();
    let base = m.cycles;
    let mut trace = String::new();
    let mut samples = 0;
    let mut audible = 0;
    let mut pcm_capture = Vec::new();
    let mut changed = 0;
    let mut previous = [0u8; 512];
    let mut last_saved = None;
    for i in 1..=seconds * 20 {
        for &(at, mask) in &events {
            if at == (i - 1) * 50 {
                m.buttons(mask);
            }
        }
        m.advance_cycles(base + i as f64 * HZ as f64 / 20.)?;
        let pcm = m.drain_audio();
        samples += pcm.len();
        if seconds <= 60 {
            for sample in &pcm {
                pcm_capture.extend_from_slice(&sample.to_le_bytes());
            }
        }
        audible += pcm.iter().filter(|s| **s != 0).count();
        if previous != m.display() {
            changed += 1;
            previous = m.display();
        }
        if i % 10 != 0 {
            continue;
        }
        let ram: Vec<_> = m.ram.iter().map(|v| *v as u8).collect();
        let raw = m.dpram.map(|v| v as u8);
        let io: Vec<_> = (0..64).map(|a| m.read_mem(a) as u8).collect();
        trace.push_str(&format!(
            "{:.8}|{:X},{:X},{:X},{:X},{:X},{:X}|{}|{}|{}|{}\n",
            m.cycles,
            m.physical_pc(),
            m.pc,
            m.a,
            m.x,
            m.y,
            m.sp,
            hex(&ram),
            hex(&raw),
            hex(&m.ee.data),
            hex(&io)
        ));
        if i % 20 == 0 {
            let t = i / 20;
            if t <= 60 || t % 60 == 0 {
                fs::write(out.join(format!("{t:04}.rgb")), lcd.render(&m.display()))?;
            }
            if t % 10 == 0 || t <= 5 {
                println!(
                    "t={:.3} pc={:05X} instructions={} active_bytes={} ram={} eeprom={}",
                    m.cycles / HZ as f64,
                    m.physical_pc(),
                    m.instructions,
                    m.display().iter().filter(|v| **v != 0).count(),
                    &hex(&sha256(&ram))[..12],
                    &hex(&sha256(&m.ee.data))[..12]
                );
            }
        }
        if i % 100 == 0 {
            let state = m.save_state()?;
            let mut restored = m.restored(&state)?;
            let mut original = m.clone();
            let target = m.cycles + HZ as f64 / 20.;
            original.advance_cycles(target)?;
            restored.advance_cycles(target)?;
            if original.save_state()? != restored.save_state()? {
                return Err("State continuation diverged".into());
            }
            last_saved = Some(state);
        }
    }
    fs::write(out.join("reference.trace"), trace)?;
    if seconds <= 60 {
        fs::write(out.join("pcm.bin"), pcm_capture)?;
    }
    fs::write(out.join("lcd.bin"), m.display())?;
    fs::write(out.join("eeprom.bin"), m.ee.data)?;
    fs::write(out.join("final.rhstate"), m.save_state()?)?;
    if let Some(state) = last_saved {
        fs::write(out.join("checkpoint.rhstate"), state)?;
    }
    println!(
        "wall={:.6} samples={samples} audible={audible} changed_frames={changed} exact_icons={}",
        start.elapsed().as_secs_f64(),
        lcd.exact_icons()
    );
    Ok(())
}
