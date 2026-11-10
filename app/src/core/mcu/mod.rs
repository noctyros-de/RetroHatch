// SPDX-License-Identifier: GPL-3.0-only
// SPDX-FileCopyrightText: 2026 Noktyros
// Applies to original RetroHatch contributions; third-party provenance is retained below and in THIRD_PARTY_LICENSES.md.
//! Deterministic E0C6200 MCU integration based on BrickEmuPy (CC0).
pub mod buzzer;
pub mod io;
pub mod metrics;
use super::profile::DeviceProfile;
use metrics::{CoreProfile, CycleCounters};
use std::time::Instant;

use super::cpu::e0c6200::{Cpu, CpuError, Step};
use io::Port;

/// LCM of the P1 profile's 1.6 MHz OSC2 and 32768 Hz OSC1.
pub const MASTER_HZ: u64 = 102_400_000;
pub const OSC1_TICKS: u64 = 3125;
pub const OSC2_TICKS: u64 = 64;
pub const AUDIO_RATE: u32 = 44_100;

pub struct ClockResult {
    pub instruction: Option<Step>,
    pub interrupt: Option<u8>,
    pub cycles: u8,
}

pub struct Mcu {
    cpu: Cpu,
    now: u64,
    next_osc1: u64,
    next_sample: u64,
    audio: Vec<i16>,
    pub instructions: u64,
    pub interrupts: u64,
    reset_held: bool,
    pub counters: CycleCounters,
    pub profile: CoreProfile,
    profiling: bool,
    osc2_hz: u32,
    osc2_remainder: u64,
    // Derived presentation policy, not an MCU register or serialized hardware field.
    audio_headroom: bool,
}

impl Mcu {
    pub fn new(words: &[u16]) -> Result<Self, CpuError> {
        Self::for_profile(words, DeviceProfile::P1)
    }
    pub fn for_profile(words: &[u16], profile: DeviceProfile) -> Result<Self, CpuError> {
        let mut cpu = Cpu::new(words)?;
        cpu.attach_io();
        if matches!(
            profile,
            DeviceProfile::Genjintch
                | DeviceProfile::Angel
                | DeviceProfile::Morino
                | DeviceProfile::Umino
        ) {
            cpu.enable_lcd_gap_bus();
        }
        if profile == DeviceProfile::Umino {
            cpu.enable_reference_unmapped_bus();
        }
        Ok(Self {
            cpu,
            now: 0,
            next_osc1: 0,
            next_sample: 1,
            audio: Vec::new(),
            instructions: 0,
            interrupts: 0,
            reset_held: false,
            counters: CycleCounters::default(),
            profile: CoreProfile::default(),
            profiling: false,
            osc2_hz: profile.osc2_hz(),
            osc2_remainder: 0,
            audio_headroom: profile == DeviceProfile::Angel,
        })
    }
    pub fn set_profiling(&mut self, enabled: bool) {
        self.profiling = enabled;
        self.profile = CoreProfile::default();
    }
    pub fn cpu(&self) -> &Cpu {
        &self.cpu
    }
    pub fn time(&self) -> u64 {
        self.now
    }
    pub fn reset_held(&self) -> bool {
        self.reset_held
    }
    pub fn read(&self, address: u16) -> Result<u8, CpuError> {
        self.cpu.read_memory(address)
    }
    pub fn write(&mut self, address: u16, value: u8) -> Result<(), CpuError> {
        self.cpu.write_memory(address, value)
    }
    pub fn pin(&mut self, port: Port, mask: u8, level: Option<bool>) {
        self.cpu.with_io(|io| io.pin(port, mask, level));
    }
    pub fn reset(&mut self) {
        // A UI reset is a cold device reset: all external buttons are released.
        self.cpu.reset_mcu();
        self.next_osc1 = self.now;
        self.audio.clear();
        self.instructions = 0;
        self.interrupts = 0;
        self.counters = CycleCounters::default();
        self.profile = CoreProfile::default();
        self.reset_held = false;
        self.osc2_remainder = 0;
    }
    pub fn hold_reset(&mut self, held: bool) {
        if held && !self.reset_held {
            self.reset();
        }
        self.reset_held = held;
    }
    pub fn drain_audio(&mut self) -> Vec<i16> {
        std::mem::take(&mut self.audio)
    }

    pub fn clock(&mut self) -> Result<ClockResult, CpuError> {
        if self.reset_held {
            self.counters.reset += 7;
            self.counters.fast += 7;
            self.counters.clock_calls += 1;
            let ticks = self.fast_ticks(7);
            self.elapse(ticks, false);
            self.next_osc1 = self.now;
            return Ok(ClockResult {
                instruction: None,
                interrupt: None,
                cycles: 7,
            });
        }
        let cpu_start = self.profiling.then(Instant::now);
        let instruction = if self.cpu.state().halted {
            None
        } else {
            let step = self.cpu.step()?;
            self.instructions += 1;
            Some(step)
        };
        if let Some(start) = cpu_start {
            self.profile.cpu_ns += start.elapsed().as_nanos() as u64;
        }
        self.counters.clock_calls += 1;
        if let Some(step) = instruction {
            self.counters.instructions += u64::from(step.cycles);
        } else {
            self.counters.halt += 7;
        }
        let irq_start = self.profiling.then(Instant::now);
        let mut cycles = instruction.map_or(7, |step| step.cycles);
        let interrupt = if self.cpu.state().interrupt_enable && !self.cpu.state().interrupt_delay {
            self.cpu.io().pending_vector()
        } else {
            None
        };
        if let Some(vector) = interrupt {
            self.cpu.enter_interrupt(vector);
            self.interrupts += 1;
            self.counters.interrupts += 13;
            cycles += 13;
        }
        if let Some(start) = irq_start {
            self.profile.irq_ns += start.elapsed().as_nanos() as u64;
        }
        let ticks = if self.cpu.io().fast_clock() {
            self.counters.fast += u64::from(cycles);
            self.fast_ticks(u64::from(cycles))
        } else {
            self.counters.slow += u64::from(cycles);
            u64::from(cycles) * OSC1_TICKS
        };
        self.elapse(ticks, true);
        Ok(ClockResult {
            instruction,
            interrupt,
            cycles,
        })
    }

