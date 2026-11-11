// SPDX-License-Identifier: GPL-3.0-only
// SPDX-FileCopyrightText: 2026 Noctyros
//! Exercise the actual egui menus/dialogs without launching a browser during ordinary tests.
use super::*;
use retrohatch::host::library::{Entry, Key};

fn render(
    ctx: &egui::Context,
    events: Vec<egui::Event>,
    draw: impl FnMut(&mut egui::Ui),
) -> egui::FullOutput {
    let mut out = ctx.run_ui(input(events), draw);
    // Headless checks inspect shapes and actions without a GPU texture uploader.
    out.textures_delta.clear();
    out
}

fn texts(out: &egui::FullOutput) -> Vec<(String, egui::Pos2)> {
    fn walk(shape: &egui::epaint::Shape, found: &mut Vec<(String, egui::Pos2)>) {
        match shape {
            egui::epaint::Shape::Text(text) => found.push((
                text.galley.text().into(),
                text.pos + text.galley.size() / 2.0,
            )),
            egui::epaint::Shape::Vec(shapes) => {
                for shape in shapes {
                    walk(shape, found)
                }
            }
            _ => {}
        }
    }
    let mut found = vec![];
    for shape in &out.shapes {
        walk(&shape.shape, &mut found)
    }
    found
}
fn position(out: &egui::FullOutput, label: &str) -> egui::Pos2 {
    texts(out)
        .iter()
        .find(|(text, _)| text == label)
        .unwrap_or_else(|| panic!("Missing {label}: {:?}", texts(out)))
        .1
}
fn input(events: Vec<egui::Event>) -> egui::RawInput {
    egui::RawInput {
        focused: true,
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(760.0, 800.0),
        )),
        events,
        ..Default::default()
    }
}
fn pointer(pos: egui::Pos2, pressed: bool) -> Vec<egui::Event> {
    vec![
        egui::Event::PointerMoved(pos),
        egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        },
    ]
}
fn menu_frame(ctx: &egui::Context, events: Vec<egui::Event>) -> (egui::FullOutput, Option<Action>) {
    let mut action = None;
    let out = render(&ctx, events, |ui| {
        egui::MenuBar::new().ui(ui, |ui| action = menu(ui));
    });
    (out, action)
}
fn menu_click(ctx: &egui::Context, pos: egui::Pos2) -> (egui::FullOutput, Option<Action>) {
    let (_, first) = menu_frame(ctx, pointer(pos, true));
    let (_, second) = menu_frame(ctx, pointer(pos, false));
    let (out, last) = menu_frame(ctx, vec![]);
    (out, first.or(second).or(last))
}

#[test]
fn help_menu_dispatches_all_four_entries() {
    for (label, expected) in [
        ("Keyboard Shortcuts", Action::Shortcuts),
        ("Supported Tamagotchis", Action::Supported),
        ("About RetroHatch", Action::About),
        ("Discord Community", Action::Link(ExternalLink::Discord)),
    ] {
        let ctx = egui::Context::default();
        menu_frame(&ctx, vec![]);
        let (out, _) = menu_frame(&ctx, vec![]);
        let (out, _) = menu_click(&ctx, position(&out, "Help"));
        for name in [
            "Keyboard Shortcuts",
            "Supported Tamagotchis",
            "About RetroHatch",
            "Discord Community",
        ] {
            position(&out, name);
        }
        let (_, action) = menu_click(&ctx, position(&out, label));
        assert_eq!(action, Some(expected));
    }
}

#[test]
fn about_metadata_profile_list_and_links_render_from_the_current_implementation() {
    let ctx = egui::Context::default();
    let mut state = HelpUi {
        about: true,
        supported: false,
    };
    let frame = |events, state: &mut HelpUi| {
        let mut link = None;
        let out = render(&ctx, events, |_| link = dialogs(&ctx, state));
        (out, link)
    };
    frame(vec![], &mut state);
    let (out, _) = frame(vec![], &mut state);
    for label in [
        "RetroHatch",
        "Creator / Developer: Finn Lülsdorf",
        "Developer alias: Noctyros",
        &format!("Version: {}", env!("CARGO_PKG_VERSION")),
        "GNU General Public License v3.0 only (GPL-3.0-only)",
        ExternalLink::Website.url(),
        ExternalLink::Source.url(),
    ] {
        position(&out, label);
    }
    assert!(
        texts(&out)
            .iter()
            .any(|(text, _)| text.contains("THIRD_PARTY_LICENSES.md"))
    );
    for expected in [ExternalLink::Website, ExternalLink::Source] {
        let (out, _) = frame(vec![], &mut state);
        let pos = position(&out, expected.url());
        frame(pointer(pos, true), &mut state);
        let (_, link) = frame(pointer(pos, false), &mut state);
        assert_eq!(link, Some(expected));
    }
    state = HelpUi {
        about: false,
        supported: true,
    };
    frame(vec![], &mut state);
    let (out, _) = frame(vec![], &mut state);
    for profile in DeviceProfile::ALL {
        position(&out, profile.name());
    }
    position(&out, "Color devices (limited support)");
    let mut allowed = vec![
        "Supported Tamagotchis",
        "Classic devices",
        "Color devices",
        "Color devices (limited support)",
    ];
    allowed.extend(DeviceProfile::ALL.into_iter().map(DeviceProfile::name));
    for (text, _) in texts(&out) {
        assert!(
            allowed.contains(&text.as_str()),
            "Unexpected device-selection explanation: {text}"
        );
    }
    assert!(
        !texts(&out)
            .iter()
            .any(|(text, _)| text.contains("TamaOtch"))
    );
}

