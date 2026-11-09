//! Single-owner emulation worker, bounded real-time work, and latest-frame publication.
use super::performance::{
    CsvLog, Pacer, Performance, Telemetry, UiReads, WORK_TICKS, milliseconds,
};
use super::{
    audio::AudioSink,
    roms,
    saves::{SavePolicy, SaveTarget, Saves},
};
use crate::core::cpu::e0c6200::CpuState;
use crate::core::device::{Button, Device, DisplaySnapshot};
use crate::core::mcu::metrics::{CoreProfile, CycleCounters};
use crate::core::{profile::DeviceProfile, tamagotchi::Tamagotchi};
use std::path::PathBuf;
use std::sync::{Arc, Mutex, mpsc};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

pub enum Command {
    Load(PathBuf),
    LoadProfile(PathBuf, DeviceProfile),
    Run,
    Pause,
    Reset,
    Button(Button, bool),
    ReleaseInputs,
    Audio(bool),
    Profiling(bool),
    Save(SaveTarget),
    LoadSave(SaveTarget),
    Export(PathBuf),
    Import(PathBuf),
    SavePolicy(SavePolicy),
    PrepareExit,
    DiscardExit,
    ClearSaveError,
    Quit,
}

struct Envelope {
    command: Command,
    sent: Instant,
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
    pub profile: DeviceProfile,
    pub ticks: u64,
    pub instructions: u64,
    pub interrupts: u64,
    pub error: Option<String>,
    pub audio_status: String,
    pub audio_drops: u64,
    pub performance: Performance,
    pub cycles: CycleCounters,
    pub core_profile: CoreProfile,
    pub detailed_profiling: bool,
    pub diagnostics_error: Option<String>,
    pub save_error: Option<String>,
    pub save_message: Option<String>,
    pub saving: bool,
    pub loading: bool,
    pub exit_ready: bool,
    pub save_count: u64,
    pub restores: u64,
    pub loads: u64,
    pub slots: [bool; 3],
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
            profile: DeviceProfile::P1,
            ticks: 0,
            instructions: 0,
            interrupts: 0,
            error: None,
            audio_status: "Audio: no device loaded".into(),
            audio_drops: 0,
            performance: Performance::default(),
            cycles: CycleCounters::default(),
            core_profile: CoreProfile::default(),
            detailed_profiling: false,
            diagnostics_error: None,
            save_error: None,
            save_message: None,
            saving: false,
            loading: false,
            exit_ready: false,
            save_count: 0,
            restores: 0,
            loads: 0,
            slots: [false; 3],
        }
    }
}

