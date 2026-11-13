// SPDX-License-Identifier: GPL-3.0-only
//! Private, user-supplied firmware probe. Writes captures only to the requested directory.
use retrohatch::core::tamago::{HZ, Machine, figure::Figure};
use std::{fs, path::PathBuf, sync::Arc, time::Instant};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    let rom = fs::read(
        args.get(1)
            .ok_or("Usage: tamago_probe ROM OUTPUT [FIGURE] [SECONDS]")?,
    )?;
    let out = PathBuf::from(args.get(2).ok_or("Missing private output directory")?);
    fs::create_dir_all(&out)?;
    let mut m = Machine::new(&rom)?;
    if let Some(path) = args.get(3).filter(|p| p.as_str() != "-") {
        m.set_figure(Some(Arc::new(Figure::decode(&fs::read(path)?)?)));
    }
    let seconds: u64 = args.get(4).map(|s| s.parse()).transpose()?.unwrap_or(30);
    let start = Instant::now();
    let mut audio = 0;
    let mut audible = 0;
    let mut checkpoints = Vec::new();
    for tick in 1..=seconds * 100 {
        // Upstream host/setup.txt: first select an egg, confirm the date/time and name.
        let keys = if (300..320).contains(&tick) {
            2
        } else if (520..540).contains(&tick) {
            2
        } else if (600..620).contains(&tick) {
            2
        } else if (680..700).contains(&tick) {
            2
        } else if (760..780).contains(&tick) {
            1
        } else if (840..1340).contains(&tick) && (tick - 840) % 80 < 20 {
            2
        } else if (1400..2220).contains(&tick) && (tick - 1400) % 100 < 20 {
            2
        } else {
            0
        };
        m.buttons(keys);
        m.advance_cycles(tick * HZ / 100)?;
        let pcm = m.drain_audio();
        audio += pcm.len();
        audible += pcm.iter().filter(|s| **s != 0).count();
        if tick % 100 == 0 {
            let mut b = Vec::new();
            b.extend_from_slice(&m.cpu.pc.to_le_bytes());
            b.extend_from_slice(&[m.cpu.a, m.cpu.x, m.cpu.y, m.cpu.s, m.cpu.p]);
            b.extend_from_slice(&m.ram);
            b.extend_from_slice(&m.dram);
            b.extend_from_slice(&m.reg);
            checkpoints.extend_from_slice(&b);
        }
    }
    if args.get(5).is_some_and(|v| v == "menus") {
        for count in 1..=10 {
            let mut branch = m.clone();
            branch.buttons(0);
            branch.advance_cycles(branch.cycles + HZ * 3)?;
            for _ in 0..3 {
                branch.buttons(4);
                branch.advance_cycles(branch.cycles + HZ / 3)?;
                branch.buttons(0);
                branch.advance_cycles(branch.cycles + HZ)?;
            }
            branch.advance_cycles(branch.cycles + HZ * 20)?;
            branch.drain_audio();
            for _ in 0..count {
                branch.buttons(1);
                branch.advance_cycles(branch.cycles + HZ / 5)?;
                branch.buttons(0);
                branch.advance_cycles(branch.cycles + HZ / 2)?;
            }
            branch.buttons(2);
            branch.advance_cycles(branch.cycles + HZ / 5)?;
            branch.buttons(0);
            branch.advance_cycles(branch.cycles + HZ * 2)?;
            let mut image = format!("P6\\n48 31\\n255\\n")
                .replace("\\n", "\n")
                .into_bytes();
            image.extend_from_slice(&branch.rgb());
            fs::write(out.join(format!("menu-{count}.ppm")), image)?;
            branch.drain_audio();
            let snapshot = branch.save_state();
            let mut reopened = Machine::new(&rom)?.restored(&snapshot)?;
            branch.advance_cycles(branch.cycles + HZ)?;
            reopened.advance_cycles(reopened.cycles + HZ)?;
            assert_eq!(branch.save_state(), reopened.save_state());
            fs::write(out.join(format!("menu-{count}.rhstate")), snapshot)?;
            if count == 5 {
                for _ in 0..3 {
                    branch.buttons(2);
                    branch.advance_cycles(branch.cycles + HZ / 5)?;
                    branch.buttons(0);
                    branch.advance_cycles(branch.cycles + HZ * 2)?;
                    branch.drain_audio();
                }
                let mut game = b"P6\n48 31\n255\n".to_vec();
                game.extend_from_slice(&branch.rgb());
                fs::write(out.join("figure-game.ppm"), game)?;
                println!(
                    "figure_game spi_reads={} pc={:04X}",
                    branch.spi_reads, branch.cpu.pc
                );
            }
            println!(
                "menu={count} spi_reads={} icons={:?} pc={:04X} reopen=exact",
                branch.spi_reads,
                branch.icons(),
                branch.cpu.pc
            );
        }
    }
    if args.get(5).is_some_and(|v| v == "swap") {
        for _ in 0..3 {
            m.buttons(4);
            m.advance_cycles(m.cycles + HZ / 5)?;
            m.buttons(0);
            m.advance_cycles(m.cycles + HZ)?;
            m.drain_audio();
        }
        m.advance_cycles(m.cycles + HZ * 20)?;
        m.drain_audio();
        let before = *m.eeprom();
        m.set_figure(None);
        assert_eq!(&before, m.eeprom());
        m.advance_cycles(m.cycles + HZ * 3)?;
        m.drain_audio();
        let replacement = Figure::decode(&fs::read(
            args.get(6)
                .ok_or("swap requires a replacement figure path")?,
        )?)?;
        let before = *m.eeprom();
        m.set_figure(Some(Arc::new(replacement)));
        assert_eq!(&before, m.eeprom());
        m.advance_cycles(m.cycles + HZ * 30)?;
        m.drain_audio();
        let mut image = b"P6\n48 31\n255\n".to_vec();
        image.extend_from_slice(&m.rgb());
        fs::write(out.join("swapped.ppm"), image)?;
        println!(
            "live_swap spi_reads={} cycles={} figure={:?} eeprom_never_erased=true",
            m.spi_reads,
            m.cycles,
            m.figure.as_ref().map(|f| &f.name)
        );
    }
    let elapsed = start.elapsed().as_secs_f64();
    let state = m.save_state();
    fs::write(out.join("state.rhstate"), &state)?;
    let mut restored = m.restored(&state)?;
    m.advance_cycles((seconds + 1) * HZ)?;
    restored.advance_cycles((seconds + 1) * HZ)?;
    assert_eq!(m.save_state(), restored.save_state());
    let rgb = m.rgb();
    let mut ppm = format!("P6\n48 31\n255\n").into_bytes();
    ppm.extend_from_slice(&rgb);
    fs::write(out.join("lcd.ppm"), ppm)?;
    fs::write(out.join("checkpoints.bin"), checkpoints)?;
    fs::write(out.join("ram.bin"), m.ram)?;
    println!(
        "emulated={}s wall={elapsed:.3}s speed={:.1}% pc={:04X} instructions={} irq={} pixels={} audio={} audible={} spi_reads={} figure={:?} clock={:02}:{:02}:{:02} save_roundtrip=exact",
        seconds,
        seconds as f64 / elapsed * 100.,
        m.cpu.pc,
        m.instructions,
        m.interrupts,
        m.pixels().iter().filter(|v| **v != 0).count(),
        audio,
        audible,
        m.spi_reads,
        m.figure.as_ref().map(|f| &f.name),
        m.ram[0x306],
        m.ram[0x307],
        m.ram[0x396]
    );
    Ok(())
}
