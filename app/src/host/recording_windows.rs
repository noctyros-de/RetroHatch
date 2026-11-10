// SPDX-License-Identifier: GPL-3.0-only
// SPDX-FileCopyrightText: 2026 Noctyros
//! Minimal SDK ABI bindings for the Windows Media Foundation sink writer.
//! Interface slots/signatures follow mfobjects.h/mfreadwrite.h; no upstream emulator code.
use super::super::screenshots::Image;
use std::{ffi::c_void, path::Path};
use windows_sys::core::GUID;
type Raw = *mut c_void;
type Factory = unsafe extern "system" fn(*mut Raw) -> i32;
type Startup = unsafe extern "system" fn(u32, u32) -> i32;
type Shutdown = unsafe extern "system" fn() -> i32;
type Memory = unsafe extern "system" fn(u32, *mut Raw) -> i32;
type Sink = unsafe extern "system" fn(*const u16, Raw, Raw, *mut Raw) -> i32;
struct Dll(windows_sys::Win32::Foundation::HMODULE);
impl Dll {
    fn load(name: &str) -> Result<Self, String> {
        use windows_sys::Win32::System::LibraryLoader::*;
        let wide: Vec<u16> = name.encode_utf16().chain(Some(0)).collect();
        let handle = unsafe {
            LoadLibraryExW(
                wide.as_ptr(),
                std::ptr::null_mut(),
                LOAD_LIBRARY_SEARCH_SYSTEM32,
            )
        };
        if handle.is_null() {
            return Err(format!(
                "{name} is unavailable. Recording requires Windows Media Foundation (install Media Feature Pack on Windows N)."
            ));
        }
        Ok(Self(handle))
    }
    fn symbol(&self, name: &'static [u8]) -> Result<Raw, String> {
        unsafe { windows_sys::Win32::System::LibraryLoader::GetProcAddress(self.0, name.as_ptr()) }
            .map(|f| f as *const () as Raw)
            .ok_or("Windows Media Foundation entry point unavailable".into())
    }
}
impl Drop for Dll {
    fn drop(&mut self) {
        unsafe {
            windows_sys::Win32::Foundation::FreeLibrary(self.0);
        }
    }
}
struct Api {
    startup: Startup,
    shutdown: Shutdown,
    media_type: Factory,
    sample: Factory,
    memory: Memory,
    sink: Sink,
    _plat: Dll,
    _readwrite: Dll,
}
impl Api {
    fn load() -> Result<Self, String> {
        let plat = Dll::load("mfplat.dll")?;
        let readwrite = Dll::load("mfreadwrite.dll")?;
        // SAFETY: function names/signatures match the installed Windows SDK API.
        unsafe {
            Ok(Self {
                startup: std::mem::transmute::<Raw, Startup>(plat.symbol(b"MFStartup\0")?),
                shutdown: std::mem::transmute::<Raw, Shutdown>(plat.symbol(b"MFShutdown\0")?),
                media_type: std::mem::transmute::<Raw, Factory>(
                    plat.symbol(b"MFCreateMediaType\0")?,
                ),
                sample: std::mem::transmute::<Raw, Factory>(plat.symbol(b"MFCreateSample\0")?),
                memory: std::mem::transmute::<Raw, Memory>(plat.symbol(b"MFCreateMemoryBuffer\0")?),
                sink: std::mem::transmute::<Raw, Sink>(
                    readwrite.symbol(b"MFCreateSinkWriterFromURL\0")?,
                ),
                _plat: plat,
                _readwrite: readwrite,
            })
        }
    }
}
fn check(hr: i32) -> Result<(), String> {
    if hr >= 0 {
        Ok(())
    } else {
        Err(format!(
            "Windows MP4/H.264 encoder failed ({hr:#010x}). Check disk space and Windows Media Foundation/Media Feature Pack availability."
        ))
    }
}
struct Com(Raw);
macro_rules! method {
    ($obj:expr, $slot:expr, $signature:ty) => {{
        // SAFETY: every call uses the documented SDK interface slot and exact ABI signature.
        std::mem::transmute::<Raw, $signature>(*(*($obj.0 as *const *const Raw)).add($slot))
    }};
}
impl Drop for Com {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe {
                method!(self, 2, unsafe extern "system" fn(Raw) -> u32)(self.0);
            }
        }
    }
}
impl Com {
    fn create(factory: unsafe extern "system" fn(*mut Raw) -> i32) -> Result<Self, String> {
        let mut ptr = std::ptr::null_mut();
        check(unsafe { factory(&mut ptr) })?;
        if ptr.is_null() {
            return Err("Windows returned an empty media interface".into());
        }
        Ok(Self(ptr))
    }
    fn guid(&self, key: u128, value: u128) -> Result<(), String> {
        check(unsafe {
            method!(
                self,
                24,
                unsafe extern "system" fn(Raw, *const GUID, *const GUID) -> i32
            )(self.0, &GUID::from_u128(key), &GUID::from_u128(value))
        })
    }
    fn u32(&self, key: u128, value: u32) -> Result<(), String> {
        check(unsafe {
            method!(
                self,
                21,
                unsafe extern "system" fn(Raw, *const GUID, u32) -> i32
            )(self.0, &GUID::from_u128(key), value)
        })
    }
    fn u64(&self, key: u128, value: u64) -> Result<(), String> {
        check(unsafe {
            method!(
                self,
                22,
                unsafe extern "system" fn(Raw, *const GUID, u64) -> i32
            )(self.0, &GUID::from_u128(key), value)
        })
    }
}
struct Foundation {
    api: Api,
}
impl Foundation {
    fn new() -> Result<Self, String> {
        use windows_sys::Win32::System::Com::*;
        let api = Api::load()?;
        check(unsafe { CoInitializeEx(std::ptr::null(), COINIT_MULTITHREADED as u32) })?;
        if let Err(e) = check(unsafe { (api.startup)(0x20070, 0) }) {
            unsafe {
                CoUninitialize();
            }
            return Err(e);
        }
        Ok(Self { api })
    }
}
impl Drop for Foundation {
    fn drop(&mut self) {
        unsafe {
            (self.api.shutdown)();
            windows_sys::Win32::System::Com::CoUninitialize();
        }
    }
}
const MAJOR: u128 = 0x48eba18e_f8c9_4687_bf11_0a74c9f96a8f;
const SUBTYPE: u128 = 0xf7e34c9a_42e8_4714_b74b_cb29d72c35e5;
const VIDEO: u128 = 0x73646976_0000_0010_8000_00aa00389b71;
const H264: u128 = 0x34363248_0000_0010_8000_00aa00389b71;
const RGB32: u128 = 0x00000016_0000_0010_8000_00aa00389b71;
const SIZE: u128 = 0x1652c33d_d6b2_4012_b834_72030849a37d;
const RATE: u128 = 0xc459a2e8_3d2c_4e44_b132_fee5156c7bb0;
const ASPECT: u128 = 0xc6376a1e_8d0a_4027_be45_6d9a0ad39bb6;
const INTERLACE: u128 = 0xe2724bb8_e676_4806_b4b2_a8d6efb44ccd;
const BITRATE: u128 = 0x20332624_fb0d_4d9e_bd0d_cbf6786c102e;
// MF wants 100 ns timestamps, with exact rational 30 Hz boundaries.
pub(super) fn sample_time(index: u64) -> (i64, i64) {
    let start = (index * 10_000_000 / 30) as i64;
    (start, ((index + 1) * 10_000_000 / 30) as i64 - start)
}
pub(super) struct Encoder {
    writer: Com,
    _foundation: Foundation,
    stream: u32,
    pub width: usize,
    pub height: usize,
}
impl Encoder {
    pub fn new(path: &Path, width: usize, height: usize) -> Result<Self, String> {
        use std::os::windows::ffi::OsStrExt;
        let foundation = Foundation::new()?;
        let mut raw = std::ptr::null_mut();
        let url: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
        check(unsafe {
            (foundation.api.sink)(
                url.as_ptr(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                &mut raw,
            )
        })?;
        let writer = Com(raw);
        let mut stream = 0;
        for output in [true, false] {
            let media = Com::create(foundation.api.media_type)?;
            media.guid(MAJOR, VIDEO)?;
            media.guid(SUBTYPE, if output { H264 } else { RGB32 })?;
            media.u64(SIZE, ((width as u64) << 32) | height as u64)?;
            media.u64(RATE, (30u64 << 32) | 1)?;
            media.u64(ASPECT, (1u64 << 32) | 1)?;
            media.u32(INTERLACE, 2)?;
            if output {
                media.u32(
                    BITRATE,
                    (width * height * 8).clamp(1_000_000, 12_000_000) as u32,
                )?;
                check(unsafe {
                    method!(
                        writer,
                        3,
                        unsafe extern "system" fn(Raw, Raw, *mut u32) -> i32
                    )(writer.0, media.0, &mut stream)
                })?;
            } else {
                check(unsafe {
                    method!(
                        writer,
                        4,
                        unsafe extern "system" fn(Raw, u32, Raw, Raw) -> i32
                    )(writer.0, stream, media.0, std::ptr::null_mut())
                })?;
            }
        }
        check(unsafe { method!(writer, 5, unsafe extern "system" fn(Raw) -> i32)(writer.0) })?;
        Ok(Self {
            writer,
            _foundation: foundation,
            stream,
            width,
            height,
        })
    }
    pub fn write(&mut self, bgra: &[u8], index: u64) -> Result<(), String> {
        if bgra.len() != self.width * self.height * 4 {
            return Err("Recording buffer dimensions changed".into());
        }
        let mut raw = std::ptr::null_mut();
        check(unsafe { (self._foundation.api.memory)(bgra.len() as u32, &mut raw) })?;
        let buffer = Com(raw);
        let mut ptr = std::ptr::null_mut();
        let mut max = 0;
        check(unsafe {
            method!(
                buffer,
                3,
                unsafe extern "system" fn(Raw, *mut *mut u8, *mut u32, *mut u32) -> i32
            )(buffer.0, &mut ptr, &mut max, std::ptr::null_mut())
        })?;
        if ptr.is_null() || (max as usize) < bgra.len() {
            unsafe {
                method!(buffer, 4, unsafe extern "system" fn(Raw) -> i32)(buffer.0);
            }
            return Err("Windows supplied an invalid recording buffer".into());
        }
        unsafe {
            std::ptr::copy_nonoverlapping(bgra.as_ptr(), ptr, bgra.len());
        }
        check(unsafe { method!(buffer, 4, unsafe extern "system" fn(Raw) -> i32)(buffer.0) })?;
        check(unsafe {
            method!(buffer, 6, unsafe extern "system" fn(Raw, u32) -> i32)(
                buffer.0,
                bgra.len() as u32,
            )
        })?;
        let sample = Com::create(self._foundation.api.sample)?;
        check(unsafe {
            method!(sample, 42, unsafe extern "system" fn(Raw, Raw) -> i32)(sample.0, buffer.0)
        })?;
        let (start, duration) = sample_time(index);
        check(unsafe {
            method!(sample, 36, unsafe extern "system" fn(Raw, i64) -> i32)(sample.0, start)
        })?;
        check(unsafe {
            method!(sample, 38, unsafe extern "system" fn(Raw, i64) -> i32)(sample.0, duration)
        })?;
        check(unsafe {
            method!(
                self.writer,
                6,
                unsafe extern "system" fn(Raw, u32, Raw) -> i32
            )(self.writer.0, self.stream, sample.0)
        })
    }
    pub fn finish(&mut self) -> Result<(), String> {
        check(unsafe {
            method!(self.writer, 11, unsafe extern "system" fn(Raw) -> i32)(self.writer.0)
        })
    }
}
/// Letterbox resizing preserves aspect ratio; unchanged frames remain byte-sharp.
/// RGB32 is bottom-up BGRX, as required by the sink writer input media type.
pub(super) fn canvas(image: &Image, w: usize, h: usize) -> Vec<u8> {
    let scale = (w as f64 / image.width as f64)
        .min(h as f64 / image.height as f64)
        .min(1.0);
    let dw = (image.width as f64 * scale).round().max(1.0) as usize;
    let dh = (image.height as f64 * scale).round().max(1.0) as usize;
    let mut bgra = vec![0; w * h * 4];
    for y in 0..dh {
        for x in 0..dw {
            let src = ((y * image.height / dh) * image.width + x * image.width / dw) * 3;
            let dst = (((h - 1 - (y + (h - dh) / 2)) * w) + x + (w - dw) / 2) * 4;
            bgra[dst..dst + 4].copy_from_slice(&[
                image.rgb[src + 2],
                image.rgb[src + 1],
                image.rgb[src],
                255,
            ]);
        }
    }
    bgra
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn missing_optional_media_library_reports_an_error_without_a_process_load_failure() {
        assert!(Dll::load("RetroHatch-deliberately-unavailable-codec.dll").is_err());
    }
    #[test]
    fn video_timestamps_and_bottom_up_pixels_preserve_duration_orientation_and_padding() {
        assert_eq!(sample_time(30), (10_000_000, 333_333));
        assert_eq!((0..30).map(|i| sample_time(i).1).sum::<i64>(), 10_000_000);
        let image = Image::new(1, 2, vec![255, 0, 0, 0, 0, 255]).unwrap();
        assert_eq!(
            canvas(&image, 2, 2),
            vec![255, 0, 0, 255, 0, 0, 0, 0, 0, 0, 255, 255, 0, 0, 0, 0]
        );
    }
}
