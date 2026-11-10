// SPDX-License-Identifier: GPL-3.0-only
// SPDX-FileCopyrightText: 2026 Noctyros
//! Pixels decoded from the optional official ICO; no artwork generation or substitution.
pub fn icon() -> Option<eframe::egui::IconData> {
    let png = include_bytes!(concat!(env!("OUT_DIR"), "/icon.png"));
    if png.is_empty() {
        None
    } else {
        eframe::icon_data::from_png_bytes(png).ok()
    }
}
#[cfg(test)]
mod tests {
    #[test]
    fn optional_icon_is_valid_or_absent() {
        if let Some(icon) = super::icon() {
            assert!(icon.width > 0 && icon.height > 0);
            assert_eq!(icon.rgba.len(), (icon.width * icon.height * 4) as usize);
        }
    }
}
