// SPDX-License-Identifier: GPL-3.0-only
// SPDX-FileCopyrightText: 2026 Noktyros
// Applies to original RetroHatch contributions; third-party provenance is retained below and in THIRD_PARTY_LICENSES.md.
//! DPI-aware area sampling of exact Qt SVG coverage; square matrix pixels stay geometric.
use crate::lcd_masks::{Icon, Masks};
use retrohatch::core::profile::DeviceProfile;
pub fn masks(p: DeviceProfile) -> &'static Masks {
    match p {
        DeviceProfile::P1 | DeviceProfile::P2 => &crate::lcd_masks::P1,
        DeviceProfile::Genjintch => &crate::lcd_masks::GENJINTCH,
        DeviceProfile::Angel => &crate::lcd_masks::ANGEL,
        DeviceProfile::Morino => &crate::lcd_masks::MORINO,
        DeviceProfile::Umino => &crate::lcd_masks::UMINO,
    }
}
pub struct Coverage {
    pub left: i32,
    pub top: i32,
    pub width: usize,
    pub height: usize,
    pub alpha: Vec<u8>,
}
pub fn coverage(data: &[u8], icon: &Icon, scale: f32) -> Coverage {
    let x = f64::from(icon.origin[0]) * f64::from(scale);
    let y = f64::from(icon.origin[1]) * f64::from(scale);
    let factor = 8.0 / f64::from(scale);
    let left = x.floor() as i32;
    let top = y.floor() as i32;
    let width = (x + icon.size[0] as f64 / factor).ceil() as i32 - left;
    let height = (y + icon.size[1] as f64 / factor).ceil() as i32 - top;
    let mut alpha = vec![0; width as usize * height as usize];
    for row in 0..height as usize {
        for col in 0..width as usize {
            let sx = (f64::from(left) + col as f64 - x) * factor;
            let sy = (f64::from(top) + row as f64 - y) * factor;
            let ex = sx + factor;
            let ey = sy + factor;
            let mut sum = 0.0;
            for py in (sy.floor() as i32).max(0)..(ey.ceil() as i32).min(icon.size[1] as i32) {
                let vy = (ey.min(f64::from(py + 1)) - sy.max(f64::from(py))).max(0.0);
                for px in (sx.floor() as i32).max(0)..(ex.ceil() as i32).min(icon.size[0] as i32) {
                    let vx = (ex.min(f64::from(px + 1)) - sx.max(f64::from(px))).max(0.0);
                    sum += vx
                        * vy
                        * f64::from(data[icon.offset + py as usize * icon.size[0] + px as usize]);
                }
            }
            alpha[row * width as usize + col] =
                (sum / (factor * factor)).round().clamp(0.0, 255.0) as u8;
        }
    }
    Coverage {
        left,
        top,
        width: width as usize,
        height: height as usize,
        alpha,
    }
}
struct Glyph {
    texture: eframe::egui::TextureHandle,
    offset: eframe::egui::Vec2,
    size: eframe::egui::Vec2,
}
struct Cache {
    key: (u8, u32),
    glyphs: Vec<Glyph>,
}
pub fn paint(
    ui: &eframe::egui::Ui,
    rect: eframe::egui::Rect,
    profile: DeviceProfile,
    display: &retrohatch::core::device::DisplaySnapshot,
    on: eframe::egui::Color32,
    off: eframe::egui::Color32,
) {
    use eframe::egui::{self, Color32, ColorImage, Rect, Vec2};
    use std::sync::Arc;
    let ppp = ui.ctx().pixels_per_point();
    let physical_width = (rect.width() * ppp).round() as u32;
    let key = (
        match profile {
            DeviceProfile::P1 | DeviceProfile::P2 => 0,
            DeviceProfile::Genjintch => 1,
            DeviceProfile::Angel => 2,
            DeviceProfile::Morino => 3,
            DeviceProfile::Umino => 4,
        },
        physical_width,
    );
    let id = egui::Id::unique("lcd-icon-coverage");
    let old = ui.ctx().data_mut(|d| d.get_temp::<Arc<Cache>>(id));
    let cached = if let Some(c) = old.filter(|c| c.key == key) {
        c
    } else {
        let source = masks(profile);
        let scale = physical_width as f32 / 352.0;
        let glyphs = source
            .icons
            .iter()
            .enumerate()
            .map(|(i, icon)| {
                let c = coverage(source.data, icon, scale);
                let image = ColorImage::new(
                    [c.width, c.height],
                    c.alpha
                        .iter()
                        .map(|&a| Color32::from_white_alpha(a))
                        .collect(),
                );
                let texture = ui.ctx().load_texture(
                    format!("lcd-icon-{}-{i}", key.0),
                    image,
                    egui::TextureOptions::LINEAR,
                );
                Glyph {
                    texture,
                    offset: Vec2::new(c.left as f32, c.top as f32),
                    size: Vec2::new(c.width as f32, c.height as f32),
                }
            })
            .collect();
        let c = Arc::new(Cache { key, glyphs });
        ui.ctx().data_mut(|d| d.insert_temp(id, c.clone()));
        c
    };
    let source = masks(profile);
    let origin = egui::pos2(
        (rect.min.x * ppp).round() / ppp,
        (rect.min.y * ppp).round() / ppp,
    );
    for (i, g) in cached.glyphs.iter().enumerate() {
        let icon = &source.icons[i];
        let color = if display.segment(icon.ram as usize, icon.bit) {
            on
        } else {
            off
        };
        let r = Rect::from_min_size(origin + g.offset / ppp, g.size / ppp);
        ui.painter().image(
            g.texture.id(),
            r,
            Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
            color,
        );
    }
}
