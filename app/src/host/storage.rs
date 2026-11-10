// SPDX-License-Identifier: GPL-3.0-only
// SPDX-FileCopyrightText: 2026 Noctyros
// Applies to original RetroHatch contributions; third-party provenance is retained below and in THIRD_PARTY_LICENSES.md.
//! Ordered background I/O and crash-safe replacement in the destination directory.
use crate::core::state::{MAGIC, sha256};
const MAX_FILE: usize = 64 * 1024 * 1024;
use std::{
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicU64, Ordering},
        mpsc,
    },
    thread,
};
static TEMP_ID: AtomicU64 = AtomicU64::new(0);
pub fn user_dir() -> Result<PathBuf, String> {
    if let Some(p) = std::env::var_os("RETROHATCH_DATA_DIR") {
        return Ok(p.into());
    }
    #[cfg(windows)]
    {
        std::env::var_os("LOCALAPPDATA")
            .map(|p| PathBuf::from(p).join("RetroHatch"))
            .ok_or("Windows LOCALAPPDATA is unavailable".into())
    }
    #[cfg(not(windows))]
    {
        std::env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|p| PathBuf::from(p).join(".local/share")))
            .map(|p| p.join("RetroHatch"))
            .ok_or("User data directory is unavailable".into())
    }
}
pub fn read_bounded(path: &Path, limit: usize) -> Result<Option<Vec<u8>>, String> {
    let file = match std::fs::File::open(path) {
        Ok(f) => f,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(format!("Cannot read {}: {e}", path.display())),
    };
    let mut bytes = Vec::new();
    file.take(limit as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > limit {
        return Err("File exceeds the supported size limit".into());
    }
    Ok(Some(bytes))
}
pub fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let parent = path.parent().ok_or("Destination has no parent directory")?;
    std::fs::create_dir_all(parent)
        .map_err(|e| format!("Cannot create {}: {e}", parent.display()))?;
    let temp = parent.join(format!(
        ".{}.{}.{}.tmp",
        path.file_name().unwrap_or_default().to_string_lossy(),
        std::process::id(),
        TEMP_ID.fetch_add(1, Ordering::Relaxed)
    ));
    let result = (|| {
        let mut f = std::fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temp)
            .map_err(|e| e.to_string())?;
        f.write_all(bytes)
            .and_then(|_| f.sync_all())
            .map_err(|e| e.to_string())?;
        drop(f);
        #[cfg(windows)]
        {
            use std::os::windows::ffi::OsStrExt;
            use windows_sys::Win32::Storage::FileSystem::{
                MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH, MoveFileExW,
            };
            let a: Vec<u16> = temp.as_os_str().encode_wide().chain(Some(0)).collect();
            let b: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
            // SAFETY: both NUL-terminated paths remain alive; replacement stays on one volume.
            if unsafe {
                MoveFileExW(
                    a.as_ptr(),
                    b.as_ptr(),
                    MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
                )
            } == 0
            {
                return Err(std::io::Error::last_os_error().to_string());
            }
        }
        #[cfg(not(windows))]
        {
            std::fs::rename(&temp, path).map_err(|e| e.to_string())?;
            std::fs::File::open(parent)
                .and_then(|f| f.sync_all())
                .map_err(|e| e.to_string())?;
        }
        Ok(())
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&temp);
    }
    result.map_err(|e: String| format!("Cannot write {}: {e}", path.display()))
}
fn state_write(path: &Path, data: &[u8]) -> Result<(), String> {
    if data.len() < 81
        || data.len() > MAX_FILE
        || !data.starts_with(MAGIC)
        || sha256(&data[..data.len() - 32]).as_slice() != &data[data.len() - 32..]
    {
        return Err("Refusing to write an invalid save envelope".into());
    }

    if !path
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("rhstate"))
    {
        return Err(
            "Save files must use the .rhstate extension. ROM files are never overwritten.".into(),
        );
    }
    if let Some(old) = read_bounded(path, MAX_FILE)? {
        if !old.starts_with(MAGIC) {
            return Err(format!(
                "Refusing to overwrite a non-save file: {}",
                path.display()
            ));
        }
        if old.len() >= 81 && sha256(&old[..old.len() - 32]).as_slice() == &old[old.len() - 32..] {
            let backup = path.with_extension("rhstate.bak");
            if let Some(previous) = read_bounded(&backup, MAX_FILE)?
                && !previous.starts_with(MAGIC)
            {
                return Err("Refusing to overwrite a non-save backup file".into());
            }
            atomic_write(&backup, &old)?;
        }
    }
    atomic_write(path, data)
}
enum Job {
    Write(u64, PathBuf, Vec<u8>),
    Capture(u64, PathBuf, super::emulator::SaveCapture),
    Read(u64, PathBuf),
    Barrier(u64),
    Stop,
}
pub struct Completion {
    pub id: u64,
    pub result: Result<Option<Vec<u8>>, String>,
}
pub struct Writer {
    tx: mpsc::SyncSender<Job>,
    rx: mpsc::Receiver<Completion>,
    thread: Option<thread::JoinHandle<()>>,
    next: u64,
}
impl Writer {
    pub fn new() -> Self {
        let (tx, jobs) = mpsc::sync_channel(32);
        let (done, rx) = mpsc::channel();
        let thread = thread::Builder::new()
            .name("RetroHatch save writer".into())
            .spawn(move || {
                for job in jobs {
                    let (id, result) = match job {
                        Job::Write(id, path, data) => (id, state_write(&path, &data).map(|_| None)),
                        Job::Capture(id, path, capture) => (
                            id,
                            capture()
                                .and_then(|data| state_write(&path, &data))
                                .map(|_| None),
                        ),
                        Job::Read(id, path) => (id, read_bounded(&path, MAX_FILE)),
                        Job::Barrier(id) => (id, Ok(None)),
                        Job::Stop => break,
                    };
                    let _ = done.send(Completion { id, result });
                }
            })
            .expect("cannot start save writer");
        Self {
            tx,
            rx,
            thread: Some(thread),
            next: 0,
        }
    }
    fn submit(&mut self, f: impl FnOnce(u64) -> Job) -> Result<u64, String> {
        self.next += 1;
        self.tx
            .try_send(f(self.next))
            .map_err(|e| format!("Save queue unavailable: {e}"))?;
        Ok(self.next)
    }
    pub fn write(&mut self, p: PathBuf, d: Vec<u8>) -> Result<u64, String> {
        self.submit(|id| Job::Write(id, p, d))
    }
    pub fn write_capture(
        &mut self,
        p: PathBuf,
        capture: super::emulator::SaveCapture,
    ) -> Result<u64, String> {
        self.submit(|id| Job::Capture(id, p, capture))
    }
    pub fn read(&mut self, p: PathBuf) -> Result<u64, String> {
        self.submit(|id| Job::Read(id, p))
    }
    pub fn barrier(&mut self) -> Result<u64, String> {
        self.submit(Job::Barrier)
    }
    pub fn poll(&self) -> Vec<Completion> {
        self.rx.try_iter().collect()
    }
    pub fn finish(&mut self) {
        let _ = self.tx.send(Job::Stop);
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}
impl Default for Writer {
    fn default() -> Self {
        Self::new()
    }
}
impl Drop for Writer {
    fn drop(&mut self) {
        self.finish();
    }
}
