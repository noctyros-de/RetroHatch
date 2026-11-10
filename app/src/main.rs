// SPDX-License-Identifier: GPL-3.0-only
// SPDX-FileCopyrightText: 2026 Noctyros
// Applies to original RetroHatch contributions; third-party provenance is retained below and in THIRD_PARTY_LICENSES.md.
#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

#[rustfmt::skip]
mod lcd_geometry;
#[rustfmt::skip]
mod genjintch_lcd_geometry;
#[rustfmt::skip]
mod angel_lcd_geometry;
#[rustfmt::skip]
mod morino_lcd_geometry;
#[rustfmt::skip]
mod umino_lcd_geometry;
#[rustfmt::skip]
mod lcd_masks;
mod branding;
mod lcd_render;
mod ui;

fn main() -> eframe::Result {
    let mut viewport = eframe::egui::ViewportBuilder::default()
        .with_inner_size([460.0, 660.0])
        .with_min_inner_size([350.0, 500.0]);
    if let Some(icon) = branding::icon() {
        viewport = viewport.with_icon(icon);
    }
    let options = eframe::NativeOptions {
        viewport,
        renderer: eframe::Renderer::Glow,
        ..Default::default()
    };

    eframe::run_native(
        "RetroHatch",
        options,
        Box::new(|cc| Ok(Box::new(ui::RetroHatchApp::new(cc)))),
    )
}
