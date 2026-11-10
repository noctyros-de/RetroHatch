// SPDX-License-Identifier: GPL-3.0-only
// SPDX-FileCopyrightText: 2026 Noctyros
// Applies to original RetroHatch contributions; third-party provenance is retained below and in THIRD_PARTY_LICENSES.md.
//! Shared E0C6200 Tamagotchi device; profile-specific wiring never enters the CPU.
use super::cpu::e0c6200::{CpuError, CpuState};
use super::device::{Button, Descriptor, Device, DisplaySnapshot};
use super::mcu::{Mcu, io::Port};
use super::p1::RomImage;
use super::profile::DeviceProfile;

pub struct Tamagotchi {
    pub profile: DeviceProfile,
    pub mcu: Mcu,
    pub(crate) rom_hash: [u8; 32],
}
impl Tamagotchi {
    pub fn new(rom: &RomImage) -> Result<Self, CpuError> {
        Ok(Self {
            mcu: Mcu::for_profile(&rom.words, rom.profile)?,
            profile: rom.profile,
            rom_hash: crate::core::state::fingerprint(&rom.words),
        })
    }
    pub(crate) fn new_from_program(
        words: &[u16],
        profile: DeviceProfile,
    ) -> crate::core::state::Result<Self> {
        Ok(Self {
            mcu: Mcu::for_profile(words, profile).map_err(|e| e.to_string())?,
            profile,
            rom_hash: crate::core::state::fingerprint(words),
        })
    }
}
impl Device for Tamagotchi {
    fn descriptor(&self) -> Descriptor {
        Descriptor {
            id: self.profile.id(),
            name: self.profile.name(),
        }
    }
    fn advance_to(&mut self, master_tick: u64) -> Result<(), CpuError> {
        self.mcu.advance_to(master_tick)
    }
    fn time(&self) -> u64 {
        self.mcu.time()
    }
    fn cpu_state(&self) -> &CpuState {
        self.mcu.cpu().state()
    }
    fn set_button(&mut self, button: Button, pressed: bool) {
        let mask = match button {
            Button::A => 4,
            Button::B => 2,
            Button::C => 1,
            Button::Knock if self.profile.has_knock() => 8,
            Button::Knock => return,
        };
        self.mcu
            .pin(Port::K0, mask, if pressed { Some(false) } else { None });
    }
    fn release_inputs(&mut self) {
        for button in [Button::A, Button::B, Button::C, Button::Knock] {
            self.set_button(button, false);
        }
    }
    fn reset(&mut self) {
        self.mcu.reset();
    }
    fn display(&self) -> DisplaySnapshot {
        let ctrl = self.mcu.cpu().io().lcd_control();
        let nibbles = if self.mcu.reset_held() || ctrl & 8 != 0 {
            [0; 160]
        } else if ctrl & 4 != 0 {
            // ALON means every segment. The upstream FULL_VRAM=[1;160] only sets bit 0.
            [15; 160]
        } else {
            *self.mcu.cpu().lcd_memory()
        };
        DisplaySnapshot { nibbles }
    }
    fn drain_audio(&mut self) -> Vec<i16> {
        self.mcu.drain_audio()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn device(profile: DeviceProfile) -> Tamagotchi {
        Tamagotchi::new(
            &RomImage::decode_for_profile(&vec![0; profile.rom_bytes()], profile).unwrap(),
        )
        .unwrap()
    }
    #[test]
    fn genjintch_buttons_knock_release_reset_and_p1_isolation_match_profile_wiring() {
        let mut genjintch = device(DeviceProfile::Genjintch);
        assert_eq!(genjintch.descriptor().id, "tamagotchi-genjintch");
        assert_eq!(genjintch.mcu.read(0xF42), Ok(15));
        for (button, mask) in [
            (Button::A, 4),
            (Button::B, 2),
            (Button::C, 1),
            (Button::Knock, 8),
        ] {
            genjintch.set_button(button, true);
            assert_eq!(genjintch.mcu.read(0xF40), Ok(15 ^ mask));
            genjintch.set_button(button, false);
            assert_eq!(genjintch.mcu.read(0xF40), Ok(15));
        }
        genjintch.set_button(Button::A, true);
        genjintch.set_button(Button::Knock, true);
        assert_eq!(genjintch.mcu.read(0xF40), Ok(3));
        genjintch.release_inputs();
        assert_eq!(genjintch.mcu.read(0xF40), Ok(15));
        genjintch.set_button(Button::Knock, true);
        genjintch.reset();
        assert_eq!(genjintch.mcu.read(0xF40), Ok(15));
        let mut p1 = device(DeviceProfile::P1);
        p1.set_button(Button::Knock, true);
        assert_eq!(p1.mcu.read(0xF40), Ok(15));
    }
    #[test]
    fn disconnected_bus_is_precise_and_other_invalid_accesses_remain_atomic() {
        let mut genjintch = device(DeviceProfile::Genjintch);
        for a in [0xE50, 0xE7F, 0xED0, 0xEFF, 0xF7C] {
            genjintch.mcu.write(a, 15).unwrap();
            assert_eq!(genjintch.mcu.read(a), Ok(0));
            let p1 = device(DeviceProfile::P1);
            assert_eq!(p1.mcu.read(a), Err(CpuError::UnsupportedAddress(a)));
        }
        for a in [0x300, 0xDFF, 0xF06, 0xF7F] {
            let cpu = genjintch.mcu.cpu().clone();
            let tick = genjintch.time();
            assert_eq!(
                genjintch.mcu.write(a, 15),
                Err(CpuError::UnsupportedAddress(a))
            );
            assert_eq!(genjintch.mcu.cpu(), &cpu);
            assert_eq!(genjintch.time(), tick);
        }
        genjintch.reset();
        genjintch.mcu.write(0xE50, 3).unwrap();
    }
    #[test]
    fn rom_profile_assignment_uses_format_and_explicit_selection_not_filenames() {
        let bytes = vec![0; 0x4000];
        assert!(RomImage::decode(&bytes).is_err());
        let rom = RomImage::decode_for_profile(&bytes, DeviceProfile::Genjintch).unwrap();
        assert_eq!(rom.words.len(), 8192);
        assert!(!rom.known_world_p1());
        assert!(RomImage::decode_for_profile(&vec![0; 0x3000], DeviceProfile::Genjintch).is_err());
    }
    #[test]
    fn angel_profile_rom_inputs_reset_and_display_share_real_mcu_behavior() {
        let mut angel = device(DeviceProfile::Angel);
        assert_eq!(angel.descriptor().id, "tamagotchi-angel");
        assert_eq!(angel.profile.osc2_hz(), 1_060_000);
        assert_eq!(angel.profile.rom_bytes(), 16384);
        assert_eq!(angel.mcu.read(0xF42), Ok(15));
        for (button, mask) in [
            (Button::A, 4),
            (Button::B, 2),
            (Button::C, 1),
            (Button::Knock, 8),
        ] {
            angel.set_button(button, true);
            assert_eq!(angel.mcu.read(0xF40), Ok(15 ^ mask));
            angel.set_button(button, false);
            assert_eq!(angel.mcu.read(0xF40), Ok(15));
        }
        angel.set_button(Button::A, true);
        angel.set_button(Button::Knock, true);
        assert_eq!(angel.mcu.read(0xF40), Ok(3));
        angel.release_inputs();
        assert_eq!(angel.mcu.read(0xF40), Ok(15));
        angel.set_button(Button::Knock, true);
        angel
            .advance_to(angel.time() + crate::core::mcu::MASTER_HZ / 10)
            .unwrap();
        assert_eq!(angel.mcu.read(0xF40), Ok(7)); // No invented automatic sensor pulse.
        angel.set_button(Button::Knock, false);
        angel
            .advance_to(angel.time() + crate::core::mcu::MASTER_HZ / 10)
            .unwrap();
        assert_eq!(angel.mcu.read(0xF40), Ok(15));
        angel.mcu.write(0xE10, 5).unwrap();
        angel.mcu.write(0xF71, 0).unwrap();
        assert_eq!(angel.display().nibbles[16], 5);
        angel.set_button(Button::Knock, true);
        angel.reset();
        assert_eq!(angel.mcu.read(0xF40), Ok(15));
        assert_eq!(angel.display(), DisplaySnapshot::default());
        assert!(RomImage::decode_for_profile(&vec![0; 12288], DeviceProfile::Angel).is_err());
        let mut malformed = vec![0; 16384];
        malformed[0] = 0x10;
        assert!(RomImage::decode_for_profile(&malformed, DeviceProfile::Angel).is_err());
    }
    #[test]
    fn morino_profile_rom_inputs_reset_and_display_share_real_mcu_behavior() {
        let mut morino = device(DeviceProfile::Morino);
        assert_eq!(morino.descriptor().id, "tamagotchi-morino");
        assert_eq!(morino.profile.osc2_hz(), 1_060_000);
        assert_eq!(morino.profile.rom_bytes(), 16384);
        assert_eq!(morino.mcu.read(0xF42), Ok(15));
        for (button, mask) in [
            (Button::A, 4),
            (Button::B, 2),
            (Button::C, 1),
            (Button::Knock, 8),
        ] {
            morino.set_button(button, true);
            assert_eq!(morino.mcu.read(0xF40), Ok(15 ^ mask));
            morino.set_button(button, false);
            assert_eq!(morino.mcu.read(0xF40), Ok(15));
        }
        morino.set_button(Button::A, true);
        morino.set_button(Button::Knock, true);
        assert_eq!(morino.mcu.read(0xF40), Ok(3));
        morino.release_inputs();
        assert_eq!(morino.mcu.read(0xF40), Ok(15));
        morino.set_button(Button::Knock, true);
        morino
            .advance_to(morino.time() + crate::core::mcu::MASTER_HZ / 10)
            .unwrap();
        assert_eq!(morino.mcu.read(0xF40), Ok(7)); // No invented automatic sensor pulse.
        morino.set_button(Button::Knock, false);
        morino
            .advance_to(morino.time() + crate::core::mcu::MASTER_HZ / 10)
            .unwrap();
        assert_eq!(morino.mcu.read(0xF40), Ok(15));
        morino.mcu.write(0xE10, 5).unwrap();
        morino.mcu.write(0xF71, 0).unwrap();
        assert_eq!(morino.display().nibbles[16], 5);
        morino.set_button(Button::Knock, true);
        morino.reset();
        assert_eq!(morino.mcu.read(0xF40), Ok(15));
        assert_eq!(morino.display(), DisplaySnapshot::default());
        assert!(RomImage::decode_for_profile(&vec![0; 12288], DeviceProfile::Morino).is_err());
        let mut malformed = vec![0; 16384];
        malformed[0] = 0x10;
        assert!(RomImage::decode_for_profile(&malformed, DeviceProfile::Morino).is_err());
    }
    #[test]
    fn umino_profile_rom_inputs_reset_and_display_share_real_mcu_behavior() {
        assert_eq!(DeviceProfile::parse("ocean"), Some(DeviceProfile::Umino));
        assert_eq!(
            DeviceProfile::parse("tamagotchi-umino"),
            Some(DeviceProfile::Umino)
        );
        let mut umino = device(DeviceProfile::Umino);
        assert_eq!(umino.descriptor().id, "tamagotchi-umino");
        assert_eq!(umino.profile.osc2_hz(), 1_060_000);
        assert_eq!(umino.profile.rom_bytes(), 16384);
        assert_eq!(umino.mcu.read(0xF42), Ok(15));
        for (button, mask) in [
            (Button::A, 4),
            (Button::B, 2),
            (Button::C, 1),
            (Button::Knock, 8),
        ] {
            umino.set_button(button, true);
            assert_eq!(umino.mcu.read(0xF40), Ok(15 ^ mask));
            umino.set_button(button, false);
            assert_eq!(umino.mcu.read(0xF40), Ok(15));
        }
        umino.set_button(Button::A, true);
        umino.set_button(Button::Knock, true);
        assert_eq!(umino.mcu.read(0xF40), Ok(3));
        umino.release_inputs();
        assert_eq!(umino.mcu.read(0xF40), Ok(15));
        umino.set_button(Button::Knock, true);
        umino
            .advance_to(umino.time() + crate::core::mcu::MASTER_HZ / 10)
            .unwrap();
        assert_eq!(umino.mcu.read(0xF40), Ok(7)); // No invented automatic sensor pulse.
        umino.set_button(Button::Knock, false);
        umino
            .advance_to(umino.time() + crate::core::mcu::MASTER_HZ / 10)
            .unwrap();
        assert_eq!(umino.mcu.read(0xF40), Ok(15));
        umino.mcu.write(0xE10, 5).unwrap();
        umino.mcu.write(0xF71, 0).unwrap();
        assert_eq!(umino.display().nibbles[16], 5);
        umino.set_button(Button::Knock, true);
        umino.reset();
        assert_eq!(umino.mcu.read(0xF40), Ok(15));
        assert_eq!(umino.display(), DisplaySnapshot::default());
        assert!(RomImage::decode_for_profile(&vec![0; 12288], DeviceProfile::Umino).is_err());
        let mut malformed = vec![0; 16384];
        malformed[0] = 0x10;
        assert!(RomImage::decode_for_profile(&malformed, DeviceProfile::Umino).is_err());
    }
    #[test]
    fn p2_uses_p1_hardware_with_distinct_identity_and_no_sensor() {
        assert_eq!(DeviceProfile::parse("p2"), Some(DeviceProfile::P2));
        assert_eq!(DeviceProfile::P2.rom_bytes(), 12_288);
        assert_eq!(DeviceProfile::P2.osc2_hz(), 1_600_000);
        assert!(!DeviceProfile::P2.has_knock());
        let mut p2 = device(DeviceProfile::P2);
        assert_eq!(p2.descriptor().id, "tamagotchi-p2");
        for (button, mask) in [(Button::A, 4), (Button::B, 2), (Button::C, 1)] {
            p2.set_button(button, true);
            assert_eq!(p2.mcu.read(0xF40), Ok(15 ^ mask));
            p2.set_button(Button::Knock, true);
            assert_eq!(p2.mcu.read(0xF40), Ok(15 ^ mask));
            p2.release_inputs();
            assert_eq!(p2.mcu.read(0xF40), Ok(15));
        }
        p2.mcu.write(0xF71, 0).unwrap();
        p2.mcu.write(0xE10, 5).unwrap();
        assert_eq!(p2.display().nibbles[16], 5);
        assert_eq!(p2.mcu.read(0xE50), Err(CpuError::UnsupportedAddress(0xE50)));
        p2.set_button(Button::B, true);
        p2.reset();
        assert_eq!(p2.cpu_state(), &CpuState::default());
        assert_eq!(p2.mcu.read(0xF40), Ok(15));
        assert!(RomImage::decode_for_profile(&vec![0; 16_384], DeviceProfile::P2).is_err());
        let mut bad = vec![0; DeviceProfile::P2.rom_bytes()];
        bad[0x200] = 16;
        assert!(RomImage::decode_for_profile(&bad, DeviceProfile::P2).is_err());
    }
}
