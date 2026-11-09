// SPDX-License-Identifier: GPL-3.0-only
// SPDX-FileCopyrightText: 2026 Noktyros
// Applies to original RetroHatch contributions; third-party provenance is retained below and in THIRD_PARTY_LICENSES.md.
#[derive(Clone, Copy, Debug, Default)]
pub struct AudioStats {
    pub submitted: u64,
    pub completed: u64,
    pub empty_queue_events: u64,
    pub writes: u64,
    pub accepted: u64,
    pub pending: usize,
    pub max_write_gap_us: u64,
}
// WinMM PCM output adapter. All handles and stable buffers stay on one host thread.
#[cfg(windows)]
mod native {
    use super::AudioStats;
    use std::collections::VecDeque;
    use std::time::Instant;
    use windows_sys::Win32::Media::Audio::*;

    struct Buffer {
        header: Box<WAVEHDR>,
        _samples: Vec<i16>,
    }

    pub struct AudioSink {
        handle: HWAVEOUT,
        buffers: VecDeque<Buffer>,
        pending: Vec<i16>,
        stats: AudioStats,
        last_write: Option<Instant>,
        priming: bool,
    }

    impl AudioSink {
        pub fn new() -> Result<Self, String> {
            let format = WAVEFORMATEX {
                wFormatTag: 1,
                nChannels: 1,
                nSamplesPerSec: 44100,
                nAvgBytesPerSec: 88200,
                nBlockAlign: 2,
                wBitsPerSample: 16,
                cbSize: 0,
            };
            let mut handle = std::ptr::null_mut();
            // SAFETY: format and output handle storage remain valid for this synchronous call.
            let code = unsafe { waveOutOpen(&mut handle, WAVE_MAPPER, &format, 0, 0, 0) };
            if code != 0 {
                return Err(format!(
                    "Windows audio initialization failed (WinMM {code})."
                ));
            }
            // Prime 80 ms before starting, including after a genuine underrun.
            // This changes host latency only; sample values and emulated time are untouched.
            // SAFETY: the newly opened handle is valid and has no queued buffers.
            let code = unsafe { waveOutPause(handle) };
            if code != 0 {
                unsafe {
                    waveOutClose(handle);
                }
                return Err(format!("Windows audio priming failed (WinMM {code})."));
            }
            Ok(Self {
                handle,
                buffers: VecDeque::new(),
                pending: Vec::new(),
                stats: AudioStats::default(),
                last_write: None,
                priming: true,
            })
        }
        fn reap(&mut self) -> Result<(), String> {
            let had_buffers = !self.buffers.is_empty();
            while let Some(buffer) = self.buffers.front_mut() {
                // SAFETY: the boxed header is stable and remains owned until the driver completes it.
                let flags = unsafe { std::ptr::read_volatile(&raw const buffer.header.dwFlags) };
                if flags & WHDR_DONE == 0 {
                    break;
                }
                // SAFETY: WHDR_DONE permits unpreparing; no storage is freed before success.
                let code = unsafe {
                    waveOutUnprepareHeader(
                        self.handle,
                        &mut *buffer.header,
                        std::mem::size_of::<WAVEHDR>() as u32,
                    )
                };
                if code != 0 {
                    return Err(format!("Windows audio completion failed (WinMM {code})."));
                }
                self.stats.completed += buffer._samples.len() as u64;
                self.buffers.pop_front();
            }
            if had_buffers && self.buffers.is_empty() && !self.priming {
                self.stats.empty_queue_events += 1;
                // SAFETY: reaping confirmed that playback has drained.
                let code = unsafe { waveOutPause(self.handle) };
                if code != 0 {
                    return Err(format!("Windows audio re-priming failed (WinMM {code})."));
                }
                self.priming = true;
            }
            Ok(())
        }
        pub fn stats(&self) -> AudioStats {
            AudioStats {
                pending: self.pending.len(),
                ..self.stats
            }
        }
        /// Queue every sample in order; emulated time is never paced by the audio device.
        pub fn push(&mut self, samples: Vec<i16>) -> Result<bool, String> {
            self.reap()?;
            if self.pending.len().saturating_add(samples.len()) > 44100 {
                return Err("Windows audio backlog exceeded one second; playback stopped instead of dropping samples.".into());
            }
            self.stats.accepted += samples.len() as u64;
            self.pending.extend(samples);
            while self.pending.len() >= 882 && self.buffers.len() < 6 {
                let tail = self.pending.split_off(882);
                let mut data = std::mem::replace(&mut self.pending, tail);
                let mut header = Box::new(WAVEHDR {
                    lpData: data.as_mut_ptr().cast(),
                    dwBufferLength: (data.len() * 2) as u32,
                    ..WAVEHDR::default()
                });
                // SAFETY: both header and PCM allocations stay stable in the owned Buffer.
                let code = unsafe {
                    waveOutPrepareHeader(
                        self.handle,
                        &mut *header,
                        std::mem::size_of::<WAVEHDR>() as u32,
                    )
                };
                if code != 0 {
                    return Err(format!(
                        "Windows audio buffer preparation failed (WinMM {code})."
                    ));
                }
                // SAFETY: header is prepared, data is mono signed PCM16 at the opened rate.
                let code = unsafe {
                    waveOutWrite(
                        self.handle,
                        &mut *header,
                        std::mem::size_of::<WAVEHDR>() as u32,
                    )
                };
                if code != 0 {
                    // SAFETY: the failed write has not queued this buffer.
                    unsafe {
                        waveOutUnprepareHeader(
                            self.handle,
                            &mut *header,
                            std::mem::size_of::<WAVEHDR>() as u32,
                        );
                    }
                    return Err(format!("Windows audio playback failed (WinMM {code})."));
                }
                let now = Instant::now();
                if let Some(last) = self.last_write {
                    self.stats.max_write_gap_us = self
                        .stats
                        .max_write_gap_us
                        .max(now.duration_since(last).as_micros() as u64);
                }
                self.last_write = Some(now);
                self.stats.submitted += data.len() as u64;
                self.stats.writes += 1;
                self.buffers.push_back(Buffer {
                    header,
                    _samples: data,
                });
            }
            if self.priming && self.buffers.len() >= 4 {
                // SAFETY: four prepared buffers own 80 ms of stable PCM storage.
                let code = unsafe { waveOutRestart(self.handle) };
                if code != 0 {
                    return Err(format!("Windows audio start failed (WinMM {code})."));
                }
                self.priming = false;
            }
            Ok(true)
        }
        pub fn clear(&mut self) {
            self.priming = true;
            // SAFETY: the handle is open, and every queued allocation remains owned.
            if unsafe { waveOutReset(self.handle) } == 0 {
                let _ = self.reap();
                // SAFETY: waveOutReset completed all buffers; the handle stays open.
                unsafe {
                    waveOutPause(self.handle);
                }
            }
            self.pending.clear();
            self.last_write = None;
        }
    }

