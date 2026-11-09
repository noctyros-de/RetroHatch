//! Deterministic E0C6200 MCU integration based on BrickEmuPy (CC0).
pub mod buzzer;
pub mod io;

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
}

impl Mcu {
    pub fn new(words: &[u16]) -> Result<Self, CpuError> {
        let mut cpu = Cpu::new(words)?;
        cpu.attach_io();
        Ok(Self {
            cpu,
            now: 0,
            next_osc1: 0,
            next_sample: 1,
            audio: Vec::new(),
            instructions: 0,
            interrupts: 0,
            reset_held: false,
        })
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
        self.reset_held = false;
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
            self.elapse(7 * OSC2_TICKS, false);
            self.next_osc1 = self.now;
            return Ok(ClockResult {
                instruction: None,
                interrupt: None,
                cycles: 7,
            });
        }
        let instruction = if self.cpu.state().halted {
            None
        } else {
            let step = self.cpu.step()?;
            self.instructions += 1;
            Some(step)
        };
        let mut cycles = instruction.map_or(7, |step| step.cycles);
        let interrupt = if self.cpu.state().interrupt_enable && !self.cpu.state().interrupt_delay {
            self.cpu.io().pending_vector()
        } else {
            None
        };
        if let Some(vector) = interrupt {
            self.cpu.enter_interrupt(vector);
            self.interrupts += 1;
            cycles += 13;
        }
        let cycle_ticks = if self.cpu.io().fast_clock() {
            OSC2_TICKS
        } else {
            OSC1_TICKS
        };
        self.elapse(u64::from(cycles) * cycle_ticks, true);
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
                self.cpu.with_io(|io| io.clock());
                self.next_osc1 += OSC1_TICKS;
            } else {
                let sample = if oscillator_running {
                    self.cpu.with_io(|io| io.buzzer.sample(AUDIO_RATE))
                } else {
                    0
                };
                self.audio.push(sample);
                self.next_sample += 1;
            }
        }
        self.now = end;
    }
}

#[cfg(test)]
mod tests;
