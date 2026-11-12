// SPDX-License-Identifier: GPL-3.0-only
// SPDX-FileCopyrightText: 2026 Noctyros
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
        DeviceProfile::Yasashii => unreachable!("Yasashii has no fixed LCD icons"),
        DeviceProfile::ConnectionV3Eu
        | DeviceProfile::On
        | DeviceProfile::MeetsFairy
        | DeviceProfile::MeetsMagical
        | DeviceProfile::MeetsFantasy
        | DeviceProfile::MeetsPastel
        | DeviceProfile::MeetsSanrio
        | DeviceProfile::MeetsSweets
        | DeviceProfile::Ps
        | DeviceProfile::Id
        | DeviceProfile::IdMelody
        | DeviceProfile::Idl
        | DeviceProfile::IdlEnglish
        | DeviceProfile::IdlPrincess
        | DeviceProfile::Idl15
        | DeviceProfile::PlusColor
        | DeviceProfile::PlusColorHexa
        | DeviceProfile::FourUPlus
        | DeviceProfile::FourUPlusEnglish => {
            panic!("On has a color LCD, not fixed segment masks")
        }
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
    if profile == DeviceProfile::Yasashii {
        return;
    }
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
            DeviceProfile::Yasashii => unreachable!("Yasashii has no fixed LCD icons"),
            DeviceProfile::ConnectionV3Eu
            | DeviceProfile::On
            | DeviceProfile::MeetsFairy
            | DeviceProfile::MeetsMagical
            | DeviceProfile::MeetsFantasy
            | DeviceProfile::MeetsPastel
            | DeviceProfile::MeetsSanrio
            | DeviceProfile::MeetsSweets
            | DeviceProfile::Ps
            | DeviceProfile::Id
            | DeviceProfile::IdMelody
            | DeviceProfile::Idl
            | DeviceProfile::IdlEnglish
            | DeviceProfile::IdlPrincess
            | DeviceProfile::Idl15
            | DeviceProfile::PlusColor
            | DeviceProfile::PlusColorHexa
            | DeviceProfile::FourUPlus
            | DeviceProfile::FourUPlusEnglish => {
                unreachable!("Color LCD does not use segment coverage")
            }
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

/// Segment LCDs are vector layouts, not 32x16 framebuffers. Export at the reference bounds.
pub fn native_lcd(
    profile: DeviceProfile,
    display: &retrohatch::core::device::DisplaySnapshot,
) -> Result<retrohatch::host::screenshots::Image, String> {
    use retrohatch::host::screenshots::Image;
    let (size, pixels) = match profile {
        DeviceProfile::P1 | DeviceProfile::P2 => {
            (crate::lcd_geometry::LCD_SIZE, crate::lcd_geometry::PIXELS)
        }
        DeviceProfile::Genjintch => (
            crate::genjintch_lcd_geometry::LCD_SIZE,
            crate::genjintch_lcd_geometry::PIXELS,
        ),
        DeviceProfile::Angel => (
            crate::angel_lcd_geometry::LCD_SIZE,
            crate::angel_lcd_geometry::PIXELS,
        ),
        DeviceProfile::Morino => (
            crate::morino_lcd_geometry::LCD_SIZE,
            crate::morino_lcd_geometry::PIXELS,
        ),
        DeviceProfile::Yasashii => (
            crate::yasashii_lcd_geometry::LCD_SIZE,
            crate::yasashii_lcd_geometry::PIXELS,
        ),
        DeviceProfile::Umino => (
            crate::umino_lcd_geometry::LCD_SIZE,
            crate::umino_lcd_geometry::PIXELS,
        ),
        _ => return Err("Color LCD capture requires its native framebuffer".into()),
    };
    let w = size[0].round() as usize;
    let h = size[1].round() as usize;
    let background = [215u8, 225, 204];
    let on = [45u8, 56, 41];
    let off = [207u8, 217, 196];
    let mut rgb = background.repeat(w * h);
    for &(x, y, index, bit) in pixels {
        let color = if display.segment(index as usize, bit) {
            on
        } else {
            off
        };
        for py in (y.round() as usize).min(h)..((y + 10.0).round() as usize).min(h) {
            for px in (x.round() as usize).min(w)..((x + 10.0).round() as usize).min(w) {
                rgb[(py * w + px) * 3..(py * w + px) * 3 + 3].copy_from_slice(&color);
            }
        }
    }
    if profile == DeviceProfile::Yasashii {
        return Image::new(w, h, rgb);
    }
    let source = masks(profile);
    for icon in &source.icons {
        let color = if display.segment(icon.ram as usize, icon.bit) {
            on
        } else {
            off
        };
        let c = coverage(source.data, icon, 1.0);
        for row in 0..c.height {
            for col in 0..c.width {
                let x = c.left + col as i32;
                let y = c.top + row as i32;
                if x < 0 || y < 0 || x >= w as i32 || y >= h as i32 {
                    continue;
                }
                let alpha = c.alpha[row * c.width + col] as u32;
                let i = (y as usize * w + x as usize) * 3;
                for ch in 0..3 {
                    rgb[i + ch] =
                        ((rgb[i + ch] as u32 * (255 - alpha) + color[ch] as u32 * alpha + 127)
                            / 255) as u8;
                }
            }
        }
    }
    Image::new(w, h, rgb)
}
#[cfg(test)]
mod screenshot_tests {
    #[test]
    fn every_classic_profile_exports_only_native_lcd_bounds() {
        use retrohatch::core::{device::DisplaySnapshot, profile::DeviceProfile};
        for p in DeviceProfile::CLASSIC {
            let off = DisplaySnapshot::default();
            let mut on = off.clone();
            on.nibbles = [15; 160];
            let a = super::native_lcd(p, &off).unwrap();
            let b = super::native_lcd(p, &on).unwrap();
            let size = if p == DeviceProfile::Yasashii {
                (351, 175)
            } else {
                (352, 329)
            };
            assert_eq!((a.width, a.height), size);
            assert_eq!(a.rgb.len(), size.0 * size.1 * 3);
            assert_ne!(a.rgb, b.rgb);
            assert!(b.rgb.chunks_exact(3).any(|v| v == [45, 56, 41]));
        }
    }
}
