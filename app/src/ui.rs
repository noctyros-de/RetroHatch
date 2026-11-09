//! Minimal themed device view; all machine state and disk I/O belong to the host worker.
use crate::lcd_geometry::{ICON_BITS, ICON_SPANS, LCD_SIZE, PIXELS};
use eframe::egui;
use retrohatch::core::{
    device::{Button, DisplaySnapshot},
    mcu::MASTER_HZ,
    profile::DeviceProfile,
};
use retrohatch::host::{
    platform, roms,
    runner::{Command, Controller, Snapshot},
    saves::{SavePolicy, SaveTarget},
    settings::{AUTOSAVES, Settings, Theme},
    storage,
};
use std::{
    path::PathBuf,
    sync::mpsc,
    time::{Duration, Instant},
};
#[derive(Clone, Copy)]
enum PickerKind {
    Rom,
    Export,
    Import,
}
type PickerResult = (PickerKind, Result<Option<PathBuf>, String>);
pub struct RetroHatchApp {
    controller: Controller,
    settings: Settings,
    data_dir: Option<PathBuf>,
    dirty: Option<Instant>,
    selected_profile: DeviceProfile,
    pressed: [bool; 4],
    click_until: [Option<Instant>; 4],
    picker: Option<mpsc::Receiver<PickerResult>>,
    pending_rom: Option<PathBuf>,
    folder: String,
    local_error: Option<String>,
    reset_confirm: bool,
    exit_pending: bool,
    allow_close: bool,
    exit_was_running: bool,
    applied_theme: Option<Theme>,
    title: String,
    last_restore: u64,
    waiting_load: Option<(PathBuf, DeviceProfile, u64)>,
    last_message: Option<String>,
    message_until: Instant,
    message_serial: u64,
}
impl RetroHatchApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let mut app = Self::create(true);
        let mut args = std::env::args_os().skip(1);
        if let Some(first) = args.next() {
            if first == "--profile" {
                if let Some(profile) = args
                    .next()
                    .and_then(|s| s.to_str().and_then(DeviceProfile::parse))
                {
                    app.selected_profile = profile;
                    if let Some(path) = args.next() {
                        app.load_rom(path.into());
                    }
                } else {
                    app.local_error = Some(
                        "Use --profile p1, genjintch or angel followed by a local ROM path.".into(),
                    );
                }
            } else {
                app.pending_rom = Some(first.into());
            }
        } else if let Some(path) = app.settings.last_rom.clone() {
            app.selected_profile = app.settings.last_profile;
            app.load_rom(path);
        }
        app.apply_theme(&cc.egui_ctx);
        app
    }
    fn create(persistent: bool) -> Self {
        let mut error = None;
        let data_dir = if persistent {
            match storage::user_dir() {
                Ok(p) => Some(p),
                Err(e) => {
                    error = Some(e);
                    None
                }
            }
        } else {
            None
        };
        let settings = data_dir
            .as_ref()
            .map(|p| {
                Settings::load(p).unwrap_or_else(|e| {
                    error = Some(format!("Settings could not be restored: {e}"));
                    Settings::default()
                })
            })
            .unwrap_or_default();
        let policy = SavePolicy {
            autosave_minutes: settings.autosave_minutes,
            save_on_exit: settings.save_on_exit,
            volume: settings.volume,
        };
        let controller = if let Some(root) = &data_dir {
            Controller::with_storage(!settings.muted, root.clone(), policy)
        } else {
            Controller::new(persistent && !settings.muted)
        };
        let folder = settings
            .rom_folder
            .clone()
            .unwrap_or_else(roms::default_folder)
            .to_string_lossy()
            .into_owned();
        Self {
            controller,
            selected_profile: settings.last_profile,
            settings,
            data_dir,
            dirty: if persistent {
                Some(Instant::now())
            } else {
                None
            },
            pressed: [false; 4],
            click_until: [None; 4],
            picker: None,
            pending_rom: None,
            folder,
            local_error: error,
            reset_confirm: false,
            exit_pending: false,
            allow_close: false,
            exit_was_running: false,
            applied_theme: None,
            title: String::new(),
            last_restore: 0,
            waiting_load: None,
            last_message: None,
            message_until: Instant::now(),
            message_serial: 0,
        }
    }
    fn load_rom(&mut self, path: PathBuf) {
        self.release_inputs();
        let loads = self.controller.snapshot().loads;
        self.waiting_load = Some((path.clone(), self.selected_profile, loads));
        self.controller
            .send(Command::LoadProfile(path, self.selected_profile));
    }
    fn release_inputs(&mut self) {
        self.pressed = [false; 4];
        self.click_until = [None; 4];
        self.controller.send(Command::ReleaseInputs);
    }
    fn changed(&mut self, before: &Settings) {
        self.dirty = Some(Instant::now());
        if before.muted != self.settings.muted {
            self.controller.send(Command::Audio(!self.settings.muted));
        }
        if before.volume != self.settings.volume
            || before.autosave_minutes != self.settings.autosave_minutes
            || before.save_on_exit != self.settings.save_on_exit
        {
            self.controller.send(Command::SavePolicy(SavePolicy {
                autosave_minutes: self.settings.autosave_minutes,
                save_on_exit: self.settings.save_on_exit,
                volume: self.settings.volume,
            }));
        }
    }
    fn persist(&mut self) -> bool {
        if let Some(root) = &self.data_dir {
            match self.settings.store(root) {
                Ok(()) => {
                    self.dirty = None;
                    true
                }
                Err(e) => {
                    self.local_error = Some(e);
                    self.dirty = None;
                    false
                }
            }
        } else {
            true
        }
    }
    fn begin_exit(&mut self, s: &Snapshot) {
        if self.exit_pending {
            return;
        }
        self.exit_pending = true;
        self.exit_was_running = s.running;
        self.release_inputs();
        if self.persist() {
            self.controller.send(Command::PrepareExit);
        }
    }
    fn picker(&mut self, kind: PickerKind, ctx: &egui::Context) {
        if self.picker.is_some() {
            return;
        }
        self.release_inputs();
        let folder = if matches!(kind, PickerKind::Rom) {
            PathBuf::from(&self.folder)
        } else {
            self.data_dir
                .clone()
                .unwrap_or_else(|| PathBuf::from(&self.folder))
        };
        let (tx, rx) = mpsc::channel();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let result = match kind {
                PickerKind::Rom => platform::pick_rom(&folder),
                PickerKind::Export => platform::pick_save(&folder, true),
                PickerKind::Import => platform::pick_save(&folder, false),
            };
            let _ = tx.send((kind, result));
            ctx.request_repaint();
        });
        self.picker = Some(rx);
    }
    fn apply_theme(&mut self, ctx: &egui::Context) {
        if self.applied_theme == Some(self.settings.theme) {
            return;
        }
        let (bg, accent, ink) = palette(self.settings.theme);
        let mut v = egui::Visuals::light();
        v.panel_fill = bg;
        v.window_fill = blend(bg, egui::Color32::WHITE, 0.72);
        v.override_text_color = Some(ink);
        v.selection.bg_fill = accent;
        v.selection.stroke.color = ink;
        v.widgets.inactive.bg_fill = blend(bg, egui::Color32::WHITE, 0.66);
        v.widgets.hovered.bg_fill = blend(accent, egui::Color32::WHITE, 0.60);
        v.widgets.active.bg_fill = accent;
        ctx.set_visuals(v);
        ctx.style_mut_of(egui::Theme::Light, |s| {
            s.spacing.item_spacing = egui::vec2(9.0, 7.0);
            s.spacing.button_padding = egui::vec2(10.0, 6.0);
        });
        self.applied_theme = Some(self.settings.theme);
    }
    fn menus(&mut self, ui: &mut egui::Ui, s: &Snapshot) {
        let ctx = ui.ctx().clone();
        let available =
            s.loaded && !s.loading && !s.saving && !self.exit_pending && self.data_dir.is_some();
        let before = self.settings.clone();
        egui::MenuBar::new().ui(ui, |ui| {
            ui.menu_button("File", |ui| {
                if ui
                    .add_enabled(
                        self.picker.is_none() && !self.exit_pending && !s.loading && !s.saving,
                        egui::Button::new("Open ROM"),
                    )
                    .clicked()
                {
                    self.picker(PickerKind::Rom, &ctx);
                    ui.close();
                }
                if ui.button("Open ROM Folder").clicked() {
                    if let Err(e) = platform::open_folder(std::path::Path::new(&self.folder)) {
                        self.local_error = Some(e);
                    }
                    ui.close();
                }
                ui.separator();
                if ui
                    .add_enabled(available, egui::Button::new("Save Now"))
                    .clicked()
                {
                    self.release_inputs();
                    self.controller.send(Command::Save(SaveTarget::Autosave));
                    ui.close();
                }
                ui.add_enabled_ui(available, |ui| {
                    ui.menu_button("Save to Slot", |ui| {
                        for n in 1..=3 {
                            if ui.button(format!("Slot {n}")).clicked() {
                                self.release_inputs();
                                self.controller.send(Command::Save(SaveTarget::Slot(n)));
                                ui.close();
                            }
                        }
                    });
                });
                ui.add_enabled_ui(available, |ui| {
                    ui.menu_button("Load Slot", |ui| {
                        for n in 1..=3 {
                            if ui
                                .add_enabled(s.slots[n - 1], egui::Button::new(format!("Slot {n}")))
                                .clicked()
                            {
                                self.release_inputs();
                                self.controller
                                    .send(Command::LoadSave(SaveTarget::Slot(n as u8)));
                                ui.close();
                            }
                        }
                    });
                });
                if ui
                    .add_enabled(
                        available && self.picker.is_none(),
                        egui::Button::new("Export Save File"),
                    )
                    .clicked()
                {
                    self.picker(PickerKind::Export, &ctx);
                    ui.close();
                }
                if ui
                    .add_enabled(
                        available && self.picker.is_none(),
                        egui::Button::new("Import Save File"),
                    )
                    .clicked()
                {
                    self.picker(PickerKind::Import, &ctx);
                    ui.close();
                }
                ui.separator();
                if ui.button("Exit").clicked() {
                    self.begin_exit(s);
                    ui.close();
                }
            });
            ui.menu_button("Emulation", |ui| {
                if ui
                    .add_enabled(
                        s.loaded && !s.loading && !self.exit_pending,
                        egui::Button::new(if s.running { "Pause" } else { "Resume" }),
                    )
                    .clicked()
                {
                    self.release_inputs();
                    self.controller.send(if s.running {
                        Command::Pause
                    } else {
                        Command::Run
                    });
                    ui.close();
                }
                if ui
                    .add_enabled(
                        s.loaded && !s.loading && !self.exit_pending,
                        egui::Button::new("Reset"),
                    )
                    .clicked()
                {
                    self.release_inputs();
                    self.reset_confirm = true;
                    ui.close();
                }
            });
            ui.menu_button("Settings", |ui| {
                ui.menu_button("Theme", |ui| {
                    for t in Theme::ALL {
                        if ui
                            .selectable_value(&mut self.settings.theme, t, t.name())
                            .clicked()
                        {
                            ui.close();
                        }
                    }
                });
                ui.checkbox(&mut self.settings.labels, "Show A/B/C Labels");
                ui.checkbox(&mut self.settings.muted, "Mute Sound");
                ui.add(egui::Slider::new(&mut self.settings.volume, 0..=100).text("Volume"));
                ui.menu_button("Autosave Interval", |ui| {
                    for n in AUTOSAVES {
                        let label = if n == 0 {
                            "Off".into()
                        } else {
                            format!("{n} minute{}", if n == 1 { "" } else { "s" })
                        };
                        ui.selectable_value(&mut self.settings.autosave_minutes, n, label);
                    }
                });
                ui.checkbox(&mut self.settings.save_on_exit, "Save on Exit");
                ui.separator();
                ui.checkbox(&mut self.settings.diagnostics, "Show Diagnostics");
            });
            if s.saving || s.loading {
                ui.spinner();
            } else if Instant::now() < self.message_until {
                if let Some(message) = &self.last_message {
                    ui.small(message);
                }
            }
        });
        if self.settings != before {
            self.changed(&before);
        }
    }
    fn content(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        self.apply_theme(&ctx);
        ui.set_style(ctx.global_style());
        if let Some((kind, result)) = self.picker.as_ref().and_then(|p| p.try_recv().ok()) {
            self.picker = None;
            match result {
                Ok(Some(path)) => match kind {
                    PickerKind::Rom => self.pending_rom = Some(path),
                    PickerKind::Export => self.controller.send(Command::Export(path)),
                    PickerKind::Import => self.controller.send(Command::Import(path)),
                },
                Ok(None) => {}
                Err(e) => self.local_error = Some(e),
            }
        }
        if let Some(path) =
            ctx.input(|i| i.raw.dropped_files.first().map(|f| f.path().to_path_buf()))
        {
            self.release_inputs();
            self.pending_rom = Some(path);
        }
        let s = self.controller.snapshot();
        if self.last_restore != s.restores {
            self.last_restore = s.restores;
            self.release_inputs();
        }
        if self
            .waiting_load
            .as_ref()
            .is_some_and(|(_, _, n)| s.loads > *n)
        {
            let (path, profile, _) = self.waiting_load.take().unwrap();
            self.settings.last_rom = Some(path.clone());
            self.settings.last_profile = profile;
            self.settings.rom_folder = path.parent().map(|p| p.to_path_buf());
            if let Some(folder) = &self.settings.rom_folder {
                self.folder = folder.to_string_lossy().into_owned();
            }
            self.dirty = Some(Instant::now());
        }
        if s.save_message != self.last_message || self.message_serial != s.save_count + s.restores {
            self.message_serial = s.save_count + s.restores;
            self.last_message = s.save_message.clone();
            self.message_until = Instant::now() + Duration::from_secs(4);
        }
        let title = if s.loaded {
            format!("RetroHatch - {}", s.profile.name())
        } else {
            "RetroHatch".into()
        };
        if title != self.title {
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(title.clone()));
            self.title = title;
        }
        if ctx.input(|i| i.viewport().close_requested()) && !self.allow_close {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.begin_exit(&s);
        }
        if self.exit_pending && s.exit_ready && self.local_error.is_none() && s.save_error.is_none()
        {
            self.allow_close = true;
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        if !self.exit_pending
            && self
                .dirty
                .is_some_and(|t| t.elapsed() >= Duration::from_millis(500))
        {
            self.persist();
        }
        let keys = ctx.input(|i| {
            let groups: [&[egui::Key]; 4] = [
                &[egui::Key::A, egui::Key::Num1, egui::Key::ArrowLeft],
                &[egui::Key::B, egui::Key::Num2, egui::Key::ArrowDown],
                &[egui::Key::C, egui::Key::Num3, egui::Key::ArrowRight],
                &[egui::Key::K, egui::Key::Num4],
            ];
            (
                i.focused,
                groups.map(|g| g.iter().any(|k| i.key_down(*k))),
                groups.map(|g| g.iter().any(|k| i.key_pressed(*k))),
                i.key_pressed(egui::Key::Space),
                i.key_pressed(egui::Key::F5),
            )
        });
        let blocked = self.picker.is_some()
            || self.pending_rom.is_some()
            || self.reset_confirm
            || self.exit_pending;
        let accepts = keys.0 && !ctx.egui_wants_keyboard_input() && !blocked;
        if accepts && keys.3 {
            self.release_inputs();
            self.controller.send(if s.running {
                Command::Pause
            } else {
                Command::Run
            });
        }
        if accepts && keys.4 && s.loaded {
            self.release_inputs();
            self.reset_confirm = true;
        }
        self.menus(ui, &s);
        let mut pointer = [false; 4];
        let (_, accent, ink) = palette(self.settings.theme);
        egui::CentralPanel::default()
            .frame(
                egui::Frame::NONE
                    .fill(palette(self.settings.theme).0)
                    .inner_margin(8.0),
            )
            .show(ui, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.vertical_centered(|ui| {
                        let reserve = if s.loaded && s.profile.has_knock() {
                            133.0
                        } else {
                            100.0
                        };
                        let width = (ui.available_width() - 40.0)
                            .min((ui.available_height() - reserve) * 352.0 / 329.0)
                            .clamp(220.0, 384.0);
                        let total = width * 329.0 / 352.0 + reserve;
                        ui.add_space(((ui.available_height() - total) / 2.0).max(14.0));
                        let profile = if s.loaded {
                            s.profile
                        } else {
                            self.selected_profile
                        };
                        let lcd = paint_profile_lcd(ui, &s.display, width, !s.loaded, profile);
                        let enabled = s.loaded && s.running && !s.loading && !blocked;
                        if enabled && profile.has_knock() {
                            pointer[3] = lcd.is_pointer_button_down_on();
                            if lcd.clicked() {
                                self.click_until[3] =
                                    Some(Instant::now() + Duration::from_millis(50));
                            }
                        }
                        ui.add_space(20.0);
                        ui.horizontal(|ui| {
                            let total = 3.0 * 52.0 + 2.0 * 24.0;
                            ui.add_space(((ui.available_width() - total) / 2.0).max(0.0));
                            for (index, label) in ["A", "B", "C"].into_iter().enumerate() {
                                if index > 0 {
                                    ui.add_space(15.0);
                                }
                                let response = round_button(
                                    ui,
                                    label,
                                    self.settings.labels,
                                    enabled,
                                    self.pressed[index],
                                    accent,
                                    ink,
                                );
                                pointer[index] = response.is_pointer_button_down_on() && enabled;
                                if response.clicked() && enabled {
                                    self.click_until[index] =
                                        Some(Instant::now() + Duration::from_millis(50));
                                }
                            }
                        });
                        if s.loaded && s.profile.has_knock() {
                            ui.add_space(10.0);
                            let response = ui
                            .add_enabled(
                                enabled,
                                egui::Button::new("Tap")
                                    .corner_radius(14)
                                    .min_size(egui::vec2(68.0, 26.0)),
                            )
                            .on_hover_text(
                                "Tap / knock sensor. You can also click the LCD or press 4 or K.",
                            );
                            pointer[3] |= response.is_pointer_button_down_on() && enabled;
                            if response.clicked() {
                                self.click_until[3] =
                                    Some(Instant::now() + Duration::from_millis(50));
                            }
                        }
                    });
                });
            });
        let now = Instant::now();
        for (index, button) in [Button::A, Button::B, Button::C, Button::Knock]
            .into_iter()
            .enumerate()
        {
            if accepts && keys.2[index] {
                self.click_until[index] = Some(now + Duration::from_millis(50));
            }
            let down = keys.0
                && s.loaded
                && s.running
                && !s.loading
                && !blocked
                && (index != 3 || s.profile.has_knock())
                && (pointer[index]
                    || (accepts && keys.1[index])
                    || self.click_until[index].is_some_and(|t| now < t));
            if down != self.pressed[index] {
                self.pressed[index] = down;
                self.controller.send(Command::Button(button, down));
            }
        }
        if !keys.0
            && (self.pressed.iter().any(|v| *v) || self.click_until.iter().any(Option::is_some))
        {
            self.release_inputs();
        }
        self.dialogs(&ctx, &s);
        if self.settings.diagnostics {
            diagnostics(&ctx, &s, &mut self.settings.diagnostics);
            if !self.settings.diagnostics {
                self.dirty = Some(Instant::now());
            }
        }
        ctx.request_repaint_after(Duration::from_millis(16));
    }
    fn dialogs(&mut self, ctx: &egui::Context, s: &Snapshot) {
        if let Some(path) = self.pending_rom.clone() {
            let mut load = false;
            let mut cancel = false;
            egui::Window::new("Load ROM").collapsible(false).resizable(false).anchor(egui::Align2::CENTER_CENTER,egui::Vec2::ZERO).show(ctx,|ui|{ui.label(path.file_name().unwrap_or_default().to_string_lossy());ui.label("Select the hardware for this image:");egui::ComboBox::from_label("Device profile").selected_text(self.selected_profile.name()).show_ui(ui,|ui|{for p in DeviceProfile::ALL{ui.selectable_value(&mut self.selected_profile,p,p.name());}});ui.weak(format!("{} bytes; unpacked big-endian 12-bit words.",self.selected_profile.rom_bytes()));ui.label(format!("Unidentified firmware will be assigned to {}.",self.selected_profile.name()));ui.weak("Filenames do not identify hardware. Existing saved progress will resume when compatible.");ui.horizontal(|ui|{load=ui.button("Load").clicked();cancel=ui.button("Cancel").clicked();});});
            if load {
                self.pending_rom = None;
                self.load_rom(path);
            } else if cancel {
                self.pending_rom = None;
            }
        }
        if self.reset_confirm {
            let mut yes = false;
            let mut no = false;
            egui::Window::new("Reset device?")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
                .show(ctx, |ui| {
                    ui.label("Restart the device? Unsaved progress will be lost.");
                    ui.weak("Your three manual save slots will be kept.");
                    ui.horizontal(|ui| {
                        yes = ui.button("Reset").clicked();
                        no = ui.button("Cancel").clicked();
                    });
                });
            if yes {
                self.controller.send(Command::Reset);
                self.reset_confirm = false;
            }
            if no {
                self.reset_confirm = false;
            }
        }
        let error = self
            .local_error
            .clone()
            .or(s.save_error.clone())
            .or(s.error.clone());
        if let Some(error) = error {
            let mut close = false;
            let mut retry = false;
            let mut discard = false;
            let mut cancel = false;
            egui::Window::new(if self.exit_pending {
                "Could not save before closing"
            } else {
                "RetroHatch error"
            })
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
            .show(ctx, |ui| {
                ui.label(&error);
                if self.exit_pending {
                    ui.horizontal(|ui| {
                        retry = ui.button("Retry").clicked();
                        discard = ui.button("Exit Without Saving").clicked();
                        cancel = ui.button("Cancel").clicked();
                    });
                } else {
                    close = ui.button("Close").clicked();
                }
            });
            if close {
                self.local_error = None;
                self.controller.send(Command::ClearSaveError);
            }
            if retry {
                self.local_error = None;
                self.exit_pending = false;
                self.begin_exit(s);
            }
            if discard {
                self.local_error = None;
                self.dirty = None;
                self.controller.send(Command::DiscardExit);
            }
            if cancel {
                self.exit_pending = false;
                self.local_error = None;
                self.controller.send(Command::ClearSaveError);
                if self.exit_was_running {
                    self.controller.send(Command::Run);
                }
            }
        }
    }
}
fn palette(t: Theme) -> (egui::Color32, egui::Color32, egui::Color32) {
    let (bg, accent, ink) = match t {
        Theme::Pink => ([249, 229, 237], [211, 136, 167], [78, 45, 61]),
        Theme::Blue => ([227, 239, 250], [126, 168, 209], [37, 63, 90]),
        Theme::Green => ([229, 243, 231], [129, 181, 142], [40, 73, 49]),
        Theme::Yellow => ([251, 245, 217], [212, 184, 105], [82, 66, 30]),
        Theme::Lilac => ([240, 231, 250], [173, 145, 209], [67, 47, 88]),
    };
    (
        egui::Color32::from_rgb(bg[0], bg[1], bg[2]),
        egui::Color32::from_rgb(accent[0], accent[1], accent[2]),
        egui::Color32::from_rgb(ink[0], ink[1], ink[2]),
    )
}
fn blend(a: egui::Color32, b: egui::Color32, t: f32) -> egui::Color32 {
    let c = |x: u8, y: u8| (f32::from(x) * (1.0 - t) + f32::from(y) * t).round() as u8;
    egui::Color32::from_rgb(c(a.r(), b.r()), c(a.g(), b.g()), c(a.b(), b.b()))
}
fn round_button(
    ui: &mut egui::Ui,
    label: &str,
    labels: bool,
    enabled: bool,
    held: bool,
    accent: egui::Color32,
    ink: egui::Color32,
) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(52.0, 52.0), egui::Sense::click());
    let center = rect.center();
    let down = enabled && (held || response.is_pointer_button_down_on());
    let color = if !enabled {
        blend(accent, egui::Color32::WHITE, 0.80)
    } else if down {
        accent
    } else if response.hovered() {
        blend(accent, egui::Color32::WHITE, 0.48)
    } else {
        blend(accent, egui::Color32::WHITE, 0.72)
    };
    ui.painter().circle_filled(
        center + egui::vec2(0.0, 2.0),
        24.0,
        blend(accent, egui::Color32::BLACK, 0.04),
    );
    ui.painter().circle_filled(center, 24.0, color);
    ui.painter().circle_stroke(
        center,
        24.0,
        egui::Stroke::new(1.0, blend(accent, egui::Color32::WHITE, 0.22)),
    );
    if labels {
        ui.painter().text(
            center,
            egui::Align2::CENTER_CENTER,
            label,
            egui::FontId::proportional(16.0),
            ink,
        );
    }
    response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, enabled, label));
    response.on_hover_text(format!("Button {label}"))
}
fn diagnostics(ctx: &egui::Context, s: &Snapshot, open: &mut bool) {
    egui::Window::new("Diagnostics").open(open).show(ctx, |ui| {
        ui.label(s.profile.name());
        ui.monospace(format!(
            "PC {:04X} | {:.3} emulated seconds",
            s.state.pc,
            s.ticks as f64 / MASTER_HZ as f64
        ));
        let p = &s.performance;
        ui.monospace(format!(
            "Speed {:.2}% | Average {:.2}% | Lag {:.2} ms",
            p.speed_percent, p.average_speed_percent, p.lag_ms
        ));
        ui.monospace(format!(
            "{} instructions | {} interrupts | {} cycles",
            s.instructions,
            s.interrupts,
            s.cycles.total()
        ));
        ui.monospace(format!("OSC1 32768 Hz | OSC2 {} Hz", s.profile.osc2_hz()));
        ui.monospace(format!(
            "Core {:.2} ms | Audio {:.2} ms | Wait {:.2} ms",
            p.core_ms, p.audio_ms, p.wait_ms
        ));
        ui.label(&s.audio_status);
        ui.label(format!("Audio overruns: {}", s.audio_drops));
        if let Some(error) = &s.diagnostics_error {
            ui.colored_label(egui::Color32::RED, error);
        }
    });
}
fn paint_profile_lcd(
    ui: &mut egui::Ui,
    display: &DisplaySnapshot,
    width: f32,
    empty: bool,
    profile: DeviceProfile,
) -> egui::Response {
    let (lcd_size, pixels, icon_bits, icon_spans) = match profile {
        DeviceProfile::P1 => (LCD_SIZE, PIXELS, ICON_BITS, ICON_SPANS),
        DeviceProfile::Genjintch => {
            use crate::genjintch_lcd_geometry as g;
            (g::LCD_SIZE, g::PIXELS, g::ICON_BITS, g::ICON_SPANS)
        }
        DeviceProfile::Angel => {
            use crate::angel_lcd_geometry as g;
            (g::LCD_SIZE, g::PIXELS, g::ICON_BITS, g::ICON_SPANS)
        }
    };
    let scale = width / lcd_size[0];
    let size = egui::vec2(width, lcd_size[1] * scale);
    let (rect, response) = ui.allocate_exact_size(
        size,
        if profile.has_knock() {
            egui::Sense::click()
        } else {
            egui::Sense::hover()
        },
    );
    let background = egui::Color32::from_rgb(215, 225, 204);
    let on = egui::Color32::from_rgb(45, 56, 41);
    let off = egui::Color32::from_rgb(207, 217, 196);
    ui.painter().rect_filled(rect, 4.0, background);
    for &(x, y, index, bit) in pixels {
        let cell = egui::Rect::from_min_size(
            rect.min + egui::vec2(x, y) * scale,
            egui::vec2(10.0, 10.0) * scale,
        );
        ui.painter().rect_filled(
            cell,
            0.0,
            if display.segment(index as usize, bit) {
                on
            } else {
                off
            },
        );
    }
    for (index, &(ram, bit)) in icon_bits.iter().enumerate() {
        let color = if display.segment(ram as usize, bit) {
            on
        } else {
            off
        };
        for &(x, y, length) in icon_spans[index] {
            let span = egui::Rect::from_min_size(
                rect.min + egui::vec2(f32::from(x), f32::from(y)) * scale,
                egui::vec2(f32::from(length), 1.0) * scale,
            );
            ui.painter().rect_filled(span, 0.0, color);
        }
    }
    if empty {
        ui.painter().text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            "Open a ROM from File",
            egui::FontId::proportional(15.0),
            on,
        );
    }
    response
}

