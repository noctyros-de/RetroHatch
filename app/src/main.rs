// SPDX-License-Identifier: GPL-3.0-only
// SPDX-FileCopyrightText: 2026 Noktyros
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
mod lcd_render;
mod ui;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([460.0, 660.0])
            .with_min_inner_size([350.0, 500.0]),
        renderer: eframe::Renderer::Glow,
        ..Default::default()
    };

    eframe::run_native(
        "RetroHatch",
        options,
        Box::new(|cc| Ok(Box::new(ui::RetroHatchApp::new(cc)))),
    )
}
