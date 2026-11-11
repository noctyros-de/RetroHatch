// SPDX-License-Identifier: GPL-3.0-only
// SPDX-FileCopyrightText: 2026 Noctyros
//! Backend dispatch at the host boundary. Classic serialization and CPU execution stay unchanged.
use crate::core::s1c33::{machine::Machine as S1Machine, profile::HardwareProfile};
use crate::core::{
    cpu::e0c6200::CpuState,
    device::{Button, Device, DisplaySnapshot},
    mcu::{
        MASTER_HZ,
        metrics::{CoreProfile, CycleCounters},
    },
    p1::RomImage,
    profile::DeviceProfile,
    state::{Decoder, Encoder, MAGIC, hex, profile_code, require, sha256},
    tamagotchi::Tamagotchi,
};
use std::sync::Arc;
use tg18::snapshot::Snapshot as ArmSnapshot;
use tg18::sound::ToneSynth;
use tg18::{CPU_HZ, Key, Machine, Outcome};

pub fn validate_rom(bytes: &[u8], profile: DeviceProfile) -> Result<(), String> {
    if let Some(key) = profile.s1c33_key() {
        return crate::core::s1c33::machine::validate_image(
            bytes,
            HardwareProfile::find(key).unwrap(),
        );
    }
    if profile.is_color() {
        require(
            tg18::is_image(bytes),
            "On/Meets requires an exact 8-MiB SPII flash dump; packed, truncated and SOME images are not supported",
        )
    } else {
        RomImage::decode_for_profile(bytes, profile)
            .map(|_| ())
            .map_err(|e| e.to_string())
    }
}

pub type SaveCapture = Box<dyn FnOnce() -> Result<Vec<u8>, String> + Send>;

