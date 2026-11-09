//! E0C6200sound.py control behavior (BrickEmuPy, CC0), with deterministic PCM synthesis.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Buzzer {
    frequency: u8,
    output_frequency: u8,
    step: u8,
    envelope: bool,
    cycle: u16,
    envelope_counter: u16,
    one_shot: u16,
    sound_on: bool,
    level: u8,
    phase: u64,
}

impl Default for Buzzer {
    fn default() -> Self {
        Self {
            frequency: 0,
            output_frequency: 0,
            step: 7,
            envelope: false,
            cycle: 2048,
            envelope_counter: 0,
            one_shot: 0,
            sound_on: false,
            level: 0,
            phase: 0,
        }
    }
}

impl Buzzer {
    pub fn frequency_hz(&self) -> f64 {
        32768.0 / f64::from(self.divisor())
    }
    fn divisor(&self) -> u16 {
        [8, 10, 12, 14, 16, 20, 24, 28][self.frequency as usize]
    }
    pub fn level(&self) -> u8 {
        self.level
    }
    pub fn ringing(&self) -> bool {
        self.one_shot > 0
    }
    pub fn set_frequency(&mut self, value: u8) {
        self.frequency = value & 7;
        if self.sound_on {
            self.level = self.step;
            self.output_frequency = self.frequency;
        }
    }
    pub fn set_on(&mut self, on: bool) {
        self.sound_on = on;
        self.one_shot = 0;
        self.level = if on { self.step } else { 0 };
        if on {
            self.output_frequency = self.frequency;
        }
        if on && self.envelope {
            self.envelope_counter = self.cycle;
        }
        if !on {
            self.envelope_counter = 0;
        }
    }
    pub fn control(&mut self, value: u8, long_shot: bool) {
        self.cycle = if value & 2 != 0 { 4096 } else { 2048 };
        if value & 8 != 0 && self.one_shot == 0 {
            self.one_shot = if long_shot { 2048 } else { 1024 };
            if !self.sound_on {
                self.level = self.step;
                self.output_frequency = self.frequency;
            }
        }
        self.envelope = value & 1 != 0;
        self.step = 7;
        if !self.envelope {
            self.envelope_counter = 0;
            self.level = 0;
        }
        // ENVRST also sets step=7; envelope phase is intentionally preserved.
    }
    pub fn clock(&mut self) {
        if self.one_shot > 0 {
            self.one_shot -= 1;
            if self.one_shot == 0 {
                self.level = 0;
            }
        }
        if self.envelope_counter > 0 {
            self.envelope_counter -= 1;
            if self.envelope_counter == 0 {
                if self.step > 0 {
                    self.level = self.step;
                    self.output_frequency = self.frequency;
                    self.step -= 1;
                }
                self.envelope_counter = self.cycle;
            }
        }
    }
    #[cfg(test)]
    pub(crate) fn inspect(&self) -> [u16; 7] {
        [
            self.one_shot,
            u16::from(self.step),
            self.envelope_counter,
            u16::from(self.sound_on),
            u16::from(self.envelope),
            u16::from(self.level),
            u16::from(self.output_frequency),
        ]
    }
    pub fn sample(&mut self, rate: u32) -> i16 {
        if self.level == 0 {
            self.phase = 0;
            return 0;
        }
        let period = u64::from(rate)
            * u64::from([8u16, 10, 12, 14, 16, 20, 24, 28][self.output_frequency as usize]);
        // A frequency-independent turn accumulator preserves phase across retuning.
        let wave = (std::f64::consts::TAU * self.phase as f64 / 18446744073709551616.0).sin();
        let increment = ((32768u128 << 64) / u128::from(period)) as u64;
        self.phase = self.phase.wrapping_add(increment);
        // Match the reference's clipped sine: amplitude=5*step/7, output ceiling=2%.
        (wave * 655.0 * 5.0 * f64::from(self.level) / 7.0).clamp(-655.0, 655.0) as i16
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn retuning_preserves_waveform_phase_and_silence_restarts_it() {
        let mut retuned = Buzzer::default();
        retuned.set_on(true);
        let mut unchanged = retuned.clone();
        for _ in 0..31 {
            assert_eq!(retuned.sample(44100), unchanged.sample(44100));
        }
        retuned.set_frequency(7);
        assert_eq!(retuned.sample(44100), unchanged.sample(44100));
        let changed: Vec<_> = (0..64).map(|_| retuned.sample(44100)).collect();
        let original: Vec<_> = (0..64).map(|_| unchanged.sample(44100)).collect();
        assert_ne!(changed, original);
        retuned.set_on(false);
        assert_eq!(retuned.sample(44100), 0);
        retuned.set_on(true);
        assert_eq!(retuned.sample(44100), 0);
    }
}
