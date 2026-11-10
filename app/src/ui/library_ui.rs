// SPDX-License-Identifier: GPL-3.0-only
// SPDX-FileCopyrightText: 2026 Noctyros
// Applies to original RetroHatch contributions; third-party provenance is retained below and in THIRD_PARTY_LICENSES.md.
//! ROM-library presentation; hashing and filesystem mutations run on a separate host thread.
use super::*;
use retrohatch::{
    core::state::hex,
    host::{
        fingerprints::Assignment,
        library::{self, Action, Entry, Event, Key, Prepared, Service},
    },
};
#[derive(Clone)]
struct Edit {
    entry: Entry,
    profile: DeviceProfile,
    keep: bool,
}
struct Choice {
    prepared: Prepared,
    suggestions: Vec<Assignment>,
    selected: Option<DeviceProfile>,
}
#[derive(Default)]
pub(super) struct LibraryUi {
    service: Option<Service>,
    pub entries: Vec<Entry>,
    jobs: u32,
    pub manage: bool,
    choice: Option<Choice>,
    edit: Option<Edit>,
    remove: Option<(Entry, bool)>,
    restore: bool,
    pub repair: Option<Key>,
    pub failed: bool,
    pub close_after: bool,
}
impl LibraryUi {
    pub fn busy(&self) -> bool {
        self.jobs > 0
    }
    pub fn available(&self) -> bool {
        self.service.is_some()
    }
    pub fn blocked(&self) -> bool {
        self.manage
            || self.choice.is_some()
            || self.edit.is_some()
            || self.remove.is_some()
            || self.restore
    }
}
impl RetroHatchApp {
    pub(super) fn start_library(&mut self, root: PathBuf) {
        self.library.service = Some(Service::new(root));
        self.library_send(Action::Refresh);
    }
    fn library_send(&mut self, action: Action) {
        self.release_inputs();
        let result = self
            .library
            .service
            .as_ref()
            .ok_or_else(|| {
                "The private ROM library is unavailable. Check the user-data directory.".to_string()
            })
            .and_then(|s| s.send(action));
        match result {
            Ok(()) => self.library.jobs += 1,
            Err(e) => {
                self.local_error = Some(e);
                self.library.failed = true;
            }
        }
    }

