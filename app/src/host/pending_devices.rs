// SPDX-License-Identifier: GPL-3.0-only
// SPDX-FileCopyrightText: 2026 Noktyros
// Applies to original RetroHatch contributions; third-party provenance is retained below and in THIRD_PARTY_LICENSES.md.
//! Recognized research targets with insufficient evidence for executable device profiles.
//! Facts and blockers: docs/TAMAOTCH.md. No third-party implementation or artwork is included.

/// This is intentionally separate from `DeviceProfile`: incomplete hardware cannot be run.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PendingDevice {
    pub id: &'static str,
    pub name: &'static str,
    pub processor: &'static str,
    pub rom_bytes: usize,
    pub osc1_hz: u32,
    /// The fast oscillator must be measured or independently documented for this device.
    pub fast_clock_hz: Option<u32>,
    /// Exact local-image identity; the model attribution is research evidence, not hardware validation.
    pub sha256: &'static str,
    pub blockers: &'static str,
}

pub const TAMAOTCH: PendingDevice = PendingDevice {
    id: "tamagotchi-tamaotch",
    name: "Tamagotchi TamaOtch",
    processor: "Epson E0C6S48 / E0C6200",
    rom_bytes: 0x4000,
    osc1_hz: 32_768,
    fast_clock_hz: None,
    sha256: "b4eecf69ef28de62c4b4270c5773f229e1ba9591cfd6669f2d14e0adbd8e7481",
    blockers: "LCD wiring and original icon geometry, GPIO/sensor wiring and the exact fast clock are not verified",
};

/// Exact hashes only. A filename, a 16 KiB length or the weak research checksum is insufficient.
pub fn identify(hash: &str) -> Option<&'static PendingDevice> {
    (hash == TAMAOTCH.sha256).then_some(&TAMAOTCH)
}

pub fn ensure_supported(hash: &str) -> Result<(), String> {
    if let Some(device) = identify(hash) {
        return Err(format!(
            "{} research image recognized. Playable support is unavailable: {}. No substitute model was selected. Existing saves are unchanged.",
            device.name, device.blockers
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::profile::DeviceProfile;

    #[test]
    fn pending_tamaotch_is_not_an_executable_profile_or_a_guessed_configuration() {
        assert_eq!(identify(TAMAOTCH.sha256), Some(&TAMAOTCH));
        assert_eq!(TAMAOTCH.rom_bytes, 8_192 * 2);
        assert_eq!(TAMAOTCH.osc1_hz, 32_768);
        assert_eq!(TAMAOTCH.fast_clock_hz, None);
        assert_eq!(DeviceProfile::parse(TAMAOTCH.id), None);
        assert!(DeviceProfile::ALL.iter().all(|p| p.id() != TAMAOTCH.id));
        assert!(
            ensure_supported(TAMAOTCH.sha256)
                .unwrap_err()
                .contains("LCD wiring")
        );
        for other in ["TamagotchiTamaotch.bin", "5e1091e3", "", "b4eecf69"] {
            assert_eq!(identify(other), None);
            assert!(ensure_supported(other).is_ok());
        }
        for f in crate::host::fingerprints::CATALOG {
            assert_eq!(identify(f.sha256), None);
        }
    }
}
