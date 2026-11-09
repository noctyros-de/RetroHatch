//! Device configurations transcribed from the unchanged BrickEmuPy .brick profiles.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DeviceProfile {
    #[default]
    P1,
    Genjintch,
    Angel,
    Morino,
}
impl DeviceProfile {
    pub const ALL: [Self; 4] = [Self::P1, Self::Genjintch, Self::Angel, Self::Morino];
    pub const fn id(self) -> &'static str {
        match self {
            Self::P1 => "tamagotchi-p1",
            Self::Genjintch => "tamagotchi-genjintch",
            Self::Angel => "tamagotchi-angel",
            Self::Morino => "tamagotchi-morino",
        }
    }
    pub const fn name(self) -> &'static str {
        match self {
            Self::P1 => "Tamagotchi P1",
            Self::Genjintch => "Tamagotchi Genjintch",
            Self::Angel => "Tamagotchi Angel (Angelgotchi)",
            Self::Morino => "Tamagotchi Morino",
        }
    }
    pub const fn rom_bytes(self) -> usize {
        match self {
            Self::P1 => 0x3000,
            Self::Genjintch | Self::Angel | Self::Morino => 0x4000,
        }
    }
    pub const fn osc2_hz(self) -> u32 {
        match self {
            Self::P1 => 1_600_000,
            Self::Genjintch | Self::Angel | Self::Morino => 1_060_000,
        }
    }
    pub const fn has_knock(self) -> bool {
        matches!(self, Self::Genjintch | Self::Angel | Self::Morino)
    }
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "p1" | "tamagotchi-p1" => Some(Self::P1),
            "genjintch" | "tamagotchi-genjintch" => Some(Self::Genjintch),
            "angel" | "angelgotchi" | "tamagotchi-angel" => Some(Self::Angel),
            "morino" | "tamagotchi-morino" => Some(Self::Morino),
            _ => None,
        }
    }
    // All four reference profiles use RAM=768, K0/K1 pull-ups=15 and P3 dedicated=0.
    // These are the existing MCU mask defaults; no other masks are implied supported.
}
