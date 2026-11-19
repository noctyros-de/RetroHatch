// SPDX-License-Identifier: GPL-3.0-only
// SPDX-FileCopyrightText: 2026 Noctyros
//! Opt-in RTC safety experiment; never used by the application resume path.
//! Usage: clock_resume_probe tg18|ps ROM STATE|- NEW_PRIVATE_OUTPUT DELTA_SECONDS RUN_SECONDS [time:mask,...]
//! Changes documented RTC registers only; firmware is allowed to respond normally.
use retrohatch::core::{
    s1c33::{machine::Machine as Ps, profile::HardwareProfile},
    state::sha256,
};
use std::{env, fs, path::PathBuf, sync::Arc};
type Error = Box<dyn std::error::Error>;

// Calendar carries are adapted from maragotchi/tamaemu src/periph.c (GPL-3.0),
// through RetroHatch core/s1c33/peripherals.rs; see THIRD_PARTY_NOTICES.md.
// This diagnostic intentionally follows the existing reference carry behavior.
fn bcd_next(value: u8, max: u8) -> Option<u8> {
    let mut n = value.wrapping_add(1);
    if n & 15 > 9 {
        n = (n & 240).wrapping_add(16);
    }
    (n <= max).then_some(n)
}
fn calendar_second(io: &mut [u8]) {
    for (a, max, start) in [(0x1910, 0x59, 0), (0x1914, 0x59, 0), (0x1918, 0x23, 0)] {
        if let Some(v) = bcd_next(io[a], max) {
            io[a] = v;
            return;
        }
        io[a] = start;
    }
    io[0x1928] = (io[0x1928] + 1) % 7;
    let m = io[0x1920];
    let month = ((m >> 4) & 1) * 10 + (m & 15);
    let days = [
        0, 0x31, 0x28, 0x31, 0x30, 0x31, 0x30, 0x31, 0x31, 0x30, 0x31, 0x30, 0x31,
    ];
    let max = if month == 2 && (io[0x1924] & 15) % 4 == 0 {
        0x29
    } else {
        *days
            .get(month as usize)
            .filter(|_| month != 0)
            .unwrap_or(&0x31)
    };
    for (a, max, start) in [(0x191c, max, 1), (0x1920, 0x12, 1), (0x1924, 0x99, 0)] {
        if let Some(v) = bcd_next(io[a], max) {
            io[a] = v;
            return;
        }
        io[a] = start;
    }
}
fn main() -> Result<(), Error> {
    let a: Vec<_> = env::args().skip(1).collect();
    if a.len() < 6 {
        return Err(
            "Provide backend, ROM, state or -, new private output, RTC delta and run duration"
                .into(),
        );
    }
    let out = PathBuf::from(&a[3]);
    if out.exists() {
        return Err("Output already exists; use a fresh isolated directory".into());
    }
    let parent = out
        .parent()
        .ok_or("Missing output parent")?
        .canonicalize()?;
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .canonicalize()?;
    if parent.starts_with(repo) {
        return Err("Output must be outside the public repository".into());
    }
    if env::var_os("RETROHATCH_DATA_DIR")
        .map(PathBuf::from)
        .as_deref()
        != Some(out.as_path())
    {
        return Err("Set RETROHATCH_DATA_DIR to the new isolated output directory".into());
    }
    let delta: u64 = a[4].parse()?;
    let seconds: u64 = a[5].parse()?;
    if delta > 366 * 86400 || seconds > 600 {
        return Err("Bound the experiment to one RTC year and 600 execution seconds".into());
    }
    // Refuse the personal application root even when an override was supplied.
    if let Some(local) = env::var_os("LOCALAPPDATA") {
        let personal = PathBuf::from(local).join("RetroHatch");
        if within(&parent, &personal) {
            return Err("Never use the personal RetroHatch data directory for experiments".into());
        }
    }
    fs::create_dir(&out)?;
    let image = fs::read(&a[1])?;
    let identity = sha256(&image);
    let state = if a[2] == "-" {
        None
    } else {
        Some(fs::read(&a[2])?)
    };
    let events: Vec<(f64, u8)> = a
        .get(6)
        .map(|s| {
            s.split(',')
                .map(|s| {
                    let (t, m) = s.split_once(':').ok_or("Expected time:mask")?;
                    Ok((t.parse()?, m.parse()?))
                })
                .collect::<Result<_, Error>>()
        })
        .transpose()?
        .unwrap_or_default();
    let mut report = String::new();
    if a[0] == "tg18" {
        let mut m = tg18::Machine::new(Arc::new(image.clone()), &image, 0.0);
        if let Some(bytes) = state.as_ref() {
            if bytes.len() < 113 {
                return Err("Truncated TG18 state".into());
            }
            tg18::snapshot::Snapshot::from_bytes(if bytes.starts_with(b"T18SNAP1") {
                bytes
            } else {
                &bytes[81..bytes.len() - 32]
            })
            .map_err(|e| e.to_string())?
            .restore(&mut m)
            .map_err(|e| e.to_string())?;
        }
        m.never_sleep = false;
        let signature = tg18::sigs::Pattern::parse(tg18::sigs::CLOCK_REFRESH_SIG);
        let cache = signature
            .find(&image, 0, tg18::sigs::CODE_END)
            .and_then(|off| {
                (off..off + 64).step_by(2).find_map(|pc| {
                    let op = u16::from_le_bytes(image[pc..pc + 2].try_into().unwrap());
                    if op & 0xff00 == 0x4c00 && image[pc + 2..pc + 4] == [0x25, 0x73] {
                        let literal = ((pc + 4) & !3) + (op as usize & 255) * 4;
                        Some(u32::from_le_bytes(
                            image[literal..literal + 4].try_into().unwrap(),
                        ))
                    } else {
                        None
                    }
                })
            });
        if let Some(cache) = cache {
            let mut fields = [0; 4];
            m.sys.read_block(cache + 12, &mut fields);
            println!("clock_cache_before={fields:?} at={cache:08x}");
        }

        let ticks = m.sys.rtc.ticks(m.sys.game_time);
        let before_time = m.sys.game_time;
        let before_cycles = m.executed;
        let before_ram = sha256(&m.sys.ram);
        let before_sram = sha256(&m.sys.sram);
        m.sys.rtc.base_ticks = m
            .sys
            .rtc
            .base_ticks
            .checked_add((delta * 32768) as i64)
            .ok_or("RTC overflow")?;
        m.sys.rtc_irq.tod_base = m
            .sys
            .rtc_irq
            .tod_base
            .checked_add(delta as i64)
            .ok_or("TOD overflow")?;
        assert_eq!(
            m.sys.rtc.ticks(m.sys.game_time) - ticks,
            (delta * 32768) as i64
        );
        assert_eq!(m.executed, before_cycles);
        assert_eq!(m.sys.game_time, before_time);
        assert_eq!(sha256(&m.sys.ram), before_ram);
        assert_eq!(sha256(&m.sys.sram), before_sram);
        if a.iter().any(|v| v == "--refresh-firmware-clock") {
            let offset = tg18::sigs::Pattern::parse(tg18::sigs::CLOCK_REFRESH_SIG)
                .find(&image, 0, tg18::sigs::CODE_END)
                .ok_or("No verified clock-refresh signature")?;
            let before = m.cpu.state();
            if !m.call_firmware(tg18::FLASH_BASE + offset as u32, 0, 5_000_000) {
                return Err("Firmware clock refresh did not return".into());
            }
            assert_eq!(m.cpu.state(), before);
            report.push_str("existing_firmware_clock_refresh=true\n");
        }
        use retrohatch::{
            core::{device::Button, mcu::MASTER_HZ, profile::DeviceProfile},
            host::emulator::Emulator,
        };
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
            .ok_or("Unknown Meets edition")?
        };
        let base = Emulator::from_bytes(&image, profile)?;
        // Rebuild the existing integrity-checked host envelope around the RTC-only snapshot.
        let template = if let Some(bytes) = state.as_ref().filter(|b| b.starts_with(b"RTHSTATE")) {
            base.restored(bytes)?; // Validate ROM identity, schema and integrity before using the header.
            bytes.clone()
        } else {
            base.save_state()?
        };
        let snapshot = tg18::snapshot::Snapshot::capture(&m).to_bytes();
        let mut envelope = template[..81].to_vec();
        envelope[45..49].copy_from_slice(&((32 + snapshot.len()) as u32).to_le_bytes());
        envelope.extend(snapshot);
        envelope.extend(sha256(&envelope));
        let mut d = base.restored(&envelope)?;
        d.set_keep_screen_awake(false);
        fs::write(out.join("injected.rhstate"), d.save_state()?)?;
        let start = d.time();
        let mut event = 0;
        let mut saved_sleep = false;
        for step in 0..=seconds * 100 {
            let elapsed = step as f64 / 100.0;
            while event < events.len() && events[event].0 <= elapsed {
                for (i, button) in [Button::A, Button::B, Button::C].into_iter().enumerate() {
                    d.set_button(button, events[event].1 & (1 << i) != 0);
                }
                event += 1;
            }
            d.advance_to(start + (elapsed * MASTER_HZ as f64) as u64)?;
            d.drain_audio();
            if d.sleeping() && !saved_sleep {
                fs::write(out.join("first-sleep.rhstate"), d.save_state()?)?;
                saved_sleep = true;
            }
            if step % 100 == 0 {
                fs::write(
                    out.join(format!("lcd-{}.png", step / 100)),
                    tg18::png::encode(
                        128,
                        128,
                        &d.color_display().ok_or("Missing framebuffer")?,
                        1,
                    ),
                )?;
                report.push_str(&format!(
                    "t={} instructions={} irqs={} sleeping={}\n",
                    step / 100,
                    d.instructions(),
                    d.interrupts(),
                    d.sleeping()
                ));
            }
        }
        let saved = d.save_state()?;
        fs::write(out.join("final.rhstate"), &saved)?;
        tg18::snapshot::Snapshot::from_bytes(&saved[81..saved.len() - 32])
            .map_err(|e| e.to_string())?
            .restore(&mut m)
            .map_err(|e| e.to_string())?;
        report.push_str(&format!(
            "final rtc={} tod={} flash_writes={} alarm_wake={}\n",
            m.sys.rtc.ticks(m.sys.game_time) / 32768,
            m.sys.rtc_irq.tod(m.sys.game_time),
            m.sys.flash.writes,
            m.alarm_wake
        ));
        if let Some(cache) = cache {
            let mut fields = [0; 4];
            m.sys.read_block(cache + 12, &mut fields);
            println!("clock_cache_after={fields:?} at={cache:08x}");
        }
        fs::write(out.join("sram.bin"), &m.sys.sram)?;
    } else if a[0] == "ps" {
        let mut m = if let Some(bytes) = state.as_ref() {
            Ps::restored(bytes, identity)?
        } else {
            Ps::new(&image, HardwareProfile::find("ps").unwrap())?
        };
        let before = m.clone();
        let p = m.memory.peripherals.as_mut().unwrap();
        if delta > 0 && (p.io[0x1908] & 2 != 0 || p.io[0x190c] & 1 != 0) {
            return Err("The diagnostic requires a running, unheld RTC".into());
        }
        let rtc_before = calendar(&p.io);
        for _ in 0..delta {
            calendar_second(&mut p.io);
        }
        // Only the seven documented calendar bytes may change, not IRQ/timer fractions.
        let mut expected = before.memory.peripherals.clone().unwrap();
        for at in [0x1910, 0x1914, 0x1918, 0x191c, 0x1920, 0x1924, 0x1928] {
            expected.io[at] = p.io[at];
        }
        assert_eq!(&expected, p);
        assert_eq!(m.cpu.cycles, before.cpu.cycles);
        assert_eq!(m.memory.a0ram, before.memory.a0ram);
        report.push_str(&format!(
            "calendar_before={rtc_before} calendar_after={}\n",
            calendar(&p.io)
        ));
        if a.iter().any(|v| v == "--hold-calendar") {
            // Documented RTC stop bit: isolate calendar from the still-running cyclic IRQ.
            m.memory.peripherals.as_mut().unwrap().io[0x1908] |= 2;
            report
                .push_str("documented_calendar_stop=true; CPU and cyclic IRQ continue normally\n");
        }
        let start = m.seconds();
        let mut event = 0;
        for step in 0..=seconds * 100 {
            let elapsed = step as f64 / 100.0;
            while event < events.len() && events[event].0 <= elapsed {
                m.buttons(events[event].1);
                event += 1;
            }
            m.advance_to(start + elapsed)?;
            m.drain_audio();
            if step % 100 == 0 {
                fs::write(
                    out.join(format!("lcd-{}.png", step / 100)),
                    tg18::png::encode(128, 128, &m.rgb(), 1),
                )?;
                report.push_str(&format!(
                    "t={} calendar={} irqs={} flash_writes={} ram={}\n",
                    step / 100,
                    calendar(&m.memory.peripherals.as_ref().unwrap().io),
                    m.interrupts,
                    m.memory.flash_state.programs,
                    short(&m.memory.a0ram)
                ));
            }
        }
        fs::write(out.join("final.rhstate"), m.save_state(identity)?)?;
        fs::write(out.join("ram.bin"), &m.memory.a0ram)?;
    } else {
        return Err("Only tg18 and ps are accepted".into());
    }
    assert_eq!(identity, sha256(&fs::read(&a[1])?));
    if let Some(original) = state.as_ref() {
        assert_eq!(
            sha256(original),
            sha256(&fs::read(&a[2])?),
            "Source checkpoint was modified"
        );
    }
    fs::write(out.join("report.txt"), &report)?;
    println!("{report}");
    Ok(())
}
fn short(data: &[u8]) -> String {
    retrohatch::core::state::hex(&sha256(data))[..16].to_owned()
}
fn calendar(io: &[u8]) -> String {
    format!(
        "{:02x}-{:02x}-{:02x} {:02x}:{:02x}:{:02x} weekday={}",
        io[0x1924], io[0x1920], io[0x191c], io[0x1918], io[0x1914], io[0x1910], io[0x1928]
    )
}

