// SPDX-License-Identifier: GPL-3.0-only
// SPDX-FileCopyrightText: 2026 Noctyros
// Applies to original RetroHatch contributions; third-party provenance is retained below and in THIRD_PARTY_LICENSES.md.
//! Local firmware diagnostic: record deterministic states for comparison with BrickEmuPy.
//! Usage: cargo run --release --locked --example device_trace -- PROFILE ROM TRACE [--gameplay]
//! The trace is private diagnostic state; firmware must be supplied by the user.
use retrohatch::core::{
    device::{Button, Device},
    mcu::MASTER_HZ,
    profile::DeviceProfile,
    tamagotchi::Tamagotchi,
};
use retrohatch::host::roms;
use std::io::{BufWriter, Write};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args_os().skip(1);
    let profile = args
        .next()
        .and_then(|v| v.to_str().and_then(DeviceProfile::parse))
        .ok_or("Specify p1, p2, genjintch, angel, morino or umino")?;
    let path = args.next().ok_or("Provide a local ROM path")?;
    let target = args.next().ok_or("Provide a private trace output path")?;
    let gameplay = match args.next() {
        None => false,
        Some(value) if value == "--angel-gameplay" || value == "--gameplay" => true,
        _ => {
            return Err(
                "Unknown option; use --gameplay for the extended button acceptance sequence".into(),
            );
        }
    };
    let mut out = BufWriter::new(std::fs::File::create(target)?);
    let rom = roms::load_for_profile(std::path::Path::new(&path), profile)?;
    let mut d = Tamagotchi::new(&rom)?;
    // Normal electrical input only. No firmware data or timer values are patched.
    let mut events = if gameplay {
        vec![
            (5.0, Button::B, 2, true),
            (6.0, Button::B, 2, false),
            (7.0, Button::A, 4, true),
            (8.0, Button::A, 4, false),
            (9.0, Button::C, 1, true),
            (10.0, Button::C, 1, false),
            (385.0, Button::A, 4, true),
            (385.15, Button::A, 4, false),
            (386.0, Button::B, 2, true),
            (386.15, Button::B, 2, false),
            (387.0, Button::C, 1, true),
            (387.15, Button::C, 1, false),
            (388.0, Button::A, 4, true),
            (388.15, Button::A, 4, false),
            (389.0, Button::B, 2, true),
            (389.15, Button::B, 2, false),
            (390.0, Button::B, 2, true),
            (390.15, Button::B, 2, false),
            (391.0, Button::Knock, 8, true),
            (391.15, Button::Knock, 8, false),
            (392.0, Button::C, 1, true),
            (392.15, Button::C, 1, false),
        ]
    } else {
        vec![
            (5.0, Button::A, 4, true),
            (5.15, Button::A, 4, false),
            (6.0, Button::B, 2, true),
            (6.15, Button::B, 2, false),
            (7.0, Button::C, 1, true),
            (7.15, Button::C, 1, false),
            (8.0, Button::Knock, 8, true),
            (8.15, Button::Knock, 8, false),
            (12.0, Button::Knock, 8, true),
            (12.15, Button::Knock, 8, false),
        ]
    };
    if gameplay && profile == DeviceProfile::Morino {
        // Morino presents an egg-selection screen after clock setup. Confirm it.
        events.splice(
            6..6,
            [(12.0, Button::B, 2, true), (12.15, Button::B, 2, false)],
        );
    }
    let mut event = 0;
    let mut next = 0;
    let mut samples = 0u64;
    let mut nonzero = 0u64;
    let mut peak = 0i32;
    let seconds = if gameplay { 420 } else { 60 };
    let sample_hz = if gameplay { 4 } else { 64 };
    while d.time() < MASTER_HZ * seconds {
        while event < events.len() && d.time() >= (events[event].0 * MASTER_HZ as f64) as u64 {
            let (_, button, mask, pressed) = events[event];
            if button != Button::Knock || profile.has_knock() {
                writeln!(
                    out,
                    "E|{}|{}|{}",
                    d.mcu.counters.clock_calls,
                    mask,
                    u8::from(pressed)
                )?;
                d.set_button(button, pressed);
            }
            event += 1;
        }
        d.mcu.clock()?;
        if d.time() >= next {
            let s = d.cpu_state();
            let flags = u8::from(s.carry)
                | (u8::from(s.zero) << 1)
                | (u8::from(s.decimal) << 2)
                | (u8::from(s.interrupt_enable) << 3);
            let values = [
                s.pc,
                s.npc,
                s.a.into(),
                s.b.into(),
                s.ix,
                s.iy,
                s.sp.into(),
                flags.into(),
                u16::from(s.interrupt_delay),
                u16::from(s.halted),
            ];
            let state = values.map(|v| format!("{v:X}")).join(",");
            let ram: String = d.mcu.cpu().ram().iter().map(|v| format!("{v:X}")).collect();
            let lcd: String = d
                .mcu
                .cpu()
                .lcd_memory()
                .iter()
                .map(|v| format!("{v:X}"))
                .collect();
            writeln!(
                out,
                "S|{}|{}|{}|{}|{}|{:X},{:X}",
                d.mcu.counters.clock_calls,
                d.time(),
                state,
                ram,
                lcd,
                d.mcu.read(0xF70)?,
                d.mcu.read(0xF71)?
            )?;
            next += MASTER_HZ / sample_hz;
            for sample in d.drain_audio() {
                samples += 1;
                nonzero += u64::from(sample != 0);
                peak = peak.max(i32::from(sample).abs());
            }
        }
    }
    writeln!(out, "A|{}|{}|{}", samples, nonzero, peak)?;
    out.flush()?;
    println!(
        "Generated {samples} PCM samples; {nonzero} nonzero; peak {peak}. No host audio playback in this trace mode."
    );
    println!(
        "Recorded {:.6} seconds, {} instructions and {} interrupts for {}.",
        d.time() as f64 / MASTER_HZ as f64,
        d.mcu.instructions,
        d.mcu.interrupts,
        profile.name()
    );
    Ok(())
}