    pub fn advance_to(&mut self, target: u64) -> Result<(), CpuError> {
        while self.now < target {
            self.clock()?;
        }
        Ok(())
    }

    fn fast_ticks(&mut self, cycles: u64) -> u64 {
        if self.osc2_hz == 1_600_000 {
            return cycles * OSC2_TICKS; // Preserve the exact existing P1 path.
        }
        // 1.06 MHz does not divide MASTER_HZ. Carry the fractional master tick
        // across instructions and slow/fast switches instead of rounding each cycle.
        let numerator = cycles * MASTER_HZ + self.osc2_remainder;
        let ticks = numerator / u64::from(self.osc2_hz);
        self.osc2_remainder = numerator % u64::from(self.osc2_hz);
        ticks
    }

    fn elapse(&mut self, ticks: u64, oscillator_running: bool) {
        let end = self.now + ticks;
        loop {
            let sample_at = (u128::from(self.next_sample) * u128::from(MASTER_HZ))
                .div_ceil(u128::from(AUDIO_RATE)) as u64;
            let oscillator_at = if oscillator_running {
                self.next_osc1
            } else {
                u64::MAX
            };
            if oscillator_at.min(sample_at) > end {
                break;
            }
            if oscillator_at <= sample_at {
                let start = self.profiling.then(Instant::now);
                self.cpu.with_io(|io| io.clock());
                if let Some(start) = start {
                    self.profile.timers_ns += start.elapsed().as_nanos() as u64;
                }
                self.counters.osc1_edges += 1;
                self.next_osc1 += OSC1_TICKS;
            } else {
                let start = self.profiling.then(Instant::now);
                let sample = if oscillator_running {
                    let headroom = self.audio_headroom;
                    self.cpu.with_io(|io| {
                        if headroom {
                            io.buzzer.sample_with_headroom(AUDIO_RATE)
                        } else {
                            io.buzzer.sample(AUDIO_RATE)
                        }
                    })
                } else {
                    0
                };
                self.audio.push(sample);
                self.counters.audio_samples += 1;
                if let Some(start) = start {
                    self.profile.pcm_ns += start.elapsed().as_nanos() as u64;
                }
                self.next_sample += 1;
            }
        }
        self.now = end;
    }
}

#[cfg(test)]
mod tests;

impl Mcu {
    pub(crate) fn encode_state(
        &self,
        e: &mut crate::core::state::Encoder,
    ) -> crate::core::state::Result<()> {
        crate::core::state::require(self.audio.len() <= 441000, "Too much undrained PCM to save")?;
        self.cpu.encode_state(e);
        e.u64(self.now);
        e.u64(self.next_osc1);
        e.u64(self.next_sample);
        e.u64(self.instructions);
        e.u64(self.interrupts);
        e.bool(self.reset_held);
        e.u64(self.osc2_remainder);
        self.counters.encode_state(e);
        e.u32(self.audio.len() as u32);
        for &sample in &self.audio {
            e.u16(sample as u16);
        }
        Ok(())
    }
    pub(crate) fn decode_state(
        &mut self,
        d: &mut crate::core::state::Decoder<'_>,
    ) -> crate::core::state::Result<()> {
        use crate::core::state::require;
        self.cpu.decode_state(d)?;
        self.now = d.u64()?;
        self.next_osc1 = d.u64()?;
        self.next_sample = d.u64()?;
        self.instructions = d.u64()?;
        self.interrupts = d.u64()?;
        self.reset_held = d.bool()?;
        self.osc2_remainder = d.u64()?;
        self.counters.decode_state(d)?;
        require(
            [
                self.instructions,
                self.interrupts,
                self.counters.instructions,
                self.counters.halt,
                self.counters.interrupts,
                self.counters.reset,
                self.counters.fast,
                self.counters.slow,
                self.counters.osc1_edges,
                self.counters.audio_samples,
                self.counters.clock_calls,
            ]
            .iter()
            .all(|v| *v < u64::MAX / 64),
            "Invalid execution counters",
        )?;
        require(
            self.now < u64::MAX - MASTER_HZ * 60
                && self.next_osc1 >= self.now
                && self.next_osc1 - self.now <= OSC1_TICKS,
            "Invalid oscillator schedule",
        )?;
        let sample_at =
            (u128::from(self.next_sample) * u128::from(MASTER_HZ)).div_ceil(u128::from(AUDIO_RATE));
        require(
            sample_at > u128::from(self.now)
                && sample_at <= u128::from(self.now + MASTER_HZ / u64::from(AUDIO_RATE) + 1),
            "Invalid audio sample schedule",
        )?;
        require(
            self.osc2_remainder < u64::from(self.osc2_hz),
            "Invalid fractional oscillator phase",
        )?;
        let n = d.u32()? as usize;
        require(n <= 441000, "Invalid PCM buffer size")?;
        self.audio.clear();
        for _ in 0..n {
            let sample = d.u16()? as i16;
            require((-655..=655).contains(&sample), "Invalid MCU PCM sample")?;
            self.audio.push(sample);
        }
        Ok(())
    }
}
