// SPDX-License-Identifier: GPL-3.0-only
// SPDX-FileCopyrightText: 2026 Noctyros
//! Explicit native codec acceptance probe, with synthetic colors rather than firmware.
use retrohatch::host::{
    recording::Session,
    screenshots::{Image, Kind},
};
use std::{
    path::PathBuf,
    time::{Duration, Instant},
};
fn main() -> Result<(), String> {
    let root = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .ok_or("Pass a private output directory")?;
    if let Some(rom) = std::env::args_os().nth(2) {
        return firmware(root, PathBuf::from(rom));
    }
    let mut session = Session::start(&root, "codec-test", Kind::Lcd)?;
    for frame in 0..30 {
        let mut rgb = vec![0; 128 * 128 * 3];
        for y in 0..128 {
            for x in 0..128 {
                let i = (y * 128 + x) * 3;
                rgb[i..i + 3].copy_from_slice(if y < 64 { &[255, 0, 0] } else { &[0, 0, 255] });
            }
        }
        session.submit(Image::new(128, 128, rgb)?);
        if frame < 29 {
            std::thread::sleep(Duration::from_secs_f64(1.0 / 30.0));
        }
    }
    session.stop();
    let until = Instant::now() + Duration::from_secs(15);
    while Instant::now() < until {
        if let Some(result) = session.poll() {
            let done = result?;
            println!(
                "{} | {} frames | {:.3}s | repeated {} | dropped {}",
                done.path.display(),
                done.frames,
                done.elapsed.as_secs_f64(),
                done.repeated,
                done.dropped
            );
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    Err("Recording finalization timed out".into())
}

fn firmware(root: PathBuf, rom: PathBuf) -> Result<(), String> {
    use retrohatch::{
        core::{device::Button, profile::DeviceProfile},
        host::{
            runner::{Command, Controller},
            screenshots,
        },
    };
    let controller = Controller::new(false);
    controller.send(Command::LoadProfile(rom, DeviceProfile::Ps));
    let begin = Instant::now();
    let mut last_input = Instant::now();
    let mut down = false;
    let snapshot = loop {
        let s = controller.snapshot();
        if let Some(e) = s.error {
            return Err(e);
        }
        if s.loaded {
            controller.send(Command::KeepScreenAwake(true));
            if last_input.elapsed() > Duration::from_millis(if down { 80 } else { 1000 }) {
                down = !down;
                last_input = Instant::now();
                controller.send(Command::Button(Button::A, down));
            }
            if s.color_display
                .as_ref()
                .is_some_and(|p| p.iter().any(|v| *v > 20))
            {
                break s;
            }
        }
        if begin.elapsed() > Duration::from_secs(30) {
            return Err(format!(
                "Color firmware display stayed black; PC {:?}",
                s.arm_pc
            ));
        }
        std::thread::sleep(Duration::from_millis(20));
    };
    controller.send(Command::Button(Button::A, false));
    let png = screenshots::save(
        &root,
        "tamagotchi-ps",
        Kind::Lcd,
        Image::new(128, 128, snapshot.color_display.unwrap().as_ref().clone())?,
    )?;
    let mut session = Session::start(&root, "tamagotchi-ps", Kind::Lcd)?;
    for _ in 0..150 {
        let s = controller.snapshot();
        if let Some(rgb) = s.color_display {
            session.submit(Image::new(128, 128, rgb.as_ref().clone())?);
        }
        std::thread::sleep(Duration::from_secs_f64(1. / 30.));
    }
    session.stop();
    let until = Instant::now() + Duration::from_secs(10);
    while Instant::now() < until {
        if let Some(result) = session.poll() {
            let done = result?;
            let s = controller.snapshot();
            println!(
                "Native firmware PNG: {}\nVideo: {} | {} frames | {:.3}s | speed {:.3}%",
                png.display(),
                done.path.display(),
                done.frames,
                done.elapsed.as_secs_f64(),
                s.performance.average_speed_percent
            );
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    Err("Firmware recording finalization timed out".into())
}
