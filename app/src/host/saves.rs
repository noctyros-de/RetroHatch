// SPDX-License-Identifier: GPL-3.0-only
// SPDX-FileCopyrightText: 2026 Noctyros
// Applies to original RetroHatch contributions; third-party provenance is retained below and in THIRD_PARTY_LICENSES.md.
//! Per-ROM storage coordination. Capture on the owning worker; disk I/O stays off it.
use super::emulator::Emulator;
use super::{runner::Snapshot, storage::Writer};
use std::{
    collections::BTreeMap,
    path::PathBuf,
    time::{Duration, Instant},
};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SaveTarget {
    Autosave,
    Slot(u8),
}
impl SaveTarget {
    pub fn filename(self) -> Result<String, String> {
        match self {
            Self::Autosave => Ok("autosave.rhstate".into()),
            Self::Slot(n) if (1..=3).contains(&n) => Ok(format!("slot{n}.rhstate")),
            _ => Err("Only slots 1, 2 and 3 exist".into()),
        }
    }
}
#[derive(Clone, Debug)]
pub struct SavePolicy {
    pub autosave_minutes: u32,
    pub save_on_exit: bool,
    pub volume: u8,
}
impl Default for SavePolicy {
    fn default() -> Self {
        Self {
            autosave_minutes: 1,
            save_on_exit: true,
            volume: 100,
        }
    }
}
enum Task {
    Write {
        identity: String,
        target: Option<SaveTarget>,
        label: String,
    },
    Read {
        identity: String,
        automatic: bool,
        generation: u64,
    },
    Restore {
        identity: String,
        prepared: Emulator,
        generation: u64,
    },
    Exit,
}
pub struct Saves {
    pub root: PathBuf,
    pub policy: SavePolicy,
    writer: Writer,
    tasks: BTreeMap<u64, Task>,
    next: Instant,
    generation: u64,
    resume_blocked: bool,
}
impl Saves {
    pub fn new(root: PathBuf, policy: SavePolicy) -> Self {
        Self {
            root,
            policy,
            writer: Writer::new(),
            tasks: BTreeMap::new(),
            next: Instant::now(),
            generation: 0,
            resume_blocked: false,
        }
    }
    pub fn directory(&self, d: &Emulator) -> PathBuf {
        self.root.join("saves").join(d.identity())
    }
    pub fn read_path(&self, d: &Emulator, filename: &str) -> PathBuf {
        let current = self.directory(d).join(filename);
        if !current.exists()
            && let Some(legacy) = d.legacy_identity()
        {
            let old = self.root.join("saves").join(legacy).join(filename);
            if old.is_file() {
                return old;
            }
        }
        current
    }
    pub fn configure(&mut self, p: SavePolicy) {
        let reset = p.autosave_minutes != self.policy.autosave_minutes;
        self.policy = p;
        if reset {
            self.reset_timer();
        }
    }
    pub fn reset_timer(&mut self) {
        self.next =
            Instant::now() + Duration::from_secs(u64::from(self.policy.autosave_minutes) * 60);
    }
    pub fn busy(&self) -> bool {
        !self.tasks.is_empty()
    }
    pub fn save(&mut self, d: &Emulator, target: SaveTarget) -> Result<(), String> {
        if self.resume_blocked {
            return Err("Saved progress was not restored. Resume or Reset explicitly before saving a fresh device.".into());
        }
        let data = d.capture_save()?;
        let path = self.directory(d).join(target.filename()?);
        let id = self.writer.write_capture(path, data)?;
        self.tasks.insert(
            id,
            Task::Write {
                identity: d.identity(),
                target: Some(target),
                label: match target {
                    SaveTarget::Autosave => "Saved current progress".into(),
                    SaveTarget::Slot(n) => format!("Saved Slot {n}"),
                },
            },
        );
        self.reset_timer();
        Ok(())
    }
    pub fn export(&mut self, d: &Emulator, path: PathBuf) -> Result<(), String> {
        let id = self.writer.write_capture(path, d.capture_save()?)?;
        self.tasks.insert(
            id,
            Task::Write {
                identity: d.identity(),
                target: None,
                label: "Save file exported".into(),
            },
        );
        Ok(())
    }
    pub fn load(&mut self, d: &Emulator, path: PathBuf, automatic: bool) -> Result<(), String> {
        let id = self.writer.read(path)?;
        self.tasks.insert(
            id,
            Task::Read {
                identity: d.identity(),
                automatic,
                generation: self.generation,
            },
        );
        Ok(())
    }
    pub fn start(&mut self, d: &Emulator, s: &mut Snapshot) -> Result<(), String> {
        self.generation += 1;
        self.resume_blocked = false;
        self.reset_timer();
        s.slots = std::array::from_fn(|i| {
            self.read_path(d, &format!("slot{}.rhstate", i + 1))
                .is_file()
        });
        let result = self.load(d, self.read_path(d, "autosave.rhstate"), true);
        if result.is_err() {
            self.resume_blocked = true;
        }
        result
    }
    pub fn exit(&mut self, d: Option<&Emulator>) -> Result<(), String> {
        if self.policy.save_on_exit && self.resume_blocked {
            return Err("Saved progress was not restored. Import a valid save, or explicitly Resume/Reset to start fresh. The existing autosave has been kept.".into());
        }
        let id = if self.policy.save_on_exit {
            if let Some(d) = d {
                self.writer.write_capture(
                    self.directory(d).join("autosave.rhstate"),
                    d.capture_save()?,
                )?
            } else {
                self.writer.barrier()?
            }
        } else {
            self.writer.barrier()?
        };
        self.tasks.insert(id, Task::Exit);
        Ok(())
    }
    pub fn automatic_due(&mut self, d: &Emulator, running: bool) -> Result<(), String> {
        if running && self.policy.autosave_minutes != 0 && Instant::now() >= self.next {
            self.save(d, SaveTarget::Autosave)?;
        }
        Ok(())
    }
    /// Returns true when a newly prepared device was applied, or automatic boot was released.
    pub fn poll(
        &mut self,
        device: &mut Option<Emulator>,
        state: &mut Snapshot,
        running: &mut bool,
    ) -> bool {
        let mut rebased = false;
        for done in self.writer.poll() {
            let Some(task) = self.tasks.remove(&done.id) else {
                continue;
            };
            if matches!(&task,Task::Read{generation,..}|Task::Restore{generation,..} if *generation!=self.generation)
            {
                continue;
            }
            if let Err(e) = &done.result {
                state.save_error = Some(e.clone());
                if matches!(
                    task,
                    Task::Read {
                        automatic: true,
                        ..
                    }
                ) {
                    state.loading = false;
                    self.resume_blocked = true;
                    *running = false;
                }
                continue;
            }
            match task {
                Task::Write {
                    identity,
                    target,
                    label,
                } => {
                    if device.as_ref().is_some_and(|d| d.identity() == identity) {
                        if let Some(SaveTarget::Slot(n)) = target {
                            state.slots[n as usize - 1] = true;
                        }
                        state.save_message = Some(label);
                        state.save_count += 1;
                    }
                }
                Task::Exit => {
                    state.exit_ready = true;
                    state.save_message = Some("Ready to close.".into());
                }
                Task::Read {
                    identity,
                    automatic,
                    generation,
                } => {
                    let Some(d) = device.as_ref().filter(|d| d.identity() == identity) else {
                        continue;
                    };
                    match done.result.unwrap() {
                        None if automatic => {
                            state.loading = false;
                            *running = true;
                            rebased = true;
                        }
                        None => {
                            state.save_error =
                                Some("This save slot is empty or the file no longer exists".into())
                        }
                        Some(bytes) => match d.restored(&bytes) {
                            Err(e) => {
                                state.save_error = Some(e);
                                if automatic {
                                    state.loading = false;
                                    self.resume_blocked = true;
                                    *running = false;
                                }
                            }
                            Ok(prepared) => {
                                if automatic {
                                    *device = Some(prepared);
                                    *running = true;
                                    state.loading = false;
                                    state.restores += 1;
                                    state.save_message = Some("Resumed saved progress".into());
                                    rebased = true;
                                } else {
                                    match d.capture_save().and_then(|data| {
                                        self.writer.write_capture(
                                            self.directory(d).join("recovery.rhstate"),
                                            data,
                                        )
                                    }) {
                                        Ok(id) => {
                                            self.tasks.insert(
                                                id,
                                                Task::Restore {
                                                    identity,
                                                    prepared,
                                                    generation,
                                                },
                                            );
                                        }
                                        Err(e) => state.save_error = Some(e),
                                    }
                                }
                            }
                        },
                    }
                }
                Task::Restore {
                    identity, prepared, ..
                } => {
                    if device.as_ref().is_some_and(|d| d.identity() == identity) {
                        *device = Some(prepared);
                        state.restores += 1;
                        self.resume_blocked = false;
                        state.save_error = None;
                        state.save_message = Some("Saved state restored".into());
                        rebased = true;
                    }
                }
            }
        }
        state.saving = self.busy();
        rebased
    }
    pub fn can_capture(&self) -> bool {
        !self.resume_blocked
    }
    pub fn allow_fresh_session(&mut self) {
        self.resume_blocked = false;
    }
    pub fn cancel_restore(&mut self) {
        self.generation += 1;
        self.resume_blocked = false;
    }
    pub fn finish(&mut self) {
        self.writer.finish();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn every_autosave_interval_and_off_use_monotonic_deadlines_independent_of_volume() {
        for minutes in crate::host::settings::AUTOSAVES {
            let start = Instant::now();
            let mut s = Saves::new(
                std::env::temp_dir(),
                SavePolicy {
                    autosave_minutes: minutes,
                    save_on_exit: true,
                    volume: 100,
                },
            );
            s.reset_timer();
            let elapsed = s.next.duration_since(start).as_secs_f64();
            assert!(
                elapsed >= f64::from(minutes) * 60.0 && elapsed < f64::from(minutes) * 60.0 + 1.0
            );
            let deadline = s.next;
            s.configure(SavePolicy {
                volume: 20,
                ..s.policy.clone()
            });
            assert_eq!(s.next, deadline);
            assert_eq!(s.policy.autosave_minutes, minutes);
        }
    }
}
