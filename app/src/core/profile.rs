// SPDX-License-Identifier: GPL-3.0-only
// SPDX-FileCopyrightText: 2026 Noctyros
// Applies to original RetroHatch contributions; third-party provenance is retained below and in THIRD_PARTY_LICENSES.md.
//! Device identities backed by the BrickEmuPy and tamaemu hardware configurations.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DeviceProfile {
    #[default]
    P1,
    P2,
    ConnectionV3Eu,
    TamaGo,
    Genjintch,
    Angel,
    Morino,
    Umino,
    Yasashii,
    On,
    MeetsFairy,
    MeetsMagical,
    MeetsFantasy,
    MeetsPastel,
    MeetsSanrio,
    MeetsSweets,
    Ps,
    Id,
    IdMelody,
    Idl,
    IdlEnglish,
    IdlPrincess,
    Idl15,
    PlusColor,
    PlusColorHexa,
    FourUPlus,
    FourUPlusEnglish,
}
impl DeviceProfile {
    pub const CLASSIC: [Self; 7] = [
        Self::P1,
        Self::P2,
        Self::Genjintch,
        Self::Angel,
        Self::Morino,
        Self::Umino,
        Self::Yasashii,
    ];
    pub const ALL: [Self; 27] = [
        Self::P1,
        Self::P2,
        Self::ConnectionV3Eu,
        Self::TamaGo,
        Self::Genjintch,
        Self::Angel,
        Self::Morino,
        Self::Umino,
        Self::Yasashii,
        Self::On,
        Self::MeetsFairy,
        Self::MeetsMagical,
        Self::MeetsFantasy,
        Self::MeetsPastel,
        Self::MeetsSanrio,
        Self::MeetsSweets,
        Self::Ps,
        Self::Id,
        Self::IdMelody,
        Self::Idl,
        Self::IdlEnglish,
        Self::IdlPrincess,
        Self::Idl15,
        Self::PlusColor,
        Self::PlusColorHexa,
        Self::FourUPlus,
        Self::FourUPlusEnglish,
    ];
    pub const MEETS: [Self; 6] = [
        Self::MeetsFairy,
        Self::MeetsMagical,
        Self::MeetsFantasy,
        Self::MeetsPastel,
        Self::MeetsSanrio,
        Self::MeetsSweets,
    ];
    pub const fn is_meets(self) -> bool {
        matches!(
            self,
            Self::MeetsFairy
                | Self::MeetsMagical
                | Self::MeetsFantasy
                | Self::MeetsPastel
                | Self::MeetsSanrio
                | Self::MeetsSweets
        )
    }
    pub const fn is_tg18(self) -> bool {
        matches!(self, Self::On) || self.is_meets()
    }
    pub const fn uses_framebuffer(self) -> bool {
        self.is_color() || matches!(self, Self::ConnectionV3Eu | Self::TamaGo)
    }
    pub const fn framebuffer_size(self) -> [usize; 2] {
        if matches!(self, Self::ConnectionV3Eu) {
            [352, 363]
        } else if matches!(self, Self::TamaGo) {
            [48, 31]
        } else {
            [128, 128]
        }
    }
    pub const fn is_color(self) -> bool {
        self.s1c33_key().is_some() || self.is_tg18()
    }
    pub const fn s1c33_key(self) -> Option<&'static str> {
        match self {
            Self::Ps => Some("ps"),
            Self::Id => Some("id"),
            Self::IdMelody => Some("id-melody"),
            Self::Idl => Some("idl"),
            Self::IdlEnglish => Some("idl-en"),
            Self::IdlPrincess => Some("idl-princess"),
            Self::Idl15 => Some("idl-15"),
            Self::PlusColor => Some("plus-color"),
            Self::PlusColorHexa => Some("plus-color-hexa"),
            Self::FourUPlus => Some("4u"),
            Self::FourUPlusEnglish => Some("4u-en"),

            _ => None,
        }
    }
    pub const fn id(self) -> &'static str {
        match self {
            Self::P1 => "tamagotchi-p1",
            Self::P2 => "tamagotchi-p2",
            Self::ConnectionV3Eu => "tamagotchi-connection-v3-eu",
            Self::TamaGo => "tamagotchi-tama-go",
            Self::Genjintch => "tamagotchi-genjintch",
            Self::Angel => "tamagotchi-angel",
            Self::Morino => "tamagotchi-morino",
            Self::Umino => "tamagotchi-umino",
            Self::Yasashii => "tamagotchi-yasashii",
            Self::On => "tamagotchi-on-meets",
            Self::MeetsFairy => "tamagotchi-meets-fairy",
            Self::MeetsMagical => "tamagotchi-meets-magical",
            Self::MeetsFantasy => "tamagotchi-meets-fantasy",
            Self::MeetsPastel => "tamagotchi-meets-pastel",
            Self::MeetsSanrio => "tamagotchi-meets-sanrio",
            Self::MeetsSweets => "tamagotchi-meets-sweets",
            Self::Ps => "tamagotchi-ps",
            Self::Id => "tamagotchi-id",
            Self::IdMelody => "tamagotchi-id-melody",
            Self::Idl => "tamagotchi-idl",
            Self::IdlEnglish => "tamagotchi-idl-en",
            Self::IdlPrincess => "tamagotchi-idl-princess",
            Self::Idl15 => "tamagotchi-idl-15",
            Self::PlusColor => "tamagotchi-plus-color",
            Self::PlusColorHexa => "tamagotchi-plus-color-hexa",
            Self::FourUPlus => "tamagotchi-4u-plus",
            Self::FourUPlusEnglish => "tamagotchi-4u-plus-en",
        }
    }
    pub const fn name(self) -> &'static str {
        match self {
            Self::P1 => "Tamagotchi P1",
            Self::P2 => "Tamagotchi P2",
            Self::ConnectionV3Eu => "Tamagotchi Connection V3",
            Self::TamaGo => "Tamagotchi Tama-Go",
            Self::Genjintch => "Tamagotchi Genjintch",
            Self::Angel => "Tamagotchi Angel (Angelgotchi)",
            Self::Morino => "Tamagotchi Morino",
            Self::Umino => "Tamagotchi Umino (Ocean)",
            Self::Yasashii => "Tamagotchi Yasashii",
            Self::On => "Tamagotchi On / Meets",
            Self::MeetsFairy => "Tamagotchi Meets Fairy",
            Self::MeetsMagical => "Tamagotchi Meets Magical",
            Self::MeetsFantasy => "Tamagotchi Meets Fantasy",
            Self::MeetsPastel => "Tamagotchi Meets Pastel",
            Self::MeetsSanrio => "Tamagotchi Meets Sanrio",
            Self::MeetsSweets => "Tamagotchi Meets Sweets",
            Self::Ps => "Tamagotchi P's",
            Self::Id => "Tamagotchi iD",
            Self::IdMelody => "Tamagotchi iD Lovely Melody",
            Self::Idl => "Tamagotchi iD L",
            Self::IdlEnglish => "Tamagotchi iD L English",
            Self::IdlPrincess => "Tamagotchi iD L Princess Spacy",
            Self::Idl15 => "Tamagotchi iD L 15th Anniversary",
            Self::PlusColor => "Tamagotchi Plus Color",
            Self::PlusColorHexa => "Tamagotchi Plus Color Hexagontchi",
            Self::FourUPlus => "Tamagotchi 4U+",
            Self::FourUPlusEnglish => "Tamagotchi 4U+ (English)",
        }
    }
    pub const fn rom_bytes(self) -> usize {
        match self {
            Self::P1 | Self::P2 => 0x3000,
            Self::ConnectionV3Eu => 524288,
            Self::TamaGo => 655360,
            Self::On
            | Self::MeetsFairy
            | Self::MeetsMagical
            | Self::MeetsFantasy
            | Self::MeetsPastel
            | Self::MeetsSanrio
            | Self::MeetsSweets
            | Self::Ps
            | Self::Idl
            | Self::IdlEnglish
            | Self::IdlPrincess
            | Self::Idl15
            | Self::FourUPlus
            | Self::FourUPlusEnglish => 0x80_0000,
            Self::Id | Self::IdMelody | Self::PlusColor | Self::PlusColorHexa => 0x40_0000,
            Self::Genjintch | Self::Angel | Self::Morino | Self::Umino | Self::Yasashii => 0x4000,
        }
    }
    pub const fn osc2_hz(self) -> u32 {
        match self {
            Self::P1 | Self::P2 => 1_600_000,
            Self::ConnectionV3Eu => 4_000_000,
            Self::TamaGo => 4_000_000,
            Self::On
            | Self::MeetsFairy
            | Self::MeetsMagical
            | Self::MeetsFantasy
            | Self::MeetsPastel
            | Self::MeetsSanrio
            | Self::MeetsSweets => 96_000_000,
            Self::FourUPlus | Self::FourUPlusEnglish => 30_000_000,
            Self::PlusColor | Self::PlusColorHexa => 20_000_000,
            Self::Ps
            | Self::Id
            | Self::IdMelody
            | Self::Idl
            | Self::IdlEnglish
            | Self::IdlPrincess
            | Self::Idl15 => 18_432_000,
            Self::Genjintch | Self::Angel | Self::Morino | Self::Umino | Self::Yasashii => {
                1_060_000
            }
        }
    }
    pub fn supports_keep_screen_awake(self) -> bool {
        // The reference 0x368 marker matches November v040, but not both earlier
        // verified dumps. Never clear an unverified byte in those firmware revisions.
        if self == Self::PlusColor {
            return false;
        }
        self.s1c33_key()
            .and_then(super::s1c33::profile::HardwareProfile::find)
            .is_some_and(|p| p.has_sleep_flag)
    }
    pub const fn has_knock(self) -> bool {
        matches!(
            self,
            Self::Genjintch | Self::Angel | Self::Morino | Self::Umino
        )
    }
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "p1" | "tamagotchi-p1" => Some(Self::P1),
            "p2" | "tamagotchi-p2" => Some(Self::P2),
            "v3-eu" | "connection-v3-eu" | "tamagotchi-connection-v3-eu" => {
                Some(Self::ConnectionV3Eu)
            }
            "tamago" | "tama-go" | "tamagotchi-tama-go" => Some(Self::TamaGo),
            "genjintch" | "tamagotchi-genjintch" => Some(Self::Genjintch),
            "angel" | "angelgotchi" | "tamagotchi-angel" => Some(Self::Angel),
            "morino" | "tamagotchi-morino" => Some(Self::Morino),
            "umino" | "ocean" | "tamagotchi-umino" | "tamagotchi-ocean" => Some(Self::Umino),
            "yasashii" | "tamagotchi-yasashii" => Some(Self::Yasashii),
            "meets-fairy" | "tamagotchi-meets-fairy" => Some(Self::MeetsFairy),
            "meets-magical" | "tamagotchi-meets-magical" => Some(Self::MeetsMagical),
            "meets-fantasy" | "tamagotchi-meets-fantasy" => Some(Self::MeetsFantasy),
            "meets-pastel" | "tamagotchi-meets-pastel" => Some(Self::MeetsPastel),
            "meets-sanrio" | "tamagotchi-meets-sanrio" => Some(Self::MeetsSanrio),
            "meets-sweets" | "tamagotchi-meets-sweets" => Some(Self::MeetsSweets),
            "ps" | "tamagotchi-ps" => Some(Self::Ps),
            "id" | "tamagotchi-id" => Some(Self::Id),
            "on" | "meets" | "tg18" | "tamagotchi-on-meets" => Some(Self::On),
            "id-melody" | "tamagotchi-id-melody" => Some(Self::IdMelody),
            "idl" | "tamagotchi-idl" => Some(Self::Idl),
            "idl-en" | "tamagotchi-idl-en" => Some(Self::IdlEnglish),
            "idl-princess" | "tamagotchi-idl-princess" => Some(Self::IdlPrincess),
            "idl-15" | "tamagotchi-idl-15" => Some(Self::Idl15),
            "plus-color" | "tamagotchi-plus-color" => Some(Self::PlusColor),
            "plus-color-hexa" | "hexagontchi" | "tamagotchi-plus-color-hexa" => {
                Some(Self::PlusColorHexa)
            }
            "4u" | "4u-plus" | "tamagotchi-4u-plus" => Some(Self::FourUPlus),
            "4u-en" | "tamagotchi-4u-plus-en" => Some(Self::FourUPlusEnglish),
            _ => None,
        }
    }
    // All seven classic reference profiles use RAM=768, K0/K1 pull-ups=15 and P3 dedicated=0.
    // Yasashii additionally sets all R-port push-pull masks to 15.
    // Other masks are not implied supported.
}
