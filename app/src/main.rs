#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

#[rustfmt::skip]
mod lcd_geometry;
#[rustfmt::skip]
mod genjintch_lcd_geometry;
#[rustfmt::skip]
mod angel_lcd_geometry;
mod ui;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([440.0, 530.0])
            .with_min_inner_size([350.0, 430.0]),
        renderer: eframe::Renderer::Glow,
        ..Default::default()
    };

    eframe::run_native(
        "RetroHatch",
        options,
        Box::new(|cc| Ok(Box::new(ui::RetroHatchApp::new(cc)))),
    )
}
