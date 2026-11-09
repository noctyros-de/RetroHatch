use crate::lcd_geometry::{ICON_BITS, ICON_SPANS, LCD_SIZE, PIXELS};
use eframe::egui;
use retrohatch::core::device::{Button, DisplaySnapshot};
use retrohatch::core::mcu::MASTER_HZ;
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
    pressed: [bool; 3],
    click_until: [Option<Instant>; 3],
    picker: Option<mpsc::Receiver<Result<Option<PathBuf>, String>>>,
    folder: String,
    library_open: bool,
    library: Vec<PathBuf>,
    local_error: Option<String>,
}

impl RetroHatchApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        cc.egui_ctx.set_visuals(egui::Visuals::light());
        let app = Self::create(true);
        if let Some(path) = std::env::args_os().nth(1) {
            app.controller.send(Command::Load(PathBuf::from(path)));
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
            pressed: [false; 3],
            click_until: [None; 3],
            picker: None,
            folder: folder.to_string_lossy().into_owned(),
            library_open: false,
            library: Vec::new(),
            local_error,
        }
    }
    fn release_inputs(&mut self) {
        self.pressed = [false; 3];
        self.click_until = [None; 3];
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
            let _ = sender.send(platform::pick_p1_rom(&folder));
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
                Ok(Some(path)) => self.controller.send(Command::Load(path)),
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
            self.controller.send(Command::Load(path));
        }
        let snapshot = self.controller.snapshot();
        let keyboard = context.input(|input| {
            let groups = [
                [egui::Key::A, egui::Key::Num1, egui::Key::ArrowLeft],
                [egui::Key::B, egui::Key::Num2, egui::Key::ArrowDown],
                [egui::Key::C, egui::Key::Num3, egui::Key::ArrowRight],
            ];
            let down = groups.map(|group| group.into_iter().any(|key| input.key_down(key)));
            let tapped = groups.map(|group| group.into_iter().any(|key| input.key_pressed(key)));
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
                    .add_enabled(self.picker.is_none(), egui::Button::new("Open P1 ROM..."))
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
        let mut pointer = [false; 3];
        egui::CentralPanel::default().show(ui, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.vertical_centered(|ui| {
                    ui.heading("RetroHatch");
                    ui.label(if snapshot.loaded {
                        "Model: Tamagotchi P1"
                    } else {
                        "Model: No device loaded"
                    });
                    if snapshot.loaded {
                        ui.label(&snapshot.rom_name);
                        ui.weak(if snapshot.known_world_p1 {
                            "Recognized P1 (World)"
                        } else {
                            "Unidentified image - manually assigned to P1"
                        });
                    }
                    ui.add_space(8.0);
                    let width = (ui.available_width() - 16.0).clamp(200.0, 352.0);
                    paint_lcd(ui, &snapshot.display, width, !snapshot.loaded);
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
                        });
                    }
                });
            });
        });
        let now = Instant::now();
        for (index, button) in [Button::A, Button::B, Button::C].into_iter().enumerate() {
            if accepts_keys && keyboard.2[index] {
                self.click_until[index] = Some(now + Duration::from_millis(50));
            }
            let pressed = keyboard.0
                && snapshot.loaded
                && snapshot.running
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
        if !keyboard.0 {
            self.release_inputs();
        }
        if self.library_open {
            let mut open = self.library_open;
            let mut selected = None;
            egui::Window::new("P1 ROM library")
                .open(&mut open)
                .show(&context, |ui| {
                    ui.label(
                        "User-provided P1 dumps only: 12288 bytes, unpacked big-endian words.",
                    );
                    ui.label("Unknown images are manually assigned to the P1 profile.");
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
                self.controller.send(Command::Load(path));
                open = false;
            }
            self.library_open = open;
        }
        context.request_repaint_after(Duration::from_millis(16));
    }
}

fn paint_lcd(ui: &mut egui::Ui, display: &DisplaySnapshot, width: f32, empty: bool) {
    let scale = width / LCD_SIZE[0];
    let size = egui::vec2(width, LCD_SIZE[1] * scale);
    let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
    let background = egui::Color32::from_rgb(215, 225, 204);
    let on = egui::Color32::from_rgb(45, 56, 41);
    let off = egui::Color32::from_rgb(207, 217, 196);
    ui.painter().rect_filled(rect, 4.0, background);
    for &(x, y, index, bit) in PIXELS {
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
    for (index, &(ram, bit)) in ICON_BITS.iter().enumerate() {
        let color = if display.segment(ram as usize, bit) {
            on
        } else {
            off
        };
        for &(x, y, length) in ICON_SPANS[index] {
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
            "Open a P1 ROM to start",
            egui::FontId::proportional(15.0),
            on,
        );
    }
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
}
