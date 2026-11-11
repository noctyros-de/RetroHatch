<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- SPDX-FileCopyrightText: 2026 Noctyros -->
# Branding and screenshots

## Asset handling and builds

The optional sibling `../local/resources/` contains the unmodified owner-supplied
PNG/ICO, outside Git. RETROHATCH_RESOURCES_DIR can select another private directory.
Only the byte-identical PNG copy at `docs/assets/retrohatchlogo.png` is public.
Private research moved outside the repository; public docs/assets and the
separately approved docs/screenshots/ gallery remain public. Release source/package allowlists include the logo and build.rs,
and continue rejecting private resources and screenshots. No new artwork is made.

`app/build.rs` optionally compiles the ICO as Windows resource group 101 with
Windows SDK rc.exe, discovered from RC/PATH/the installed SDK. It stages copies
only in OUT_DIR. The supplied ICO contains one PNG-backed 256×256 image; that
original PNG payload is also embedded for eframe IconData. Windows/eframe then
set small and large window icons for the caption, taskbar and Alt+Tab.
No new Cargo package or image encoder dependency is introduced.

When private resources are absent, or RETROHATCH_UNBRANDED=1, the script emits an empty
optional icon. The normal Windows/eframe icons remain. Windows builds always
embed Cargo-derived VERSIONINFO and require Windows SDK rc.exe, even without
private artwork. The compiler is discovered from RC/PATH/the installed SDK;
a missing compiler reports a clear SDK installation/path error. Non-Windows
builds skip Windows resource compilation. Adding/removing local branding
invalidates the build-script output. Public checkouts need no private icon.

## Capture pipeline

The existing immutable worker Snapshot supplies display data. F8/Screenshots captures
color pixels at 128×128 directly. Classic segment LCDs are vector geometries,
not ordinary framebuffers: the export rasterizes the canonical 352×329 reference
bounds, all matrix segments and eight symbols using existing icon coverage.
It retains the intrinsic LCD substrate/on/off colors, excludes app decoration,
and never resizes a rendered window or changes segment data.

F9/Screenshots requests the next clean egui frame after the menu closes. On Windows,
PrintWindow renders only this process's native window; its unreliable OpenGL
client area is replaced with egui's actual GPU readback. DWM visible-frame
bounds remove invisible resize margins. This includes caption/icon without
screen/desktop blitting, unrelated windows, interpolation or stale GPU pixels.
On other platforms egui captures the application client viewport only.
Resize races, unavailable windows and failed native/API writes report errors.

PNG compression/writing uses the existing attributed tg18::png encoder on a
background thread. Only one job is in flight, with a timeout and visible
confirmation/error. Pending captures finish before normal application exit.
There is no pause, emulation command or state/time change for a screenshot.
Capture folders are created below the Windows Pictures known folder; see
[updated storage/recording behavior](CAPTURE_MEDIA.md). Old AppData PNGs remain untouched.
Filenames contain the model ID, mode, UTC timestamp (Z) and counter. create_new
provides atomic collision refusal; previous files are never overwritten.
Generated captures remain ignored/private and may contain firmware artwork.

## Validation

- Original asset hashes are preserved; the public PNG is byte-identical.
- Git tracks the public branding logo copy, not ../local/resources/. The owner-managed
  gallery has its own narrow exception; captures are never automatically added.
- Windows loads EXE resource 101 at 16/32/48/128/256 pixels with correct bitmap
  dimensions. Small and large native window icons are both installed.
- A separate copy of current public build sources without resources/, ROMs,
  saves or private docs passes locked offline compilation.
- F8/F9 create valid native-LCD/full-window PNGs for actual local P1 and P's
  firmware in an isolated data directory. Images were opened and inspected:
  352×329 classic layout, 128×128 color pixels, and the complete native window
  with caption/icon, menus and controls, without desktop content.
- Unit tests cover PNG pixel round-trip, unique non-overwriting paths, directory
  creation, I/O failure, icon presence/fallback and all six classic LCD layouts.
  The full Rust suite passes: 177 tests, zero failures, one ignored test.
  All eight package/privacy Python tests pass. cargo fmt --check, locked check
  and the optimized Windows release build pass.

Changed files: root .gitignore/README, public logo, app/build.rs, Cargo Windows
API feature selection, src/branding.rs/main.rs, host/screenshots.rs and
screenshot_windows.rs, lcd_render.rs, ui.rs/ui/screenshots.rs, the lcd_probe example
geometry imports, package allowlists/
tests and the dependency feature inventory. CPU/MCU, ROM and save logic are unchanged.
