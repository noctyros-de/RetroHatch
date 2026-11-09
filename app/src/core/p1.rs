// SPDX-License-Identifier: GPL-3.0-only
// SPDX-FileCopyrightText: 2026 Noktyros
// Applies to original RetroHatch contributions; third-party provenance is retained below and in THIRD_PARTY_LICENSES.md.
//! Tamagotchi P1 profile from BrickEmuPy's TamagotchiP1.brick (CC0).
use super::profile::DeviceProfile;
use std::fmt;

pub const ROM_BYTES: usize = 0x3000;
pub const PROFILE_ID: &str = "tamagotchi-p1";
pub const OSC2_HZ: u32 = 1_600_000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RomError {
    Size(usize),
    Word { offset: usize, value: u16 },
}
impl fmt::Display for RomError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Size(size) => write!(
                f,
                "Invalid ROM size: {size} bytes. Expected 12288 bytes for P1/P2 or 16384 bytes for Genjintch/Angel/Morino/Umino, as selected (big-endian 16-bit words). Packed 12-bit dumps are not supported."
            ),
            Self::Word { offset, value } => write!(
                f,
                "Invalid E0C6200 opcode {value:#06x} at byte offset {offset:#06x}: the upper four bits must be zero. Use an unpacked big-endian dump."
            ),
        }
    }
}
impl std::error::Error for RomError {}

pub struct RomImage {
    pub words: Vec<u16>,
    pub crc32: u32,
    pub profile: DeviceProfile,
}
impl RomImage {
    pub fn decode(bytes: &[u8]) -> Result<Self, RomError> {
        Self::decode_for_profile(bytes, DeviceProfile::P1)
    }
    pub fn decode_for_profile(bytes: &[u8], profile: DeviceProfile) -> Result<Self, RomError> {
        if bytes.len() != profile.rom_bytes() {
            return Err(RomError::Size(bytes.len()));
        }
        let mut words = Vec::with_capacity(profile.rom_bytes() / 2);
        for (index, pair) in bytes.chunks_exact(2).enumerate() {
            let word = u16::from_be_bytes([pair[0], pair[1]]);
            if word > 0xFFF {
                return Err(RomError::Word {
                    offset: index * 2,
                    value: word,
                });
            }
            words.push(word);
        }
        let mut crc = !0u32;
        for &byte in bytes {
            crc ^= u32::from(byte);
            for _ in 0..8 {
                crc = (crc >> 1) ^ (0xEDB88320 & 0u32.wrapping_sub(crc & 1));
            }
        }
        Ok(Self {
            words,
            crc32: !crc,
            profile,
        })
    }
    /// Identity only: structurally valid unknown dumps still require manual P1 assignment.
    pub fn known_world_p1(&self) -> bool {
        self.profile == DeviceProfile::P1 && self.crc32 == 0x5C864CB1
    }
}

// Preserve the established P1 constructor/import while sharing the MCU device.
pub use super::tamagotchi::Tamagotchi as P1;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::cpu::e0c6200::CpuState;
    use crate::core::device::{Button, Device, DisplaySnapshot};
    #[test]
    fn rom_validation_rejects_sizes_and_endianness_errors_without_executing_firmware() {
        for size in [0, 2, 8192, 9216, 16384] {
            assert!(
                matches!(RomImage::decode(&vec![0; size]), Err(RomError::Size(n)) if n == size)
            );
        }
        let mut bytes = vec![0; ROM_BYTES];
        bytes[512] = 0xFF;
        assert!(matches!(
            RomImage::decode(&bytes),
            Err(RomError::Word { offset: 512, .. })
        ));
        bytes[512] = 0x0F;
        let image = RomImage::decode(&bytes).unwrap();
        assert_eq!(image.words[256], 0xF00);
        assert!(!image.known_world_p1());
    }
    #[test]
    fn p1_buttons_are_independent_active_low_inputs_and_reset_releases_them() {
        let rom = RomImage::decode(&vec![0; ROM_BYTES]).unwrap();
        let mut device = P1::new(&rom).unwrap();
        assert_eq!(device.mcu.read(0xF40), Ok(15));
        device.set_button(Button::A, true);
        device.set_button(Button::C, true);
        assert_eq!(device.mcu.read(0xF40), Ok(10));
        device.set_button(Button::A, false);
        assert_eq!(device.mcu.read(0xF40), Ok(14));
        device.release_inputs();
        assert_eq!(device.mcu.read(0xF40), Ok(15));
        device.set_button(Button::B, true);
        device.reset();
        assert_eq!(device.mcu.read(0xF40), Ok(15));
        assert_eq!(device.cpu_state(), &CpuState::default());
    }
    #[test]
    fn lcd_off_all_on_and_normal_scan_preserve_the_underlying_lcd_memory() {
        let rom = RomImage::decode(&vec![0; ROM_BYTES]).unwrap();
        let mut device = P1::new(&rom).unwrap();
        device.mcu.write(0xE10, 5).unwrap();
        assert_eq!(device.display(), DisplaySnapshot::default());
        device.mcu.write(0xF71, 0).unwrap();
        assert_eq!(device.display().nibbles[16], 5);
        device.mcu.write(0xF71, 4).unwrap();
        assert_eq!(device.display().nibbles, [15; 160]);
        device.mcu.write(0xF71, 12).unwrap();
        assert_eq!(device.display(), DisplaySnapshot::default());
        assert_eq!(device.mcu.read(0xE10), Ok(5));
    }
}