// Windows canonical paths may have an extended-length prefix and different case.
fn within(path: &std::path::Path, root: &std::path::Path) -> bool {
    fn normalized(path: &std::path::Path) -> String {
        let text = path.to_string_lossy().replace('\\', "/").to_lowercase();
        text.strip_prefix("//?/")
            .unwrap_or(&text)
            .trim_end_matches('/')
            .to_owned()
    }
    let path = normalized(path);
    let root = normalized(root);
    path == root || path.starts_with(&(root + "/"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use retrohatch::core::s1c33::peripherals::Peripherals;

    #[test]
    fn personal_directory_guard_handles_windows_canonical_paths_and_boundaries() {
        use std::path::Path;
        let root = Path::new("Q:/synthetic-profile/AppData/Local/RetroHatch");
        assert!(within(root, root));
        assert!(within(
            Path::new("//?/q:/synthetic-profile/appdata/local/retrohatch/probe"),
            root
        ));
        assert!(!within(
            Path::new("Q:/synthetic-profile/AppData/Local/RetroHatch-other"),
            root
        ));
        assert!(!within(Path::new("Q:/isolated-test/probe"), root));
    }

    #[test]
    fn diagnostic_calendar_carries_match_existing_peripheral_without_replaying_irqs() {
        let profile = HardwareProfile::find("ps").unwrap();
        // Second/minute/day/month/year carries, including the current reference's
        // two-digit-year leap behavior. This is not a new calendar specification.
        for (year, month, day, hour, minute, second, weekday, delta) in [
            (0x08, 0x03, 0x15, 0x23, 0x39, 0x14, 0, 61200),
            (0x08, 0x02, 0x28, 0x23, 0x59, 0x59, 3, 2),
            (0x09, 0x02, 0x28, 0x23, 0x59, 0x59, 4, 2),
            (0x99, 0x12, 0x31, 0x23, 0x59, 0x59, 6, 2),
            (0x08, 0x04, 0x30, 0x23, 0x59, 0x59, 0, 2),
            (0x08, 0x03, 0x15, 0x21, 0x00, 0x00, 6, 0),
        ] {
            let mut rtc = Peripherals::new(profile);
            for (at, value) in [
                (0x1924, year),
                (0x1920, month),
                (0x191c, day),
                (0x1918, hour),
                (0x1914, minute),
                (0x1910, second),
                (0x1928, weekday),
            ] {
                rtc.io[at] = value;
            }
            let original = rtc.clone();
            let mut expected = rtc.clone();
            for _ in 0..delta {
                calendar_second(&mut rtc.io);
                expected.tick(profile.osc3_hz, profile);
            }
            for at in [0x1910, 0x1914, 0x1918, 0x191c, 0x1920, 0x1924, 0x1928] {
                assert_eq!(rtc.io[at], expected.io[at], "Calendar register {at:x}");
                rtc.io[at] = original.io[at];
            }
            assert_eq!(
                rtc, original,
                "The diagnostic changed non-calendar peripheral state"
            );
        }
    }
}