pub struct Controller {
    sender: mpsc::Sender<Envelope>,
    latest: Arc<Mutex<Snapshot>>,
    worker: Option<JoinHandle<()>>,
    ui_reads: Arc<UiReads>,
}
impl Controller {
    pub fn new(audio_enabled: bool) -> Self {
        Self::start(audio_enabled, None)
    }
    pub fn with_storage(audio_enabled: bool, root: PathBuf, policy: SavePolicy) -> Self {
        Self::start(audio_enabled, Some((root, policy)))
    }
    fn start(audio_enabled: bool, storage: Option<(PathBuf, SavePolicy)>) -> Self {
        let (sender, receiver) = mpsc::channel();
        let latest = Arc::new(Mutex::new(Snapshot::default()));
        let shared = latest.clone();
        let ui_reads = Arc::new(UiReads::default());
        let worker_ui_reads = ui_reads.clone();
        let worker = thread::Builder::new()
            .name("RetroHatch MCU".into())
            .spawn(move || run(receiver, shared, worker_ui_reads, audio_enabled, storage))
            .expect("cannot start emulation worker");
        Self {
            sender,
            latest,
            worker: Some(worker),
            ui_reads,
        }
    }
    pub fn send(&self, command: Command) {
        let _ = self.sender.send(Envelope {
            command,
            sent: Instant::now(),
        });
    }
    pub fn snapshot(&self) -> Snapshot {
        let start = Instant::now();
        let guard = self.latest.lock().unwrap_or_else(|e| e.into_inner());
        self.ui_reads.record(start.elapsed().as_nanos() as u64);
        guard.clone()
    }
}
impl Drop for Controller {
    fn drop(&mut self) {
        self.send(Command::Quit);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

fn publish(
    latest: &Mutex<Snapshot>,
    state: &mut Snapshot,
    device: Option<&Tamagotchi>,
    running: bool,
    pacer: &Pacer,
    telemetry: &mut Telemetry,
    ui: &UiReads,
) {
    let started = Instant::now();
    state.loaded = device.is_some();
    state.running = running;
    if let Some(p1) = device {
        state.display = p1.display();
        state.state = p1.cpu_state().clone();
        state.ticks = p1.time();
        state.instructions = p1.mcu.instructions;
        state.interrupts = p1.mcu.interrupts;
        state.cycles = p1.mcu.counters;
        state.core_profile = p1.mcu.profile;
        let now = Instant::now();
        state.performance = telemetry.sample(
            now,
            p1.time(),
            if running {
                pacer.target(now)
            } else {
                p1.time()
            },
            p1.mcu.instructions,
            p1.mcu.counters,
            ui,
        );
    }
    telemetry.values.snapshot_ms += milliseconds(started);
    state.performance.snapshot_ms = telemetry.values.snapshot_ms;
    let lock_start = Instant::now();
    let mut latest = latest.lock().unwrap_or_else(|e| e.into_inner());
    telemetry.values.snapshot_lock_ms += milliseconds(lock_start);
    state.performance.snapshot_lock_ms = telemetry.values.snapshot_lock_ms;
    *latest = state.clone();
}

fn run(
    receiver: mpsc::Receiver<Envelope>,
    latest: Arc<Mutex<Snapshot>>,
    ui_reads: Arc<UiReads>,
    mut enabled: bool,
    storage: Option<(PathBuf, SavePolicy)>,
) {
    let mut saves = storage.map(|(root, policy)| Saves::new(root, policy));
    let mut exit_requested = false;
    let mut volume = saves.as_ref().map_or(100, |s| s.policy.volume);
    let mut device: Option<Tamagotchi> = None;
    let mut running = false;
    let mut state = Snapshot::default();
    let mut audio: Option<AudioSink> = None;
    let mut attempted_audio = false;
    let mut pacer = Pacer::new(Instant::now(), 0);
    let mut telemetry = Telemetry::new(0);
    let mut last_publish = Instant::now();
    let mut detailed = std::env::var("RETROHATCH_PROFILE_CORE").is_ok_and(|v| v == "1");
    state.detailed_profiling = detailed;
    let mut log = match CsvLog::from_environment() {
        Ok(log) => log,
        Err(error) => {
            state.diagnostics_error = Some(error);
            None
        }
    };
    loop {
        if let Some(store) = &mut saves {
            if store.poll(&mut device, &mut state, &mut running) {
                if let Some(d) = &mut device {
                    d.mcu.set_profiling(detailed);
                    let now = Instant::now();
                    pacer = Pacer::new(now, d.time());
                    telemetry = Telemetry::new(d.time());
                    if running {
                        telemetry.resume(now);
                    }
                    if let Some(audio) = &mut audio {
                        audio.clear();
                    }
                }
                if let Some(log) = &mut log {
                    log.reset_epoch();
                }
            }
            if exit_requested && !state.loading {
                exit_requested = false;
                running = false;
                telemetry.pause(Instant::now());
                if let Err(e) = store.exit(device.as_ref()) {
                    state.save_error = Some(e);
                }
            }
            if let Some(d) = &device
                && let Err(e) = store.automatic_due(d, running && !state.loading)
            {
                state.save_error = Some(e);
                store.reset_timer();
            }
        }
        // A Windows timeout may wake after ~15.6 ms. Never sleep again while behind.
        let wait = if running {
            device
                .as_ref()
                .map_or(Duration::ZERO, |p1| pacer.wait(Instant::now(), p1.time()))
        } else {
            Duration::from_millis(50)
        };
        let waiting = Instant::now();
        let first = if wait.is_zero() {
            match receiver.try_recv() {
                Ok(command) => Some(command),
                Err(mpsc::TryRecvError::Empty) => None,
                Err(mpsc::TryRecvError::Disconnected) => break,
            }
        } else {
            match receiver.recv_timeout(wait) {
                Ok(command) => Some(command),
                Err(mpsc::RecvTimeoutError::Timeout) => None,
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
            }
        };
        if running && !wait.is_zero() {
            let elapsed = milliseconds(waiting);
            telemetry.values.wait_ms += elapsed;
            telemetry.values.max_wait_ms = telemetry.values.max_wait_ms.max(elapsed);
            telemetry.values.wait_calls += 1;
        }
        let command_start = Instant::now();
        let mut changed = first.is_some();
        for envelope in first.into_iter().chain(receiver.try_iter().take(256)) {
            changed = true;
            telemetry.observe_command(envelope.sent, Instant::now());
            match envelope.command {
                Command::Quit => {
                    if let Some(store) = &mut saves {
                        if !state.exit_ready {
                            if let Err(e) = store.exit(device.as_ref()) {
                                state.save_error = Some(e);
                            }
                        }
                        store.finish();
                        store.poll(&mut device, &mut state, &mut running);
                    }
                    telemetry.pause(Instant::now());
                    publish(
                        &latest,
                        &mut state,
                        device.as_ref(),
                        false,
                        &pacer,
                        &mut telemetry,
                        &ui_reads,
                    );
                    if let Some(log) = &mut log {
                        let _ = log.write_due(
                            &state.performance,
                            state.cycles,
                            state.core_profile,
                            state.instructions,
                            false,
                            state.state.halted,
                        );
                    }
                    return;
                }
                command @ (Command::Load(_) | Command::LoadProfile(..)) => {
                    if state.loading || exit_requested {
                        state.save_error=Some("Please wait for saved progress to finish loading before switching ROMs.".into());
                        continue;
                    }
                    let (path, profile) = match command {
                        Command::Load(path) => (path, DeviceProfile::P1),
                        Command::LoadProfile(path, profile) => (path, profile),
                        _ => unreachable!(),
                    };
                    let prepared = roms::load_for_profile(&path, profile).and_then(|rom| {
                        let known = rom.known_world_p1();
                        let crc = rom.crc32;
                        Tamagotchi::new(&rom)
                            .map(|p1| (p1, crc, known))
                            .map_err(|e| e.to_string())
                    });
                    match prepared {
                        Ok((mut p1, crc, known)) => {
                            if let Some(store) = &mut saves
                                && store.policy.save_on_exit
                                && store.can_capture()
                                && let Some(old) = &device
                                && let Err(e) = store.save(old, SaveTarget::Autosave)
                            {
                                state.save_error = Some(e);
                                continue;
                            }
                            if let Some(audio) = &mut audio {
                                audio.clear();
                            }
                            p1.mcu.set_profiling(detailed);
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
                            state.profile = profile;
                            state.loads += 1;
                            state.exit_ready = false;
                            state.save_error = None;
                            state.slots = [false; 3];
                            if let Some(store) = &mut saves {
                                running = false;
                                state.loading = true;
                                if let Err(e) = store.start(device.as_ref().unwrap(), &mut state) {
                                    state.save_error = Some(e);
                                    state.loading = false;
                                }
                            }
                            let now = Instant::now();
                            pacer = Pacer::new(now, 0);
                            telemetry = Telemetry::new(0);
                            if running {
                                telemetry.resume(now);
                            }
                            if let Some(log) = &mut log {
                                log.reset_epoch();
                            }
                        }
                        Err(error) => state.error = Some(error),
                    }
                }
                Command::Run => {
                    if !running
                        && !state.loading
                        && let Some(p1) = &device
                    {
                        if let Some(store) = &mut saves {
                            store.allow_fresh_session();
                        }
                        let now = Instant::now();
                        running = true;
                        pacer = Pacer::new(now, p1.time());
                        telemetry.resume(now);
                    }
                }
                Command::Pause => {
                    running = false;
                    telemetry.pause(Instant::now());
                    if let Some(p1) = &mut device {
                        p1.release_inputs();
                    }
                    if let Some(audio) = &mut audio {
                        audio.clear();
                    }
                }
                Command::Reset => {
                    if let Some(store) = &mut saves {
                        store.cancel_restore();
                    }
                    state.loading = false;
                    if let Some(p1) = &mut device {
                        p1.reset();
                        running = true;
                        state.error = None;
                        let now = Instant::now();
                        pacer = Pacer::new(now, p1.time());
                        telemetry = Telemetry::new(p1.time());
                        telemetry.resume(now);
                        if let Some(log) = &mut log {
                            log.reset_epoch();
                        }
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
                Command::Save(target) => {
                    let result = match (&mut saves, &device) {
                        (Some(store), Some(d)) => store.save(d, target),
                        _ => Err("No persistent device is loaded".into()),
                    };
                    if let Err(e) = result {
                        state.save_error = Some(e);
                    }
                }
                Command::LoadSave(target) => {
                    let result = match (&mut saves, &device) {
                        (Some(store), Some(d)) => target
                            .filename()
                            .and_then(|n| store.load(d, store.directory(d).join(n), false)),
                        _ => Err("No persistent device is loaded".into()),
                    };
                    if let Err(e) = result {
                        state.save_error = Some(e);
                    }
                }
                Command::Export(path) => {
                    let result = match (&mut saves, &device) {
                        (Some(store), Some(d)) => store.export(d, path),
                        _ => Err("No persistent device is loaded".into()),
                    };
                    if let Err(e) = result {
                        state.save_error = Some(e);
                    }
                }
                Command::Import(path) => {
                    let result = match (&mut saves, &device) {
                        (Some(store), Some(d)) => store.load(d, path, false),
                        _ => Err("No persistent device is loaded".into()),
                    };
                    if let Err(e) = result {
                        state.save_error = Some(e);
                    }
                }
                Command::SavePolicy(policy) => {
                    volume = policy.volume.min(100);
                    if let Some(store) = &mut saves {
                        store.configure(policy);
                    }
                }
                Command::PrepareExit => {
                    running = false;
                    telemetry.pause(Instant::now());
                    state.exit_ready = false;
                    state.save_error = None;
                    if let Some(audio) = &mut audio {
                        audio.clear();
                    }
                    if state.loading {
                        exit_requested = true;
                    } else if let Some(store) = &mut saves {
                        if let Err(e) = store.exit(device.as_ref()) {
                            state.save_error = Some(e);
                        }
                    } else {
                        state.exit_ready = true;
                    }
                }
                Command::DiscardExit => {
                    exit_requested = false;
                    if let Some(store) = &mut saves {
                        store.finish();
                        store.poll(&mut device, &mut state, &mut running);
                    }
                    running = false;
                    state.save_error = None;
                    state.exit_ready = true;
                }
                Command::ClearSaveError => {
                    state.save_error = None;
                    state.error = None;
                }
                Command::Profiling(value) => {
                    detailed = value;
                    state.detailed_profiling = value;
                    if let Some(p1) = &mut device {
                        p1.mcu.set_profiling(value);
                    }
                }
            }
        }
        if running {
            telemetry.values.commands_ms += milliseconds(command_start);
            telemetry.values.iterations += 1;
        }
        if running && let Some(p1) = &mut device {
            let requested = pacer.target(Instant::now());
            let current = p1.time();
            if requested > current {
                telemetry.values.work_batches += 1;
                if requested - current > WORK_TICKS {
                    telemetry.values.catchup_batches += 1;
                }
                let core_start = Instant::now();
                if let Err(error) = p1.advance_to(Pacer::chunk(current, requested)) {
                    state.error = Some(format!(
                        "Emulation stopped at PC {:04X}: {error}",
                        p1.cpu_state().pc
                    ));
                    running = false;
                    telemetry.pause(Instant::now());
                    p1.release_inputs();
                    if let Some(audio) = &mut audio {
                        audio.clear();
                    }
                }
                telemetry.values.core_ms += milliseconds(core_start);
                let audio_start = Instant::now();
                let mut samples = p1.drain_audio();
                super::audio::apply_volume(&mut samples, volume);
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
                telemetry.values.audio_ms += milliseconds(audio_start);
            }
        }
        if changed || last_publish.elapsed() >= Duration::from_millis(16) {
            publish(
                &latest,
                &mut state,
                device.as_ref(),
                running,
                &pacer,
                &mut telemetry,
                &ui_reads,
            );
            last_publish = Instant::now();
            if state.loaded
                && let Some(csv) = &mut log
                && let Err(error) = csv.write_due(
                    &state.performance,
                    state.cycles,
                    state.core_profile,
                    state.instructions,
                    running,
                    state.state.halted,
                )
            {
                state.diagnostics_error = Some(error);
                log = None;
            }
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

    #[test]
    fn worker_keeps_real_time_without_ui_polling_or_focus_and_repeated_run_keeps_its_epoch() {
        let folder =
            std::env::temp_dir().join(format!("retrohatch-background-{}", std::process::id()));
        std::fs::create_dir_all(&folder).unwrap();
        let path = folder.join("synthetic-halt.bin");
        let mut bytes = vec![0; ROM_BYTES];
        bytes[0x200..0x202].copy_from_slice(&0xFF8u16.to_be_bytes());
        std::fs::write(&path, bytes).unwrap();
        let controller = Controller::new(false);
        controller.send(Command::Load(path.clone()));
        wait(&controller, |s| s.loaded && s.state.halted);
        // Deliberately stop polling snapshots, as when the UI is minimized.
        thread::sleep(Duration::from_secs(1));
        controller.send(Command::ReleaseInputs);
        controller.send(Command::Run); // Already running: must not hide lag or reset elapsed time.
        let live = wait(&controller, |s| s.performance.wall_seconds >= 1.0);
        assert!(live.running);
        assert!(
            (live.performance.emulated_seconds - live.performance.wall_seconds).abs() < 0.1,
            "{:?}",
            live.performance
        );
        assert_eq!(live.cycles.instructions, 5);
        assert!(live.cycles.halt > 28000);
        assert_eq!(live.cycles.fast, 0);
        assert!(live.cycles.osc1_edges > 28000);
        assert!(live.cycles.audio_samples > 38000);
        controller.send(Command::Pause);
        let paused = wait(&controller, |s| !s.running);
        thread::sleep(Duration::from_millis(40));
        let later = controller.snapshot();
        assert_eq!(paused.ticks, later.ticks);
        assert_eq!(
            paused.performance.wall_seconds,
            later.performance.wall_seconds
        );
        drop(controller);
        std::fs::remove_file(path).unwrap();
        std::fs::remove_dir(folder).unwrap();
    }
    #[test]
    fn genjintch_worker_knock_irq_pause_reset_failed_load_and_switch_back_to_p1() {
        let folder =
            std::env::temp_dir().join(format!("retrohatch-genjintch-{}", std::process::id()));
        std::fs::create_dir_all(&folder).unwrap();
        let path = folder.join("misleading-p1-name.bin");
        let line = include_str!("../../tests/data/genjintch_programs.txt")
            .lines()
            .find(|s| s.starts_with("genjintch_inputs|"))
            .unwrap();
        let mut bytes = vec![0; DeviceProfile::Genjintch.rom_bytes()];
        for pair in bytes.chunks_exact_mut(2) {
            pair.copy_from_slice(&0xFFBu16.to_be_bytes());
        }
        for item in line.split('|').nth(2).unwrap().split(',') {
            let (a, v) = item.split_once(':').unwrap();
            let offset = usize::from_str_radix(a, 16).unwrap() * 2;
            bytes[offset..offset + 2]
                .copy_from_slice(&u16::from_str_radix(v, 16).unwrap().to_be_bytes());
        }
        std::fs::write(&path, bytes).unwrap();
        let c = Controller::new(false);
        c.send(Command::LoadProfile(path.clone(), DeviceProfile::Genjintch));
        let initial = wait(&c, |s| {
            s.loaded && s.state.halted && s.display.nibbles[0] == 15
        });
        assert_eq!(initial.profile, DeviceProfile::Genjintch);
        c.send(Command::Button(Button::Knock, true));
        wait(&c, |s| s.display.nibbles[16] == 7 && s.interrupts >= 1);
        c.send(Command::Pause);
        let paused = wait(&c, |s| !s.running);
        c.send(Command::Load(path.clone())); // Explicit P1 assignment rejects a Genjintch-sized image.
        let failed = wait(&c, |s| s.error.is_some());
        assert_eq!(failed.profile, DeviceProfile::Genjintch);
        assert_eq!(failed.ticks, paused.ticks);
        c.send(Command::Reset);
        wait(&c, |s| {
            s.running && s.error.is_none() && s.state.halted && s.display.nibbles[16] == 0
        });
        c.send(Command::Button(Button::Knock, true));
        wait(&c, |s| s.display.nibbles[16] == 7);
        let mut p1 = vec![0; ROM_BYTES];
        p1[0x200..0x202].copy_from_slice(&0xFF8u16.to_be_bytes());
        std::fs::write(&path, p1).unwrap();
        c.send(Command::LoadProfile(path.clone(), DeviceProfile::P1));
        wait(&c, |s| {
            s.profile == DeviceProfile::P1 && s.state.halted && s.error.is_none()
        });
        drop(c);
        std::fs::remove_file(path).unwrap();
        std::fs::remove_dir(folder).unwrap();
    }
    #[test]
    fn angel_worker_knock_irq_pause_reset_failed_load_and_switch_back_to_p1() {
        let folder = std::env::temp_dir().join(format!("retrohatch-angel-{}", std::process::id()));
        std::fs::create_dir_all(&folder).unwrap();
        let path = folder.join("misleading-p1-name.bin");
        let line = include_str!("../../tests/data/genjintch_programs.txt")
            .lines()
            .find(|s| s.starts_with("genjintch_inputs|"))
            .unwrap();
        let mut bytes = vec![0; DeviceProfile::Angel.rom_bytes()];
        for pair in bytes.chunks_exact_mut(2) {
            pair.copy_from_slice(&0xFFBu16.to_be_bytes());
        }
        for item in line.split('|').nth(2).unwrap().split(',') {
            let (a, v) = item.split_once(':').unwrap();
            let offset = usize::from_str_radix(a, 16).unwrap() * 2;
            bytes[offset..offset + 2]
                .copy_from_slice(&u16::from_str_radix(v, 16).unwrap().to_be_bytes());
        }
        std::fs::write(&path, bytes).unwrap();
        let c = Controller::new(false);
        c.send(Command::LoadProfile(path.clone(), DeviceProfile::Angel));
        let initial = wait(&c, |s| {
            s.loaded && s.state.halted && s.display.nibbles[0] == 15
        });
        assert_eq!(initial.profile, DeviceProfile::Angel);
        c.send(Command::Button(Button::Knock, true));
        wait(&c, |s| s.display.nibbles[16] == 7 && s.interrupts >= 1);
        c.send(Command::Pause);
        let paused = wait(&c, |s| !s.running);
        c.send(Command::Load(path.clone())); // Explicit P1 assignment rejects a Angel-sized image.
        let failed = wait(&c, |s| s.error.is_some());
        assert_eq!(failed.profile, DeviceProfile::Angel);
        assert_eq!(failed.ticks, paused.ticks);
        c.send(Command::Reset);
        wait(&c, |s| {
            s.running && s.error.is_none() && s.state.halted && s.display.nibbles[16] == 0
        });
        c.send(Command::Button(Button::Knock, true));
        wait(&c, |s| s.display.nibbles[16] == 7);
        let mut p1 = vec![0; ROM_BYTES];
        p1[0x200..0x202].copy_from_slice(&0xFF8u16.to_be_bytes());
        std::fs::write(&path, p1).unwrap();
        c.send(Command::LoadProfile(path.clone(), DeviceProfile::P1));
        wait(&c, |s| {
            s.profile == DeviceProfile::P1 && s.state.halted && s.error.is_none()
        });
        drop(c);
        std::fs::remove_file(path).unwrap();
        std::fs::remove_dir(folder).unwrap();
    }
}

#[cfg(test)]
mod save_tests {
    use super::*;
    fn wait(c: &Controller, condition: impl Fn(&Snapshot) -> bool) -> Snapshot {
        let end = Instant::now() + Duration::from_secs(5);
        loop {
            let s = c.snapshot();
            if condition(&s) {
                return s;
            }
            assert!(
                Instant::now() < end,
                "State did not arrive: running={}, loading={}, saves={}, restored={}, error={:?}",
                s.running,
                s.loading,
                s.save_count,
                s.restores,
                s.save_error
            );
            thread::sleep(Duration::from_millis(5));
        }
    }
    #[test]
    fn real_worker_slots_export_import_failed_write_and_restart_resume_are_transactional() {
        let root =
            std::env::temp_dir().join(format!("retrohatch-state-worker-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let path = root.join("synthetic.bin");
        let mut rom = vec![0; 0x4000];
        rom[0x200..0x202].copy_from_slice(&0xFF8u16.to_be_bytes());
        std::fs::write(&path, rom).unwrap();
        let policy = SavePolicy {
            autosave_minutes: 0,
            save_on_exit: true,
            volume: 100,
        };
        let c = Controller::with_storage(false, root.clone(), policy.clone());
        c.send(Command::LoadProfile(path.clone(), DeviceProfile::Angel));
        wait(&c, |s| s.loaded && !s.loading && s.state.halted);
        c.send(Command::Pause);
        let paused = wait(&c, |s| !s.running);
        for slot in 1..=3 {
            c.send(Command::Save(SaveTarget::Slot(slot)));
            wait(&c, |s| s.save_count >= u64::from(slot));
        }
        let exported = root.join("export.rhstate");
        c.send(Command::Export(exported.clone()));
        wait(&c, |s| s.save_count >= 4);
        let saved = std::fs::read(&exported).unwrap();
        c.send(Command::Reset);
        wait(&c, |s| s.running && s.ticks > paused.ticks);
        c.send(Command::Pause);
        wait(&c, |s| !s.running);
        c.send(Command::LoadSave(SaveTarget::Slot(2)));
        let loaded = wait(&c, |s| s.restores >= 1);
        assert_eq!(loaded.ticks, paused.ticks);
        assert_eq!(loaded.state, paused.state);
        assert!(!loaded.running);
        c.send(Command::Export(root.join("roundtrip.rhstate")));
        wait(&c, |s| s.save_count >= 5);
        assert_eq!(
            saved,
            std::fs::read(root.join("roundtrip.rhstate")).unwrap()
        );
        c.send(Command::Import(exported));
        wait(&c, |s| s.restores >= 2);
        let rom_before = std::fs::read(&path).unwrap();
        c.send(Command::Export(path.clone()));
        let bad = wait(&c, |s| s.save_error.is_some());
        assert_eq!(bad.ticks, paused.ticks);
        assert_eq!(std::fs::read(&path).unwrap(), rom_before);
        c.send(Command::ClearSaveError);
        c.send(Command::PrepareExit);
        wait(&c, |s| s.exit_ready);
        drop(c);
        let again = Controller::with_storage(false, root.clone(), policy);
        again.send(Command::LoadProfile(path.clone(), DeviceProfile::Angel));
        let resumed = wait(&again, |s| s.restores >= 1 && !s.loading);
        assert!(resumed.ticks >= paused.ticks);
        assert!(resumed.ticks - paused.ticks < crate::core::mcu::MASTER_HZ / 5);
        assert_eq!(resumed.state, paused.state);
        again.send(Command::PrepareExit);
        wait(&again, |s| s.exit_ready);
        drop(again);
        assert_eq!(root.parent(), Some(std::env::temp_dir().as_path()));
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn corrupt_autosave_is_kept_on_failed_resume_and_exit_does_not_replace_it() {
        use crate::core::p1::RomImage;
        let root =
            std::env::temp_dir().join(format!("retrohatch-corrupt-save-{}", std::process::id()));
        let mut bytes = vec![0; 0x3000];
        bytes[0x200..0x202].copy_from_slice(&0xFF8u16.to_be_bytes());
        let d = Tamagotchi::new(&RomImage::decode(&bytes).unwrap()).unwrap();
        let dir = root.join("saves").join(d.identity());
        std::fs::create_dir_all(&dir).unwrap();
        let path = root.join("synthetic.bin");
        std::fs::write(&path, bytes).unwrap();
        let mut bad = d.save_state().unwrap();
        bad[100] ^= 1;
        let auto = dir.join("autosave.rhstate");
        std::fs::write(&auto, &bad).unwrap();
        let c = Controller::with_storage(false, root.clone(), SavePolicy::default());
        c.send(Command::Load(path));
        let failed = wait(&c, |s| s.save_error.is_some() && !s.loading);
        assert!(!failed.running);
        c.send(Command::PrepareExit);
        let exit = wait(&c, |s| {
            s.save_error
                .as_ref()
                .is_some_and(|e| e.contains("not restored"))
        });
        assert!(!exit.exit_ready);
        drop(c);
        assert_eq!(std::fs::read(auto).unwrap(), bad);
        assert_eq!(root.parent(), Some(std::env::temp_dir().as_path()));
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn closing_while_auto_resume_is_pending_saves_restored_progress_instead_of_reset_state() {
        use crate::core::p1::RomImage;
        let root =
            std::env::temp_dir().join(format!("retrohatch-early-close-{}", std::process::id()));
        let mut bytes = vec![0; 0x3000];
        bytes[0x200..0x202].copy_from_slice(&0xFF8u16.to_be_bytes());
        let mut d = Tamagotchi::new(&RomImage::decode(&bytes).unwrap()).unwrap();
        d.advance_to(crate::core::mcu::MASTER_HZ / 10).unwrap();
        d.drain_audio();
        let saved = d.save_state().unwrap();
        let dir = root.join("saves").join(d.identity());
        std::fs::create_dir_all(&dir).unwrap();
        let auto = dir.join("autosave.rhstate");
        std::fs::write(&auto, &saved).unwrap();
        let path = root.join("synthetic.bin");
        std::fs::write(&path, bytes).unwrap();
        let c = Controller::with_storage(false, root.clone(), SavePolicy::default());
        c.send(Command::Load(path));
        c.send(Command::PrepareExit);
        wait(&c, |s| s.exit_ready);
        drop(c);
        assert_eq!(std::fs::read(auto).unwrap(), saved);
        assert_eq!(root.parent(), Some(std::env::temp_dir().as_path()));
        std::fs::remove_dir_all(root).unwrap();
    }
}
