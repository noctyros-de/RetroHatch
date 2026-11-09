//! Single-owner emulation worker, bounded real-time work, and latest-frame publication.
use super::{audio::AudioSink, roms};
use crate::core::cpu::e0c6200::CpuState;
use crate::core::device::{Button, Device, DisplaySnapshot};
use crate::core::mcu::MASTER_HZ;
use crate::core::p1::P1;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, mpsc};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

pub enum Command {
    Load(PathBuf),
    Run,
    Pause,
    Reset,
    Button(Button, bool),
    ReleaseInputs,
    Audio(bool),
    Quit,
}

#[derive(Clone)]
pub struct Snapshot {
    pub display: DisplaySnapshot,
    pub state: CpuState,
    pub loaded: bool,
    pub running: bool,
    pub rom_name: String,
    pub crc32: Option<u32>,
    pub known_world_p1: bool,
    pub ticks: u64,
    pub instructions: u64,
    pub interrupts: u64,
    pub error: Option<String>,
    pub audio_status: String,
    pub audio_drops: u64,
}
impl Default for Snapshot {
    fn default() -> Self {
        Self {
            display: DisplaySnapshot::default(),
            state: CpuState::default(),
            loaded: false,
            running: false,
            rom_name: String::new(),
            crc32: None,
            known_world_p1: false,
            ticks: 0,
            instructions: 0,
            interrupts: 0,
            error: None,
            audio_status: "Audio: no device loaded".into(),
            audio_drops: 0,
        }
    }
}