#[test]
fn external_actions_use_exact_urls_and_return_useful_failures() {
    for (link, url) in [
        (ExternalLink::Website, "https://tamatown.de"),
        (
            ExternalLink::Source,
            "https://github.com/noctyros-de/RetroHatch",
        ),
        (ExternalLink::Discord, "https://discord.gg/w4zuzCKaZf"),
    ] {
        open_link(link, |requested| {
            assert_eq!(requested, url);
            Ok(())
        })
        .unwrap();
        let error = open_link(link, |_| Err("No browser is configured".into())).unwrap_err();
        assert!(error.contains(url) && error.contains("No browser is configured"));
    }
}

#[test]
fn shortcuts_move_to_help_and_existing_dialog_is_reused() {
    fn frame(
        ctx: &egui::Context,
        app: &mut RetroHatchApp,
        events: Vec<egui::Event>,
    ) -> egui::FullOutput {
        render(&ctx, events, |ui| app.content(ui))
    }
    fn click(ctx: &egui::Context, app: &mut RetroHatchApp, pos: egui::Pos2) -> egui::FullOutput {
        frame(ctx, app, pointer(pos, true));
        frame(ctx, app, pointer(pos, false));
        frame(ctx, app, vec![])
    }
    let ctx = egui::Context::default();
    let mut app = RetroHatchApp::create(false);
    frame(&ctx, &mut app, vec![]);
    let out = frame(&ctx, &mut app, vec![]);
    let out = click(&ctx, &mut app, position(&out, "Settings"));
    assert!(
        !texts(&out)
            .iter()
            .any(|(text, _)| text == "Keyboard Shortcuts")
    );
    let ctx = egui::Context::default();
    let mut app = RetroHatchApp::create(false);
    frame(&ctx, &mut app, vec![]);
    let out = frame(&ctx, &mut app, vec![]);
    let out = click(&ctx, &mut app, position(&out, "Help"));
    let out = click(&ctx, &mut app, position(&out, "Keyboard Shortcuts"));
    assert!(app.shortcuts_open);
    for label in [
        "A: Left Arrow, A or 1",
        "B: Down Arrow, B or 2",
        "C: Right Arrow, C or 3",
        "Pause / Resume: Space",
        "Reset (confirmation): F5",
    ] {
        position(&out, label);
    }
}

#[test]
fn rom_menu_hides_variants_hashes_and_current_prefix_without_mutating_entries() {
    let mut app = RetroHatchApp::create(false);
    let entry = Entry {
        key: Key {
            hash: "ab".repeat(32),
            profile: DeviceProfile::P1,
        },
        variant: "Japanese / Region not verified".into(),
        evidence: "Original fingerprint evidence".into(),
        filename: "private-internal.bin".into(),
        source_name: "source.bin".into(),
        size: 12288,
        imported: 1,
        listed: true,
        problem: None,
    };
    app.library.entries = vec![entry.clone()];
    let snapshot = Snapshot {
        loaded: true,
        profile: DeviceProfile::P1,
        rom_hash: Some([0xab; 32]),
        ..Default::default()
    };
    let ctx = egui::Context::default();
    let mut frame = |events| {
        render(&ctx, events, |ui| {
            egui::MenuBar::new().ui(ui, |ui| app.library_menu(ui, &snapshot));
        })
    };
    frame(vec![]);
    let out = frame(vec![]);
    let pos = position(&out, "My Tamagotchis");
    frame(pointer(pos, true));
    frame(pointer(pos, false));
    let out = frame(vec![]);
    position(&out, DeviceProfile::P1.name());
    for (text, _) in texts(&out) {
        assert!(
            !text.contains("Region")
                && !text.contains("Japanese")
                && !text.contains("abababab")
                && !text.contains("Current:"),
            "Unexpected menu metadata: {text}"
        );
    }
    assert_eq!(app.library.entries, [entry]);
}