enum Backend {
    Classic(Box<Tamagotchi>),
    On(Box<OnDevice>),
    Ps(Box<S1Machine>),
}
pub struct Emulator {
    profile: DeviceProfile,
    hash: [u8; 32],
    metadata: (Option<u32>, bool),
    backend: Backend,
}
impl Emulator {
    pub fn from_bytes(bytes: &[u8], profile: DeviceProfile) -> Result<Self, String> {
        validate_rom(bytes, profile)?;
        let (backend, metadata) = if let Some(key) = profile.s1c33_key() {
            (
                Backend::Ps(Box::new(S1Machine::new(
                    bytes,
                    HardwareProfile::find(key).unwrap(),
                )?)),
                (None, false),
            )
        } else if profile.is_color() {
            (
                Backend::On(Box::new(OnDevice::new(Arc::new(bytes.to_vec())))),
                (None, false),
            )
        } else {
            let rom = RomImage::decode_for_profile(bytes, profile).map_err(|e| e.to_string())?;
            let metadata = (Some(rom.crc32), rom.known_world_p1());
            (
                Backend::Classic(Box::new(Tamagotchi::new(&rom).map_err(|e| e.to_string())?)),
                metadata,
            )
        };
        Ok(Self {
            profile,
            hash: sha256(bytes),
            metadata,
            backend,
        })
    }
    pub fn rom_hash(&self) -> [u8; 32] {
        self.hash
    }
    pub fn identity(&self) -> String {
        format!("{}-{}", self.profile.id(), hex(&self.hash))
    }
    pub fn classic_rom_metadata(&self) -> (Option<u32>, bool) {
        self.metadata
    }
    pub fn time(&self) -> u64 {
        match &self.backend {
            Backend::Classic(d) => d.time(),
            Backend::Ps(d) => (d.seconds() * MASTER_HZ as f64) as u64,
            Backend::On(d) => {
                (u128::from(d.machine.executed) * u128::from(MASTER_HZ) / CPU_HZ as u128) as u64
            }
        }
    }
    pub fn advance_to(&mut self, tick: u64) -> Result<(), String> {
        match &mut self.backend {
            Backend::Classic(d) => d.advance_to(tick).map_err(|e| e.to_string()),
            Backend::Ps(d) => d.advance_to(tick as f64 / MASTER_HZ as f64),
            Backend::On(d) => d.advance_to(
                (u128::from(tick) * CPU_HZ as u128)
                    .div_ceil(u128::from(MASTER_HZ))
                    .min(u128::from(u64::MAX)) as u64,
            ),
        }
    }
    pub fn classic_cpu_state(&self) -> Option<&CpuState> {
        match &self.backend {
            Backend::Classic(d) => Some(d.cpu_state()),
            Backend::On(_) | Backend::Ps(_) => None,
        }
    }
    pub fn arm_pc(&self) -> Option<u32> {
        match &self.backend {
            Backend::On(d) => Some(d.machine.cpu.r[15]),
            // Legacy snapshot field also carries the S1C33 full-width program counter.
            Backend::Ps(d) => Some(d.cpu.pc),
            _ => None,
        }
    }
    pub fn pc(&self) -> u32 {
        if let Backend::Ps(d) = &self.backend {
            d.cpu.pc
        } else {
            self.arm_pc()
                .unwrap_or_else(|| u32::from(self.classic_cpu_state().unwrap().pc))
        }
    }
    pub fn instructions(&self) -> u64 {
        match &self.backend {
            Backend::Classic(d) => d.mcu.instructions,
            Backend::On(d) => d.machine.executed,
            Backend::Ps(d) => d.instructions,
        }
    }
    pub fn interrupts(&self) -> u64 {
        match &self.backend {
            Backend::Classic(d) => d.mcu.interrupts,
            Backend::On(d) => d.machine.irqs,
            Backend::Ps(d) => d.interrupts,
        }
    }
    pub fn counters(&self) -> CycleCounters {
        match &self.backend {
            Backend::Classic(d) => d.mcu.counters,
            Backend::On(_) | Backend::Ps(_) => CycleCounters::default(),
        }
    }
    pub fn core_profile(&self) -> CoreProfile {
        match &self.backend {
            Backend::Classic(d) => d.mcu.profile,
            Backend::On(_) | Backend::Ps(_) => CoreProfile::default(),
        }
    }
    pub fn set_profiling(&mut self, value: bool) {
        if let Backend::Classic(d) = &mut self.backend {
            d.mcu.set_profiling(value);
        }
    }
    pub fn set_keep_screen_awake(&mut self, enabled: bool) {
        if let Backend::Ps(d) = &mut self.backend {
            d.keep_screen_awake = enabled && self.profile.supports_keep_screen_awake();
        }
    }
    pub fn sleeping(&self) -> bool {
        match &self.backend {
            Backend::On(d) => d.machine.sys.powered_off || d.machine.alarm_wake,
            Backend::Ps(d) => {
                !d.keep_screen_awake && (!d.memory.lcd.sleep_out || !d.memory.lcd.display_on)
            }
            _ => false,
        }
    }
    pub fn display(&self) -> DisplaySnapshot {
        match &self.backend {
            Backend::Classic(d) => d.display(),
            Backend::On(_) | Backend::Ps(_) => DisplaySnapshot::default(),
        }
    }
    pub fn color_display(&self) -> Option<Arc<Vec<u8>>> {
        match &self.backend {
            Backend::Ps(d) => Some(Arc::new(d.rgb())),
            Backend::On(d) => Some(Arc::new(if self.sleeping() {
                vec![0; 128 * 128 * 3]
            } else {
                d.machine.lcd_rgb()
            })),
            _ => None,
        }
    }
    pub fn set_button(&mut self, button: Button, pressed: bool) {
        match &mut self.backend {
            Backend::Classic(d) => d.set_button(button, pressed),
            Backend::On(d) => d.set_button(button, pressed),
            Backend::Ps(d) => {
                let bit = match button {
                    Button::A => 1,
                    Button::B => 2,
                    Button::C => 4,
                    Button::Knock => 0,
                };
                let mask = d.memory.peripherals.as_ref().unwrap().buttons;
                d.buttons(if pressed { mask | bit } else { mask & !bit });
            }
        }
    }
    pub fn release_inputs(&mut self) {
        match &mut self.backend {
            Backend::Classic(d) => d.release_inputs(),
            Backend::Ps(d) => d.buttons(0),
            Backend::On(d) => {
                d.machine.sys.live_keys = [false; 3];
                d.machine.sys.hold_until = [d.machine.now(); 3];
                d.machine.sys.key_script.clear();
            }
        }
    }
    pub fn reset(&mut self) {
        match &mut self.backend {
            Backend::Classic(d) => d.reset(),
            Backend::Ps(d) => {
                d.reset().expect("Previously validated S1C33 firmware");
            }
            Backend::On(d) => {
                d.machine.restart();
                d.reset_audio();
                d.rewind = None;
            }
        }
        self.release_inputs();
    }
    pub fn drain_audio(&mut self) -> Vec<i16> {
        match &mut self.backend {
            Backend::Classic(d) => d.drain_audio(),
            Backend::On(d) => std::mem::take(&mut d.pcm),
            Backend::Ps(d) => d.drain_audio(),
        }
    }
    pub fn save_state(&self) -> Result<Vec<u8>, String> {
        (self.capture_save()?)()
    }
    /// Capture coherently on the emulation owner; compress/encode on the ordered I/O worker.
    pub fn capture_save(&self) -> Result<SaveCapture, String> {
        if let Backend::Ps(d) = &self.backend {
            let snapshot = d.as_ref().clone();
            let hash = self.hash;
            return Ok(Box::new(move || snapshot.save_state(hash)));
        }
        let Backend::On(on) = &self.backend else {
            if let Backend::Classic(d) = &self.backend {
                let data = d.save_state()?;
                return Ok(Box::new(move || Ok(data)));
            }
            unreachable!()
        };
        let snapshot = ArmSnapshot::capture_delta(&on.machine);
        let audio_samples = on.audio_samples;
        let tone = [on.synth.phase(), on.synth.freq, on.synth.duty];
        let hash = self.hash;
        let profile = self.profile;
        Ok(Box::new(move || {
            let mut payload = Encoder(Vec::new());
            payload.u64(audio_samples);
            for value in tone {
                payload.u64(value.to_bits());
            }
            payload.bytes(&snapshot.to_bytes());
            require(
                payload.0.len() < 64 * 1024 * 1024 - 81,
                "TG18 state exceeds the supported size limit",
            )?;
            let mut out = Encoder(Vec::new());
            out.bytes(MAGIC);
            out.u16(1);
            out.u16(2);
            out.u8(profile_code(profile));
            out.bytes(&hash);
            out.u32(payload.0.len() as u32);
            out.bytes(&payload.0);
            let digest = sha256(&out.0);
            out.bytes(&digest);
            Ok(out.0)
        }))
    }
    pub fn restored(&self, data: &[u8]) -> Result<Self, String> {
        if let Backend::Ps(_) = &self.backend {
            let restored = S1Machine::restored(data, self.hash)?;
            require(
                Some(restored.memory.profile.key) == self.profile.s1c33_key(),
                "Save belongs to a different device profile",
            )?;
            return Ok(Self {
                profile: self.profile,
                hash: self.hash,
                metadata: self.metadata,
                backend: Backend::Ps(Box::new(restored)),
            });
        }
        if let Backend::Classic(d) = &self.backend {
            return Ok(Self {
                profile: self.profile,
                hash: self.hash,
                metadata: self.metadata,
                backend: Backend::Classic(Box::new(d.restored(data)?)),
            });
        }
        require(
            data.len() >= 121 && data.len() <= 64 * 1024 * 1024,
            "Invalid TG18 state size",
        )?;
        let body = &data[..data.len() - 32];
        require(
            sha256(body).as_slice() == &data[data.len() - 32..],
            "Save integrity check failed",
        )?;
        let mut r = Decoder::new(body);
        require(
            &r.array::<8>()? == MAGIC && r.u16()? == 1 && r.u16()? == 2,
            "Unsupported TG18 save format or backend schema",
        )?;
        require(
            r.u8()? == profile_code(self.profile) && r.array::<32>()? == self.hash,
            "Save belongs to a different ROM or device",
        )?;
        require(
            r.u32()? as usize == body.len() - r.at,
            "Invalid save payload length",
        )?;
        let audio_samples = r.u64()?;
        let phase = f64::from_bits(r.u64()?);
        let frequency = f64::from_bits(r.u64()?);
        let duty = f64::from_bits(r.u64()?);
        require(
            phase.is_finite()
                && (0.0..1.0).contains(&phase)
                && frequency.is_finite()
                && frequency >= 0.0
                && duty.is_finite()
                && (0.0..=1.0).contains(&duty),
            "Invalid saved audio state",
        )?;
        let snapshot = ArmSnapshot::from_bytes(&body[r.at..]).map_err(|e| e.to_string())?;
        let Backend::On(old) = &self.backend else {
            unreachable!()
        };
        let mut on = OnDevice::new(old.machine.image.clone());
        // Upstream decodes into this fresh machine; any error leaves the running device untouched.
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            snapshot.restore(&mut on.machine)
        }))
        .map_err(|_| "Malformed TG18 peripheral state".to_string())?
        .map_err(|e| e.to_string())?;
        require(
            audio_samples <= (on.machine.now() * 44100.0).floor() as u64 + 1,
            "Invalid saved audio cursor",
        )?;
        require(
            on.machine.sys.now.is_finite()
                && on.machine.sys.game_time.is_finite()
                && on.machine.sys.turbo == 1.0,
            "Invalid saved clock",
        )?;
        on.audio_samples = audio_samples;
        on.synth.restore_phase(phase);
        on.synth.freq = frequency;
        on.synth.duty = duty;
        Ok(Self {
            profile: self.profile,
            hash: self.hash,
            metadata: self.metadata,
            backend: Backend::On(Box::new(on)),
        })
    }
}

