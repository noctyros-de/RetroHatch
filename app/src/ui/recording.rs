// SPDX-License-Identifier: GPL-3.0-only
// SPDX-FileCopyrightText: 2026 Noctyros
//! Capture menus and frame acquisition; all MP4 encoding stays on the host thread.
use super::*;
use retrohatch::host::{
    recording::Session,
    screenshots::{self as files, Image, Kind},
};
#[derive(Default)]
pub(super) struct RecordingUi {
    pending: Option<Kind>,
    session: Option<Session>,
    frames: Option<mpsc::Receiver<Result<Image, String>>>,
    frame_tx: Option<mpsc::Sender<Result<Image, String>>>,
    in_flight: bool,
    next: Option<Instant>,
    window: Option<usize>,
    lcd: Option<(DeviceProfile, [u8; 160], Image)>,
    notice: Option<(String, Instant)>,
}
impl RecordingUi {
    pub fn busy(&self) -> bool {
        self.pending.is_some() || self.session.is_some()
    }
}
impl RetroHatchApp {
    pub(super) fn capture_menu(&mut self, ui: &mut egui::Ui, s: &Snapshot) {
        ui.menu_button("Screenshots", |ui| {
            for (label, kind, enabled) in [
                ("Take LCD Screenshot", Kind::Lcd, s.loaded && !s.loading),
                ("Take Emulator Screenshot", Kind::Full, true),
            ] {
                if ui
                    .add_enabled(
                        enabled
                            && self.media_dir.is_some()
                            && !self.captures.busy()
                            && !self.exit_pending,
                        egui::Button::new(label).shortcut_text(if kind == Kind::Lcd {
                            "F8"
                        } else {
                            "F9"
                        }),
                    )
                    .clicked()
                {
                    self.queue_screenshot(kind);
                    ui.close();
                }
            }
            for (label, kind) in [
                ("Open LCD Folder", Kind::Lcd),
                ("Open Emulator Folder", Kind::Full),
            ] {
                if ui
                    .add_enabled(self.media_dir.is_some(), egui::Button::new(label))
                    .clicked()
                {
                    if let Some(root) = &self.media_dir {
                        if let Err(e) = platform::open_folder(&files::folder(root, kind)) {
                            self.local_error = Some(e);
                        }
                    }
                    ui.close();
                }
            }
            ui.separator();
            if self.recording.busy() {
                if ui
                    .button("Stop Recording")
                    .on_hover_text("Finish and save the MP4 video. F10 also stops recording.")
                    .clicked()
                {
                    self.stop_recording();
                    ui.close();
                }
            } else {
                for (label, kind, enabled) in [
                    ("Start Emulator Recording", Kind::Full, true),
                    ("Start LCD Recording", Kind::Lcd, s.loaded && !s.loading),
                ] {
                    if ui
                        .add_enabled(
                            enabled && self.media_dir.is_some() && !self.exit_pending,
                            egui::Button::new(label).shortcut_text(if kind == Kind::Full {
                                "F10"
                            } else {
                                ""
                            }),
                        )
                        .on_hover_text(
                            "Native MP4 / H.264, 30 FPS. Video only: game audio is not recorded.",
                        )
                        .clicked()
                    {
                        self.start_recording(kind, s);
                        ui.close();
                    }
                }
            }
            if ui
                .add_enabled(
                    self.media_dir.is_some(),
                    egui::Button::new("Open Recordings Folder"),
                )
                .clicked()
            {
                if let Some(root) = &self.media_dir {
                    if let Err(e) = platform::open_folder(&root.join("Recordings")) {
                        self.local_error = Some(e);
                    }
                }
                ui.close();
            }
        });
    }
    pub(super) fn start_recording(&mut self, kind: Kind, _s: &Snapshot) {
        if !self.recording.busy() && self.media_dir.is_some() {
            self.recording.pending = Some(kind);
        }
    }
    pub(super) fn stop_recording(&mut self) {
        self.recording.pending = None;
        if let Some(session) = &mut self.recording.session {
            session.stop();
        }
    }
    pub(super) fn poll_recording(&mut self, ctx: &egui::Context, s: &Snapshot) {
        if let Some(result) = self.recording.session.as_ref().and_then(Session::poll) {
            self.recording.session = None;
            self.recording.frames = None;
            self.recording.frame_tx = None;
            self.recording.in_flight = false;
            self.recording.lcd = None;
            match result {
                Ok(done) => {
                    self.recording.notice = Some((
                        format!(
                            "Recording saved: {}\n{:.1}s, {} frames; {} repeated, {} input frames dropped.\nVideo only (no audio).",
                            done.path.display(),
                            done.elapsed.as_secs_f64(),
                            done.frames,
                            done.repeated,
                            done.dropped
                        ),
                        Instant::now() + Duration::from_secs(12),
                    ))
                }
                Err(e) => self.local_error = Some(e),
            }
        }
        if let Some(kind) = self.recording.pending.take() {
            let Some(root) = &self.media_dir else {
                return;
            };
            #[cfg(windows)]
            if kind == Kind::Full {
                match files::native::window() {
                    Ok(h) => self.recording.window = Some(h),
                    Err(e) => {
                        self.local_error = Some(e);
                        return;
                    }
                }
            }
            let model = if s.loaded {
                s.profile.id()
            } else {
                "retrohatch"
            };
            match Session::start(root, model, kind) {
                Ok(session) => {
                    self.recording.session = Some(session);
                    let (tx, rx) = mpsc::channel();
                    self.recording.frames = Some(rx);
                    self.recording.frame_tx = Some(tx);
                    self.recording.next = None;
                    self.recording.notice = None;
                }
                Err(e) => self.local_error = Some(e),
            }
        }
        if let Some(frame) = self
            .recording
            .frames
            .as_ref()
            .and_then(|rx| rx.try_recv().ok())
        {
            self.recording.in_flight = false;
            match frame {
                Ok(image) => {
                    if let Some(session) = &self.recording.session {
                        session.submit(image);
                    }
                }
                Err(e) if e.contains("changed size") => {} // A resize race drops one capture; the next uses current bounds.
                Err(e) => {
                    self.local_error = Some(e);
                    self.stop_recording();
                }
            }
        }
        let Some(session) = &self.recording.session else {
            return;
        };
        if session.stopping
            || self.recording.in_flight
            || self.recording.next.is_some_and(|t| Instant::now() < t)
        {
            return;
        }
        self.recording.next = Some(Instant::now() + Duration::from_secs_f64(1.0 / 30.0));
        match session.kind {
            Kind::Lcd => {
                if !s.loaded {
                    self.local_error = Some("The LCD recording device was unloaded".into());
                    self.stop_recording();
                    return;
                }
                let image =
                    if s.profile.is_color() {
                        s.color_display
                            .as_ref()
                            .ok_or("Native color display unavailable".into())
                            .and_then(|rgb| Image::new(128, 128, rgb.as_ref().clone()))
                    } else {
                        if !self.recording.lcd.as_ref().is_some_and(|(p, bits, _)| {
                            *p == s.profile && *bits == s.display.nibbles
                        }) {
                            match crate::lcd_render::native_lcd(s.profile, &s.display) {
                                Ok(image) => {
                                    self.recording.lcd = Some((s.profile, s.display.nibbles, image))
                                }
                                Err(e) => {
                                    self.local_error = Some(e);
                                    self.stop_recording();
                                    return;
                                }
                            }
                        }
                        Ok(self.recording.lcd.as_ref().unwrap().2.clone())
                    };
                match image {
                    Ok(image) => session.submit(image),
                    Err(e) => {
                        self.local_error = Some(e);
                        self.stop_recording();
                    }
                }
            }
            Kind::Full => {
                #[cfg(windows)]
                let window = self.recording.window.unwrap();
                #[cfg(windows)]
                if !files::native::visible(window) {
                    return;
                }
                let tx = self.recording.frame_tx.as_ref().unwrap().clone();
                let repaint = ctx.clone();
                self.recording.in_flight = true;
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
                    let _ = tx.send(image);
                    repaint.request_repaint();
                });
            }
        }
    }
    pub(super) fn recording_notice(&self, ctx: &egui::Context) {
        if let Some(session) = &self.recording.session {
            let seconds = session.started.elapsed().as_secs();
            egui::Area::new(egui::Id::unique("recording-status"))
                .anchor(egui::Align2::LEFT_BOTTOM, egui::vec2(10., -10.))
                .show(ctx, |ui| {
                    egui::Frame::popup(ui.style()).show(ui, |ui| {
                        ui.colored_label(
                            egui::Color32::from_rgb(170, 35, 45),
                            format!(
                                "{}  {:02}:{:02}",
                                if session.stopping {
                                    "Finalizing"
                                } else {
                                    "REC"
                                },
                                seconds / 60,
                                seconds % 60
                            ),
                        );
                        ui.small("MP4 video only - no audio");
                    });
                });
        } else if let Some((notice, until)) = &self.recording.notice {
            if Instant::now() < *until {
                egui::Area::new(egui::Id::unique("recording-completed"))
                    .anchor(egui::Align2::LEFT_BOTTOM, egui::vec2(10., -10.))
                    .show(ctx, |ui| {
                        egui::Frame::popup(ui.style()).show(ui, |ui| {
                            ui.set_max_width(390.);
                            ui.small(notice);
                        });
                    });
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn labels(out: &egui::FullOutput) -> Vec<(String, egui::Pos2)> {
        fn walk(s: &egui::epaint::Shape, out: &mut Vec<(String, egui::Pos2)>) {
            match s {
                egui::epaint::Shape::Text(t) => {
                    out.push((t.galley.text().into(), t.pos + t.galley.size() / 2.))
                }
                egui::epaint::Shape::Vec(shapes) => {
                    for s in shapes {
                        walk(s, out)
                    }
                }
                _ => {}
            }
        }
        let mut labels = vec![];
        for s in &out.shapes {
            walk(&s.shape, &mut labels)
        }
        labels
    }
    fn frame(
        ctx: &egui::Context,
        app: &mut RetroHatchApp,
        events: Vec<egui::Event>,
    ) -> egui::FullOutput {
        let mut out = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(760., 800.),
                )),
                events,
                ..Default::default()
            },
            |ui| app.menus(ui, &Snapshot::default()),
        );
        out.textures_delta.clear();
        out
    }
    fn click(
        ctx: &egui::Context,
        app: &mut RetroHatchApp,
        out: &egui::FullOutput,
        label: &str,
    ) -> egui::FullOutput {
        let pos = labels(out)
            .into_iter()
            .find(|(s, _)| s == label)
            .unwrap_or_else(|| panic!("Missing {label}"))
            .1;
        for pressed in [true, false] {
            frame(
                ctx,
                app,
                vec![
                    egui::Event::PointerMoved(pos),
                    egui::Event::PointerButton {
                        pos,
                        button: egui::PointerButton::Primary,
                        pressed,
                        modifiers: egui::Modifiers::NONE,
                    },
                ],
            );
        }
        frame(ctx, app, vec![])
    }
    #[test]
    fn capture_actions_live_in_one_menu_and_recording_can_be_stopped_before_capture() {
        let ctx = egui::Context::default();
        let mut app = RetroHatchApp::create(false);
        app.media_dir = Some(std::env::temp_dir());
        frame(&ctx, &mut app, vec![]);
        let out = frame(&ctx, &mut app, vec![]);
        let out = click(&ctx, &mut app, &out, "File");
        assert!(
            !labels(&out)
                .iter()
                .any(|(s, _)| s == "Take LCD Screenshot" || s == "Take Emulator Screenshot")
        );
        egui::Popup::close_all(&ctx);
        let out = frame(&ctx, &mut app, vec![]);
        let out = click(&ctx, &mut app, &out, "Screenshots");
        for label in [
            "Take LCD Screenshot",
            "Take Emulator Screenshot",
            "Open LCD Folder",
            "Open Emulator Folder",
            "Start Emulator Recording",
            "Start LCD Recording",
            "Open Recordings Folder",
        ] {
            assert!(
                labels(&out).iter().any(|(s, _)| s == label),
                "Missing {label}"
            );
        }
        click(&ctx, &mut app, &out, "Start Emulator Recording");
        assert!(app.recording.busy());
        let out = frame(&ctx, &mut app, vec![]);
        let out = click(&ctx, &mut app, &out, "Screenshots");
        click(&ctx, &mut app, &out, "Stop Recording");
        assert!(!app.recording.busy());
    }
    #[test]
    fn all_display_scales_render_for_classic_and_color_without_scaling_ui_buttons() {
        for profile in [DeviceProfile::P1, DeviceProfile::Ps] {
            for scale in DISPLAY_SCALES {
                let ctx = egui::Context::default();
                let mut app = RetroHatchApp::create(false);
                app.selected_profile = profile;
                app.settings.display_scale = scale;
                let mut out = ctx.run_ui(
                    egui::RawInput {
                        screen_rect: Some(egui::Rect::from_min_size(
                            egui::Pos2::ZERO,
                            egui::vec2(760., 800.),
                        )),
                        ..Default::default()
                    },
                    |ui| app.content(ui),
                );
                assert!(labels(&out).iter().any(|(s, _)| s == "A"));
                assert!(out.shapes.iter().any(|s|matches!(&s.shape,egui::epaint::Shape::Circle(c) if (c.radius-24.).abs()<0.01)));
                out.textures_delta.clear();
            }
        }
    }
}
