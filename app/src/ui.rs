use crate::lcd_geometry::{ICON_BITS, ICON_SPANS, LCD_SIZE, PIXELS};
use eframe::egui;
use retrohatch::core::device::{Button, DisplaySnapshot};
use retrohatch::core::{mcu::MASTER_HZ, profile::DeviceProfile};
use retrohatch::host::{
    platform, roms,
    runner::{Command, Controller},
};
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::{Duration, Instant};

pub struct RetroHatchApp {
    controller: Controller,
    audio_enabled: bool,
    selected_profile: DeviceProfile,
    pressed: [bool; 4],
    click_until: [Option<Instant>; 4],
    picker: Option<mpsc::Receiver<Result<Option<PathBuf>, String>>>,
    folder: String,
    library_open: bool,
    library: Vec<PathBuf>,
    local_error: Option<String>,
}

impl RetroHatchApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        cc.egui_ctx.set_visuals(egui::Visuals::light());
        let mut app = Self::create(true);
        let mut args = std::env::args_os().skip(1);
        if let Some(first) = args.next() {
            if first == "--profile" {
                let profile = args
                    .next()
                    .and_then(|s| s.to_str().and_then(DeviceProfile::parse));
                if let Some(profile) = profile {
                    app.selected_profile = profile;
                    if let Some(path) = args.next() {
                        app.load_rom(path.into());
                    }
                } else {
                    app.local_error = Some(
                        "Use --profile p1, --profile genjintch or --profile angel followed by a local ROM path."
                            .into(),
                    );
                }
            } else {
                app.load_rom(first.into());
            }
        }
        app
    }
    fn create(audio_enabled: bool) -> Self {
        let folder = roms::default_folder();
        let local_error = std::fs::create_dir_all(&folder).err().map(|error| {
            format!("Cannot create ROM folder: {error}. You can still open a ROM directly.")
        });
        Self {
            controller: Controller::new(audio_enabled),
            audio_enabled,
            selected_profile: DeviceProfile::P1,
            pressed: [false; 4],
            click_until: [None; 4],
            picker: None,
            folder: folder.to_string_lossy().into_owned(),
            library_open: false,
            library: Vec::new(),
            local_error,
        }
    }
    fn load_rom(&mut self, path: PathBuf) {
        self.release_inputs();
        self.controller
            .send(Command::LoadProfile(path, self.selected_profile));
    }
    fn release_inputs(&mut self) {
        self.pressed = [false; 4];
        self.click_until = [None; 4];
        self.controller.send(Command::ReleaseInputs);
    }
    fn select_rom(&mut self, context: &egui::Context) {
        if self.picker.is_some() {
            return;
        }
        self.release_inputs();
        self.controller.send(Command::Pause);
        let folder = PathBuf::from(&self.folder);
        let (sender, receiver) = mpsc::channel();
        let context = context.clone();
        std::thread::spawn(move || {
            let _ = sender.send(platform::pick_rom(&folder));
            context.request_repaint();
        });
        self.picker = Some(receiver);
    }
    fn rescan(&mut self) {
        match roms::list(std::path::Path::new(&self.folder)) {
            Ok(paths) => {
                self.library = paths;
                self.local_error = None;
            }
            Err(error) => {
                self.library.clear();
                self.local_error = Some(error);
            }
        }
    }
    fn content(&mut self, ui: &mut egui::Ui) {
        let context = ui.ctx().clone();
        let picker_result = self
            .picker
            .as_ref()
            .and_then(|receiver| receiver.try_recv().ok());
        if let Some(result) = picker_result {
            self.picker = None;
            match result {
                Ok(Some(path)) => self.load_rom(path),
                Ok(None) => {}
                Err(error) => self.local_error = Some(error),
            }
        }
        let dropped = context.input(|input| input.raw.dropped_files.clone());
        if let Some(path) = dropped
            .into_iter()
            .next()
            .map(|file| file.path().to_path_buf())
        {
            self.release_inputs();
            self.load_rom(path);
        }
        let snapshot = self.controller.snapshot();
        let keyboard = context.input(|input| {
            let groups: [&[egui::Key]; 4] = [
                &[egui::Key::A, egui::Key::Num1, egui::Key::ArrowLeft],
                &[egui::Key::B, egui::Key::Num2, egui::Key::ArrowDown],
                &[egui::Key::C, egui::Key::Num3, egui::Key::ArrowRight],
                &[egui::Key::K, egui::Key::Num4],
            ];
            let down = groups.map(|group| group.into_iter().any(|key| input.key_down(*key)));
            let tapped = groups.map(|group| group.into_iter().any(|key| input.key_pressed(*key)));
            (
                input.focused,
                down,
                tapped,
                input.key_pressed(egui::Key::Space),
                input.key_pressed(egui::Key::F5),
            )
        });
        let accepts_keys = keyboard.0
            && !context.egui_wants_keyboard_input()
            && self.picker.is_none()
            && !self.library_open;
        if accepts_keys && keyboard.3 {
            self.controller.send(if snapshot.running {
                Command::Pause
            } else {
                Command::Run
            });
        }
        if accepts_keys && keyboard.4 {
            self.release_inputs();
            self.controller.send(Command::Reset);
        }

        egui::MenuBar::new().ui(ui, |ui| {
            ui.menu_button("File", |ui| {
                if ui
                    .add_enabled(self.picker.is_none(), egui::Button::new("Open ROM..."))
                    .clicked()
                {
                    self.select_rom(&context);
                    ui.close();
                }
                if ui.button("ROM library...").clicked() {
                    self.release_inputs();
                    self.rescan();
                    self.library_open = true;
                    ui.close();
                }
                ui.separator();
                if ui.button("Exit").clicked() {
                    context.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            });
            ui.menu_button("Emulation", |ui| {
                if ui
                    .add_enabled(snapshot.loaded, egui::Button::new("Run"))
                    .clicked()
                {
                    self.controller.send(Command::Run);
                    ui.close();
                }
                if ui
                    .add_enabled(snapshot.loaded, egui::Button::new("Pause"))
                    .clicked()
                {
                    self.release_inputs();
                    self.controller.send(Command::Pause);
                    ui.close();
                }
                if ui
                    .add_enabled(snapshot.loaded, egui::Button::new("Reset"))
                    .clicked()
                {
                    self.release_inputs();
                    self.controller.send(Command::Reset);
                    ui.close();
                }
                ui.separator();
                if ui
                    .checkbox(&mut self.audio_enabled, "Audio enabled")
                    .changed()
                {
                    self.controller.send(Command::Audio(self.audio_enabled));
                }
            });
            ui.menu_button("Save States", |ui| {
                ui.label("Not available in this milestone");
            });
        });
        ui.separator();
        let mut pointer = [false; 4];
        egui::CentralPanel::default().show(ui, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.vertical_centered(|ui| {
                    ui.heading("RetroHatch");
                    ui.label(if snapshot.loaded {
                        format!("Model: {}", snapshot.profile.name())
                    } else { "Model: No device loaded".into() });
                    egui::ComboBox::from_label("Profile for next ROM")
                        .selected_text(self.selected_profile.name())
                        .show_ui(ui, |ui| {
                            for profile in DeviceProfile::ALL {
                                ui.selectable_value(&mut self.selected_profile, profile, profile.name());
                            }
                        });
                    if snapshot.loaded {
                        ui.label(&snapshot.rom_name);
                        ui.weak(if snapshot.known_world_p1 {
                            "Recognized P1 (World)".into()
                        } else { format!("Unidentified image - manually assigned to {}", snapshot.profile.name()) });
                    }
                    ui.add_space(8.0);
                    let width = (ui.available_width() - 16.0).clamp(200.0, 352.0);
                    let profile = if snapshot.loaded { snapshot.profile } else { self.selected_profile };
                    let lcd = paint_profile_lcd(ui, &snapshot.display, width, !snapshot.loaded, profile);
                    if snapshot.loaded && snapshot.running && profile.has_knock() {
                        pointer[3] = lcd.is_pointer_button_down_on();
                        if lcd.clicked() { self.click_until[3] = Some(Instant::now() + Duration::from_millis(50)); }
                    }
                    ui.add_space(12.0);
                    ui.horizontal(|ui| {
                        let total = 3.0 * 64.0 + 2.0 * ui.spacing().item_spacing.x;
                        ui.add_space(((ui.available_width() - total) / 2.0).max(0.0));
                        for (index, label) in ["A", "B", "C"].into_iter().enumerate() {
                            let response = ui.add_enabled(
                                snapshot.loaded && snapshot.running,
                                egui::Button::new(label).min_size(egui::vec2(64.0, 36.0)),
                            );
                            pointer[index] = response.is_pointer_button_down_on();
                            if response.clicked() {
                                self.click_until[index] =
                                    Some(Instant::now() + Duration::from_millis(50));
                            }
                        }
                    });
                    if snapshot.loaded && snapshot.profile.has_knock() {
                        let response = ui.add_enabled(snapshot.running,
                            egui::Button::new("Knock / Tap screen (4 or K)").min_size(egui::vec2(220.0, 32.0)));
                        pointer[3] |= response.is_pointer_button_down_on();
                        if response.clicked() { self.click_until[3] = Some(Instant::now() + Duration::from_millis(50)); }
                    }
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        if ui
                            .add_enabled(
                                snapshot.loaded && !snapshot.running,
                                egui::Button::new("Run"),
                            )
                            .clicked()
                        {
                            self.controller.send(Command::Run);
                        }
                        if ui
                            .add_enabled(snapshot.running, egui::Button::new("Pause"))
                            .clicked()
                        {
                            self.release_inputs();
                            self.controller.send(Command::Pause);
                        }
                        if ui
                            .add_enabled(snapshot.loaded, egui::Button::new("Reset"))
                            .clicked()
                        {
                            self.release_inputs();
                            self.controller.send(Command::Reset);
                        }
                        ui.label(if !snapshot.loaded {
                            "No ROM loaded"
                        } else if !snapshot.running {
                            "Paused"
                        } else if snapshot.state.halted {
                            "Sleeping (timers running)"
                        } else {
                            "Running"
                        });
                    });
                    ui.weak("A/B/C or 1/2/3 or Left/Down/Right | Space: Run/Pause | F5: Reset");
                    if snapshot.loaded {
                        ui.weak(&snapshot.audio_status);
                    }
                    if let Some(error) = self.local_error.as_ref().or(snapshot.error.as_ref()) {
                        ui.colored_label(egui::Color32::from_rgb(160, 35, 35), error);
                    }
                    if snapshot.loaded {
                        ui.collapsing("Diagnostics", |ui| {
                            ui.monospace(format!(
                                "PC {:04X} | {:.3} s | {} instructions | {} interrupts",
                                snapshot.state.pc,
                                snapshot.ticks as f64 / MASTER_HZ as f64,
                                snapshot.instructions,
                                snapshot.interrupts
                            ));
                            if let Some(crc) = snapshot.crc32 {
                                ui.monospace(format!("CRC32 {crc:08X}"));
                            }
                            ui.label(format!("Audio output overruns: {}", snapshot.audio_drops));
                            ui.monospace(format!("OSC1: 32768 Hz | OSC2: {} Hz", snapshot.profile.osc2_hz()));
                            let p = &snapshot.performance;
                            ui.monospace(format!("Real-time {:.1}% | Average {:.2}% | Lag {:.2} ms",
                                p.speed_percent, p.average_speed_percent, p.lag_ms));
                            ui.monospace(format!("Emulated {:.3} s / active wall {:.3} s",
                                p.emulated_seconds, p.wall_seconds));
                            ui.monospace(format!("Cycles {} (CPU {}, HALT {}, IRQ {})",
                                snapshot.cycles.total(), snapshot.cycles.instructions,
                                snapshot.cycles.halt, snapshot.cycles.interrupts));
                            ui.monospace(format!("OSC2 cycles {} | OSC1 cycles {} | OSC1 edges {}",
                                snapshot.cycles.fast, snapshot.cycles.slow, snapshot.cycles.osc1_edges));
                            ui.monospace(format!("Reset cycles {} | PCM samples {} | Worker waits {}",
                                snapshot.cycles.reset, snapshot.cycles.audio_samples, p.wait_calls));
                            ui.monospace(format!("Command processing {:.3} ms | UI snapshot reads {}",
                                p.commands_ms, p.ui_reads));
                            ui.monospace(format!("{:.0} instructions/s | {:.0} cycles/s | {:.0} worker iterations/s",
                                p.instructions_per_second, p.cycles_per_second, p.iterations_per_second));
                            ui.monospace(format!("Work batches {} | catch-up batches {} | maximum wait {:.2} ms",
                                p.work_batches, p.catchup_batches, p.max_wait_ms));
                            ui.monospace(format!("Host totals (ms): core {:.2}, audio {:.2}, wait {:.2}",
                                p.core_ms, p.audio_ms, p.wait_ms));
                            ui.monospace(format!("Snapshots {:.2} ms, worker lock wait {:.3} ms, UI lock wait {:.3} ms",
                                p.snapshot_ms, p.snapshot_lock_ms, p.ui_lock_ms));
                            ui.monospace(format!("Maximum command latency {:.2} ms", p.command_latency_max_ms));
                            let mut detailed = snapshot.detailed_profiling;
                            if ui.checkbox(&mut detailed, "Detailed core profiling").changed() {
                                self.controller.send(Command::Profiling(detailed));
                            }
                            if detailed {
                                let c = snapshot.core_profile;
                                ui.monospace(format!("Core totals (ms): CPU {:.2}, IRQ {:.2}, timers {:.2}, PCM {:.2}",
                                    c.cpu_ns as f64 / 1e6, c.irq_ns as f64 / 1e6,
                                    c.timers_ns as f64 / 1e6, c.pcm_ns as f64 / 1e6));
                                ui.weak("Detailed profiling adds timing overhead.");
                            }
                            if let Some(error) = &snapshot.diagnostics_error {
                                ui.colored_label(egui::Color32::RED, error);
                            }
                        });
                    }
                });
            });
        });
        let now = Instant::now();
        for (index, button) in [Button::A, Button::B, Button::C, Button::Knock]
            .into_iter()
            .enumerate()
        {
            if accepts_keys && keyboard.2[index] {
                self.click_until[index] = Some(now + Duration::from_millis(50));
            }
            let pressed = keyboard.0
                && snapshot.loaded
                && snapshot.running
                && (index != 3 || snapshot.profile.has_knock())
                && self.picker.is_none()
                && !self.library_open
                && (pointer[index]
                    || (accepts_keys && keyboard.1[index])
                    || self.click_until[index].is_some_and(|until| now < until));
            if pressed != self.pressed[index] {
                self.pressed[index] = pressed;
                self.controller.send(Command::Button(button, pressed));
            }
        }
        if !keyboard.0
            && (self.pressed.iter().any(|&p| p) || self.click_until.iter().any(Option::is_some))
        {
            self.release_inputs();
        }
        if self.library_open {
            let mut open = self.library_open;
            let mut selected = None;
            egui::Window::new("ROM library")
                .open(&mut open)
                .show(&context, |ui| {
                    egui::ComboBox::from_label("Device profile")
                        .selected_text(self.selected_profile.name())
                        .show_ui(ui, |ui| {
                            for profile in DeviceProfile::ALL { ui.selectable_value(&mut self.selected_profile, profile, profile.name()); }
                        });
                    ui.label(format!("{}: {} bytes, unpacked big-endian words.", self.selected_profile.name(), self.selected_profile.rom_bytes()));
                    ui.label("Select the hardware profile explicitly. Filenames do not identify a device.");
                    ui.text_edit_singleline(&mut self.folder);
                    if ui.button("Rescan folder").clicked() {
                        self.rescan();
                    }
                    if self.library.is_empty() {
                        ui.weak("No .bin or .rom files found. No firmware is included.");
                    }
                    for path in &self.library {
                        if ui
                            .button(path.file_name().unwrap_or_default().to_string_lossy())
                            .clicked()
                        {
                            selected = Some(path.clone());
                        }
                    }
                });
            if let Some(path) = selected {
                self.load_rom(path);
                open = false;
            }
            self.library_open = open;
        }
        context.request_repaint_after(Duration::from_millis(16));
    }
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
            "Select a profile and open a ROM",
            egui::FontId::proportional(15.0),
            on,
        );
    }
    response
}
impl eframe::App for RetroHatchApp {
    fn clear_color(&self, visuals: &egui::Visuals) -> [f32; 4] {
        visuals.panel_fill.to_normalized_gamma_f32()
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
}
