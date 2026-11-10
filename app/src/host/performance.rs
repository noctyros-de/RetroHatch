// SPDX-License-Identifier: GPL-3.0-only
// SPDX-FileCopyrightText: 2026 Noctyros
// Applies to original RetroHatch contributions; third-party provenance is retained below and in THIRD_PARTY_LICENSES.md.
//! Monotonic real-time pacing and low-frequency observational telemetry.
use crate::core::mcu::{MASTER_HZ, metrics::CycleCounters};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

pub const WORK_TICKS: u64 = MASTER_HZ / 200;
const IDLE_WAIT: Duration = Duration::from_millis(2);

pub struct Pacer {
    epoch: Instant,
    tick: u64,
}
impl Pacer {
    pub fn new(now: Instant, tick: u64) -> Self {
        Self { epoch: now, tick }
    }
    pub fn target(&self, now: Instant) -> u64 {
        let ticks = now.saturating_duration_since(self.epoch).as_nanos() * u128::from(MASTER_HZ)
            / 1_000_000_000;
        self.tick
            .saturating_add(ticks.min(u128::from(u64::MAX)) as u64)
    }
    pub fn wait(&self, now: Instant, current: u64) -> Duration {
        if current < self.target(now) {
            Duration::ZERO
        } else {
            IDLE_WAIT
        }
    }
    pub fn chunk(current: u64, target: u64) -> u64 {
        target.min(current.saturating_add(WORK_TICKS))
    }
}

#[derive(Default)]
pub struct UiReads {
    pub count: AtomicU64,
    pub lock_ns: AtomicU64,
}
impl UiReads {
    pub fn record(&self, lock_ns: u64) {
        self.count.fetch_add(1, Ordering::Relaxed);
        self.lock_ns.fetch_add(lock_ns, Ordering::Relaxed);
    }
}

#[derive(Clone, Debug, Default)]
pub struct Performance {
    pub wall_seconds: f64,
    pub emulated_seconds: f64,
    pub speed_percent: f64,
    pub average_speed_percent: f64,
    pub lag_ms: f64,
    pub instructions_per_second: f64,
    pub cycles_per_second: f64,
    pub iterations_per_second: f64,
    pub iterations: u64,
    pub work_batches: u64,
    pub catchup_batches: u64,
    pub wait_calls: u64,
    pub wait_ms: f64,
    pub max_wait_ms: f64,
    pub core_ms: f64,
    pub audio_ms: f64,
    pub snapshot_ms: f64,
    pub snapshot_lock_ms: f64,
    pub commands_ms: f64,
    pub command_latency_max_ms: f64,
    pub ui_reads: u64,
    pub ui_lock_ms: f64,
}

