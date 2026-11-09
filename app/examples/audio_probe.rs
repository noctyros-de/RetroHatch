//! Real-worker audio queue measurements plus deterministic, unfiltered MCU WAV capture.
use retrohatch::{
    core::{
        device::{Button, Device},
        mcu::MASTER_HZ,
        profile::DeviceProfile,
        tamagotchi::Tamagotchi,
    },
    host::{
        roms,
        runner::{Command, Controller},
    },
};
use std::{
    io::Write,
    path::Path,
    time::{Duration, Instant},
};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() < 2 {
        return Err("Provide profile ROM [WAV output]".into());
    }
    let profile = DeviceProfile::parse(&args[0]).ok_or("Unknown profile")?;
    let c = Controller::new(true);
    c.send(Command::LoadProfile(args[1].clone().into(), profile));
    let start = Instant::now();
    let mut pressed = false;
    let mut released = false;
    while start.elapsed() < Duration::from_secs(12) {
        let s = c.snapshot();
        if let Some(e) = s.error {
            return Err(e.into());
        }
        if s.performance.emulated_seconds > 5.0 && !pressed {
            c.send(Command::Button(Button::B, true));
            pressed = true;
        }
        if s.performance.emulated_seconds > 6.0 && !released {
            c.send(Command::Button(Button::B, false));
            released = true;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    c.send(Command::Pause);
    while c.snapshot().running {
        std::thread::sleep(Duration::from_millis(1));
    }
    let s = c.snapshot();
    println!(
        "{}: {:?}, drops={}, status={}, speed={:.4}%",
        profile.name(),
        s.audio_stats,
        s.audio_drops,
        s.audio_status,
        s.performance.average_speed_percent
    );
    if let Some(out) = args.get(2) {
        let rom = roms::load_for_profile(Path::new(&args[1]), profile)?;
        let mut d = Tamagotchi::new(&rom)?;
        let mut pcm = Vec::new();
        while d.time() < MASTER_HZ * 12 {
            let time = d.time();
            d.set_button(Button::B, (MASTER_HZ * 5..MASTER_HZ * 6).contains(&time));
            d.advance_to((time + MASTER_HZ / 200).min(MASTER_HZ * 12))?;
            pcm.extend(d.drain_audio());
        }
        let bytes: Vec<_> = pcm.iter().flat_map(|s| s.to_le_bytes()).collect();
        let mut f = std::fs::File::create(out)?;
        f.write_all(b"RIFF")?;
        f.write_all(&(bytes.len() as u32 + 36).to_le_bytes())?;
        f.write_all(b"WAVEfmt ")?;
        f.write_all(&16u32.to_le_bytes())?;
        f.write_all(&1u16.to_le_bytes())?;
        f.write_all(&1u16.to_le_bytes())?;
        f.write_all(&44100u32.to_le_bytes())?;
        f.write_all(&88200u32.to_le_bytes())?;
        f.write_all(&2u16.to_le_bytes())?;
        f.write_all(&16u16.to_le_bytes())?;
        f.write_all(b"data")?;
        f.write_all(&(bytes.len() as u32).to_le_bytes())?;
        f.write_all(&bytes)?;
        println!("Captured {} unfiltered MCU samples.", pcm.len());
    }
    Ok(())
}
