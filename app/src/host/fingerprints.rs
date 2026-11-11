// SPDX-License-Identifier: GPL-3.0-only
// SPDX-FileCopyrightText: 2026 Noctyros
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
        sha256: "ed97fa667fe7389cbd4fb10e6eaec8927cdab2ddd88ee8fb4e3c1aa145b57128",
        profile: DeviceProfile::PlusColor,
        variant: "Japanese",
    },
    Fingerprint {
        sha256: "6e209efc22e23f1f41ee54e7874648b24d255c9a2869d9bd6f22efa252b89e61",
        profile: DeviceProfile::PlusColor,
        variant: "Japanese",
    },
    Fingerprint {
        sha256: "4a56271ae1e13e291b34306fb42d84904f89630266ff40ed8e05781d3f753200",
        profile: DeviceProfile::PlusColor,
        variant: "Japanese",
    },
    Fingerprint {
        sha256: "5f6ea377bc66ac89ae20f76a1f06fea383fa122a33e4b00a9df20867a79b3bb2",
        profile: DeviceProfile::PlusColorHexa,
        variant: "Japanese",
    },
    Fingerprint {
        sha256: "1ffc2595d36e951ef9ff5e00fad75c12a76004d2bd41eb6f692123afebc86bb7",
        profile: DeviceProfile::IdMelody,
        variant: "Japanese",
    },
    Fingerprint {
        sha256: "3395f83033596c03a9844ce98fcde6f1e3b4cf7a3fe9e2acb12410bc09a7fa05",
        profile: DeviceProfile::Idl,
        variant: "Japanese",
    },
    Fingerprint {
        sha256: "40639648dafd133ed350635819d1bb724ddfde4bc076c81580180a42c17e4caf",
        profile: DeviceProfile::IdlEnglish,
        variant: "English",
    },
    Fingerprint {
        sha256: "ffdbac6bf19ef303814c38c4cb6235052ab60b53952de3b4031d693f30589fed",
        profile: DeviceProfile::IdlPrincess,
        variant: "Japanese",
    },
    Fingerprint {
        sha256: "1ab67c17de301185f8c0b76f62e2b4d60d0b678a61b88edb5dcfe4cd7ed520b1",
        profile: DeviceProfile::Idl15,
        variant: "Japanese",
    },
    Fingerprint {
        sha256: "59c067987804a6c2a200e0a98b95b332c452c1b8b11daa4fd80cd2cbaff319ff",
        profile: DeviceProfile::Idl,
        variant: "Japanese",
    },
    Fingerprint {
        sha256: "1f8202c77f952524fec841e978b142bcb6c627225b1e761f00058c2be6901d48",
        profile: DeviceProfile::Id,
        variant: "Japanese, October 2009",
    },
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
            evidence: if f.profile.s1c33_key().is_some() {
                "Verified SHA-256 (local tamaemu firmware boot comparison)"
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
