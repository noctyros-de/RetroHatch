// SPDX-License-Identifier: GPL-3.0-only
// SPDX-FileCopyrightText: 2026 Noktyros
// Applies to original RetroHatch contributions; third-party provenance is retained below and in THIRD_PARTY_LICENSES.md.
//! Emit original MCU PCM for a compact, firmware-free Python signal comparison.
use retrohatch::core::mcu::buzzer::Buzzer;
fn main() {
    let mut b = Buzzer::default();
    for frequency in 0..8 {
        b.set_frequency(frequency);
        b.control(1, false);
        b.set_on(true);
        for _ in 0..8 {
            let samples: Vec<_> = (0..137).map(|_| b.sample(44100).to_string()).collect();
            println!("{frequency}|{}|{}", b.level(), samples.join(","));
            for _ in 0..2048 {
                b.clock();
            }
        }
        b.set_on(false);
        println!("{frequency}|0|{}", b.sample(44100));
    }
}
