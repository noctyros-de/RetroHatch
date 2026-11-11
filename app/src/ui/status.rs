// SPDX-License-Identifier: GPL-3.0-only
// SPDX-FileCopyrightText: 2026 Noctyros
//! Fixed footer for action feedback; messages never participate in menu/LCD layout.
use super::*;

impl RetroHatchApp {
    pub(super) fn show_status(
        &mut self,
        message: impl Into<String>,
        details: Option<String>,
        duration: Duration,
    ) {
        self.last_message = Some(message.into());
        self.message_details = details;
        self.message_until = Instant::now() + duration;
    }

    pub(super) fn sync_save_notice(&mut self, s: &Snapshot) {
        if s.save_message != self.last_save_message
            || self.message_serial != s.save_count + s.restores
        {
            self.message_serial = s.save_count + s.restores;
            self.last_save_message = s.save_message.clone();
            if let Some(message) = &s.save_message {
                self.show_status(message.clone(), None, Duration::from_secs(4));
            }
        }
    }

    pub(super) fn status_footer(&mut self, ui: &mut egui::Ui, s: &Snapshot) {
        if Instant::now() >= self.message_until {
            self.last_message = None;
            self.message_details = None;
        }
        let error = self
            .local_error
            .as_deref()
            .or(s.save_error.as_deref())
            .or(s.error.as_deref());
        let busy = if s.saving {
            Some("Saving progress...")
        } else if s.loading {
            Some("Loading ROM...")
        } else if self.library.busy() {
            Some("Updating library...")
        } else if self.captures.busy() {
            Some("Saving screenshot...")
        } else {
            None
        };
        let text = error
            .or(busy)
            .or(self.last_message.as_deref())
            .unwrap_or("");
        let recording = self.recording_status();
        let text = match &recording {
            Some(recording) if !text.is_empty() => format!("{recording} | {text}"),
            Some(recording) => format!("{recording} | Video only (no audio)"),
            None => text.to_owned(),
        };
        let details = if error.is_some() || busy.is_some() {
            text.clone()
        } else {
            self.message_details.clone().unwrap_or_else(|| text.clone())
        };
        let color = if error.is_some() {
            ui.visuals().error_fg_color
        } else {
            ui.visuals().weak_text_color()
        };
        egui::Panel::bottom("action-status-footer")
            .exact_size(52.0)
            .show_separator_line(false)
            .frame(
                egui::Frame::NONE
                    .fill(palette(self.settings.theme).0)
                    .inner_margin(egui::Margin::symmetric(8, 4)),
            )
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 2.0;
                ui.vertical_centered(|ui| {
                    ui.weak("Left = A    Down = B    Right = C");
                    ui.add_sized(
                        [ui.available_width(), 22.0],
                        egui::Label::new(
                            egui::RichText::new(text.replace(['\r', '\n'], " "))
                                .small()
                                .color(color),
                        )
                        .truncate()
                        .halign(egui::Align::Center),
                    )
                    .on_hover_text(details);
                });
            });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn frame(
        ctx: &egui::Context,
        app: &mut RetroHatchApp,
        size: egui::Vec2,
        ppp: f32,
    ) -> egui::FullOutput {
        let mut input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, size)),
            focused: true,
            ..Default::default()
        };
        input
            .viewports
            .get_mut(&egui::ViewportId::ROOT)
            .unwrap()
            .native_pixels_per_point = Some(ppp);
        let mut out = ctx.run_ui(input, |ui| app.content(ui));
        out.textures_delta.clear();
        out
    }
    fn text_rect(out: &egui::FullOutput, label: &str) -> egui::Rect {
        fn find(shape: &egui::epaint::Shape, label: &str) -> Option<egui::Rect> {
            match shape {
                egui::epaint::Shape::Text(t) if t.galley.text() == label => {
                    Some(t.galley.rect.translate(t.pos.to_vec2()))
                }
                egui::epaint::Shape::Vec(shapes) => shapes.iter().find_map(|s| find(s, label)),
                _ => None,
            }
        }
        out.shapes
            .iter()
            .find_map(|s| find(&s.shape, label))
            .unwrap_or_else(|| panic!("Missing visible text: {label}"))
    }
    fn lcd_rect(out: &egui::FullOutput, profile: DeviceProfile) -> egui::Rect {
        let color = if profile.is_color() {
            egui::Color32::BLACK
        } else {
            egui::Color32::from_rgb(215, 225, 204)
        };
        fn find(shape: &egui::epaint::Shape, color: egui::Color32) -> Option<egui::Rect> {
            match shape {
                egui::epaint::Shape::Rect(r)
                    if r.fill == color && r.rect.width() > 50. && r.rect.height() > 40. =>
                {
                    Some(r.rect)
                }
                egui::epaint::Shape::Vec(v) => v.iter().find_map(|s| find(s, color)),
                _ => None,
            }
        }
        out.shapes
            .iter()
            .find_map(|s| find(&s.shape, color))
            .expect("LCD surface must be rendered")
    }
    #[test]
    fn notifications_keep_menus_lcd_controls_and_footer_stable_at_small_sizes_and_scales() {
        for (profile, size, ppp, scale) in [
            (DeviceProfile::P1, egui::vec2(350., 500.), 1., 100),
            (DeviceProfile::Id, egui::vec2(460., 660.), 2., 100),
            (DeviceProfile::Angel, egui::vec2(350., 500.), 2., 33),
            (DeviceProfile::Id, egui::vec2(350., 500.), 2., 300),
        ] {
            let ctx = egui::Context::default();
            let mut app = RetroHatchApp::create(false);
            app.initial_scale_applied = true;
            app.selected_profile = profile;
            app.settings.display_scale = scale;
            frame(&ctx, &mut app, size, ppp);
            let baseline = frame(&ctx, &mut app, size, ppp);
            let labels = ["File", "A", "B", "C", "Left = A    Down = B    Right = C"];
            let positions = labels.map(|label| text_rect(&baseline, label));
            let lcd = lcd_rect(&baseline, profile);
            for message in [
                "Saved current progress",
                "Screenshot saved",
                "Recording saved",
                "Imported Tamagotchi iD",
            ] {
                app.show_status(
                    message,
                    Some("Full action details".into()),
                    Duration::from_secs(4),
                );
                let out = frame(&ctx, &mut app, size, ppp);
                for (label, expected) in labels.iter().zip(positions) {
                    assert_eq!(
                        text_rect(&out, label),
                        expected,
                        "{profile:?}, {scale}%: {label} moved"
                    );
                }
                assert_eq!(
                    lcd_rect(&out, profile),
                    lcd,
                    "LCD moved with a notification"
                );
                let notice = text_rect(&out, message);
                assert!(notice.top() > positions[4].bottom());
                assert!(egui::Rect::from_min_size(egui::Pos2::ZERO, size).contains_rect(notice));
                assert!(notice.height() <= 22.0);
                assert!(
                    !out.viewport_output
                        .values()
                        .flat_map(|v| &v.commands)
                        .any(|c| matches!(c, egui::ViewportCommand::InnerSize(_)))
                );
            }
            app.show_status(
                "Saved a very long Unicode filename ".repeat(15) + "Ω 日本語",
                None,
                Duration::from_secs(4),
            );
            let out = frame(&ctx, &mut app, size, ppp);
            assert_eq!(lcd_rect(&out, profile), lcd);
            let long = text_rect(&out, app.last_message.as_ref().unwrap());
            assert!(egui::Rect::from_min_size(egui::Pos2::ZERO, size).contains_rect(long));
            assert!(long.height() <= 22.0);
            for (label, expected) in labels.iter().zip(positions) {
                assert_eq!(text_rect(&out, label), expected);
            }
            app.message_until = Instant::now() - Duration::from_secs(1);
            let out = frame(&ctx, &mut app, size, ppp);
            assert!(app.last_message.is_none());
            for (label, expected) in labels.iter().zip(positions) {
                assert_eq!(text_rect(&out, label), expected);
            }
        }
    }
    #[test]
    fn stale_save_snapshot_does_not_replace_newer_action_feedback() {
        let mut app = RetroHatchApp::create(false);
        let mut snapshot = Snapshot {
            save_message: Some("Saved current progress".into()),
            save_count: 1,
            ..Default::default()
        };
        app.sync_save_notice(&snapshot);
        for message in [
            "Imported Tamagotchi iD",
            "Screenshot saved",
            "Recording saved",
        ] {
            app.show_status(message, Some("Details".into()), Duration::from_secs(4));
            app.sync_save_notice(&snapshot);
            assert_eq!(app.last_message.as_deref(), Some(message));
        }
        snapshot.save_count += 1;
        app.sync_save_notice(&snapshot);
        assert_eq!(app.last_message.as_deref(), Some("Saved current progress"));
    }
    #[test]
    fn busy_and_error_feedback_stay_below_the_hint_and_out_of_the_menu_bar() {
        let ctx = egui::Context::default();
        let mut app = RetroHatchApp::create(false);
        let draw = |app: &mut RetroHatchApp, s: &Snapshot| {
            let mut out = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(350., 500.),
                    )),
                    ..Default::default()
                },
                |ui| {
                    app.menus(ui, s);
                    app.status_footer(ui, s);
                },
            );
            out.textures_delta.clear();
            out
        };
        draw(&mut app, &Snapshot::default());
        let idle = draw(&mut app, &Snapshot::default());
        let menu = text_rect(&idle, "File");
        let busy = draw(
            &mut app,
            &Snapshot {
                saving: true,
                ..Default::default()
            },
        );
        assert_eq!(text_rect(&busy, "File"), menu);
        let hint = text_rect(&busy, "Left = A    Down = B    Right = C");
        assert!(text_rect(&busy, "Saving progress...").top() > hint.bottom());
        app.local_error = Some("Cannot write save".into());
        let error = draw(&mut app, &Snapshot::default());
        assert_eq!(text_rect(&error, "File"), menu);
        assert!(text_rect(&error, "Cannot write save").top() > hint.bottom());
        fn error_color(shape: &egui::epaint::Shape) -> Option<egui::Color32> {
            match shape {
                egui::epaint::Shape::Text(t) if t.galley.text() == "Cannot write save" => Some(
                    t.override_text_color
                        .unwrap_or(t.galley.job.sections[0].format.color),
                ),
                egui::epaint::Shape::Vec(v) => v.iter().find_map(error_color),
                _ => None,
            }
        }
        assert_eq!(
            error.shapes.iter().find_map(|s| error_color(&s.shape)),
            Some(ctx.global_style().visuals.error_fg_color)
        );
    }
}
