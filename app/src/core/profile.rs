// SPDX-License-Identifier: GPL-3.0-only
// SPDX-FileCopyrightText: 2026 Noctyros
// Applies to original RetroHatch contributions; third-party provenance is retained below and in THIRD_PARTY_LICENSES.md.
//! Device configurations transcribed from the unchanged BrickEmuPy .brick profiles.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DeviceProfile {
    #[default]
    P1,
    P2,
    Genjintch,
    Angel,
    Morino,
    Umino,
    On,
    Ps,
}
impl DeviceProfile {
    pub const CLASSIC: [Self; 6] = [
        Self::P1,
        Self::P2,
        Self::Genjintch,
        Self::Angel,
        Self::Morino,
        Self::Umino,
    ];
    pub const ALL: [Self; 8] = [
        Self::P1,
        Self::P2,
        Self::Genjintch,
        Self::Angel,
        Self::Morino,
        Self::Umino,
        Self::On,
        Self::Ps,
    ];
    pub const fn is_color(self) -> bool {
        matches!(self, Self::On | Self::Ps)
    }
    pub const fn id(self) -> &'static str {
        match self {
            Self::P1 => "tamagotchi-p1",
            Self::P2 => "tamagotchi-p2",
            Self::Genjintch => "tamagotchi-genjintch",
            Self::Angel => "tamagotchi-angel",
            Self::Morino => "tamagotchi-morino",
            Self::Umino => "tamagotchi-umino",
            Self::On => "tamagotchi-on-meets",
            Self::Ps => "tamagotchi-ps",
        }
    }
    pub const fn name(self) -> &'static str {
        match self {
            Self::P1 => "Tamagotchi P1",
            Self::P2 => "Tamagotchi P2",
            Self::Genjintch => "Tamagotchi Genjintch",
            Self::Angel => "Tamagotchi Angel (Angelgotchi)",
            Self::Morino => "Tamagotchi Morino",
            Self::Umino => "Tamagotchi Umino (Ocean)",
            Self::On => "Tamagotchi On / Meets",
            Self::Ps => "Tamagotchi P's",
        }
    }
    pub const fn rom_bytes(self) -> usize {
        match self {
            Self::P1 | Self::P2 => 0x3000,
            Self::On | Self::Ps => 0x80_0000,
            Self::Genjintch | Self::Angel | Self::Morino | Self::Umino => 0x4000,
        }
    }
    pub const fn osc2_hz(self) -> u32 {
        match self {
            Self::P1 | Self::P2 => 1_600_000,
            Self::On => 96_000_000,
            Self::Ps => 18_432_000,
            Self::Genjintch | Self::Angel | Self::Morino | Self::Umino => 1_060_000,
        }
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
            "genjintch" | "tamagotchi-genjintch" => Some(Self::Genjintch),
            "angel" | "angelgotchi" | "tamagotchi-angel" => Some(Self::Angel),
            "morino" | "tamagotchi-morino" => Some(Self::Morino),
            "umino" | "ocean" | "tamagotchi-umino" | "tamagotchi-ocean" => Some(Self::Umino),
            "ps" | "tamagotchi-ps" => Some(Self::Ps),
            "on" | "meets" | "tg18" | "tamagotchi-on-meets" => Some(Self::On),
            _ => None,
        }
    }
    // All six reference profiles use RAM=768, K0/K1 pull-ups=15 and P3 dedicated=0.
    // These are the existing MCU mask defaults; no other masks are implied supported.
}
