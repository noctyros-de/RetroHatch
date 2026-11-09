//! Tamagotchi P1 profile from BrickEmuPy's TamagotchiP1.brick (CC0).
use super::cpu::e0c6200::{CpuError, CpuState};
use super::device::{Button, Descriptor, Device, DisplaySnapshot};
use super::mcu::{Mcu, io::Port};
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
                "Invalid P1 ROM size: {size} bytes. Expected 12288 bytes (6144 big-endian 16-bit words). Packed 12-bit dumps are not supported."
            ),
            Self::Word { offset, value } => write!(
                f,
                "Invalid P1 opcode {value:#06x} at byte offset {offset:#06x}: the upper four bits must be zero. Use an unpacked big-endian dump."
            ),
        }
    }
}
impl std::error::Error for RomError {}

pub struct RomImage {
    pub words: Vec<u16>,
    pub crc32: u32,
}
impl RomImage {
    pub fn decode(bytes: &[u8]) -> Result<Self, RomError> {
        if bytes.len() != ROM_BYTES {
            return Err(RomError::Size(bytes.len()));
        }
        let mut words = Vec::with_capacity(ROM_BYTES / 2);
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
        Ok(Self { words, crc32: !crc })
    }
    /// Identity only: structurally valid unknown dumps still require manual P1 assignment.
    pub fn known_world_p1(&self) -> bool {
        self.crc32 == 0x5C864CB1
    }
}

pub struct P1 {
    pub mcu: Mcu,
}
impl P1 {
    pub fn new(rom: &RomImage) -> Result<Self, CpuError> {
        Ok(Self {
            mcu: Mcu::new(&rom.words)?,
        })
    }
}
impl Device for P1 {
    fn descriptor(&self) -> Descriptor {
        Descriptor {
            id: PROFILE_ID,
            name: "Tamagotchi P1",
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
        };
        self.mcu
            .pin(Port::K0, mask, if pressed { Some(false) } else { None });
    }
    fn release_inputs(&mut self) {
        for button in [Button::A, Button::B, Button::C] {
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