impl eframe::App for RetroHatchApp {
    fn clear_color(&self, _: &egui::Visuals) -> [f32; 4] {
        palette(self.settings.theme).0.to_normalized_gamma_f32()
    }
    fn ui(&mut self, ui: &mut egui::Ui, _: &mut eframe::Frame) {
        self.content(ui);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn lcd_geometry_has_unique_bits_and_bounded_pixel_and_icon_coordinates() {
        let mut bits = std::collections::HashSet::new();
        assert_eq!(PIXELS.len(), 512);
        for &(x, y, ram, bit) in PIXELS {
            assert!(x >= 0.0 && x + 10.0 <= LCD_SIZE[0]);
            assert!(y >= 0.0 && y + 10.0 <= LCD_SIZE[1]);
            assert!(ram < 160 && bit < 4 && bits.insert((ram, bit)));
        }
        for (index, &(ram, bit)) in ICON_BITS.iter().enumerate() {
            assert!(bits.insert((ram, bit)));
            assert!(!ICON_SPANS[index].is_empty());
            for &(x, y, length) in ICON_SPANS[index] {
                assert!(f32::from(x + length) <= LCD_SIZE[0] && f32::from(y + 1) <= LCD_SIZE[1]);
            }
        }
        assert_eq!(bits.len(), 520);
    }
    #[test]
    fn genjintch_geometry_has_unique_bits_and_preserves_original_icon_locations() {
        use crate::genjintch_lcd_geometry::{ICON_BITS, ICON_SPANS, LCD_SIZE, PIXELS};
        let mut bits = std::collections::HashSet::new();
        assert_eq!(PIXELS.len(), 512);
        for &(x, y, ram, bit) in PIXELS {
            assert!(x >= 0.0 && x + 10.0 <= LCD_SIZE[0]);
            assert!(y >= 0.0 && y + 10.0 <= LCD_SIZE[1]);
            assert!(ram < 160 && bit < 4 && bits.insert((ram, bit)));
        }
        for (index, &(ram, bit)) in ICON_BITS.iter().enumerate() {
            assert!(bits.insert((ram, bit)));
            assert!(!ICON_SPANS[index].is_empty());
            for &(x, y, length) in ICON_SPANS[index] {
                assert!(f32::from(x + length) <= LCD_SIZE[0] && f32::from(y + 1) <= LCD_SIZE[1]);
            }
        }
        assert_eq!(bits.len(), 520);
        assert_eq!(
            ICON_BITS,
            &[
                (137, 3),
                (155, 3),
                (157, 3),
                (159, 3),
                (38, 0),
                (36, 0),
                (34, 0),
                (16, 0)
            ]
        );
    }
    #[test]
    fn angel_geometry_has_all_original_pixels_and_eight_unique_symbols() {
        use crate::angel_lcd_geometry::{ICON_BITS, ICON_SPANS, LCD_SIZE, PIXELS};
        let mut bits = std::collections::HashSet::new();
        assert_eq!(PIXELS.len(), 512);
        for &(x, y, ram, bit) in PIXELS {
            assert!(x >= 0.0 && x + 10.0 <= LCD_SIZE[0]);
            assert!(y >= 0.0 && y + 10.0 <= LCD_SIZE[1]);
            assert!(ram < 160 && bit < 4 && bits.insert((ram, bit)));
        }
        for (index, &(ram, bit)) in ICON_BITS.iter().enumerate() {
            assert!(bits.insert((ram, bit)));
            assert!(!ICON_SPANS[index].is_empty());
            for &(x, y, length) in ICON_SPANS[index] {
                assert!(f32::from(x + length) <= LCD_SIZE[0] && f32::from(y + 1) <= LCD_SIZE[1]);
            }
        }
        assert_eq!(bits.len(), 520);
        assert_eq!(
            ICON_BITS,
            &[
                (16, 1),
                (137, 2),
                (16, 2),
                (137, 1),
                (16, 3),
                (16, 0),
                (137, 0),
                (137, 3)
            ]
        );
    }
    #[test]
    fn egui_interface_and_actual_segment_geometry_render_without_a_firmware_or_native_window() {
        let context = egui::Context::default();
        let mut app = RetroHatchApp::create(false);
        let mut output = context.run_ui(egui::RawInput::default(), |ui| {
            app.content(ui);
        });
        assert!(!output.shapes.is_empty());
        let primitives = context.tessellate(output.shapes.clone(), output.pixels_per_point);
        assert!(!primitives.is_empty());
        // This test deliberately has no GPU backend to upload the font atlas.
        output.textures_delta.clear();
    }
    #[test]
    fn genjintch_display_tessellates_with_original_layout_and_no_shell() {
        let context = egui::Context::default();
        let mut output = context.run_ui(egui::RawInput::default(), |ui| {
            paint_profile_lcd(
                ui,
                &DisplaySnapshot { nibbles: [15; 160] },
                352.0,
                false,
                DeviceProfile::Genjintch,
            );
        });
        assert!(
            !context
                .tessellate(output.shapes.clone(), output.pixels_per_point)
                .is_empty()
        );
        output.textures_delta.clear();
    }
    #[test]
    fn angel_display_tessellates_with_original_layout_and_no_shell() {
        let context = egui::Context::default();
        let mut output = context.run_ui(egui::RawInput::default(), |ui| {
            paint_profile_lcd(
                ui,
                &DisplaySnapshot { nibbles: [15; 160] },
                352.0,
                false,
                DeviceProfile::Angel,
            );
        });
        assert!(
            !context
                .tessellate(output.shapes.clone(), output.pixels_per_point)
                .is_empty()
        );
        output.textures_delta.clear();
    }
    #[test]
    fn knock_shortcuts_reach_the_worker_and_focus_loss_releases_the_sensor() {
        let folder =
            std::env::temp_dir().join(format!("retrohatch-ui-knock-{}", std::process::id()));
        std::fs::create_dir_all(&folder).unwrap();
        let path = folder.join("synthetic.bin");
        let mut bytes = vec![0; DeviceProfile::Genjintch.rom_bytes()];
        for pair in bytes.chunks_exact_mut(2) {
            pair.copy_from_slice(&0xFFBu16.to_be_bytes());
        }
        let program = include_str!("../tests/data/genjintch_programs.txt")
            .lines()
            .find(|s| s.starts_with("genjintch_inputs|"))
            .unwrap();
        for item in program.split('|').nth(2).unwrap().split(',') {
            let (a, v) = item.split_once(':').unwrap();
            let offset = usize::from_str_radix(a, 16).unwrap() * 2;
            bytes[offset..offset + 2]
                .copy_from_slice(&u16::from_str_radix(v, 16).unwrap().to_be_bytes());
        }
        std::fs::write(&path, bytes).unwrap();
        let mut app = RetroHatchApp::create(false);
        app.selected_profile = DeviceProfile::Genjintch;
        app.load_rom(path.clone());
        let wait_for = |app: &RetroHatchApp, nibble: u8, interrupts: u64| {
            let end = Instant::now() + Duration::from_secs(3);
            loop {
                let s = app.controller.snapshot();
                if s.state.halted && s.display.nibbles[16] == nibble && s.interrupts >= interrupts {
                    break;
                }
                assert!(
                    Instant::now() < end,
                    "Expected LCD {nibble:X}; error {:?}",
                    s.error
                );
                std::thread::sleep(Duration::from_millis(5));
            }
        };
        wait_for(&app, 0, 0);
        let context = egui::Context::default();
        let frame = |app: &mut RetroHatchApp, focused: bool, key: Option<egui::Key>| {
            // Match egui-winit: RawInput.focused and WindowFocused are both sent.
            // The event clears egui's held keys on focus loss, just as on Windows.
            let mut events = vec![egui::Event::WindowFocused(focused)];
            events.extend(key.map(|key| egui::Event::Key {
                key,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            }));
            let mut output = context.run_ui(
                egui::RawInput {
                    focused,
                    events,
                    ..Default::default()
                },
                |ui| app.content(ui),
            );
            output.textures_delta.clear();
        };
        frame(&mut app, true, Some(egui::Key::K));
        wait_for(&app, 7, 1);
        assert!(app.pressed[3]);
        frame(&mut app, false, None);
        assert!(!app.pressed[3]);
        frame(&mut app, true, Some(egui::Key::Num4));
        wait_for(&app, 7, 2);
        frame(&mut app, false, None);
        frame(&mut app, true, Some(egui::Key::A));
        wait_for(&app, 11, 3);
        drop(app);
        std::fs::remove_file(path).unwrap();
        std::fs::remove_dir(folder).unwrap();
    }
    #[test]
    fn angel_knock_shortcuts_and_focus_release_reach_the_actual_lcd() {
        let folder =
            std::env::temp_dir().join(format!("retrohatch-ui-angel-knock-{}", std::process::id()));
        std::fs::create_dir_all(&folder).unwrap();
        let path = folder.join("synthetic.bin");
        let mut bytes = vec![0; DeviceProfile::Angel.rom_bytes()];
        for pair in bytes.chunks_exact_mut(2) {
            pair.copy_from_slice(&0xFFBu16.to_be_bytes());
        }
        let program = include_str!("../tests/data/genjintch_programs.txt")
            .lines()
            .find(|s| s.starts_with("genjintch_inputs|"))
            .unwrap();
        for item in program.split('|').nth(2).unwrap().split(',') {
            let (a, v) = item.split_once(':').unwrap();
            let offset = usize::from_str_radix(a, 16).unwrap() * 2;
            bytes[offset..offset + 2]
                .copy_from_slice(&u16::from_str_radix(v, 16).unwrap().to_be_bytes());
        }
        std::fs::write(&path, bytes).unwrap();
        let mut app = RetroHatchApp::create(false);
        app.selected_profile = DeviceProfile::Angel;
        app.load_rom(path.clone());
        let wait_for = |app: &RetroHatchApp, nibble: u8, interrupts: u64| {
            let end = Instant::now() + Duration::from_secs(3);
            loop {
                let s = app.controller.snapshot();
                if s.state.halted && s.display.nibbles[16] == nibble && s.interrupts >= interrupts {
                    break;
                }
                assert!(
                    Instant::now() < end,
                    "Expected LCD {nibble:X}; error {:?}",
                    s.error
                );
                std::thread::sleep(Duration::from_millis(5));
            }
        };
        wait_for(&app, 0, 0);
        let context = egui::Context::default();
        let frame = |app: &mut RetroHatchApp, focused: bool, key: Option<egui::Key>| {
            // Match egui-winit: RawInput.focused and WindowFocused are both sent.
            // The event clears egui's held keys on focus loss, just as on Windows.
            let mut events = vec![egui::Event::WindowFocused(focused)];
            events.extend(key.map(|key| egui::Event::Key {
                key,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            }));
            let mut output = context.run_ui(
                egui::RawInput {
                    focused,
                    events,
                    ..Default::default()
                },
                |ui| app.content(ui),
            );
            output.textures_delta.clear();
        };
        frame(&mut app, true, Some(egui::Key::K));
        wait_for(&app, 7, 1);
        assert!(app.pressed[3]);
        frame(&mut app, false, None);
        assert!(!app.pressed[3]);
        frame(&mut app, true, Some(egui::Key::Num4));
        wait_for(&app, 7, 2);
        frame(&mut app, false, None);
        frame(&mut app, true, Some(egui::Key::A));
        wait_for(&app, 11, 3);
        drop(app);
        std::fs::remove_file(path).unwrap();
        std::fs::remove_dir(folder).unwrap();
    }
    #[test]
    fn all_pastel_views_have_exactly_three_menus_optional_labels_and_unchanged_lcd_colors() {
        fn texts(shapes: &[egui::epaint::ClippedShape]) -> Vec<String> {
            fn walk(s: &egui::epaint::Shape, out: &mut Vec<String>) {
                match s {
                    egui::epaint::Shape::Text(t) => out.push(t.galley.text().to_owned()),
                    egui::epaint::Shape::Vec(v) => {
                        for s in v {
                            walk(s, out)
                        }
                    }
                    _ => {}
                }
            }
            let mut out = Vec::new();
            for s in shapes {
                walk(&s.shape, &mut out);
            }
            out
        }
        for theme in Theme::ALL {
            for labels in [false, true] {
                let ctx = egui::Context::default();
                let mut app = RetroHatchApp::create(false);
                app.settings.theme = theme;
                app.settings.labels = labels;
                let mut output = ctx.run_ui(egui::RawInput::default(), |ui| app.content(ui));
                let strings = texts(&output.shapes);
                for menu in ["File", "Emulation", "Settings"] {
                    assert!(
                        strings.iter().any(|s| s == menu),
                        "Missing {menu}: {strings:?}"
                    );
                }
                for label in ["A", "B", "C"] {
                    assert_eq!(strings.iter().any(|s| s == label), labels);
                }
                assert!(!strings.iter().any(|s| s.contains("Model:")
                    || s.contains("Unidentified")
                    || s == "RetroHatch"
                    || s == "Tap"
                    || s == "Save States"));
                let has_fill = |color| {
                    output
                        .shapes
                        .iter()
                        .any(|s| matches!(&s.shape,egui::epaint::Shape::Rect(r)if r.fill==color))
                };
                assert!(has_fill(palette(theme).0));
                assert!(has_fill(egui::Color32::from_rgb(215, 225, 204)));
                output.textures_delta.clear();
            }
        }
    }
    #[test]
    fn settings_menu_checkboxes_and_theme_selection_persist_the_actual_user_actions() {
        fn find(shape: &egui::epaint::Shape, label: &str) -> Option<egui::Pos2> {
            match shape {
                egui::epaint::Shape::Text(t) if t.galley.text() == label => {
                    Some(t.pos + t.galley.size() / 2.0)
                }
                egui::epaint::Shape::Vec(v) => v.iter().find_map(|s| find(s, label)),
                _ => None,
            }
        }
        fn center(out: &egui::FullOutput, label: &str) -> egui::Pos2 {
            out.shapes
                .iter()
                .find_map(|s| find(&s.shape, label))
                .unwrap_or_else(|| panic!("Menu item not rendered: {label}"))
        }
        fn frame(
            ctx: &egui::Context,
            app: &mut RetroHatchApp,
            events: Vec<egui::Event>,
        ) -> egui::FullOutput {
            let mut out = ctx.run_ui(
                egui::RawInput {
                    focused: true,
                    events,
                    ..Default::default()
                },
                |ui| app.content(ui),
            );
            out.textures_delta.clear();
            out
        }
        fn click(
            ctx: &egui::Context,
            app: &mut RetroHatchApp,
            pos: egui::Pos2,
        ) -> egui::FullOutput {
            frame(
                ctx,
                app,
                vec![
                    egui::Event::PointerMoved(pos),
                    egui::Event::PointerButton {
                        pos,
                        button: egui::PointerButton::Primary,
                        pressed: true,
                        modifiers: egui::Modifiers::NONE,
                    },
                ],
            );
            frame(
                ctx,
                app,
                vec![egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed: false,
                    modifiers: egui::Modifiers::NONE,
                }],
            );
            frame(ctx, app, vec![])
        }
        let ctx = egui::Context::default();
        let mut app = RetroHatchApp::create(false);
        let first = frame(&ctx, &mut app, vec![]);
        let pos = center(&first, "Settings");
        let menu = click(&ctx, &mut app, pos);
        let pos = center(&menu, "Show A/B/C Labels");
        let menu = click(&ctx, &mut app, pos);
        assert!(!app.settings.labels);
        let pos = center(&menu, "Settings");
        let menu = click(&ctx, &mut app, pos);
        let pos = center(&menu, "Mute Sound");
        let menu = click(&ctx, &mut app, pos);
        assert!(app.settings.muted);
        let pos = center(&menu, "Settings");
        let menu = click(&ctx, &mut app, pos);
        let pos = center(&menu, "Save on Exit");
        let menu = click(&ctx, &mut app, pos);
        assert!(!app.settings.save_on_exit);
        let pos = center(&menu, "Settings");
        let menu = click(&ctx, &mut app, pos);
        let pos = center(&menu, "Theme");
        let menu = click(&ctx, &mut app, pos);
        let pos = center(&menu, "Blue");
        click(&ctx, &mut app, pos);
        assert_eq!(app.settings.theme, Theme::Blue);
        let root =
            std::env::temp_dir().join(format!("retrohatch-menu-settings-{}", std::process::id()));
        app.data_dir = Some(root.clone());
        assert!(app.persist());
        let loaded = Settings::load(&root).unwrap();
        assert_eq!(loaded, app.settings);
        drop(app);
        assert_eq!(root.parent(), Some(std::env::temp_dir().as_path()));
        std::fs::remove_dir_all(root).unwrap();
    }
}
