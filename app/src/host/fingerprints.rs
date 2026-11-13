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
        sha256: "7f4da8dc89d3843b42b3ba73b9311989332706978af5367b5ef041c365a1902f",
        profile: DeviceProfile::ConnectionV3Eu,
        variant: "Shared EU/US firmware; European hardware configuration",
    },
    Fingerprint {
        sha256: "197d9aa7c5f0bea21b2f6f8d1ce53de5fee09cfba708731b5380c68dee255444",
        profile: DeviceProfile::TamaGo,
        variant: "",
    },
    Fingerprint {
        sha256: "d4639d3ecf464762c7911dccae6404ee3737c9bea71bd9fc43b20a6ad1e53b8a",
        profile: DeviceProfile::MeetsFairy,
        variant: "Japanese",
    },
    Fingerprint {
        sha256: "ab2cc5c488b3b0bce2b5d5598b148bf3105394535d9ce4f1b1d646dfd31c6953",
        profile: DeviceProfile::MeetsMagical,
        variant: "Japanese",
    },
    Fingerprint {
        sha256: "315c473af925b47b3025db1ba4b3c66d923a4c3eadbce877065e22d255a1e587",
        profile: DeviceProfile::MeetsFantasy,
        variant: "Japanese",
    },
    Fingerprint {
        sha256: "b527e68dcdf10cf4b56228512f14b6e587be6e73ce8ef5abc1a7eb91b1ebccac",
        profile: DeviceProfile::MeetsPastel,
        variant: "Japanese",
    },
    Fingerprint {
        sha256: "43479ce1253f4ae6c8f3c25fce9f0d091e3a6c2400a68e707208e90334be3ce7",
        profile: DeviceProfile::MeetsSanrio,
        variant: "Japanese",
    },
    Fingerprint {
        sha256: "1b0972205bc104183e2406510f264d7ed4193bb5dc08c4811447ab986bbd2305",
        profile: DeviceProfile::MeetsSweets,
        variant: "Japanese",
    },
    Fingerprint {
        sha256: "6722497ff4ce7c12fb380da31a067485ad8e42e0de156d770e426c9059d5eb6c",
        profile: DeviceProfile::Yasashii,
        variant: "Japanese",
    },
    Fingerprint {
        sha256: "a8f42d045529a2bdcd9db2aeabc3b04b1461affb93ff681f4b12da5967239c57",
        profile: DeviceProfile::FourUPlus,
        variant: "Japanese",
    },
    Fingerprint {
        sha256: "a9fa972ec0e481214902906b3b91ab5d56142329dae6961e919442be7f36162f",
        profile: DeviceProfile::FourUPlusEnglish,
        variant: "English translation; some Japanese input labels remain",
    },
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
            evidence: if f.profile==DeviceProfile::ConnectionV3Eu{"Verified local SPLB32 execution; EU hardware strap selected, not a region-unique ROM hash"} else if f.profile == DeviceProfile::TamaGo {"Verified SHA-256 (local Tama-Go firmware boot)"} else if f.profile.is_tg18() {
                "Verified full SHA-256; local TG18 firmware validation and upstream code identity"
            } else if f.profile.s1c33_key().is_some() {
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

/// Narrow TG18 editions once per import; exact hashes take precedence over code metadata.
pub fn tg18_candidates(bytes: &[u8], hash: &str) -> Option<Vec<DeviceProfile>> {
    if !tg18::is_image(bytes) {
        return None;
    }
    let mut choices = vec![DeviceProfile::On];
    let exact: Vec<_> = matches(hash, CATALOG)
        .into_iter()
        .filter(|a| a.profile.is_meets())
        .map(|a| a.profile)
        .collect();
    if !exact.is_empty() {
        choices.extend(exact);
        return Some(choices);
    }
    if let Some(rom) = tg18::roms::identify(bytes) {
        if rom.region == "jp" {
            let slug = if rom.slug == "magic" {
                "magical"
            } else {
                rom.slug
            };
            if let Some(p) = DeviceProfile::parse(&format!("meets-{slug}")) {
                choices.push(p);
            }
        }
    } else {
        choices.extend(DeviceProfile::MEETS);
    }
    Some(choices)
}
