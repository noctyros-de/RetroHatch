// SPDX-License-Identifier: GPL-3.0-only
// SPDX-FileCopyrightText: 2026 Noctyros
//! Deferred renderer capture and bounded background PNG writing; emulation keeps running.
use super::*;
use retrohatch::host::screenshots::{self as files, Image, Kind};
#[derive(Default)]
pub(super) struct Captures {
    pub pending: Option<Kind>,
    rx: Option<mpsc::Receiver<Result<PathBuf, String>>>,
    deadline: Option<Instant>,
    notice: Option<(PathBuf, Instant)>,
}
impl Captures {
    pub fn busy(&self) -> bool {
        self.pending.is_some() || self.rx.is_some()
    }
}
fn write_later(
    root: PathBuf,
    model: String,
    kind: Kind,
    image: Result<Image, String>,
    tx: mpsc::Sender<Result<PathBuf, String>>,
    ctx: egui::Context,
) {
    std::thread::spawn(move || {
        let result = image.and_then(|image| files::save(&root, &model, kind, image));
        let _ = tx.send(result);
        ctx.request_repaint();
    });
}
impl RetroHatchApp {
    pub(super) fn queue_screenshot(&mut self, kind: Kind) {
        if self.media_dir.is_none() {
            self.local_error = Some("Screenshot storage is unavailable".into());
            return;
        }
        if self.captures.busy() {
            self.local_error = Some("Please wait for the current screenshot to finish".into());
            return;
        }
        self.captures.notice = None;
        self.captures.pending = Some(kind);
    }
    pub(super) fn poll_screenshots(&mut self, ctx: &egui::Context, s: &Snapshot) {
        if let Some(rx) = &self.captures.rx {
            match rx.try_recv() {
                Ok(result) => {
                    self.captures.rx = None;
                    self.captures.deadline = None;
                    match result {
                        Ok(path) => {
                            self.captures.notice =
                                Some((path, Instant::now() + Duration::from_secs(6)))
                        }
                        Err(error) => self.local_error = Some(error),
                    }
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.captures.rx = None;
                    self.local_error = Some("Screenshot writer stopped unexpectedly".into());
                }
                Err(mpsc::TryRecvError::Empty) => {
                    if self.captures.deadline.is_some_and(|t| Instant::now() > t) {
                        self.captures.rx = None;
                        self.local_error = Some(
                            "Screenshot capture timed out; keep the window visible and try again"
                                .into(),
                        );
                    }
                }
            }
        }
        let Some(kind) = self.captures.pending.take() else {
            return;
        };
        let Some(root) = self.media_dir.clone() else {
            return;
        };
        let model = if s.loaded {
            s.profile.id()
        } else {
            "retrohatch"
        }
        .to_string();
        let (tx, rx) = mpsc::channel();
        self.captures.rx = Some(rx);
        self.captures.deadline = Some(Instant::now() + Duration::from_secs(15));
        match kind {
            Kind::Lcd => {
                if !s.loaded {
                    let _ = tx.send(Err(
                        "Load a Tamagotchi before taking an LCD screenshot".into()
                    ));
                    return;
                }
                let color = s.color_display.clone();
                let display = s.display.clone();
                let profile = s.profile;
                let ctx = ctx.clone();
                std::thread::spawn(move || {
                    let image = if profile.is_color() {
                        color
                            .ok_or("The native color framebuffer is unavailable".into())
                            .and_then(|rgb| Image::new(128, 128, rgb.as_ref().clone()))
                    } else {
                        crate::lcd_render::native_lcd(profile, &display)
                    };
                    let result = image.and_then(|image| files::save(&root, &model, kind, image));
                    let _ = tx.send(result);
                    ctx.request_repaint();
                });
            }
            Kind::Full => {
                #[cfg(windows)]
                let window = match files::native::window() {
                    Ok(h) => h,
                    Err(error) => {
                        let _ = tx.send(Err(error));
                        return;
                    }
                };
                let repaint = ctx.clone();
                ctx.request_screenshot(move |pixels| {
                    let rgb = pixels
                        .pixels
                        .iter()
                        .flat_map(|p| {
                            let [r, g, b, _] = p.to_array();
                            [r, g, b]
                        })
                        .collect();
                    let image = Image::new(pixels.size[0], pixels.size[1], rgb);
                    #[cfg(windows)]
                    let image = image.and_then(|image| files::native::compose(window, image));
                    write_later(root, model, kind, image, tx, repaint);
                });
            }
        }
    }
    pub(super) fn screenshot_notice(&mut self, ctx: &egui::Context) {
        let Some((path, until)) = &self.captures.notice else {
            return;
        };
        if Instant::now() >= *until {
            self.captures.notice = None;
            return;
        }
        egui::Area::new(egui::Id::unique("screenshot-confirmation"))
            .anchor(egui::Align2::RIGHT_BOTTOM, egui::vec2(-12., -12.))
            .show(ctx, |ui| {
                egui::Frame::popup(ui.style()).show(ui, |ui| {
                    ui.set_max_width(310.);
                    ui.strong("Screenshot saved");
                    ui.small(path.file_name().unwrap_or_default().to_string_lossy())
                        .on_hover_text(path.display().to_string());
                });
            });
    }
}