pub struct Telemetry {
    base_tick: u64,
    resumed: Option<Instant>,
    active: Duration,
    window_wall: f64,
    window_tick: u64,
    window_instructions: u64,
    window_cycles: u64,
    window_iterations: u64,
    pub values: Performance,
}
impl Telemetry {
    pub fn new(tick: u64) -> Self {
        Self {
            base_tick: tick,
            resumed: None,
            active: Duration::ZERO,
            window_wall: 0.0,
            window_tick: tick,
            window_instructions: 0,
            window_cycles: 0,
            window_iterations: 0,
            values: Performance::default(),
        }
    }
    pub fn resume(&mut self, now: Instant) {
        if self.resumed.is_none() {
            self.resumed = Some(now);
        }
    }
    pub fn pause(&mut self, now: Instant) {
        if let Some(start) = self.resumed.take() {
            self.active += now.saturating_duration_since(start);
        }
    }
    pub fn observe_command(&mut self, sent: Instant, now: Instant) {
        self.values.command_latency_max_ms = self
            .values
            .command_latency_max_ms
            .max(now.saturating_duration_since(sent).as_secs_f64() * 1000.0);
    }
    pub fn sample(
        &mut self,
        now: Instant,
        ticks: u64,
        target: u64,
        instructions: u64,
        cycles: CycleCounters,
        ui: &UiReads,
    ) -> Performance {
        let wall = (self.active
            + self
                .resumed
                .map_or(Duration::ZERO, |start| now.saturating_duration_since(start)))
        .as_secs_f64();
        let elapsed = wall - self.window_wall;
        if elapsed >= 0.5 {
            self.values.speed_percent =
                (ticks.saturating_sub(self.window_tick)) as f64 / MASTER_HZ as f64 / elapsed
                    * 100.0;
            self.values.instructions_per_second =
                instructions.saturating_sub(self.window_instructions) as f64 / elapsed;
            self.values.cycles_per_second =
                cycles.total().saturating_sub(self.window_cycles) as f64 / elapsed;
            self.values.iterations_per_second =
                self.values
                    .iterations
                    .saturating_sub(self.window_iterations) as f64
                    / elapsed;
            self.window_wall = wall;
            self.window_tick = ticks;
            self.window_instructions = instructions;
            self.window_cycles = cycles.total();
            self.window_iterations = self.values.iterations;
        }
        let emulated = ticks.saturating_sub(self.base_tick) as f64 / MASTER_HZ as f64;
        self.values.wall_seconds = wall;
        self.values.emulated_seconds = emulated;
        self.values.average_speed_percent = if wall > 0.0 {
            emulated / wall * 100.0
        } else {
            0.0
        };
        self.values.lag_ms = target.saturating_sub(ticks) as f64 / MASTER_HZ as f64 * 1000.0;
        self.values.ui_reads = ui.count.load(Ordering::Relaxed);
        self.values.ui_lock_ms = ui.lock_ns.load(Ordering::Relaxed) as f64 / 1_000_000.0;
        self.values.clone()
    }
}

pub fn milliseconds(start: Instant) -> f64 {
    start.elapsed().as_secs_f64() * 1000.0
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn coarse_windows_wakeup_is_drained_in_bounded_chunks_without_another_sleep() {
        let start = Instant::now();
        let pacer = Pacer::new(start, 0);
        let late = start + Duration::from_millis(16);
        let target = pacer.target(late);
        let mut ticks = 0;
        let mut chunks = 0;
        while ticks < target {
            assert_eq!(pacer.wait(late, ticks), Duration::ZERO);
            let next = Pacer::chunk(ticks, target);
            assert!(next - ticks <= WORK_TICKS);
            ticks = next;
            chunks += 1;
        }
        assert_eq!(chunks, 4);
        assert_eq!(ticks, MASTER_HZ * 16 / 1000);
        assert_eq!(pacer.wait(late, ticks), IDLE_WAIT);
    }
    #[test]
    fn real_time_debt_is_not_discarded_when_the_worker_is_late() {
        let start = Instant::now();
        let pacer = Pacer::new(start, 1234);
        assert_eq!(
            pacer.target(start + Duration::from_secs(60)),
            1234 + MASTER_HZ * 60
        );
        assert_eq!(
            pacer.wait(start + Duration::from_secs(60), 1234),
            Duration::ZERO
        );
    }
    #[test]
    fn speed_measurement_excludes_explicit_pause_but_not_halt_or_lag() {
        let start = Instant::now();
        let ui = UiReads::default();
        let mut t = Telemetry::new(0);
        t.resume(start);
        let p = t.sample(
            start + Duration::from_secs(2),
            MASTER_HZ,
            MASTER_HZ * 2,
            0,
            CycleCounters::default(),
            &ui,
        );
        assert_eq!(p.average_speed_percent, 50.0);
        assert_eq!(p.speed_percent, 50.0);
        assert_eq!(p.lag_ms, 1000.0);
        t.pause(start + Duration::from_secs(2));
        t.resume(start + Duration::from_secs(102));
        let p = t.sample(
            start + Duration::from_secs(104),
            MASTER_HZ * 3,
            MASTER_HZ * 3,
            0,
            CycleCounters::default(),
            &ui,
        );
        assert_eq!(p.wall_seconds, 4.0);
        assert_eq!(p.speed_percent, 100.0);
        assert_eq!(p.average_speed_percent, 75.0);
    }
}