    pub(super) fn library_import(&mut self, path: PathBuf, preferred: Option<DeviceProfile>) {
        if let Some(parent) = path.parent() {
            self.folder = parent.to_string_lossy().into_owned();
            self.settings.rom_folder = Some(parent.to_path_buf());
            self.dirty = Some(Instant::now());
        }
        self.library_send(Action::Import(path, preferred));
    }
    pub(super) fn library_startup(&mut self, path: PathBuf, profile: DeviceProfile) {
        let key = path
            .file_stem()
            .and_then(|s| s.to_str())
            .filter(|s| library::hash_bytes(s).is_ok())
            .map(|s| Key {
                hash: s.into(),
                profile,
            });
        let internal = self.data_dir.as_ref().is_some_and(|root| {
            path == root
                .join(library::DIRECTORY)
                .join("roms")
                .join(path.file_name().unwrap_or_default())
        });
        if internal && key.is_some() {
            self.library_send(Action::Start(key.unwrap()));
        } else {
            self.library_send(Action::Migrate(path, profile));
        }
    }
    pub(super) fn library_repair(&mut self, path: PathBuf) {
        if let Some(key) = self.library.repair.clone() {
            self.library_send(Action::Repair(path, key));
        } else {
            self.library_import(path, None)
        }
    }
    pub(super) fn poll_library(&mut self) {
        while let Some(event) = self.library.service.as_ref().and_then(Service::poll) {
            self.library.jobs = self.library.jobs.saturating_sub(1);
            match event {
                Event::View(entries) => self.library.entries = entries,
                Event::Choice {
                    prepared,
                    suggestions,
                    entries,
                } => {
                    self.library.entries = entries;
                    self.library.choice = Some(Choice {
                        prepared,
                        suggestions,
                        selected: None,
                    });
                }
                Event::Ready {
                    entry,
                    path,
                    entries,
                } => {
                    self.library.failed = false;
                    self.library.repair = None;
                    self.library.choice = None;
                    self.library.entries = entries;
                    self.selected_profile = entry.key.profile;
                    self.release_inputs();
                    let loads = self.controller.snapshot().loads;
                    self.waiting_load = Some((path.clone(), entry.key.profile, loads));
                    match library::hash_bytes(&entry.key.hash) {
                        Ok(hash) => self.controller.send(Command::LoadVerified(
                            path,
                            entry.key.profile,
                            hash,
                        )),
                        Err(e) => self.local_error = Some(e),
                    }
                }
                Event::Changed { entries, message } => {
                    if self.settings.last_rom.as_ref().is_some_and(|path| {
                        path.components()
                            .any(|c| c.as_os_str() == library::DIRECTORY)
                            && path
                                .file_stem()
                                .and_then(|h| h.to_str())
                                .is_some_and(|hash| {
                                    !entries.iter().any(|e| {
                                        e.key.hash == hash
                                            && e.key.profile == self.settings.last_profile
                                    })
                                })
                    }) {
                        self.settings.last_rom = None;
                        self.dirty = Some(Instant::now());
                    }
                    self.library.failed = false;
                    self.library.repair = None;
                    self.library.entries = entries;
                    self.last_message = Some(message);
                    self.message_until = Instant::now() + Duration::from_secs(4);
                }
                Event::Failed {
                    message,
                    repair,
                    entries,
                } => {
                    if let Some(entries) = entries {
                        self.library.entries = entries;
                    }
                    self.library.failed = true;
                    self.library.repair = repair;
                    self.local_error = Some(message);
                }
            }
        }
    }
    pub(super) fn library_menu(&mut self, ui: &mut egui::Ui, s: &Snapshot) {
        ui.menu_button("My Tamagotchis", |ui| {
            if self.library.entries.is_empty() {
                ui.weak(if self.library.busy() {
                    "Loading library..."
                } else {
                    "Import a ROM from File to get started."
                });
            }
            let busy = self.library.busy()
                || s.loading
                || s.saving
                || self.exit_pending
                || self.picker.is_some();
            for entry in self.library.entries.clone() {
                let current = s.loaded
                    && s.profile == entry.key.profile
                    && s.rom_hash.is_some_and(|h| hex(&h) == entry.key.hash);
                let response = ui
                    .push_id((&entry.key.hash, entry.key.profile.id()), |ui| {
                        ui.add_enabled(
                            !busy,
                            egui::Button::new(entry.key.profile.name()).selected(current),
                        )
                    })
                    .inner;
                if response.clicked() {
                    self.library.repair = Some(entry.key.clone());
                    self.library_send(Action::Start(entry.key));
                    ui.close();
                }
            }
            ui.separator();
            if ui.button("Manage ROMs...").clicked() {
                self.release_inputs();
                self.library.manage = true;
                if !self.library.busy() {
                    self.library_send(Action::Refresh);
                }
                ui.close();
            }
        });
    }
    pub(super) fn library_dialogs(&mut self, ctx: &egui::Context, s: &Snapshot) {
        if let Some(mut choice) = self.library.choice.take() {
            let mut cancel = false;
            let mut confirm = false;
            let mut open = true;
            egui::Window::new("Identify imported ROM").open(&mut open).collapsible(false).resizable(false).default_width(340.).max_width((ctx.input(|i|i.content_rect().width())-32.).max(280.)).max_height((ctx.input(|i|i.content_rect().height())-72.).max(200.)).vscroll(true).anchor(egui::Align2::CENTER_CENTER,egui::Vec2::ZERO).show(ctx,|ui|{
    ui.label(&choice.prepared.source_name);ui.label("This content has no unique model association. Select its hardware once.");
    ui.weak(format!("{} bytes",choice.prepared.bytes.len()));
    egui::ComboBox::from_label("Device profile").selected_text(choice.selected.map(|p|p.name()).unwrap_or("Select model...")).show_ui(ui,|ui|{for &p in &choice.prepared.candidates{ui.selectable_value(&mut choice.selected,Some(p),p.name());}});
    ui.weak("Filenames and size do not prove identity. The chosen association is stored locally by content hash.");
    ui.horizontal(|ui|{confirm=ui.add_enabled(choice.selected.is_some(),egui::Button::new("Import and Start")).clicked();cancel=ui.button("Cancel").clicked();});
   });
            if confirm {
                let profile = choice.selected.unwrap();
                let a = retrohatch::host::fingerprints::for_profile(&choice.suggestions, profile);
                self.library_send(Action::Confirm(choice.prepared, a));
            } else if !cancel && open {
                self.library.choice = Some(choice)
            }
        }
        if self.library.manage {
            let mut open = true;
            let busy = self.library.busy() || self.picker.is_some();
            egui::Window::new("Manage ROMs").open(&mut open).default_width(410.).max_width((ctx.input(|i|i.content_rect().width())-32.).max(280.)).max_height((ctx.input(|i|i.content_rect().height())-72.).max(200.)).vscroll(true).show(ctx,|ui|{
    ui.weak("Private copies only. Removing or correcting entries never deletes saved games or original ROMs.");
    for entry in self.library.entries.clone(){ui.separator();ui.strong(entry.key.profile.name());let age=std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).ok().and_then(|now|now.as_secs().checked_sub(entry.imported));
    let imported=match age {Some(s) if s<86400=>"Imported today".into(),Some(s)=>format!("Imported {} days ago",s/86400),None=>"Import date unavailable".into()};
    ui.small(format!("{} bytes | {imported}",entry.size));if self.settings.diagnostics { ui.small(format!("Internal file: {}",entry.filename)); }ui.small(format!("Source filename: {}",entry.source_name));if self.settings.diagnostics { ui.small(&entry.evidence); }
     if let Some(error)=&entry.problem{ui.colored_label(egui::Color32::RED,error);}
     ui.add_enabled_ui(!busy&&!s.loading&&!s.saving,|ui|{ui.horizontal_wrapped(|ui|{
      if ui.button("Start").clicked(){self.library.repair=Some(entry.key.clone());self.library_send(Action::Start(entry.key.clone()));}
      if ui.button("Edit model").clicked(){self.library.edit=Some(Edit{profile:entry.key.profile,entry:entry.clone(),keep:false});}
      if ui.button("Add model association").clicked(){self.library.edit=Some(Edit{profile:entry.key.profile,entry:entry.clone(),keep:true});}
      if ui.button("Repair...").clicked(){self.library.repair=Some(entry.key.clone());self.picker(PickerKind::Repair,ctx);}
      if ui.button("Remove...").clicked(){self.library.remove=Some((entry.clone(),false));}
     });});
    }
    ui.separator();ui.add_enabled_ui(!busy,|ui|{if ui.button("Restore Library Index Backup...").clicked(){self.library.restore=true;}});
   });
            self.library.manage = open;
        }
        if let Some(mut edit) = self.library.edit.take() {
            let mut confirm = false;
            let mut cancel = false;
            egui::Window::new(if edit.keep{"Add model association"}else{"Edit model association"}).collapsible(false).resizable(false).default_width(340.).max_width((ctx.input(|i|i.content_rect().width())-32.).max(280.)).max_height((ctx.input(|i|i.content_rect().height())-72.).max(200.)).vscroll(true).show(ctx,|ui|{
    ui.label(edit.entry.key.profile.name());egui::ComboBox::from_label("Model").selected_text(edit.profile.name()).show_ui(ui,|ui|{for p in DeviceProfile::ALL{if p.rom_bytes()==edit.entry.size{ui.selectable_value(&mut edit.profile,p,p.name());}}});
    ui.weak("Model changes apply on the next start. Previous-profile saves stay intact and are not converted. Add an association to keep both models for shared ROM content.");
    ui.horizontal(|ui|{confirm=ui.button("Save association").clicked();cancel=ui.button("Cancel").clicked();});
   });
            if confirm {
                self.library_send(Action::Update(
                    edit.entry.key,
                    Assignment {
                        profile: edit.profile,
                        variant: edit.entry.variant.clone(),
                        evidence: edit.entry.evidence.clone(),
                    },
                    edit.keep,
                ));
            } else if !cancel {
                self.library.edit = Some(edit)
            }
        }
        if let Some((entry, mut delete)) = self.library.remove.take() {
            let mut confirm = false;
            let mut cancel = false;
            let shared = self
                .library
                .entries
                .iter()
                .any(|e| e.key.hash == entry.key.hash && e.key != entry.key);
            egui::Window::new("Remove library entry?")
                .collapsible(false)
                .resizable(false)
                .default_width(340.)
                .max_width((ctx.input(|i| i.content_rect().width()) - 32.).max(280.))
                .max_height((ctx.input(|i| i.content_rect().height()) - 72.).max(200.))
                .vscroll(true)
                .show(ctx, |ui| {
                    ui.label(entry.key.profile.name());
                    ui.label("Original files and all saved games will be kept.");
                    ui.weak(
                        "Your confirmed model choice is remembered if this ROM is imported again.",
                    );
                    ui.add_enabled(
                        !shared,
                        egui::Checkbox::new(&mut delete, "Also delete the internal ROM copy"),
                    );
                    if shared {
                        delete = false;
                        ui.weak("Another association still uses this ROM copy.");
                    }
                    ui.horizontal(|ui| {
                        confirm = ui.button("Remove entry").clicked();
                        cancel = ui.button("Cancel").clicked();
                    });
                });
            if confirm {
                self.library_send(Action::Remove(entry.key, delete));
            } else if !cancel {
                self.library.remove = Some((entry, delete))
            }
        }
        if self.library.restore {
            let mut yes = false;
            let mut no = false;
            egui::Window::new("Restore library index backup?").collapsible(false).resizable(false).default_width(340.).max_width((ctx.input(|i|i.content_rect().width())-32.).max(280.)).max_height((ctx.input(|i|i.content_rect().height())-72.).max(200.)).vscroll(true).show(ctx,|ui|{ui.label("The current index will be preserved separately. The previous index may omit recent entries; ROM copies and saves are kept.");ui.horizontal(|ui|{yes=ui.button("Restore backup").clicked();no=ui.button("Cancel").clicked();});});
            if yes {
                self.library.restore = false;
                self.library_send(Action::RestoreBackup);
            } else if no {
                self.library.restore = false;
            }
        }
    }
}
