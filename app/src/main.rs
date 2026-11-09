#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

#[rustfmt::skip]
mod lcd_geometry;
mod ui;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([480.0, 620.0])
            .with_min_inner_size([360.0, 500.0]),
        renderer: eframe::Renderer::Glow,
        ..Default::default()
    };

    eframe::run_native(
        "RetroHatch",
        options,
        Box::new(|cc| Ok(Box::new(ui::RetroHatchApp::new(cc)))),
    )
}
