// SPDX-License-Identifier: GPL-3.0-only
// SPDX-FileCopyrightText: 2026 Noctyros
//! Bounded capture handoff and a wall-clock paced native encoder; never drives emulation.
use super::screenshots::{self, Image, Kind};
use std::{
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
        mpsc,
    },
    time::{Duration, Instant, SystemTime},
};
#[cfg(windows)]
#[path = "recording_windows.rs"]
mod native;
#[derive(Debug)]
pub struct Completed {
    pub path: PathBuf,
    pub frames: u64,
    pub repeated: u64,
    pub dropped: u64,
    pub elapsed: Duration,
}
pub struct Session {
    pub kind: Kind,
    pub started: Instant,
    pub stopping: bool,
    tx: mpsc::SyncSender<Image>,
    done: mpsc::Receiver<Result<Completed, String>>,
    end_ns: Arc<AtomicU64>,
    pub dropped: Arc<AtomicU64>,
}
impl Session {
    pub fn start(root: &Path, model: &str, kind: Kind) -> Result<Self, String> {
        #[cfg(not(windows))]
        {
            let _ = (root, model, kind);
            return Err("Native MP4 recording requires Windows Media Foundation".into());
        }
        #[cfg(windows)]
        {
            screenshots::prepare(root)?;
            let model: String = model
                .chars()
                .filter(|c| c.is_ascii_alphanumeric() || *c == '-')
                .take(80)
                .collect();
            if model.is_empty() {
                return Err("Invalid recording model name".into());
            }
            let stamp = screenshots::timestamp(SystemTime::now())?;
            let folder = root.join("Recordings").join(kind.folder());
            let mut reserved = None;
            for n in 0..10000 {
                let path = folder.join(format!("{model}_{stamp}_{n:04}.mp4"));
                match std::fs::OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(&path)
                {
                    Ok(_) => {
                        reserved = Some(path);
                        break;
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
                    Err(e) => return Err(format!("Cannot create recording: {e}")),
                }
            }
            let path = reserved.ok_or("Cannot choose a unique recording filename")?;
            let (tx, rx) = mpsc::sync_channel(2);
            let (done_tx, done) = mpsc::channel();
            let end_ns = Arc::new(AtomicU64::new(u64::MAX));
            let dropped = Arc::new(AtomicU64::new(0));
            let started = Instant::now();
            let end = end_ns.clone();
            let losses = dropped.clone();
            std::thread::spawn(move || {
                let result = run(&path, rx, started, end, losses)
                    .map_err(|e| format!("{e}\nIncomplete recording: {}", path.display()));
                let _ = done_tx.send(result);
            });
            Ok(Self {
                kind,
                started,
                stopping: false,
                tx,
                done,
                end_ns,
                dropped,
            })
        }
    }
    pub fn submit(&self, image: Image) {
        if self.stopping {
            return;
        }
        if self.tx.try_send(image).is_err() {
            self.dropped.fetch_add(1, Ordering::Relaxed);
        }
    }
    pub fn stop(&mut self) {
        if !self.stopping {
            self.end_ns.store(
                self.started.elapsed().as_nanos().min(u64::MAX as u128 - 1) as u64,
                Ordering::Release,
            );
            self.stopping = true;
        }
    }
    pub fn poll(&self) -> Option<Result<Completed, String>> {
        match self.done.try_recv() {
            Ok(result) => Some(result),
            Err(mpsc::TryRecvError::Empty) => None,
            Err(mpsc::TryRecvError::Disconnected) => {
                Some(Err("Recording worker stopped unexpectedly".into()))
            }
        }
    }
}
impl Drop for Session {
    fn drop(&mut self) {
        self.stop();
    }
}
#[cfg(windows)]
fn run(
    path: &Path,
    rx: mpsc::Receiver<Image>,
    started: Instant,
    end: Arc<AtomicU64>,
    dropped: Arc<AtomicU64>,
) -> Result<Completed, String> {
    let first = rx
        .recv_timeout(Duration::from_secs(5))
        .map_err(|_| "No recording frame arrived. Keep the emulator window visible.".to_string())?;
    let factor = (1920.0 / first.width.max(first.height) as f64).min(1.0);
    let w = ((first.width as f64 * factor).ceil() as usize + 1) & !1;
    let h = ((first.height as f64 * factor).ceil() as usize + 1) & !1;
    let mut encoder = native::Encoder::new(path, w, h)?;
    let mut pixels = native::canvas(&first, w, h);
    let mut index = 0;
    let mut repeated = 0;
    let mut fresh = true;
    loop {
        let stop = end.load(Ordering::Acquire);
        let target = if stop == u64::MAX {
            u64::MAX
        } else {
            ((u128::from(stop) * 30).div_ceil(1_000_000_000)) as u64
        };
        if index >= target {
            break;
        }
        let deadline = started + Duration::from_nanos(index * 1_000_000_000 / 30);
        let wait = deadline.saturating_duration_since(Instant::now());
        if !wait.is_zero() {
            match rx.recv_timeout(wait) {
                Ok(frame) => {
                    pixels = native::canvas(&frame, w, h);
                    fresh = true;
                    continue;
                }
                Err(mpsc::RecvTimeoutError::Disconnected) if stop == u64::MAX => break,
                _ => {}
            }
        }
        while let Ok(frame) = rx.try_recv() {
            pixels = native::canvas(&frame, w, h);
            fresh = true;
        }
        if !fresh {
            repeated += 1;
        }
        encoder.write(&pixels, index)?;
        index += 1;
        fresh = false;
    }
    encoder.finish()?;
    Ok(Completed {
        path: path.into(),
        frames: index,
        repeated,
        dropped: dropped.load(Ordering::Relaxed),
        elapsed: Duration::from_secs_f64(index as f64 / 30.0),
    })
}
