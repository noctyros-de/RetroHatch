//! Small WinMM PCM output adapter. All handles and stable buffers stay on one host thread.
#[cfg(windows)]
mod native {
    use std::collections::VecDeque;
    use windows_sys::Win32::Media::Audio::*;

    struct Buffer {
        header: Box<WAVEHDR>,
        _samples: Vec<i16>,
    }

    pub struct AudioSink {
        handle: HWAVEOUT,
        buffers: VecDeque<Buffer>,
        pending: Vec<i16>,
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
            Ok(Self {
                handle,
                buffers: VecDeque::new(),
                pending: Vec::new(),
            })
        }
        fn reap(&mut self) -> Result<(), String> {
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
                self.buffers.pop_front();
            }
            Ok(())
        }
        /// Returns false when output is behind; emulated time is never paced by the audio device.
        pub fn push(&mut self, samples: Vec<i16>) -> Result<bool, String> {
            self.reap()?;
            if self.buffers.len() >= 6 {
                self.pending.clear();
                return Ok(false);
            }
            self.pending.extend(samples);
            while self.pending.len() >= 882 {
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
                self.buffers.push_back(Buffer {
                    header,
                    _samples: data,
                });
            }
            Ok(true)
        }
        pub fn clear(&mut self) {
            // SAFETY: the handle is open, and every queued allocation remains owned.
            if unsafe { waveOutReset(self.handle) } == 0 {
                let _ = self.reap();
            }
            self.pending.clear();
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
}
