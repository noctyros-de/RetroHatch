// SPDX-License-Identifier: GPL-3.0-only
// SPDX-FileCopyrightText: 2026 Noctyros
// Applies to original RetroHatch contributions; third-party provenance is retained below and in THIRD_PARTY_LICENSES.md.
//! Versioned full-machine states. Explicit little-endian fields; never raw Rust memory.
use super::{profile::DeviceProfile, tamagotchi::Tamagotchi};
pub type Result<T> = std::result::Result<T, String>;
pub const MAX_FILE: usize = 2 * 1024 * 1024;
pub const MAGIC: &[u8; 8] = b"RTHSTATE";
pub struct Encoder(pub Vec<u8>);
impl Encoder {
    pub fn bytes(&mut self, v: &[u8]) {
        self.0.extend_from_slice(v);
    }
    pub fn u8(&mut self, v: u8) {
        self.0.push(v);
    }
    pub fn bool(&mut self, v: bool) {
        self.u8(u8::from(v));
    }
    pub fn u16(&mut self, v: u16) {
        self.bytes(&v.to_le_bytes());
    }
    pub fn u32(&mut self, v: u32) {
        self.bytes(&v.to_le_bytes());
    }
    pub fn i32(&mut self, v: i32) {
        self.bytes(&v.to_le_bytes());
    }
    pub fn u64(&mut self, v: u64) {
        self.bytes(&v.to_le_bytes());
    }
}
pub struct Decoder<'a> {
    pub data: &'a [u8],
    pub at: usize,
}
impl<'a> Decoder<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self { data, at: 0 }
    }
    pub fn array<const N: usize>(&mut self) -> Result<[u8; N]> {
        let end = self.at.checked_add(N).ok_or("Invalid state length")?;
        let slice = self.data.get(self.at..end).ok_or("Truncated state")?;
        self.at = end;
        Ok(slice.try_into().unwrap())
    }
    pub fn u8(&mut self) -> Result<u8> {
        Ok(self.array::<1>()?[0])
    }
    pub fn bool(&mut self) -> Result<bool> {
        match self.u8()? {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err("Invalid boolean in state".into()),
        }
    }
    pub fn u16(&mut self) -> Result<u16> {
        Ok(u16::from_le_bytes(self.array()?))
    }
    pub fn u32(&mut self) -> Result<u32> {
        Ok(u32::from_le_bytes(self.array()?))
    }
    pub fn i32(&mut self) -> Result<i32> {
        Ok(i32::from_le_bytes(self.array()?))
    }
    pub fn u64(&mut self) -> Result<u64> {
        Ok(u64::from_le_bytes(self.array()?))
    }
    pub fn finish(&self) -> Result<()> {
        require(
            self.at == self.data.len(),
            "Unexpected trailing state fields",
        )
    }
}
pub fn require(valid: bool, message: &str) -> Result<()> {
    if valid { Ok(()) } else { Err(message.into()) }
}
pub fn profile_code(p: DeviceProfile) -> u8 {
    match p {
        DeviceProfile::P1 => 0,
        DeviceProfile::Genjintch => 1,
        DeviceProfile::Angel => 2,
        DeviceProfile::Morino => 3,
        DeviceProfile::Umino => 4,
        DeviceProfile::P2 => 5,
        DeviceProfile::On => 6,
        DeviceProfile::Ps => 7,
        DeviceProfile::Id => 8,
        DeviceProfile::IdMelody => 9,
        DeviceProfile::Idl => 10,
        DeviceProfile::IdlEnglish => 11,
        DeviceProfile::IdlPrincess => 12,
        DeviceProfile::Idl15 => 13,
        DeviceProfile::PlusColor => 14,
        DeviceProfile::PlusColorHexa => 15,
    }
}
pub fn fingerprint(words: &[u16]) -> [u8; 32] {
    let bytes: Vec<u8> = words.iter().flat_map(|v| v.to_be_bytes()).collect();
    sha256(&bytes)
}
pub fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// SHA-256 (FIPS 180-4), for content identity and corruption detection, not authentication.
pub fn sha256(data: &[u8]) -> [u8; 32] {
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];
    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];
    let mut padded = data.to_vec();
    padded.push(0x80);
    while padded.len() % 64 != 56 {
        padded.push(0);
    }
    padded.extend_from_slice(&(data.len() as u64 * 8).to_be_bytes());
    for block in padded.chunks_exact(64) {
        let mut w = [0u32; 64];
        for (i, c) in block.chunks_exact(4).enumerate() {
            w[i] = u32::from_be_bytes(c.try_into().unwrap());
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }
        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut hh] = h;
        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ (!e & g);
            let t1 = hh
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[i])
                .wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let t2 = s0.wrapping_add(maj);
            hh = g;
            g = f;
            f = e;
            e = d.wrapping_add(t1);
            d = c;
            c = b;
            b = a;
            a = t1.wrapping_add(t2);
        }
        for (x, v) in h.iter_mut().zip([a, b, c, d, e, f, g, hh]) {
            *x = x.wrapping_add(v);
        }
    }
    let mut out = [0; 32];
    for (i, v) in h.iter().enumerate() {
        out[i * 4..i * 4 + 4].copy_from_slice(&v.to_be_bytes());
    }
    out
}
impl Tamagotchi {
    pub fn identity(&self) -> String {
        format!("{}-{}", self.profile.id(), hex(&self.rom_hash))
    }
    pub fn save_state(&self) -> Result<Vec<u8>> {
        let mut payload = Encoder(Vec::new());
        self.mcu.encode_state(&mut payload)?;
        require(
            payload.0.len() < MAX_FILE - 81,
            "State exceeds the supported size limit",
        )?;
        let mut e = Encoder(Vec::new());
        e.bytes(MAGIC);
        e.u16(1);
        e.u16(1);
        e.u8(profile_code(self.profile));
        e.bytes(&self.rom_hash);
        e.u32(payload.0.len() as u32);
        e.bytes(&payload.0);
        let digest = sha256(&e.0);
        e.bytes(&digest);
        Ok(e.0)
    }
    /// Decode into a prepared device. Failure leaves the current machine untouched.
    pub fn restored(&self, bytes: &[u8]) -> Result<Self> {
        require(
            bytes.len() >= 81 && bytes.len() <= MAX_FILE,
            "Invalid save size",
        )?;
        let body = &bytes[..bytes.len() - 32];
        require(
            sha256(body).as_slice() == &bytes[bytes.len() - 32..],
            "Save integrity check failed",
        )?;
        let mut d = Decoder::new(body);
        require(
            &d.array::<8>()? == MAGIC,
            "This is not a RetroHatch save file",
        )?;
        require(d.u16()? == 1, "Unsupported save format version")?;
        require(d.u16()? == 1, "Unsupported MCU schema version")?;
        require(
            d.u8()? == profile_code(self.profile),
            "Save belongs to a different device profile",
        )?;
        require(
            d.array::<32>()? == self.rom_hash,
            "Save belongs to a different ROM (SHA-256 mismatch)",
        )?;
        let length = d.u32()? as usize;
        require(length == body.len() - d.at, "Invalid save payload length")?;
        let mut prepared = Self::new_from_program(self.mcu.cpu().program_words(), self.profile)?;
        prepared.mcu.decode_state(&mut d)?;
        d.finish()?;
        Ok(prepared)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sha256_matches_published_vectors_and_multi_block_input() {
        assert_eq!(
            hex(&sha256(b"")),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            hex(&sha256(b"abc")),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_eq!(
            hex(&sha256(&vec![b'a'; 1000000])),
            "cdc76e5c9914fb9281a1c7e284d73e67f1809a48a497200e046d39ccc7112cd0"
        );
    }
    fn synthetic(profile: DeviceProfile) -> Tamagotchi {
        use crate::core::p1::RomImage;
        let line = include_str!("../../tests/data/e0c6200_mcu_programs.txt")
            .lines()
            .find(|l| l.starts_with("timers_audio|"))
            .unwrap();
        let mut words = vec![0xFFB; profile.rom_bytes() / 2];
        for item in line.split('|').nth(2).unwrap().split(',') {
            let (a, v) = item.split_once(':').unwrap();
            words[usize::from_str_radix(a, 16).unwrap()] = u16::from_str_radix(v, 16).unwrap();
        }
        let bytes: Vec<_> = words.iter().flat_map(|v| v.to_be_bytes()).collect();
        Tamagotchi::new(&RomImage::decode_for_profile(&bytes, profile).unwrap()).unwrap()
    }
    #[test]
    fn complete_states_preserve_every_field_and_future_cpu_timers_irq_gpio_and_pcm() {
        use crate::core::{
            device::{Button, Device},
            mcu::io::Port,
        };
        for profile in DeviceProfile::CLASSIC {
            let mut original = synthetic(profile);
            for _ in 0..501 {
                original.mcu.clock().unwrap();
            }
            original.mcu.write(0xF70, 8).unwrap();
            original.mcu.clock().unwrap();
            original.set_button(Button::A, true);
            let saved = original.save_state().unwrap();
            let mut restored = original.restored(&saved).unwrap();
            assert_eq!(saved, restored.save_state().unwrap());
            assert_eq!(original.drain_audio(), restored.drain_audio());
            for i in 0..1000 {
                for d in [&mut original, &mut restored] {
                    if i == 57 {
                        d.mcu.pin(Port::K1, 1, Some(false));
                    }
                    if i == 91 {
                        d.release_inputs();
                    }
                    if i == 400 {
                        d.mcu.write(0xF70, 0).unwrap();
                    }
                    d.mcu.clock().unwrap();
                }
                if i % 31 == 0 {
                    assert_eq!(
                        original.save_state().unwrap(),
                        restored.save_state().unwrap(),
                        "{profile:?} at {i}"
                    );
                    assert_eq!(original.drain_audio(), restored.drain_audio());
                }
            }
            assert_eq!(original.mcu.interrupts, restored.mcu.interrupts);
            assert!(original.mcu.interrupts > 0);
        }
    }
    #[test]
    fn corrupted_future_wrong_rom_and_wrong_profile_states_never_change_current_machine() {
        let d = synthetic(DeviceProfile::Angel);
        let saved = d.save_state().unwrap();
        let before = d.save_state().unwrap();
        for i in 0..saved.len() {
            let mut bad = saved.clone();
            bad[i] ^= 1;
            assert!(d.restored(&bad).is_err());
        }
        for i in [8, 10, 49] {
            let mut bad = saved.clone();
            bad[i] = 255;
            let n = bad.len() - 32;
            let digest = sha256(&bad[..n]);
            bad[n..].copy_from_slice(&digest);
            assert!(d.restored(&bad).is_err());
        }
        assert!(
            synthetic(DeviceProfile::Genjintch)
                .restored(&saved)
                .is_err()
        );
        let mut other = synthetic(DeviceProfile::Angel);
        other.rom_hash[0] ^= 1;
        assert!(other.restored(&saved).is_err());
        for n in [0, 1, 80, saved.len() - 1] {
            assert!(d.restored(&saved[..n]).is_err());
        }
        assert_eq!(d.save_state().unwrap(), before);
    }
    #[test]
    fn reset_hold_and_initial_clock_boundaries_round_trip() {
        for profile in DeviceProfile::CLASSIC {
            let mut d = synthetic(profile);
            let initial = d.save_state().unwrap();
            assert_eq!(d.restored(&initial).unwrap().save_state().unwrap(), initial);
            d.mcu.hold_reset(true);
            d.mcu.clock().unwrap();
            let saved = d.save_state().unwrap();
            let mut loaded = d.restored(&saved).unwrap();
            d.mcu.hold_reset(false);
            loaded.mcu.hold_reset(false);
            for _ in 0..100 {
                d.mcu.clock().unwrap();
                loaded.mcu.clock().unwrap();
            }
            assert_eq!(d.save_state().unwrap(), loaded.save_state().unwrap());
        }
    }
    #[test]
    fn a_checksummed_state_cannot_inject_pcm_outside_the_mcu_output_range() {
        let d = synthetic(DeviceProfile::Angel);
        let good = d.save_state().unwrap();
        let mut body = good[..good.len() - 32].to_vec();
        let end = body.len();
        body[end - 4..].copy_from_slice(&1u32.to_le_bytes());
        body.extend_from_slice(&10000i16.to_le_bytes());
        let length = (body.len() - 49) as u32;
        body[45..49].copy_from_slice(&length.to_le_bytes());
        let hash = sha256(&body);
        body.extend_from_slice(&hash);
        assert!(d.restored(&body).err().unwrap().contains("PCM"));
        assert_eq!(d.save_state().unwrap(), good);
    }
    #[test]
    fn umino_appends_a_profile_code_without_changing_existing_save_identities() {
        for (p, code) in [
            (DeviceProfile::P1, 0),
            (DeviceProfile::Genjintch, 1),
            (DeviceProfile::Angel, 2),
            (DeviceProfile::Morino, 3),
            (DeviceProfile::Umino, 4),
        ] {
            assert_eq!(profile_code(p), code);
            assert_eq!(synthetic(p).save_state().unwrap()[12], code);
        }
        let umino = synthetic(DeviceProfile::Umino);
        let saved = umino.save_state().unwrap();
        for profile in [
            DeviceProfile::Genjintch,
            DeviceProfile::Angel,
            DeviceProfile::Morino,
        ] {
            let other = synthetic(profile);
            let before = other.save_state().unwrap();
            assert_eq!(
                other.rom_hash, umino.rom_hash,
                "Same program must exercise profile identity independently of ROM identity"
            );
            assert!(other.restored(&saved).is_err());
            assert_eq!(other.save_state().unwrap(), before);
        }
    }
    #[test]
    fn p2_appends_code_five_and_cannot_load_identical_program_p1_states() {
        assert_eq!(profile_code(DeviceProfile::P2), 5);
        for (p, code) in [
            (DeviceProfile::P1, 0),
            (DeviceProfile::Genjintch, 1),
            (DeviceProfile::Angel, 2),
            (DeviceProfile::Morino, 3),
            (DeviceProfile::Umino, 4),
        ] {
            assert_eq!(profile_code(p), code);
        }
        let p1 = synthetic(DeviceProfile::P1);
        let p2 = synthetic(DeviceProfile::P2);
        assert_eq!(p1.rom_hash, p2.rom_hash);
        assert_ne!(p1.identity(), p2.identity());
        let a = p1.save_state().unwrap();
        let b = p2.save_state().unwrap();
        assert_eq!(b[12], 5);
        assert!(p1.restored(&b).is_err());
        assert!(p2.restored(&a).is_err());
        assert_eq!(p1.save_state().unwrap(), a);
        assert_eq!(p2.save_state().unwrap(), b);
    }
}