pub const CSV_HEADER: &str = "wall_seconds,emulated_seconds,speed_percent,average_speed_percent,lag_ms,instructions,cycles,halt_cycles,irq_cycles,fast_cycles,slow_cycles,osc1_edges,audio_samples,worker_iterations,work_batches,catchup_batches,wait_calls,wait_ms,max_wait_ms,core_ms,audio_ms,snapshot_ms,snapshot_lock_ms,commands_ms,command_latency_max_ms,ui_reads,ui_lock_ms,profile_cpu_ms,profile_irq_ms,profile_timers_ms,profile_pcm_ms,running,halted";

pub fn csv_row(
    p: &Performance,
    c: CycleCounters,
    profile: crate::core::mcu::metrics::CoreProfile,
    instructions: u64,
    running: bool,
    halted: bool,
) -> String {
    let mut row = vec![
        format!("{:.6}", p.wall_seconds),
        format!("{:.6}", p.emulated_seconds),
        format!("{:.4}", p.speed_percent),
        format!("{:.4}", p.average_speed_percent),
        format!("{:.6}", p.lag_ms),
        instructions.to_string(),
        c.total().to_string(),
        c.halt.to_string(),
        c.interrupts.to_string(),
        c.fast.to_string(),
        c.slow.to_string(),
        c.osc1_edges.to_string(),
        c.audio_samples.to_string(),
        p.iterations.to_string(),
        p.work_batches.to_string(),
        p.catchup_batches.to_string(),
        p.wait_calls.to_string(),
    ];
    row.extend(
        [
            p.wait_ms,
            p.max_wait_ms,
            p.core_ms,
            p.audio_ms,
            p.snapshot_ms,
            p.snapshot_lock_ms,
            p.commands_ms,
            p.command_latency_max_ms,
        ]
        .map(|v| format!("{v:.6}")),
    );
    row.push(p.ui_reads.to_string());
    row.push(format!("{:.6}", p.ui_lock_ms));
    row.extend(
        [
            profile.cpu_ns,
            profile.irq_ns,
            profile.timers_ns,
            profile.pcm_ns,
        ]
        .map(|v| format!("{:.6}", v as f64 / 1_000_000.0)),
    );
    row.push(u8::from(running).to_string());
    row.push(u8::from(halted).to_string());
    row.join(",")
}

/// Optional one-Hz diagnostic logging for native foreground/background measurements.
pub struct CsvLog {
    writer: std::io::BufWriter<std::fs::File>,
    last_wall: f64,
}
impl CsvLog {
    pub fn from_environment() -> Result<Option<Self>, String> {
        use std::io::Write;
        let Some(path) = std::env::var_os("RETROHATCH_PERF_CSV") else {
            return Ok(None);
        };
        let mut writer = std::io::BufWriter::new(
            std::fs::File::create(&path)
                .map_err(|e| format!("Cannot create performance log: {e}"))?,
        );
        writeln!(writer, "{CSV_HEADER}").map_err(|e| e.to_string())?;
        Ok(Some(Self {
            writer,
            last_wall: f64::NEG_INFINITY,
        }))
    }
    pub fn reset_epoch(&mut self) {
        self.last_wall = f64::NEG_INFINITY;
    }
    pub fn write_due(
        &mut self,
        p: &Performance,
        c: CycleCounters,
        profile: crate::core::mcu::metrics::CoreProfile,
        instructions: u64,
        running: bool,
        halted: bool,
    ) -> Result<(), String> {
        use std::io::Write;
        if running && p.wall_seconds - self.last_wall < 1.0 {
            return Ok(());
        }
        writeln!(
            self.writer,
            "{}",
            csv_row(p, c, profile, instructions, running, halted)
        )
        .and_then(|_| self.writer.flush())
        .map_err(|e| format!("Performance log failed: {e}"))?;
        self.last_wall = p.wall_seconds;
        Ok(())
    }
}