    impl Drop for AudioSink {
        fn drop(&mut self) {
            self.clear();
            // A failing driver must never retain pointers into freed Rust buffers.
            if !self.buffers.is_empty() {
                for buffer in self.buffers.drain(..) {
                    std::mem::forget(buffer);
                }
                return;
            }
            // SAFETY: all queued buffers have been completed and unprepared.
            unsafe {
                waveOutClose(self.handle);
            }
        }
    }
}
#[cfg(windows)]
pub use native::AudioSink;

#[cfg(not(windows))]
pub struct AudioSink;
#[cfg(not(windows))]
impl AudioSink {
    pub fn new() -> Result<Self, String> {
        Err("Native audio output is only implemented on Windows.".into())
    }
    pub fn push(&mut self, _: Vec<i16>) -> Result<bool, String> {
        Ok(false)
    }
    pub fn clear(&mut self) {}
    pub fn stats(&self) -> AudioStats {
        AudioStats::default()
    }
}

/// User volume is a host-only gain. MCU waveform phase and saved PCM remain untouched.
pub fn apply_volume(samples: &mut [i16], volume: u8) {
    let gain = i32::from(volume.min(100));
    if gain != 100 {
        for s in samples {
            *s = (i32::from(*s) * gain / 100) as i16;
        }
    }
}
#[cfg(test)]
mod gain_tests {
    #[test]
    fn host_volume_preserves_sign_and_never_changes_the_input_waveform_source() {
        let source = [-32768, -655, -100, 0, 100, 655, 32767];
        let mut quiet = source;
        super::apply_volume(&mut quiet, 25);
        assert_eq!(quiet, [-8192, -163, -25, 0, 25, 163, 8191]);
        let mut silent = source;
        super::apply_volume(&mut silent, 0);
        assert_eq!(silent, [0; 7]);
        let mut full = source;
        super::apply_volume(&mut full, 100);
        assert_eq!(full, source);
    }
}