pub struct Controller {
    sender: mpsc::Sender<Command>,
    latest: Arc<Mutex<Snapshot>>,
    worker: Option<JoinHandle<()>>,
}
impl Controller {
    pub fn new(audio_enabled: bool) -> Self {
        let (sender, receiver) = mpsc::channel();
        let latest = Arc::new(Mutex::new(Snapshot::default()));
        let shared = latest.clone();
        let worker = thread::Builder::new()
            .name("RetroHatch P1".into())
            .spawn(move || run(receiver, shared, audio_enabled))
            .expect("cannot start emulation worker");
        Self {
            sender,
            latest,
            worker: Some(worker),
        }
    }
    pub fn send(&self, command: Command) {
        let _ = self.sender.send(command);
    }
    pub fn snapshot(&self) -> Snapshot {
        self.latest
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }
}
impl Drop for Controller {
    fn drop(&mut self) {
        let _ = self.sender.send(Command::Quit);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

fn publish(latest: &Mutex<Snapshot>, state: &mut Snapshot, device: Option<&P1>, running: bool) {
    state.loaded = device.is_some();
    state.running = running;
    if let Some(p1) = device {
        state.display = p1.display();
        state.state = p1.cpu_state().clone();
        state.ticks = p1.time();
        state.instructions = p1.mcu.instructions;
        state.interrupts = p1.mcu.interrupts;
    }
    *latest.lock().unwrap_or_else(|e| e.into_inner()) = state.clone();
}

fn run(receiver: mpsc::Receiver<Command>, latest: Arc<Mutex<Snapshot>>, mut enabled: bool) {
    let mut device: Option<P1> = None;
    let mut running = false;
    let mut state = Snapshot::default();
    let mut audio: Option<AudioSink> = None;
    let mut attempted_audio = false;
    let mut anchor = Instant::now();
    let mut anchor_tick = 0u64;
    let mut last_publish = Instant::now();
    loop {
        let first = match receiver.recv_timeout(Duration::from_millis(2)) {
            Ok(command) => Some(command),
            Err(mpsc::RecvTimeoutError::Timeout) => None,
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        };
        let mut commands = Vec::new();
        if let Some(command) = first {
            commands.push(command);
        }
        commands.extend(receiver.try_iter().take(256));
        let changed = !commands.is_empty();
        for command in commands {
            match command {
                Command::Quit => return,
                Command::Load(path) => {
                    let prepared = roms::load(&path).and_then(|rom| {
                        let known = rom.known_world_p1();
                        let crc = rom.crc32;
                        P1::new(&rom)
                            .map(|p1| (p1, crc, known))
                            .map_err(|e| e.to_string())
                    });
                    match prepared {
                        Ok((p1, crc, known)) => {
                            if let Some(audio) = &mut audio {
                                audio.clear();
                            }
                            device = Some(p1);
                            running = true;
                            state.error = None;
                            state.rom_name = path
                                .file_name()
                                .unwrap_or_default()
                                .to_string_lossy()
                                .into_owned();
                            state.crc32 = Some(crc);
                            state.known_world_p1 = known;
                            anchor = Instant::now();
                            anchor_tick = 0;
                        }
                        Err(error) => state.error = Some(error),
                    }
                }
                Command::Run => {
                    if let Some(p1) = &device {
                        running = true;
                        anchor = Instant::now();
                        anchor_tick = p1.time();
                    }
                }
                Command::Pause => {
                    running = false;
                    if let Some(p1) = &mut device {
                        p1.release_inputs();
                    }
                    if let Some(audio) = &mut audio {
                        audio.clear();
                    }
                }
                Command::Reset => {
                    if let Some(p1) = &mut device {
                        p1.reset();
                        running = true;
                        state.error = None;
                        anchor = Instant::now();
                        anchor_tick = p1.time();
                        if let Some(audio) = &mut audio {
                            audio.clear();
                        }
                    }
                }
                Command::Button(button, pressed) => {
                    if let Some(p1) = &mut device {
                        p1.set_button(button, pressed && running);
                    }
                }
                Command::ReleaseInputs => {
                    if let Some(p1) = &mut device {
                        p1.release_inputs();
                    }
                }
                Command::Audio(value) => {
                    enabled = value;
                    if let Some(audio) = &mut audio {
                        audio.clear();
                    }
                    if value {
                        attempted_audio = false;
                    }
                }
            }
        }
        if running && let Some(p1) = &mut device {
            let elapsed_ticks =
                (anchor.elapsed().as_nanos() * u128::from(MASTER_HZ) / 1_000_000_000) as u64;
            if anchor_tick.saturating_add(elapsed_ticks) > p1.time().saturating_add(MASTER_HZ / 4) {
                // Do not emulate arbitrary wall-clock gaps after suspension or debugging.
                anchor = Instant::now();
                anchor_tick = p1.time();
            }
            let requested = anchor_tick
                + (anchor.elapsed().as_nanos() * u128::from(MASTER_HZ) / 1_000_000_000) as u64;
            let target = requested.min(p1.time() + MASTER_HZ / 200);
            if let Err(error) = p1.advance_to(target) {
                state.error = Some(format!(
                    "Emulation stopped at PC {:04X}: {error}",
                    p1.cpu_state().pc
                ));
                running = false;
                p1.release_inputs();
                if let Some(audio) = &mut audio {
                    audio.clear();
                }
            }
            let samples = p1.drain_audio();
            if enabled && !attempted_audio {
                attempted_audio = true;
                match AudioSink::new() {
                    Ok(sink) => {
                        audio = Some(sink);
                        state.audio_status = "Audio: Windows PCM output active".into();
                    }
                    Err(error) => {
                        audio = None;
                        state.audio_status = error;
                    }
                }
            }
            if enabled
                && running
                && let Some(sink) = &mut audio
            {
                match sink.push(samples) {
                    Ok(true) => {}
                    Ok(false) => state.audio_drops += 1,
                    Err(error) => {
                        state.audio_status = error;
                        audio = None;
                    }
                }
            }
            if !enabled {
                state.audio_status = "Audio: muted".into();
            }
        }
        if changed || last_publish.elapsed() >= Duration::from_millis(16) {
            publish(&latest, &mut state, device.as_ref(), running);
            last_publish = Instant::now();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::p1::ROM_BYTES;
    fn wait(controller: &Controller, condition: impl Fn(&Snapshot) -> bool) -> Snapshot {
        let end = Instant::now() + Duration::from_secs(3);
        loop {
            let snapshot = controller.snapshot();
            if condition(&snapshot) {
                return snapshot;
            }
            assert!(
                Instant::now() < end,
                "worker state did not reach the expected condition"
            );
            thread::sleep(Duration::from_millis(5));
        }
    }
    #[test]
    fn user_rom_loading_pause_reset_and_failed_replacement_use_the_actual_worker() {
        // Entirely synthetic instructions, never copyrighted firmware.
        let folder = std::env::temp_dir().join(format!("retrohatch-worker-{}", std::process::id()));
        std::fs::create_dir_all(&folder).unwrap();
        let path = folder.join("synthetic-p1.bin");
        let mut bytes = vec![0; ROM_BYTES];
        for pair in bytes.chunks_exact_mut(2) {
            pair.copy_from_slice(&0xFFBu16.to_be_bytes());
        }
        bytes[0x200..0x202].copy_from_slice(&0xFF8u16.to_be_bytes());
        std::fs::write(&path, bytes).unwrap();
        let controller = Controller::new(false);
        controller.send(Command::Load(path.clone()));
        let ready = wait(&controller, |s| s.loaded && s.state.halted && s.ticks > 0);
        assert!(!ready.known_world_p1);
        controller.send(Command::Pause);
        let paused = wait(&controller, |s| !s.running);
        thread::sleep(Duration::from_millis(25));
        assert_eq!(controller.snapshot().ticks, paused.ticks);
        controller.send(Command::Load(folder.join("missing.bin")));
        let failed = wait(&controller, |s| s.error.is_some());
        assert_eq!(failed.crc32, ready.crc32);
        assert_eq!(failed.ticks, paused.ticks);
        controller.send(Command::Reset);
        wait(&controller, |s| s.running && s.error.is_none());
        drop(controller);
        std::fs::remove_file(path).unwrap();
        std::fs::remove_dir(folder).unwrap();
    }

    #[test]
    fn actual_worker_connects_cpu_gpio_interrupts_buttons_and_lcd_snapshots() {
        let description = include_str!("../../tests/data/e0c6200_mcu_programs.txt")
            .lines()
            .find(|line| line.starts_with("buttons_lcd|"))
            .unwrap();
        let fields: Vec<_> = description.split('|').collect();
        let mut bytes = vec![0; ROM_BYTES];
        for pair in bytes.chunks_exact_mut(2) {
            pair.copy_from_slice(&0xFFBu16.to_be_bytes());
        }
        for instruction in fields[2].split(',') {
            let (address, opcode) = instruction.split_once(':').unwrap();
            let offset = usize::from_str_radix(address, 16).unwrap() * 2;
            let opcode = u16::from_str_radix(opcode, 16).unwrap();
            bytes[offset..offset + 2].copy_from_slice(&opcode.to_be_bytes());
        }
        let folder =
            std::env::temp_dir().join(format!("retrohatch-connected-{}", std::process::id()));
        std::fs::create_dir_all(&folder).unwrap();
        let path = folder.join("synthetic-input-lcd.bin");
        std::fs::write(&path, bytes).unwrap();
        let controller = Controller::new(false);
        controller.send(Command::Load(path.clone()));
        wait(&controller, |s| {
            s.state.halted && s.display.nibbles[0] == 15
        });
        for (button, expected, interrupts) in
            [(Button::A, 11, 1), (Button::B, 13, 2), (Button::C, 14, 3)]
        {
            controller.send(Command::ReleaseInputs);
            controller.send(Command::Button(button, true));
            wait(&controller, |s| {
                s.display.nibbles[16] == expected && s.interrupts >= interrupts
            });
        }
        controller.send(Command::ReleaseInputs);
        controller.send(Command::Pause);
        wait(&controller, |s| !s.running);
        drop(controller);
        std::fs::remove_file(path).unwrap();
        std::fs::remove_dir(folder).unwrap();
    }
}
