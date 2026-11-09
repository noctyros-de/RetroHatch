// SPDX-License-Identifier: GPL-3.0-only
// SPDX-FileCopyrightText: 2026 Noktyros
// Applies to original RetroHatch contributions; third-party provenance is retained below and in THIRD_PARTY_LICENSES.md.
//! Exercise the actual Windows queue with producer scheduling stalls, without firmware.
use retrohatch::host::audio::AudioSink;
use std::time::{Duration, Instant};
fn main() -> Result<(), String> {
    let mut sink = AudioSink::new()?;
    let start = Instant::now();
    let burst = std::env::args().any(|s| s == "--burst");
    let offset = if burst { 8820 } else { 0 };
    if burst {
        assert!(sink.push(vec![123; offset])?);
    }
    let mut frame = offset;
    let mut stall = 1u64;
    let mut drops = 0;
    while start.elapsed() < Duration::from_secs(6) {
        let target = offset + (start.elapsed().as_secs_f64() * 44100.0) as usize;
        if target > frame {
            let pcm = (frame..target)
                .map(|i| {
                    ((i as f64 * std::f64::consts::TAU * 1000.0 / 44100.0).sin() * 655.0) as i16
                })
                .collect();
            if !sink.push(pcm)? {
                drops += 1;
            }
            frame = target;
        }
        if start.elapsed() >= Duration::from_secs(stall) && stall < 6 {
            std::thread::sleep(Duration::from_millis(50));
            stall += 1;
        } else {
            std::thread::sleep(Duration::from_millis(5));
        }
    }
    println!("Produced={frame}, drops={drops}, {:?}", sink.stats());
    let stats = sink.stats();
    assert_eq!(stats.accepted, frame as u64);
    assert_eq!(stats.submitted + stats.pending as u64, stats.accepted);
    assert_eq!(drops, 0);
    assert_eq!(
        stats.empty_queue_events, 0,
        "50 ms producer stalls exhausted the audio reserve"
    );
    Ok(())
}
