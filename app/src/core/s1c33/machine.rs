// SPDX-License-Identifier: GPL-3.0-only
// SPDX-FileCopyrightText: 2026 Noctyros (Rust translation and integration)
//! Deterministic CPU/peripheral owner, using tamaemu's 256-cycle synchronization boundary.
use super::{
    cpu::{Cpu, CpuError},
    memory::Memory,
    peripherals::Peripherals,
    profile::HardwareProfile,
};
#[derive(Clone)]
pub struct Machine {
    pub cpu: Cpu,
    pub memory: Memory,
    pub last_tick: u64,
    pub instructions: u64,
    pub interrupts: u64,
    /// Host presentation policy; deliberately excluded from machine save schemas.
    pub keep_screen_awake: bool,
}
impl Machine {
    pub fn new(image: &[u8], profile: HardwareProfile) -> Result<Self, String> {
        let image = super::rom::decode(image, profile)?;
        validate_image(&image, profile)?;
        let mut memory = Memory::new(profile, &image).map_err(|e| e.to_string())?;
        memory.peripherals = Some(Peripherals::new(profile));
        let mut cpu = Cpu::default();
        cpu.reset(&mut memory, profile.ttbr_reset)
            .map_err(|e| e.to_string())?;
        Ok(Self {
            cpu,
            memory,
            last_tick: 0,
            instructions: 0,
            interrupts: 0,
            keep_screen_awake: false,
        })
    }
    pub fn step(&mut self) -> Result<(), CpuError> {
        let p = self.memory.peripherals.as_mut().unwrap();
        p.audio_until(self.cpu.cycles);
        self.cpu.pending_irq = p.pending;
        self.cpu.pending_level = p.level;
        self.cpu.wake_request = p.wake;
        self.cpu.clock_option = p.opt;
        let scans = p.scan_serial;
        let outcome = self.cpu.step(&mut self.memory)?;
        // Optional tamaemu --stay-awake policy, not a fake button press or faster clock.
        if self.keep_screen_awake && self.memory.profile.has_sleep_flag {
            let offset = self
                .memory
                .profile
                .sleep_flag
                .wrapping_sub(self.memory.profile.a0ram_base) as usize;
            if let Some(flag) = self.memory.a0ram.get_mut(offset) {
                *flag = 0;
            }
        }
        self.instructions += outcome.instruction as u64;
        self.interrupts += outcome.interrupt.is_some() as u64;
        let p = self.memory.peripherals.as_mut().unwrap();
        if p.scan_serial == scans {
            p.pending = self.cpu.pending_irq;
        }
        p.audio_until(self.cpu.cycles);
        if self.cpu.cycles - self.last_tick >= 256 {
            p.tick(
                (self.cpu.cycles - self.last_tick) as u32,
                self.memory.profile,
            );
            self.last_tick = self.cpu.cycles;
        }
        Ok(())
    }
    pub fn seconds(&self) -> f64 {
        let p = self.memory.peripherals.as_ref().unwrap();
        p.seconds + (self.cpu.cycles - self.last_tick) as f64 / p.mclk
    }
    pub fn advance_to(&mut self, seconds: f64) -> Result<(), String> {
        while self.seconds() < seconds {
            self.step().map_err(|e| e.to_string())?;
        }
        Ok(())
    }
    pub fn buttons(&mut self, mask: u8) {
        self.memory.peripherals.as_mut().unwrap().buttons(mask);
    }
    pub fn rgb(&self) -> Vec<u8> {
        self.memory.lcd.rgb(self.keep_screen_awake)
    }
    pub fn drain_audio(&mut self) -> Vec<i16> {
        std::mem::take(&mut self.memory.peripherals.as_mut().unwrap().pcm)
    }
    pub fn reset(&mut self) -> Result<(), String> {
        let image = self.memory.flash().to_vec();
        let policy = self.keep_screen_awake;
        *self = Self::new(&image, self.memory.profile)?;
        self.keep_screen_awake = policy;
        Ok(())
    }
}
pub fn validate_image(image: &[u8], p: HardwareProfile) -> Result<(), String> {
    let image = super::rom::decode(image, p)?;
    if image.len() != p.rom_size as usize {
        return Err(format!(
            "{} requires an unpacked {}-byte NOR image",
            p.title, p.rom_size
        ));
    }
    let offset = (p.ttbr_reset - p.rom_base) as usize;
    let pc = u32::from_le_bytes(image[offset..offset + 4].try_into().unwrap());
    if pc & 1 != 0 || pc.wrapping_sub(p.rom_base) >= p.rom_size {
        return Err("Invalid S1C33 reset vector; select the matching hardware profile and an unpacked NOR dump".into());
    }
    Ok(())
}

/// Validate persisted source length without accepting arbitrary truncated firmware.
pub fn source_size_matches(p: HardwareProfile, hash: &str, size: usize) -> bool {
    super::rom::source_size_matches(p, hash, size)
}
