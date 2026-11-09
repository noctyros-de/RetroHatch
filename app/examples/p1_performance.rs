//! Repeatable E0C6200 Tamagotchi timing probe. Firmware must be supplied by the user.
//! Usage: cargo run --release --locked --example p1_performance -- ROM --seconds 60 [--audio] [--device p1|genjintch|angel|morino|umino] [--profile] [--unpaced]
use retrohatch::core::{
    device::Device, mcu::MASTER_HZ, profile::DeviceProfile, tamagotchi::Tamagotchi,
};
use retrohatch::host::{
    performance::{CSV_HEADER, Performance, csv_row},
    roms,
    runner::{Command, Controller},
};
use std::time::{Duration, Instant};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args_os().skip(1);
    let path = args.next().ok_or("Provide a local Tamagotchi ROM path.")?;
    let mut seconds = 60.0;
    let mut audio = false;
    let mut profile = false;
    let mut unpaced = false;
    let mut device_profile = DeviceProfile::P1;
    while let Some(arg) = args.next() {
        match arg.to_str() {
            Some("--seconds") => {
                seconds = args
                    .next()
                    .ok_or("--seconds requires a value")?
                    .to_str()
                    .ok_or("Invalid duration")?
                    .parse::<f64>()?;
            }
            Some("--device") => {
                let value = args
                    .next()
                    .ok_or("--device requires p1, genjintch, angel, morino or umino")?;
                device_profile = value
                    .to_str()
                    .and_then(DeviceProfile::parse)
                    .ok_or("Unknown profile; use p1, genjintch, angel, morino or umino")?;
            }
            Some("--audio") => audio = true,
            Some("--profile") => profile = true,
            Some("--unpaced") => unpaced = true,
            _ => return Err(format!("Unknown argument: {}", arg.to_string_lossy()).into()),
        }
    }
    if !seconds.is_finite() || seconds <= 0.0 {
        return Err("Duration must be positive and finite.".into());
    }
    println!("{CSV_HEADER}");
    if unpaced {
        let rom = roms::load_for_profile(std::path::Path::new(&path), device_profile)?;
        let mut device = Tamagotchi::new(&rom)?;
        device.mcu.set_profiling(profile);
        let start = Instant::now();
        let target = (seconds * MASTER_HZ as f64) as u64;
        while device.time() < target {
            device.advance_to((device.time() + MASTER_HZ / 200).min(target))?;
            std::hint::black_box(device.drain_audio());
        }
        let wall = start.elapsed().as_secs_f64();
        let emulated = device.time() as f64 / MASTER_HZ as f64;
        let p = Performance {
            wall_seconds: wall,
            emulated_seconds: emulated,
            speed_percent: emulated / wall * 100.0,
            average_speed_percent: emulated / wall * 100.0,
            instructions_per_second: device.mcu.instructions as f64 / wall,
            cycles_per_second: device.mcu.counters.total() as f64 / wall,
            ..Performance::default()
        };
        println!(
            "{}",
            csv_row(
                &p,
                device.mcu.counters,
                device.mcu.profile,
                device.mcu.instructions,
                false,
                device.cpu_state().halted
            )
        );
        return Ok(());
    }
    let controller = Controller::new(audio);
    controller.send(Command::Profiling(profile));
    controller.send(Command::LoadProfile(path.into(), device_profile));
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let snapshot = controller.snapshot();
        if let Some(error) = snapshot.error {
            return Err(error.into());
        }
        if snapshot.loaded {
            break;
        }
        if Instant::now() >= deadline {
            return Err("ROM loading timed out.".into());
        }
        std::thread::sleep(Duration::from_millis(2));
    }
    let mut last = -1.0;
    loop {
        let snapshot = controller.snapshot();
        if let Some(error) = &snapshot.error {
            return Err(error.clone().into());
        }
        if snapshot.performance.wall_seconds >= seconds {
            break;
        }
        if snapshot.performance.wall_seconds - last >= 1.0 {
            println!(
                "{}",
                csv_row(
                    &snapshot.performance,
                    snapshot.cycles,
                    snapshot.core_profile,
                    snapshot.instructions,
                    snapshot.running,
                    snapshot.state.halted
                )
            );
            last = snapshot.performance.wall_seconds;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    controller.send(Command::Pause);
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let snapshot = controller.snapshot();
        if !snapshot.running {
            println!(
                "{}",
                csv_row(
                    &snapshot.performance,
                    snapshot.cycles,
                    snapshot.core_profile,
                    snapshot.instructions,
                    false,
                    snapshot.state.halted
                )
            );
            break;
        }
        if Instant::now() >= deadline {
            return Err("Pause timed out.".into());
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    Ok(())
}