struct OnDevice {
    machine: Machine,
    synth: ToneSynth,
    audio_samples: u64,
    pcm: Vec<i16>,
    rewind: Option<ArmSnapshot>,
}
impl OnDevice {
    fn new(image: Arc<Vec<u8>>) -> Self {
        let mut machine = Machine::new(image.clone(), &image, tg18::rtc_seconds_now());
        machine.stop_on_bluetooth = true;
        Self {
            machine,
            synth: ToneSynth::new(44100, 1.0),
            audio_samples: 0,
            pcm: Vec::new(),
            rewind: None,
        }
    }
    fn reset_audio(&mut self) {
        self.audio_samples = (self.machine.now() * 44100.0).floor() as u64;
        self.synth = ToneSynth::new(44100, 1.0);
        self.pcm.clear();
    }
    fn set_button(&mut self, button: Button, pressed: bool) {
        let key = match button {
            Button::A => Key::A,
            Button::B => Key::B,
            Button::C => Key::C,
            Button::Knock => return,
        };
        let i = key.index();
        if pressed && !self.machine.sys.live_keys[i] {
            // Like TG18's frontend, keep a rollback point before entering unsupported wireless menus.
            self.rewind = Some(ArmSnapshot::capture(&self.machine));
            self.machine.alarm_wake = false;
            let now = self.machine.now();
            self.machine.sys.hold_until[i] = now + 0.05;
            if self.machine.sys.powered_off {
                self.machine.press(now, key);
            }
        }
        self.machine.sys.live_keys[i] = pressed;
    }
    fn advance_to(&mut self, limit: u64) -> Result<(), String> {
        while self.machine.executed < limit {
            if self.machine.sys.powered_off {
                if !self.machine.power_cycle(limit) {
                    let dt = (limit - self.machine.executed) as f64 / CPU_HZ;
                    self.machine.sys.game_time += dt;
                    self.machine.executed = limit;
                    self.machine.sys.now = limit as f64 / CPU_HZ;
                }
                continue;
            }
            match self.machine.run(limit) {
                Outcome::Limit | Outcome::PowerOff => {}
                Outcome::Bluetooth | Outcome::Infrared => {
                    if let Some(snapshot) = self.rewind.take() {
                        let image = self.machine.image.clone();
                        let mut fresh = Machine::new(image.clone(), &image, 0.0);
                        snapshot.restore(&mut fresh).map_err(|e| e.to_string())?;
                        fresh.stop_on_bluetooth = true;
                        self.machine = fresh;
                        self.reset_audio();
                    }
                    return Err("Bluetooth and infrared connections are not emulated. Returned to the previous input state; resume offline play.".into());
                }
                out => {
                    return Err(format!(
                        "TG18 stopped: {out:?}; CPU fault: {:?}",
                        self.machine.fault()
                    ));
                }
            }
        }
        if !self.machine.sys.powered_off {
            self.machine.sys.advance(self.machine.now());
        }
        let next = (self.machine.now() * 44100.0).floor() as u64;
        let n = next.saturating_sub(self.audio_samples) as usize;
        if n != 0 {
            let t0 = self.audio_samples as f64 / 44100.0;
            let t1 = next as f64 / 44100.0;
            let events = std::mem::take(&mut self.machine.sys.sound.events);
            let boundary = events.partition_point(|e| e.0 <= t1);
            self.synth
                .render(&events[..boundary], t0, t1, n, &mut self.pcm);
            self.machine
                .sys
                .sound
                .events
                .extend_from_slice(&events[boundary..]);
            self.audio_samples = next;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
