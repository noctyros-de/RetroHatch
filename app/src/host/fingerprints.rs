// SPDX-License-Identifier: GPL-3.0-only
// SPDX-FileCopyrightText: 2026 Noktyros
// Applies to original RetroHatch contributions; third-party provenance is retained below and in THIRD_PARTY_LICENSES.md.
//! Exact, locally reference-validated ROM identities. No firmware or personal paths.
use crate::core::profile::DeviceProfile;
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Assignment {
    pub profile: DeviceProfile,
    pub variant: String,
    pub evidence: String,
}
pub struct Fingerprint<'a> {
    pub sha256: &'a str,
    pub profile: DeviceProfile,
    pub variant: &'a str,
}
pub const CATALOG: &[Fingerprint<'static>] = &[
    Fingerprint {
        sha256: "d64b56098cc09996dbb465b8cfb690d8f21b747325a1bfe56d9025b5dde2159b",
        profile: DeviceProfile::Ps,
        variant: "Region not verified",
    },
    Fingerprint {
        sha256: "7ee70ec703f390e17f6a3a4c817ba6e6f6ea97a718969aefd8641b7e2e1aa6c8",
        profile: DeviceProfile::P2,
        variant: "Region not verified",
    },
    Fingerprint {
        sha256: "efd0eb757a052e109ec65e3e695653b63bbcd76506e9cdf17e3dd6bab2eb609a",
        profile: DeviceProfile::P1,
        variant: "Japanese",
    },
    Fingerprint {
        sha256: "95d9dfe5cc3acbb84b555f3ff12e906caf838d19621106b9158af08b1b33471d",
        profile: DeviceProfile::Genjintch,
        variant: "Region not verified",
    },
    Fingerprint {
        sha256: "ca00d3bfcf854cd6fc5a7d7dbf991f72cd5419a91cdd010f88d354e1bdccd2f2",
        profile: DeviceProfile::Angel,
        variant: "Region not verified",
    },
    Fingerprint {
        sha256: "a956dcf287a58396f60332434184e10099e257f83610368bc8ab24d59a9d0d7e",
        profile: DeviceProfile::Morino,
        variant: "Region not verified",
    },
    Fingerprint {
        sha256: "9142d58924afe0e122b446c605157f5ba99826c546daf949ebc57438384f9342",
        profile: DeviceProfile::Umino,
        variant: "Region not verified",
    },
];
pub fn matches(hash: &str, catalog: &[Fingerprint<'_>]) -> Vec<Assignment> {
    catalog
        .iter()
        .filter(|f| f.sha256 == hash)
        .map(|f| Assignment {
            profile: f.profile,
            variant: f.variant.into(),
            evidence: if f.profile == DeviceProfile::Ps {
                "Verified SHA-256 (local tamaemu P's boot comparison)"
            } else {
                "Verified SHA-256 (local BrickEmuPy comparison)"
            }
            .into(),
        })
        .collect()
}

/// A model choice does not resolve indistinguishable regional labels.
pub fn for_profile(choices: &[Assignment], profile: DeviceProfile) -> Assignment {
    let matching: Vec<_> = choices.iter().filter(|a| a.profile == profile).collect();
    if matching.len() == 1 {
        return matching[0].clone();
    }
    Assignment {
        profile,
        variant: "Region not verified".into(),
        evidence: if matching.is_empty() {
            "User-confirmed model"
        } else {
            "User-selected model; regional identity remains ambiguous"
        }
        .into(),
    }
}
