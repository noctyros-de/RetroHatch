// SPDX-License-Identifier: GPL-3.0-only
// SPDX-FileCopyrightText: 2026 Noctyros
//! Application information and external links; device controls remain in the existing UI.
use super::*;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ExternalLink {
    Website,
    Source,
    Discord,
}
impl ExternalLink {
    pub fn url(self) -> &'static str {
        match self {
            Self::Website => "https://tamatown.de",
            Self::Source => "https://github.com/noctyros-de/RetroHatch",
            Self::Discord => "https://discord.gg/w4zuzCKaZf",
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Action {
    Shortcuts,
    Supported,
    About,
    Link(ExternalLink),
}
#[derive(Default)]
pub(super) struct HelpUi {
    pub supported: bool,
    pub about: bool,
}
impl HelpUi {
    pub fn blocked(&self) -> bool {
        self.supported || self.about
    }
}

pub(super) fn menu(ui: &mut egui::Ui) -> Option<Action> {
    let mut action = None;
    ui.menu_button("Help", |ui| {
        for (label, value) in [
            ("Keyboard Shortcuts", Action::Shortcuts),
            ("Supported Tamagotchis", Action::Supported),
            ("About RetroHatch", Action::About),
            ("Discord Community", Action::Link(ExternalLink::Discord)),
        ] {
            if ui.button(label).clicked() {
                action = Some(value);
                ui.close();
            }
        }
    });
    action
}

pub(super) fn open_link(
    link: ExternalLink,
    opener: impl FnOnce(&str) -> Result<(), String>,
) -> Result<(), String> {
    opener(link.url()).map_err(|error| {
        format!(
            "Could not open {} in your default browser: {error}",
            link.url()
        )
    })
}

pub(super) fn dialogs(ctx: &egui::Context, state: &mut HelpUi) -> Option<ExternalLink> {
    let mut link = None;
    if state.supported {
        egui::Window::new("Supported Tamagotchis")
            .open(&mut state.supported)
            .collapsible(false)
            .resizable(false)
            .default_width(400.0)
            .default_height((DeviceProfile::ALL.len() as f32 + 5.0) * 20.0)
            .max_width((ctx.input(|i| i.content_rect().width()) - 32.0).max(240.0))
            .max_height((ctx.input(|i| i.content_rect().height()) - 64.0).max(200.0))
            .vscroll(true)
            .show(ctx, |ui| {
                ui.strong("Classic devices");
                for profile in DeviceProfile::ALL
                    .into_iter()
                    .filter(|p| !p.uses_framebuffer())
                {
                    ui.label(profile.name());
                }
                ui.separator();
                ui.strong("Connection devices (experimental)");
                ui.label(DeviceProfile::ConnectionV3Eu.name());
                ui.separator();
                ui.strong("Grayscale devices (experimental)");
                ui.label(DeviceProfile::TamaGo.name());
                if DeviceProfile::ALL.iter().any(|p| p.is_color()) {
                    ui.separator();
                    ui.strong("Color devices");
                    for profile in DeviceProfile::ALL
                        .into_iter()
                        .filter(|p| p.is_color() && *p != DeviceProfile::On)
                    {
                        ui.label(profile.name());
                    }
                    ui.separator();
                    ui.strong("Color devices (limited support)");
                    for profile in DeviceProfile::ALL
                        .into_iter()
                        .filter(|p| *p == DeviceProfile::On)
                    {
                        ui.label(profile.name());
                    }
                }
            });
    }
    if state.about {
        egui::Window::new("About RetroHatch").open(&mut state.about)
            .collapsible(false).resizable(false).default_width(430.0)
            .max_width((ctx.input(|i| i.content_rect().width()) - 32.0).max(240.0))
            .max_height((ctx.input(|i| i.content_rect().height()) - 64.0).max(200.0)).vscroll(true)
            .show(ctx, |ui| {
                ui.heading("RetroHatch");
                ui.label("RetroHatch is an open-source Tamagotchi emulator for Windows, developed in Rust.");
                ui.separator();
                ui.label("Creator / Developer: Finn Lülsdorf");
                ui.label("Developer alias: Noctyros");
                ui.label(format!("Version: {VERSION}"));
                ui.label("GNU General Public License v3.0 only (GPL-3.0-only)");
                ui.label("Copyright © 2026 Finn Lülsdorf / Noctyros");
                ui.separator();
                ui.label("Website");
                if ui.link(ExternalLink::Website.url()).clicked() { link = Some(ExternalLink::Website); }
                ui.label("Source Code");
                if ui.link(ExternalLink::Source.url()).clicked() { link = Some(ExternalLink::Source); }
                ui.separator();
                ui.weak("Third-party components retain their own licenses. See THIRD_PARTY_NOTICES.md and the bundled license notices.");
                ui.weak("Unofficial fan project; not affiliated with or endorsed by Bandai.");
            });
    }
    link
}

#[cfg(test)]
mod tests;
