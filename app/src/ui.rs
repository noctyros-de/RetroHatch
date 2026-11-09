use eframe::egui;

/// UI only: no device machine is running yet.
pub struct RetroHatchApp;

impl RetroHatchApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        cc.egui_ctx.set_visuals(egui::Visuals::light());
        Self
    }
}

impl eframe::App for RetroHatchApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::MenuBar::new().ui(ui, |ui| {
            ui.menu_button("File", |ui| {
                ui.add_enabled(false, egui::Button::new("Open ROM …"));
                ui.separator();
                if ui.button("Exit").clicked() {
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                }
            });
            ui.menu_button("Save States", |ui| {
                ui.label("Not yet available");
            });
        });
        ui.separator();

        egui::CentralPanel::default().show(ui, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(24.0);
                ui.heading("RetroHatch");
                ui.label("Model: No model loaded");
                ui.add_space(24.0);

                let side = (ui.available_width() - 32.0).clamp(120.0, 256.0);
                let (rect, _) =
                    ui.allocate_exact_size(egui::vec2(side, side), egui::Sense::hover());
                ui.painter()
                    .rect_filled(rect, 4.0, egui::Color32::from_rgb(215, 225, 204));
                ui.painter().text(
                    rect.center(),
                    egui::Align2::CENTER_CENTER,
                    "LCD\nNo device loaded",
                    egui::FontId::proportional(16.0),
                    egui::Color32::from_rgb(64, 76, 58),
                );

                ui.add_space(24.0);
                ui.horizontal(|ui| {
                    let button_width = 64.0;
                    let total_width = 3.0 * button_width + 2.0 * ui.spacing().item_spacing.x;
                    ui.add_space(((ui.available_width() - total_width) / 2.0).max(0.0));
                    for label in ["A", "B", "C"] {
                        ui.add_enabled(
                            false,
                            egui::Button::new(label).min_size(egui::vec2(button_width, 36.0)),
                        )
                        .on_disabled_hover_text("Available once an emulation core is integrated.");
                    }
                });
                ui.add_space(16.0);
                ui.weak("UI scaffold – emulation is not yet available");
            });
        });
    }
}
