<!-- SPDX-License-Identifier: GPL-3.0-only -->
<!-- SPDX-FileCopyrightText: 2026 Noctyros -->
# Capture storage, LCD scale and video

Version: this feature advances Cargo from 0.1.0 to 0.2.0. Cargo remains the
single version source; no archive, tag or release is created.

## Use and storage

Screenshots follows My Tamagotchis in the menu bar. F8 captures native LCD PNG;
F9 captures the visible application window, including Windows chrome. Folder
commands open Explorer. F10 starts/stops the default Emulator video; the same
menu also starts LCD video and stops either recording mode. Help lists all
three shortcuts. Keyboard controls still require application focus.

FOLDERID_Pictures/SHGetKnownFolderPath resolves the real Windows Pictures folder,
including redirection. Under Pictures/RetroHatch, Screenshots and Recordings each
have LCD and Emulator subfolders. All four are created at startup; errors are
visible. UTC/model names use exclusive creation. Existing AppData captures are
not moved or deleted. RETROHATCH_CAPTURE_DIR overrides this media root for
isolated testing; RETROHATCH_DATA_DIR continues controlling settings/saves only.

Display Scale persists as an optional, validated settings field: 33, 50, 75,
100, 150, 200 or 300 percent. Legacy settings default to 100. Only LCD geometry
scales; buttons, menus and input coordinates keep their original UI units.
100% retains the responsive normal LCD size. Window resizing is monitor-bounded;
large displays scroll in both directions. Color textures use nearest-neighbor
sampling; existing exact segment/icon rendering is retained. F8 stays 352x329
for classic segment layouts and 128x128 for supported color framebuffers.

## Native recording and limitations

host/recording.rs owns a bounded two-frame input queue and a separate encoder
thread. Media Foundation's sink writer encodes MP4/H.264 at rational 30 FPS
using monotonic wall time. No external executable or new Cargo package is used.
SDK API bindings live in recording_windows.rs. Optional MF DLLs load securely
from System32 at runtime, so missing MF cannot prevent ordinary app startup.
Missing codecs/Windows N Media Feature Pack, I/O and finalization failures report
errors and identify any incomplete file; incomplete files are not called saved.

Encoding never drives the emulated clock. Delayed input repeats the last frame;
queue overflow drops input frames without pausing the CPU. Counts appear on
completion. The first frame fixes dimensions, capped at 1920 pixels per side.
Later window resizing letterboxes using nearest-neighbor sampling without
stretching. H.264 requires even dimensions: a classic 352x329 LCD gets one black
padding row (352x330), rather than a changed LCD shape. H.264 is lossy YUV420;
PNG remains the exact-color export. A minimized window holds its last frame.
There is one active session, a visible elapsed-time indicator and explicit
finalization on Stop/normal exit. Game audio is NOT recorded; UI/tooltips say
video only. Audio muxing/synchronization is future work. Playback audio remains
unchanged. Non-Windows native recording/storage is not implemented.

## Validation

Native release tests used isolated data roots and user-provided P1/P's firmware.
P1 F8/F9 produced valid Pictures PNGs and LCD/window MP4s. P's window recording
showed actual color firmware graphics; LCD recording and worker PNG export were
also tested (including awake color output). Real clips decoded through Qt's
installed media backend: 30 FPS, correct orientation, P1 LCD 352x330, P's LCD
128x128, full window 462x692. Synthetic red/blue frames verify RGB/BGR conversion
and top/bottom orientation. Normal exit produced a playable finalized MP4.
P1 window recording measured 99.98% real-time speed; awake P's LCD recording
measured 100.00%. This is focused local validation, not a long-session benchmark.
Windows foreground policy limited automated P's key-input checks; existing
focus-dependent shortcuts were preserved and menu dispatch is tested headlessly.

Tests cover all scale values and migration, actual egui menu dispatch, fixed-size
controls, existing size/DPI/input regressions, exact timestamp totals, padding,
PNG integrity/collision refusal/I/O errors and missing optional media libraries.
Physical disk-full and an actual Windows N machine remain untested. Personal
ROMs, saves, graphics references and emulator timing were not modified.

Final checks: cargo fmt --check, cargo check --locked, the complete Rust suite
(182 passed, one ignored), nine Python package tests, the 106-package license
inventory check and the Windows release build pass. A native restart test found
that creation-time viewport commands are discarded; initial scaling is now
applied in the first UI frame instead. All UI tests were rerun after that fix.

The final native 200% restart viewport measures 848x1028 and exits normally.
Release import validation recognizes combase.dll as a Windows system component;
no extra DLL is bundled. The original test fixture was corrected to preserve
LF bytes when calculating its settings integrity hash.
